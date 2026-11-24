//! [`Finding`], [`Detail`], and [`Findings`] collection.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

use super::code::Code;
use super::location::Location;
use super::severity::Severity;
use crate::meta::MtdKey;
use crate::param::ParamError;
use crate::value::{NumberError, TextError};
use crate::vocab::{Profile, ProhibitedProfile, VersionError};

/// Structured detail for a finding. Rendered only on demand.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Detail {
    /// No detail beyond the code.
    None,
    /// A parameter grammar error.
    Param(ParamError),
    /// A number grammar error.
    Number(NumberError),
    /// A text cell error.
    Text(TextError),
    /// A version grammar error.
    Version(VersionError),
    /// What the cell should have been.
    Expected(&'static str),
    /// How many cells the header has and how many the row has.
    CellCount {
        /// Cells in the header, prefix included.
        expected: u32,
        /// Cells in the row, prefix included.
        found: u32,
    },
    /// A line prefix that matches a known one except for letter case.
    PrefixCase(&'static str),
    /// The metadata key concerned (for example, a missing field).
    Key(MtdKey),
    /// The element, column or row id concerned, as written.
    Label(Cow<'static, str>),
    /// The profile the tables form.
    Profile(Profile),
    /// The prohibited combination the tables form.
    Prohibited(ProhibitedProfile),
    /// A CV term: its accession, the name its vocabulary gives it, and the
    /// CV-mapping rule it was checked against.
    Term {
        /// The accession.
        accession: Cow<'static, str>,
        /// The vocabulary's name for the term, when it differs.
        name: Option<Cow<'static, str>>,
        /// The CV-mapping rule's id.
        rule: Option<&'static str>,
    },
}

impl fmt::Display for Detail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Param(e) => e.fmt(f),
            Self::Number(e) => e.fmt(f),
            Self::Text(e) => e.fmt(f),
            Self::Version(e) => e.fmt(f),
            Self::Expected(what) => write!(f, "expected {what}"),
            Self::CellCount { expected, found } => {
                write!(f, "expected {expected} cells, found {found}")
            }
            Self::PrefixCase(prefix) => {
                write!(f, "prefixes are case-sensitive; did you mean `{prefix}`?")
            }
            Self::Key(key) => write!(f, "`{key}`"),
            Self::Label(label) => write!(f, "`{label}`"),
            Self::Profile(profile) => write!(f, "profile `{profile}`"),
            Self::Prohibited(tables) => write!(f, "tables `{tables}`"),
            Self::Term {
                accession,
                name,
                rule,
            } => {
                write!(f, "`{accession}`")?;
                if let Some(name) = name {
                    write!(f, " is `{name}`")?;
                }
                rule.map_or(Ok(()), |rule| write!(f, " (rule `{rule}`)"))
            }
        }
    }
}

/// One problem (or note) about a document.
///
/// `Display` renders `severity[id] location: summary: detail`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Finding {
    /// What rule is concerned.
    pub code: Code,
    /// How serious it is; defaults to the code's RFC 2119 level.
    pub severity: Severity,
    /// Where it applies.
    pub location: Location,
    /// Structured detail.
    pub detail: Detail,
}

impl Finding {
    /// A finding at the code's default severity.
    #[must_use]
    pub const fn new(code: Code, location: Location, detail: Detail) -> Self {
        Self {
            code,
            severity: code.requirement().severity(),
            location,
            detail,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = if matches!(self.detail, Detail::None) {
            ""
        } else {
            ": "
        };
        write!(
            f,
            "{}[{}] {}: {}{separator}{}",
            self.severity,
            self.code.id(),
            self.location,
            self.code.summary(),
            self.detail
        )
    }
}

/// Every finding about a document, in the order they were produced.
///
/// # Examples
///
/// ```
/// use mztab_m::{Code, Detail, Finding, Findings, Location, Severity};
///
/// let mut findings = Findings::new();
/// assert_eq!(findings.max_severity(), None);
/// findings.push(Finding::new(Code::LinePrefix, Location::text(1, 1), Detail::None));
/// assert!(findings.has_errors());
/// assert_eq!(findings.len(), 1);
/// assert_eq!(findings.iter().count(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Findings(Vec<Finding>);

impl Findings {
    /// No findings.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a finding.
    pub fn push(&mut self, finding: Finding) {
        self.0.push(finding);
    }

