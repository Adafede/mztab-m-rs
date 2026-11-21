//! Table rules: empty tables, columns, references, row ids, nulls, values,
//! reliability, adducts, ambiguity codes and profile cross-references.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use super::{Report, Site, label};
use crate::diag::{Code, Detail, Severity};
use crate::id::IdConfidenceMeasureId;
use crate::meta::MtdKey;
use crate::model::{
    AbundanceColumn, ColUnit, Column, Header, List, MzTab, OptTarget, SmeColumn, SmeId, SmeRow,
    SmfColumn, SmlColumn, Table,
};
use crate::param::Param;
use crate::value::{Cell, Decimal, Text};
use crate::vocab::{Profile, RuleSet, Section};

pub(super) fn check(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let declared = mztab
        .metadata
        .profile
        .as_ref()
        .and_then(|c| c.value())
        .copied();
    if let Some(t) = &mztab.sml {
        table_shape(mztab, declared, Section::Sml, t, SmlColumn::ALL, report);
    }
    if let Some(t) = &mztab.smf {
        table_shape(mztab, declared, Section::Smf, t, SmfColumn::ALL, report);
    }
    if let Some(t) = &mztab.sme {
        table_shape(mztab, declared, Section::Sme, t, SmeColumn::ALL, report);
    }
    sml_rows(mztab, rules, report);
    smf_rows(mztab, rules, report);
    sme_rows(mztab, rules, report);
    colunits(mztab, report);
}

/// A header column without its table's stable-column type.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A stable column, by canonical rank.
    Stable(usize),
    Abundance(AbundanceColumn),
    IdConfidence(IdConfidenceMeasureId),
    Opt(OptTarget),
    Unknown,
}

impl Kind {
    /// The canonical rank, for order checks; `None` for columns whose place
    /// is not specified (`opt_`, unknown).
    const fn rank(self) -> Option<usize> {
        match self {
            Self::Stable(rank) => Some(rank),
            Self::Abundance(AbundanceColumn::Assay(_)) => Some(1000),
            Self::Abundance(AbundanceColumn::StudyVariable(_)) => Some(1001),
            Self::Abundance(AbundanceColumn::VariationStudyVariable(_)) => Some(1002),
            // SME: after `ms_level`, before `rank` (field reference order).
            Self::IdConfidence(_) => Some(155),
            Self::Opt(_) | Self::Unknown => None,
        }
    }
}

/// The kinds of a header's columns.
fn kinds<S: Copy + Ord>(header: &Header<'_, S>, stable: &[S]) -> Vec<Kind> {
    header
        .columns
        .iter()
        .map(|column| match column {
            Column::Stable(s) => {
                Kind::Stable(stable.iter().position(|x| x == s).unwrap_or_default() * 10)
            }
            Column::Abundance(a) => Kind::Abundance(*a),
            Column::IdConfidence(i) => Kind::IdConfidence(*i),
            Column::Opt(o) => Kind::Opt(o.target),
            Column::Unknown(_) => Kind::Unknown,
        })
        .collect()
}

/// Columns: emptiness, mandatory columns, order, `opt_` position, and
/// references from column names to metadata.
fn table_shape<S: Copy + Ord + core::fmt::Display, R>(
    mztab: &MzTab<'_>,
    declared: Option<Profile>,
    section: Section,
    table: &Table<'_, S, R>,
    stable: &[S],
    report: &mut Report<'_, '_>,
) {
    for s in stable {
        if !table
            .header
            .columns
            .iter()
            .any(|c| matches!(c, Column::Stable(x) if x == s))
        {
            report.add(Code::MissingColumn, Site::Header(section), label(s));
        }
    }
    let shape = Shape {
        declared,
        section,
        empty: table.rows.is_empty(),
        kinds: kinds(&table.header, stable),
    };
    shape.check(mztab, report);
}

/// What the column checks need to know about a table.
struct Shape {
    declared: Option<Profile>,
    section: Section,
    empty: bool,
    kinds: Vec<Kind>,
}

