//! Small molecule evidence (`SME`) row validation.

use alloc::collections::BTreeSet;

use super::row::{Positions, Rows, duplicates, refs};
use crate::diag::{Code, Detail, Severity};
use crate::id::MsRunId;
use crate::model::{Column, Header, MzTab, SmeColumn, SmeId, SmeRow};
use crate::param::Param;
use crate::validate::patterns::{is_uri, native_id_ok};
use crate::validate::{Report, label};
use crate::value::Cell;
use crate::vocab::{RuleSet, TableSection};

struct SmeCols {
    sme_id: usize,
    evidence_input_id: usize,
    database_identifier: usize,
    chemical_formula: usize,
    smiles: usize,
    inchi: usize,
    chemical_name: usize,
    uri: usize,
    derivatized_form: usize,
    adduct_ion: usize,
    exp_mass_to_charge: usize,
    charge: usize,
    theoretical_mass_to_charge: usize,
    spectra_ref: usize,
    identification_method: usize,
    ms_level: usize,
    rank: usize,
}

impl SmeCols {
    fn of(header: &Header<'_, SmeColumn>) -> Self {
        let end = header.columns.len();
        let mut cols = Self {
            sme_id: end,
            evidence_input_id: end,
            database_identifier: end,
            chemical_formula: end,
            smiles: end,
            inchi: end,
            chemical_name: end,
            uri: end,
            derivatized_form: end,
            adduct_ion: end,
            exp_mass_to_charge: end,
            charge: end,
            theoretical_mass_to_charge: end,
            spectra_ref: end,
            identification_method: end,
            ms_level: end,
            rank: end,
        };
        for (i, column) in header.columns.iter().enumerate().rev() {
            let Column::Stable(c) = column else { continue };
            match c {
                SmeColumn::SmeId => cols.sme_id = i,
                SmeColumn::EvidenceInputId => cols.evidence_input_id = i,
                SmeColumn::DatabaseIdentifier => cols.database_identifier = i,
                SmeColumn::ChemicalFormula => cols.chemical_formula = i,
                SmeColumn::Smiles => cols.smiles = i,
                SmeColumn::Inchi => cols.inchi = i,
                SmeColumn::ChemicalName => cols.chemical_name = i,
                SmeColumn::Uri => cols.uri = i,
                SmeColumn::DerivatizedForm => cols.derivatized_form = i,
                SmeColumn::AdductIon => cols.adduct_ion = i,
                SmeColumn::ExpMassToCharge => cols.exp_mass_to_charge = i,
                SmeColumn::Charge => cols.charge = i,
                SmeColumn::TheoreticalMassToCharge => cols.theoretical_mass_to_charge = i,
                SmeColumn::SpectraRef => cols.spectra_ref = i,
                SmeColumn::IdentificationMethod => cols.identification_method = i,
                SmeColumn::MsLevel => cols.ms_level = i,
                SmeColumn::Rank => cols.rank = i,
            }
        }
        cols
    }
}

struct SmeCtx<'m, 'a> {
    mztab: &'m MzTab<'a>,
    cols: SmeCols,
    positions: Positions,
    referenced: BTreeSet<SmeId>,
    must_be_referenced: bool,
}

pub(super) fn sme_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.sme else { return };
    let cols = SmeCols::of(&table.header);
    duplicates(
        report,
        TableSection::Sme,
        cols.sme_id,
        table.rows.iter().map(|r| &r.sme_id),
    );
    let referenced = mztab
        .smf
        .iter()
        .flat_map(|t| &t.rows)
        .flat_map(|row| refs(&row.sme_id_refs))
        .collect();
    let ctx = SmeCtx {
        mztab,
        cols,
        positions: Positions::of(&table.header),
        referenced,
        must_be_referenced: rules == RuleSet::V2_1 && mztab.smf.is_some(),
    };
    let header_len = table.header.columns.len();
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: TableSection::Sme,
            header_len,
            row: i,
        };
        check_row(row, &ctx, &mut r);
    }
}

