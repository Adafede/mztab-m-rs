//! Validation: [`syntax`], [`structure`], [`semantic()`] and [`all`],
//! configured by [`Config`].
//!
//! The layers are independent. [`syntax`] reads text and reports what the
//! reader finds. [`structure`] checks a typed model, never re-parsing it:
//! mandatory fields, profiles, references, columns and value rules, with
//! the rule set chosen by the model's `mzTab-version`. [`semantic()`] checks
//! the model's controlled vocabulary: declarations, terms (against a
//! [`TermLookup`]) and the CV-mapping rules of [`cv::rules`](crate::cv::rules).
//! [`all`] runs the three on a text.
//!
//! Every finding of [`structure`] and [`semantic()`] on a model read from
//! text is located by line, column and cell (through the model's
//! [`SourceLayout`](crate::SourceLayout)); on a model built in code it
//! carries a model path instead.
//!
//! ```
//! use mztab_m::{Code, cv::NoTerms, validate};
//!
//! let input = "MTD\tmzTab-version\t2.1.0-M\n";
//! let findings = validate::all(input, &validate::Config::default(), &NoTerms);
//! let missing = findings.iter().filter(|f| f.code == Code::MissingField).count();
//! assert!(missing > 0, "mzTab-ID, assays, study variables, cv, ... are missing");
//! ```

mod metadata;
mod patterns;
mod report;
mod semantic;
mod tables;

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::ToString as _;

use crate::cv::TermLookup;
use crate::diag::{Code, Detail, Finding, Findings, Severity};
use crate::model::MzTab;
pub(crate) use report::{Report, Site};

/// Validation settings: per-code severity overrides and disabled codes.
///
/// # Examples
///
/// ```
/// use mztab_m::{Code, Severity, validate::Config};
///
/// let strict = Config::default()
///     .with_severity(Code::ColumnOrder, Severity::Error)
///     .without(Code::IndexGap);
/// assert_eq!(strict.severity_of(Code::ColumnOrder), Some(Severity::Error));
/// assert_eq!(strict.severity_of(Code::IndexGap), None);
/// assert_eq!(strict.severity_of(Code::MissingField), Some(Severity::Error));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    overrides: BTreeMap<Code, Option<Severity>>,
}

impl Config {
    /// Reports `code` at `severity` instead of its default.
    #[must_use]
    pub fn with_severity(mut self, code: Code, severity: Severity) -> Self {
        self.overrides.insert(code, Some(severity));
        self
    }

    /// Does not report `code`.
    #[must_use]
    pub fn without(mut self, code: Code) -> Self {
        self.overrides.insert(code, None);
        self
    }

    /// The severity `code` is reported at, or `None` if it is disabled.
    #[must_use]
    pub fn severity_of(&self, code: Code) -> Option<Severity> {
        self.overrides
            .get(&code)
            .copied()
            .unwrap_or_else(|| Some(code.requirement().severity()))
    }

    /// Applies the configuration to findings.
    fn apply(&self, findings: impl IntoIterator<Item = Finding>) -> Findings {
        let mut out = Findings::new();
        for mut finding in findings {
            match self.overrides.get(&finding.code) {
                Some(None) => {}
                Some(Some(severity)) => {
                    finding.severity = *severity;
                    out.push(finding);
                }
                None => out.push(finding),
            }
        }
        out
    }
}

/// The findings of the reader on `input`: line prefixes, cell grammars,
/// metadata keys, headers and section order.
#[must_use]
pub fn syntax(input: &str, config: &Config) -> Findings {
    config.apply(crate::read::parse(input).findings.into_vec())
}

/// Checks a typed model. See the [module documentation](self).
#[must_use]
pub fn structure(mztab: &MzTab<'_>, config: &Config) -> Findings {
    let mut report = Report::new(mztab);
    let rules = report.rule_set;
    metadata::check(mztab, rules, &mut report);
    tables::check(mztab, rules, &mut report);
    config.apply(report.into_findings())
}

/// Checks a typed model's controlled vocabulary against `terms`. See the
/// [module documentation](self).
///
/// # Examples
///
/// ```
/// use mztab_m::{Code, cv::NoTerms, parse, validate};
///
/// let model = parse("MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free, ]\n").mztab;
/// let findings = validate::semantic(&model, &validate::Config::default(), &NoTerms);
/// assert!(findings.iter().any(|f| f.code == Code::CvUndeclared), "no cv[1]-label MS");
/// ```
#[must_use]
pub fn semantic(mztab: &MzTab<'_>, config: &Config, terms: &dyn TermLookup) -> Findings {
    let mut report = Report::new(mztab);
    semantic::check(mztab, terms, &mut report);
    config.apply(report.into_findings())
}

/// [`syntax`], [`structure`] and [`semantic()`] on a text. Values that failed
/// to type are reported once, by the reader, not again as
/// [`Code::InvalidValue`].
///
/// # Examples
///
/// ```
/// use mztab_m::{cv::NoTerms, validate::{self, Config}};
///
/// let findings = validate::all("MTD\tmzTab-version\t2.1\n", &Config::default(), &NoTerms);
/// let codes: Vec<_> = findings.iter().map(|f| f.code.id()).collect();
/// assert!(codes.contains(&"MZT-S006"));
/// assert!(!codes.contains(&"MZT-T028"));
/// ```
#[must_use]
pub fn all(input: &str, config: &Config, terms: &dyn TermLookup) -> Findings {
    let parsed = crate::read::parse(input);
    let mut findings = parsed.findings.into_vec();
    let mut report = Report::new(&parsed.mztab);
    let rules = report.rule_set;
    metadata::check(&parsed.mztab, rules, &mut report);
    tables::check(&parsed.mztab, rules, &mut report);
    semantic::check(&parsed.mztab, terms, &mut report);
    findings.extend(
        report
            .into_findings()
            .into_iter()
            .filter(|f| f.code != Code::InvalidValue),
    );
    config.apply(findings)
}

/// An owned label for a finding detail.
pub(crate) fn label(text: impl core::fmt::Display) -> Detail {
    Detail::Label(Cow::Owned(text.to_string()))
}