impl Shape {
    fn check(&self, mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
        let m = &mztab.metadata;
        let section = self.section;
        if report.rule_set == RuleSet::V2_1 && self.empty {
            // DECISIONS D-019: an error when the profile is declared, a
            // warning when it is inferred.
            let severity = if self.declared.is_some() {
                Severity::Error
            } else {
                Severity::Warning
            };
            report.add_at(
                Code::EmptyTable,
                severity,
                Site::Header(section),
                Detail::None,
            );
        }
        // Every assay and study variable needs its abundance columns, and
        // every confidence measure its SME column: (column, its label).
        let mut expected: Vec<(Kind, Detail)> = Vec::new();
        let abundance = |column| (Kind::Abundance(column), label(column));
        if matches!(section, Section::Sml | Section::Smf) {
            expected.extend(
                m.assay
                    .keys()
                    .map(|id| abundance(AbundanceColumn::Assay(id))),
            );
        }
        if section == Section::Sml {
            let svs = m.study_variable.keys();
            expected.extend(svs.map(|id| abundance(AbundanceColumn::StudyVariable(id))));
            let svs = m.study_variable.keys();
            expected.extend(svs.map(|id| abundance(AbundanceColumn::VariationStudyVariable(id))));
        }
        if section == Section::Sme {
            let measures = m.id_confidence_measure.keys();
            expected.extend(measures.map(|id| (Kind::IdConfidence(id), label(id))));
        }
        for (kind, name) in expected {
            let present = self.kinds.iter().any(|k| match (k, kind) {
                (Kind::Abundance(a), Kind::Abundance(b)) => *a == b,
                (Kind::IdConfidence(a), Kind::IdConfidence(b)) => *a == b,
                _ => false,
            });
            if !present {
                report.add(Code::MissingColumn, Site::Header(section), name);
            }
        }
        let mut highest = None;
        let mut out_of_order = false;
        let mut seen_opt = false;
        let mut opt_misplaced = false;
        for (i, kind) in self.kinds.iter().enumerate() {
            if let Some(r) = kind.rank() {
                if highest.is_some_and(|h| r < h) && !out_of_order {
                    out_of_order = true;
                    report.add(Code::ColumnOrder, Site::Column(section, i), Detail::None);
                }
                highest = highest.max(Some(r));
                if seen_opt && !opt_misplaced {
                    opt_misplaced = true;
                    report.add(
                        Code::OptColumnPosition,
                        Site::Column(section, i),
                        Detail::None,
                    );
                }
            }
            seen_opt |= matches!(kind, Kind::Opt(_));
            let dangling = match *kind {
                Kind::Abundance(AbundanceColumn::Assay(id)) | Kind::Opt(OptTarget::Assay(id)) => {
                    !m.assay.contains_key(id)
                }
                Kind::Abundance(
                    AbundanceColumn::StudyVariable(id)
                    | AbundanceColumn::VariationStudyVariable(id),
                )
                | Kind::Opt(OptTarget::StudyVariable(id)) => !m.study_variable.contains_key(id),
                Kind::IdConfidence(id) => !m.id_confidence_measure.contains_key(id),
                Kind::Opt(OptTarget::MsRun(id)) => !m.ms_run.contains_key(id),
                Kind::Opt(OptTarget::Global) | Kind::Stable(_) | Kind::Unknown => false,
            };
            if dangling {
                report.add(
                    Code::DanglingReference,
                    Site::Column(section, i),
                    Detail::None,
                );
            }
        }
    }
}

/// The header position of each column kind, to locate row cells.
struct Positions {
    numbers: Vec<usize>,
    opt: Vec<usize>,
}

impl Positions {
    fn of<S>(header: &Header<'_, S>) -> Self {
        let mut numbers = Vec::new();
        let mut opt = Vec::new();
        for (i, column) in header.columns.iter().enumerate() {
            match column {
                Column::Abundance(_) | Column::IdConfidence(_) => numbers.push(i),
                Column::Opt(_) => opt.push(i),
                Column::Stable(_) | Column::Unknown(_) => {}
            }
        }
        Self { numbers, opt }
    }
}

/// The header position of a stable column (or the end of the header, for
/// a value of a missing column).
pub(super) fn position<S: PartialEq>(header: &Header<'_, S>, column: &S) -> usize {
    header
        .columns
        .iter()
        .position(|c| matches!(c, Column::Stable(x) if x == column))
        .unwrap_or(header.columns.len())
}

