//! Metadata key parsing, formatting, and value-kind classification.

use core::fmt;
use core::num::NonZeroU32;
use core::str::FromStr;

use super::subkey::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InstrumentKey, MsRunKey, ProtocolKey, SampleKey,
    Segment, SoftwareKey, StudyVariableGroupKey, StudyVariableKey, fmt_elem, segment,
};
use crate::id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, InstrumentId, Ix, MsRunId,
    ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId, StudyVariableId,
};
use crate::vocab::VocabError;

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
        if let Some(simple) = Self::simple_key(first, rest) {
            return Some(simple);
        }
        let (name, Some(n)) = first else {
            return None;
        };
        let n = NonZeroU32::new(n)?;
        if rest.is_none()
            && let Some(scalar) = Self::bare_indexed_key(name, n)
        {
            return Some(scalar);
        }
        Self::element_key(name, n, rest)
    }

    fn simple_key(first: Segment<'_>, rest: Option<Segment<'_>>) -> Option<Self> {
        match (first, rest) {
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
        }
    }

    fn bare_indexed_key(name: &str, n: NonZeroU32) -> Option<Self> {
        Some(match name {
            "sample_processing" => Self::SampleProcessing(n.into()),
            "publication" => Self::Publication(n.into()),
            "uri" => Self::Uri(n.into()),
            "external_study_uri" => Self::ExternalStudyUri(n.into()),
            "custom" => Self::Custom(n.into()),
            "derivatization_agent" => Self::DerivatizationAgent(n.into()),
            "id_confidence_measure" => Self::IdConfidenceMeasure(n.into()),
            _ => return None,
        })
    }

    fn element_key(name: &str, n: NonZeroU32, rest: Option<Segment<'_>>) -> Option<Self> {
        match name {
            "instrument" => InstrumentKey::parse(rest).map(|k| Self::Instrument(n.into(), k)),
            "software" => SoftwareKey::parse(rest).map(|k| Self::Software(n.into(), k)),
            "contact" => ContactKey::parse(rest).map(|k| Self::Contact(n.into(), k)),
            "sample" => SampleKey::parse(rest).map(|k| Self::Sample(n.into(), k)),
            "ms_run" => MsRunKey::parse(rest).map(|k| Self::MsRun(n.into(), k)),
            "assay" => AssayKey::parse(rest).map(|k| Self::Assay(n.into(), k)),
            "study_variable" => {
                StudyVariableKey::parse(rest).map(|k| Self::StudyVariable(n.into(), k))
            }
            "study_variable_group" => {
                StudyVariableGroupKey::parse(rest).map(|k| Self::StudyVariableGroup(n.into(), k))
            }
            "protocol" => ProtocolKey::parse(rest).map(|k| Self::Protocol(n.into(), k)),
            "cv" => CvKey::parse(rest).map(|k| Self::Cv(n.into(), k)),
            "database" => DatabaseKey::parse(rest).map(|k| Self::Database(n.into(), k)),
            _ => None,
        }
    }

    /// The indexed element this key belongs to, as its name and index:
    /// `assay[2]-ms_run_ref` belongs to `("assay", 2)`. `None` for keys of
    /// the document as a whole (`mzTab-version`, `title`, …).
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::MtdKey;
    ///
    /// let key = MtdKey::parse("assay[2]-ms_run_ref").ok_or("unknown")?;
    /// assert_eq!(key.element(), Some(("assay", 2)));
    /// assert_eq!(MtdKey::Title.element(), None);
    /// # Ok::<(), &str>(())
    /// ```
    #[must_use]
    pub const fn element(self) -> Option<(&'static str, u32)> {
        Some(match self {
            Self::SampleProcessing(n) => ("sample_processing", n.get()),
            Self::Instrument(n, _) => ("instrument", n.get()),
            Self::Software(n, _) => ("software", n.get()),
            Self::Publication(n) => ("publication", n.get()),
            Self::Contact(n, _) => ("contact", n.get()),
            Self::Uri(n) => ("uri", n.get()),
            Self::ExternalStudyUri(n) => ("external_study_uri", n.get()),
            Self::Sample(n, _) => ("sample", n.get()),
            Self::MsRun(n, _) => ("ms_run", n.get()),
            Self::Assay(n, _) => ("assay", n.get()),
            Self::StudyVariable(n, _) => ("study_variable", n.get()),
            Self::StudyVariableGroup(n, _) => ("study_variable_group", n.get()),
            Self::Protocol(n, _) => ("protocol", n.get()),
            Self::Custom(n) => ("custom", n.get()),
            Self::Cv(n, _) => ("cv", n.get()),
            Self::Database(n, _) => ("database", n.get()),
            Self::DerivatizationAgent(n) => ("derivatization_agent", n.get()),
            Self::IdConfidenceMeasure(n) => ("id_confidence_measure", n.get()),
            Self::Version
            | Self::Id
            | Self::Profile
            | Self::Title
            | Self::Description
            | Self::QuantificationMethod
            | Self::SmallMoleculeQuantificationUnit
            | Self::SmallMoleculeFeatureQuantificationUnit
            | Self::SmallMoleculeIdentificationReliability
            | Self::ColunitSmallMolecule
            | Self::ColunitSmallMoleculeFeature
            | Self::ColunitSmallMoleculeEvidence => return None,
        })
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
            | Self::Protocol(_, ProtocolKey::Parameters) => V::ParamList,
            Self::Publication(_) => V::Publication,
            Self::MsRun(_, sub) => ms_run_kind(sub),
            Self::Assay(_, sub) => assay_kind(sub),
            Self::StudyVariable(_, sub) => study_variable_kind(sub),
            Self::StudyVariableGroup(_, sub) => study_variable_group_kind(sub),
            Self::ColunitSmallMolecule
            | Self::ColunitSmallMoleculeFeature
            | Self::ColunitSmallMoleculeEvidence => V::ColUnit,
        }
    }
}

