//! Metadata rules: version, profile, mandatory and profile-dependent
//! fields, index gaps, references, study-variable groups, and field values.

use alloc::collections::BTreeMap;
use core::num::NonZeroU32;

use super::{Report, Site, label};
use crate::diag::{Code, Detail};
use crate::id::{
    AssayId, CvId, DatabaseId, InstrumentId, Ix, MsRunId, ProtocolId, SampleId,
    StudyVariableGroupId, StudyVariableId,
};
use crate::meta::{
    AssayKey, CvKey, DatabaseKey, MsRunKey, MtdKey, ProtocolKey, StudyVariableGroupKey,
    StudyVariableKey, ValueRef,
};
use crate::model::{Field, List, MzTab};
use crate::param::Param;
use crate::value::{Cell, Decimal, Text};
use crate::vocab::{Datatype, Profile, RuleSet};

pub(super) fn check(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    version(mztab, report);
    profile(mztab, rules, report);
    mandatory(mztab, rules, report);
    gaps(mztab, report);
    references(mztab, report);
    groups(mztab, report);
    values(mztab, report);
}

/// Whether a field holds something other than `null` (a value, or text
/// that failed to type, which the reader reports).
fn present<T>(field: &Field<'_, T>) -> bool {
    field.as_ref().is_some_and(|cell| !cell.is_null())
}

