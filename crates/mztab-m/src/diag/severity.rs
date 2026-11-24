//! Severity levels, RFC 2119 requirement levels, validation layers, and specification references.

use core::fmt;
use core::str::FromStr;

use crate::vocab::VocabError;

/// How serious a finding is. Ordered from least to most serious.
///
/// # Examples
///
/// ```
/// use mztab_m::Severity;
///
/// assert!(Severity::Error > Severity::Warning);
/// assert_eq!(Severity::Info.to_string(), "info");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth knowing; not a defect.
    Info,
    /// A SHOULD-level violation, or a likely mistake.
    Warning,
    /// A MUST-level violation: the file does not conform.
    Error,
}

impl Severity {
    /// Lowercase name (`"info"`, `"warning"`, or `"error"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl FromStr for Severity {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(VocabError::new("`error`, `warning` or `info`")),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The RFC 2119 level of the rule a code checks.
///
/// # Examples
///
/// ```
/// use mztab_m::{Requirement, Severity};
///
/// assert_eq!(Requirement::Should.severity(), Severity::Warning);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Requirement {
    /// MUST / MUST NOT / REQUIRED.
    Must,
    /// SHOULD / SHOULD NOT / RECOMMENDED.
    Should,
    /// MAY / OPTIONAL, or informational.
    May,
}

impl Requirement {
    /// The default severity of a violation: MUST gives an error, SHOULD a
    /// warning, MAY an informational note.
    #[must_use]
    pub const fn severity(self) -> Severity {
        match self {
            Self::Must => Severity::Error,
            Self::Should => Severity::Warning,
            Self::May => Severity::Info,
        }
    }

    /// Uppercase RFC 2119 keyword (`"MUST"`, `"SHOULD"`, or `"MAY"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Must => "MUST",
            Self::Should => "SHOULD",
            Self::May => "MAY",
        }
    }
}

/// The validation layer a code belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// The lexical form of lines and cells, including each cell's grammar.
    /// Code prefix `S`.
    Syntax,
    /// Fields, types, cross-references and profiles on the typed model.
    /// Code prefix `T`.
    Structure,
    /// Controlled-vocabulary rules. Code prefix `X`.
    Semantic,
}

impl Layer {
    /// Lowercase name (`"syntax"`, `"structure"`, or `"semantic"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Structure => "structure",
            Self::Semantic => "semantic",
        }
    }
}

impl FromStr for Layer {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "syntax" => Ok(Self::Syntax),
            "structure" => Ok(Self::Structure),
            "semantic" => Ok(Self::Semantic),
            _ => Err(VocabError::new("`syntax`, `structure` or `semantic`")),
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A specification document at the pinned commit
/// ([`SPEC_COMMIT`](crate::SPEC_COMMIT)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecDoc {
    /// `mzTab_format_specification_2_1-M.adoc`, the prose specification.
    Format,
    /// `mzTab_m_2_1_schema.adoc`, the field reference.
    FieldReference,
}

impl SpecDoc {
    /// The document's path in the specification repository.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Format => spec_path!(Format),
            Self::FieldReference => spec_path!(FieldReference),
        }
    }
}

/// A reference to the section of the specification that states a rule.
///
/// `Display` renders a URL to the anchor at the pinned commit.
///
/// # Examples
///
/// ```
/// use mztab_m::Code;
///
/// let spec = Code::NumberSyntax.spec_ref();
/// assert!(spec.to_string().ends_with("#encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpecRef {
    /// The document.
    pub doc: SpecDoc,
    /// The `[[anchor]]` id in the document.
    pub anchor: &'static str,
}

impl fmt::Display for SpecRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "https://github.com/HUPO-PSI/mzTab-M/blob/{}/{}#{}",
            crate::SPEC_COMMIT,
            self.doc.path(),
            self.anchor
        )
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Layer, Requirement, Severity, SpecDoc};
    use alloc::string::ToString as _;
    use alloc::vec::Vec;

    #[test]
    fn layers_render() {
        let names: Vec<alloc::string::String> = [Layer::Syntax, Layer::Structure, Layer::Semantic]
            .iter()
            .map(alloc::string::ToString::to_string)
            .collect();
        assert_eq!(names, ["syntax", "structure", "semantic"], "names");
        assert_eq!("syntax".parse::<Layer>(), Ok(Layer::Syntax));
        assert_eq!("structure".parse::<Layer>(), Ok(Layer::Structure));
        assert_eq!("semantic".parse::<Layer>(), Ok(Layer::Semantic));
        assert!("all".parse::<Layer>().is_err());
        assert_eq!(
            SpecDoc::Format.path(),
            "specification_documents/mzTab_format_specification_2_1-M.adoc",
            "path"
        );
    }

    #[test]
    fn severities() {
        assert_eq!(Requirement::Must.severity(), Severity::Error, "must");
        assert_eq!(Requirement::May.severity(), Severity::Info, "may");
        assert_eq!(Requirement::Should.severity(), Severity::Warning, "should");
        assert_eq!(Requirement::Must.as_str(), "MUST");
        assert_eq!(Requirement::Should.as_str(), "SHOULD");
        assert_eq!(Requirement::May.as_str(), "MAY");
        assert_eq!(Severity::Info.to_string(), "info", "info");
        assert_eq!(Severity::Warning.to_string(), "warning", "warning");
        assert_eq!(Severity::Error.to_string(), "error", "error");
        assert_eq!("info".parse::<Severity>(), Ok(Severity::Info));
        assert_eq!("warning".parse::<Severity>(), Ok(Severity::Warning));
        assert_eq!("error".parse::<Severity>(), Ok(Severity::Error));
        assert!("fatal".parse::<Severity>().is_err());
        assert!(Severity::Warning > Severity::Info, "order");
    }
}
