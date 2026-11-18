//! The typed document model: [`MzTab`], [`Metadata`] and the three tables.
//!
//! The model follows the 2.1 field reference literally, plus the two fields
//! that 2.0 had and 2.1 removed (`ms_run[n]-usi_identifier` and
//! `study_variable[n]-factors`), so 2.0 files read losslessly.
//!
//! - A metadata field is a [`Field`]: `None` when the key is absent,
//!   otherwise a [`Cell`] (value, `null`, or invalid text).
//! - A field typed "… List" in the field reference holds a [`List`]: a
//!   `|`-separated list whose items are cells, so `null|HMDB:1` is kept.
//! - `element[n]` objects live in an [`IndexMap`] keyed by their typed id.
//! - Unknown metadata keys are kept, in source order, in
//!   [`Metadata::unknown`].
//!
//! ```
//! use mztab_m::{Cell, IndexMap, Metadata, MzTab, Text, Version};
//!
//! let mut mztab = MzTab::default();
//! mztab.metadata.version = Some(Cell::Value(Version::V2_1_0));
//! mztab.metadata.id = Some(Cell::Value(Text::new("MTBLS263")?));
//! assert!(mztab.sml.is_none());
//! assert!(mztab.metadata.unknown.is_empty());
//! # Ok::<(), mztab_m::TextError>(())
//! ```
//!
//! # What the types do not enforce
//!
//! These rules are checked by `validate`, not by the type system:
//! cross-references resolve; row ids are unique; profile-dependent fields are
//! present; SML `|`-lists have equal lengths; study-variable values match
//! their group's `datatype`; CV terms exist; metadata order; contiguous
//! indices; `spectra_ref` matches the run's `id_format`; and no cell is
//! [`Cell::Invalid`].

use alloc::vec::Vec;
use core::fmt;

use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
use crate::param::{Param, ParamError};
use crate::value::{Cell, Decimal, Raw, Text, TextError};
use crate::vocab::{AmbiguityCode, Profile, PublicationItem, Version};

/// A metadata field: absent (`None`) or present as a [`Cell`].
pub type Field<'a, T> = Option<Cell<'a, T>>;

/// A `|`-separated list. Each item is a cell, so `null` items and invalid
/// items are kept.
pub type List<'a, T> = Vec<Cell<'a, T>>;

/// A whole mzTab-M document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MzTab<'a> {
    /// The metadata section (`MTD`), always present.
    pub metadata: Metadata<'a>,
    /// The small molecule summary (`SMH` / `SML`), if present.
    pub sml: Option<Table<'a, SmlColumn, SmlRow<'a>>>,
    /// The small molecule features (`SFH` / `SMF`), if present.
    pub smf: Option<Table<'a, SmfColumn, SmfRow<'a>>>,
    /// The small molecule evidence (`SEH` / `SME`), if present.
    pub sme: Option<Table<'a, SmeColumn, SmeRow<'a>>>,
}

impl MzTab<'_> {
    /// The profile implied by the tables present (SPEC §5.18), or the
    /// prohibited combination they form.
    ///
    /// A header-only table counts as present.
    ///
    /// # Errors
    ///
    /// Returns the prohibited combination for evidence without features.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{MzTab, Profile};
    ///
    /// assert_eq!(MzTab::default().inferred_profile(), Ok(Profile::M));
    /// ```
    pub const fn inferred_profile(&self) -> Result<Profile, crate::vocab::ProhibitedProfile> {
        Profile::from_tables(self.sml.is_some(), self.smf.is_some(), self.sme.is_some())
    }
}

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
    pub datatype: Field<'a, Param<'a>>,
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

/// Why a string is not a `column=[param]` unit mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColUnitError {
    /// There is no `=`.
    MissingEquals,
    /// The column name is not a valid [`Text`].
    Column(TextError),
    /// The unit is not a valid parameter. The offset is relative to the
    /// parameter, which starts after the first `=`.
    Param(ParamError),
}

impl fmt::Display for ColUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEquals => f.write_str("expected `column=[unit parameter]`"),
            Self::Column(e) => write!(f, "bad column name: {e}"),
            Self::Param(e) => write!(f, "bad unit: {e}"),
        }
    }
}

