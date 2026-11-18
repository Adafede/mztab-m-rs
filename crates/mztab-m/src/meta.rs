//! Metadata keys ([`MtdKey`]), typed metadata values ([`MtdValue`]) and
//! their placement into [`Metadata`].
//!
//! ```
//! use mztab_m::{AssayKey, MtdKey};
//!
//! let key = MtdKey::parse("assay[2]-ms_run_ref").ok_or("unknown key")?;
//! assert!(matches!(key, MtdKey::Assay(id, AssayKey::MsRunRef) if id.get() == 2));
//! assert_eq!(key.to_string(), "assay[2]-ms_run_ref");
//! assert_eq!(MtdKey::parse("assay[0]"), None);
//! # Ok::<(), &str>(())
//! ```

use core::fmt;

use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
use crate::model::{ColUnit, Field, List, Metadata, parse_index};
use crate::param::Param;
use crate::value::{Cell, Raw, Text};
use crate::vocab::{Datatype, Profile, PublicationItem, Version};

/// One `name` or `name[n]` part of a key, split at `-`.
type Segment<'k> = (&'k str, Option<u32>);

/// Splits `name[n]` into its name and index. `None` if the brackets or the
/// index are malformed.
fn segment(text: &str) -> Option<Segment<'_>> {
    match text.split_once('[') {
        None => Some((text, None)),
        Some((name, rest)) => {
            let index = rest.strip_suffix(']').and_then(parse_index)?;
            Some((name, Some(index)))
        }
    }
}

/// How a sub-key's variant is written: a label, and whether it takes `[m]`.
macro_rules! sub_keys {
    ($(
        $(#[$doc:meta])*
        $name:ident {
            $( $(#[$vdoc:meta])* $variant:ident $(($ix:ident))? = $label:tt, )+
        }
    )+) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$vdoc])* $variant $(($ix))?, )+
        }

        impl $name {
            /// Parses the part after the element, e.g. `custom[2]` in
            /// `assay[1]-custom[2]`. `None` is the bare element.
            fn parse(rest: Option<Segment<'_>>) -> Option<Self> {
                $( sub_keys!(@parse rest, $variant $(($ix))?, $label); )+
                None
            }

            /// Writes `-label` or `-label[m]`, or nothing for the bare element.
            fn fmt_suffix(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $( sub_keys!(@pat m, $variant $(($ix))?) => sub_keys!(@fmt f, m, $label $(, $ix)?), )+
                }
            }
        }
    )+};

    (@parse $rest:ident, $variant:ident, "") => {
        if $rest.is_none() {
            return Some(Self::$variant);
        }
    };
    (@parse $rest:ident, $variant:ident, $label:literal) => {
        if $rest == Some(($label, None)) {
            return Some(Self::$variant);
        }
    };
    (@parse $rest:ident, $variant:ident($ix:ident), $label:literal) => {
        if let Some(($label, Some(m))) = $rest {
            return $ix::new(m).map(Self::$variant);
        }
    };
    (@pat $m:ident, $variant:ident) => { Self::$variant };
    (@pat $m:ident, $variant:ident($ix:ident)) => { Self::$variant($m) };
    (@fmt $f:ident, $m:ident, "") => { Ok(()) };
    (@fmt $f:ident, $m:ident, $label:literal) => { $f.write_str(concat!("-", $label)) };
    (@fmt $f:ident, $m:ident, $label:literal, $ix:ident) => {
        write!($f, concat!("-", $label, "[{}]"), $m.get())
    };
}

