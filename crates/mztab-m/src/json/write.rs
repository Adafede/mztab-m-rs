//! Model → JSCH JSON, streamed directly from canonical TSV text.
//!
//! The model is formatted into canonical TSV without cloning, validated
//! (metadata and table schemas) up front before writing to the sink, and
//! then streamed directly as compact JSON in schema order.

mod emit;
mod meta;
mod table;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::schema::TABLES;
use crate::model::document::MzTab;
use crate::model::layout::{ExtraLine, LineContent};
use crate::write::error::WriteError;
use emit::JsonWriter;

/// Writes `mztab` as JSCH JSON (compact, members in schema order).
///
/// # Errors
///
/// [`WriteError::Fmt`] if `out` fails; [`WriteError::Unwritable`] if the
/// model cannot be written as TSV, or holds what JSCH has no place for:
/// unknown metadata keys, unknown columns, or abundance columns that do
/// not match the metadata's assays and study variables one to one.
pub(super) fn write(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    let mut tsv = String::new();
    crate::write::write_canonical(mztab, &mut tsv)?;
    let doc = Doc::of(&tsv);
    let (meta_json, ids) = meta::encode_metadata(&doc.mtd)?;
    let plans = table::plan_tables(&doc.tables, &ids)?;
    stream_root(&meta_json, &plans, mztab, out).map_err(|_| WriteError::Fmt)
}

fn stream_root(
    meta_json: &str,
    plans: &[Option<table::TablePlan<'_>>],
    mztab: &MzTab<'_>,
    out: &mut dyn fmt::Write,
) -> fmt::Result {
    let mut w = JsonWriter::new(out);
    w.raw("{\"metadata\":")?;
    w.raw(meta_json)?;
    for (spec, plan) in TABLES.iter().zip(plans) {
        w.char(',')?;
        w.str(spec.json)?;
        w.char(':')?;
        match plan {
            Some(plan) => table::write_table(&mut w, plan)?,
            None => w.raw("null")?,
        }
    }
    w.raw(",\"comment\":")?;
    write_comments(mztab, &mut w)?;
    w.char('}')
}

/// The source's `COM` lines as `Comment` objects, with their line numbers (D-075).
fn write_comments(mztab: &MzTab<'_>, w: &mut JsonWriter<'_>) -> fmt::Result {
    let mut any = false;
    for (n, line) in mztab.layout.lines.iter().enumerate() {
        let LineContent::Extra(ExtraLine::Comment(raw)) = &line.content else {
            continue;
        };
        let msg = raw.as_str().split_once('\t').map_or("", |(_, msg)| msg);
        w.char(if any { ',' } else { '[' })?;
        any = true;
        w.raw("{\"prefix\":\"COM\",\"msg\":")?;
        w.str(msg)?;
        write!(w.out(), ",\"line_number\":{}}}", n + 1)?;
    }
    if any { w.char(']') } else { w.raw("null") }
}

pub(super) const fn unwritable(line: u32, reason: &'static str) -> WriteError {
    WriteError::Unwritable { line, reason }
}

/// Splits `name[n]rest` into its parts.
pub(super) fn split_key(key: &str) -> Option<(&str, u32, &str)> {
    let (name, rest) = key.split_once('[')?;
    let (n, rest) = rest.split_once(']')?;
    Some((name, n.parse().ok()?, rest))
}

/// The lines of the canonical TSV, by section.
struct Doc<'t> {
    /// `(line, key, value)`.
    mtd: Vec<(u32, &'t str, &'t str)>,
    /// Header prefix → section.
    tables: BTreeMap<&'t str, Section<'t>>,
}

pub(super) struct Section<'t> {
    pub(super) line: u32,
    pub(super) columns: Vec<&'t str>,
    pub(super) rows: Vec<(u32, Vec<&'t str>)>,
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
