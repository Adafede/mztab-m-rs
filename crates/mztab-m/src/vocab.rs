//! Closed vocabularies: [`Version`], [`Profile`], [`Section`],
//! [`AmbiguityCode`] and [`PublicationItem`].
//!
//! ```
//! use mztab_m::{Profile, RuleSet, Version};
//!
//! let version = Version::parse("2.1.0-M")?;
//! assert_eq!(version.rule_set(), RuleSet::V2_1);
//!
//! assert_eq!(Profile::parse("M+F+E"), Some(Profile::MFE));
//! assert_eq!(Profile::from_tables(true, true, true), Ok(Profile::MSFE));
//! # Ok::<(), mztab_m::VersionError>(())
//! ```

pub(crate) mod field;
pub(crate) mod profile;
pub(crate) mod section;
pub(crate) mod version;

use core::fmt;

/// Why a string could not be parsed as a closed vocabulary value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VocabError {
    expected: &'static str,
}

impl VocabError {
    pub(crate) const fn new(expected: &'static str) -> Self {
        Self { expected }
    }
}

impl fmt::Display for VocabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected {}", self.expected)
    }
}

impl core::error::Error for VocabError {}