/// Per-row reporting context.
struct Rows<'r, 'm, 'a, S> {
    report: &'r mut Report<'m, 'a>,
    section: Section,
    header: &'r Header<'r, S>,
    row: usize,
}

impl<S: PartialEq> Rows<'_, '_, '_, S> {
    fn at(&self, column: &S) -> Site {
        Site::Cell(self.section, self.row, position(self.header, column))
    }

    fn add(&mut self, code: Code, column: &S, detail: Detail) {
        let site = self.at(column);
        self.report.add(code, site, detail);
    }

    /// `null` in a column that is not nullable (and present in the header).
    fn not_null<T>(&mut self, cell: &Cell<'_, T>, column: &S, severity: Severity) {
        let in_header = position(self.header, column) < self.header.columns.len();
        if cell.is_null() && in_header {
            let site = self.at(column);
            self.report
                .add_at(Code::NullValue, severity, site, Detail::None);
        }
    }

    /// A value that failed to type, or a number in a forbidden form.
    fn valid<T>(&mut self, cell: &Cell<'_, T>, column: &S) {
        if cell.invalid().is_some() {
            self.add(Code::InvalidValue, column, Detail::None);
        }
    }

    fn valid_list<T>(&mut self, cell: &Cell<'_, List<'_, T>>, column: &S) {
        let items_invalid = cell
            .value()
            .is_some_and(|items| items.iter().any(|i| i.invalid().is_some()));
        if cell.invalid().is_some() || items_invalid {
            self.add(Code::InvalidValue, column, Detail::None);
        }
    }

    fn number(&mut self, cell: &Cell<'_, Decimal<'_>>, column: &S) {
        let forbidden = cell.value().is_some_and(|d| !d.kind().is_conforming());
        if cell.invalid().is_some() || forbidden {
            self.add(Code::InvalidValue, column, Detail::None);
        }
    }

    fn numbers(&mut self, cells: &[Cell<'_, Decimal<'_>>], positions: &[usize]) {
        for (cell, &at) in cells.iter().zip(positions) {
            let forbidden = cell.value().is_some_and(|d| !d.kind().is_conforming());
            if cell.invalid().is_some() || forbidden {
                let site = Site::Cell(self.section, self.row, at);
                self.report.add(Code::InvalidValue, site, Detail::None);
            }
        }
    }

    fn opts(&mut self, cells: &[Cell<'_, Text<'_>>], positions: &[usize]) {
        for (cell, &at) in cells.iter().zip(positions) {
            if cell.invalid().is_some() {
                let site = Site::Cell(self.section, self.row, at);
                self.report.add(Code::InvalidValue, site, Detail::None);
            }
        }
    }

    fn adduct(&mut self, cell: &Cell<'_, Text<'_>>, column: &S) {
        if cell.value().is_some_and(|t| !is_adduct(t.as_str())) {
            self.add(Code::AdductIon, column, Detail::None);
        }
    }
}

/// Whether `text` matches `^\[\d*M([+-][\w\d]+)*\]\d*[+-]$` (the SML and
/// SME pattern of the JSON Schema, used for SMF too; DECISIONS D-016).
fn is_adduct(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('[') else {
        return false;
    };
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let Some(mut rest) = rest.strip_prefix('M') else {
        return false;
    };
    loop {
        match rest.as_bytes().first() {
            Some(b'+' | b'-') => {
                let after = rest.get(1..).unwrap_or_default();
                let word = after.len()
                    - after
                        .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                        .len();
                if word == 0 {
                    return false;
                }
                rest = after.get(word..).unwrap_or_default();
            }
            Some(b']') => break,
            _ => return false,
        }
    }
    let charge = rest
        .get(1..)
        .unwrap_or_default()
        .trim_start_matches(|c: char| c.is_ascii_digit());
    charge == "+" || charge == "-"
}

/// Duplicate row ids, keyed by the id's text.
fn duplicates<'c, I: Ord + Copy + core::fmt::Display + 'c>(
    report: &mut Report<'_, '_>,
    section: Section,
    column: usize,
    ids: impl Iterator<Item = &'c Cell<'c, I>>,
) {
    let mut seen = BTreeSet::new();
    for (row, cell) in ids.enumerate() {
        if let Some(&id) = cell.value()
            && !seen.insert(id)
        {
            report.add(
                Code::DuplicateRowId,
                Site::Cell(section, row, column),
                label(id),
            );
        }
    }
}

