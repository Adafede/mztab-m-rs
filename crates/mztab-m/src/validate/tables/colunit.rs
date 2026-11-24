//! `colunit-*` validation against table headers.

use alloc::string::ToString as _;
use alloc::vec::Vec;

use crate::diag::code::Code;
use crate::meta::key::MtdKey;
use crate::model::colunit::ColUnit;
use crate::model::document::MzTab;
use crate::model::table::{Column, Table};
use crate::validate::label;
use crate::validate::report::{Report, Site};
use crate::value::cell::Cell;

/// `colunit-*` entries name columns of their table.
pub(super) fn check(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    check_table(
        MtdKey::ColunitSmallMolecule,
        &m.colunit_small_molecule,
        mztab.sml.as_ref(),
        report,
    );
    check_table(
        MtdKey::ColunitSmallMoleculeFeature,
        &m.colunit_small_molecule_feature,
        mztab.smf.as_ref(),
        report,
    );
    check_table(
        MtdKey::ColunitSmallMoleculeEvidence,
        &m.colunit_small_molecule_evidence,
        mztab.sme.as_ref(),
        report,
    );
}

fn check_table<S: core::fmt::Display, R>(
    key: MtdKey,
    units: &[Cell<'_, ColUnit<'_>>],
    table: Option<&Table<'_, S, R>>,
    report: &mut Report<'_, '_>,
) {
    let Some(table) = table.filter(|_| !units.is_empty()) else {
        return;
    };
    let columns: Vec<alloc::string::String> = table
        .header
        .columns
        .iter()
        .map(|c| match c {
            Column::Stable(s) => s.to_string(),
            Column::Abundance(a) => a.to_string(),
            Column::IdConfidence(i) => i.to_string(),
            Column::Opt(o) => o.to_string(),
            Column::Unknown(raw) => raw.as_str().into(),
        })
        .collect();
    for (nth, unit) in units.iter().enumerate() {
        let Some(ColUnit { column, .. }) = unit.value() else {
            continue;
        };
        if !columns.iter().any(|c| c == column.as_str()) {
            report.add(Code::ColunitColumn, Site::Nth(key, nth), label(column));
        }
    }
}