    /// The number of findings.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The findings in order.
    pub fn iter(&self) -> core::slice::Iter<'_, Finding> {
        self.0.iter()
    }

    /// The most serious severity, or `None` if there are no findings.
    #[must_use]
    pub fn max_severity(&self) -> Option<Severity> {
        self.0.iter().map(|f| f.severity).max()
    }

    /// Whether any finding is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.max_severity() == Some(Severity::Error)
    }

    /// The findings as a `Vec`.
    #[must_use]
    pub fn into_vec(self) -> Vec<Finding> {
        self.0
    }
}

impl<'a> IntoIterator for &'a Findings {
    type Item = &'a Finding;
    type IntoIter = core::slice::Iter<'a, Finding>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Extend<Finding> for Findings {
    fn extend<I: IntoIterator<Item = Finding>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Detail, Finding, Findings};
    use crate::diag::code::Code;
    use crate::diag::location::Location;
    use crate::diag::severity::Severity;
    use crate::param::{ParamError, ParamErrorKind};
    use crate::value::{NumberError, TextError};
    use crate::vocab::VersionError;
    use alloc::string::ToString as _;

    #[test]
    fn findings_render_with_detail() {
        let at = Location::text(1, 2);
        let render = |code, detail| Finding::new(code, at.clone(), detail).to_string();
        assert_eq!(
            render(
                Code::ParamSyntax,
                Detail::Param(ParamError {
                    kind: ParamErrorKind::MissingName,
                    offset: 3
                })
            ),
            "error[MZT-S002] line 1, column 2: invalid parameter: the name field is empty (at byte 3)",
            "param"
        );
        assert_eq!(
            render(
                Code::NumberSyntax,
                Detail::Number(NumberError { offset: 0 })
            ),
            "error[MZT-S003] line 1, column 2: invalid number: not a number (unexpected input at byte 0)",
            "number"
        );
        assert_eq!(
            render(Code::TextCell, Detail::Text(TextError::Empty)),
            "error[MZT-S005] line 1, column 2: empty or malformed cell: text is empty",
            "text"
        );
        assert_eq!(
            render(
                Code::VersionSyntax,
                Detail::Version(VersionError { offset: 0 })
            ),
            "error[MZT-S006] line 1, column 2: invalid mzTab-version: not an mzTab-M version `major.minor.patch-M` (unexpected input at byte 0)",
            "version"
        );
        assert_eq!(Detail::None.to_string(), "", "none");
        assert_eq!(
            render(Code::LinePrefix, Detail::None),
            "error[MZT-S001] line 1, column 2: unknown line prefix",
            "no detail"
        );
    }

    #[test]
    fn findings_collect() {
        let mut findings = Findings::new();
        assert_eq!(findings, Findings::default(), "new is default");
        assert!(findings.is_empty(), "empty");
        assert_eq!(findings.len(), 0, "len 0");
        assert!(!findings.has_errors(), "no errors");
        let mut warn = Finding::new(Code::NumberForm, Location::Document, Detail::None);
        warn.severity = Severity::Warning;
        findings.push(warn.clone());
        assert_eq!(findings.len(), 1, "len");
        assert_eq!(findings.iter().next(), Some(&warn), "iter");
        assert_eq!(findings.max_severity(), Some(Severity::Warning), "max");
        assert!(!findings.has_errors(), "warnings only");
        assert!(!findings.is_empty(), "not empty");
        findings.extend([warn.clone()]);
        assert_eq!((&findings).into_iter().count(), 2, "into_iter");
        assert_eq!(findings.len(), 2, "len 2");
        findings.push(Finding::new(
            Code::LinePrefix,
            Location::Document,
            Detail::None,
        ));
        assert!(findings.has_errors(), "error present");
        let all = findings.into_vec();
        assert_eq!(all.get(..2), Some(&[warn.clone(), warn][..]), "into_vec");
        assert_eq!(all.len(), 3, "into_vec len");
    }
}