/// The ids a list cell names, if any.
fn refs<I: Copy>(cell: &Cell<'_, List<'_, I>>) -> Vec<I> {
    cell.value().map_or_else(Vec::new, |items| {
        items.iter().filter_map(|c| c.value().copied()).collect()
    })
}

fn ids<I: Ord + Copy>(cells: impl Iterator<Item = I>) -> BTreeSet<I> {
    cells.collect()
}

fn profile(mztab: &MzTab<'_>) -> Option<Profile> {
    mztab.inferred_profile().ok()
}

#[expect(
    clippy::too_many_lines,
    reason = "one check per SML column, in column order"
)]
fn sml_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.sml else { return };
    let header = &table.header;
    let positions = Positions::of(header);
    let smf_ids = ids(mztab
        .smf
        .iter()
        .flat_map(|t| &t.rows)
        .filter_map(|r| r.smf_id.value().copied()));
    let has_smf = mztab.smf.is_some();
    let scheme = mztab
        .metadata
        .small_molecule_identification_reliability
        .as_ref();
    let default_scheme = scheme.is_none_or(Cell::is_null);
    let profile = profile(mztab);
    duplicates(
        report,
        Section::Sml,
        position(header, &SmlColumn::SmlId),
        table.rows.iter().map(|r| &r.sml_id),
    );
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: Section::Sml,
            header,
            row: i,
        };
        r.not_null(&row.sml_id, &SmlColumn::SmlId, Severity::Error);
        // DECISIONS D-012: nullable in practice, `recommended` in the schema.
        r.not_null(
            &row.best_id_confidence_value,
            &SmlColumn::BestIdConfidenceValue,
            Severity::Info,
        );
        r.valid(&row.sml_id, &SmlColumn::SmlId);
        r.valid_list(&row.smf_id_refs, &SmlColumn::SmfIdRefs);
        for (cell, column) in [
            (&row.database_identifier, SmlColumn::DatabaseIdentifier),
            (&row.chemical_formula, SmlColumn::ChemicalFormula),
            (&row.smiles, SmlColumn::Smiles),
            (&row.inchi, SmlColumn::Inchi),
            (&row.chemical_name, SmlColumn::ChemicalName),
            (&row.uri, SmlColumn::Uri),
            (&row.adduct_ions, SmlColumn::AdductIons),
        ] {
            r.valid_list(cell, &column);
        }
        let masses = &row.theoretical_neutral_mass;
        let forbidden = masses.value().is_some_and(|items| {
            items
                .iter()
                .any(|c| c.value().is_some_and(|d| !d.kind().is_conforming()))
        });
        r.valid_list(masses, &SmlColumn::TheoreticalNeutralMass);
        if forbidden && masses.invalid().is_none() {
            r.add(
                Code::InvalidValue,
                &SmlColumn::TheoreticalNeutralMass,
                Detail::None,
            );
        }
        r.valid(&row.reliability, &SmlColumn::Reliability);
        r.valid(
            &row.best_id_confidence_measure,
            &SmlColumn::BestIdConfidenceMeasure,
        );
        r.number(
            &row.best_id_confidence_value,
            &SmlColumn::BestIdConfidenceValue,
        );
        r.numbers(&row.abundance, &positions.numbers);
        r.opts(&row.opt, &positions.opt);
        for item in row.adduct_ions.value().into_iter().flatten() {
            r.adduct(item, &SmlColumn::AdductIons);
        }
        // DECISIONS D-015: a null reliability is a warning; a value outside
        // 1-4 is an error only under the default scheme.
        if row.reliability.is_null()
            && position(header, &SmlColumn::Reliability) < header.columns.len()
        {
            r.add(
                Code::ReliabilityMissing,
                &SmlColumn::Reliability,
                Detail::None,
            );
        }
        let level = row.reliability.value().map(Text::as_str);
        if default_scheme && level.is_some_and(|l| !matches!(l, "1" | "2" | "3" | "4")) {
            r.add(
                Code::ReliabilityValue,
                &SmlColumn::Reliability,
                Detail::None,
            );
        }
        for id in refs(&row.smf_id_refs) {
            if has_smf && !smf_ids.contains(&id) {
                r.add(Code::DanglingReference, &SmlColumn::SmfIdRefs, label(id));
            }
        }
        if rules == RuleSet::V2_1 {
            let has_refs = !row.smf_id_refs.is_null();
            let breaks = match profile {
                Some(Profile::MS) => has_refs,
                Some(Profile::MSFE) => !has_refs,
                _ => false,
            };
            if breaks {
                r.add(Code::CrossReference, &SmlColumn::SmfIdRefs, Detail::None);
            }
        }
    }
}

