//! Model → JSCH JSON, through the model's canonical TSV text.
//!
//! The model is written as canonical TSV first (so every value is in its
//! one canonical form), and each line is then carried into JSON by the
//! tables of [`schema`]. A value whose text does not have
//! the JSON form of its field (an invalid cell) is written as a string
//! holding that text, which the reader parses back with the TSV grammar.

use alloc::borrow::Cow;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;
use core::fmt;

use super::schema::{self, Col, Element, Item, METADATA, Shape, TABLES, Table};
use super::value::{is_number, write_string};
use crate::meta::MtdKey;
use crate::model::MzTab;
use crate::param::Param;
use crate::tsv::split_list;
use crate::write::WriteError;

/// A JSON value to write.
#[derive(Debug)]
enum J {
    Null,
    /// A number lexeme, written as is.
    Number(String),
    Str(String),
    Arr(Vec<Self>),
    Obj(Vec<(Cow<'static, str>, Self)>),
}

impl J {
    fn str(text: &str) -> Self {
        Self::Str(text.to_string())
    }

    fn write(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Self::Null => out.write_str("null"),
            Self::Number(n) => out.write_str(n),
            Self::Str(s) => write_string(out, s),
            Self::Arr(items) => {
                out.write_char('[')?;
                for (n, item) in items.iter().enumerate() {
                    if n > 0 {
                        out.write_char(',')?;
                    }
                    item.write(out)?;
                }
                out.write_char(']')
            }
            Self::Obj(members) => {
                out.write_char('{')?;
                for (n, (key, value)) in members.iter().enumerate() {
                    if n > 0 {
                        out.write_char(',')?;
                    }
                    write_string(out, key)?;
                    out.write_char(':')?;
                    value.write(out)?;
                }
                out.write_char('}')
            }
        }
    }
}

/// Writes `mztab` as JSCH JSON (compact, members in schema order).
///
/// # Errors
///
/// [`WriteError::Fmt`] if `out` fails; [`WriteError::Unwritable`] if the
/// model cannot be written as TSV, or holds what JSCH has no place for:
/// unknown metadata keys, unknown columns, or abundance columns that do
/// not match the metadata's assays and study variables one to one.
pub(super) fn write(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    let mut canonical = mztab.clone();
    canonical.layout = crate::model::SourceLayout::default();
    let tsv = crate::write::to_string(&canonical)?;
    let doc = Doc::of(&tsv);
    let mut metadata = Metadata::new(&doc.mtd);
    let mut root = vec_obj();
    root.push(("metadata".into(), metadata.object()));
    if let Some(line) = metadata.unused() {
        return Err(unwritable(line, "metadata key has no JSON form"));
    }
    if let Some(line) = metadata.refused {
        return Err(unwritable(
            line,
            "indices too far apart to place in a JSON array",
        ));
    }
    for table in &TABLES {
        let rows = match doc.tables.get(table.header) {
            Some(section) => rows(table, section, &metadata.ids)?,
            None => J::Null,
        };
        root.push((table.json.into(), rows));
    }
    root.push(("comment".into(), comments(mztab)));
    J::Obj(root).write(out).map_err(|_| WriteError::Fmt)
}

/// The source's `COM` lines as `Comment` objects, with their line numbers
/// (D-075).
fn comments(mztab: &MzTab<'_>) -> J {
    let items: Vec<J> = mztab
        .layout
        .lines
        .iter()
        .enumerate()
        .filter_map(|(n, line)| match &line.content {
            crate::model::LineContent::Extra(crate::model::ExtraLine::Comment(raw)) => {
                let text = raw.as_str();
                let msg = text.split_once('\t').map_or("", |(_, msg)| msg);
                Some(J::Obj(Vec::from([
                    ("prefix".into(), J::str("COM")),
                    ("msg".into(), J::str(msg)),
                    ("line_number".into(), J::Number((n + 1).to_string())),
                ])))
            }
            _ => None,
        })
        .collect();
    if items.is_empty() {
        J::Null
    } else {
        J::Arr(items)
    }
}

const fn unwritable(line: u32, reason: &'static str) -> WriteError {
    WriteError::Unwritable { line, reason }
}

const fn vec_obj() -> Vec<(Cow<'static, str>, J)> {
    Vec::new()
}

/// The lines of the canonical TSV, by section.
struct Doc<'t> {
    /// `(line, key, value)`.
    mtd: Vec<(u32, &'t str, &'t str)>,
    /// Header prefix → section.
    tables: BTreeMap<&'t str, Section<'t>>,
}

