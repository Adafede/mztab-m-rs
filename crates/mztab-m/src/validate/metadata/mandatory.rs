//! Mandatory metadata elements, element fields, and profile-dependent fields.

use core::num::NonZeroU32;

use super::{missing, present};
use crate::diag::{Code, Detail};
use crate::meta::{
    AssayKey, CvKey, DatabaseKey, MsRunKey, MtdKey, ProtocolKey, SoftwareKey,
    StudyVariableGroupKey, StudyVariableKey,
};
use crate::model::{Metadata, MzTab};
use crate::validate::{Report, Site};
use crate::vocab::RuleSet;

const ONE: NonZeroU32 = NonZeroU32::MIN;

pub(super) fn check(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
    mandatory_core_elements(m, rules, report);
    mandatory_secondary_elements(m, report);
    profile_dependent(mztab, rules, report);
}

fn mandatory_core_elements(m: &Metadata<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    if m.assay.is_empty() {
        missing(
            report,
            Site::Document,
            MtdKey::Assay(ONE.into(), AssayKey::Name),
        );
    }
    for (n, assay) in m.assay.iter() {
        let site = Site::Element("assay", n.get());
        if !present(&assay.name) {
            missing(report, site, MtdKey::Assay(n, AssayKey::Name));
        }
        if !present(&assay.ms_run_ref) {
            missing(report, site, MtdKey::Assay(n, AssayKey::MsRunRef));
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
        let site = Site::Element("ms_run", n.get());
        if !present(&run.location) {
            missing(report, site, MtdKey::MsRun(n, MsRunKey::Location));
        }
        if rules == RuleSet::V2_0 && run.scan_polarity.is_empty() {
            missing(
                report,
                site,
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
}

fn mandatory_secondary_elements(m: &Metadata<'_>, report: &mut Report<'_, '_>) {
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
            MtdKey::Software(ONE.into(), SoftwareKey::Software),
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
}

fn profile_dependent(mztab: &MzTab<'_>, rules: RuleSet, report: &mut Report<'_, '_>) {
    let m = &mztab.metadata;
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
        (
            !m.id_confidence_measure.is_empty(),
            MtdKey::IdConfidenceMeasure(ONE.into()),
            sme,
        ),
        (
            !m.database.is_empty(),
            MtdKey::Database(ONE.into(), DatabaseKey::Database),
            identified,
        ),
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