fn smf_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.smf else { return };
    let header = &table.header;
    let positions = Positions::of(header);
    let sme_ids: BTreeSet<SmeId> = ids(mztab
        .sme
        .iter()
        .flat_map(|t| &t.rows)
        .filter_map(|r| r.sme_id.value().copied()));
    let has_sme = mztab.sme.is_some();
    let profile = profile(mztab);
    duplicates(
        report,
        Section::Smf,
        position(header, &SmfColumn::SmfId),
        table.rows.iter().map(|r| &r.smf_id),
    );
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: Section::Smf,
            header,
            row: i,
        };
        r.not_null(&row.smf_id, &SmfColumn::SmfId, Severity::Error);
        r.not_null(
            &row.exp_mass_to_charge,
            &SmfColumn::ExpMassToCharge,
            Severity::Error,
        );
        r.valid(&row.smf_id, &SmfColumn::SmfId);
        r.valid_list(&row.sme_id_refs, &SmfColumn::SmeIdRefs);
        r.valid(
            &row.sme_id_ref_ambiguity_code,
            &SmfColumn::SmeIdRefAmbiguityCode,
        );
        r.valid(&row.adduct_ion, &SmfColumn::AdductIon);
        r.valid(&row.isotopomer, &SmfColumn::Isotopomer);
        r.valid(&row.charge, &SmfColumn::Charge);
        for (cell, column) in [
            (&row.exp_mass_to_charge, SmfColumn::ExpMassToCharge),
            (
                &row.retention_time_in_seconds,
                SmfColumn::RetentionTimeInSeconds,
            ),
            (
                &row.retention_time_in_seconds_start,
                SmfColumn::RetentionTimeInSecondsStart,
            ),
            (
                &row.retention_time_in_seconds_end,
                SmfColumn::RetentionTimeInSecondsEnd,
            ),
        ] {
            r.number(cell, &column);
        }
        r.numbers(&row.abundance, &positions.numbers);
        r.opts(&row.opt, &positions.opt);
        r.adduct(&row.adduct_ion, &SmfColumn::AdductIon);
        let refs = refs(&row.sme_id_refs);
        for &id in &refs {
            if has_sme && !sme_ids.contains(&id) {
                r.add(Code::DanglingReference, &SmfColumn::SmeIdRefs, label(id));
            }
        }
        let several = refs.len() > 1;
        if several == row.sme_id_ref_ambiguity_code.is_null() {
            r.add(
                Code::AmbiguityCode,
                &SmfColumn::SmeIdRefAmbiguityCode,
                Detail::None,
            );
        }
        if rules == RuleSet::V2_1 && profile == Some(Profile::MF) && !row.sme_id_refs.is_null() {
            r.add(Code::CrossReference, &SmfColumn::SmeIdRefs, Detail::None);
        }
    }
}