struct Section<'t> {
    line: u32,
    columns: Vec<&'t str>,
    rows: Vec<(u32, Vec<&'t str>)>,
}

impl<'t> Doc<'t> {
    fn of(tsv: &'t str) -> Self {
        let mut doc = Doc {
            mtd: Vec::new(),
            tables: BTreeMap::new(),
        };
        let mut current: Option<&'t str> = None;
        for (n, line) in tsv.lines().enumerate() {
            let number = u32::try_from(n + 1).unwrap_or(u32::MAX);
            let mut cells = line.split('\t');
            let prefix = cells.next().unwrap_or_default();
            if prefix == "MTD" {
                let key = cells.next().unwrap_or_default();
                doc.mtd
                    .push((number, key, cells.next().unwrap_or_default()));
            } else if let Some(table) = TABLES.iter().find(|t| t.header == prefix) {
                current = Some(table.header);
                doc.tables.insert(
                    table.header,
                    Section {
                        line: number,
                        columns: cells.collect(),
                        rows: Vec::new(),
                    },
                );
            } else if let Some(table) = TABLES.iter().find(|t| t.row == prefix)
                && current == Some(table.header)
                && let Some(section) = doc.tables.get_mut(table.header)
            {
                section.rows.push((number, cells.collect()));
            }
        }
        doc
    }
}

/// Splits `name[n]rest` into its parts.
fn split_key(key: &str) -> Option<(&str, u32, &str)> {
    let (name, rest) = key.split_once('[')?;
    let (n, rest) = rest.split_once(']')?;
    Some((name, n.parse().ok()?, rest))
}

/// The metadata lines and what has been carried into JSON.
struct Metadata<'d, 't> {
    lines: &'d [(u32, &'t str, &'t str)],
    used: BTreeSet<usize>,
    /// Element name → its ids, ascending (for the abundance arrays).
    ids: BTreeMap<&'static str, Vec<u32>>,
    /// The line of a key that needs too much padding to be placed.
    refused: Option<u32>,
}

impl<'d, 't> Metadata<'d, 't> {
    const fn new(lines: &'d [(u32, &'t str, &'t str)]) -> Self {
        Self {
            lines,
            used: BTreeSet::new(),
            ids: BTreeMap::new(),
            refused: None,
        }
    }

    /// The first line not carried into JSON.
    fn unused(&self) -> Option<u32> {
        self.lines
            .iter()
            .enumerate()
            .find(|(n, _)| !self.used.contains(n))
            .map(|(_, (line, ..))| *line)
    }

