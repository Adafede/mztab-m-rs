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

mod rule;
#[rustfmt::skip]
mod rules;

pub use rule::{Logic, Rule, RuleTerm, Target};

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
    /// Other names that mean exactly this term (OBO `EXACT` synonyms), which
    /// a parameter may use instead of the name.
    pub synonyms: &'t [&'t str],
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

/// The CV-mapping rules of the specification
/// (`schema/mzTab_2_1-M_mapping.xml` at [`SPEC_COMMIT`](crate::SPEC_COMMIT)),
/// with the corrections recorded in DECISIONS D-068.
#[must_use]
pub const fn rules() -> &'static [Rule] {
    rules::RULES
}