sub_keys! {
    /// The fields of `instrument[n]`.
    InstrumentKey {
        /// `-name`.
        Name = "name",
        /// `-source`.
        Source = "source",
        /// `-analyzer[m]`.
        Analyzer(Ix) = "analyzer",
        /// `-detector`.
        Detector = "detector",
    }

    /// `software[n]` and its settings.
    SoftwareKey {
        /// `software[n]` itself.
        Software = "",
        /// `-setting[m]`.
        Setting(Ix) = "setting",
    }

    /// The fields of `contact[n]`.
    ContactKey {
        /// `-name`.
        Name = "name",
        /// `-affiliation`.
        Affiliation = "affiliation",
        /// `-email`.
        Email = "email",
        /// `-orcid`.
        Orcid = "orcid",
    }

    /// `sample[n]` and its fields.
    SampleKey {
        /// `sample[n]` itself: the name.
        Name = "",
        /// `-species[m]`.
        Species(Ix) = "species",
        /// `-tissue[m]`.
        Tissue(Ix) = "tissue",
        /// `-cell_type[m]`.
        CellType(Ix) = "cell_type",
        /// `-disease[m]`.
        Disease(Ix) = "disease",
        /// `-description`.
        Description = "description",
        /// `-custom[m]`.
        Custom(Ix) = "custom",
    }

    /// The fields of `ms_run[n]`.
    MsRunKey {
        /// `-location`.
        Location = "location",
        /// `-instrument_ref`.
        InstrumentRef = "instrument_ref",
        /// `-format`.
        Format = "format",
        /// `-id_format`.
        IdFormat = "id_format",
        /// `-fragmentation_method[m]`.
        FragmentationMethod(Ix) = "fragmentation_method",
        /// `-scan_polarity[m]`.
        ScanPolarity(Ix) = "scan_polarity",
        /// `-hash`.
        Hash = "hash",
        /// `-hash_method`.
        HashMethod = "hash_method",
        /// `-parameters`.
        Parameters = "parameters",
        /// `-usi_identifier` (2.0 only).
        UsiIdentifier = "usi_identifier",
    }

    /// `assay[n]` and its fields.
    AssayKey {
        /// `assay[n]` itself: the name.
        Name = "",
        /// `-custom[m]`.
        Custom(Ix) = "custom",
        /// `-external_uri`.
        ExternalUri = "external_uri",
        /// `-sample_ref`.
        SampleRef = "sample_ref",
        /// `-ms_run_ref`.
        MsRunRef = "ms_run_ref",
        /// `-protocol_refs`.
        ProtocolRefs = "protocol_refs",
        /// `-parameters`.
        Parameters = "parameters",
    }

    /// `study_variable[n]` and its fields.
    StudyVariableKey {
        /// `study_variable[n]` itself: the name.
        Name = "",
        /// `-assay_refs`.
        AssayRefs = "assay_refs",
        /// `-ms_run_refs`.
        MsRunRefs = "ms_run_refs",
        /// `-description`.
        Description = "description",
        /// `-average_function`.
        AverageFunction = "average_function",
        /// `-variation_function`.
        VariationFunction = "variation_function",
        /// `-factors` (2.0 only).
        Factors = "factors",
    }

    /// `study_variable_group[n]` and its fields.
    StudyVariableGroupKey {
        /// `study_variable_group[n]` itself.
        Group = "",
        /// `-description`.
        Description = "description",
        /// `-type`.
        Type = "type",
        /// `-datatype`.
        Datatype = "datatype",
        /// `-unit`.
        Unit = "unit",
        /// `-study_variable_refs`.
        StudyVariableRefs = "study_variable_refs",
    }

    /// The fields of `protocol[n]`.
    ProtocolKey {
        /// `-name`.
        Name = "name",
        /// `-type`.
        Type = "type",
        /// `-description`.
        Description = "description",
        /// `-parameters`.
        Parameters = "parameters",
    }

    /// The fields of `cv[n]`.
    CvKey {
        /// `-label`.
        Label = "label",
        /// `-full_name`.
        FullName = "full_name",
        /// `-version`.
        Version = "version",
        /// `-uri`.
        Uri = "uri",
    }

    /// `database[n]` and its fields.
    DatabaseKey {
        /// `database[n]` itself.
        Database = "",
        /// `-prefix`.
        Prefix = "prefix",
        /// `-version`.
        Version = "version",
        /// `-uri`.
        Uri = "uri",
    }
}

