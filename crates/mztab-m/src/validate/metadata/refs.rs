//! Metadata element cross-references and study-variable grouping rules.

use alloc::collections::BTreeMap;

use crate::diag::code::Code;
use crate::id::StudyVariableId;
use crate::meta::key::MtdKey;
use crate::meta::subkey::{AssayKey, MsRunKey, StudyVariableGroupKey, StudyVariableKey};
use crate::model::document::MzTab;
use crate::model::{Field, List};
use crate::validate::label;
use crate::validate::report::{Report, Site};

/// Reports each item of `list` that `defined` does not contain.
fn check_refs<I: Copy + core::fmt::Display>(
    report: &mut Report<'_, '_>,
    site: Site,
    list: &Field<'_, List<'_, I>>,
    defined: impl Fn(I) -> bool,
) {
    for item in list.iter().filter_map(|c| c.value()).flatten() {
        if let Some(&id) = item.value()
            && !defined(id)
        {
            report.add(Code::DanglingReference, site, label(id));
        }
    }
}

pub(super) fn references(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    for (n, run) in m.ms_run.iter() {
        if let Some(&id) = run.instrument_ref.as_ref().and_then(|c| c.value())
            && !m.instrument.contains_key(id)
        {
            report.add(
                Code::DanglingReference,
                Site::Value(MtdKey::MsRun(n, MsRunKey::InstrumentRef)),
                label(id),
            );
        }
    }
    for (n, assay) in m.assay.iter() {
        if let Some(&id) = assay.sample_ref.as_ref().and_then(|c| c.value())
            && !m.sample.contains_key(id)
        {
            report.add(
                Code::DanglingReference,
                Site::Value(MtdKey::Assay(n, AssayKey::SampleRef)),
                label(id),
            );
        }
        let runs = Site::Value(MtdKey::Assay(n, AssayKey::MsRunRef));
        check_refs(report, runs, &assay.ms_run_ref, |id| {
            m.ms_run.contains_key(id)
        });
        let protocols = Site::Value(MtdKey::Assay(n, AssayKey::ProtocolRefs));
        check_refs(report, protocols, &assay.protocol_refs, |id| {
            m.protocol.contains_key(id)
        });
    }
    for (n, sv) in m.study_variable.iter() {
        let assays = Site::Value(MtdKey::StudyVariable(n, StudyVariableKey::AssayRefs));
        check_refs(report, assays, &sv.assay_refs, |id| {
            m.assay.contains_key(id)
        });
        let runs = Site::Value(MtdKey::StudyVariable(n, StudyVariableKey::MsRunRefs));
        check_refs(report, runs, &sv.ms_run_refs, |id| {
            m.ms_run.contains_key(id)
        });
    }
    for (n, group) in m.study_variable_group.iter() {
        let site = Site::Value(MtdKey::StudyVariableGroup(
            n,
            StudyVariableGroupKey::StudyVariableRefs,
        ));
        check_refs(report, site, &group.study_variable_refs, |id| {
            m.study_variable.contains_key(id)
        });
    }
}

/// When groups exist, every study variable is in exactly one.
pub(super) fn groups(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    if m.study_variable_group.is_empty() {
        return;
    }
    let mut members = BTreeMap::<StudyVariableId, usize>::new();
    for group in m.study_variable_group.values() {
        let refs = group
            .study_variable_refs
            .iter()
            .filter_map(|c| c.value())
            .flatten();
        for id in refs.filter_map(|c| c.value()) {
            *members.entry(*id).or_default() += 1;
        }
    }
    for n in m.study_variable.keys() {
        if members.get(&n).copied().unwrap_or_default() != 1 {
            report.add(
                Code::StudyVariableGrouping,
                Site::Element("study_variable", n.get()),
                label(n),
            );
        }
    }
}
