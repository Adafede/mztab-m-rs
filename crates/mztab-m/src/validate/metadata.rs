//! Metadata validation: version, profile, mandatory/profile-dependent fields,
//! contiguous index gaps, references, study-variable groups, and values.

mod mandatory;
mod refs;
mod values;

use super::{Report, Site, label};
use crate::diag::{Code, Detail};
use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, InstrumentId, Ix, MsRunId,
    ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId, StudyVariableId,
};
use crate::meta::MtdKey;
use crate::model::{Field, MzTab};
use crate::vocab::{Profile, RuleSet};

pub(super) fn check(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    version(mztab, report);
    profile(mztab, rules, report);
    mandatory::check(mztab, rules, report);
    gaps(mztab, report);
    refs::references(mztab, report);
    refs::groups(mztab, report);
    values::check(mztab, report);
}

/// Whether a field holds something other than `null` (a value, or text
/// that failed to type, which the reader reports).
pub(super) fn present<T>(field: &Field<'_, T>) -> bool {
    field.as_ref().is_some_and(|cell| !cell.is_null())
}

pub(super) fn missing(report: &mut Report<'_, '_>, site: Site, key: MtdKey) {
    report.add(Code::MissingField, site, Detail::Key(key));
}

fn version(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    match mztab.metadata.version.as_ref() {
        None => missing(report, Site::Document, MtdKey::Version),
        Some(cell) => {
            if cell.value().is_some_and(|v| !v.is_known()) {
                report.add(
                    Code::UnknownVersion,
                    Site::Value(MtdKey::Version),
                    Detail::None,
                );
            }
        }
    }
    if !present(&mztab.metadata.id) {
        missing(report, Site::Document, MtdKey::Id);
    }
}

fn profile(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let declared = mztab
        .metadata
        .profile
        .as_ref()
        .and_then(|cell| cell.value())
        .copied();
    let inferred = mztab.inferred_profile();
    if rules == RuleSet::V2_0 {
        // Profiles were introduced in 2.1: a 2.0 file only learns its
        // profile (DECISIONS J-12).
        if let Ok(profile) = inferred {
            report.add(
                Code::ProfileInferred,
                Site::Document,
                Detail::Profile(profile),
            );
        }
        return;
    }
    match (declared, inferred) {
        (_, Err(tables)) => {
            report.add(
                Code::ProhibitedProfile,
                Site::Header(crate::vocab::TableSection::Sme),
                Detail::Prohibited(tables),
            );
        }
        (None, Ok(profile)) => {
            report.add(
                Code::ProfileInferred,
                Site::Document,
                Detail::Profile(profile),
            );
            if profile == Profile::MSF {
                report.add(
                    Code::DiscouragedProfile,
                    Site::Document,
                    Detail::Profile(profile),
                );
            }
        }
        (Some(declared), Ok(profile)) => {
            if declared != profile {
                report.add(
                    Code::ProfileMismatch,
                    Site::Value(MtdKey::Profile),
                    Detail::Profile(profile),
                );
            }
        }
    }
}

fn check_gap(
    report: &mut Report<'_, '_>,
    name: &'static str,
    indices: &mut dyn Iterator<Item = u32>,
) {
    if let Some((index, expected)) = indices.zip(1_u32..).find(|(i, e)| i != e) {
        report.add(
            Code::IndexGap,
            Site::Element(name, index),
            label(alloc::format!("{name}[{expected}]")),
        );
    }
}

/// Element indices that are not `1..=n`.
fn gaps(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    macro_rules! gap {
        ($field:ident, $id:ty) => {
            check_gap(
                report,
                stringify!($field),
                &mut m.$field.keys().map(<$id>::get),
            );
        };
    }
    gap!(sample_processing, Ix);
    gap!(instrument, InstrumentId);
    gap!(software, SoftwareId);
    gap!(publication, PublicationId);
    gap!(contact, ContactId);
    gap!(uri, Ix);
    gap!(external_study_uri, Ix);
    gap!(sample, SampleId);
    gap!(ms_run, MsRunId);
    gap!(assay, AssayId);
    gap!(study_variable, StudyVariableId);
    gap!(study_variable_group, StudyVariableGroupId);
    gap!(protocol, ProtocolId);
    gap!(custom, Ix);
    gap!(cv, CvId);
    gap!(database, DatabaseId);
    gap!(derivatization_agent, Ix);
    gap!(id_confidence_measure, IdConfidenceMeasureId);
}