fn sme_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.sme else { return };
    let header = &table.header;
    let positions = Positions::of(header);
    let runs = &mztab.metadata.ms_run;
    let referenced: BTreeSet<SmeId> = mztab
        .smf
        .iter()
        .flat_map(|t| &t.rows)
        .flat_map(|row| refs(&row.sme_id_refs))
        .collect();
    let must_be_referenced = rules == RuleSet::V2_1 && mztab.smf.is_some();
    duplicates(
        report,
        Section::Sme,
        position(header, &SmeColumn::SmeId),
        table.rows.iter().map(|r| &r.sme_id),
    );
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: Section::Sme,
            header,
            row: i,
        };
        let error = Severity::Error;
        r.not_null(&row.sme_id, &SmeColumn::SmeId, error);
        r.not_null(&row.evidence_input_id, &SmeColumn::EvidenceInputId, error);
        r.not_null(&row.exp_mass_to_charge, &SmeColumn::ExpMassToCharge, error);
        r.not_null(&row.charge, &SmeColumn::Charge, error);
        r.not_null(
            &row.theoretical_mass_to_charge,
            &SmeColumn::TheoreticalMassToCharge,
            error,
        );
        r.not_null(&row.spectra_ref, &SmeColumn::SpectraRef, error);
        r.not_null(
            &row.identification_method,
            &SmeColumn::IdentificationMethod,
            error,
        );
        r.not_null(&row.ms_level, &SmeColumn::MsLevel, error);
        r.not_null(&row.rank, &SmeColumn::Rank, error);
        r.valid(&row.sme_id, &SmeColumn::SmeId);
        for (cell, column) in [
            (&row.evidence_input_id, SmeColumn::EvidenceInputId),
            (&row.database_identifier, SmeColumn::DatabaseIdentifier),
            (&row.chemical_formula, SmeColumn::ChemicalFormula),
            (&row.smiles, SmeColumn::Smiles),
            (&row.inchi, SmeColumn::Inchi),
            (&row.chemical_name, SmeColumn::ChemicalName),
            (&row.uri, SmeColumn::Uri),
            (&row.adduct_ion, SmeColumn::AdductIon),
        ] {
            r.valid(cell, &column);
        }
        for (cell, column) in [
            (&row.derivatized_form, SmeColumn::DerivatizedForm),
            (&row.identification_method, SmeColumn::IdentificationMethod),
            (&row.ms_level, SmeColumn::MsLevel),
        ] {
            r.valid(cell, &column);
        }
        sme_references(mztab, row, &mut r);
        r.valid(&row.charge, &SmeColumn::Charge);
        r.valid(&row.rank, &SmeColumn::Rank);
        r.number(&row.exp_mass_to_charge, &SmeColumn::ExpMassToCharge);
        r.number(
            &row.theoretical_mass_to_charge,
            &SmeColumn::TheoreticalMassToCharge,
        );
        r.valid_list(&row.spectra_ref, &SmeColumn::SpectraRef);
        r.numbers(&row.id_confidence, &positions.numbers);
        r.opts(&row.opt, &positions.opt);
        r.adduct(&row.adduct_ion, &SmeColumn::AdductIon);
        for spectrum in row
            .spectra_ref
            .value()
            .into_iter()
            .flatten()
            .filter_map(Cell::value)
        {
            if !runs.contains_key(spectrum.ms_run) {
                r.add(
                    Code::DanglingReference,
                    &SmeColumn::SpectraRef,
                    label(spectrum.ms_run),
                );
            }
        }
        // Every SME row MUST be referenced by an SMF row (profiles with
        // features and evidence).
        if must_be_referenced
            && row
                .sme_id
                .value()
                .is_some_and(|id| !referenced.contains(id))
        {
            r.add(Code::CrossReference, &SmeColumn::SmeId, Detail::None);
        }
    }
}

/// `colunit-*` entries name columns of their table.
fn colunits(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    fn labels<S: core::fmt::Display, R>(
        table: Option<&Table<'_, S, R>>,
    ) -> Option<Vec<alloc::string::String>> {
        use alloc::string::ToString as _;
        table.map(|t| {
            t.header
                .columns
                .iter()
                .map(|c| match c {
                    Column::Stable(s) => s.to_string(),
                    Column::Abundance(a) => a.to_string(),
                    Column::IdConfidence(i) => i.to_string(),
                    Column::Opt(o) => o.to_string(),
                    Column::Unknown(raw) => raw.as_str().into(),
                })
                .collect()
        })
    }
    let m = &mztab.metadata;
    let tables = [
        (
            MtdKey::ColunitSmallMolecule,
            &m.colunit_small_molecule,
            labels(mztab.sml.as_ref()),
        ),
        (
            MtdKey::ColunitSmallMoleculeFeature,
            &m.colunit_small_molecule_feature,
            labels(mztab.smf.as_ref()),
        ),
        (
            MtdKey::ColunitSmallMoleculeEvidence,
            &m.colunit_small_molecule_evidence,
            labels(mztab.sme.as_ref()),
        ),
    ];
    for (key, units, columns) in tables {
        let Some(columns) = columns else { continue };
        for (nth, unit) in units.iter().enumerate() {
            let Some(ColUnit { column, .. }) = unit.value() else {
                continue;
            };
            if !columns.iter().any(|c| c == column.as_str()) {
                report.add(Code::ColunitColumn, Site::Nth(key, nth), label(column));
            }
        }
    }
}