    /// The value of `key`, marked as used.
    fn take(&mut self, key: &str) -> Option<&'t str> {
        let (n, (_, _, value)) = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, (_, k, _))| *k == key)?;
        self.used.insert(n);
        Some(value)
    }

    /// Every `prefix[n]rest` metadata key: `n`, `rest` and value, marked as
    /// used. (Every metadata key has a place in [`METADATA`]: the base
    /// document, which has every key form, round-trips.)
    fn take_indexed(&mut self, prefix: &str) -> Vec<Taken<'t>> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for (n, (line, key, value)) in self.lines.iter().enumerate() {
            // A repeated key (kept by the reader as unknown metadata) has no
            // JSON form: only its first occurrence is taken.
            // A key that is not a metadata key (`x[0]`, `x[01]`) is unknown
            // metadata, which JSON cannot carry.
            if let Some((name, index, rest)) = split_key(key)
                && name == prefix
                && MtdKey::parse(key).is_some()
                && seen.insert(*key)
            {
                out.push(Taken {
                    line: *line,
                    index,
                    rest,
                    value,
                });
                self.used.insert(n);
            }
        }
        out
    }

    fn object(&mut self) -> J {
        let mut members = vec_obj();
        members.push(("prefix".into(), J::str("MTD")));
        for item in METADATA {
            let value = match item {
                Item::Field(json, tsv, shape) => {
                    let value = self.take(tsv).map_or(J::Null, |v| shaped(v, *shape));
                    (Cow::Borrowed(*json), value)
                }
                Item::Element(element) => (Cow::Borrowed(element.json), self.element(element)),
                Item::Indexed(name) => {
                    let items = self.take_indexed(name);
                    let mut ids: Vec<u32> = items.iter().map(|t| t.index).collect();
                    ids.sort_unstable();
                    self.ids.insert(name, ids);
                    let value = self.positioned(&items, false);
                    (Cow::Borrowed(*name), value)
                }
                Item::Colunit(key) => {
                    let mut units = Vec::new();
                    for (n, (_, k, v)) in self.lines.iter().enumerate() {
                        if k == key {
                            self.used.insert(n);
                            units.push(colunit(v));
                        }
                    }
                    let value = if units.is_empty() {
                        J::Null
                    } else {
                        J::Arr(units)
                    };
                    (Cow::Borrowed(*key), value)
                }
            };
            members.push(value);
        }
        J::Obj(members)
    }

    fn element(&mut self, element: &Element) -> J {
        let keys = self.take_indexed(element.tsv);
        let ids: BTreeSet<u32> = keys.iter().map(|t| t.index).collect();
        self.ids.insert(element.tsv, ids.iter().copied().collect());
        if ids.is_empty() {
            return J::Null;
        }
        let mut objects = Vec::new();
        for id in &ids {
            let mine: Vec<&Taken<'_>> = keys.iter().filter(|t| t.index == *id).collect();
            let mut members = vec_obj();
            members.push(("id".into(), J::Number(id.to_string())));
            if let Some((json, shape)) = element.value {
                let value = mine.iter().find(|t| t.rest.is_empty());
                members.push((
                    json.into(),
                    value.map_or(J::Null, |t| shaped(t.value, shape)),
                ));
            }
            for (json, tsv, shape) in element.props {
                let sub = format!("-{tsv}");
                let value = if matches!(shape, Shape::Indexed | Shape::Strings) {
                    let items: Vec<Taken<'_>> = mine
                        .iter()
                        .filter_map(|t| {
                            let (name, n, tail) = split_key(t.rest)?;
                            (name == sub && tail.is_empty()).then_some(Taken { index: n, ..**t })
                        })
                        .collect();
                    self.positioned(&items, *shape == Shape::Strings)
                } else {
                    let value = mine.iter().find(|t| t.rest == sub);
                    value.map_or(J::Null, |t| shaped(t.value, *shape))
                };
                members.push(((*json).into(), value));
            }
            objects.push(J::Obj(members));
        }
        J::Arr(objects)
    }
}

/// A key `prefix[index]rest` taken into JSON, and its line.
#[derive(Clone, Copy, Debug)]
struct Taken<'t> {
    line: u32,
    index: u32,
    rest: &'t str,
    value: &'t str,
}

/// Padding beyond this many `null`s makes a model unwritable.
const MAX_PADDING: u32 = 1024;

