//! Small molecule feature (`SMF`) row validation.

use alloc::collections::BTreeSet;

use super::row::{Positions, Rows, duplicates, refs};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::diag::severity::Severity;
use crate::model::document::MzTab;
use crate::model::rows::SmfRow;
use crate::model::table::{Column, Header, SmeId, SmfColumn};
use crate::validate::label;
use crate::validate::report::Report;
use crate::vocab::profile::Profile;
use crate::vocab::section::TableSection;
use crate::vocab::version::RuleSet;

struct SmfCols {
    smf_id: usize,
    sme_id_refs: usize,
    sme_id_ref_ambiguity_code: usize,
    adduct_ion: usize,
    isotopomer: usize,
    exp_mass_to_charge: usize,
    charge: usize,
    retention_time_in_seconds: usize,
    retention_time_in_seconds_start: usize,
    retention_time_in_seconds_end: usize,
}

impl SmfCols {
    fn of(header: &Header<'_, SmfColumn>) -> Self {
        let end = header.columns.len();
        let mut cols = Self {
            smf_id: end,
            sme_id_refs: end,
            sme_id_ref_ambiguity_code: end,
            adduct_ion: end,
            isotopomer: end,
            exp_mass_to_charge: end,
            charge: end,
            retention_time_in_seconds: end,
            retention_time_in_seconds_start: end,
            retention_time_in_seconds_end: end,
        };
        for (i, column) in header.columns.iter().enumerate().rev() {
            let Column::Stable(c) = column else { continue };
            match c {
                SmfColumn::SmfId => cols.smf_id = i,
                SmfColumn::SmeIdRefs => cols.sme_id_refs = i,
                SmfColumn::SmeIdRefAmbiguityCode => cols.sme_id_ref_ambiguity_code = i,
                SmfColumn::AdductIon => cols.adduct_ion = i,
                SmfColumn::Isotopomer => cols.isotopomer = i,
                SmfColumn::ExpMassToCharge => cols.exp_mass_to_charge = i,
                SmfColumn::Charge => cols.charge = i,
                SmfColumn::RetentionTimeInSeconds => cols.retention_time_in_seconds = i,
                SmfColumn::RetentionTimeInSecondsStart => cols.retention_time_in_seconds_start = i,
                SmfColumn::RetentionTimeInSecondsEnd => cols.retention_time_in_seconds_end = i,
            }
        }
        cols
    }
}

struct SmfCtx {
    cols: SmfCols,
    positions: Positions,
    sme_ids: BTreeSet<SmeId>,
    has_sme: bool,
    profile: Option<Profile>,
    rules: RuleSet,
}

pub(super) fn smf_rows(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let Some(table) = &mztab.smf else { return };
    let cols = SmfCols::of(&table.header);
    duplicates(
        report,
        TableSection::Smf,
        cols.smf_id,
        table.rows.iter().map(|r| &r.smf_id),
    );
    let sme_ids = mztab
        .sme
        .iter()
        .flat_map(|t| &t.rows)
        .filter_map(|r| r.sme_id.value().copied())
        .collect();
    let ctx = SmfCtx {
        cols,
        positions: Positions::of(&table.header),
        sme_ids,
        has_sme: mztab.sme.is_some(),
        profile: mztab.inferred_profile().ok(),
        rules,
    };
    let header_len = table.header.columns.len();
    for (i, row) in table.rows.iter().enumerate() {
        let mut r = Rows {
            report,
            section: TableSection::Smf,
            header_len,
            row: i,
        };
        check_row(row, &ctx, &mut r);
    }
}

fn check_row(row: &SmfRow<'_>, ctx: &SmfCtx, r: &mut Rows<'_, '_, '_>) {
    let c = &ctx.cols;
    r.not_null(&row.smf_id, c.smf_id, Severity::Error);
    r.not_null(
        &row.exp_mass_to_charge,
        c.exp_mass_to_charge,
        Severity::Error,
    );
    r.valid(&row.smf_id, c.smf_id);
    r.valid_list(&row.sme_id_refs, c.sme_id_refs);
    r.valid(&row.sme_id_ref_ambiguity_code, c.sme_id_ref_ambiguity_code);
    r.valid(&row.adduct_ion, c.adduct_ion);
    r.valid(&row.isotopomer, c.isotopomer);
    r.valid(&row.charge, c.charge);
    for (cell, col) in [
        (&row.exp_mass_to_charge, c.exp_mass_to_charge),
        (&row.retention_time_in_seconds, c.retention_time_in_seconds),
        (
            &row.retention_time_in_seconds_start,
            c.retention_time_in_seconds_start,
        ),
        (
            &row.retention_time_in_seconds_end,
            c.retention_time_in_seconds_end,
        ),
    ] {
        r.number(cell, col);
    }
    r.numbers(&row.abundance, &ctx.positions.numbers);
    r.opts(&row.opt, &ctx.positions.opt);
    r.adduct(&row.adduct_ion, c.adduct_ion);
    let mut ref_count = 0_usize;
    for id in refs(&row.sme_id_refs) {
        ref_count += 1;
        if ctx.has_sme && !ctx.sme_ids.contains(&id) {
            r.add(Code::DanglingReference, c.sme_id_refs, label(id));
        }
    }
    let several = ref_count > 1;
    if c.sme_id_ref_ambiguity_code < r.header_len
        && several == row.sme_id_ref_ambiguity_code.is_null()
    {
        r.add(
            Code::AmbiguityCode,
            c.sme_id_ref_ambiguity_code,
            Detail::None,
        );
    }
    if ctx.rules == RuleSet::V2_1 && ctx.profile == Some(Profile::MF) && !row.sme_id_refs.is_null()
    {
        r.add(Code::CrossReference, c.sme_id_refs, Detail::None);
    }
}