impl core::error::Error for ColUnitError {}

/// A `colunit-*` value: `column_name=[unit parameter]`.
///
/// # Examples
///
/// ```
/// use mztab_m::ColUnit;
///
/// let unit = ColUnit::parse("retention_time=[UO, UO:0000031, minute, ]")?;
/// assert_eq!(unit.column.as_str(), "retention_time");
/// assert_eq!(unit.param.name().as_str(), "minute");
/// assert_eq!(unit.to_string(), "retention_time=[UO, UO:0000031, minute, ]");
/// # Ok::<(), mztab_m::ColUnitError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColUnit<'a> {
    /// The column the unit applies to.
    pub column: Text<'a>,
    /// The unit.
    pub param: Param<'a>,
}

impl<'a> ColUnit<'a> {
    /// Parses `column=[param]`, splitting at the first `=`.
    ///
    /// # Errors
    ///
    /// Returns a [`ColUnitError`] if the `=` is missing or either side is
    /// malformed.
    pub fn parse(text: &'a str) -> Result<Self, ColUnitError> {
        let (column, param) = text.split_once('=').ok_or(ColUnitError::MissingEquals)?;
        Ok(Self {
            column: Text::new(column).map_err(ColUnitError::Column)?,
            param: Param::parse(param).map_err(ColUnitError::Param)?,
        })
    }
}

impl fmt::Display for ColUnit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.column, self.param)
    }
}

/// A table: its header and its rows.
///
/// `S` is the table's stable-column enum and `R` its row type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table<'a, S, R> {
    /// The header line, in source order.
    pub header: Header<'a, S>,
    /// The rows, in source order.
    pub rows: Vec<R>,
}

/// A table header: the ordered list of columns after the line prefix.
///
/// Source order is kept (DECISIONS D-010). A row stores its stable columns
/// by name, and every other column in the vector of its kind, in the order
/// those columns appear here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header<'a, S> {
    /// The columns, in source order.
    pub columns: Vec<Column<'a, S>>,
}

/// One column of a header.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Column<'a, S> {
    /// A column defined by the field reference, such as `SML_ID`.
    Stable(S),
    /// An `abundance_*` column.
    Abundance(AbundanceColumn),
    /// SME `id_confidence_measure[n]`.
    IdConfidence(IdConfidenceMeasureId),
    /// An `opt_` column.
    Opt(OptColumn<'a>),
    /// A column this crate does not recognise, kept as written.
    Unknown(Raw<'a>),
}

/// The kind of an `abundance_*` column and the element it refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbundanceColumn {
    /// `abundance_assay[n]` (SML and SMF).
    Assay(AssayId),
    /// `abundance_study_variable[n]` (SML).
    StudyVariable(StudyVariableId),
    /// `abundance_variation_study_variable[n]` (SML).
    VariationStudyVariable(StudyVariableId),
}

impl fmt::Display for AbundanceColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assay(id) => write!(f, "abundance_{id}"),
            Self::StudyVariable(id) => write!(f, "abundance_{id}"),
            Self::VariationStudyVariable(id) => write!(f, "abundance_variation_{id}"),
        }
    }
}

/// What an `opt_` column refers to: `opt_{identifier}_…`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptTarget {
    /// `global`: the value relates to all replicates.
    Global,
    /// `assay[n]`.
    Assay(AssayId),
    /// `study_variable[n]`.
    StudyVariable(StudyVariableId),
    /// `ms_run[n]`.
    MsRun(MsRunId),
}

impl fmt::Display for OptTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => f.write_str("global"),
            Self::Assay(id) => id.fmt(f),
            Self::StudyVariable(id) => id.fmt(f),
            Self::MsRun(id) => id.fmt(f),
        }
    }
}

/// An `opt_{identifier}_{name}` column.
///
/// `name` is everything after the identifier's trailing `_`. For a
/// CV-typed column it reads `cv_{accession}_{parameter_name}`; splitting it
/// is left to the validator, because accessions may themselves contain `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptColumn<'a> {
    /// What the column refers to.
    pub target: OptTarget,
    /// The rest of the column name.
    pub name: Text<'a>,
}

