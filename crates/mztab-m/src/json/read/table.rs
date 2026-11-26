//! JSON table arrays → TSV headers and rows.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::emit::cell_text;
use super::{Tsv, items};
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
enum Column {
    Stable(&'static str, Col),
    /// Array index, position in the array.
    Array(usize, usize),
    Opt(String),
}

pub(super) fn table_lines(table: &Table, node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
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
        let mut header = Vec::from([(table.header.to_string(), at.clone())]);
        header.extend(names.into_iter().map(|name| (name, at.clone())));
        tsv.line(&header);
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
    let mut header: Vec<(String, Path)> = Vec::from([(table.header.to_string(), at.clone())]);
    header.extend(columns.iter().map(|(name, _)| (name.clone(), at.clone())));
    tsv.line(&header);
    for (_, row, here) in &rows {
        emit_row(table, &columns, row, here, tsv);
    }
}

struct ArrayCol {
    index: usize,
    element: &'static str,
    prefix: &'static str,
}

fn infer_columns(
    table: &Table,
    rows: &[(u32, &Node<'_>, Path)],
    tsv: &Tsv,
) -> Vec<(String, Column)> {
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

    let mut columns: Vec<(String, Column)> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_opts = BTreeSet::new();
    for (_, row, _) in rows {
        let Value::Object(members) = &row.value else {
            continue;
        };
        for (key, value) in members {
            match row_key(table, key) {
                Some(Key::Stable(tsv_name, json_key, col)) => {
                    if seen.insert(key.to_string()) {
                        columns.push((tsv_name.to_string(), Column::Stable(json_key, col)));
                    }
                }
                Some(Key::Array(index, element, prefix)) => {
                    if matches!(value.value, Value::Array(_)) && seen.insert(key.to_string()) {
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
    columns: &mut Vec<(String, Column)>,
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

fn push_opt_columns(
    columns: &mut Vec<(String, Column)>,
    names: &mut BTreeSet<String>,
    value: &Node<'_>,
) {
    let Value::Array(opts) = &value.value else {
        return;
    };
    for opt in opts {
        if let Some(name) = opt_name(opt)
            && !name.contains(['\t', '\n', '\r'])
            && names.insert(name.clone())
        {
            columns.push((format!("opt_{name}"), Column::Opt(name)));
        }
    }
}

fn emit_row(
    table: &Table,
    columns: &[(String, Column)],
    row: &Node<'_>,
    here: &Path,
    tsv: &mut Tsv,
) {
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
    let mut cells = Vec::from([(table.row.to_string(), here.clone())]);
    for (_, column) in columns {
        cells.push(row_cell(table, column, &by_key, here, tsv));
    }
    tsv.line(&cells);
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

fn opt_name(opt: &Node<'_>) -> Option<String> {
    match opt.get("identifier").map(|n| &n.value) {
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn row_cell(
    table: &Table,
    column: &Column,
    by_key: &BTreeMap<&str, &Node<'_>>,
    at: &Path,
    tsv: &mut Tsv,
) -> (String, Path) {
    let null = || "null".to_string();
    match column {
        Column::Stable(json_key, col) => {
            let value = by_key.get(json_key).copied();
            let here = at.clone().key(*json_key);
            let text = value
                .and_then(|value| cell_text(value, *col, &here, tsv))
                .unwrap_or_else(null);
            (text, here)
        }
        Column::Array(a, p) => {
            let json = table.arrays.get(*a).map_or("", |(json, ..)| json);
            let index = u32::try_from(*p).unwrap_or(u32::MAX);
            let here = at.clone().key(json).index(index);
            let item = match by_key.get(json).map(|n| &n.value) {
                Some(Value::Array(items)) => items.get(*p),
                _ => None,
            };
            let text = item
                .and_then(|value| cell_text(value, Col::Number, &here, tsv))
                .unwrap_or_else(null);
            (text, here)
        }
        Column::Opt(name) => {
            let opts = match by_key.get("opt").map(|n| &n.value) {
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