fn missing(report: &mut Report<'_, '_>, site: Site, key: MtdKey) {
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
                Site::Header(crate::vocab::Section::Sme),
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

/// Mandatory fields, by rule set, and profile-dependent fields.
#[expect(
    clippy::too_many_lines,
    reason = "one block per element: the mandatory set reads best as one list"
)]
fn mandatory(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    const ONE: NonZeroU32 = NonZeroU32::MIN;
    let m = &mztab.metadata;
    if m.assay.is_empty() {
        missing(
            report,
            Site::Document,
            MtdKey::Assay(ONE.into(), AssayKey::Name),
        );
    }
    for (n, assay) in m.assay.iter() {
        if !present(&assay.name) {
            missing(
                report,
                Site::Element("assay", n.get()),
                MtdKey::Assay(n, AssayKey::Name),
            );
        }
        if !present(&assay.ms_run_ref) {
            missing(
                report,
                Site::Element("assay", n.get()),
                MtdKey::Assay(n, AssayKey::MsRunRef),
            );
        }
    }
    if m.ms_run.is_empty() {
        missing(
            report,
            Site::Document,
            MtdKey::MsRun(ONE.into(), MsRunKey::Location),
        );
    }
    for (n, run) in m.ms_run.iter() {
        if !present(&run.location) {
            missing(
                report,
                Site::Element("ms_run", n.get()),
                MtdKey::MsRun(n, MsRunKey::Location),
            );
        }
        if rules == RuleSet::V2_0 && run.scan_polarity.is_empty() {
            missing(
                report,
                Site::Element("ms_run", n.get()),
                MtdKey::MsRun(n, MsRunKey::ScanPolarity(ONE.into())),
            );
        }
    }
    if m.study_variable.is_empty() {
        missing(
            report,
            Site::Document,
            MtdKey::StudyVariable(ONE.into(), StudyVariableKey::Name),
        );
    }
    for (n, sv) in m.study_variable.iter() {
        let site = Site::Element("study_variable", n.get());
        if !present(&sv.name) {
            missing(
                report,
                site,
                MtdKey::StudyVariable(n, StudyVariableKey::Name),
            );
        }
        if rules == RuleSet::V2_0 {
            if !present(&sv.assay_refs) {
                missing(
                    report,
                    site,
                    MtdKey::StudyVariable(n, StudyVariableKey::AssayRefs),
                );
            }
            if !present(&sv.description) {
                missing(
                    report,
                    site,
                    MtdKey::StudyVariable(n, StudyVariableKey::Description),
                );
            }
        }
    }
    if m.cv.is_empty() {
        missing(report, Site::Document, MtdKey::Cv(ONE.into(), CvKey::Label));
    }
    for (n, cv) in m.cv.iter() {
        let site = Site::Element("cv", n.get());
        for (field, key) in [
            (&cv.label, CvKey::Label),
            (&cv.full_name, CvKey::FullName),
            (&cv.version, CvKey::Version),
            (&cv.uri, CvKey::Uri),
        ] {
            if !present(field) {
                missing(report, site, MtdKey::Cv(n, key));
            }
        }
    }
    for (n, db) in m.database.iter() {
        let site = Site::Element("database", n.get());
        if !present(&db.param) {
            missing(report, site, MtdKey::Database(n, DatabaseKey::Database));
        }
        for (field, key) in [
            (&db.prefix, DatabaseKey::Prefix),
            (&db.version, DatabaseKey::Version),
            (&db.uri, DatabaseKey::Uri),
        ] {
            if !present(field) {
                missing(report, site, MtdKey::Database(n, key));
            }
        }
    }
    if m.software.is_empty() {
        missing(
            report,
            Site::Document,
            MtdKey::Software(ONE.into(), crate::meta::SoftwareKey::Software),
        );
    }
    for (n, protocol) in m.protocol.iter() {
        let site = Site::Element("protocol", n.get());
        if !present(&protocol.name) {
            missing(report, site, MtdKey::Protocol(n, ProtocolKey::Name));
        }
        if !present(&protocol.type_) {
            missing(report, site, MtdKey::Protocol(n, ProtocolKey::Type));
        }
    }
    for (n, group) in m.study_variable_group.iter() {
        if !present(&group.param) {
            let key = MtdKey::StudyVariableGroup(n, StudyVariableGroupKey::Group);
            missing(report, Site::Element("study_variable_group", n.get()), key);
        }
    }
    let (sml, smf, sme) = (
        mztab.sml.is_some(),
        mztab.smf.is_some(),
        mztab.sme.is_some(),
    );
    let identified = sme
        || mztab
            .sml
            .as_ref()
            .is_some_and(|t| t.rows.iter().any(|row| !row.database_identifier.is_null()));
    let has_measure = !m.id_confidence_measure.is_empty();
    let first_measure = MtdKey::IdConfidenceMeasure(ONE.into());
    let first_database = MtdKey::Database(ONE.into(), DatabaseKey::Database);
    // (field present, required by the tables, key, table present)
    let dependent = [
        (
            present(&m.quantification_method),
            MtdKey::QuantificationMethod,
            sml || smf,
        ),
        (
            present(&m.small_molecule_quantification_unit),
            MtdKey::SmallMoleculeQuantificationUnit,
            sml,
        ),
        (
            present(&m.small_molecule_feature_quantification_unit),
            MtdKey::SmallMoleculeFeatureQuantificationUnit,
            smf,
        ),
        (has_measure, first_measure, sme),
        (!m.database.is_empty(), first_database, identified),
    ];
    for (is_present, key, needed) in dependent {
        let needed = match rules {
            // 2.0 required these regardless of the tables (DECISIONS D-014).
            RuleSet::V2_0 => {
                needed || !matches!(key, MtdKey::SmallMoleculeFeatureQuantificationUnit)
            }
            RuleSet::V2_1 => needed,
        };
        if needed && !is_present {
            missing(report, Site::Document, key);
        } else if !needed && is_present && rules == RuleSet::V2_1 {
            report.add(Code::ProfileField, Site::Key(key), Detail::Key(key));
        }
    }
    if rules == RuleSet::V2_1 && m.study_variable_group.is_empty() {
        report.add(Code::NoStudyVariableGroup, Site::Document, Detail::None);
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
    gap!(software, crate::id::SoftwareId);
    gap!(publication, crate::id::PublicationId);
    gap!(contact, crate::id::ContactId);
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
    gap!(id_confidence_measure, crate::id::IdConfidenceMeasureId);
}

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

fn references(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
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
fn groups(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
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

/// Whether `text` matches `^[0-9]{4}-[0-9]{4}-[0-9]{4}-[0-9]{3}[0-9X]$`.
fn is_orcid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 19
        && bytes.iter().enumerate().all(|(i, &b)| match i {
            4 | 9 | 14 => b == b'-',
            18 => b.is_ascii_digit() || b == b'X',
            _ => b.is_ascii_digit(),
        })
}

/// Whether `text` is an RFC 3986 URI: a scheme, `:`, then only characters
/// RFC 3986 allows (a `%` only before two hex digits, at most one `#`).
pub(super) fn is_uri(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    let scheme_ok = scheme
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
    let bytes = rest.as_bytes();
    let chars_ok = bytes.iter().enumerate().all(|(i, &b)| match b {
        b'%' => bytes
            .get(i + 1..i + 3)
            .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)),
        // Unreserved, sub-delims and the delimiters of the URI syntax.
        b => b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?#[]".contains(&b),
    });
    scheme_ok && chars_ok && rest.matches('#').count() <= 1
}

