//! JSCH JSON → model, through TSV text.
//!
//! Each JSON value is carried into the TSV line that holds it, and the
//! TSV reader types the result; its findings are then relocated to JSON
//! pointers. A string where a structured value is expected is taken as
//! that value's TSV text, so whatever the writer could not express in JSON
//! form reads back unchanged.

mod emit;
mod meta;
mod table;

use alloc::borrow::Cow;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::lexer;
use super::schema::{SNAKE_DIALECT, TABLES};
use super::value::{Node, Value};
use crate::diag::code::Code;
use crate::diag::finding::{Detail, Finding, Findings};
use crate::diag::location::{Location, Path};
use crate::owned::IntoOwned as _;
use crate::read::event::Parsed;

/// Reads JSCH JSON. See the [module documentation](super).
pub(super) fn parse(input: &str) -> Parsed<'static> {
    let mut findings = Vec::new();
    let text = input.strip_prefix('\u{feff}').unwrap_or(input);
    let root = match lexer::parse(text) {
        Ok(root) => root,
        Err(e) => {
            let (line, column) = line_column(text, e.at);
            findings.push(Finding::new(
                Code::JsonSyntax,
                Location::text(line, column),
                Detail::Expected(e.what),
            ));
            return Parsed {
                mztab: crate::model::document::MzTab::default(),
                findings: collect(findings),
            };
        }
    };
    let mut tsv = Tsv::default();
    transcode(&root, &mut tsv);
    let layout = comments(&root, &mut tsv);
    let parsed = crate::read::builder::parse(&tsv.text);
    let mut mztab = parsed.mztab.into_owned();
    mztab.layout = layout;
    findings.append(&mut tsv.findings);
    for mut finding in parsed.findings.into_vec() {
        finding.location = tsv.relocate(&finding.location);
        findings.push(finding);
    }
    Parsed {
        mztab,
        findings: collect(findings),
    }
}

/// The document's `comment` items as a layout of `COM` lines only: written
/// as TSV they follow the metadata (D-075).
fn comments(root: &Node<'_>, tsv: &mut Tsv) -> crate::model::layout::SourceLayout<'static> {
    use crate::model::layout::{ExtraLine, LineContent, LineEnding, SourceLayout, SourceLine};
    let mut layout = SourceLayout::default();
    let Some(node) = root.get("comment").filter(|n| !n.is_null()) else {
        return layout;
    };
    let at = Path::new().key("comment");
    for (_, item, here) in items(node, &at, tsv) {
        let msg = match item.get("msg").map(|n| &n.value) {
            Some(Value::String(s)) => s.to_string(),
            Some(Value::Null) | None => String::new(),
            Some(_) => {
                tsv.report(Code::JsonShape, &here, Detail::Expected("a comment"));
                continue;
            }
        };
        if msg.contains(['\n', '\r']) {
            tsv.report(Code::JsonText, &here, Detail::None);
            continue;
        }
        layout.lines.push(SourceLine {
            content: LineContent::Extra(ExtraLine::Comment(crate::value::text::Raw::owned(
                format!("COM\t{msg}"),
            ))),
            ending: LineEnding::Lf,
            verbatim: None,
        });
    }
    layout
}

fn collect(findings: Vec<Finding>) -> Findings {
    let mut out = Findings::new();
    out.extend(findings);
    out
}

/// The 1-based line and column (in characters) of byte `at`.
fn line_column(text: &str, at: usize) -> (u32, u32) {
    let before = text.get(..at).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |n| n + 1);
    let column = before.get(start..).map_or(0, |s| s.chars().count()) + 1;
    let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    (n(line), n(column))
}

/// The TSV text being built, the JSON pointer of each of its cells, and
/// the findings of the transcoding.
#[derive(Default)]
pub(super) struct Tsv {
    text: String,
    /// Per line: the pointer of each cell (cell 1, the prefix, holds the
    /// line's own pointer).
    cells: Vec<Vec<Path>>,
    findings: Vec<Finding>,
    /// Element name → ids, in input order (for the abundance arrays).
    pub(super) ids: BTreeMap<&'static str, Vec<u32>>,
}

impl Tsv {
    pub(super) fn report(&mut self, code: Code, at: &Path, detail: Detail) {
        self.findings
            .push(Finding::new(code, Location::Json(at.clone()), detail));
    }

