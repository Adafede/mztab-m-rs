//! Controlled vocabularies: the [`TermLookup`] trait that semantic
//! validation asks about terms, and the CV-mapping [`rules()`] of the
//! specification.
//!
//! The core never reads a vocabulary: a [`TermLookup`] answers for the
//! vocabularies it holds (the `mztab-m-obo` crate builds one from OBO
//! files), and [`NoTerms`] holds none. Checks that need a term the lookup
//! does not cover are skipped, never guessed.
//!
//! ```
//! use mztab_m::cv::{self, NoTerms, Target, TermLookup};
//!
//! assert!(!NoTerms.covers("MS"));
//! let polarity = cv::rules()
//!     .iter()
//!     .find(|r| r.target == Target::MsRunScanPolarity)
//!     .ok_or("no rule")?;
//! assert_eq!(polarity.id, "msrun_scan_polarity_must");
//! assert_eq!(polarity.terms.len(), 2);
//! # Ok::<(), &str>(())
//! ```

#[rustfmt::skip]
mod rules;

use core::fmt;

use crate::diag::Requirement;

/// What a vocabulary says about one term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term<'t> {
    /// The term's name.
    pub name: &'t str,
    /// Whether the term is marked obsolete.
    pub obsolete: bool,
    /// The XML Schema type a value of this term must have, by its local
    /// name (`int`, `double`, `string`, ...), if the vocabulary gives one
    /// (PSI-MS: `xref: value-type:xsd\:int`).
    pub value_type: Option<&'t str>,
}

/// Answers questions about the terms of the vocabularies it holds.
///
/// Every question is about an accession such as `MS:1000511`. A lookup
/// *covers* a prefix (`MS`) when it holds that vocabulary; for an accession
/// it does not cover, its answers mean "unknown", and the validator skips
/// the checks that need them.
pub trait TermLookup {
    /// Whether the lookup holds the vocabulary with this accession prefix.
    fn covers(&self, prefix: &str) -> bool;

    /// The term with this accession, if the lookup holds it.
    fn term(&self, accession: &str) -> Option<Term<'_>>;

    /// Whether `accession` is a strict descendant of `ancestor`.
    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool;

    /// The version of the vocabulary with this prefix, if known.
    fn version(&self, _prefix: &str) -> Option<&str> {
        None
    }
}

/// A lookup that holds no vocabulary: only the checks that need no terms
/// run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTerms;

impl TermLookup for NoTerms {
    fn covers(&self, _prefix: &str) -> bool {
        false
    }

    fn term(&self, _accession: &str) -> Option<Term<'_>> {
        None
    }

    fn is_descendant(&self, _accession: &str, _ancestor: &str) -> bool {
        false
    }
}

impl<T: TermLookup + ?Sized> TermLookup for &T {
    fn covers(&self, prefix: &str) -> bool {
        (**self).covers(prefix)
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        (**self).term(accession)
    }

    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        (**self).is_descendant(accession, ancestor)
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        (**self).version(prefix)
    }
}

/// Several lookups, asked in order: the first that covers an accession's
/// prefix answers for it.
///
/// ```
/// use mztab_m::cv::{NoTerms, TermLookup};
///
/// let lookups: [&dyn TermLookup; 2] = [&NoTerms, &NoTerms];
/// assert!(!lookups.covers("UO"));
/// let as_dyn: &dyn TermLookup = &lookups;
/// assert_eq!(as_dyn.version("UO"), None);
/// ```
impl<T: TermLookup> TermLookup for [T] {
    fn covers(&self, prefix: &str) -> bool {
        self.iter().any(|t| t.covers(prefix))
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        self.iter()
            .find(|t| t.covers(prefix(accession)))
            .and_then(|t| t.term(accession))
    }

    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        self.iter()
            .find(|t| t.covers(prefix(accession)))
            .is_some_and(|t| t.is_descendant(accession, ancestor))
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        self.iter()
            .find(|t| t.covers(prefix))
            .and_then(|t| t.version(prefix))
    }
}

/// Several lookups in an array, asked as a slice; `&[&ms, &uo]` passes as
/// a `&dyn TermLookup`.
impl<T: TermLookup, const N: usize> TermLookup for [T; N] {
    fn covers(&self, prefix: &str) -> bool {
        self.as_slice().covers(prefix)
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        self.as_slice().term(accession)
    }

    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        self.as_slice().is_descendant(accession, ancestor)
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        self.as_slice().version(prefix)
    }
}

/// The prefix of an accession: the text before its first `:` (the whole
/// text if it has none).
#[must_use]
pub fn prefix(accession: &str) -> &str {
    accession
        .split_once(':')
        .map_or(accession, |(prefix, _)| prefix)
}

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

/// The CV-mapping rules of the specification
/// (`schema/mzTab_2_1-M_mapping.xml` at [`SPEC_COMMIT`](crate::SPEC_COMMIT)),
/// with the corrections recorded in DECISIONS D-068.
#[must_use]
pub const fn rules() -> &'static [Rule] {
    rules::RULES
}
