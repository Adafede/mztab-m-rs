//! Model item resolution, precomputed header slots, and single-line text formatting.

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::error::WriteError;
use super::line::{extra_line, header_line, metadata_line, row_line, unknown_line};
use super::render::RenderRow;
use super::sink::Out;
use crate::meta::key::MtdKey;
use crate::model::document::MzTab;
use crate::model::layout::{ExtraLine, LineContent};
use crate::model::meta_value::ValueRef;
use crate::model::table::{SmeColumn, SmfColumn, SmlColumn, Table};
use crate::tsv::table::{Slot, slots};
use crate::value::text::Raw;
use crate::vocab::section::TableSection;

/// Header slots computed once per table rather than once per row.
pub(super) struct TableSlots {
    sml: Option<Vec<Slot<SmlColumn>>>,
    smf: Option<Vec<Slot<SmfColumn>>>,
    sme: Option<Vec<Slot<SmeColumn>>>,
}

impl TableSlots {
    pub(super) fn of(mztab: &MzTab<'_>) -> Self {
        Self {
            sml: mztab.sml.as_ref().map(|t| slots(&t.header)),
            smf: mztab.smf.as_ref().map(|t| slots(&t.header)),
            sme: mztab.sme.as_ref().map(|t| slots(&t.header)),
        }
    }
}

/// What a layout line refers to, resolved against the model.
pub(super) enum Item<'r, 'a> {
    Metadata(MtdKey, ValueRef<'r, 'a>),
    Unknown(&'r (Raw<'a>, Raw<'a>)),
    Header(TableSection),
    Row(TableSection, usize),
    Extra(&'r ExtraLine<'a>),
}

/// The model's lines, consumed as the layout refers to them.
pub(super) struct Pending<'r, 'a> {
    mztab: &'r MzTab<'a>,
    pub(super) entries: Vec<(MtdKey, ValueRef<'r, 'a>)>,
    by_key: BTreeMap<MtdKey, VecDeque<usize>>,
    pub(super) used: Vec<bool>,
    pub(super) unknown: usize,
    pub(super) headers: [bool; 3],
    pub(super) rows: [usize; 3],
}

impl<'r, 'a> Pending<'r, 'a> {
    pub(super) fn new(mztab: &'r MzTab<'a>) -> Self {
        let entries = mztab.metadata.entries();
        let mut by_key = BTreeMap::<MtdKey, VecDeque<usize>>::new();
        for (i, (key, _)) in entries.iter().enumerate() {
            by_key.entry(*key).or_default().push_back(i);
        }
        let used = alloc::vec![false; entries.len()];
        Self {
            mztab,
            entries,
            by_key,
            used,
            unknown: 0,
            headers: [false; 3],
            rows: [0; 3],
        }
    }

    pub(super) fn row_count(&self, section: TableSection) -> usize {
        row_count(self.mztab, section)
    }

