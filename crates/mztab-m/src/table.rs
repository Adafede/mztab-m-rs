//! Typing table headers and rows.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::diag::{Code, Detail};
use crate::id::{AssayId, ElementId, IdConfidenceMeasureId, MsRunId, StudyVariableId};
use crate::model::{
    AbundanceColumn, Column, Header, OptColumn, OptTarget, SmeColumn, SmeId, SmeRow, SmfColumn,
    SmfId, SmfRow, SmlColumn, SmlId, SmlRow, parse_index,
};
use crate::typing::{self, Cx, RawCell};
use crate::value::{Cell, Decimal, Raw, Text};
use crate::vocab::Section;
use crate::write::{Out, Render as _, WriteError};

/// What a header column holds, without borrowing the header text. The
/// reader keeps one per column to type rows, including across stream chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot<S> {
    Stable(S),
    /// An abundance or SME confidence-measure column.
    Number,
    Opt,
    Unknown,
}

/// Parses `prefix[n]` with the given element prefix, e.g. `abundance_assay`.
fn indexed<I: ElementId>(label: &str, prefix: &str) -> Option<I> {
    let inner = label
        .strip_prefix(prefix)?
        .strip_prefix('[')?
        .strip_suffix(']')?;
    I::from_index(parse_index(inner)?)
}

/// Parses `element[n]_` at the start of `text`, returning the id and the rest.
fn target_prefix<I: ElementId>(text: &str) -> Option<(I, &str)> {
    let (head, rest) = text.split_once("]_")?;
    let index = head.strip_prefix(I::ELEMENT)?.strip_prefix('[')?;
    Some((I::from_index(parse_index(index)?)?, rest))
}

/// Parses `opt_{identifier}_{name}`.
fn opt_column(label: &str) -> Option<OptColumn<'_>> {
    let rest = label.strip_prefix("opt_")?;
    let (target, name) = if let Some(name) = rest.strip_prefix("global_") {
        (OptTarget::Global, name)
    } else if let Some((id, name)) = target_prefix::<AssayId>(rest) {
        (OptTarget::Assay(id), name)
    } else if let Some((id, name)) = target_prefix::<StudyVariableId>(rest) {
        (OptTarget::StudyVariable(id), name)
    } else {
        let (id, name) = target_prefix::<MsRunId>(rest)?;
        (OptTarget::MsRun(id), name)
    };
    Some(OptColumn {
        target,
        name: Text::new(name).ok()?,
    })
}

/// A column that is neither stable nor unknown.
enum Special<'a> {
    Abundance(AbundanceColumn),
    IdConfidence(IdConfidenceMeasureId),
    Opt(OptColumn<'a>),
}

impl<'a> Special<'a> {
    /// The column, for a table of any stable-column type.
    fn column<S>(self) -> Column<'a, S> {
        match self {
            Self::Abundance(a) => Column::Abundance(a),
            Self::IdConfidence(i) => Column::IdConfidence(i),
            Self::Opt(o) => Column::Opt(o),
        }
    }
}

/// The non-stable columns a table allows. Not generic over the table, so
/// each branch is one function (and one coverage region), not three.
fn special_column(label: &str, section: Section) -> Option<Special<'_>> {
    if let Some(opt) = opt_column(label) {
        return Some(Special::Opt(opt));
    }
    if section == Section::Sme {
        let id = indexed::<IdConfidenceMeasureId>(label, "id_confidence_measure")?;
        return Some(Special::IdConfidence(id));
    }
    let abundance = if let Some(id) = indexed(label, "abundance_assay") {
        AbundanceColumn::Assay(id)
    } else if section == Section::Smf {
        return None;
    } else if let Some(id) = indexed(label, "abundance_study_variable") {
        AbundanceColumn::StudyVariable(id)
    } else {
        AbundanceColumn::VariationStudyVariable(indexed(
            label,
            "abundance_variation_study_variable",
        )?)
    };
    Some(Special::Abundance(abundance))
}

