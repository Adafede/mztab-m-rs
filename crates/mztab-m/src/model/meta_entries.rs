//! Canonical iteration of [`Metadata`] entries in field-reference order.

use alloc::vec::Vec;

use super::Field;
use super::meta_value::ValueRef;
use super::metadata::Metadata;
use crate::id::{IndexMap, Ix};
use crate::meta::key::MtdKey;
use crate::meta::subkey::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InstrumentKey, MsRunKey, ProtocolKey, SampleKey,
    SoftwareKey, StudyVariableGroupKey, StudyVariableKey,
};
use crate::value::cell::Cell;

pub(super) fn collect_entries<'r, 'a>(meta: &'r Metadata<'a>) -> Vec<(MtdKey, ValueRef<'r, 'a>)> {
    let mut e = Entries(Vec::new());
    let c = Collector(meta);
    c.push_header_entries(&mut e);
    c.push_sample_and_run_entries(&mut e);
    c.push_study_and_vocab_entries(&mut e);
    c.push_tail_entries(&mut e);
    e.0
}

struct Collector<'r, 'a>(&'r Metadata<'a>);

impl<'r, 'a> Collector<'r, 'a> {
    fn push_header_entries(&self, e: &mut Entries<'r, 'a>) {
        use ValueRef as V;
        let m = self.0;
        e.one(MtdKey::Version, &m.version, V::Version);
        e.one(MtdKey::Id, &m.id, V::Text);
        e.one(MtdKey::Profile, &m.profile, V::Profile);
        e.one(MtdKey::Title, &m.title, V::Text);
        e.one(MtdKey::Description, &m.description, V::Text);
        e.each(&m.sample_processing, MtdKey::SampleProcessing, V::ParamList);
        for (n, x) in m.instrument.iter() {
            let k = |sub| MtdKey::Instrument(n, sub);
            e.one(k(InstrumentKey::Name), &x.name, V::Param);
            e.one(k(InstrumentKey::Source), &x.source, V::Param);
            e.each(&x.analyzer, |i| k(InstrumentKey::Analyzer(i)), V::ParamList);
            e.one(k(InstrumentKey::Detector), &x.detector, V::Param);
        }
        for (n, x) in m.software.iter() {
            e.one(
                MtdKey::Software(n, SoftwareKey::Software),
                &x.param,
                V::Param,
            );
            e.each(
                &x.setting,
                |i| MtdKey::Software(n, SoftwareKey::Setting(i)),
                V::Text,
            );
        }
        e.each(&m.publication, MtdKey::Publication, V::Publication);
        for (n, x) in m.contact.iter() {
            let k = |sub| MtdKey::Contact(n, sub);
            e.one(k(ContactKey::Name), &x.name, V::Text);
            e.one(k(ContactKey::Affiliation), &x.affiliation, V::Text);
            e.one(k(ContactKey::Email), &x.email, V::Text);
            e.one(k(ContactKey::Orcid), &x.orcid, V::Text);
        }
        e.each(&m.uri, MtdKey::Uri, V::Text);
        e.each(&m.external_study_uri, MtdKey::ExternalStudyUri, V::Text);
        e.one(
            MtdKey::QuantificationMethod,
            &m.quantification_method,
            V::Param,
        );
    }

    fn push_sample_and_run_entries(&self, e: &mut Entries<'r, 'a>) {
        use ValueRef as V;
        let m = self.0;
        for (n, x) in m.sample.iter() {
            let k = |sub| MtdKey::Sample(n, sub);
            e.one(k(SampleKey::Name), &x.name, V::Text);
            e.each(&x.species, |i: Ix| k(SampleKey::Species(i)), V::ParamList);
            e.each(&x.tissue, |i| k(SampleKey::Tissue(i)), V::ParamList);
            e.each(&x.cell_type, |i| k(SampleKey::CellType(i)), V::ParamList);
            e.each(&x.disease, |i| k(SampleKey::Disease(i)), V::ParamList);
            e.one(k(SampleKey::Description), &x.description, V::Text);
            e.each(&x.custom, |i| k(SampleKey::Custom(i)), V::ParamList);
        }
        for (n, x) in m.ms_run.iter() {
            let k = |sub| MtdKey::MsRun(n, sub);
            e.one(k(MsRunKey::Location), &x.location, V::Text);
            e.one(
                k(MsRunKey::InstrumentRef),
                &x.instrument_ref,
                V::InstrumentRef,
            );
            e.one(k(MsRunKey::Format), &x.format, V::Param);
            e.one(k(MsRunKey::IdFormat), &x.id_format, V::Param);
            e.each(
                &x.fragmentation_method,
                |i| k(MsRunKey::FragmentationMethod(i)),
                V::ParamList,
            );
            e.each(
                &x.scan_polarity,
                |i| k(MsRunKey::ScanPolarity(i)),
                V::ParamList,
            );
            e.one(k(MsRunKey::Hash), &x.hash, V::Text);
            e.one(k(MsRunKey::HashMethod), &x.hash_method, V::Param);
            e.one(k(MsRunKey::Parameters), &x.parameters, V::ParamList);
            e.one(k(MsRunKey::UsiIdentifier), &x.usi_identifier, V::Text);
        }
        for (n, x) in m.assay.iter() {
            let k = |sub| MtdKey::Assay(n, sub);
            e.one(k(AssayKey::Name), &x.name, V::Text);
            e.each(&x.custom, |i| k(AssayKey::Custom(i)), V::ParamList);
            e.one(k(AssayKey::ExternalUri), &x.external_uri, V::Text);
            e.one(k(AssayKey::SampleRef), &x.sample_ref, V::SampleRef);
            e.one(k(AssayKey::MsRunRef), &x.ms_run_ref, V::MsRunRefs);
            e.one(k(AssayKey::ProtocolRefs), &x.protocol_refs, V::ProtocolRefs);
            e.one(k(AssayKey::Parameters), &x.parameters, V::ParamList);
        }
    }

