//! `MTD` TSV lines → validated JSCH JSON `metadata` string and element id map.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::emit::JsonWriter;
use super::{split_key, unwritable};
use crate::json::schema::{Element, Item, METADATA, Shape};
use crate::meta::key::MtdKey;
use crate::param::Param;
use crate::write::error::WriteError;

/// Padding beyond this many `null`s makes a model unwritable.
const MAX_PADDING: u32 = 1024;

/// A key `prefix[index]rest` taken into JSON, and its line.
#[derive(Clone, Copy, Debug)]
struct Taken<'t> {
    line: u32,
    index: u32,
    rest: &'t str,
    value: &'t str,
}

/// Validates and serializes metadata into JSON, returning the JSON object
/// string and the collected element ids per metadata element.
pub(super) fn encode_metadata<'t>(
    lines: &[(u32, &'t str, &'t str)],
) -> Result<(String, BTreeMap<&'static str, Vec<u32>>), WriteError> {
    let mut meta = Metadata::new(lines);
    let mut out = String::new();
    // Writing into a `String` cannot fail.
    meta.write_object(&mut JsonWriter::new(&mut out))
        .unwrap_or_default();
    if let Some(line) = meta.unused() {
        return Err(unwritable(line, "metadata key has no JSON form"));
    }
    if let Some(line) = meta.refused {
        return Err(unwritable(
            line,
            "indices too far apart to place in a JSON array",
        ));
    }
    Ok((out, meta.ids))
}

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

    fn unused(&self) -> Option<u32> {
        self.lines
            .iter()
            .enumerate()
            .find(|(n, _)| !self.used.contains(n))
            .map(|(_, (line, ..))| *line)
    }

    fn take(&mut self, key: &str) -> Option<&'t str> {
        let (n, (_, _, value)) = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, (_, k, _))| *k == key)?;
        self.used.insert(n);
        Some(value)
    }

    fn take_indexed(&mut self, prefix: &str) -> Vec<Taken<'t>> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for (n, (line, key, value)) in self.lines.iter().enumerate() {
            // Only first occurrence, and only valid metadata keys.
            if let Some((name, index, rest)) = split_key(key)
                && name == prefix
                && key.parse::<MtdKey>().is_ok()
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

    fn write_object(&mut self, w: &mut JsonWriter<'_>) -> fmt::Result {
        w.raw("{\"prefix\":\"MTD\"")?;
        for item in METADATA {
            w.char(',')?;
            w.str(item.json_name())?;
            w.char(':')?;
            match item {
                Item::Field(_, tsv, shape) => match self.take(tsv) {
                    Some(v) => w.shaped(v, *shape)?,
                    None => w.raw("null")?,
                },
                Item::Element(element) => self.write_element(element, w)?,
                Item::Indexed(name) => {
                    let items = self.take_indexed(name);
                    let mut ids: Vec<u32> = items.iter().map(|t| t.index).collect();
                    ids.sort_unstable();
                    self.ids.insert(name, ids);
                    self.write_positioned(&items, false, w)?;
                }
                Item::Colunit(key) => self.write_colunits(key, w)?,
            }
        }
        w.char('}')
    }

    fn write_colunits(&mut self, key: &str, w: &mut JsonWriter<'_>) -> fmt::Result {
        let mut any = false;
        for (n, (_, k, v)) in self.lines.iter().enumerate() {
            if *k == key {
                self.used.insert(n);
                w.char(if any { ',' } else { '[' })?;
                any = true;
                w.colunit(v)?;
            }
        }
        if any { w.char(']') } else { w.raw("null") }
    }

    fn write_element(&mut self, element: &Element, w: &mut JsonWriter<'_>) -> fmt::Result {
        let mut by_id: BTreeMap<u32, Vec<Taken<'t>>> = BTreeMap::new();
        for key in self.take_indexed(element.tsv) {
            by_id.entry(key.index).or_default().push(key);
        }
        self.ids
            .insert(element.tsv, by_id.keys().copied().collect());
        if by_id.is_empty() {
            return w.raw("null");
        }
        w.char('[')?;
        for (idx, (id, mine)) in by_id.iter().enumerate() {
            if idx > 0 {
                w.char(',')?;
            }
            write!(w.out(), "{{\"id\":{id}")?;
            if let Some((json, shape)) = element.value {
                w.char(',')?;
                w.str(json)?;
                w.char(':')?;
                match mine.iter().find(|t| t.rest.is_empty()) {
                    Some(t) => w.shaped(t.value, shape)?,
                    None => w.raw("null")?,
                }
            }
            self.write_element_props(element, mine, w)?;
            w.char('}')?;
        }
        w.char(']')
    }

    fn write_element_props(
        &mut self,
        element: &Element,
        mine: &[Taken<'_>],
        w: &mut JsonWriter<'_>,
    ) -> fmt::Result {
        for (json, tsv, shape) in element.props {
            w.char(',')?;
            w.str(json)?;
            w.char(':')?;
            if matches!(shape, Shape::Indexed | Shape::Strings) {
                let items: Vec<Taken<'_>> = mine
                    .iter()
                    .filter_map(|t| {
                        let rest = t.rest.strip_prefix('-')?;
                        let (name, n, tail) = split_key(rest)?;
                        (name == *tsv && tail.is_empty()).then_some(Taken { index: n, ..*t })
                    })
                    .collect();
                self.write_positioned(&items, *shape == Shape::Strings, w)?;
            } else {
                match mine.iter().find(|t| t.rest.strip_prefix('-') == Some(*tsv)) {
                    Some(t) => w.shaped(t.value, *shape)?,
                    None => w.raw("null")?,
                }
            }
        }
        Ok(())
    }

    /// Indexed keys as an array: `Parameter` objects carrying their `id`;
    /// any other item at position `index - 1`, after `null`s where indices are missing.
    fn write_positioned(
        &mut self,
        items: &[Taken<'_>],
        strings: bool,
        w: &mut JsonWriter<'_>,
    ) -> fmt::Result {
        if items.is_empty() {
            return w.raw("null");
        }
        let mut sorted = items.to_vec();
        sorted.sort_by_key(|t| t.index);
        w.char('[')?;
        let mut len: u32 = 0;
        let mut padding = 0;
        for t in sorted {
            if !strings && let Ok(param) = Param::parse(t.value) {
                if len > 0 {
                    w.char(',')?;
                }
                w.write_param(&param, Some(t.index))?;
                len += 1;
                continue;
            }
            let missing = t.index.saturating_sub(1).saturating_sub(len);
            padding += missing;
            if padding > MAX_PADDING {
                self.refused.get_or_insert(t.line);
                break;
            }
            for _ in 0..missing {
                if len > 0 {
                    w.char(',')?;
                }
                w.raw("null")?;
                len += 1;
            }
            if len > 0 {
                w.char(',')?;
            }
            w.str(t.value)?;
            len += 1;
        }
        w.char(']')
    }
}
