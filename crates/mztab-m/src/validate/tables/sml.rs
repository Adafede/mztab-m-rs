//! Small molecule summary (`SML`) row validation.

use alloc::collections::BTreeSet;

use super::row::{Positions, Rows, duplicates, refs};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::diag::severity::Severity;
use crate::model::document::MzTab;
use crate::model::rows::SmlRow;
use crate::model::table::{Column, Header, SmfId, SmlColumn};
use crate::validate::label;
use crate::validate::report::Report;
use crate::value::cell::Cell;
use crate::value::text::Text;
use crate::vocab::profile::Profile;
use crate::vocab::section::TableSection;
use crate::vocab::version::RuleSet;

struct SmlCols {
    sml_id: usize,
    smf_id_refs: usize,
    database_identifier: usize,
    chemical_formula: usize,
    smiles: usize,
    inchi: usize,
    chemical_name: usize,
    uri: usize,
    theoretical_neutral_mass: usize,
    adduct_ions: usize,
    reliability: usize,
    best_id_confidence_measure: usize,
    best_id_confidence_value: usize,
}

impl SmlCols {
    fn of(header: &Header<'_, SmlColumn>) -> Self {
        let end = header.columns.len();
        let mut cols = Self {
            sml_id: end,
            smf_id_refs: end,
            database_identifier: end,
            chemical_formula: end,
            smiles: end,
            inchi: end,
            chemical_name: end,
            uri: end,
            theoretical_neutral_mass: end,
            adduct_ions: end,
            reliability: end,
            best_id_confidence_measure: end,
            best_id_confidence_value: end,
        };
        // Reverse scan lets the first (leftmost) occurrence overwrite later duplicates.
        for (i, column) in header.columns.iter().enumerate().rev() {
            let Column::Stable(c) = column else { continue };
            match c {
                SmlColumn::SmlId => cols.sml_id = i,
                SmlColumn::SmfIdRefs => cols.smf_id_refs = i,
                SmlColumn::DatabaseIdentifier => cols.database_identifier = i,
                SmlColumn::ChemicalFormula => cols.chemical_formula = i,
                SmlColumn::Smiles => cols.smiles = i,
                SmlColumn::Inchi => cols.inchi = i,
                SmlColumn::ChemicalName => cols.chemical_name = i,
                SmlColumn::Uri => cols.uri = i,
                SmlColumn::TheoreticalNeutralMass => cols.theoretical_neutral_mass = i,
                SmlColumn::AdductIons => cols.adduct_ions = i,
                SmlColumn::Reliability => cols.reliability = i,
                SmlColumn::BestIdConfidenceMeasure => cols.best_id_confidence_measure = i,
                SmlColumn::BestIdConfidenceValue => cols.best_id_confidence_value = i,
            }
        }
        cols
    }
}

struct SmlCtx {
    cols: SmlCols,
    positions: Positions,
    smf_ids: BTreeSet<SmfId>,
    has_smf: bool,
    default_scheme: bool,
    profile: Option<Profile>,
    rules: RuleSet,
}

pub(super) fn sml_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.sml else { return };
    let cols = SmlCols::of(&table.header);
    duplicates(
        report,
        TableSection::Sml,
        cols.sml_id,
        table.rows.iter().map(|r| &r.sml_id),
    );
    let smf_ids = mztab
        .smf
        .iter()
        .flat_map(|t| &t.rows)
        .filter_map(|r| r.smf_id.value().copied())
        .collect();
    let default_scheme = mztab
        .metadata
        .small_molecule_identification_reliability
        .as_ref()
        .is_none_or(Cell::is_null);
    let ctx = SmlCtx {
        cols,
        positions: Positions::of(&table.header),
        smf_ids,
        has_smf: mztab.smf.is_some(),
        default_scheme,
        profile: mztab.inferred_profile().ok(),
        rules,
    };
    let header_len = table.header.columns.len();
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: TableSection::Sml,
            header_len,
            row: i,
        };
        check_row(row, &ctx, &mut r);
    }
}

fn check_row(row: &SmlRow<'_>, ctx: &SmlCtx, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    r.not_null(&row.sml_id, c.sml_id, Severity::Error);
    // DECISIONS D-012: nullable in practice, `recommended` in the schema.
    r.not_null(
        &row.best_id_confidence_value,
        c.best_id_confidence_value,
        Severity::Info,
    );
    r.valid(&row.sml_id, c.sml_id);
    r.valid_list(&row.smf_id_refs, c.smf_id_refs);
    for (cell, col) in [
        (&row.database_identifier, c.database_identifier),
        (&row.chemical_formula, c.chemical_formula),
        (&row.smiles, c.smiles),
        (&row.inchi, c.inchi),
        (&row.chemical_name, c.chemical_name),
        (&row.uri, c.uri),
        (&row.adduct_ions, c.adduct_ions),
    ] {
        r.valid_list(cell, col);
    }
    check_masses_and_conf(row, ctx, r);
    check_reliability_and_refs(row, ctx, r);
}

fn check_masses_and_conf(row: &SmlRow<'_>, ctx: &SmlCtx, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    r.decimal_list(&row.theoretical_neutral_mass, c.theoretical_neutral_mass);
    r.valid(&row.reliability, c.reliability);
    r.valid(
        &row.best_id_confidence_measure,
        c.best_id_confidence_measure,
    );
    r.number(&row.best_id_confidence_value, c.best_id_confidence_value);
    r.numbers(&row.abundance, &ctx.positions.numbers);
    r.opts(&row.opt, &ctx.positions.opt);
    for item in row.adduct_ions.value().into_iter().flatten() {
        r.adduct(item, c.adduct_ions);
    }
}

fn check_reliability_and_refs(row: &SmlRow<'_>, ctx: &SmlCtx, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    // DECISIONS D-015: a null reliability is a warning; a value outside
    // 1-4 is an error only under the default scheme.
    if row.reliability.is_null() && c.reliability < r.header_len {
        r.add(Code::ReliabilityMissing, c.reliability, Detail::None);
    }
    let level = row.reliability.value().map(Text::as_str);
    if ctx.default_scheme && level.is_some_and(|l| !matches!(l, "1" | "2" | "3" | "4")) {
        r.add(Code::ReliabilityValue, c.reliability, Detail::None);
    }
    for id in refs(&row.smf_id_refs) {
        if ctx.has_smf && !ctx.smf_ids.contains(&id) {
            r.add(Code::DanglingReference, c.smf_id_refs, label(id));
        }
    }
    if ctx.rules == RuleSet::V2_1 {
        let has_refs = !row.smf_id_refs.is_null();
        let breaks = match ctx.profile {
            Some(Profile::MS) => has_refs,
            Some(Profile::MSFE) => !has_refs,
            _ => false,
        };
        if breaks {
            r.add(Code::CrossReference, c.smf_id_refs, Detail::None);
        }
    }
}
