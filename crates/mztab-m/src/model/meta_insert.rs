//! Placing typed [`MtdValue`]s into [`Metadata`].

use super::Field;
use super::colunit::ColUnit;
use super::meta_value::{InsertError, MtdValue};
use super::metadata::Metadata;
use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IndexMap, InstrumentId, MsRunId, ProtocolId, SampleId,
    SoftwareId, StudyVariableGroupId, StudyVariableId,
};
use crate::meta::key::MtdKey;
use crate::meta::subkey::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InstrumentKey, MsRunKey, ProtocolKey, SampleKey,
    SoftwareKey, StudyVariableGroupKey, StudyVariableKey,
};
use crate::param::Param;
use crate::value::cell::Cell;

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
        match k {
            SoftwareKey::Software => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.software.get_or_insert_default(n).param, cell)
            }
            SoftwareKey::Setting(m) => {
                let cell = expect_kind!(value, Text);
                put_at(
                    &mut self.0.software.get_or_insert_default(n).setting,
                    m,
                    cell,
                )
            }
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
        match k {
            ProtocolKey::Name => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.protocol.get_or_insert_default(n).name, cell)
            }
            ProtocolKey::Description => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.protocol.get_or_insert_default(n).description,
                    cell,
                )
            }
            ProtocolKey::Type => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.protocol.get_or_insert_default(n).type_, cell)
            }
            ProtocolKey::Parameters => {
                let cell = expect_kind!(value, ParamList);
                put(
                    &mut self.0.protocol.get_or_insert_default(n).parameters,
                    cell,
                )
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
        match k {
            DatabaseKey::Database => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.database.get_or_insert_default(n).param, cell)
            }
            DatabaseKey::Prefix => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.database.get_or_insert_default(n).prefix, cell)
            }
            DatabaseKey::Version => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.database.get_or_insert_default(n).version, cell)
            }
            DatabaseKey::Uri => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.database.get_or_insert_default(n).uri, cell)
            }
        }
    }

    fn instrument_value(
        &mut self,
        n: InstrumentId,
        k: InstrumentKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            InstrumentKey::Analyzer(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(
                    &mut self.0.instrument.get_or_insert_default(n).analyzer,
                    m,
                    cell,
                )
            }
            InstrumentKey::Name => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.instrument.get_or_insert_default(n).name, cell)
            }
            InstrumentKey::Source => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.instrument.get_or_insert_default(n).source, cell)
            }
            InstrumentKey::Detector => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self.0.instrument.get_or_insert_default(n).detector,
                    cell,
                )
            }
        }
    }

    fn sample_value(
        &mut self,
        n: SampleId,
        k: SampleKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            SampleKey::Name => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.sample.get_or_insert_default(n).name, cell)
            }
            SampleKey::Description => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.sample.get_or_insert_default(n).description,
                    cell,
                )
            }
            SampleKey::Species(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(&mut self.0.sample.get_or_insert_default(n).species, m, cell)
            }
            SampleKey::Tissue(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(&mut self.0.sample.get_or_insert_default(n).tissue, m, cell)
            }
            SampleKey::CellType(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(
                    &mut self.0.sample.get_or_insert_default(n).cell_type,
                    m,
                    cell,
                )
            }
            SampleKey::Disease(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(&mut self.0.sample.get_or_insert_default(n).disease, m, cell)
            }
            SampleKey::Custom(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(&mut self.0.sample.get_or_insert_default(n).custom, m, cell)
            }
        }
    }

    fn ms_run_value(
        &mut self,
        n: MsRunId,
        k: MsRunKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            MsRunKey::Location => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.ms_run.get_or_insert_default(n).location, cell)
            }
            MsRunKey::Hash => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.ms_run.get_or_insert_default(n).hash, cell)
            }
            MsRunKey::UsiIdentifier => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.ms_run.get_or_insert_default(n).usi_identifier,
                    cell,
                )
            }
            MsRunKey::InstrumentRef => {
                let cell = expect_kind!(value, InstrumentRef);
                put(
                    &mut self.0.ms_run.get_or_insert_default(n).instrument_ref,
                    cell,
                )
            }
            MsRunKey::Format => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.ms_run.get_or_insert_default(n).format, cell)
            }
            MsRunKey::IdFormat => {
                let cell = expect_kind!(value, Param);
                put(&mut self.0.ms_run.get_or_insert_default(n).id_format, cell)
            }
            MsRunKey::HashMethod => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self.0.ms_run.get_or_insert_default(n).hash_method,
                    cell,
                )
            }
            MsRunKey::Parameters => {
                let cell = expect_kind!(value, ParamList);
                put(&mut self.0.ms_run.get_or_insert_default(n).parameters, cell)
            }
            MsRunKey::FragmentationMethod(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(
                    &mut self.0.ms_run.get_or_insert_default(n).fragmentation_method,
                    m,
                    cell,
                )
            }
            MsRunKey::ScanPolarity(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(
                    &mut self.0.ms_run.get_or_insert_default(n).scan_polarity,
                    m,
                    cell,
                )
            }
        }
    }

    fn assay_value(
        &mut self,
        n: AssayId,
        k: AssayKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            AssayKey::Name => {
                let cell = expect_kind!(value, Text);
                put(&mut self.0.assay.get_or_insert_default(n).name, cell)
            }
            AssayKey::ExternalUri => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.assay.get_or_insert_default(n).external_uri,
                    cell,
                )
            }
            AssayKey::Custom(m) => {
                let cell = expect_kind!(value, ParamList);
                put_at(&mut self.0.assay.get_or_insert_default(n).custom, m, cell)
            }
            AssayKey::SampleRef => {
                let cell = expect_kind!(value, SampleRef);
                put(&mut self.0.assay.get_or_insert_default(n).sample_ref, cell)
            }
            AssayKey::MsRunRef => {
                let cell = expect_kind!(value, MsRunRefs);
                put(&mut self.0.assay.get_or_insert_default(n).ms_run_ref, cell)
            }
            AssayKey::ProtocolRefs => {
                let cell = expect_kind!(value, ProtocolRefs);
                put(
                    &mut self.0.assay.get_or_insert_default(n).protocol_refs,
                    cell,
                )
            }
            AssayKey::Parameters => {
                let cell = expect_kind!(value, ParamList);
                put(&mut self.0.assay.get_or_insert_default(n).parameters, cell)
            }
        }
    }

    fn study_variable_value(
        &mut self,
        n: StudyVariableId,
        k: StudyVariableKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            StudyVariableKey::Name => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.study_variable.get_or_insert_default(n).name,
                    cell,
                )
            }
            StudyVariableKey::Description => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self.0.study_variable.get_or_insert_default(n).description,
                    cell,
                )
            }
            StudyVariableKey::AssayRefs => {
                let cell = expect_kind!(value, AssayRefs);
                put(
                    &mut self.0.study_variable.get_or_insert_default(n).assay_refs,
                    cell,
                )
            }
            StudyVariableKey::MsRunRefs => {
                let cell = expect_kind!(value, MsRunRefs);
                put(
                    &mut self.0.study_variable.get_or_insert_default(n).ms_run_refs,
                    cell,
                )
            }
            StudyVariableKey::AverageFunction => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self
                        .0
                        .study_variable
                        .get_or_insert_default(n)
                        .average_function,
                    cell,
                )
            }
            StudyVariableKey::VariationFunction => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self
                        .0
                        .study_variable
                        .get_or_insert_default(n)
                        .variation_function,
                    cell,
                )
            }
            StudyVariableKey::Factors => {
                let cell = expect_kind!(value, ParamList);
                put(
                    &mut self.0.study_variable.get_or_insert_default(n).factors,
                    cell,
                )
            }
        }
    }

    fn study_variable_group_value(
        &mut self,
        n: StudyVariableGroupId,
        k: StudyVariableGroupKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        match k {
            StudyVariableGroupKey::Group => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self.0.study_variable_group.get_or_insert_default(n).param,
                    cell,
                )
            }
            StudyVariableGroupKey::Description => {
                let cell = expect_kind!(value, Text);
                put(
                    &mut self
                        .0
                        .study_variable_group
                        .get_or_insert_default(n)
                        .description,
                    cell,
                )
            }
            StudyVariableGroupKey::Type => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self.0.study_variable_group.get_or_insert_default(n).type_,
                    cell,
                )
            }
            StudyVariableGroupKey::Datatype => {
                let cell = expect_kind!(value, Datatype);
                put(
                    &mut self
                        .0
                        .study_variable_group
                        .get_or_insert_default(n)
                        .datatype,
                    cell,
                )
            }
            StudyVariableGroupKey::Unit => {
                let cell = expect_kind!(value, Param);
                put(
                    &mut self.0.study_variable_group.get_or_insert_default(n).unit,
                    cell,
                )
            }
            StudyVariableGroupKey::StudyVariableRefs => {
                let cell = expect_kind!(value, StudyVariableRefs);
                put(
                    &mut self
                        .0
                        .study_variable_group
                        .get_or_insert_default(n)
                        .study_variable_refs,
                    cell,
                )
            }
        }
    }
}