impl fmt::Display for OptColumn<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "opt_{}_{}", self.target, self.name)
    }
}

macro_rules! stable_columns {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $col:literal,)+ }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$vdoc])* #[doc = concat!("`", $col, "`.")] $variant,)+
        }

        impl $name {
            /// The stable columns in canonical order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The column label as written in the header. Case-sensitive.
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self { $(Self::$variant => $col,)+ }
            }

            /// The column with this label, if it is a stable column.
            #[must_use]
            pub fn from_label(label: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|c| c.label() == label)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.label())
            }
        }
    };
}

stable_columns! {
    /// The stable columns of the small molecule summary.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmlColumn;
    ///
    /// assert_eq!(SmlColumn::from_label("SMF_ID_REFS"), Some(SmlColumn::SmfIdRefs));
    /// assert_eq!(SmlColumn::ALL.first().map(|c| c.label()), Some("SML_ID"));
    /// assert_eq!(SmlColumn::from_label("sml_id"), None);
    /// ```
    SmlColumn {
        SmlId = "SML_ID",
        SmfIdRefs = "SMF_ID_REFS",
        DatabaseIdentifier = "database_identifier",
        ChemicalFormula = "chemical_formula",
        Smiles = "smiles",
        Inchi = "inchi",
        ChemicalName = "chemical_name",
        Uri = "uri",
        TheoreticalNeutralMass = "theoretical_neutral_mass",
        AdductIons = "adduct_ions",
        Reliability = "reliability",
        BestIdConfidenceMeasure = "best_id_confidence_measure",
        BestIdConfidenceValue = "best_id_confidence_value",
    }
}

stable_columns! {
    /// The stable columns of the small molecule feature table.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmfColumn;
    ///
    /// assert_eq!(SmfColumn::Charge.to_string(), "charge");
    /// ```
    SmfColumn {
        SmfId = "SMF_ID",
        SmeIdRefs = "SME_ID_REFS",
        SmeIdRefAmbiguityCode = "SME_ID_REF_ambiguity_code",
        AdductIon = "adduct_ion",
        Isotopomer = "isotopomer",
        ExpMassToCharge = "exp_mass_to_charge",
        Charge = "charge",
        RetentionTimeInSeconds = "retention_time_in_seconds",
        RetentionTimeInSecondsStart = "retention_time_in_seconds_start",
        RetentionTimeInSecondsEnd = "retention_time_in_seconds_end",
    }
}

stable_columns! {
    /// The stable columns of the small molecule evidence table.
    ///
    /// SME's `id_confidence_measure[n]` columns are not stable columns:
    /// they are [`Column::IdConfidence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmeColumn;
    ///
    /// assert_eq!(SmeColumn::ALL.last(), Some(&SmeColumn::Rank));
    /// ```
    SmeColumn {
        SmeId = "SME_ID",
        EvidenceInputId = "evidence_input_id",
        DatabaseIdentifier = "database_identifier",
        ChemicalFormula = "chemical_formula",
        Smiles = "smiles",
        Inchi = "inchi",
        ChemicalName = "chemical_name",
        Uri = "uri",
        DerivatizedForm = "derivatized_form",
        AdductIon = "adduct_ion",
        ExpMassToCharge = "exp_mass_to_charge",
        Charge = "charge",
        TheoreticalMassToCharge = "theoretical_mass_to_charge",
        SpectraRef = "spectra_ref",
        IdentificationMethod = "identification_method",
        MsLevel = "ms_level",
        Rank = "rank",
    }
}

