//! JSCH JSON → model, through TSV text.
//!
//! Each JSON value is carried into the TSV line that holds it, and the
//! TSV reader types the result; its findings are then relocated to JSON
//! pointers. A string where a structured value is expected is taken as
//! that value's TSV text, so whatever the writer could not express in JSON
//! form reads back unchanged.

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::schema::{self, Col, Element, Item, METADATA, SNAKE_DIALECT, Shape, TABLES, Table};
use super::value::{self, Node, Value};
use crate::diag::{Code, Detail, Finding, Findings, Location, Path};
use crate::owned::IntoOwned as _;
use crate::param::{Accession, Param, ParamText};
use crate::read::Parsed;

/// Reads JSCH JSON. See the [module documentation](super).
pub(super) fn parse(input: &str) -> Parsed<'static> {
    let mut findings = Vec::new();
    let text = input.strip_prefix('\u{feff}').unwrap_or(input);
    let root = match value::parse(text) {
        Ok(root) => root,
        Err(e) => {
            let (line, column) = line_column(text, e.at);
            findings.push(Finding::new(
                Code::JsonSyntax,
                Location::text(line, column),
                Detail::Expected(e.what),
            ));
            return Parsed {
                mztab: crate::model::MzTab::default(),
                findings: collect(findings),
            };
        }
    };
    let mut tsv = Tsv::default();
    transcode(&root, &mut tsv);
    let layout = comments(&root, &mut tsv);
    let parsed = crate::read::parse(&tsv.text);
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
fn comments(root: &Node<'_>, tsv: &mut Tsv) -> crate::model::SourceLayout<'static> {
    use crate::model::{ExtraLine, LineContent, LineEnding, SourceLine};
    let mut layout = crate::model::SourceLayout::default();
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
            content: LineContent::Extra(ExtraLine::Comment(crate::value::Raw::owned(format!(
                "COM\t{msg}"
            )))),
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
struct Tsv {
    text: String,
    /// Per line: the pointer of each cell (cell 1, the prefix, holds the
    /// line's own pointer).
    cells: Vec<Vec<Path>>,
    findings: Vec<Finding>,
    /// Element name → ids, in input order (for the abundance arrays).
    ids: BTreeMap<&'static str, Vec<u32>>,
}

impl Tsv {
    fn report(&mut self, code: Code, at: &Path, detail: Detail) {
        self.findings
            .push(Finding::new(code, Location::Json(at.clone()), detail));
    }

    /// Appends a line; refuses cells that TSV cannot carry.
    fn line(&mut self, cells: &[(String, Path)]) {
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
            let at = at.clone();
            self.report(Code::JsonText, &at, Detail::None);
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
        metadata_lines(metadata, &top.clone().key("metadata"), tsv);
    }
    for table in &TABLES {
        if let Some(rows) = root.get(table.json).filter(|n| !n.is_null()) {
            table_lines(table, rows, &top.clone().key(table.json), tsv);
        }
    }
}

/// Reports members not in `known`, and repeated members (the first one is
/// read).
fn unknown_keys(members: &[(Cow<'_, str>, Node<'_>)], known: &[&str], at: &Path, tsv: &mut Tsv) {
    check_members(members, at, tsv, |key| known.contains(&key));
}

fn check_members(
    members: &[(Cow<'_, str>, Node<'_>)],
    at: &Path,
    tsv: &mut Tsv,
    known: impl Fn(&str) -> bool,
) {
    let mut seen = alloc::collections::BTreeSet::new();
    for (key, _) in members {
        let here = at.clone().key(key.to_string());
        if !seen.insert(key.as_ref()) {
            tsv.report(
                Code::JsonShape,
                &here,
                Detail::Expected("a member only once"),
            );
        } else if !known(key) {
            tsv.report(Code::JsonUnknownKey, &here, Detail::None);
        }
    }
}

fn mtd(tsv: &mut Tsv, key: String, value: String, at: &Path) {
    tsv.line(&[
        ("MTD".to_string(), at.clone()),
        (key, at.clone()),
        (value, at.clone()),
    ]);
}

fn metadata_lines(node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
    let Value::Object(members) = &node.value else {
        tsv.report(Code::JsonShape, at, Detail::Expected("an object"));
        return;
    };
    let mut known = Vec::from(["prefix"]);
    for item in METADATA {
        let name = match item {
            Item::Field(json, ..) | Item::Indexed(json) | Item::Colunit(json) => *json,
            Item::Element(e) => e.json,
        };
        known.push(name);
        let Some(value) = node.get(name).filter(|n| !n.is_null()) else {
            continue;
        };
        let here = at.clone().key(name);
        match item {
            Item::Field(_, tsv_key, shape) => {
                if let Some(text) = text(value, *shape, &here, tsv) {
                    mtd(tsv, (*tsv_key).to_string(), text, &here);
                }
            }
            Item::Element(element) => element_lines(element, value, &here, tsv),
            Item::Indexed(name) => {
                let ids = indexed_lines(name, value, &here, tsv);
                tsv.ids.insert(name, ids);
            }
            Item::Colunit(key) => {
                for (n, unit, there) in items(value, &here, tsv) {
                    let _ = n;
                    let text = match &unit.value {
                        Value::String(s) => Some(s.to_string()),
                        Value::Object(_) => colunit_text(unit, &there, tsv),
                        _ => shape_error(tsv, &there, "a column unit mapping"),
                    };
                    if let Some(text) = text {
                        mtd(tsv, (*key).to_string(), text, &there);
                    }
                }
            }
        }
    }
    unknown_keys(members, &known, at, tsv);
}

/// The items of an array, with their positions and pointers.
fn items<'n, 'a>(node: &'n Node<'a>, at: &Path, tsv: &mut Tsv) -> Vec<(u32, &'n Node<'a>, Path)> {
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

fn shape_error<T>(tsv: &mut Tsv, at: &Path, expected: &'static str) -> Option<T> {
    tsv.report(Code::JsonShape, at, Detail::Expected(expected));
    None
}

/// The index of an array item: its `id`, or its position + 1.
fn item_id(item: &Node<'_>, position: u32, at: &Path, tsv: &mut Tsv) -> u32 {
    match item.get("id").map(|n| &n.value) {
        Some(Value::Number(n)) => match n.parse::<u32>() {
            Ok(id) if id > 0 => return id,
            _ => {
                let at = at.clone().key("id");
                tsv.report(Code::JsonShape, &at, Detail::Expected("a positive integer"));
            }
        },
        Some(Value::Null) | None => {}
        Some(_) => {
            let at = at.clone().key("id");
            tsv.report(Code::JsonShape, &at, Detail::Expected("a positive integer"));
        }
    }
    position.saturating_add(1)
}

fn element_lines(element: &Element, node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
    let mut ids = Vec::new();
    for (position, object, here) in items(node, at, tsv) {
        let Value::Object(members) = &object.value else {
            tsv.report(Code::JsonShape, &here, Detail::Expected("an object"));
            continue;
        };
        let id = item_id(object, position, &here, tsv);
        ids.push(id);
        let base = format!("{}[{id}]", element.tsv);
        let mut known = Vec::from(["id"]);
        if let Some((json, shape)) = element.value {
            known.push(json);
            if let Some(value) = object.get(json).filter(|n| !n.is_null()) {
                let there = here.clone().key(json);
                if let Some(text) = text(value, shape, &there, tsv) {
                    mtd(tsv, base.clone(), text, &there);
                }
            }
        }
        for (json, sub, shape) in element.props {
            known.push(json);
            let Some(value) = object.get(json).filter(|n| !n.is_null()) else {
                continue;
            };
            let there = here.clone().key(*json);
            let key = format!("{base}-{sub}");
            match shape {
                Shape::Indexed => {
                    indexed_lines(&key, value, &there, tsv);
                }
                Shape::Strings => {
                    for (n, item, place) in items(value, &there, tsv) {
                        if let Some(text) = text(item, Shape::Text, &place, tsv) {
                            mtd(tsv, format!("{key}[{}]", n + 1), text, &place);
                        }
                    }
                }
                _ => {
                    if let Some(text) = text(value, *shape, &there, tsv) {
                        mtd(tsv, key, text, &there);
                    }
                }
            }
        }
        // `name` of an ms_run is JSCH's, with no TSV key (DECISIONS D-071).
        if element.json == "ms_run" {
            known.push("name");
        }
        unknown_keys(members, &known, &here, tsv);
    }
    tsv.ids.insert(element.tsv, ids);
}

/// `key[n]` lines from an array of parameters; returns the indices.
fn indexed_lines(key: &str, node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Vec<u32> {
    let mut ids = Vec::new();
    for (position, item, here) in items(node, at, tsv) {
        let id = match &item.value {
            Value::Object(_) => item_id(item, position, &here, tsv),
            _ => position + 1,
        };
        // A `null` item only pads the array (D-071): no key.
        if let Some(text) = text(item, Shape::Param, &here, tsv) {
            ids.push(id);
            mtd(tsv, format!("{key}[{id}]"), text, &here);
        }
    }
    ids
}

/// The TSV text of a metadata value.
fn text(node: &Node<'_>, shape: Shape, at: &Path, tsv: &mut Tsv) -> Option<String> {
    match (&node.value, shape) {
        // JSON `null` is an absent key; the string `"null"` is a `null` value.
        (Value::Null, _) => None,
        (Value::String(s), _) => Some(s.to_string()),
        (Value::Number(n), Shape::Text | Shape::Strings) => Some(plain(n)),
        (Value::Number(n), Shape::Ref(element)) => Some(format!("{element}[{n}]")),
        (Value::Object(_), Shape::Param | Shape::Indexed) => param_text(node, at, tsv),
        (Value::Array(_), Shape::Params) => joined(node, at, tsv, param_text),
        (Value::Array(_), Shape::Refs(element)) => {
            joined(node, at, tsv, |item, here, tsv| match &item.value {
                Value::Number(n) => Some(format!("{element}[{n}]")),
                _ => shape_error(tsv, here, "an integer"),
            })
        }
        (Value::Array(_), Shape::Publication) => joined(node, at, tsv, |item, here, tsv| {
            let field = |key: &str| match item.get(key).map(|n| &n.value) {
                Some(Value::String(s)) => Some(s.to_string()),
                _ => None,
            };
            match (field("type"), field("accession")) {
                (Some(kind), Some(accession)) => Some(format!("{kind}:{accession}")),
                _ => shape_error(tsv, here, "a publication item"),
            }
        }),
        _ => shape_error(tsv, at, expected(shape)),
    }
}

const fn expected(shape: Shape) -> &'static str {
    match shape {
        Shape::Text | Shape::Strings => "a string",
        Shape::Param | Shape::Indexed => "a parameter",
        Shape::Params => "an array of parameters",
        Shape::Ref(_) => "an integer",
        Shape::Refs(_) => "an array of integers",
        Shape::Publication => "an array of publication items",
    }
}

/// Items joined with `|`; an empty array is `null`.
fn joined(
    node: &Node<'_>,
    at: &Path,
    tsv: &mut Tsv,
    mut item: impl FnMut(&Node<'_>, &Path, &mut Tsv) -> Option<String>,
) -> Option<String> {
    let parts: Vec<Option<String>> = items(node, at, tsv)
        .into_iter()
        .map(|(_, node, here)| match &node.value {
            Value::Null => Some("null".to_string()),
            Value::String(s) => Some(s.to_string()),
            _ => item(node, &here, tsv),
        })
        .collect();
    if parts.is_empty() {
        return Some("null".to_string());
    }
    let parts: Option<Vec<String>> = parts.into_iter().collect();
    parts.map(|p| p.join("|"))
}

/// The TSV text of a `Parameter` object.
fn param_text(node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Option<String> {
    let field = |key: &str| -> Result<Option<&str>, ()> {
        match node.get(key).map(|n| &n.value) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) if s.is_empty() => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(Value::Number(n)) => Ok(Some(n)),
            Some(_) => Err(()),
        }
    };
    let (Ok(label), Ok(accession), Ok(Some(name)), Ok(value)) = (
        field("cv_label"),
        field("cv_accession"),
        field("name"),
        field("value"),
    ) else {
        return shape_error(tsv, at, "a parameter");
    };
    let built = (|| {
        let value = value.map(ParamText::new).transpose().ok()?;
        let name = ParamText::new(name).ok()?;
        Some(match (label, accession) {
            (None, None) => Param::User { name, value },
            (Some(label), Some(accession)) => Param::Cv {
                label: ParamText::new(label).ok()?,
                accession: Accession::new(ParamText::new(accession).ok()?),
                name,
                value,
            },
            _ => return None,
        })
    })();
    built.map_or_else(
        || shape_error(tsv, at, "a parameter"),
        |param| Some(param.to_string()),
    )
}

fn colunit_text(node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Option<String> {
    let column = match node.get("column_name").map(|n| &n.value) {
        Some(Value::String(s)) => s.to_string(),
        _ => return shape_error(tsv, at, "a column unit mapping"),
    };
    let param = node.get("param")?;
    let unit = param_text(param, &at.clone().key("param"), tsv)?;
    Some(format!("{column}={unit}"))
}

/// What a JSON row key is in the header.
enum Key {
    Stable(&'static str, Col),
    /// Index in `table.arrays`, element, column prefix.
    Array(usize, &'static str, &'static str),
    Opt,
    Ignored,
}

fn row_key(table: &Table, key: &str) -> Option<Key> {
    if matches!(key, "prefix" | "header_prefix" | "comment") {
        return Some(Key::Ignored);
    }
    if key == "opt" {
        return Some(Key::Opt);
    }
    if let Some((name, col)) = table.columns.iter().find(|(c, _)| schema::key(c) == key) {
        return Some(Key::Stable(name, *col));
    }
    table
        .arrays
        .iter()
        .enumerate()
        .find(|(_, (json, ..))| *json == key)
        .map(|(a, (_, element, prefix))| Key::Array(a, element, prefix))
}

/// A header column of the transcoded table.
enum Column {
    Stable(&'static str, Col),
    /// Array index, position in the array.
    Array(usize, usize),
    Opt(String),
}

fn table_lines(table: &Table, node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
    let rows = items(node, at, tsv);
    let Some((_, first, first_at)) = rows.first() else {
        // No rows, so no keys to take columns from: the full header of the
        // table, with one abundance column per metadata element (D-071).
        let ids = |element: &str| tsv.ids.get(element).cloned().unwrap_or_default();
        let names = schema::standard_header(table, ids);
        let mut header = Vec::from([(table.header.to_string(), at.clone())]);
        header.extend(names.into_iter().map(|name| (name, at.clone())));
        tsv.line(&header);
        return;
    };
    let Value::Object(members) = &first.value else {
        tsv.report(Code::JsonShape, first_at, Detail::Expected("an object"));
        return;
    };
    let mut columns: Vec<(String, Column)> = Vec::new();
    let mut seen = alloc::collections::BTreeSet::new();
    for (key, value) in members {
        if !seen.insert(key.as_ref()) {
            continue;
        }
        match row_key(table, key) {
            Some(Key::Stable(name, col)) => {
                columns.push((name.to_string(), Column::Stable(name, col)));
            }
            Some(Key::Array(a, element, prefix)) => {
                let len = match &value.value {
                    Value::Array(items) => items.len(),
                    _ => 0,
                };
                let ids = tsv.ids.get(element).cloned().unwrap_or_default();
                if ids.len() != len {
                    let here = first_at.clone().key(key.to_string());
                    tsv.report(
                        Code::JsonShape,
                        &here,
                        Detail::Expected("one value per metadata element"),
                    );
                }
                for p in 0..len {
                    let id = ids
                        .get(p)
                        .copied()
                        .unwrap_or_else(|| u32::try_from(p + 1).unwrap_or(u32::MAX));
                    columns.push((format!("{prefix}[{id}]"), Column::Array(a, p)));
                }
            }
            Some(Key::Opt) => {
                if let Value::Array(opts) = &value.value {
                    // A repeated identifier is one column (its first value).
                    let mut names = alloc::collections::BTreeSet::new();
                    for opt in opts {
                        if let Some(name) = opt_name(opt)
                            && names.insert(name.clone())
                        {
                            columns.push((format!("opt_{name}"), Column::Opt(name)));
                        }
                    }
                }
            }
            Some(Key::Ignored) | None => {}
        }
    }
    let mut header: Vec<(String, Path)> = Vec::from([(table.header.to_string(), at.clone())]);
    header.extend(columns.iter().map(|(name, _)| (name.clone(), at.clone())));
    tsv.line(&header);
    for (_, row, here) in &rows {
        let Value::Object(members) = &row.value else {
            tsv.report(Code::JsonShape, here, Detail::Expected("an object"));
            continue;
        };
        check_members(members, here, tsv, |key| row_key(table, key).is_some());
        let mut cells = Vec::from([(table.row.to_string(), here.clone())]);
        for (_, column) in &columns {
            cells.push(row_cell(table, column, row, here, tsv));
        }
        tsv.line(&cells);
    }
}

fn opt_name(opt: &Node<'_>) -> Option<String> {
    match opt.get("identifier").map(|n| &n.value) {
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn row_cell(
    table: &Table,
    column: &Column,
    row: &Node<'_>,
    at: &Path,
    tsv: &mut Tsv,
) -> (String, Path) {
    let null = || "null".to_string();
    match column {
        Column::Stable(name, col) => {
            let key = schema::key(name);
            let here = at.clone().key(key.clone());
            let text = row
                .get(&key)
                .and_then(|value| cell_text(value, *col, &here, tsv))
                .unwrap_or_else(null);
            (text, here)
        }
        Column::Array(a, p) => {
            let json = table.arrays.get(*a).map_or("", |(json, ..)| json);
            let index = u32::try_from(*p).unwrap_or(u32::MAX);
            let here = at.clone().key(json).index(index);
            let item = match row.get(json).map(|n| &n.value) {
                Some(Value::Array(items)) => items.get(*p),
                _ => None,
            };
            let text = item
                .and_then(|value| cell_text(value, Col::Number, &here, tsv))
                .unwrap_or_else(null);
            (text, here)
        }
        Column::Opt(name) => {
            let opts = match row.get("opt").map(|n| &n.value) {
                Some(Value::Array(opts)) => opts.as_slice(),
                _ => &[],
            };
            let found = opts
                .iter()
                .enumerate()
                .find(|(_, o)| opt_name(o).as_deref() == Some(name.as_str()));
            let Some((n, opt)) = found else {
                return (null(), at.clone().key("opt"));
            };
            let index = u32::try_from(n).unwrap_or(u32::MAX);
            let here = at.clone().key("opt").index(index).key("value");
            let text = opt
                .get("value")
                .and_then(|value| cell_text(value, Col::Text, &here, tsv))
                .unwrap_or_else(null);
            (text, here)
        }
    }
}

/// A JSON number in plain decimal notation (mzTab forbids exponents): the
/// digits of `1.5e3` are moved to `1500`, exactly.
fn plain(lexeme: &str) -> String {
    let Some((mantissa, exponent)) = lexeme.split_once(['e', 'E']) else {
        return lexeme.to_string();
    };
    let Ok(exponent) = exponent.parse::<i64>() else {
        return lexeme.to_string();
    };
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |m| ("-", m));
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{int}{frac}");
    let point = i64::try_from(int.len())
        .unwrap_or(i64::MAX)
        .saturating_add(exponent);
    let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    // Bound the padding: a pathological exponent stays as written.
    if !(-1000..=1000).contains(&point) {
        return lexeme.to_string();
    }
    let (int, frac) = if point <= 0 {
        let zeros = "0".repeat(usize::try_from(-point).unwrap_or(0));
        ("0".to_string(), format!("{zeros}{digits}"))
    } else if point >= len {
        let zeros = "0".repeat(usize::try_from(point - len).unwrap_or(0));
        (format!("{digits}{zeros}"), String::new())
    } else {
        let at = usize::try_from(point).unwrap_or(0);
        let (i, f) = digits.split_at(at);
        (i.to_string(), f.to_string())
    };
    let int = int.trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    if frac.is_empty() {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    }
}

/// The TSV text of a table cell.
fn cell_text(node: &Node<'_>, col: Col, at: &Path, tsv: &mut Tsv) -> Option<String> {
    match (&node.value, col) {
        (Value::Null, _) => Some("null".to_string()),
        (Value::String(s), _) => Some(s.to_string()),
        (Value::Number(n), Col::Text | Col::Number | Col::Integer) => Some(plain(n)),
        (Value::Object(_), Col::Param) => param_text(node, at, tsv),
        (Value::Array(_), Col::Texts) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Text, here, tsv)
        }),
        (Value::Array(_), Col::Numbers) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Number, here, tsv)
        }),
        (Value::Array(_), Col::Integers) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Integer, here, tsv)
        }),
        (Value::Array(_), Col::SpectraRefs) => joined(node, at, tsv, |item, here, tsv| {
            match (
                item.get("ms_run").map(|n| &n.value),
                item.get("reference").map(|n| &n.value),
            ) {
                (Some(Value::Number(run)), Some(Value::String(reference))) => {
                    Some(format!("ms_run[{run}]:{reference}"))
                }
                _ => shape_error(tsv, here, "a spectra reference"),
            }
        }),
        _ => shape_error(tsv, at, col_expected(col)),
    }
}

const fn col_expected(col: Col) -> &'static str {
    match col {
        Col::Text => "a string",
        Col::Number => "a number",
        Col::Integer => "an integer",
        Col::Param => "a parameter",
        Col::Texts => "an array of strings",
        Col::Numbers => "an array of numbers",
        Col::Integers => "an array of integers",
        Col::SpectraRefs => "an array of spectra references",
    }
}

#[cfg(test)]
mod tests {
    use super::Tsv;
    use crate::diag::{Location, Path};

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
