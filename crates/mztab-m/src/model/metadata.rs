//! [`Metadata`] section and indexed metadata element structs.

use alloc::vec::Vec;

use super::colunit::ColUnit;
use super::meta_value::{InsertError, MtdValue, ValueRef};
use super::{Field, List};
use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
use crate::meta::key::MtdKey;
use crate::param::Param;
use crate::value::cell::Cell;
use crate::value::text::{Raw, Text};
use crate::vocab::field::{Datatype, PublicationItem};
use crate::vocab::profile::Profile;
use crate::vocab::version::Version;

/// The metadata section. Field names follow the metadata keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata<'a> {
    /// `mzTab-version`.
    pub version: Field<'a, Version>,
    /// `mzTab-ID`.
    pub id: Field<'a, Text<'a>>,
    /// `mzTab-profile` (2.1).
    pub profile: Field<'a, Profile>,
    /// `title`.
    pub title: Field<'a, Text<'a>>,
    /// `description`.
    pub description: Field<'a, Text<'a>>,
    /// `sample_processing[n]`.
    pub sample_processing: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `instrument[n]-*`.
    pub instrument: IndexMap<InstrumentId, Instrument<'a>>,
    /// `software[n]` and `software[n]-setting[m]`.
    pub software: IndexMap<SoftwareId, Software<'a>>,
    /// `publication[n]`.
    pub publication: IndexMap<PublicationId, Cell<'a, List<'a, PublicationItem<'a>>>>,
    /// `contact[n]-*`.
    pub contact: IndexMap<ContactId, Contact<'a>>,
    /// `uri[n]`.
    pub uri: IndexMap<Ix, Cell<'a, Text<'a>>>,
    /// `external_study_uri[n]`.
    pub external_study_uri: IndexMap<Ix, Cell<'a, Text<'a>>>,
    /// `quantification_method`.
    pub quantification_method: Field<'a, Param<'a>>,
    /// `sample[n]` and `sample[n]-*`.
    pub sample: IndexMap<SampleId, Sample<'a>>,
    /// `ms_run[n]-*`.
    pub ms_run: IndexMap<MsRunId, MsRun<'a>>,
    /// `assay[n]` and `assay[n]-*`.
    pub assay: IndexMap<AssayId, Assay<'a>>,
    /// `study_variable[n]` and `study_variable[n]-*`.
    pub study_variable: IndexMap<StudyVariableId, StudyVariable<'a>>,
    /// `study_variable_group[n]` and `study_variable_group[n]-*` (2.1).
    pub study_variable_group: IndexMap<StudyVariableGroupId, StudyVariableGroup<'a>>,
    /// `protocol[n]-*` (2.1).
    pub protocol: IndexMap<ProtocolId, Protocol<'a>>,
    /// `custom[n]`.
    pub custom: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `cv[n]-*`.
    pub cv: IndexMap<CvId, Cv<'a>>,
    /// `database[n]` and `database[n]-*`.
    pub database: IndexMap<DatabaseId, Database<'a>>,
    /// `derivatization_agent[n]`.
    pub derivatization_agent: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `small_molecule-quantification_unit`.
    pub small_molecule_quantification_unit: Field<'a, Param<'a>>,
    /// `small_molecule_feature-quantification_unit`.
    pub small_molecule_feature_quantification_unit: Field<'a, Param<'a>>,
    /// `small_molecule-identification_reliability`.
    pub small_molecule_identification_reliability: Field<'a, Param<'a>>,
    /// `id_confidence_measure[n]`.
    pub id_confidence_measure: IndexMap<IdConfidenceMeasureId, Cell<'a, List<'a, Param<'a>>>>,
    /// `colunit-small_molecule`, one entry per line.
    pub colunit_small_molecule: Vec<Cell<'a, ColUnit<'a>>>,
    /// `colunit-small_molecule_feature`, one entry per line.
    pub colunit_small_molecule_feature: Vec<Cell<'a, ColUnit<'a>>>,
    /// `colunit-small_molecule_evidence`, one entry per line.
    pub colunit_small_molecule_evidence: Vec<Cell<'a, ColUnit<'a>>>,
    /// Lines with a key this crate does not recognise: `(key, value)`, in
    /// source order. The value is every cell after the key, tabs included.
    pub unknown: Vec<(Raw<'a>, Raw<'a>)>,
}