/// A metadata key: the second cell of an `MTD` line.
///
/// Keys are case-sensitive (SPEC §6) and indices are positive integers
/// without sign or leading zeros. `Display` writes the key back exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MtdKey {
    /// `mzTab-version`.
    Version,
    /// `mzTab-ID`.
    Id,
    /// `mzTab-profile`.
    Profile,
    /// `title`.
    Title,
    /// `description`.
    Description,
    /// `sample_processing[n]`.
    SampleProcessing(Ix),
    /// `instrument[n]-…`.
    Instrument(InstrumentId, InstrumentKey),
    /// `software[n]…`.
    Software(SoftwareId, SoftwareKey),
    /// `publication[n]`.
    Publication(PublicationId),
    /// `contact[n]-…`.
    Contact(ContactId, ContactKey),
    /// `uri[n]`.
    Uri(Ix),
    /// `external_study_uri[n]`.
    ExternalStudyUri(Ix),
    /// `quantification_method`.
    QuantificationMethod,
    /// `sample[n]…`.
    Sample(SampleId, SampleKey),
    /// `ms_run[n]-…`.
    MsRun(MsRunId, MsRunKey),
    /// `assay[n]…`.
    Assay(AssayId, AssayKey),
    /// `study_variable[n]…`.
    StudyVariable(StudyVariableId, StudyVariableKey),
    /// `study_variable_group[n]…`.
    StudyVariableGroup(StudyVariableGroupId, StudyVariableGroupKey),
    /// `protocol[n]-…`.
    Protocol(ProtocolId, ProtocolKey),
    /// `custom[n]`.
    Custom(Ix),
    /// `cv[n]-…`.
    Cv(CvId, CvKey),
    /// `database[n]…`.
    Database(DatabaseId, DatabaseKey),
    /// `derivatization_agent[n]`.
    DerivatizationAgent(Ix),
    /// `small_molecule-quantification_unit`.
    SmallMoleculeQuantificationUnit,
    /// `small_molecule_feature-quantification_unit`.
    SmallMoleculeFeatureQuantificationUnit,
    /// `small_molecule-identification_reliability`.
    SmallMoleculeIdentificationReliability,
    /// `id_confidence_measure[n]`.
    IdConfidenceMeasure(IdConfidenceMeasureId),
    /// `colunit-small_molecule`. May repeat.
    ColunitSmallMolecule,
    /// `colunit-small_molecule_feature`. May repeat.
    ColunitSmallMoleculeFeature,
    /// `colunit-small_molecule_evidence`. May repeat.
    ColunitSmallMoleculeEvidence,
}

