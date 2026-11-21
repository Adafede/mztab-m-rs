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
mod semantic;
mod tables;

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use crate::cv::TermLookup;
use crate::diag::{Code, Detail, Finding, Findings, Location, Path, Severity};
use crate::meta::MtdKey;
use crate::model::{LineContent, MzTab};
use crate::vocab::{RuleSet, Section};

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
    config.apply(report.findings)
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
    config.apply(report.findings)
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
            .findings
            .into_iter()
            .filter(|f| f.code != Code::InvalidValue),
    );
    config.apply(findings)
}

/// Where in a model a finding applies.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Site {
    /// The document as a whole.
    Document,
    /// A metadata line, by key (its first occurrence).
    Key(MtdKey),
    /// The value cell of a metadata line.
    Value(MtdKey),
    /// The `n`-th value of a repeating key (`colunit-*`).
    Nth(MtdKey, usize),
    /// An element, located at its first line: `("assay", 2)`.
    Element(&'static str, u32),
    /// A table's header line.
    Header(Section),
    /// A header column (0-based, after the prefix).
    Column(Section, usize),
    /// A cell: row and column (0-based, after the prefix).
    Cell(Section, usize, usize),
}

/// Collects findings and locates them.
pub(crate) struct Report<'m, 'a> {
    mztab: &'m MzTab<'a>,
    pub(crate) rule_set: RuleSet,
    findings: Vec<Finding>,
    /// Lazily built: source line numbers of keys, headers and rows.
    lines: Option<Lines>,
}

/// Source line numbers (1-based) of the model's items.
#[derive(Default)]
struct Lines {
    keys: BTreeMap<MtdKey, Vec<u32>>,
    elements: BTreeMap<(&'static str, u32), u32>,
    headers: [Option<u32>; 3],
    rows: [Vec<u32>; 3],
    first: Option<u32>,
}

/// The model-path name of a table (metadata shares `sml`; no table site
/// ever names it).
const fn table_name(section: Section) -> &'static str {
    match section {
        Section::Mtd | Section::Sml => "sml",
        Section::Smf => "smf",
        Section::Sme => "sme",
    }
}

impl<'m, 'a> Report<'m, 'a> {
    fn new(mztab: &'m MzTab<'a>) -> Self {
        let rule_set = mztab
            .metadata
            .version
            .as_ref()
            .and_then(|cell| cell.value())
            .map_or(RuleSet::V2_1, |v| v.rule_set());
        Self {
            mztab,
            rule_set,
            findings: Vec::new(),
            lines: None,
        }
    }

    /// Records a finding at its code's default severity.
    pub(crate) fn add(&mut self, code: Code, site: Site, detail: Detail) {
        let location = self.locate(site);
        self.findings.push(Finding::new(code, location, detail));
    }

    /// Records a finding at a given severity.
    pub(crate) fn add_at(&mut self, code: Code, severity: Severity, site: Site, detail: Detail) {
        let location = self.locate(site);
        let mut finding = Finding::new(code, location, detail);
        finding.severity = severity;
        self.findings.push(finding);
    }

    fn lines(&mut self) -> &Lines {
        let layout = &self.mztab.layout;
        self.lines.get_or_insert_with(|| {
            let mut lines = Lines::default();
            for (i, line) in layout.lines.iter().enumerate() {
                let number = u32::try_from(i + 1).unwrap_or(u32::MAX);
                match &line.content {
                    LineContent::Metadata(key) => {
                        lines.keys.entry(*key).or_default().push(number);
                        if let Some(element) = key.element() {
                            lines.elements.entry(element).or_insert(number);
                        }
                        lines.first.get_or_insert(number);
                    }
                    LineContent::UnknownMetadata => {
                        lines.first.get_or_insert(number);
                    }
                    LineContent::Header(s) => {
                        s.pick_mut(&mut lines.headers).get_or_insert(number);
                    }
                    LineContent::Row(s) => s.pick_mut(&mut lines.rows).push(number),
                    LineContent::Extra(_) => {}
                }
            }
            lines
        })
    }