    /// The model item a layout line refers to, if it still exists.
    pub(super) fn take(&mut self, content: &'r LineContent<'a>) -> Option<Item<'r, 'a>> {
        match content {
            LineContent::Metadata(key) => {
                let at = self.by_key.get_mut(key)?.pop_front()?;
                *self.used.get_mut(at)? = true;
                let (key, value) = *self.entries.get(at)?;
                Some(Item::Metadata(key, value))
            }
            LineContent::UnknownMetadata => {
                let entry = self.mztab.metadata.unknown.get(self.unknown)?;
                self.unknown += 1;
                Some(Item::Unknown(entry))
            }
            LineContent::Header(section) => {
                let table = TableSection::from_section(*section)?;
                if !has_table(self.mztab, table) {
                    return None;
                }
                let seen = table.pick_mut(&mut self.headers);
                if *seen {
                    return None;
                }
                *seen = true;
                Some(Item::Header(table))
            }
            LineContent::Row(section) => {
                let table = TableSection::from_section(*section)?;
                let rows = self.row_count(table);
                let written = table.pick_mut(&mut self.rows);
                let n = *written;
                if n >= rows {
                    return None;
                }
                *written += 1;
                Some(Item::Row(table, n))
            }
            LineContent::Extra(line) => Some(Item::Extra(line)),
        }
    }
}

pub(super) fn row_count(mztab: &MzTab<'_>, section: TableSection) -> usize {
    match section {
        TableSection::Sml => mztab.sml.as_ref().map_or(0, |t| t.rows.len()),
        TableSection::Smf => mztab.smf.as_ref().map_or(0, |t| t.rows.len()),
        TableSection::Sme => mztab.sme.as_ref().map_or(0, |t| t.rows.len()),
    }
}

pub(super) const fn has_table(mztab: &MzTab<'_>, section: TableSection) -> bool {
    match section {
        TableSection::Sml => mztab.sml.is_some(),
        TableSection::Smf => mztab.smf.is_some(),
        TableSection::Sme => mztab.sme.is_some(),
    }
}

/// Writes one item's line (no line ending).
pub(super) fn item_line(
    out: &mut Out<'_>,
    mztab: &MzTab<'_>,
    table_slots: &TableSlots,
    item: &Item<'_, '_>,
) -> Result<(), WriteError> {
    match item {
        Item::Metadata(key, value) => metadata_line(out, *key, *value),
        Item::Unknown((key, value)) => unknown_line(out, key, value),
        Item::Header(section) => match section {
            TableSection::Sml => table_header(out, *section, mztab.sml.as_ref()),
            TableSection::Smf => table_header(out, *section, mztab.smf.as_ref()),
            TableSection::Sme => table_header(out, *section, mztab.sme.as_ref()),
        },
        Item::Row(section, n) => match section {
            TableSection::Sml => table_row(
                out,
                *section,
                mztab.sml.as_ref(),
                table_slots.sml.as_deref().unwrap_or_default(),
                *n,
            ),
            TableSection::Smf => table_row(
                out,
                *section,
                mztab.smf.as_ref(),
                table_slots.smf.as_deref().unwrap_or_default(),
                *n,
            ),
            TableSection::Sme => table_row(
                out,
                *section,
                mztab.sme.as_ref(),
                table_slots.sme.as_deref().unwrap_or_default(),
                *n,
            ),
        },
        Item::Extra(line) => extra_line(out, line),
    }
}

fn table_header<S: fmt::Display, R>(
    out: &mut Out<'_>,
    section: TableSection,
    table: Option<&Table<'_, S, R>>,
) -> Result<(), WriteError> {
    table.map_or(Ok(()), |t| header_line(out, section, &t.header))
}

fn table_row<'a, R: RenderRow<'a>>(
    out: &mut Out<'_>,
    section: TableSection,
    table: Option<&Table<'a, R::Column, R>>,
    row_slots: &[Slot<R::Column>],
    n: usize,
) -> Result<(), WriteError>
where
    R::Column: Copy,
{
    table
        .and_then(|t| t.rows.get(n))
        .map_or(Ok(()), |row| row_line(out, section, row_slots, row))
}

/// The canonical text of a table's header line.
pub(crate) fn header_text(mztab: &MzTab<'_>, section: TableSection) -> Option<String> {
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    let res = match section {
        TableSection::Sml => table_header(&mut out, section, mztab.sml.as_ref()),
        TableSection::Smf => table_header(&mut out, section, mztab.smf.as_ref()),
        TableSection::Sme => table_header(&mut out, section, mztab.sme.as_ref()),
    };
    res.ok()?;
    Some(text)
}

/// The canonical text of a row line.
pub(crate) fn row_text(mztab: &MzTab<'_>, section: TableSection, row: usize) -> Option<String> {
    let mut text = String::new();
    let mut out = Out::new(&mut text);
    let res = match section {
        TableSection::Sml => table_row(
            &mut out,
            section,
            mztab.sml.as_ref(),
            &mztab
                .sml
                .as_ref()
                .map_or_else(Vec::new, |t| slots(&t.header)),
            row,
        ),
        TableSection::Smf => table_row(
            &mut out,
            section,
            mztab.smf.as_ref(),
            &mztab
                .smf
                .as_ref()
                .map_or_else(Vec::new, |t| slots(&t.header)),
            row,
        ),
        TableSection::Sme => table_row(
            &mut out,
            section,
            mztab.sme.as_ref(),
            &mztab
                .sme
                .as_ref()
                .map_or_else(Vec::new, |t| slots(&t.header)),
            row,
        ),
    };
    res.ok()?;
    Some(text)
}