    /// Appends a line; refuses cells that TSV cannot carry.
    pub(super) fn line(&mut self, cells: &[(String, Path)]) {
        // A tab or line feed would split the line; a carriage return only
        // where it would end it (the TSV reader reads one mid-line).
        let last = cells.len().saturating_sub(1);
        if let Some((_, at)) = cells
            .iter()
            .enumerate()
            .find(|(n, (text, _))| {
                text.contains(['\t', '\n']) || (*n == last && text.ends_with('\r'))
            })
            .map(|(_, cell)| cell)
        {
            self.report(Code::JsonText, at, Detail::None);
            return;
        }
        for (n, (text, _)) in cells.iter().enumerate() {
            if n > 0 {
                self.text.push('\t');
            }
            self.text.push_str(text);
        }
        self.text.push('\n');
        self.cells
            .push(cells.iter().map(|(_, at)| at.clone()).collect());
    }

    /// The JSON pointer of a TSV location (the TSV reader locates every
    /// finding by line; anything else is kept).
    fn relocate(&self, location: &Location) -> Location {
        let Location::Text { line, cell, .. } = *location else {
            return location.clone();
        };
        let cells = usize::try_from(line)
            .ok()
            .and_then(|l| l.checked_sub(1))
            .and_then(|l| self.cells.get(l));
        // Cell 1 (the prefix) holds the line's own pointer.
        let n = cell.map_or(0, |c| {
            usize::try_from(c).unwrap_or(usize::MAX).saturating_sub(1)
        });
        let at = cells.and_then(|cells| cells.get(n));
        at.map_or(Location::Document, |at| Location::Json(at.clone()))
    }
}

fn transcode(root: &Node<'_>, tsv: &mut Tsv) {
    let top = Path::new();
    let Value::Object(members) = &root.value else {
        tsv.report(Code::JsonShape, &top, Detail::Expected("an object"));
        return;
    };
    if members
        .iter()
        .any(|(k, _)| SNAKE_DIALECT.contains(&k.as_ref()))
    {
        tsv.report(
            Code::JsonDialect,
            &top,
            Detail::Label(Cow::Borrowed("snake_case keys (MetaboScape)")),
        );
        return;
    }
    let known: Vec<&str> = ["metadata", "comment"]
        .into_iter()
        .chain(TABLES.iter().map(|t| t.json))
        .collect();
    unknown_keys(members, &known, &top, tsv);
    if let Some(metadata) = root.get("metadata").filter(|n| !n.is_null()) {
        meta::metadata_lines(metadata, &Path::new().key("metadata"), tsv);
    }
    for table in &TABLES {
        if let Some(rows) = root.get(table.json).filter(|n| !n.is_null()) {
            table::table_lines(table, rows, &Path::new().key(table.json), tsv);
        }
    }
}

/// Reports members not in `known`, and repeated members (the first one is read).
pub(super) fn unknown_keys(
    members: &[(Cow<'_, str>, Node<'_>)],
    known: &[&str],
    at: &Path,
    tsv: &mut Tsv,
) {
    let mut seen = BTreeSet::new();
    for (key, _) in members {
        let here = at.clone().key(key.to_string());
        if !seen.insert(key.as_ref()) {
            tsv.report(
                Code::JsonShape,
                &here,
                Detail::Expected("a member only once"),
            );
        } else if !known.contains(&key.as_ref()) {
            tsv.report(Code::JsonUnknownKey, &here, Detail::None);
        }
    }
}

pub(super) fn mtd(tsv: &mut Tsv, key: String, value: String, at: &Path) {
    tsv.line(&[
        ("MTD".to_string(), at.clone()),
        (key, at.clone()),
        (value, at.clone()),
    ]);
}

/// The items of an array, with their positions and pointers.
pub(super) fn items<'n, 'a>(
    node: &'n Node<'a>,
    at: &Path,
    tsv: &mut Tsv,
) -> Vec<(u32, &'n Node<'a>, Path)> {
    let Value::Array(items) = &node.value else {
        tsv.report(Code::JsonShape, at, Detail::Expected("an array"));
        return Vec::new();
    };
    items
        .iter()
        .enumerate()
        .map(|(n, item)| {
            let index = u32::try_from(n).unwrap_or(u32::MAX);
            (index, item, at.clone().index(index))
        })
        .collect()
}

pub(super) fn shape_error<T>(tsv: &mut Tsv, at: &Path, expected: &'static str) -> Option<T> {
    tsv.report(Code::JsonShape, at, Detail::Expected(expected));
    None
}

#[cfg(test)]
mod tests {
    use super::Tsv;
    use crate::diag::location::{Location, Path};

    #[test]
    fn relocation_keeps_what_is_not_a_text_location() {
        let tsv = Tsv::default();
        let model = Location::Model(Path::new().key("x"));
        assert_eq!(tsv.relocate(&model), model, "kept");
        assert_eq!(
            tsv.relocate(&Location::text(9, 1)),
            Location::Document,
            "no such line"
        );
    }
}