const fn ms_run_kind(sub: MsRunKey) -> ValueKind {
    match sub {
        MsRunKey::Location | MsRunKey::Hash | MsRunKey::UsiIdentifier => ValueKind::Text,
        MsRunKey::InstrumentRef => ValueKind::InstrumentRef,
        MsRunKey::Format | MsRunKey::IdFormat | MsRunKey::HashMethod => ValueKind::Param,
        MsRunKey::FragmentationMethod(_) | MsRunKey::ScanPolarity(_) | MsRunKey::Parameters => {
            ValueKind::ParamList
        }
    }
}

const fn assay_kind(sub: AssayKey) -> ValueKind {
    match sub {
        AssayKey::Name | AssayKey::ExternalUri => ValueKind::Text,
        AssayKey::SampleRef => ValueKind::SampleRef,
        AssayKey::MsRunRef => ValueKind::MsRunRefs,
        AssayKey::ProtocolRefs => ValueKind::ProtocolRefs,
        AssayKey::Custom(_) | AssayKey::Parameters => ValueKind::ParamList,
    }
}

const fn study_variable_kind(sub: StudyVariableKey) -> ValueKind {
    match sub {
        StudyVariableKey::Name | StudyVariableKey::Description => ValueKind::Text,
        StudyVariableKey::AssayRefs => ValueKind::AssayRefs,
        StudyVariableKey::MsRunRefs => ValueKind::MsRunRefs,
        StudyVariableKey::AverageFunction | StudyVariableKey::VariationFunction => ValueKind::Param,
        StudyVariableKey::Factors => ValueKind::ParamList,
    }
}

const fn study_variable_group_kind(sub: StudyVariableGroupKey) -> ValueKind {
    match sub {
        StudyVariableGroupKey::Description => ValueKind::Text,
        StudyVariableGroupKey::Group
        | StudyVariableGroupKey::Type
        | StudyVariableGroupKey::Unit => ValueKind::Param,
        StudyVariableGroupKey::Datatype => ValueKind::Datatype,
        StudyVariableGroupKey::StudyVariableRefs => ValueKind::StudyVariableRefs,
    }
}

impl FromStr for MtdKey {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| VocabError::new("a metadata key"))
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
            Self::Instrument(n, k) => fmt_elem(f, n, k),
            Self::Software(n, k) => fmt_elem(f, n, k),
            Self::Contact(n, k) => fmt_elem(f, n, k),
            Self::Sample(n, k) => fmt_elem(f, n, k),
            Self::MsRun(n, k) => fmt_elem(f, n, k),
            Self::Assay(n, k) => fmt_elem(f, n, k),
            Self::StudyVariable(n, k) => fmt_elem(f, n, k),
            Self::StudyVariableGroup(n, k) => fmt_elem(f, n, k),
            Self::Protocol(n, k) => fmt_elem(f, n, k),
            Self::Cv(n, k) => fmt_elem(f, n, k),
            Self::Database(n, k) => fmt_elem(f, n, k),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::MtdKey;

    #[test]
    fn from_str_round_trips() {
        assert_eq!("title".parse::<MtdKey>(), Ok(MtdKey::Title));
        assert!("bogus".parse::<MtdKey>().is_err());
    }
}