    fn push_study_and_vocab_entries(&self, e: &mut Entries<'r, 'a>) {
        use ValueRef as V;
        let m = self.0;
        for (n, x) in m.study_variable.iter() {
            let k = |sub| MtdKey::StudyVariable(n, sub);
            e.one(k(StudyVariableKey::Name), &x.name, V::Text);
            e.one(k(StudyVariableKey::AssayRefs), &x.assay_refs, V::AssayRefs);
            e.one(k(StudyVariableKey::MsRunRefs), &x.ms_run_refs, V::MsRunRefs);
            e.one(k(StudyVariableKey::Description), &x.description, V::Text);
            e.one(
                k(StudyVariableKey::AverageFunction),
                &x.average_function,
                V::Param,
            );
            e.one(
                k(StudyVariableKey::VariationFunction),
                &x.variation_function,
                V::Param,
            );
            e.one(k(StudyVariableKey::Factors), &x.factors, V::ParamList);
        }
        for (n, x) in m.study_variable_group.iter() {
            let k = |sub| MtdKey::StudyVariableGroup(n, sub);
            e.one(k(StudyVariableGroupKey::Group), &x.param, V::Param);
            e.one(
                k(StudyVariableGroupKey::Description),
                &x.description,
                V::Text,
            );
            e.one(k(StudyVariableGroupKey::Type), &x.type_, V::Param);
            e.one(k(StudyVariableGroupKey::Datatype), &x.datatype, V::Datatype);
            e.one(k(StudyVariableGroupKey::Unit), &x.unit, V::Param);
            e.one(
                k(StudyVariableGroupKey::StudyVariableRefs),
                &x.study_variable_refs,
                V::StudyVariableRefs,
            );
        }
        for (n, x) in m.protocol.iter() {
            let k = |sub| MtdKey::Protocol(n, sub);
            e.one(k(ProtocolKey::Name), &x.name, V::Text);
            e.one(k(ProtocolKey::Type), &x.type_, V::Param);
            e.one(k(ProtocolKey::Description), &x.description, V::Text);
            e.one(k(ProtocolKey::Parameters), &x.parameters, V::ParamList);
        }
        e.each(&m.custom, MtdKey::Custom, V::ParamList);
        for (n, x) in m.cv.iter() {
            let k = |sub| MtdKey::Cv(n, sub);
            e.one(k(CvKey::Label), &x.label, V::Text);
            e.one(k(CvKey::FullName), &x.full_name, V::Text);
            e.one(k(CvKey::Version), &x.version, V::Text);
            e.one(k(CvKey::Uri), &x.uri, V::Text);
        }
        for (n, x) in m.database.iter() {
            let k = |sub| MtdKey::Database(n, sub);
            e.one(k(DatabaseKey::Database), &x.param, V::Param);
            e.one(k(DatabaseKey::Prefix), &x.prefix, V::Text);
            e.one(k(DatabaseKey::Version), &x.version, V::Text);
            e.one(k(DatabaseKey::Uri), &x.uri, V::Text);
        }
    }

    fn push_tail_entries(&self, e: &mut Entries<'r, 'a>) {
        use ValueRef as V;
        let m = self.0;
        e.each(
            &m.derivatization_agent,
            MtdKey::DerivatizationAgent,
            V::ParamList,
        );
        e.one(
            MtdKey::SmallMoleculeQuantificationUnit,
            &m.small_molecule_quantification_unit,
            V::Param,
        );
        e.one(
            MtdKey::SmallMoleculeFeatureQuantificationUnit,
            &m.small_molecule_feature_quantification_unit,
            V::Param,
        );
        e.one(
            MtdKey::SmallMoleculeIdentificationReliability,
            &m.small_molecule_identification_reliability,
            V::Param,
        );
        e.each(
            &m.id_confidence_measure,
            MtdKey::IdConfidenceMeasure,
            V::ParamList,
        );
        for (key, units) in [
            (MtdKey::ColunitSmallMolecule, &m.colunit_small_molecule),
            (
                MtdKey::ColunitSmallMoleculeFeature,
                &m.colunit_small_molecule_feature,
            ),
            (
                MtdKey::ColunitSmallMoleculeEvidence,
                &m.colunit_small_molecule_evidence,
            ),
        ] {
            e.0.extend(units.iter().map(|cell| (key, V::ColUnit(cell))));
        }
    }
}

/// Collects `(key, value)` pairs in canonical order.
struct Entries<'r, 'a>(Vec<(MtdKey, ValueRef<'r, 'a>)>);

impl<'r, 'a> Entries<'r, 'a> {
    fn one<T>(
        &mut self,
        key: MtdKey,
        field: &'r Field<'a, T>,
        wrap: fn(&'r Cell<'a, T>) -> ValueRef<'r, 'a>,
    ) {
        if let Some(cell) = field {
            self.0.push((key, wrap(cell)));
        }
    }

    fn each<K: Ord + Copy, T>(
        &mut self,
        map: &'r IndexMap<K, Cell<'a, T>>,
        key: impl Fn(K) -> MtdKey,
        wrap: fn(&'r Cell<'a, T>) -> ValueRef<'r, 'a>,
    ) {
        for (k, cell) in map.iter() {
            self.0.push((key(k), wrap(cell)));
        }
    }
}
