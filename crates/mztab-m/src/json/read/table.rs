//! JSON table arrays → TSV headers and rows.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::emit::write_cell;
use super::{ColKind, Tsv, items};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::diag::location::Path;
use crate::json::schema::{Col, Table, standard_header};
use crate::json::value::{Node, Value};

/// What a JSON row key is in the header.
enum Key {
    Stable(&'static str, &'static str, Col),
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
    if let Some((tsv_name, json_key, col)) = table.columns.iter().find(|(_, json, _)| *json == key)
    {
        return Some(Key::Stable(tsv_name, json_key, *col));
    }
    table
        .arrays
        .iter()
        .enumerate()
        .find(|(_, (json, ..))| *json == key)
        .map(|(a, (_, element, prefix))| Key::Array(a, element, prefix))
}

/// A header column of the transcoded table.
enum Column<'a> {
    Stable(&'static str, Col),
    /// Array index, position in the array.
    Array(usize, usize),
    Opt(&'a str),
}

pub(super) fn table_lines(table: &'static Table, node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
    if !matches!(node.value, Value::Array(_)) {
        tsv.report(Code::JsonShape, at, Detail::Expected("an array"));
        return;
    }
    let rows = items(node, at, tsv);
    if rows.is_empty() {
        // No rows, so no keys to take columns from: the full header of the
        // table, with one abundance column per metadata element (D-071).
        let names = standard_header(table, |element| {
            tsv.ids.get(element).cloned().unwrap_or_default()
        });
        tsv.push_uniform_line(
            core::iter::once(table.header).chain(names.iter().map(String::as_str)),
            at,
        );
        return;
    }
    if !rows
        .iter()
        .any(|(_, row, _)| matches!(row.value, Value::Object(_)))
    {
        for (_, _, here) in &rows {
            tsv.report(Code::JsonShape, here, Detail::Expected("an object"));
        }
        return;
    }
    let columns = infer_columns(table, &rows, tsv);
    tsv.push_uniform_line(
        core::iter::once(table.header).chain(columns.iter().map(|(name, _)| name.as_str())),
        at,
    );
    let mut opt_slot = 0_usize;
    let col_kinds: Vec<ColKind> = columns
        .iter()
        .map(|(_, col)| match col {
            Column::Stable(json_key, _) => ColKind::Stable(json_key),
            Column::Array(a, p) => {
                let json = table.arrays.get(*a).map_or("", |(json, ..)| json);
                ColKind::Array(json, u32::try_from(*p).unwrap_or(u32::MAX))
            }
            Column::Opt(_) => {
                let slot = opt_slot;
                opt_slot += 1;
                ColKind::Opt(slot)
            }
        })
        .collect();
    let table_index = tsv.push_table(table, col_kinds);
    let mut row_buf = String::new();
    let emit = RowEmit {
        table,
        table_index,
        columns: &columns,
        opt_count: opt_slot,
    };
    for (row_index, row, here) in &rows {
        emit_row(&emit, *row_index, row, here, tsv, &mut row_buf);
    }
}

struct ArrayCol {
    index: usize,
    element: &'static str,
    prefix: &'static str,
}

fn infer_columns<'a>(
    table: &Table,
    rows: &[(u32, &'a Node<'a>, Path)],
    tsv: &Tsv,
) -> Vec<(String, Column<'a>)> {
    let mut max_len = alloc::vec![0usize; table.arrays.len()];
    for (_, row, _) in rows {
        let Value::Object(members) = &row.value else {
            continue;
        };
        for (key, value) in members {
            if let Some(Key::Array(index, ..)) = row_key(table, key)
                && let Value::Array(items) = &value.value
                && let Some(slot) = max_len.get_mut(index)
            {
                *slot = (*slot).max(items.len());
            }
        }
    }

    let mut columns: Vec<(String, Column<'a>)> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_opts = BTreeSet::new();
    for (_, row, _) in rows {
        let Value::Object(members) = &row.value else {
            continue;
        };
        for (key, value) in members {
            match row_key(table, key) {
                Some(Key::Stable(tsv_name, json_key, col)) => {
                    if seen.insert(key.as_ref()) {
                        columns.push((tsv_name.to_string(), Column::Stable(json_key, col)));
                    }
                }
                Some(Key::Array(index, element, prefix)) => {
                    if matches!(value.value, Value::Array(_)) && seen.insert(key.as_ref()) {
                        let array = ArrayCol {
                            index,
                            element,
                            prefix,
                        };
                        let len = max_len.get(index).copied().unwrap_or(0);
                        push_array_columns(&mut columns, &array, len, tsv);
                    }
                }
                Some(Key::Opt) => push_opt_columns(&mut columns, &mut seen_opts, value),
                Some(Key::Ignored) | None => {}
            }
        }
    }
    columns
}

fn push_array_columns(
    columns: &mut Vec<(String, Column<'_>)>,
    array: &ArrayCol,
    len: usize,
    tsv: &Tsv,
) {
    let ids = tsv.ids.get(array.element).cloned().unwrap_or_default();
    for p in 0..len {
        let id = ids
            .get(p)
            .copied()
            .unwrap_or_else(|| u32::try_from(p + 1).unwrap_or(u32::MAX));
        columns.push((
            format!("{}[{id}]", array.prefix),
            Column::Array(array.index, p),
        ));
    }
}

fn push_opt_columns<'a>(
    columns: &mut Vec<(String, Column<'a>)>,
    names: &mut BTreeSet<&'a str>,
    value: &'a Node<'a>,
) {
    let Value::Array(opts) = &value.value else {
        return;
    };
    for opt in opts {
        if let Some(name) = opt_name(opt)
            && !name.contains(['\t', '\n', '\r'])
            && names.insert(name)
        {
            columns.push((format!("opt_{name}"), Column::Opt(name)));
        }
    }
}

struct RowEmit<'a> {
    table: &'static Table,
    table_index: usize,
    columns: &'a [(String, Column<'a>)],
    opt_count: usize,
}