macro_rules! row_ids {
    ($($(#[$doc:meta])* $name:ident;)+) => {$(
        $(#[$doc])*
        ///
        /// The field reference types it as an integer; this crate reads it as
        /// a non-negative 32-bit integer.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

row_ids! {
    /// An `SML_ID`.
    SmlId;
    /// An `SMF_ID`, also used in `SMF_ID_REFS`.
    SmfId;
    /// An `SME_ID`, also used in `SME_ID_REFS`.
    SmeId;
}

/// One SML row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmlRow<'a> {
    /// `SML_ID`.
    pub sml_id: Cell<'a, SmlId>,
    /// `SMF_ID_REFS`.
    pub smf_id_refs: Cell<'a, List<'a, SmfId>>,
    /// `database_identifier`.
    pub database_identifier: Cell<'a, List<'a, Text<'a>>>,
    /// `chemical_formula`.
    pub chemical_formula: Cell<'a, List<'a, Text<'a>>>,
    /// `smiles`.
    pub smiles: Cell<'a, List<'a, Text<'a>>>,
    /// `inchi`.
    pub inchi: Cell<'a, List<'a, Text<'a>>>,
    /// `chemical_name`.
    pub chemical_name: Cell<'a, List<'a, Text<'a>>>,
    /// `uri`.
    pub uri: Cell<'a, List<'a, Text<'a>>>,
    /// `theoretical_neutral_mass`.
    pub theoretical_neutral_mass: Cell<'a, List<'a, Decimal<'a>>>,
    /// `adduct_ions`.
    pub adduct_ions: Cell<'a, List<'a, Text<'a>>>,
    /// `reliability`.
    pub reliability: Cell<'a, Text<'a>>,
    /// `best_id_confidence_measure`.
    pub best_id_confidence_measure: Cell<'a, Param<'a>>,
    /// `best_id_confidence_value`.
    pub best_id_confidence_value: Cell<'a, Decimal<'a>>,
    /// The `abundance_*` columns, in header order.
    pub abundance: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
}

/// One SMF row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmfRow<'a> {
    /// `SMF_ID`.
    pub smf_id: Cell<'a, SmfId>,
    /// `SME_ID_REFS`.
    pub sme_id_refs: Cell<'a, List<'a, SmeId>>,
    /// `SME_ID_REF_ambiguity_code`.
    pub sme_id_ref_ambiguity_code: Cell<'a, AmbiguityCode>,
    /// `adduct_ion`.
    pub adduct_ion: Cell<'a, Text<'a>>,
    /// `isotopomer`.
    pub isotopomer: Cell<'a, Param<'a>>,
    /// `exp_mass_to_charge`.
    pub exp_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `charge`.
    pub charge: Cell<'a, i32>,
    /// `retention_time_in_seconds`.
    pub retention_time_in_seconds: Cell<'a, Decimal<'a>>,
    /// `retention_time_in_seconds_start`.
    pub retention_time_in_seconds_start: Cell<'a, Decimal<'a>>,
    /// `retention_time_in_seconds_end`.
    pub retention_time_in_seconds_end: Cell<'a, Decimal<'a>>,
    /// The `abundance_assay[n]` columns, in header order.
    pub abundance: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
}

/// One SME row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmeRow<'a> {
    /// `SME_ID`.
    pub sme_id: Cell<'a, SmeId>,
    /// `evidence_input_id`.
    pub evidence_input_id: Cell<'a, Text<'a>>,
    /// `database_identifier`.
    pub database_identifier: Cell<'a, Text<'a>>,
    /// `chemical_formula`.
    pub chemical_formula: Cell<'a, Text<'a>>,
    /// `smiles`.
    pub smiles: Cell<'a, Text<'a>>,
    /// `inchi`.
    pub inchi: Cell<'a, Text<'a>>,
    /// `chemical_name`.
    pub chemical_name: Cell<'a, Text<'a>>,
    /// `uri`.
    pub uri: Cell<'a, Text<'a>>,
    /// `derivatized_form`.
    pub derivatized_form: Cell<'a, Param<'a>>,
    /// `adduct_ion`.
    pub adduct_ion: Cell<'a, Text<'a>>,
    /// `exp_mass_to_charge`.
    pub exp_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `charge`.
    pub charge: Cell<'a, i32>,
    /// `theoretical_mass_to_charge`.
    pub theoretical_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `spectra_ref`.
    pub spectra_ref: Cell<'a, List<'a, SpectraRef<'a>>>,
    /// `identification_method`.
    pub identification_method: Cell<'a, Param<'a>>,
    /// `ms_level`.
    pub ms_level: Cell<'a, Param<'a>>,
    /// `rank`.
    pub rank: Cell<'a, u32>,
    /// The `id_confidence_measure[n]` columns, in header order.
    pub id_confidence: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
}

/// Why a string is not a spectra reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpectraRefError {
    /// It does not start with `ms_run[n]:` for a positive `n`.
    MsRun,
    /// The reference after the colon is not a valid [`Text`].
    Reference(TextError),
}

impl fmt::Display for SpectraRefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MsRun => f.write_str("expected `ms_run[n]:`"),
            Self::Reference(e) => write!(f, "bad spectrum reference: {e}"),
        }
    }
}

