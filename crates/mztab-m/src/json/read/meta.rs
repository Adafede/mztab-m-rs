//! JSON `metadata` object → `MTD` TSV lines.

use alloc::format;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use super::emit::{colunit_text, text};
use super::{Tsv, items, mtd, shape_error, unknown_keys};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::diag::location::Path;
use crate::json::schema::{Element, Item, METADATA, Shape};
use crate::json::value::{Node, Value};

pub(super) fn metadata_lines(node: &Node<'_>, at: &Path, tsv: &mut Tsv) {
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
            Item::Colunit(key) => colunit_lines(key, value, &here, tsv),
        }
    }
    unknown_keys(members, &known, at, tsv);
}

fn colunit_lines(key: &str, value: &Node<'_>, here: &Path, tsv: &mut Tsv) {
    for (_, unit, there) in items(value, here, tsv) {
        let text = match &unit.value {
            Value::String(s) => Some(s.to_string()),
            Value::Object(_) => colunit_text(unit, &there, tsv),
            _ => shape_error(tsv, &there, "a column unit mapping"),
        };
        if let Some(text) = text {
            mtd(tsv, key.to_string(), text, &there);
        }
    }
}

/// The index of an array item: its `id`, or its position + 1.
fn item_id(item: &Node<'_>, position: u32, at: &Path, tsv: &mut Tsv) -> u32 {
    if let Some(value) = item
        .get("id")
        .map(|n| &n.value)
        .filter(|v| !matches!(v, Value::Null))
    {
        if let Value::Number(n) = value
            && let Ok(id) = n.parse::<u32>()
            && id > 0
        {
            return id;
        }
        let at = at.clone().key("id");
        tsv.report(Code::JsonShape, &at, Detail::Expected("a positive integer"));
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
        element_props(element, object, &base, &here, &mut known, tsv);
        // `name` of an ms_run is JSCH's, with no TSV key (DECISIONS D-071).
        if element.json == "ms_run" {
            known.push("name");
        }
        unknown_keys(members, &known, &here, tsv);
    }
    tsv.ids.insert(element.tsv, ids);
}

fn element_props(
    element: &Element,
    object: &Node<'_>,
    base: &str,
    here: &Path,
    known: &mut Vec<&'static str>,
    tsv: &mut Tsv,
) {
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