fn emit_row(
    emit: &RowEmit<'_>,
    row_index: u32,
    row: &Node<'_>,
    here: &Path,
    tsv: &mut Tsv,
    row_buf: &mut String,
) {
    let (table, table_index, columns, opt_count) =
        (emit.table, emit.table_index, emit.columns, emit.opt_count);
    let Value::Object(members) = &row.value else {
        tsv.report(Code::JsonShape, here, Detail::Expected("an object"));
        return;
    };
    let mut by_key: BTreeMap<&str, &Node<'_>> = BTreeMap::new();
    for (key, value) in members {
        let k = key.as_ref();
        if by_key.contains_key(k) {
            let at = here.clone().key(key.to_string());
            tsv.report(Code::JsonShape, &at, Detail::Expected("a member only once"));
            continue;
        }
        by_key.insert(k, value);
        match row_key(table, k) {
            None => {
                let at = here.clone().key(key.to_string());
                tsv.report(Code::JsonUnknownKey, &at, Detail::None);
            }
            Some(Key::Array(_, element, _)) => validate_array_field(value, k, element, here, tsv),
            Some(Key::Opt) => validate_opt_field(value, here, tsv),
            _ => {}
        }
    }
    let mut by_opt: BTreeMap<&str, (u32, &Node<'_>)> = BTreeMap::new();
    if opt_count != 0
        && let Some(Value::Array(opts)) = by_key.get("opt").map(|n| &n.value)
    {
        for (n, opt) in opts.iter().enumerate() {
            if let Some(name) = opt_name(opt) {
                let idx = u32::try_from(n).unwrap_or(u32::MAX);
                by_opt.entry(name).or_insert((idx, opt));
            }
        }
    }

    row_buf.clear();
    row_buf.push_str(table.row);
    let mut sink = CellSink {
        opt_indices: Vec::with_capacity(opt_count),
        tsv,
        row_buf,
        here,
    };
    let mut first_bad: Option<usize> = None;
    let last_col = columns.len().saturating_sub(1);
    for (col_idx, (_, column)) in columns.iter().enumerate() {
        sink.row_buf.push('\t');
        let cell_start = sink.row_buf.len();
        write_row_cell(table, column, &by_key, &by_opt, &mut sink);
        let cell_slice = sink.row_buf.get(cell_start..).unwrap_or_default();
        let bad = cell_slice.contains(['\t', '\n'])
            || (col_idx == last_col && cell_slice.ends_with('\r'));
        if bad && first_bad.is_none() {
            first_bad = Some(col_idx + 1);
        }
    }
    let opt_indices = sink.opt_indices;
    if let Some(cell_idx) = first_bad {
        let at = tsv
            .row_path(table_index, row_index, &opt_indices, cell_idx)
            .unwrap_or_else(|| here.clone());
        tsv.report(Code::JsonText, &at, Detail::None);
        return;
    }
    tsv.push_row_line(table_index, row_index, opt_indices, row_buf);
}

struct CellSink<'a, 'b> {
    opt_indices: Vec<Option<u32>>,
    tsv: &'a mut Tsv,
    row_buf: &'a mut String,
    here: &'b Path,
}

fn write_row_cell(
    table: &Table,
    column: &Column<'_>,
    by_key: &BTreeMap<&str, &Node<'_>>,
    by_opt: &BTreeMap<&str, (u32, &Node<'_>)>,
    sink: &mut CellSink<'_, '_>,
) {
    match column {
        Column::Stable(json_key, col) => {
            if let Some(value) = by_key.get(json_key).copied() {
                write_cell(
                    value,
                    *col,
                    || sink.here.clone().key(*json_key),
                    sink.tsv,
                    sink.row_buf,
                );
            } else {
                sink.row_buf.push_str("null");
            }
        }
        Column::Array(a, p) => {
            let json = table.arrays.get(*a).map_or("", |(json, ..)| json);
            let item = match by_key.get(json).map(|n| &n.value) {
                Some(Value::Array(items)) => items.get(*p),
                _ => None,
            };
            if let Some(item) = item {
                let index = u32::try_from(*p).unwrap_or(u32::MAX);
                write_cell(
                    item,
                    Col::Number,
                    || sink.here.clone().key(json).index(index),
                    sink.tsv,
                    sink.row_buf,
                );
            } else {
                sink.row_buf.push_str("null");
            }
        }
        Column::Opt(name) => {
            if let Some((opt_idx, opt)) = by_opt.get(name).copied() {
                sink.opt_indices.push(Some(opt_idx));
                if let Some(val) = opt.get("value") {
                    write_cell(
                        val,
                        Col::Text,
                        || sink.here.clone().key("opt").index(opt_idx).key("value"),
                        sink.tsv,
                        sink.row_buf,
                    );
                } else {
                    sink.row_buf.push_str("null");
                }
            } else {
                sink.opt_indices.push(None);
                sink.row_buf.push_str("null");
            }
        }
    }
}

fn validate_array_field(
    value: &Node<'_>,
    key: &str,
    element: &'static str,
    row_at: &Path,
    tsv: &mut Tsv,
) {
    match &value.value {
        Value::Null => {}
        Value::Array(items) => {
            let expected = tsv.ids.get(element).map_or(0, Vec::len);
            if items.len() != expected {
                let at = row_at.clone().key(key.to_string());
                tsv.report(
                    Code::JsonShape,
                    &at,
                    Detail::Expected("one value per metadata element"),
                );
            }
        }
        _ => {
            let at = row_at.clone().key(key.to_string());
            tsv.report(Code::JsonShape, &at, Detail::Expected("an array"));
        }
    }
}

fn validate_opt_field(value: &Node<'_>, row_at: &Path, tsv: &mut Tsv) {
    match &value.value {
        Value::Null => {}
        Value::Array(opts) => {
            let opt_root = row_at.clone().key("opt");
            for (n, opt) in opts.iter().enumerate() {
                let opt_at = opt_root.clone().index(u32::try_from(n).unwrap_or(u32::MAX));
                if !matches!(opt.value, Value::Object(_)) {
                    tsv.report(Code::JsonShape, &opt_at, Detail::Expected("an opt entry"));
                    continue;
                }
                match opt.get("identifier").map(|v| &v.value) {
                    Some(Value::String(s)) => {
                        if s.contains(['\t', '\n', '\r']) {
                            tsv.report(
                                Code::JsonText,
                                &opt_at.clone().key("identifier"),
                                Detail::None,
                            );
                        }
                    }
                    _ => tsv.report(
                        Code::JsonShape,
                        &opt_at.clone().key("identifier"),
                        Detail::Expected("a string"),
                    ),
                }
            }
        }
        _ => {
            let at = row_at.clone().key("opt");
            tsv.report(Code::JsonShape, &at, Detail::Expected("an array"));
        }
    }
}

fn opt_name<'n>(opt: &'n Node<'_>) -> Option<&'n str> {
    match opt.get("identifier").map(|n| &n.value) {
        Some(Value::String(s)) => Some(s.as_ref()),
        _ => None,
    }
}
