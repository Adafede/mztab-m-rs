//! Line-level formatting for metadata, headers, table rows, and extra lines.

use core::fmt;

use super::error::WriteError;
use super::render::{Render, RenderRow, render_value};
use super::sink::Out;
use crate::meta::MtdKey;
use crate::model::{Column, ExtraLine, Header, ValueRef};
use crate::tsv::Slot;
use crate::value::{NULL, Raw};
use crate::vocab::{Section, TableSection};

/// Writes `MTD\tkey\tvalue` (no line ending).
pub(crate) fn metadata_line(
    out: &mut Out<'_>,
    key: MtdKey,
    value: ValueRef<'_, '_>,
) -> Result<(), WriteError> {
    out.str("MTD\t")?;
    out.display(&key)?;
    out.str("\t")?;
    render_value(value, out)
}

/// Writes an unknown metadata line (no line ending).
pub(crate) fn unknown_line(
    out: &mut Out<'_>,
    key: &Raw<'_>,
    value: &Raw<'_>,
) -> Result<(), WriteError> {
    out.str("MTD\t")?;
    out.checked(key.as_str())?;
    out.str("\t")?;
    out.checked(value.as_str())
}

/// Writes a header line (no line ending).
pub(super) fn header_line<S: fmt::Display>(
    out: &mut Out<'_>,
    section: TableSection,
    header: &Header<'_, S>,
) -> Result<(), WriteError> {
    out.str(section.section().header_prefix().unwrap_or_default())?;
    for column in &header.columns {
        out.str("\t")?;
        match column {
            Column::Stable(s) => out.display(s)?,
            Column::Abundance(a) => out.display(a)?,
            Column::IdConfidence(i) => out.display(i)?,
            Column::Opt(o) => out.display(o)?,
            Column::Unknown(raw) => out.checked(raw.as_str())?,
        }
    }
    Ok(())
}

/// Writes a row line (no line ending). Cells missing from a short row are
/// written as `null`.
pub(crate) fn row_line<'a, R: RenderRow<'a>>(
    out: &mut Out<'_>,
    section: Section,
    slots: &[Slot<R::Column>],
    row: &R,
) -> Result<(), WriteError> {
    out.str(section.row_prefix())?;
    let (mut number, mut opt, mut unknown) = (0, 0, 0);
    for slot in slots {
        out.str("\t")?;
        match slot {
            Slot::Stable(column) => row.render_stable(*column, out)?,
            Slot::Number => {
                render_nth(out, row.numbers_ref(), number)?;
                number += 1;
            }
            Slot::Opt => {
                render_nth(out, row.opt_ref(), opt)?;
                opt += 1;
            }
            Slot::Unknown => {
                render_nth(out, row.unknown_ref(), unknown)?;
                unknown += 1;
            }
        }
    }
    for extra in row.extra_ref() {
        out.str("\t")?;
        out.checked(extra.as_str())?;
    }
    Ok(())
}

fn render_nth<T: Render>(out: &mut Out<'_>, cells: &[T], n: usize) -> Result<(), WriteError> {
    match cells.get(n) {
        Some(cell) => cell.render(out),
        None => out.str(NULL),
    }
}

/// Writes an extra line (no line ending), checking it reads back as one.
pub(super) fn extra_line(out: &mut Out<'_>, line: &ExtraLine<'_>) -> Result<(), WriteError> {
    let (text, ok) = match line {
        ExtraLine::Blank(raw) => (
            raw.as_str(),
            raw.as_str().bytes().all(|b| b == b' ' || b == b'\t'),
        ),
        ExtraLine::Comment(raw) => (raw.as_str(), raw.as_str().split('\t').next() == Some("COM")),
        ExtraLine::Unrecognized(raw) => (raw.as_str(), true),
    };
    if !ok {
        return Err(out.fail("a blank line with text, or a comment without `COM`"));
    }
    if text.contains('\n') {
        return Err(out.fail("raw text with a tab or line break"));
    }
    out.str(text)
}
