//! Source-layout replay and canonical TSV document writing.

use core::fmt;

use super::error::WriteError;
use super::pending::{Item, Pending, TableSlots, has_table, item_line, row_count};
use super::sink::{Fingerprint, Out};
use crate::model::document::MzTab;
use crate::model::layout::{ExtraLine, LineContent, LineEnding, SourceLine};
use crate::vocab::section::TableSection;

/// The writer: the output plus whether the last line is still open (the
/// source's last line had no line break and more lines follow).
struct Writer<'o, 'r, 'a> {
    out: Out<'o>,
    mztab: &'r MzTab<'a>,
    slots: TableSlots,
    open: bool,
    wrote_any: bool,
}

impl<'r, 'a> Writer<'_, 'r, 'a> {
    /// Writes a canonical line (`\n`-terminated).
    fn canonical(&mut self, item: &Item<'r, 'a>) -> Result<(), WriteError> {
        self.close()?;
        item_line(&mut self.out, self.mztab, &self.slots, item)?;
        self.wrote_any = true;
        self.out.ending(LineEnding::Lf)
    }

    /// Writes a blank separator line.
    fn blank(&mut self) -> Result<(), WriteError> {
        self.close()?;
        self.wrote_any = true;
        self.out.ending(LineEnding::Lf)
    }

    /// Ends a source line that had no line break, before writing more.
    fn close(&mut self) -> Result<(), WriteError> {
        if core::mem::take(&mut self.open) {
            self.out.ending(LineEnding::Lf)?;
        }
        Ok(())
    }

    /// Writes a layout line: verbatim if its content is unchanged.
    fn source_line(
        &mut self,
        line: &SourceLine<'_>,
        item: &Item<'r, 'a>,
    ) -> Result<(), WriteError> {
        self.close()?;
        let verbatim = line.verbatim.as_ref().filter(|v| {
            let mut fingerprint = Fingerprint::new();
            let mut probe = Out::new(&mut fingerprint);
            item_line(&mut probe, self.mztab, &self.slots, item).is_ok()
                && fingerprint.finish() == v.fingerprint
        });
        match verbatim {
            Some(v) => self.out.str(v.text.as_str())?,
            None => item_line(&mut self.out, self.mztab, &self.slots, item)?,
        }
        self.wrote_any = true;
        self.out.ending(line.ending)?;
        self.open = line.ending == LineEnding::None;
        Ok(())
    }

    /// Writes metadata entries and unknown lines not yet written.
    fn rest_of_metadata(&mut self, pending: &mut Pending<'r, 'a>) -> Result<(), WriteError> {
        for at in 0..pending.entries.len() {
            if pending.used.get(at) == Some(&false)
                && let Some(&(key, value)) = pending.entries.get(at)
            {
                self.canonical(&Item::Metadata(key, value))?;
            }
        }
        pending.used.fill(true);
        let unknown = &self.mztab.metadata.unknown;
        let rest = unknown.get(pending.unknown..).unwrap_or_default();
        pending.unknown = unknown.len();
        rest.iter()
            .try_for_each(|entry| self.canonical(&Item::Unknown(entry)))
    }

    /// Writes the rows of `section` not yet written.
    fn rest_of_rows(
        &mut self,
        pending: &mut Pending<'r, 'a>,
        section: TableSection,
    ) -> Result<(), WriteError> {
        let total = pending.row_count(section);
        let first = core::mem::replace(section.pick_mut(&mut pending.rows), total);
        (first..total).try_for_each(|n| self.canonical(&Item::Row(section, n)))
    }
}

/// Writes `mztab` preserving its [`SourceLayout`](crate::SourceLayout).
pub(super) fn write_layout(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    if mztab.layout.lines.is_empty() {
        if mztab.layout.bom {
            out.write_str("\u{feff}").map_err(|_| WriteError::Fmt)?;
        }
        return write_canonical_opts(mztab, out, false);
    }
    let mut pending = Pending::new(mztab);
    let mut w = Writer {
        out: Out::new(out),
        mztab,
        slots: TableSlots::of(mztab),
        open: false,
        wrote_any: false,
    };
    if mztab.layout.bom {
        w.out.str("\u{feff}")?;
    }
    let lines = &mztab.layout.lines;
    // Where content added after reading goes: after the last line of its
    // kind, or (metadata) before everything if no metadata line exists.
    let last_metadata = lines.iter().rposition(|l| {
        matches!(
            l.content,
            LineContent::Metadata(_) | LineContent::UnknownMetadata
        )
    });
    let last_table = [
        last_table_line(lines, TableSection::Sml),
        last_table_line(lines, TableSection::Smf),
        last_table_line(lines, TableSection::Sme),
    ];
    if last_metadata.is_none() {
        w.rest_of_metadata(&mut pending)?;
    }
    for (i, line) in lines.iter().enumerate() {
        if let Some(item) = pending.take(&line.content) {
            w.source_line(line, &item)?;
        }
        if last_metadata == Some(i) {
            w.rest_of_metadata(&mut pending)?;
        }
        for section in TableSection::ALL {
            let t = section.index();
            if last_table.get(t) == Some(&Some(i)) && pending.headers.get(t) == Some(&true) {
                w.rest_of_rows(&mut pending, section)?;
            }
        }
    }
    append_unplaced_tables(&mut w, &mut pending)
}

fn last_table_line(lines: &[SourceLine<'_>], section: TableSection) -> Option<usize> {
    lines.iter().rposition(
        |l| matches!(l.content, LineContent::Header(s) | LineContent::Row(s) if s == section.section()),
    )
}

fn append_unplaced_tables<'r, 'a>(
    w: &mut Writer<'_, 'r, 'a>,
    pending: &mut Pending<'r, 'a>,
) -> Result<(), WriteError> {
    for section in TableSection::ALL {
        let t = section.index();
        if has_table(w.mztab, section) && pending.headers.get(t) == Some(&false) {
            if w.wrote_any {
                w.blank()?;
            }
            w.canonical(&Item::Header(section))?;
            w.rest_of_rows(pending, section)?;
        }
    }
    Ok(())
}

/// Writes `mztab` in canonical TSV form without inspecting layout history
/// and without cloning `mztab`.
pub(super) fn write_canonical_opts(
    mztab: &MzTab<'_>,
    out: &mut dyn fmt::Write,
    keep_comments: bool,
) -> Result<(), WriteError> {
    let entries = mztab.metadata.entries();
    let mut w = Writer {
        out: Out::new(out),
        mztab,
        slots: TableSlots::of(mztab),
        open: false,
        wrote_any: false,
    };
    let mut anything = !entries.is_empty() || !mztab.metadata.unknown.is_empty();
    for (key, value) in entries {
        w.canonical(&Item::Metadata(key, value))?;
    }
    for entry in &mztab.metadata.unknown {
        w.canonical(&Item::Unknown(entry))?;
    }
    if keep_comments {
        for line in &mztab.layout.lines {
            if let LineContent::Extra(comment @ ExtraLine::Comment(_)) = &line.content {
                w.canonical(&Item::Extra(comment))?;
                anything = true;
            }
        }
    }
    for section in TableSection::ALL {
        if has_table(mztab, section) {
            if anything {
                w.blank()?;
            }
            w.canonical(&Item::Header(section))?;
            for n in 0..row_count(mztab, section) {
                w.canonical(&Item::Row(section, n))?;
            }
            anything = true;
        }
    }
    Ok(())
}