impl core::error::Error for SpectraRefError {}

/// One `ms_run[n]:{reference}` item of `spectra_ref`.
///
/// Whether `reference` matches the run's `id_format` is a validator rule.
///
/// # Examples
///
/// ```
/// use mztab_m::SpectraRef;
///
/// let r = SpectraRef::parse("ms_run[2]:index=7")?;
/// assert_eq!(r.ms_run.get(), 2);
/// assert_eq!(r.reference.as_str(), "index=7");
/// assert_eq!(r.to_string(), "ms_run[2]:index=7");
/// # Ok::<(), mztab_m::SpectraRefError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpectraRef<'a> {
    /// The run that holds the spectrum.
    pub ms_run: MsRunId,
    /// The spectrum's native id within the run.
    pub reference: Text<'a>,
}

impl<'a> SpectraRef<'a> {
    /// Parses `ms_run[n]:{reference}`.
    ///
    /// # Errors
    ///
    /// Returns a [`SpectraRefError`] if the run prefix or the reference is
    /// malformed.
    pub fn parse(text: &'a str) -> Result<Self, SpectraRefError> {
        let (run, reference) = text.split_once("]:").ok_or(SpectraRefError::MsRun)?;
        let ms_run = run
            .strip_prefix("ms_run[")
            .and_then(parse_index)
            .and_then(MsRunId::new)
            .ok_or(SpectraRefError::MsRun)?;
        let reference = Text::new(reference).map_err(SpectraRefError::Reference)?;
        Ok(Self { ms_run, reference })
    }
}

/// Parses a 1-based mzTab index: ASCII digits only, no sign, no leading zero.
#[must_use]
pub(crate) fn parse_index(text: &str) -> Option<u32> {
    let canonical =
        !text.is_empty() && !text.starts_with('0') && text.bytes().all(|b| b.is_ascii_digit());
    canonical.then(|| text.parse().ok()).flatten()
}

