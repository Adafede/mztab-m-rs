//! Table rules: empty tables, columns, references, row ids, nulls, values,
//! reliability, adducts, ambiguity codes and profile cross-references.

mod colunit;
mod row;
mod shape;
mod sme;
mod smf;
mod sml;

use super::report::Report;
use crate::model::document::MzTab;
use crate::model::table::{SmeColumn, SmfColumn, SmlColumn};
use crate::vocab::section::TableSection;
use crate::vocab::version::RuleSet;

pub(super) use row::position;

pub(super) fn check(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let declared = mztab
        .metadata
        .profile
        .as_ref()
        .and_then(|c| c.value())
        .copied();
    if let Some(t) = &mztab.sml {
        shape::table_shape(
            mztab,
            declared,
            TableSection::Sml,
            t,
            SmlColumn::ALL,
            report,
        );
    }
    if let Some(t) = &mztab.smf {
        shape::table_shape(
            mztab,
            declared,
            TableSection::Smf,
            t,
            SmfColumn::ALL,
            report,
        );
    }
    if let Some(t) = &mztab.sme {
        shape::table_shape(
            mztab,
            declared,
            TableSection::Sme,
            t,
            SmeColumn::ALL,
            report,
        );
    }
    sml::sml_rows(mztab, rules, report);
    smf::smf_rows(mztab, rules, report);
    sme::sme_rows(mztab, rules, report);
    colunit::check(mztab, report);
}