    /// The text of source line `number` (as read, or as the writer renders
    /// `item`), for computing columns.
    fn line_text(&self, number: u32, item: Item) -> Option<Cow<'_, str>> {
        let index = usize::try_from(number).ok()?.checked_sub(1)?;
        if let Some(verbatim) = &self.mztab.layout.lines.get(index)?.verbatim {
            return Some(Cow::Borrowed(verbatim.text.as_str()));
        }
        match item {
            Item::Key(key) => Some(Cow::Owned(alloc::format!("MTD\t{key}\t"))),
            Item::Header(s) => crate::write::header_text(self.mztab, s).map(Cow::Owned),
            Item::Row(s, r) => crate::write::row_text(self.mztab, s, r).map(Cow::Owned),
        }
    }

    /// A text location: line `number`, at the start of cell `cell` (1-based,
    /// the prefix is cell 1).
    fn text_at(&self, number: u32, cell: u32, item: Item) -> Location {
        let before = usize::try_from(cell).unwrap_or(1).saturating_sub(1);
        let column = self.line_text(number, item).map_or(1, |text| {
            text.split('\t')
                .take(before)
                .map(|part| part.chars().count() + 1)
                .sum::<usize>()
                + 1
        });
        Location::text(number, u32::try_from(column).unwrap_or(u32::MAX)).with_cell(cell)
    }

    fn locate(&mut self, site: Site) -> Location {
        // No source lines for items (none at all, or only comments, as a
        // JSON-read model has): locate by model path.
        let lines = &self.mztab.layout.lines;
        if lines
            .iter()
            .all(|l| matches!(l.content, LineContent::Extra(_)))
        {
            return model_path(site);
        }
        let lines = self.lines();
        let found = match site {
            Site::Document => return Location::Document,
            Site::Key(key) => first(lines.keys.get(&key)).map(|n| (n, 2, Item::Key(key))),
            Site::Value(key) => first(lines.keys.get(&key)).map(|n| (n, 3, Item::Key(key))),
            Site::Nth(key, nth) => {
                let n = lines.keys.get(&key).and_then(|l| l.get(nth)).copied();
                n.map(|n| (n, 3, Item::Key(key)))
            }
            Site::Element(name, n) => {
                let line = lines.elements.get(&(name, n)).copied();
                line.map(|n| (n, 2, Item::Key(MtdKey::Title)))
            }
            Site::Header(s) => (*s.pick(&lines.headers)).map(|n| (n, 1, Item::Header(s))),
            Site::Column(s, c) => (*s.pick(&lines.headers)).map(|n| (n, cell(c), Item::Header(s))),
            Site::Cell(s, r, c) => s
                .pick(&lines.rows)
                .get(r)
                .copied()
                .map(|n| (n, cell(c), Item::Row(s, r))),
        };
        // An item added after reading has no source line: fall back to the
        // first metadata line, or the document.
        let fallback = lines.first.map(|n| (n, 1, Item::Key(MtdKey::Title)));
        match found.or(fallback) {
            Some((number, cell, item)) => self.text_at(number, cell, item),
            None => Location::Document,
        }
    }
}

/// What a located line holds, to render it when it has no source text.
#[derive(Clone, Copy, Debug)]
enum Item {
    Key(MtdKey),
    Header(Section),
    Row(Section, usize),
}

fn first(lines: Option<&Vec<u32>>) -> Option<u32> {
    lines.and_then(|l| l.first()).copied()
}

/// The 1-based cell number of a 0-based column after the prefix.
fn cell(column: usize) -> u32 {
    u32::try_from(column + 2).unwrap_or(u32::MAX)
}

/// The model path of a site, for models built in code.
fn model_path(site: Site) -> Location {
    let path = Path::new();
    let index = |n: usize| u32::try_from(n + 1).unwrap_or(u32::MAX);
    Location::Model(match site {
        Site::Document => return Location::Document,
        Site::Key(key) | Site::Value(key) => path.key("metadata").key(key.to_string()),
        Site::Nth(key, n) => path.key("metadata").key(key.to_string()).index(index(n)),
        Site::Element(name, n) => path.key("metadata").key(name).index(n),
        Site::Header(s) => path.key(table_name(s)).key("header"),
        Site::Column(s, c) => path.key(table_name(s)).key("header").index(index(c)),
        Site::Cell(s, r, c) => path.key(table_name(s)).index(index(r)).index(index(c)),
    })
}

/// An owned label for a finding detail.
pub(crate) fn label(text: impl core::fmt::Display) -> Detail {
    Detail::Label(Cow::Owned(text.to_string()))
}