impl fmt::Display for SpectraRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ms_run, self.reference)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{
        AbundanceColumn, ColUnit, ColUnitError, Header, MzTab, OptColumn, OptTarget, SmeColumn,
        SmfColumn, SmlColumn, SmlId, SpectraRef, SpectraRefError, Table, parse_index,
    };
    use crate::id::{AssayId, MsRunId, StudyVariableId};
    use crate::param::{ParamError, ParamErrorKind};
    use crate::value::{Text, TextError};
    use crate::vocab::{Profile, ProhibitedProfile};
    use alloc::string::ToString as _;
    use alloc::vec::Vec;

    #[test]
    fn stable_columns_round_trip_labels() {
        for c in SmlColumn::ALL {
            assert_eq!(SmlColumn::from_label(c.label()), Some(*c), "{c}");
        }
        for c in SmfColumn::ALL {
            assert_eq!(SmfColumn::from_label(&c.to_string()), Some(*c), "{c}");
        }
        for c in SmeColumn::ALL {
            assert_eq!(SmeColumn::from_label(c.label()), Some(*c), "{c}");
        }
        assert_eq!(
            (
                SmlColumn::ALL.len(),
                SmfColumn::ALL.len(),
                SmeColumn::ALL.len()
            ),
            (13, 10, 17),
            "FREF counts"
        );
        assert_eq!(
            SmeColumn::from_label("id_confidence_measure[1]"),
            None,
            "not stable"
        );
    }

    #[test]
    fn column_display() {
        let assay = AssayId::new(1).expect("valid fixture");
        let sv = StudyVariableId::new(2).expect("valid fixture");
        let run = MsRunId::new(3).expect("valid fixture");
        assert_eq!(
            AbundanceColumn::Assay(assay).to_string(),
            "abundance_assay[1]",
            "assay"
        );
        assert_eq!(
            AbundanceColumn::StudyVariable(sv).to_string(),
            "abundance_study_variable[2]",
            "sv"
        );
        assert_eq!(
            AbundanceColumn::VariationStudyVariable(sv).to_string(),
            "abundance_variation_study_variable[2]",
            "variation"
        );
        let name = Text::new("cv_MS:1002217_decoy_peptide").expect("valid fixture");
        let targets = [
            (OptTarget::Global, "opt_global_"),
            (OptTarget::Assay(assay), "opt_assay[1]_"),
            (OptTarget::StudyVariable(sv), "opt_study_variable[2]_"),
            (OptTarget::MsRun(run), "opt_ms_run[3]_"),
        ];
        for (target, prefix) in targets {
            let col = OptColumn {
                target,
                name: name.clone(),
            };
            assert_eq!(
                col.to_string(),
                [prefix, name.as_str()].concat(),
                "{prefix}"
            );
        }
        assert_eq!(SmlId(4).to_string(), "4", "row id");
    }

    #[test]
    fn colunits() {
        assert_eq!(
            ColUnit::parse("x"),
            Err(ColUnitError::MissingEquals),
            "no ="
        );
        assert_eq!(
            ColUnit::parse("=[,,n,]"),
            Err(ColUnitError::Column(TextError::Empty)),
            "no column"
        );
        let bad = ParamError {
            kind: ParamErrorKind::MissingOpenBracket,
            offset: 0,
        };
        assert_eq!(
            ColUnit::parse("a=b"),
            Err(ColUnitError::Param(bad)),
            "bad param"
        );
        assert_eq!(
            ColUnit::parse("a=[,,n=1,]").map(|u| u.to_string()),
            Ok("a=[, , n=1, ]".into()),
            "first = splits"
        );
        assert_eq!(
            ColUnitError::MissingEquals.to_string(),
            "expected `column=[unit parameter]`",
            "msg"
        );
        assert_eq!(
            ColUnitError::Column(TextError::Empty).to_string(),
            "bad column name: text is empty",
            "msg"
        );
        assert_eq!(
            ColUnitError::Param(bad).to_string(),
            "bad unit: expected `[` (at byte 0)",
            "msg"
        );
    }

    #[test]
    fn spectra_refs() {
        for bad in [
            "index=1",
            "ms_run[0]:x",
            "ms_run[01]:x",
            "run[1]:x",
            "ms_run[x]:y",
            "ms_run[]:y",
        ] {
            assert_eq!(SpectraRef::parse(bad), Err(SpectraRefError::MsRun), "{bad}");
        }
        assert_eq!(
            SpectraRef::parse("ms_run[1]:"),
            Err(SpectraRefError::Reference(TextError::Empty)),
            "empty"
        );
        assert_eq!(
            SpectraRefError::MsRun.to_string(),
            "expected `ms_run[n]:`",
            "msg"
        );
        assert_eq!(
            SpectraRefError::Reference(TextError::Empty).to_string(),
            "bad spectrum reference: text is empty",
            "msg"
        );
        let ok = SpectraRef::parse("ms_run[12]:scan=3:x").expect("valid fixture");
        assert_eq!(
            (ok.ms_run.get(), ok.reference.as_str()),
            (12, "scan=3:x"),
            "parts"
        );
        assert_eq!(ok.to_string(), "ms_run[12]:scan=3:x", "display");
        assert_eq!(parse_index(""), None, "empty index");
        assert_eq!(parse_index("4294967296"), None, "overflow");
        assert_eq!(parse_index("4294967295"), Some(u32::MAX), "max");
    }

    #[test]
    fn inferred_profile_counts_header_only_tables() {
        fn empty<S, R>() -> Table<'static, S, R> {
            Table {
                header: Header {
                    columns: Vec::new(),
                },
                rows: Vec::new(),
            }
        }
        let mut m = MzTab {
            sme: Some(empty()),
            ..MzTab::default()
        };
        assert_eq!(m.inferred_profile(), Err(ProhibitedProfile::ME), "M+E");
        m.smf = Some(empty());
        assert_eq!(m.inferred_profile(), Ok(Profile::MFE), "M+F+E");
    }
}