impl Metadata<'_, '_> {
    /// Indexed keys as an array: `Parameter` objects carrying their `id`;
    /// any other item (a string, or a value that is not one parameter) at
    /// position `index - 1`, after `null`s where indices are missing.
    fn positioned(&mut self, items: &[Taken<'_>], strings: bool) -> J {
        if items.is_empty() {
            return J::Null;
        }
        let mut sorted = items.to_vec();
        sorted.sort_by_key(|t| t.index);
        let mut out = Vec::new();
        let mut padding = 0;
        for t in sorted {
            let object = (!strings)
                .then(|| Param::parse(t.value).ok())
                .flatten()
                .map(|p| param_object(&p, Some(t.index)));
            if let Some(object) = object {
                out.push(object);
                continue;
            }
            let position = u32::try_from(out.len()).unwrap_or(u32::MAX);
            let missing = t.index.saturating_sub(1).saturating_sub(position);
            padding += missing;
            if padding > MAX_PADDING {
                self.refused.get_or_insert(t.line);
                break;
            }
            out.extend((0..missing).map(|_| J::Null));
            out.push(J::str(t.value));
        }
        J::Arr(out)
    }
}

/// A metadata value in its JSON form, or its text as a string.
fn shaped(text: &str, shape: Shape) -> J {
    // An explicit `null` value: JSON `null` means the key is absent.
    if text == "null" {
        return J::str(text);
    }
    let value = match shape {
        Shape::Text | Shape::Indexed | Shape::Strings => None,
        Shape::Param => param(text),
        Shape::Params => params(text),
        Shape::Ref(element) => reference(text, element),
        Shape::Refs(element) => text
            .split('|')
            .map(|item| reference(item, element))
            .collect::<Option<Vec<J>>>()
            .map(J::Arr),
        Shape::Publication => text
            .split('|')
            .map(|item| {
                let (kind, accession) = item.split_once(':')?;
                Some(J::Obj(Vec::from([
                    ("type".into(), J::str(kind)),
                    ("accession".into(), J::str(accession)),
                ])))
            })
            .collect::<Option<Vec<J>>>()
            .map(J::Arr),
    };
    value.unwrap_or_else(|| J::str(text))
}

fn reference(text: &str, element: &str) -> Option<J> {
    let (name, n, rest) = split_key(text)?;
    (name == element && rest.is_empty()).then(|| J::Number(n.to_string()))
}

/// A `Parameter` object, if `text` is one parameter.
fn param(text: &str) -> Option<J> {
    let param = Param::parse(text).ok()?;
    Some(param_object(&param, None))
}

fn param_object(param: &Param<'_>, id: Option<u32>) -> J {
    let mut members = vec_obj();
    if let Some(id) = id {
        members.push(("id".into(), J::Number(id.to_string())));
    }
    let (label, accession) = match param {
        Param::Cv {
            label, accession, ..
        } => (J::str(label.as_str()), J::str(accession.as_str())),
        Param::User { .. } => (J::Null, J::Null),
    };
    members.push(("cv_label".into(), label));
    members.push(("cv_accession".into(), accession));
    members.push(("name".into(), J::str(param.name().as_str())));
    let value = param.value().map_or(J::Null, |v| J::str(v.as_str()));
    members.push(("value".into(), value));
    J::Obj(members)
}

/// An array of `Parameter` objects, if every item is one.
fn params(text: &str) -> Option<J> {
    split_list(text, true)
        .into_iter()
        .map(|(_, item)| param(item))
        .collect::<Option<Vec<J>>>()
        .map(J::Arr)
}

fn colunit(text: &str) -> J {
    let parsed = text.split_once('=').and_then(|(column, unit)| {
        let unit = param(unit)?;
        Some(J::Obj(Vec::from([
            ("column_name".into(), J::str(column)),
            ("param".into(), unit),
        ])))
    });
    parsed.unwrap_or_else(|| J::str(text))
}

/// What a header column is in JSON.
enum Kind {
    Stable(&'static str, Col),
    /// An abundance array: its JSON name, its index in `table.arrays`,
    /// and the id.
    Array(&'static str, usize, u32),
    Opt(String),
}

fn kind(table: &Table, column: &str) -> Option<Kind> {
    if let Some((name, col)) = table.columns.iter().find(|(c, _)| *c == column) {
        return Some(Kind::Stable(name, *col));
    }
    if let Some(name) = column.strip_prefix("opt_") {
        return Some(Kind::Opt(name.to_string()));
    }
    let (name, n, rest) = split_key(column)?;
    let (at, (json, ..)) = table
        .arrays
        .iter()
        .enumerate()
        .find(|(_, (_, _, c))| *c == name)?;
    rest.is_empty().then_some(Kind::Array(json, at, n))
}

/// A table's rows as JSON objects.
fn rows(
    table: &Table,
    section: &Section<'_>,
    ids: &BTreeMap<&'static str, Vec<u32>>,
) -> Result<J, WriteError> {
    let distinct: BTreeSet<&str> = section.columns.iter().copied().collect();
    if distinct.len() != section.columns.len() {
        return Err(unwritable(section.line, "header repeats a column"));
    }
    let kinds = section
        .columns
        .iter()
        .map(|c| kind(table, c))
        .collect::<Option<Vec<Kind>>>()
        .ok_or_else(|| unwritable(section.line, "column has no JSON form"))?;
    for (at, (_, element, _)) in table.arrays.iter().enumerate() {
        let found: Vec<u32> = kinds
            .iter()
            .filter_map(|k| match k {
                Kind::Array(_, a, n) if *a == at => Some(*n),
                _ => None,
            })
            .collect();
        let expected = ids.get(element).map_or_default(Vec::as_slice);
        if !found.is_empty() && found.as_slice() != expected {
            return Err(unwritable(
                section.line,
                "abundance columns do not match the metadata ids one to one, in order",
            ));
        }
    }
    if section.rows.is_empty() {
        let standard = schema::standard_header(table, |e| ids.get(e).cloned().unwrap_or_default());
        if section
            .columns
            .iter()
            .copied()
            .ne(standard.iter().map(String::as_str))
        {
            return Err(unwritable(
                section.line,
                "table without rows has a header other than the standard one",
            ));
        }
    }
    if let Some((line, _)) = section
        .rows
        .iter()
        .find(|(_, cells)| cells.len() != kinds.len())
    {
        return Err(unwritable(
            *line,
            "row has another number of cells than its header",
        ));
    }
    let rows = section
        .rows
        .iter()
        .map(|(_, cells)| row(table, &kinds, cells))
        .collect();
    Ok(J::Arr(rows))
}

fn row(table: &Table, kinds: &[Kind], cells: &[&str]) -> J {
    let mut members = vec_obj();
    members.push(("prefix".into(), J::str(table.row)));
    members.push(("header_prefix".into(), J::str(table.header)));
    let mut arrays: BTreeMap<&'static str, Vec<J>> = BTreeMap::new();
    let mut opts = Vec::new();
    // Arrays and `opt` go where their first column is.
    let mut order: Vec<Cow<'static, str>> = Vec::new();
    let mut values: BTreeMap<Cow<'static, str>, J> = BTreeMap::new();
    for (kind, text) in kinds.iter().zip(cells) {
        match kind {
            Kind::Stable(name, col) => {
                let key: Cow<'static, str> = Cow::Owned(schema::key(name));
                order.push(key.clone());
                values.insert(key, cell(text, *col));
            }
            Kind::Array(json, ..) => {
                let items = arrays.entry(json).or_default();
                if items.is_empty() {
                    order.push(Cow::Borrowed(*json));
                }
                items.push(cell(text, Col::Number));
            }
            Kind::Opt(name) => {
                if opts.is_empty() {
                    order.push(Cow::Borrowed("opt"));
                }
                let value = if *text == "null" {
                    J::Null
                } else {
                    J::str(text)
                };
                opts.push(J::Obj(Vec::from([
                    ("identifier".into(), J::str(name)),
                    ("param".into(), J::Null),
                    ("value".into(), value),
                ])));
            }
        }
    }
    for (json, items) in arrays {
        values.insert(Cow::Borrowed(json), J::Arr(items));
    }
    if !opts.is_empty() {
        values.insert(Cow::Borrowed("opt"), J::Arr(opts));
    }
    members.extend(
        order
            .into_iter()
            .filter_map(|key| values.remove(&key).map(|value| (key, value))),
    );
    J::Obj(members)
}

/// A cell in its JSON form, or its text as a string.
fn cell(text: &str, col: Col) -> J {
    if text == "null" {
        return J::Null;
    }
    let value = match col {
        Col::Text => None,
        // Only plain numbers: the JSON reader writes exponents out, so a
        // number in another form stays a string (its TSV text).
        Col::Number | Col::Integer => {
            (is_number(text) && !text.contains(['e', 'E'])).then(|| J::Number(text.to_string()))
        }
        Col::Param => param(text),
        Col::Texts | Col::Numbers | Col::Integers => {
            // Numbers and integers are written alike.
            let item = if col == Col::Texts {
                Col::Text
            } else {
                Col::Number
            };
            Some(J::Arr(text.split('|').map(|i| cell(i, item)).collect()))
        }
        Col::SpectraRefs => text
            .split('|')
            .map(|item| {
                let (run, reference) = item.split_once(':')?;
                let n = self::reference(run, "ms_run")?;
                Some(J::Obj(Vec::from([
                    ("ms_run".into(), n),
                    ("reference".into(), J::str(reference)),
                ])))
            })
            .collect::<Option<Vec<J>>>()
            .map(J::Arr),
    };
    value.unwrap_or_else(|| J::str(text))
}