/// Types a header line's column cells. Unknown and repeated columns become
/// [`Column::Unknown`], with a finding.
pub(crate) fn header<'a, S: Copy + Ord>(
    cx: &mut Cx<'_>,
    section: Section,
    cells: impl Iterator<Item = RawCell<'a>>,
    stable: fn(&str) -> Option<S>,
) -> (Header<'a, S>, Vec<Slot<S>>) {
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut slots = Vec::new();
    for cell in cells {
        let label = cell.text;
        let column = stable(label)
            .map(Column::Stable)
            .or_else(|| special_column(label, section).map(Special::column));
        let column = if seen.insert(label) {
            column.unwrap_or_else(|| {
                cx.report(Code::UnknownColumn, cell.offset, cell.number, Detail::None);
                Column::Unknown(Raw::new(label))
            })
        } else {
            cx.report(
                Code::DuplicateColumn,
                cell.offset,
                cell.number,
                Detail::None,
            );
            Column::Unknown(Raw::new(label))
        };
        slots.push(match &column {
            Column::Stable(s) => Slot::Stable(*s),
            Column::Abundance(_) | Column::IdConfidence(_) => Slot::Number,
            Column::Opt(_) => Slot::Opt,
            Column::Unknown(_) => Slot::Unknown,
        });
        columns.push(column);
    }
    (Header { columns }, slots)
}

