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
use super::schema::{SNAKE_DIALECT, TABLES, Table};
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
    let layout = if transcode(&root, &mut tsv) {
        comments(&root, &mut tsv)
    } else {
        crate::model::layout::SourceLayout::default()
    };
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

/// Reads JSCH JSON bytes, reporting invalid UTF-8 before parsing lossy UTF-8.
pub(super) fn parse_bytes(input: &[u8]) -> Parsed<'static> {
    match core::str::from_utf8(input) {
        Ok(text) => parse(text),
        Err(error) => {
            let lossy = String::from_utf8_lossy(input);
            let mut parsed = parse(&lossy);
            let valid = input.get(..error.valid_up_to()).unwrap_or_default();
            let text = core::str::from_utf8(valid).unwrap_or_default();
            let (line, column) = line_column(text, text.len());
            let mut findings = Findings::new();
            findings.push(Finding::new(
                Code::Utf8,
                Location::text(line, column),
                Detail::None,
            ));
            findings.extend(parsed.findings.into_vec());
            parsed.findings = findings;
            parsed
        }
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
        let Value::Object(members) = &item.value else {
            tsv.report(Code::JsonShape, &here, Detail::Expected("a comment"));
            continue;
        };
        unknown_keys(members, &["prefix", "msg", "line_number"], &here, tsv);
        let valid_prefix = match item.get("prefix").filter(|n| !n.is_null()) {
            Some(prefix) if !matches!(&prefix.value, Value::String(s) if s == "COM") => {
                tsv.report(
                    Code::JsonShape,
                    &here.clone().key("prefix"),
                    Detail::Expected("`COM`"),
                );
                false
            }
            _ => true,
        };
        let valid_line = match item.get("line_number").filter(|n| !n.is_null()) {
            Some(line) if !matches!(&line.value, Value::Number(n) if emit::parse_positive_int(n).is_some()) =>
            {
                tsv.report(
                    Code::JsonShape,
                    &here.clone().key("line_number"),
                    Detail::Expected("a positive integer"),
                );
                false
            }
            _ => true,
        };
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
        if !valid_prefix || !valid_line {
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
    let line = before.matches('\n').count().saturating_add(1);
    let start = before.rfind('\n').map_or(0, |n| n + 1);
    let column = before
        .get(start..)
        .map_or(0, |s| s.chars().count())
        .saturating_add(1);
    let n = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
    (n(line), n(column))
}

/// Metadata for one transcoded table column, used for lazy row-cell path relocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColKind {
    Stable(&'static str),
    Array(&'static str, u32),
    Opt(usize),
}

/// Origin of one transcoded TSV line.
#[derive(Clone, Debug, PartialEq, Eq)]
enum LineOrigin {
    /// Metadata lines and table headers (every cell shares one JSON pointer).
    Single { path: Path, cells: usize },
    /// A transcoded table row whose cell pointers are reconstructed lazily.
    Row {
        table_index: usize,
        row_index: u32,
        opt_indices: Vec<Option<u32>>,
    },
}

/// The TSV text being built, per-line origins for lazy pointer relocation, and
/// the findings of the transcoding.
#[derive(Default)]
pub(super) struct Tsv {
    text: String,
    lines: Vec<LineOrigin>,
    findings: Vec<Finding>,
    /// Element name → ids, in input order (for the abundance arrays).
    pub(super) ids: BTreeMap<&'static str, Vec<u32>>,
    /// Transcoded table schemas + columns, for lazy row-cell relocation.
    tables: Vec<(&'static Table, Vec<ColKind>)>,
}

impl Tsv {
    pub(super) fn report(&mut self, code: Code, at: &Path, detail: Detail) {
        self.findings
            .push(Finding::new(code, Location::Json(at.clone()), detail));
    }

    pub(super) fn push_table(&mut self, table: &'static Table, cols: Vec<ColKind>) -> usize {
        let index = self.tables.len();
        self.tables.push((table, cols));
        index
    }

    /// Checks a TSV line whose cells all share `at` and appends it if valid.
    pub(super) fn push_uniform_line<'s>(
        &mut self,
        cells: impl IntoIterator<Item = &'s str>,
        at: &Path,
    ) {
        let start = self.text.len();
        let mut count = 0_usize;
        let mut last_had_cr = false;
        for cell in cells {
            if cell.contains(['\t', '\n']) {
                self.text.truncate(start);
                self.report(Code::JsonText, at, Detail::None);
                return;
            }
            last_had_cr = cell.ends_with('\r');
            if count > 0 {
                self.text.push('\t');
            }
            self.text.push_str(cell);
            count += 1;
        }
        if last_had_cr {
            self.text.truncate(start);
            self.report(Code::JsonText, at, Detail::None);
            return;
        }
        self.text.push('\n');
        self.lines.push(LineOrigin::Single {
            path: at.clone(),
            cells: count,
        });
    }

    pub(super) fn push_row_line(
        &mut self,
        table_index: usize,
        row_index: u32,
        opt_indices: Vec<Option<u32>>,
        row_text: &str,
    ) {
        self.text.push_str(row_text);
        self.text.push('\n');
        self.lines.push(LineOrigin::Row {
            table_index,
            row_index,
            opt_indices,
        });
    }

    pub(super) fn row_path(
        &self,
        table_index: usize,
        row_index: u32,
        opt_indices: &[Option<u32>],
        cell_idx: usize,
    ) -> Option<Path> {
        let (table, cols) = self.tables.get(table_index)?;
        let row_path = || Path::new().key(table.json).index(row_index);
        let Some(col_idx) = cell_idx.checked_sub(1) else {
            return Some(row_path());
        };
        match cols.get(col_idx)? {
            ColKind::Stable(key) => Some(row_path().key(*key)),
            ColKind::Array(key, pos) => Some(row_path().key(*key).index(*pos)),
            ColKind::Opt(slot) => Some(opt_indices.get(*slot).copied().flatten().map_or_else(
                || row_path().key("opt"),
                |opt_idx| row_path().key("opt").index(opt_idx).key("value"),
            )),
        }
    }

    /// The JSON pointer of a TSV location (the TSV reader locates every
    /// finding by line; anything else is kept).
    fn relocate(&self, location: &Location) -> Location {
        let Location::Text { line, cell, .. } = *location else {
            return location.clone();
        };
        let origin = usize::try_from(line)
            .ok()
            .and_then(|l| l.checked_sub(1))
            .and_then(|l| self.lines.get(l));
        // Cell 1 (the prefix) holds the line's own pointer.
        let n = cell.map_or(0, |c| {
            usize::try_from(c).unwrap_or(usize::MAX).saturating_sub(1)
        });
        match origin {
            Some(LineOrigin::Single { path, cells }) if n < *cells => Location::Json(path.clone()),
            Some(LineOrigin::Row {
                table_index,
                row_index,
                opt_indices,
            }) => self
                .row_path(*table_index, *row_index, opt_indices, n)
                .map_or(Location::Document, Location::Json),
            _ => Location::Document,
        }
    }
}

fn transcode(root: &Node<'_>, tsv: &mut Tsv) -> bool {
    let top = Path::new();
    let Value::Object(members) = &root.value else {
        tsv.report(Code::JsonShape, &top, Detail::Expected("an object"));
        return false;
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
        return false;
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
    true
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

pub(super) fn mtd(tsv: &mut Tsv, key: &str, value: &str, at: &Path) {
    tsv.push_uniform_line(["MTD", key, value], at);
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
    use alloc::vec;

    use super::{ColKind, Tsv};
    use crate::diag::location::{Location, Path};
    use crate::json::schema::TABLES;

    #[test]
    fn relocation_keeps_what_is_not_a_text_location() {
        let mut tsv = Tsv::default();
        let model = Location::Model(Path::new().key("x"));
        assert_eq!(tsv.relocate(&model), model, "kept");
        assert_eq!(
            tsv.relocate(&Location::text(9, 1)),
            Location::Document,
            "no such line"
        );

        let meta_path = Path::new().key("metadata").key("title");
        tsv.push_uniform_line(["MTD", "title", "ok"], &meta_path);
        assert_eq!(
            tsv.relocate(&Location::Text {
                line: 1,
                column: 1,
                cell: Some(3),
            }),
            Location::Json(meta_path),
            "uniform line within bounds"
        );
        assert_eq!(
            tsv.relocate(&Location::Text {
                line: 1,
                column: 1,
                cell: Some(4),
            }),
            Location::Document,
            "uniform line out of bounds"
        );

        let smf = &TABLES[1];
        let table_idx = tsv.push_table(
            smf,
            vec![
                ColKind::Stable("smf_id"),
                ColKind::Array("abundance_assay", 1),
                ColKind::Opt(0),
                ColKind::Opt(1),
            ],
        );
        tsv.push_row_line(table_idx, 2, vec![Some(3), None], "SMF\t1\t2\ta\tnull");
        let loc = |cell| {
            tsv.relocate(&Location::Text {
                line: 2,
                column: 1,
                cell,
            })
        };
        let row = || Path::new().key("smallMoleculeFeature").index(2);
        assert_eq!(
            loc(None),
            Location::Json(row()),
            "missing cell defaults to row"
        );
        assert_eq!(loc(Some(1)), Location::Json(row()), "prefix cell");
        assert_eq!(loc(Some(2)), Location::Json(row().key("smf_id")), "stable");
        assert_eq!(
            loc(Some(3)),
            Location::Json(row().key("abundance_assay").index(1)),
            "array"
        );
        assert_eq!(
            loc(Some(4)),
            Location::Json(row().key("opt").index(3).key("value")),
            "present opt"
        );
        assert_eq!(
            loc(Some(5)),
            Location::Json(row().key("opt")),
            "absent second opt"
        );
        assert_eq!(loc(Some(6)), Location::Document, "row out of bounds");
    }
}