impl MtdKey {
    /// Parses a metadata key, or returns `None` if it is not one this crate
    /// knows.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let first = segment(parts.next()?)?;
        let second = parts.next().map(segment);
        if parts.next().is_some() {
            return None;
        }
        let second = match second {
            None => None,
            Some(s) => Some(s?),
        };
        Self::from_segments(first, second)
    }

    fn from_segments(first: Segment<'_>, rest: Option<Segment<'_>>) -> Option<Self> {
        let simple = match (first, rest) {
            (("mzTab", None), Some(("version", None))) => Some(Self::Version),
            (("mzTab", None), Some(("ID", None))) => Some(Self::Id),
            (("mzTab", None), Some(("profile", None))) => Some(Self::Profile),
            (("title", None), None) => Some(Self::Title),
            (("description", None), None) => Some(Self::Description),
            (("quantification_method", None), None) => Some(Self::QuantificationMethod),
            (("small_molecule", None), Some(("quantification_unit", None))) => {
                Some(Self::SmallMoleculeQuantificationUnit)
            }
            (("small_molecule_feature", None), Some(("quantification_unit", None))) => {
                Some(Self::SmallMoleculeFeatureQuantificationUnit)
            }
            (("small_molecule", None), Some(("identification_reliability", None))) => {
                Some(Self::SmallMoleculeIdentificationReliability)
            }
            (("colunit", None), Some(("small_molecule", None))) => Some(Self::ColunitSmallMolecule),
            (("colunit", None), Some(("small_molecule_feature", None))) => {
                Some(Self::ColunitSmallMoleculeFeature)
            }
            (("colunit", None), Some(("small_molecule_evidence", None))) => {
                Some(Self::ColunitSmallMoleculeEvidence)
            }
            _ => None,
        };
        if simple.is_some() {
            return simple;
        }
        let (name, Some(n)) = first else {
            return None;
        };
        // Elements whose key is just `element[n]`.
        if rest.is_none() {
            let indexed = match name {
                "sample_processing" => Ix::new(n).map(Self::SampleProcessing),
                "publication" => PublicationId::new(n).map(Self::Publication),
                "uri" => Ix::new(n).map(Self::Uri),
                "external_study_uri" => Ix::new(n).map(Self::ExternalStudyUri),
                "custom" => Ix::new(n).map(Self::Custom),
                "derivatization_agent" => Ix::new(n).map(Self::DerivatizationAgent),
                "id_confidence_measure" => {
                    IdConfidenceMeasureId::new(n).map(Self::IdConfidenceMeasure)
                }
                _ => None,
            };
            if indexed.is_some() {
                return indexed;
            }
        }
        match name {
            "instrument" => Some(Self::Instrument(
                InstrumentId::new(n)?,
                InstrumentKey::parse(rest)?,
            )),
            "software" => Some(Self::Software(
                SoftwareId::new(n)?,
                SoftwareKey::parse(rest)?,
            )),
            "contact" => Some(Self::Contact(ContactId::new(n)?, ContactKey::parse(rest)?)),
            "sample" => Some(Self::Sample(SampleId::new(n)?, SampleKey::parse(rest)?)),
            "ms_run" => Some(Self::MsRun(MsRunId::new(n)?, MsRunKey::parse(rest)?)),
            "assay" => Some(Self::Assay(AssayId::new(n)?, AssayKey::parse(rest)?)),
            "study_variable" => Some(Self::StudyVariable(
                StudyVariableId::new(n)?,
                StudyVariableKey::parse(rest)?,
            )),
            "study_variable_group" => Some(Self::StudyVariableGroup(
                StudyVariableGroupId::new(n)?,
                StudyVariableGroupKey::parse(rest)?,
            )),
            "protocol" => Some(Self::Protocol(
                ProtocolId::new(n)?,
                ProtocolKey::parse(rest)?,
            )),
            "cv" => Some(Self::Cv(CvId::new(n)?, CvKey::parse(rest)?)),
            "database" => Some(Self::Database(
                DatabaseId::new(n)?,
                DatabaseKey::parse(rest)?,
            )),
            _ => None,
        }
    }

    /// Whether the key may appear more than once (`colunit-*` only).
    #[must_use]
    pub const fn repeats(self) -> bool {
        matches!(
            self,
            Self::ColunitSmallMolecule
                | Self::ColunitSmallMoleculeFeature
                | Self::ColunitSmallMoleculeEvidence
        )
    }

    /// The kind of value the key takes.
    #[must_use]
    pub const fn kind(self) -> ValueKind {
        use ValueKind as V;
        match self {
            Self::Version => V::Version,
            Self::Profile => V::Profile,
            Self::Id
            | Self::Title
            | Self::Description
            | Self::Uri(_)
            | Self::ExternalStudyUri(_)
            | Self::Software(_, SoftwareKey::Setting(_))
            | Self::Contact(..)
            | Self::Sample(_, SampleKey::Name | SampleKey::Description)
            | Self::MsRun(_, MsRunKey::Location | MsRunKey::Hash | MsRunKey::UsiIdentifier)
            | Self::Assay(_, AssayKey::Name | AssayKey::ExternalUri)
            | Self::StudyVariable(_, StudyVariableKey::Name | StudyVariableKey::Description)
            | Self::StudyVariableGroup(_, StudyVariableGroupKey::Description)
            | Self::Protocol(_, ProtocolKey::Name | ProtocolKey::Description)
            | Self::Cv(..)
            | Self::Database(_, DatabaseKey::Prefix | DatabaseKey::Version | DatabaseKey::Uri) => {
                V::Text
            }
            Self::QuantificationMethod
            | Self::SmallMoleculeQuantificationUnit
            | Self::SmallMoleculeFeatureQuantificationUnit
            | Self::SmallMoleculeIdentificationReliability
            | Self::Instrument(
                _,
                InstrumentKey::Name | InstrumentKey::Source | InstrumentKey::Detector,
            )
            | Self::Software(_, SoftwareKey::Software)
            | Self::MsRun(_, MsRunKey::Format | MsRunKey::IdFormat | MsRunKey::HashMethod)
            | Self::StudyVariable(
                _,
                StudyVariableKey::AverageFunction | StudyVariableKey::VariationFunction,
            )
            | Self::StudyVariableGroup(
                _,
                StudyVariableGroupKey::Group
                | StudyVariableGroupKey::Type
                | StudyVariableGroupKey::Unit,
            )
            | Self::Protocol(_, ProtocolKey::Type)
            | Self::Database(_, DatabaseKey::Database) => V::Param,
            Self::SampleProcessing(_)
            | Self::Instrument(_, InstrumentKey::Analyzer(_))
            | Self::Custom(_)
            | Self::DerivatizationAgent(_)
            | Self::IdConfidenceMeasure(_)
            | Self::Sample(
                _,
                SampleKey::Species(_)
                | SampleKey::Tissue(_)
                | SampleKey::CellType(_)
                | SampleKey::Disease(_)
                | SampleKey::Custom(_),
            )
            | Self::MsRun(
                _,
                MsRunKey::FragmentationMethod(_) | MsRunKey::ScanPolarity(_) | MsRunKey::Parameters,
            )
            | Self::Assay(_, AssayKey::Custom(_) | AssayKey::Parameters)
            | Self::StudyVariable(_, StudyVariableKey::Factors)
            | Self::Protocol(_, ProtocolKey::Parameters) => V::ParamList,
            Self::StudyVariableGroup(_, StudyVariableGroupKey::Datatype) => V::Datatype,
            Self::Publication(_) => V::Publication,
            Self::MsRun(_, MsRunKey::InstrumentRef) => V::InstrumentRef,
            Self::Assay(_, AssayKey::SampleRef) => V::SampleRef,
            Self::Assay(_, AssayKey::MsRunRef)
            | Self::StudyVariable(_, StudyVariableKey::MsRunRefs) => V::MsRunRefs,
            Self::Assay(_, AssayKey::ProtocolRefs) => V::ProtocolRefs,
            Self::StudyVariable(_, StudyVariableKey::AssayRefs) => V::AssayRefs,
            Self::StudyVariableGroup(_, StudyVariableGroupKey::StudyVariableRefs) => {
                V::StudyVariableRefs
            }
            Self::ColunitSmallMolecule
            | Self::ColunitSmallMoleculeFeature
            | Self::ColunitSmallMoleculeEvidence => V::ColUnit,
        }
    }
}

