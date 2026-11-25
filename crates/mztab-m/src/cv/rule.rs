//! CV-mapping rule types: [`Rule`], [`RuleTerm`], [`Logic`], and [`Target`].

use core::fmt;

use crate::diag::severity::Requirement;

/// How a rule combines its terms for one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic {
    /// `AND`: the value matches the rule's terms (every rule in the mapping
    /// with this logic has one term).
    And,
    /// `OR`: the value matches at least one term.
    Or,
    /// `XOR`: the value matches exactly one term.
    Xor,
}

/// One allowed term of a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleTerm {
    /// The term's accession.
    pub accession: &'static str,
    /// The term's name, as the mapping file gives it.
    pub name: &'static str,
    /// Whether the term itself is allowed.
    pub use_term: bool,
    /// Whether the term's descendants are allowed.
    pub allow_children: bool,
    /// Whether a term matching it may repeat in one scope.
    pub repeatable: bool,
}

/// The field a rule constrains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Target {
    /// `ms_run[1-n]-format`.
    MsRunFormat,
    /// `ms_run[1-n]-id_format`.
    MsRunIdFormat,
    /// `ms_run[1-n]-fragmentation_method[1-n]`.
    MsRunFragmentationMethod,
    /// `ms_run[1-n]-scan_polarity[1-n]`.
    MsRunScanPolarity,
    /// `ms_run[1-n]-hash_method`.
    MsRunHashMethod,
    /// `sample_processing[1-n]`.
    SampleProcessing,
    /// `instrument[1-n]-name`.
    InstrumentName,
    /// `instrument[1-n]-source`.
    InstrumentSource,
    /// `instrument[1-n]-analyzer[1-n]`.
    InstrumentAnalyzer,
    /// `instrument[1-n]-detector`.
    InstrumentDetector,
    /// `software[1-n]`.
    Software,
    /// `quantification_method`.
    QuantificationMethod,
    /// `sample[1-n]-species[1-n]`.
    SampleSpecies,
    /// `sample[1-n]-tissue[1-n]`.
    SampleTissue,
    /// `sample[1-n]-cell_type[1-n]`.
    SampleCellType,
    /// `sample[1-n]-disease[1-n]`.
    SampleDisease,
    /// `database[1-n]`.
    Database,
    /// `study_variable[1-n]-average_function`.
    StudyVariableAverageFunction,
    /// `study_variable[1-n]-variation_function`.
    StudyVariableVariationFunction,
    /// `study_variable_group[1-n]`.
    StudyVariableGroup,
    /// `study_variable_group[1-n]-type`.
    StudyVariableGroupType,
    /// `study_variable_group[1-n]-unit`.
    StudyVariableGroupUnit,
    /// `derivatization_agent[1-n]`.
    DerivatizationAgent,
    /// `small_molecule-quantification_unit`.
    SmallMoleculeQuantificationUnit,
    /// `small_molecule_feature-quantification_unit`.
    SmallMoleculeFeatureQuantificationUnit,
    /// `small_molecule-identification_reliability`.
    SmallMoleculeIdentificationReliability,
    /// `id_confidence_measure[1-n]`.
    IdConfidenceMeasure,
    /// SML `best_id_confidence_measure`.
    SmlBestIdConfidenceMeasure,
    /// SME `derivatized_form`.
    SmeDerivatizedForm,
    /// SME `ms_level`.
    SmeMsLevel,
    /// SME `identification_method`.
    SmeIdentificationMethod,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MsRunFormat => "ms_run[1-n]-format",
            Self::MsRunIdFormat => "ms_run[1-n]-id_format",
            Self::MsRunFragmentationMethod => "ms_run[1-n]-fragmentation_method[1-n]",
            Self::MsRunScanPolarity => "ms_run[1-n]-scan_polarity[1-n]",
            Self::MsRunHashMethod => "ms_run[1-n]-hash_method",
            Self::SampleProcessing => "sample_processing[1-n]",
            Self::InstrumentName => "instrument[1-n]-name",
            Self::InstrumentSource => "instrument[1-n]-source",
            Self::InstrumentAnalyzer => "instrument[1-n]-analyzer[1-n]",
            Self::InstrumentDetector => "instrument[1-n]-detector",
            Self::Software => "software[1-n]",
            Self::QuantificationMethod => "quantification_method",
            Self::SampleSpecies => "sample[1-n]-species[1-n]",
            Self::SampleTissue => "sample[1-n]-tissue[1-n]",
            Self::SampleCellType => "sample[1-n]-cell_type[1-n]",
            Self::SampleDisease => "sample[1-n]-disease[1-n]",
            Self::Database => "database[1-n]",
            Self::StudyVariableAverageFunction => "study_variable[1-n]-average_function",
            Self::StudyVariableVariationFunction => "study_variable[1-n]-variation_function",
            Self::StudyVariableGroup => "study_variable_group[1-n]",
            Self::StudyVariableGroupType => "study_variable_group[1-n]-type",
            Self::StudyVariableGroupUnit => "study_variable_group[1-n]-unit",
            Self::DerivatizationAgent => "derivatization_agent[1-n]",
            Self::SmallMoleculeQuantificationUnit => "small_molecule-quantification_unit",
            Self::SmallMoleculeFeatureQuantificationUnit => {
                "small_molecule_feature-quantification_unit"
            }
            Self::SmallMoleculeIdentificationReliability => {
                "small_molecule-identification_reliability"
            }
            Self::IdConfidenceMeasure => "id_confidence_measure[1-n]",
            Self::SmlBestIdConfidenceMeasure => "SML best_id_confidence_measure",
            Self::SmeDerivatizedForm => "SME derivatized_form",
            Self::SmeMsLevel => "SME ms_level",
            Self::SmeIdentificationMethod => "SME identification_method",
        })
    }
}

/// A CV-mapping rule: the terms a field's CV parameters must come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    /// The rule's id in the mapping file.
    pub id: &'static str,
    /// The field it constrains.
    pub target: Target,
    /// Its RFC 2119 level: the severity of a value that breaks it.
    pub requirement: Requirement,
    /// How its terms combine.
    pub logic: Logic,
    /// Its allowed terms.
    pub terms: &'static [RuleTerm],
}