fn check_row(row: &SmeRow<'_>, ctx: &SmeCtx<'_, '_>, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    let error = Severity::Error;
    for (is_null, col) in [
        (row.sme_id.is_null(), c.sme_id),
        (row.evidence_input_id.is_null(), c.evidence_input_id),
        (row.exp_mass_to_charge.is_null(), c.exp_mass_to_charge),
        (row.charge.is_null(), c.charge),
        (
            row.theoretical_mass_to_charge.is_null(),
            c.theoretical_mass_to_charge,
        ),
        (row.spectra_ref.is_null(), c.spectra_ref),
        (row.identification_method.is_null(), c.identification_method),
        (row.ms_level.is_null(), c.ms_level),
        (row.rank.is_null(), c.rank),
    ] {
        if is_null && col < r.header_len {
            r.report.add_at(
                Code::NullValue,
                error,
                crate::validate::Site::Cell(r.section, r.row, col),
                Detail::None,
            );
        }
    }
    r.valid(&row.sme_id, c.sme_id);
    for (cell, col) in [
        (&row.evidence_input_id, c.evidence_input_id),
        (&row.database_identifier, c.database_identifier),
        (&row.chemical_formula, c.chemical_formula),
        (&row.smiles, c.smiles),
        (&row.inchi, c.inchi),
        (&row.chemical_name, c.chemical_name),
        (&row.uri, c.uri),
        (&row.adduct_ion, c.adduct_ion),
    ] {
        r.valid(cell, col);
    }
    for (cell, col) in [
        (&row.derivatized_form, c.derivatized_form),
        (&row.identification_method, c.identification_method),
        (&row.ms_level, c.ms_level),
    ] {
        r.valid(cell, col);
    }
    check_row_tail(row, ctx, r);
}

fn check_row_tail(row: &SmeRow<'_>, ctx: &SmeCtx<'_, '_>, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    let runs = &ctx.mztab.metadata.ms_run;
    sme_references(ctx.mztab, c, row, r);
    r.valid(&row.charge, c.charge);
    r.valid(&row.rank, c.rank);
    r.number(&row.exp_mass_to_charge, c.exp_mass_to_charge);
    r.number(
        &row.theoretical_mass_to_charge,
        c.theoretical_mass_to_charge,
    );
    r.valid_list(&row.spectra_ref, c.spectra_ref);
    r.numbers(&row.id_confidence, &ctx.positions.numbers);
    r.opts(&row.opt, &ctx.positions.opt);
    r.adduct(&row.adduct_ion, c.adduct_ion);
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
                c.spectra_ref,
                label(spectrum.ms_run),
            );
        }
    }
    // Every SME row MUST be referenced by an SMF row (profiles with
    // features and evidence).
    if ctx.must_be_referenced
        && row
            .sme_id
            .value()
            .is_some_and(|id| !ctx.referenced.contains(id))
    {
        r.add(Code::CrossReference, c.sme_id, Detail::None);
    }
}

/// An SME row's `spectra_ref` against its runs' `id_format`, and its URI.
fn sme_references(mztab: &MzTab<'_>, cols: &SmeCols, row: &SmeRow<'_>, r: &mut Rows<'_, '_, '_>) {
    let refs = row.spectra_ref.value().into_iter().flatten();
    for reference in refs.filter_map(Cell::value) {
        let format = run_id_format(mztab, reference.ms_run);
        let Some(Param::Cv { accession, .. }) = format else {
            continue;
        };
        if native_id_ok(accession.as_str(), reference.reference.as_str()) == Some(false) {
            r.add(
                Code::SpectraRefFormat,
                cols.spectra_ref,
                label(alloc::format!(
                    "{} ({})",
                    reference.reference.as_str(),
                    accession.as_str()
                )),
            );
        }
    }
    if row.uri.value().is_some_and(|u| !is_uri(u.as_str())) {
        r.add(Code::Uri, cols.uri, Detail::None);
    }
}

fn run_id_format<'m, 'a>(mztab: &'m MzTab<'a>, id: MsRunId) -> Option<&'m Param<'a>> {
    mztab
        .metadata
        .ms_run
        .get(id)
        .and_then(|run| run.id_format.as_ref())
        .and_then(Cell::value)
}