/// The keys of the native spectrum id of each `id_format` term in spec
/// table 1, and whether each value is a non-negative integer (`false`: an
/// id, `xsd:IDREF`).
const NATIVE_IDS: &[(&str, &[(&str, bool)])] = &[
    (
        "MS:1000768",
        &[
            ("controllerType", true),
            ("controllerNumber", true),
            ("scan", true),
        ],
    ),
    (
        "MS:1000769",
        &[("function", true), ("process", true), ("scan", true)],
    ),
    (
        "MS:1000770",
        &[
            ("sample", true),
            ("period", true),
            ("cycle", true),
            ("experiment", true),
        ],
    ),
    ("MS:1000771", &[("scan", true)]),
    ("MS:1000772", &[("scan", true)]),
    ("MS:1000773", &[("file", false)]),
    ("MS:1000774", &[("index", true)]),
    ("MS:1000775", &[("file", false)]),
    ("MS:1000776", &[("scan", true)]),
    ("MS:1000777", &[("spectrum", true)]),
];

/// Whether `reference` has the form spec table 1 gives for the `id_format`
/// term `accession`: space-separated `key=value` pairs, in order. `None`
/// for a term the table does not constrain (such as `MS:1001530`, any
/// string).
fn native_id_ok(accession: &str, reference: &str) -> Option<bool> {
    let (_, keys) = NATIVE_IDS.iter().find(|(a, _)| *a == accession)?;
    let parts: Vec<&str> = reference.split(' ').collect();
    Some(
        parts.len() == keys.len()
            && parts.iter().zip(keys.iter()).all(|(part, (key, number))| {
                part.split_once('=').is_some_and(|(k, v)| {
                    k == *key
                        && !v.is_empty()
                        && (!*number || v.bytes().all(|b| b.is_ascii_digit()))
                })
            }),
    )
}

/// An SME row's `spectra_ref` against its runs' `id_format`, and its URI.
fn sme_references(mztab: &MzTab<'_>, row: &SmeRow<'_>, r: &mut Rows<'_, '_, '_, SmeColumn>) {
    let refs = row.spectra_ref.value().into_iter().flatten();
    for reference in refs.filter_map(Cell::value) {
        let format = mztab
            .metadata
            .ms_run
            .get(reference.ms_run)
            .and_then(|run| run.id_format.as_ref())
            .and_then(Cell::value);
        let accession = match format {
            Some(Param::Cv { accession, .. }) => accession.as_str(),
            _ => continue,
        };
        if native_id_ok(accession, reference.reference.as_str()) == Some(false) {
            r.add(
                Code::SpectraRefFormat,
                &SmeColumn::SpectraRef,
                label(alloc::format!(
                    "{} ({accession})",
                    reference.reference.as_str()
                )),
            );
        }
    }
    if row
        .uri
        .value()
        .is_some_and(|u| !super::metadata::is_uri(u.as_str()))
    {
        r.add(Code::Uri, &SmeColumn::Uri, Detail::None);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::is_adduct;

    #[test]
    fn adducts() {
        for ok in [
            "[M+H]1+",
            "[M]+",
            "[2M+Na]1+",
            "[M-H2O+H]1+",
            "[M+2H]2+",
            "[M-H]-",
            "[M+NH4]+",
        ] {
            assert!(is_adduct(ok), "{ok}");
        }
        for bad in [
            "M+H", "[M+H]", "[M+]1+", "[X+H]1+", "[M+H]1", "[M+H]1+x", "[M*H]+", "[M+H",
        ] {
            assert!(!is_adduct(bad), "{bad}");
        }
    }
}
