//! Typing `MTD` metadata lines.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::diag::{Code, Detail};
use crate::meta::MtdKey;
use crate::model::MtdEntry;
use crate::tsv::{Cx, RawCell, mtd_value};
use crate::value::Raw;

/// The cells of `text` after the first (the prefix), numbered from 2.
pub(super) fn cells(text: &str) -> impl Iterator<Item = RawCell<'_>> {
    let mut offset = 0;
    let mut number = 0_u32;
    text.split('\t')
        .map(move |cell| {
            number = number.saturating_add(1);
            let raw = RawCell {
                text: cell,
                number,
                offset,
            };
            offset += cell.len() + 1;
            raw
        })
        .skip(1)
}

/// Types an `MTD` line.
pub(super) fn metadata<'a>(
    cx: &mut Cx<'_>,
    seen: &mut BTreeSet<MtdKey>,
    text: &'a str,
) -> (MtdEntry<'a>, bool) {
    let mut parts = cells(text);
    let Some(key) = parts.next() else {
        cx.report(Code::MtdCells, text.len(), 2, Detail::None);
        let entry = MtdEntry {
            key: Raw::new(""),
            value: Raw::new(""),
            trailing: Raw::new(""),
            typed: None,
        };
        return (entry, false);
    };
    let value = parts.next();
    let trailing = check_trailing_cells(cx, text, value, parts);
    let parsed = MtdKey::parse(key.text);
    let placed = check_key_uniqueness(cx, seen, key, parsed);
    let typed = parsed.map(|k| (k, type_mtd_value(cx, text, k, value)));
    let entry = MtdEntry {
        key: Raw::new(key.text),
        value: Raw::new(value.map_or("", |v| v.text)),
        trailing: Raw::new(trailing),
        typed,
    };
    (entry, placed)
}

fn check_trailing_cells<'a>(
    cx: &mut Cx<'_>,
    text: &'a str,
    value: Option<RawCell<'a>>,
    mut parts: impl Iterator<Item = RawCell<'a>>,
) -> &'a str {
    let trailing_at = value.map_or(text.len(), |v| v.offset + v.text.len());
    let trailing = text.get(trailing_at..).unwrap_or_default();
    if let Some(extra) = parts.find(|cell| !cell.text.is_empty()) {
        cx.report(Code::MtdCells, extra.offset, extra.number, Detail::None);
    } else if !trailing.is_empty() {
        cx.report(
            Code::TrailingEmptyCells,
            trailing_at,
            value.map_or(3, |v| v.number + 1),
            Detail::None,
        );
    }
    if value.is_none() {
        cx.report(Code::MtdCells, text.len(), 3, Detail::None);
    }
    trailing
}

fn check_key_uniqueness(
    cx: &mut Cx<'_>,
    seen: &mut BTreeSet<MtdKey>,
    key: RawCell<'_>,
    parsed: Option<MtdKey>,
) -> bool {
    match parsed {
        None => {
            cx.report(Code::UnknownMtdKey, key.offset, key.number, Detail::None);
            false
        }
        Some(k) if !k.repeats() && !seen.insert(k) => {
            cx.report(Code::DuplicateMtdKey, key.offset, key.number, Detail::None);
            false
        }
        Some(_) => true,
    }
}

fn type_mtd_value<'a>(
    cx: &mut Cx<'_>,
    text: &'a str,
    key: MtdKey,
    value: Option<RawCell<'a>>,
) -> crate::model::MtdValue<'a> {
    if let Some(raw) = value {
        return mtd_value(cx, key.kind(), raw);
    }
    // Already reported as a missing cell; typing the empty text would report it
    // a second time.
    let raw = RawCell {
        text: "",
        number: 3,
        offset: text.len(),
    };
    let mut quiet = Vec::new();
    let mut silent = Cx {
        line: cx.line,
        number: cx.number,
        findings: &mut quiet,
    };
    mtd_value(&mut silent, key.kind(), raw)
}