/// `instrument[n]-*`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Instrument<'a> {
    /// `instrument[n]-name`.
    pub name: Field<'a, Param<'a>>,
    /// `instrument[n]-source`.
    pub source: Field<'a, Param<'a>>,
    /// `instrument[n]-analyzer[m]`.
    pub analyzer: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `instrument[n]-detector`.
    pub detector: Field<'a, Param<'a>>,
}

/// `software[n]` and its settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Software<'a> {
    /// `software[n]`: the software and its version.
    pub param: Field<'a, Param<'a>>,
    /// `software[n]-setting[m]`.
    pub setting: IndexMap<Ix, Cell<'a, Text<'a>>>,
}

/// `contact[n]-*`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contact<'a> {
    /// `contact[n]-name`.
    pub name: Field<'a, Text<'a>>,
    /// `contact[n]-affiliation`.
    pub affiliation: Field<'a, Text<'a>>,
    /// `contact[n]-email`.
    pub email: Field<'a, Text<'a>>,
    /// `contact[n]-orcid` (2.1).
    pub orcid: Field<'a, Text<'a>>,
}

/// `sample[n]` and its properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample<'a> {
    /// `sample[n]`: the sample name.
    pub name: Field<'a, Text<'a>>,
    /// `sample[n]-species[m]`.
    pub species: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `sample[n]-tissue[m]`.
    pub tissue: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `sample[n]-cell_type[m]`.
    pub cell_type: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `sample[n]-disease[m]`.
    pub disease: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `sample[n]-description`.
    pub description: Field<'a, Text<'a>>,
    /// `sample[n]-custom[m]`.
    pub custom: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
}

/// `ms_run[n]-*`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MsRun<'a> {
    /// `ms_run[n]-location`: a URI.
    pub location: Field<'a, Text<'a>>,
    /// `ms_run[n]-instrument_ref`.
    pub instrument_ref: Field<'a, InstrumentId>,
    /// `ms_run[n]-format`.
    pub format: Field<'a, Param<'a>>,
    /// `ms_run[n]-id_format`.
    pub id_format: Field<'a, Param<'a>>,
    /// `ms_run[n]-fragmentation_method[m]`.
    pub fragmentation_method: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `ms_run[n]-scan_polarity[m]`.
    pub scan_polarity: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `ms_run[n]-hash`.
    pub hash: Field<'a, Text<'a>>,
    /// `ms_run[n]-hash_method`.
    pub hash_method: Field<'a, Param<'a>>,
    /// `ms_run[n]-parameters` (2.1).
    pub parameters: Field<'a, List<'a, Param<'a>>>,
    /// `ms_run[n]-usi_identifier` (2.0 only; removed in 2.1).
    pub usi_identifier: Field<'a, Text<'a>>,
}

/// `assay[n]` and its properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assay<'a> {
    /// `assay[n]`: the assay name.
    pub name: Field<'a, Text<'a>>,
    /// `assay[n]-custom[m]`.
    pub custom: IndexMap<Ix, Cell<'a, List<'a, Param<'a>>>>,
    /// `assay[n]-external_uri`.
    pub external_uri: Field<'a, Text<'a>>,
    /// `assay[n]-sample_ref`.
    pub sample_ref: Field<'a, SampleId>,
    /// `assay[n]-ms_run_ref`.
    pub ms_run_ref: Field<'a, List<'a, MsRunId>>,
    /// `assay[n]-protocol_refs` (2.1).
    pub protocol_refs: Field<'a, List<'a, ProtocolId>>,
    /// `assay[n]-parameters` (2.1).
    pub parameters: Field<'a, List<'a, Param<'a>>>,
}

/// `study_variable[n]` and its properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StudyVariable<'a> {
    /// `study_variable[n]`: the name. In 2.1 this may also be a parameter,
    /// depending on the group's `datatype`; the text is kept as written and
    /// interpreted by the validator (DECISIONS D-017).
    pub name: Field<'a, Text<'a>>,
    /// `study_variable[n]-assay_refs`.
    pub assay_refs: Field<'a, List<'a, AssayId>>,
    /// `study_variable[n]-ms_run_refs` (2.1).
    pub ms_run_refs: Field<'a, List<'a, MsRunId>>,
    /// `study_variable[n]-description`.
    pub description: Field<'a, Text<'a>>,
    /// `study_variable[n]-average_function`.
    pub average_function: Field<'a, Param<'a>>,
    /// `study_variable[n]-variation_function`.
    pub variation_function: Field<'a, Param<'a>>,
    /// `study_variable[n]-factors` (2.0 only; removed in 2.1).
    pub factors: Field<'a, List<'a, Param<'a>>>,
}

