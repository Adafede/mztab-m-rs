//! Typing table headers and rows.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::diag::{Code, Detail};
use crate::id::{AssayId, ElementId, IdConfidenceMeasureId, MsRunId, StudyVariableId, parse_index};
use crate::model::{
    AbundanceColumn, Column, Header, OptColumn, OptTarget, SmeColumn, SmeId, SmeRow, SmfColumn,
    SmfId, SmfRow, SmlColumn, SmlId, SmlRow,
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

impl<S: Copy> Slot<S> {
    pub(crate) const fn of(column: &Column<'_, S>) -> Self {
        match column {
            Column::Stable(s) => Self::Stable(*s),
            Column::Abundance(_) | Column::IdConfidence(_) => Self::Number,
            Column::Opt(_) => Self::Opt,
            Column::Unknown(_) => Self::Unknown,
        }
    }
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
        let id = typing::indexed::<IdConfidenceMeasureId>(label, "id_confidence_measure")?;
        return Some(Special::IdConfidence(id));
    }
    let abundance = if let Some(id) = typing::indexed(label, "abundance_assay") {
        AbundanceColumn::Assay(id)
    } else if section == Section::Smf {
        return None;
    } else if let Some(id) = typing::indexed(label, "abundance_study_variable") {
        AbundanceColumn::StudyVariable(id)
    } else {
        AbundanceColumn::VariationStudyVariable(typing::indexed(
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
        slots.push(Slot::of(&column));
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

macro_rules! table_row {
    ($row:ident, $col:ident, $numbers:ident, {
        $($variant:ident => $field:ident : $p:ident $(::$sub:ident)? $(($arg:path))?),+ $(,)?
    }) => {
        impl<'a> TableRow<'a> for $row<'a> {
            type Column = $col;

            fn empty() -> Self {
                Self {
                    $($field: Cell::Null,)+
                    $numbers: Vec::new(),
                    opt: Vec::new(),
                    unknown: Vec::new(),
                    extra: Vec::new(),
                }
            }

            fn set(&mut self, column: $col, cx: &mut Cx<'_>, raw: RawCell<'a>) {
                match column {
                    $($col::$variant => {
                        self.$field = table_row!(@parse cx, raw, $p $(::$sub)? $(($arg))?);
                    })+
                }
            }

            fn numbers(&mut self) -> &mut Vec<Cell<'a, Decimal<'a>>> {
                &mut self.$numbers
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

            fn render_stable(&self, column: $col, out: &mut Out<'_>) -> Result<(), WriteError> {
                match column {
                    $($col::$variant => self.$field.render(out),)+
                }
            }

            fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>] {
                &self.$numbers
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
    };
    (@parse $cx:ident, $raw:ident, id($wrap:path)) => {
        typing::integer($cx, $raw).map($wrap)
    };
    (@parse $cx:ident, $raw:ident, list_id($wrap:path)) => {
        typing::list($cx, $raw, false, |cx, raw| typing::integer(cx, raw).map($wrap))
    };
    (@parse $cx:ident, $raw:ident, list($parser:path)) => {
        typing::list($cx, $raw, false, $parser)
    };
    (@parse $cx:ident, $raw:ident, $parser:path) => {
        $parser($cx, $raw)
    };
}

table_row!(SmlRow, SmlColumn, abundance, {
    SmlId => sml_id: id(SmlId),
    SmfIdRefs => smf_id_refs: list_id(SmfId),
    DatabaseIdentifier => database_identifier: list(typing::text),
    ChemicalFormula => chemical_formula: list(typing::text),
    Smiles => smiles: list(typing::text),
    Inchi => inchi: list(typing::text),
    ChemicalName => chemical_name: list(typing::text),
    Uri => uri: list(typing::text),
    TheoreticalNeutralMass => theoretical_neutral_mass: list(typing::decimal),
    AdductIons => adduct_ions: list(typing::text),
    Reliability => reliability: typing::text,
    BestIdConfidenceMeasure => best_id_confidence_measure: typing::param,
    BestIdConfidenceValue => best_id_confidence_value: typing::decimal,
});

table_row!(SmfRow, SmfColumn, abundance, {
    SmfId => smf_id: id(SmfId),
    SmeIdRefs => sme_id_refs: list_id(SmeId),
    SmeIdRefAmbiguityCode => sme_id_ref_ambiguity_code: typing::ambiguity,
    AdductIon => adduct_ion: typing::text,
    Isotopomer => isotopomer: typing::param,
    ExpMassToCharge => exp_mass_to_charge: typing::decimal,
    Charge => charge: typing::integer,
    RetentionTimeInSeconds => retention_time_in_seconds: typing::decimal,
    RetentionTimeInSecondsStart => retention_time_in_seconds_start: typing::decimal,
    RetentionTimeInSecondsEnd => retention_time_in_seconds_end: typing::decimal,
});

table_row!(SmeRow, SmeColumn, id_confidence, {
    SmeId => sme_id: id(SmeId),
    EvidenceInputId => evidence_input_id: typing::text,
    DatabaseIdentifier => database_identifier: typing::text,
    ChemicalFormula => chemical_formula: typing::text,
    Smiles => smiles: typing::text,
    Inchi => inchi: typing::text,
    ChemicalName => chemical_name: typing::text,
    Uri => uri: typing::text,
    DerivatizedForm => derivatized_form: typing::param,
    AdductIon => adduct_ion: typing::text,
    ExpMassToCharge => exp_mass_to_charge: typing::decimal,
    Charge => charge: typing::integer,
    TheoreticalMassToCharge => theoretical_mass_to_charge: typing::decimal,
    SpectraRef => spectra_ref: list(typing::spectra_ref),
    IdentificationMethod => identification_method: typing::param,
    MsLevel => ms_level: typing::param,
    Rank => rank: typing::integer,
});

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
