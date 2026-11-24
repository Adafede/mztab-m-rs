//! Metadata field value validation: invalid cells, URIs, e-mails, ORCIDs,
//! hash methods, and study-variable datatypes.

use alloc::collections::BTreeMap;

use super::present;
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::meta::key::MtdKey;
use crate::meta::subkey::{AssayKey, ContactKey, CvKey, MsRunKey, StudyVariableKey};
use crate::model::List;
use crate::model::document::MzTab;
use crate::model::meta_value::ValueRef;
use crate::validate::label;
use crate::validate::patterns::{is_email, is_orcid, is_uri, matches_datatype};
use crate::validate::report::{Report, Site};
use crate::value::cell::Cell;
use crate::value::text::Text;

/// Whether a value failed to type anywhere (the cell or a list item).
fn has_invalid(value: ValueRef<'_, '_>) -> bool {
    const fn cell<T>(c: &Cell<'_, T>) -> bool {
        c.invalid().is_some()
    }
    fn list<T>(c: &Cell<'_, List<'_, T>>) -> bool {
        cell(c) || c.value().is_some_and(|items| items.iter().any(cell))
    }
    match value {
        ValueRef::Version(c) => cell(c),
        ValueRef::Profile(c) => cell(c),
        ValueRef::Datatype(c) => cell(c),
        ValueRef::Text(c) => cell(c),
        ValueRef::Param(c) => cell(c),
        ValueRef::InstrumentRef(c) => cell(c),
        ValueRef::SampleRef(c) => cell(c),
        ValueRef::ColUnit(c) => cell(c),
        ValueRef::ParamList(c) => list(c),
        ValueRef::Publication(c) => list(c),
        ValueRef::MsRunRefs(c) => list(c),
        ValueRef::ProtocolRefs(c) => list(c),
        ValueRef::AssayRefs(c) => list(c),
        ValueRef::StudyVariableRefs(c) => list(c),
    }
}

/// Field values: invalid cells, hash methods, ORCIDs, datatypes.
pub(super) fn check(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    let mut seen = BTreeMap::<MtdKey, usize>::new();
    for (key, value) in m.entries() {
        let nth = seen.entry(key).or_default();
        check_entry_value(key, value, *nth, report);
        *nth += 1;
    }
    for (n, run) in m.ms_run.iter() {
        if present(&run.hash) && !present(&run.hash_method) {
            report.add(
                Code::HashMethod,
                Site::Key(MtdKey::MsRun(n, MsRunKey::Hash)),
                Detail::None,
            );
        }
    }
    for (n, contact) in m.contact.iter() {
        let orcid = contact.orcid.as_ref().and_then(|c| c.value());
        if orcid.is_some_and(|text| !is_orcid(text.as_str())) {
            let key = MtdKey::Contact(n, ContactKey::Orcid);
            report.add(Code::Orcid, Site::Value(key), Detail::None);
        }
    }
    check_study_variable_datatypes(mztab, report);
}

fn check_entry_value(
    key: MtdKey,
    value: ValueRef<'_, '_>,
    nth: usize,
    report: &mut Report<'_, '_>,
) {
    if let ValueRef::Text(Cell::Value(text)) = value {
        let text = text.as_str();
        let uri = matches!(
            key,
            MtdKey::Uri(_)
                | MtdKey::ExternalStudyUri(_)
                | MtdKey::MsRun(_, MsRunKey::Location)
                | MtdKey::Assay(_, AssayKey::ExternalUri)
                | MtdKey::Cv(_, CvKey::Uri)
        );
        if uri && !is_uri(text) {
            report.add(Code::Uri, Site::Value(key), Detail::None);
        }
        if matches!(key, MtdKey::Contact(_, ContactKey::Email)) && !is_email(text) {
            report.add(Code::Email, Site::Value(key), Detail::None);
        }
    }
    if has_invalid(value) {
        let site = if key.repeats() {
            Site::Nth(key, nth)
        } else {
            Site::Value(key)
        };
        report.add(Code::InvalidValue, site, Detail::Key(key));
    }
}

fn check_study_variable_datatypes(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    for group in m.study_variable_group.values() {
        let Some(&datatype) = group.datatype.as_ref().and_then(|c| c.value()) else {
            continue;
        };
        let refs = group
            .study_variable_refs
            .iter()
            .filter_map(|c| c.value())
            .flatten();
        for &id in refs.filter_map(|c| c.value()) {
            let name = m
                .study_variable
                .get(id)
                .and_then(|sv| sv.name.as_ref())
                .and_then(|c| c.value());
            if name.is_some_and(|text: &Text<'_>| !matches_datatype(datatype, text.as_str())) {
                let key = MtdKey::StudyVariable(id, StudyVariableKey::Name);
                report.add(
                    Code::StudyVariableDatatype,
                    Site::Value(key),
                    label(datatype),
                );
            }
        }
    }
}