/// `study_variable_group[n]` and its properties (2.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StudyVariableGroup<'a> {
    /// `study_variable_group[n]`: the grouping factor.
    pub param: Field<'a, Param<'a>>,
    /// `study_variable_group[n]-description`.
    pub description: Field<'a, Text<'a>>,
    /// `study_variable_group[n]-type`.
    pub type_: Field<'a, Param<'a>>,
    /// `study_variable_group[n]-datatype`.
    pub datatype: Field<'a, Datatype>,
    /// `study_variable_group[n]-unit`.
    pub unit: Field<'a, Param<'a>>,
    /// `study_variable_group[n]-study_variable_refs`.
    pub study_variable_refs: Field<'a, List<'a, StudyVariableId>>,
}

/// `protocol[n]-*` (2.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Protocol<'a> {
    /// `protocol[n]-name`.
    pub name: Field<'a, Text<'a>>,
    /// `protocol[n]-type`.
    pub type_: Field<'a, Param<'a>>,
    /// `protocol[n]-description`.
    pub description: Field<'a, Text<'a>>,
    /// `protocol[n]-parameters`.
    pub parameters: Field<'a, List<'a, Param<'a>>>,
}

/// `cv[n]-*`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cv<'a> {
    /// `cv[n]-label`.
    pub label: Field<'a, Text<'a>>,
    /// `cv[n]-full_name`.
    pub full_name: Field<'a, Text<'a>>,
    /// `cv[n]-version`.
    pub version: Field<'a, Text<'a>>,
    /// `cv[n]-uri`.
    pub uri: Field<'a, Text<'a>>,
}

/// `database[n]` and its properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Database<'a> {
    /// `database[n]`: the database.
    pub param: Field<'a, Param<'a>>,
    /// `database[n]-prefix`.
    pub prefix: Field<'a, Text<'a>>,
    /// `database[n]-version`.
    pub version: Field<'a, Text<'a>>,
    /// `database[n]-uri`.
    pub uri: Field<'a, Text<'a>>,
}

impl<'a> Metadata<'a> {
    /// Places a typed value at `key`.
    ///
    /// # Errors
    ///
    /// [`InsertError::Occupied`] if the key already has a value (other than a
    /// repeating `colunit-*` key), and [`InsertError::WrongKind`] if `value`
    /// is not of `key.kind()`. Nothing is changed on error.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{Cell, InsertError, Metadata, MtdKey, MtdValue, Text};
    ///
    /// let mut metadata = Metadata::default();
    /// let title = || MtdValue::Text(Cell::Value(Text::new("A study").unwrap()));
    /// assert_eq!(metadata.insert(MtdKey::Title, title()), Ok(()));
    /// assert_eq!(metadata.insert(MtdKey::Title, title()), Err(InsertError::Occupied));
    /// assert_eq!(metadata.insert(MtdKey::Version, title()), Err(InsertError::WrongKind));
    /// ```
    pub fn insert(&mut self, key: MtdKey, value: MtdValue<'a>) -> Result<(), InsertError> {
        super::meta_insert::insert_value(self, key, value)
    }

    /// Every value, with its key, in canonical order: field-reference
    /// order, each element's fields together, `colunit-*` keys once per
    /// value. [`Metadata::unknown`] is not included.
    ///
    /// # Examples
    ///
    /// ```
    /// let parsed = mztab_m::parse("MTD\ttitle\tT\nMTD\tmzTab-version\t2.1.0-M\n");
    /// let keys: Vec<_> = parsed.mztab.metadata.entries().iter().map(|(k, _)| k.to_string()).collect();
    /// assert_eq!(keys, ["mzTab-version", "title"]);
    /// ```
    #[must_use]
    pub fn entries(&self) -> Vec<(MtdKey, ValueRef<'_, 'a>)> {
        super::meta_entries::collect_entries(self)
    }
}