/// A table row the reader can fill column by column.
pub(crate) trait TableRow<'a>: Sized {
    /// The table's stable-column enum.
    type Column: Copy;

    /// A row with every stable field `null` and no other cells.
    fn empty() -> Self;

    /// Types `raw` into the stable field `column`.
    fn set(&mut self, column: Self::Column, cx: &mut Cx<'_>, raw: RawCell<'a>);

    /// The numeric non-stable cells: abundances, or SME confidence measures.
    fn numbers(&mut self) -> &mut Vec<Cell<'a, Decimal<'a>>>;

    /// The `opt_` cells.
    fn opt(&mut self) -> &mut Vec<Cell<'a, Text<'a>>>;

    /// The unknown-column cells.
    fn unknown(&mut self) -> &mut Vec<Raw<'a>>;

    /// The cells beyond the header.
    fn extra(&mut self) -> &mut Vec<Raw<'a>>;

    /// Writes the stable field `column`.
    fn render_stable(&self, column: Self::Column, out: &mut Out<'_>) -> Result<(), WriteError>;

    /// The numeric non-stable cells.
    fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>];

    /// The `opt_` cells.
    fn opt_ref(&self) -> &[Cell<'a, Text<'a>>];

    /// The unknown-column cells.
    fn unknown_ref(&self) -> &[Raw<'a>];

    /// The cells beyond the header.
    fn extra_ref(&self) -> &[Raw<'a>];
}

/// Types a row against its header's slots. `cells` excludes the prefix.
pub(crate) fn row<'a, R: TableRow<'a>>(
    cx: &mut Cx<'_>,
    slots: &[Slot<R::Column>],
    cells: impl Iterator<Item = RawCell<'a>>,
) -> R {
    let mut row = R::empty();
    let mut found = 0_usize;
    let mut first_extra = None;
    for (i, cell) in cells.enumerate() {
        found = i + 1;
        match slots.get(i) {
            Some(Slot::Stable(column)) => row.set(*column, cx, cell),
            Some(Slot::Number) => {
                let value = typing::decimal(cx, cell);
                row.numbers().push(value);
            }
            Some(Slot::Opt) => {
                let value = typing::text(cx, cell);
                row.opt().push(value);
            }
            Some(Slot::Unknown) => row.unknown().push(Raw::new(cell.text)),
            None => {
                first_extra.get_or_insert((cell.offset, cell.number));
                row.extra().push(Raw::new(cell.text));
            }
        }
    }
    if found != slots.len() {
        // A short row is located at its end, a long one at its first extra cell.
        let (offset, number) = first_extra.unwrap_or_else(|| (cx.line.len(), u32_of(found + 2)));
        let detail = Detail::CellCount {
            expected: u32_of(slots.len() + 1),
            found: u32_of(found + 1),
        };
        cx.report(Code::CellCount, offset, number, detail);
    }
    row
}

fn u32_of(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn sml_id<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, SmlId> {
    typing::integer(cx, raw).map(SmlId)
}

fn smf_id<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, SmfId> {
    typing::integer(cx, raw).map(SmfId)
}

fn sme_id<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, SmeId> {
    typing::integer(cx, raw).map(SmeId)
}

fn texts<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, Vec<Cell<'a, Text<'a>>>> {
    typing::list(cx, raw, false, typing::text)
}

impl<'a> TableRow<'a> for SmlRow<'a> {
    type Column = SmlColumn;

    fn empty() -> Self {
        Self {
            sml_id: Cell::Null,
            smf_id_refs: Cell::Null,
            database_identifier: Cell::Null,
            chemical_formula: Cell::Null,
            smiles: Cell::Null,
            inchi: Cell::Null,
            chemical_name: Cell::Null,
            uri: Cell::Null,
            theoretical_neutral_mass: Cell::Null,
            adduct_ions: Cell::Null,
            reliability: Cell::Null,
            best_id_confidence_measure: Cell::Null,
            best_id_confidence_value: Cell::Null,
            abundance: Vec::new(),
            opt: Vec::new(),
            unknown: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn set(&mut self, column: SmlColumn, cx: &mut Cx<'_>, raw: RawCell<'a>) {
        match column {
            SmlColumn::SmlId => self.sml_id = sml_id(cx, raw),
            SmlColumn::SmfIdRefs => self.smf_id_refs = typing::list(cx, raw, false, smf_id),
            SmlColumn::DatabaseIdentifier => self.database_identifier = texts(cx, raw),
            SmlColumn::ChemicalFormula => self.chemical_formula = texts(cx, raw),
            SmlColumn::Smiles => self.smiles = texts(cx, raw),
            SmlColumn::Inchi => self.inchi = texts(cx, raw),
            SmlColumn::ChemicalName => self.chemical_name = texts(cx, raw),
            SmlColumn::Uri => self.uri = texts(cx, raw),
            SmlColumn::TheoreticalNeutralMass => {
                self.theoretical_neutral_mass = typing::list(cx, raw, false, typing::decimal);
            }
            SmlColumn::AdductIons => self.adduct_ions = texts(cx, raw),
            SmlColumn::Reliability => self.reliability = typing::text(cx, raw),
            SmlColumn::BestIdConfidenceMeasure => {
                self.best_id_confidence_measure = typing::param(cx, raw);
            }
            SmlColumn::BestIdConfidenceValue => {
                self.best_id_confidence_value = typing::decimal(cx, raw);
            }
        }
    }

    fn numbers(&mut self) -> &mut Vec<Cell<'a, Decimal<'a>>> {
        &mut self.abundance
    }

    fn opt(&mut self) -> &mut Vec<Cell<'a, Text<'a>>> {
        &mut self.opt
    }

    fn unknown(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.unknown
    }

    fn extra(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.extra
    }

    fn render_stable(&self, column: SmlColumn, out: &mut Out<'_>) -> Result<(), WriteError> {
        match column {
            SmlColumn::SmlId => self.sml_id.render(out),
            SmlColumn::SmfIdRefs => self.smf_id_refs.render(out),
            SmlColumn::DatabaseIdentifier => self.database_identifier.render(out),
            SmlColumn::ChemicalFormula => self.chemical_formula.render(out),
            SmlColumn::Smiles => self.smiles.render(out),
            SmlColumn::Inchi => self.inchi.render(out),
            SmlColumn::ChemicalName => self.chemical_name.render(out),
            SmlColumn::Uri => self.uri.render(out),
            SmlColumn::TheoreticalNeutralMass => self.theoretical_neutral_mass.render(out),
            SmlColumn::AdductIons => self.adduct_ions.render(out),
            SmlColumn::Reliability => self.reliability.render(out),
            SmlColumn::BestIdConfidenceMeasure => self.best_id_confidence_measure.render(out),
            SmlColumn::BestIdConfidenceValue => self.best_id_confidence_value.render(out),
        }
    }

    fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>] {
        &self.abundance
    }

    fn opt_ref(&self) -> &[Cell<'a, Text<'a>>] {
        &self.opt
    }

    fn unknown_ref(&self) -> &[Raw<'a>] {
        &self.unknown
    }

    fn extra_ref(&self) -> &[Raw<'a>] {
        &self.extra
    }
}

impl<'a> TableRow<'a> for SmfRow<'a> {
    type Column = SmfColumn;

    fn empty() -> Self {
        Self {
            smf_id: Cell::Null,
            sme_id_refs: Cell::Null,
            sme_id_ref_ambiguity_code: Cell::Null,
            adduct_ion: Cell::Null,
            isotopomer: Cell::Null,
            exp_mass_to_charge: Cell::Null,
            charge: Cell::Null,
            retention_time_in_seconds: Cell::Null,
            retention_time_in_seconds_start: Cell::Null,
            retention_time_in_seconds_end: Cell::Null,
            abundance: Vec::new(),
            opt: Vec::new(),
            unknown: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn set(&mut self, column: SmfColumn, cx: &mut Cx<'_>, raw: RawCell<'a>) {
        match column {
            SmfColumn::SmfId => self.smf_id = smf_id(cx, raw),
            SmfColumn::SmeIdRefs => self.sme_id_refs = typing::list(cx, raw, false, sme_id),
            SmfColumn::SmeIdRefAmbiguityCode => {
                self.sme_id_ref_ambiguity_code = typing::ambiguity(cx, raw);
            }
            SmfColumn::AdductIon => self.adduct_ion = typing::text(cx, raw),
            SmfColumn::Isotopomer => self.isotopomer = typing::param(cx, raw),
            SmfColumn::ExpMassToCharge => self.exp_mass_to_charge = typing::decimal(cx, raw),
            SmfColumn::Charge => self.charge = typing::integer(cx, raw),
            SmfColumn::RetentionTimeInSeconds => {
                self.retention_time_in_seconds = typing::decimal(cx, raw);
            }
            SmfColumn::RetentionTimeInSecondsStart => {
                self.retention_time_in_seconds_start = typing::decimal(cx, raw);
            }
            SmfColumn::RetentionTimeInSecondsEnd => {
                self.retention_time_in_seconds_end = typing::decimal(cx, raw);
            }
        }
    }

    fn numbers(&mut self) -> &mut Vec<Cell<'a, Decimal<'a>>> {
        &mut self.abundance
    }

    fn opt(&mut self) -> &mut Vec<Cell<'a, Text<'a>>> {
        &mut self.opt
    }

    fn unknown(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.unknown
    }

    fn extra(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.extra
    }

    fn render_stable(&self, column: SmfColumn, out: &mut Out<'_>) -> Result<(), WriteError> {
        match column {
            SmfColumn::SmfId => self.smf_id.render(out),
            SmfColumn::SmeIdRefs => self.sme_id_refs.render(out),
            SmfColumn::SmeIdRefAmbiguityCode => self.sme_id_ref_ambiguity_code.render(out),
            SmfColumn::AdductIon => self.adduct_ion.render(out),
            SmfColumn::Isotopomer => self.isotopomer.render(out),
            SmfColumn::ExpMassToCharge => self.exp_mass_to_charge.render(out),
            SmfColumn::Charge => self.charge.render(out),
            SmfColumn::RetentionTimeInSeconds => self.retention_time_in_seconds.render(out),
            SmfColumn::RetentionTimeInSecondsStart => {
                self.retention_time_in_seconds_start.render(out)
            }
            SmfColumn::RetentionTimeInSecondsEnd => self.retention_time_in_seconds_end.render(out),
        }
    }

    fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>] {
        &self.abundance
    }

    fn opt_ref(&self) -> &[Cell<'a, Text<'a>>] {
        &self.opt
    }

    fn unknown_ref(&self) -> &[Raw<'a>] {
        &self.unknown
    }

    fn extra_ref(&self) -> &[Raw<'a>] {
        &self.extra
    }
}

impl<'a> TableRow<'a> for SmeRow<'a> {
    type Column = SmeColumn;

    fn empty() -> Self {
        Self {
            sme_id: Cell::Null,
            evidence_input_id: Cell::Null,
            database_identifier: Cell::Null,
            chemical_formula: Cell::Null,
            smiles: Cell::Null,
            inchi: Cell::Null,
            chemical_name: Cell::Null,
            uri: Cell::Null,
            derivatized_form: Cell::Null,
            adduct_ion: Cell::Null,
            exp_mass_to_charge: Cell::Null,
            charge: Cell::Null,
            theoretical_mass_to_charge: Cell::Null,
            spectra_ref: Cell::Null,
            identification_method: Cell::Null,
            ms_level: Cell::Null,
            rank: Cell::Null,
            id_confidence: Vec::new(),
            opt: Vec::new(),
            unknown: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn set(&mut self, column: SmeColumn, cx: &mut Cx<'_>, raw: RawCell<'a>) {
        match column {
            SmeColumn::SmeId => self.sme_id = sme_id(cx, raw),
            SmeColumn::EvidenceInputId => self.evidence_input_id = typing::text(cx, raw),
            SmeColumn::DatabaseIdentifier => self.database_identifier = typing::text(cx, raw),
            SmeColumn::ChemicalFormula => self.chemical_formula = typing::text(cx, raw),
            SmeColumn::Smiles => self.smiles = typing::text(cx, raw),
            SmeColumn::Inchi => self.inchi = typing::text(cx, raw),
            SmeColumn::ChemicalName => self.chemical_name = typing::text(cx, raw),
            SmeColumn::Uri => self.uri = typing::text(cx, raw),
            SmeColumn::DerivatizedForm => self.derivatized_form = typing::param(cx, raw),
            SmeColumn::AdductIon => self.adduct_ion = typing::text(cx, raw),
            SmeColumn::ExpMassToCharge => self.exp_mass_to_charge = typing::decimal(cx, raw),
            SmeColumn::Charge => self.charge = typing::integer(cx, raw),
            SmeColumn::TheoreticalMassToCharge => {
                self.theoretical_mass_to_charge = typing::decimal(cx, raw);
            }
            SmeColumn::SpectraRef => {
                self.spectra_ref = typing::list(cx, raw, false, typing::spectra_ref);
            }
            SmeColumn::IdentificationMethod => self.identification_method = typing::param(cx, raw),
            SmeColumn::MsLevel => self.ms_level = typing::param(cx, raw),
            SmeColumn::Rank => self.rank = typing::integer(cx, raw),
        }
    }

    fn numbers(&mut self) -> &mut Vec<Cell<'a, Decimal<'a>>> {
        &mut self.id_confidence
    }

    fn opt(&mut self) -> &mut Vec<Cell<'a, Text<'a>>> {
        &mut self.opt
    }

    fn unknown(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.unknown
    }

    fn extra(&mut self) -> &mut Vec<Raw<'a>> {
        &mut self.extra
    }

    fn render_stable(&self, column: SmeColumn, out: &mut Out<'_>) -> Result<(), WriteError> {
        match column {
            SmeColumn::SmeId => self.sme_id.render(out),
            SmeColumn::EvidenceInputId => self.evidence_input_id.render(out),
            SmeColumn::DatabaseIdentifier => self.database_identifier.render(out),
            SmeColumn::ChemicalFormula => self.chemical_formula.render(out),
            SmeColumn::Smiles => self.smiles.render(out),
            SmeColumn::Inchi => self.inchi.render(out),
            SmeColumn::ChemicalName => self.chemical_name.render(out),
            SmeColumn::Uri => self.uri.render(out),
            SmeColumn::DerivatizedForm => self.derivatized_form.render(out),
            SmeColumn::AdductIon => self.adduct_ion.render(out),
            SmeColumn::ExpMassToCharge => self.exp_mass_to_charge.render(out),
            SmeColumn::Charge => self.charge.render(out),
            SmeColumn::TheoreticalMassToCharge => self.theoretical_mass_to_charge.render(out),
            SmeColumn::SpectraRef => self.spectra_ref.render(out),
            SmeColumn::IdentificationMethod => self.identification_method.render(out),
            SmeColumn::MsLevel => self.ms_level.render(out),
            SmeColumn::Rank => self.rank.render(out),
        }
    }

    fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>] {
        &self.id_confidence
    }

    fn opt_ref(&self) -> &[Cell<'a, Text<'a>>] {
        &self.opt
    }

    fn unknown_ref(&self) -> &[Raw<'a>] {
        &self.unknown
    }

    fn extra_ref(&self) -> &[Raw<'a>] {
        &self.extra
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::Special;
    use crate::id::{AssayId, IdConfidenceMeasureId};
    use crate::model::{AbundanceColumn, Column, OptColumn, OptTarget, SmlColumn};
    use crate::value::Text;

    /// The reader only builds the combinations a table allows; any table type
    /// can hold every kind of special column.
    #[test]
    fn special_columns_fit_any_table() {
        let assay = AssayId::new(1).expect("non-zero");
        let measure = IdConfidenceMeasureId::new(1).expect("non-zero");
        let opt = OptColumn {
            target: OptTarget::Global,
            name: Text::new("x").expect("valid text"),
        };
        let columns: [Column<'_, SmlColumn>; 3] = [
            Special::Abundance(AbundanceColumn::Assay(assay)).column(),
            Special::IdConfidence(measure).column(),
            Special::Opt(opt.clone()).column(),
        ];
        assert_eq!(
            columns,
            [
                Column::Abundance(AbundanceColumn::Assay(assay)),
                Column::IdConfidence(measure),
                Column::Opt(opt),
            ],
            "each kind keeps its value"
        );
    }
}