/// Whether `text` looks like an e-mail address: `local@domain`, with a dot
/// inside the domain and no spaces.
fn is_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    let printable = |s: &str| s.bytes().all(|b| b.is_ascii_graphic() && b != b'@');
    !local.is_empty()
        && printable(local)
        && printable(domain)
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Whether `text` is an ISO 8601 date `YYYY-MM-DD`.
fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, &c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Whether `text` is `hh:mm:ss`, optionally with fractional seconds and a
/// zone (`Z` or `±hh:mm`).
fn is_time(text: &str) -> bool {
    let b = text.as_bytes();
    let Some(rest) = b.get(8..) else { return false };
    let clock = b.iter().take(8).enumerate().all(|(i, &c)| {
        if i == 2 || i == 5 {
            c == b':'
        } else {
            c.is_ascii_digit()
        }
    });
    let rest = match rest.first() {
        Some(b'.') => {
            let digits = rest
                .iter()
                .skip(1)
                .take_while(|c| c.is_ascii_digit())
                .count();
            if digits == 0 {
                return false;
            }
            rest.get(1 + digits..).unwrap_or_default()
        }
        _ => rest,
    };
    let zone = match rest {
        [] | [b'Z'] => true,
        [sign, h1, h2, b':', m1, m2] => {
            matches!(sign, b'+' | b'-') && [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit())
        }
        _ => false,
    };
    clock && zone
}

/// Whether a study-variable value matches its group's datatype.
fn matches_datatype(datatype: Datatype, value: &str) -> bool {
    match datatype {
        Datatype::String | Datatype::AnyUri => true,
        Datatype::Integer => value.parse::<i128>().is_ok() && !value.starts_with('+'),
        Datatype::Decimal => Decimal::parse(value).is_ok_and(|d| d.kind().is_conforming()),
        Datatype::Boolean => matches!(value, "true" | "false"),
        Datatype::Date => is_date(value),
        Datatype::Time => is_time(value),
        Datatype::DateTime => value
            .split_once('T')
            .is_some_and(|(date, time)| is_date(date) && is_time(time)),
        Datatype::Parameter => Param::parse(value).is_ok(),
    }
}

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
fn values(mztab: &MzTab<'_>, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    let mut seen = BTreeMap::<MtdKey, usize>::new();
    for (key, value) in m.entries() {
        let nth = seen.entry(key).or_default();
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
            let email = matches!(key, MtdKey::Contact(_, crate::meta::ContactKey::Email));
            if email && !is_email(text) {
                report.add(Code::Email, Site::Value(key), Detail::None);
            }
        }
        if has_invalid(value) {
            let site = if key.repeats() {
                Site::Nth(key, *nth)
            } else {
                Site::Value(key)
            };
            report.add(Code::InvalidValue, site, Detail::Key(key));
        }
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
            let key = MtdKey::Contact(n, crate::meta::ContactKey::Orcid);
            report.add(Code::Orcid, Site::Value(key), Detail::None);
        }
    }
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

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{is_date, is_orcid, is_time, matches_datatype};
    use crate::vocab::Datatype;

    #[test]
    fn patterns() {
        assert!(is_orcid("0000-0002-1825-009X"), "X check digit");
        for bad in [
            "0000-0002-1825-00",
            "0000-0002-1825-009x",
            "000000021825-0097",
            "https://orcid.org/0000-0002-1825-0097",
        ] {
            assert!(!is_orcid(bad), "{bad}");
        }
        assert!(
            is_date("2025-03-14") && !is_date("2025-3-14") && !is_date("2025/03/14"),
            "dates"
        );
        for ok in [
            "09:45:30",
            "09:45:30.250Z",
            "09:45:30+02:00",
            "09:45:30-02:00",
        ] {
            assert!(is_time(ok), "{ok}");
        }
        for bad in [
            "09:45",
            "09:45:30.",
            "09:45:30+0200",
            "09-45-30",
            "09:45:30Y",
            "09:45:30*02:00",
        ] {
            assert!(!is_time(bad), "{bad}");
        }
    }

    #[test]
    fn datatypes() {
        let cases = [
            (Datatype::String, "anything", true),
            (Datatype::AnyUri, "x", true),
            (Datatype::Integer, "-12", true),
            (Datatype::Integer, "+12", false),
            (Datatype::Integer, "1.0", false),
            (Datatype::Decimal, "1.5", true),
            (Datatype::Decimal, "1e5", false),
            (Datatype::Boolean, "true", true),
            (Datatype::Boolean, "TRUE", false),
            (Datatype::Date, "2025-03-14", true),
            (Datatype::Time, "09:45:30Z", true),
            (Datatype::DateTime, "2025-03-14T09:45:30", true),
            (Datatype::DateTime, "2025-03-14 09:45:30", false),
            (Datatype::Parameter, "[,,Male,]", true),
            (Datatype::Parameter, "Male", false),
        ];
        for (datatype, value, expected) in cases {
            assert_eq!(
                matches_datatype(datatype, value),
                expected,
                "{datatype} {value}"
            );
        }
    }
}
