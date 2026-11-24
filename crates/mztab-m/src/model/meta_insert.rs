//! Placing typed [`MtdValue`]s into [`Metadata`].

use super::meta_value::{InsertError, MtdValue};
use super::metadata::Metadata;
use super::{ColUnit, Field};
use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IndexMap, InstrumentId, MsRunId, ProtocolId, SampleId,
    SoftwareId, StudyVariableGroupId, StudyVariableId,
};
use crate::meta::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InstrumentKey, MsRunKey, MtdKey, ProtocolKey,
    SampleKey, SoftwareKey, StudyVariableGroupKey, StudyVariableKey,
};
use crate::param::Param;
use crate::value::Cell;

/// Places `cell` in an empty field.
fn put<'a, T>(slot: &mut Field<'a, T>, cell: Cell<'a, T>) -> Result<(), InsertError> {
    if slot.is_some() {
        return Err(InsertError::Occupied);
    }
    *slot = Some(cell);
    Ok(())
}

/// Places `cell` at an unused index.
fn put_at<'a, K: Ord + Copy, T>(
    map: &mut IndexMap<K, Cell<'a, T>>,
    key: K,
    cell: Cell<'a, T>,
) -> Result<(), InsertError> {
    if map.contains_key(key) {
        return Err(InsertError::Occupied);
    }
    map.insert(key, cell);
    Ok(())
}

macro_rules! expect_kind {
    ($value:ident, $variant:ident) => {
        match $value {
            MtdValue::$variant(cell) => cell,
            _ => return Err(InsertError::WrongKind),
        }
    };
}

fn put_param<'a>(slot: &mut Field<'a, Param<'a>>, v: MtdValue<'a>) -> Result<(), InsertError> {
    put(slot, expect_kind!(v, Param))
}

fn push_col<'a>(
    list: &mut alloc::vec::Vec<Cell<'a, ColUnit<'a>>>,
    v: MtdValue<'a>,
) -> Result<(), InsertError> {
    list.push(expect_kind!(v, ColUnit));
    Ok(())
}

pub(super) fn insert_value<'a>(
    meta: &mut Metadata<'a>,
    key: MtdKey,
    v: MtdValue<'a>,
) -> Result<(), InsertError> {
    let mut inserter = Inserter(meta);
    inserter.insert(key, v)
}

struct Inserter<'m, 'a>(&'m mut Metadata<'a>);

