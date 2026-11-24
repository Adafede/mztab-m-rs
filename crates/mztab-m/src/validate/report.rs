//! Validation finding collection and lazy source-location resolution.

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use crate::diag::code::Code;
use crate::diag::finding::{Detail, Finding};
use crate::diag::location::{Location, Path};
use crate::diag::severity::Severity;
use crate::meta::key::MtdKey;
use crate::model::document::MzTab;
use crate::model::layout::LineContent;
use crate::vocab::section::TableSection;
use crate::vocab::version::RuleSet;

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
    Header(TableSection),
    /// A header column (0-based, after the prefix).
    Column(TableSection, usize),
    /// A cell: row and column (0-based, after the prefix).
    Cell(TableSection, usize, usize),
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

impl<'m, 'a> Report<'m, 'a> {
    pub(super) fn new(mztab: &'m MzTab<'a>) -> Self {
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

    pub(super) fn into_findings(self) -> Vec<Finding> {
        self.findings
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
                        if let Some(ts) = TableSection::from_section(*s) {
                            ts.pick_mut(&mut lines.headers).get_or_insert(number);
                        }
                    }
                    LineContent::Row(s) => {
                        if let Some(ts) = TableSection::from_section(*s) {
                            ts.pick_mut(&mut lines.rows).push(number);
                        }
                    }
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
            Item::Header(s) => crate::write::pending::header_text(self.mztab, s).map(Cow::Owned),
            Item::Row(s, r) => crate::write::pending::row_text(self.mztab, s, r).map(Cow::Owned),
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
            Site::Header(ts) => (*ts.pick(&lines.headers)).map(|n| (n, 1, Item::Header(ts))),
            Site::Column(ts, c) => {
                (*ts.pick(&lines.headers)).map(|n| (n, cell(c), Item::Header(ts)))
            }
            Site::Cell(ts, r, c) => ts
                .pick(&lines.rows)
                .get(r)
                .copied()
                .map(|n| (n, cell(c), Item::Row(ts, r))),
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
    Header(TableSection),
    Row(TableSection, usize),
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
        Site::Header(ts) => path.key(ts.name()).key("header"),
        Site::Column(ts, c) => path.key(ts.name()).key("header").index(index(c)),
        Site::Cell(ts, r, c) => path.key(ts.name()).index(index(r)).index(index(c)),
    })
}