impl fmt::Display for MtdKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Version => f.write_str("mzTab-version"),
            Self::Id => f.write_str("mzTab-ID"),
            Self::Profile => f.write_str("mzTab-profile"),
            Self::Title => f.write_str("title"),
            Self::Description => f.write_str("description"),
            Self::QuantificationMethod => f.write_str("quantification_method"),
            Self::SmallMoleculeQuantificationUnit => {
                f.write_str("small_molecule-quantification_unit")
            }
            Self::SmallMoleculeFeatureQuantificationUnit => {
                f.write_str("small_molecule_feature-quantification_unit")
            }
            Self::SmallMoleculeIdentificationReliability => {
                f.write_str("small_molecule-identification_reliability")
            }
            Self::ColunitSmallMolecule => f.write_str("colunit-small_molecule"),
            Self::ColunitSmallMoleculeFeature => f.write_str("colunit-small_molecule_feature"),
            Self::ColunitSmallMoleculeEvidence => f.write_str("colunit-small_molecule_evidence"),
            Self::SampleProcessing(n) => write!(f, "sample_processing[{}]", n.get()),
            Self::Publication(n) => n.fmt(f),
            Self::Uri(n) => write!(f, "uri[{}]", n.get()),
            Self::ExternalStudyUri(n) => write!(f, "external_study_uri[{}]", n.get()),
            Self::Custom(n) => write!(f, "custom[{}]", n.get()),
            Self::DerivatizationAgent(n) => write!(f, "derivatization_agent[{}]", n.get()),
            Self::IdConfidenceMeasure(n) => n.fmt(f),
            Self::Instrument(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Software(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Contact(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Sample(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::MsRun(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Assay(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::StudyVariable(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::StudyVariableGroup(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Protocol(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Cv(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
            Self::Database(n, k) => {
                n.fmt(f)?;
                k.fmt_suffix(f)
            }
        }
    }
}

/// The type of value a metadata key takes, from the field reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// `major.minor.patch-M`.
    Version,
    /// A profile name.
    Profile,
    /// A study-variable group datatype.
    Datatype,
    /// A string.
    Text,
    /// One parameter.
    Param,
    /// A `|`-separated parameter list.
    ParamList,
    /// A `|`-separated list of publication items.
    Publication,
    /// `instrument[n]`.
    InstrumentRef,
    /// `sample[n]`.
    SampleRef,
    /// A `|`-separated list of `ms_run[n]`.
    MsRunRefs,
    /// A `|`-separated list of `protocol[n]`.
    ProtocolRefs,
    /// A `|`-separated list of `assay[n]`.
    AssayRefs,
    /// A `|`-separated list of `study_variable[n]`.
    StudyVariableRefs,
    /// `column=[unit]`.
    ColUnit,
}

/// A typed metadata value. Each variant matches one [`ValueKind`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MtdValue<'a> {
    /// [`ValueKind::Version`].
    Version(Cell<'a, Version>),
    /// [`ValueKind::Profile`].
    Profile(Cell<'a, Profile>),
    /// [`ValueKind::Datatype`].
    Datatype(Cell<'a, Datatype>),
    /// [`ValueKind::Text`].
    Text(Cell<'a, Text<'a>>),
    /// [`ValueKind::Param`].
    Param(Cell<'a, Param<'a>>),
    /// [`ValueKind::ParamList`].
    ParamList(Cell<'a, List<'a, Param<'a>>>),
    /// [`ValueKind::Publication`].
    Publication(Cell<'a, List<'a, PublicationItem<'a>>>),
    /// [`ValueKind::InstrumentRef`].
    InstrumentRef(Cell<'a, InstrumentId>),
    /// [`ValueKind::SampleRef`].
    SampleRef(Cell<'a, SampleId>),
    /// [`ValueKind::MsRunRefs`].
    MsRunRefs(Cell<'a, List<'a, MsRunId>>),
    /// [`ValueKind::ProtocolRefs`].
    ProtocolRefs(Cell<'a, List<'a, ProtocolId>>),
    /// [`ValueKind::AssayRefs`].
    AssayRefs(Cell<'a, List<'a, AssayId>>),
    /// [`ValueKind::StudyVariableRefs`].
    StudyVariableRefs(Cell<'a, List<'a, StudyVariableId>>),
    /// [`ValueKind::ColUnit`].
    ColUnit(Cell<'a, ColUnit<'a>>),
}

/// Why [`Metadata::insert`] did not place a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InsertError {
    /// The key already has a value. Only `colunit-*` keys repeat.
    Occupied,
    /// The value is not of the key's [`ValueKind`].
    WrongKind,
}

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
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per key family: the key-to-field table reads best in one place"
    )]
    pub fn insert(&mut self, key: MtdKey, value: MtdValue<'a>) -> Result<(), InsertError> {
        match key {
            MtdKey::Version => put(&mut self.version, expect_kind!(value, Version)),
            MtdKey::Profile => put(&mut self.profile, expect_kind!(value, Profile)),
            MtdKey::Id => put(&mut self.id, expect_kind!(value, Text)),
            MtdKey::Title => put(&mut self.title, expect_kind!(value, Text)),
            MtdKey::Description => put(&mut self.description, expect_kind!(value, Text)),
            MtdKey::QuantificationMethod => {
                put(&mut self.quantification_method, expect_kind!(value, Param))
            }
            MtdKey::SmallMoleculeQuantificationUnit => put(
                &mut self.small_molecule_quantification_unit,
                expect_kind!(value, Param),
            ),
            MtdKey::SmallMoleculeFeatureQuantificationUnit => put(
                &mut self.small_molecule_feature_quantification_unit,
                expect_kind!(value, Param),
            ),
            MtdKey::SmallMoleculeIdentificationReliability => put(
                &mut self.small_molecule_identification_reliability,
                expect_kind!(value, Param),
            ),
            MtdKey::ColunitSmallMolecule => {
                self.colunit_small_molecule
                    .push(expect_kind!(value, ColUnit));
                Ok(())
            }
            MtdKey::ColunitSmallMoleculeFeature => {
                self.colunit_small_molecule_feature
                    .push(expect_kind!(value, ColUnit));
                Ok(())
            }
            MtdKey::ColunitSmallMoleculeEvidence => {
                self.colunit_small_molecule_evidence
                    .push(expect_kind!(value, ColUnit));
                Ok(())
            }
            MtdKey::SampleProcessing(m) => put_at(
                &mut self.sample_processing,
                m,
                expect_kind!(value, ParamList),
            ),
            MtdKey::Publication(n) => {
                put_at(&mut self.publication, n, expect_kind!(value, Publication))
            }
            MtdKey::Uri(m) => put_at(&mut self.uri, m, expect_kind!(value, Text)),
            MtdKey::ExternalStudyUri(m) => {
                put_at(&mut self.external_study_uri, m, expect_kind!(value, Text))
            }
            MtdKey::Custom(m) => put_at(&mut self.custom, m, expect_kind!(value, ParamList)),
            MtdKey::DerivatizationAgent(m) => put_at(
                &mut self.derivatization_agent,
                m,
                expect_kind!(value, ParamList),
            ),
            MtdKey::IdConfidenceMeasure(n) => put_at(
                &mut self.id_confidence_measure,
                n,
                expect_kind!(value, ParamList),
            ),
            MtdKey::Instrument(n, k) => self.instrument_value(n, k, value),
            MtdKey::Software(n, k) => {
                let software = self.software.get_or_insert_default(n);
                match k {
                    SoftwareKey::Software => put(&mut software.param, expect_kind!(value, Param)),
                    SoftwareKey::Setting(m) => {
                        put_at(&mut software.setting, m, expect_kind!(value, Text))
                    }
                }
            }
            MtdKey::Contact(n, k) => {
                let cell = expect_kind!(value, Text);
                let contact = self.contact.get_or_insert_default(n);
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
            MtdKey::Sample(n, k) => self.sample_value(n, k, value),
            MtdKey::MsRun(n, k) => self.ms_run_value(n, k, value),
            MtdKey::Assay(n, k) => self.assay_value(n, k, value),
            MtdKey::StudyVariable(n, k) => self.study_variable_value(n, k, value),
            MtdKey::StudyVariableGroup(n, k) => self.study_variable_group_value(n, k, value),
            MtdKey::Protocol(n, k) => {
                let protocol = self.protocol.get_or_insert_default(n);
                match k {
                    ProtocolKey::Name => put(&mut protocol.name, expect_kind!(value, Text)),
                    ProtocolKey::Description => {
                        put(&mut protocol.description, expect_kind!(value, Text))
                    }
                    ProtocolKey::Type => put(&mut protocol.type_, expect_kind!(value, Param)),
                    ProtocolKey::Parameters => {
                        put(&mut protocol.parameters, expect_kind!(value, ParamList))
                    }
                }
            }
            MtdKey::Cv(n, k) => {
                let cell = expect_kind!(value, Text);
                let cv = self.cv.get_or_insert_default(n);
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
            MtdKey::Database(n, k) => {
                let database = self.database.get_or_insert_default(n);
                match k {
                    DatabaseKey::Database => put(&mut database.param, expect_kind!(value, Param)),
                    DatabaseKey::Prefix => put(&mut database.prefix, expect_kind!(value, Text)),
                    DatabaseKey::Version => put(&mut database.version, expect_kind!(value, Text)),
                    DatabaseKey::Uri => put(&mut database.uri, expect_kind!(value, Text)),
                }
            }
        }
    }

    fn instrument_value(
        &mut self,
        n: InstrumentId,
        k: InstrumentKey,
        value: MtdValue<'a>,
    ) -> Result<(), InsertError> {
        let instrument = self.instrument.get_or_insert_default(n);
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
        let sample = self.sample.get_or_insert_default(n);
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
        let run = self.ms_run.get_or_insert_default(n);
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
        let assay = self.assay.get_or_insert_default(n);
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
        let sv = self.study_variable.get_or_insert_default(n);
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
        let group = self.study_variable_group.get_or_insert_default(n);
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

/// One `MTD` line: the key and value as written, and their typed form.
///
/// `typed` is `None` when the key is unknown; the line then belongs in
/// [`Metadata::unknown`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MtdEntry<'a> {
    /// The key cell, as written.
    pub key: Raw<'a>,
    /// The value cell, as written (empty if the line has no value cell).
    pub value: Raw<'a>,
    /// Everything after the value cell, starting with its tab; usually
    /// empty, sometimes a run of empty cells.
    pub trailing: Raw<'a>,
    /// The parsed key and typed value.
    pub typed: Option<(MtdKey, MtdValue<'a>)>,
}