impl<'a> Inserter<'_, 'a> {
    fn insert(&mut self, key: MtdKey, v: MtdValue<'a>) -> Result<(), InsertError> {
        let m = &mut *self.0;
        match key {
            MtdKey::Version => put(&mut m.version, expect_kind!(v, Version)),
            MtdKey::Profile => put(&mut m.profile, expect_kind!(v, Profile)),
            MtdKey::Id => put(&mut m.id, expect_kind!(v, Text)),
            MtdKey::Title => put(&mut m.title, expect_kind!(v, Text)),
            MtdKey::Description => put(&mut m.description, expect_kind!(v, Text)),
            MtdKey::SampleProcessing(n) => {
                put_at(&mut m.sample_processing, n, expect_kind!(v, ParamList))
            }
            MtdKey::Instrument(n, k) => self.instrument_value(n, k, v),
            MtdKey::Software(n, k) => self.software_value(n, k, v),
            MtdKey::Publication(n) => put_at(&mut m.publication, n, expect_kind!(v, Publication)),
            MtdKey::Contact(n, k) => self.contact_value(n, k, v),
            MtdKey::Uri(n) => put_at(&mut m.uri, n, expect_kind!(v, Text)),
            MtdKey::ExternalStudyUri(n) => {
                put_at(&mut m.external_study_uri, n, expect_kind!(v, Text))
            }
            MtdKey::QuantificationMethod => put_param(&mut m.quantification_method, v),
            MtdKey::Sample(n, k) => self.sample_value(n, k, v),
            MtdKey::MsRun(n, k) => self.ms_run_value(n, k, v),
            MtdKey::Assay(n, k) => self.assay_value(n, k, v),
            MtdKey::StudyVariable(n, k) => self.study_variable_value(n, k, v),
            MtdKey::StudyVariableGroup(n, k) => self.study_variable_group_value(n, k, v),
            MtdKey::Custom(n) => put_at(&mut m.custom, n, expect_kind!(v, ParamList)),
            MtdKey::Cv(n, k) => self.cv_value(n, k, v),
            MtdKey::SmallMoleculeQuantificationUnit => {
                put_param(&mut m.small_molecule_quantification_unit, v)
            }
            MtdKey::SmallMoleculeFeatureQuantificationUnit => {
                put_param(&mut m.small_molecule_feature_quantification_unit, v)
            }
            MtdKey::SmallMoleculeIdentificationReliability => {
                put_param(&mut m.small_molecule_identification_reliability, v)
            }
            MtdKey::Database(n, k) => self.database_value(n, k, v),
            MtdKey::DerivatizationAgent(n) => {
                put_at(&mut m.derivatization_agent, n, expect_kind!(v, ParamList))
            }
            MtdKey::IdConfidenceMeasure(n) => {
                put_at(&mut m.id_confidence_measure, n, expect_kind!(v, ParamList))
            }
            MtdKey::Protocol(n, k) => self.protocol_value(n, k, v),
            MtdKey::ColunitSmallMolecule => push_col(&mut m.colunit_small_molecule, v),
            MtdKey::ColunitSmallMoleculeFeature => {
                push_col(&mut m.colunit_small_molecule_feature, v)
            }
            MtdKey::ColunitSmallMoleculeEvidence => {
                push_col(&mut m.colunit_small_molecule_evidence, v)
            }
        }
    }

    fn software_value(
        &mut self,
        n: SoftwareId,
        k: SoftwareKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let software = self.0.software.get_or_insert_default(n);
        match k {
            SoftwareKey::Software => put(&mut software.param, expect_kind!(value, Param)),
            SoftwareKey::Setting(m) => put_at(&mut software.setting, m, expect_kind!(value, Text)),
        }
    }

    fn contact_value(
        &mut self,
        n: ContactId,
        k: ContactKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let cell = expect_kind!(value, Text);
        let contact = self.0.contact.get_or_insert_default(n);
        put(
            match k {
                ContactKey::Name => &mut contact.name,
                ContactKey::Affiliation => &mut contact.affiliation,
                ContactKey::Email => &mut contact.email,
                ContactKey::Orcid => &mut contact.orcid,
            },
            cell,
        )
    }

    fn protocol_value(
        &mut self,
        n: ProtocolId,
        k: ProtocolKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let protocol = self.0.protocol.get_or_insert_default(n);
        match k {
            ProtocolKey::Name => put(&mut protocol.name, expect_kind!(value, Text)),
            ProtocolKey::Description => put(&mut protocol.description, expect_kind!(value, Text)),
            ProtocolKey::Type => put(&mut protocol.type_, expect_kind!(value, Param)),
            ProtocolKey::Parameters => {
                put(&mut protocol.parameters, expect_kind!(value, ParamList))
            }
        }
    }

    fn cv_value(&mut self, n: CvId, k: CvKey, value: MtdValue<'a>) -> Result<(), InsertError> {
        let cell = expect_kind!(value, Text);
        let cv = self.0.cv.get_or_insert_default(n);
        put(
            match k {
                CvKey::Label => &mut cv.label,
                CvKey::FullName => &mut cv.full_name,
                CvKey::Version => &mut cv.version,
                CvKey::Uri => &mut cv.uri,
            },
            cell,
        )
    }

    fn database_value(
        &mut self,
        n: DatabaseId,
        k: DatabaseKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let database = self.0.database.get_or_insert_default(n);
        match k {
            DatabaseKey::Database => put(&mut database.param, expect_kind!(value, Param)),
            DatabaseKey::Prefix => put(&mut database.prefix, expect_kind!(value, Text)),
            DatabaseKey::Version => put(&mut database.version, expect_kind!(value, Text)),
            DatabaseKey::Uri => put(&mut database.uri, expect_kind!(value, Text)),
        }
    }

    fn instrument_value(
        &mut self,
        n: InstrumentId,
        k: InstrumentKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let instrument = self.0.instrument.get_or_insert_default(n);
        match k {
            InstrumentKey::Analyzer(m) => {
                put_at(&mut instrument.analyzer, m, expect_kind!(value, ParamList))
            }
            InstrumentKey::Name => put(&mut instrument.name, expect_kind!(value, Param)),
            InstrumentKey::Source => put(&mut instrument.source, expect_kind!(value, Param)),
            InstrumentKey::Detector => put(&mut instrument.detector, expect_kind!(value, Param)),
        }
    }

    fn sample_value(
        &mut self,
        n: SampleId,
        k: SampleKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let sample = self.0.sample.get_or_insert_default(n);
        match k {
            SampleKey::Name => put(&mut sample.name, expect_kind!(value, Text)),
            SampleKey::Description => put(&mut sample.description, expect_kind!(value, Text)),
            SampleKey::Species(m) => put_at(&mut sample.species, m, expect_kind!(value, ParamList)),
            SampleKey::Tissue(m) => put_at(&mut sample.tissue, m, expect_kind!(value, ParamList)),
            SampleKey::CellType(m) => {
                put_at(&mut sample.cell_type, m, expect_kind!(value, ParamList))
            }
            SampleKey::Disease(m) => put_at(&mut sample.disease, m, expect_kind!(value, ParamList)),
            SampleKey::Custom(m) => put_at(&mut sample.custom, m, expect_kind!(value, ParamList)),
        }
    }

    fn ms_run_value(
        &mut self,
        n: MsRunId,
        k: MsRunKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let run = self.0.ms_run.get_or_insert_default(n);
        match k {
            MsRunKey::Location => put(&mut run.location, expect_kind!(value, Text)),
            MsRunKey::Hash => put(&mut run.hash, expect_kind!(value, Text)),
            MsRunKey::UsiIdentifier => put(&mut run.usi_identifier, expect_kind!(value, Text)),
            MsRunKey::InstrumentRef => {
                put(&mut run.instrument_ref, expect_kind!(value, InstrumentRef))
            }
            MsRunKey::Format => put(&mut run.format, expect_kind!(value, Param)),
            MsRunKey::IdFormat => put(&mut run.id_format, expect_kind!(value, Param)),
            MsRunKey::HashMethod => put(&mut run.hash_method, expect_kind!(value, Param)),
            MsRunKey::Parameters => put(&mut run.parameters, expect_kind!(value, ParamList)),
            MsRunKey::FragmentationMethod(m) => put_at(
                &mut run.fragmentation_method,
                m,
                expect_kind!(value, ParamList),
            ),
            MsRunKey::ScanPolarity(m) => {
                put_at(&mut run.scan_polarity, m, expect_kind!(value, ParamList))
            }
        }
    }

    fn assay_value(
        &mut self,
        n: AssayId,
        k: AssayKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let assay = self.0.assay.get_or_insert_default(n);
        match k {
            AssayKey::Name => put(&mut assay.name, expect_kind!(value, Text)),
            AssayKey::ExternalUri => put(&mut assay.external_uri, expect_kind!(value, Text)),
            AssayKey::Custom(m) => put_at(&mut assay.custom, m, expect_kind!(value, ParamList)),
            AssayKey::SampleRef => put(&mut assay.sample_ref, expect_kind!(value, SampleRef)),
            AssayKey::MsRunRef => put(&mut assay.ms_run_ref, expect_kind!(value, MsRunRefs)),
            AssayKey::ProtocolRefs => {
                put(&mut assay.protocol_refs, expect_kind!(value, ProtocolRefs))
            }
            AssayKey::Parameters => put(&mut assay.parameters, expect_kind!(value, ParamList)),
        }
    }

    fn study_variable_value(
        &mut self,
        n: StudyVariableId,
        k: StudyVariableKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let sv = self.0.study_variable.get_or_insert_default(n);
        match k {
            StudyVariableKey::Name => put(&mut sv.name, expect_kind!(value, Text)),
            StudyVariableKey::Description => put(&mut sv.description, expect_kind!(value, Text)),
            StudyVariableKey::AssayRefs => put(&mut sv.assay_refs, expect_kind!(value, AssayRefs)),
            StudyVariableKey::MsRunRefs => put(&mut sv.ms_run_refs, expect_kind!(value, MsRunRefs)),
            StudyVariableKey::AverageFunction => {
                put(&mut sv.average_function, expect_kind!(value, Param))
            }
            StudyVariableKey::VariationFunction => {
                put(&mut sv.variation_function, expect_kind!(value, Param))
            }
            StudyVariableKey::Factors => put(&mut sv.factors, expect_kind!(value, ParamList)),
        }
    }

    fn study_variable_group_value(
        &mut self,
        n: StudyVariableGroupId,
        k: StudyVariableGroupKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let group = self.0.study_variable_group.get_or_insert_default(n);
        match k {
            StudyVariableGroupKey::Group => put(&mut group.param, expect_kind!(value, Param)),
            StudyVariableGroupKey::Description => {
                put(&mut group.description, expect_kind!(value, Text))
            }
            StudyVariableGroupKey::Type => put(&mut group.type_, expect_kind!(value, Param)),
            StudyVariableGroupKey::Datatype => {
                put(&mut group.datatype, expect_kind!(value, Datatype))
            }
            StudyVariableGroupKey::Unit => put(&mut group.unit, expect_kind!(value, Param)),
            StudyVariableGroupKey::StudyVariableRefs => put(
                &mut group.study_variable_refs,
                expect_kind!(value, StudyVariableRefs),
            ),
        }
    }
}
