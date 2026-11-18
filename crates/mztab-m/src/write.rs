//! Writing TSV: [`write()`] and [`to_string`].
//!
//! A model built in code is written in canonical form: metadata in
//! field-reference order (grouped by element), a blank line between
//! sections, `\n` line endings, parameters as `[label, accession, name,
//! value]`, and table columns in header order.
//!
//! A model read from a file carries a [`SourceLayout`](crate::SourceLayout): the order of the
//! source lines, their endings, blank and comment lines, and the original
//! text of every line whose canonical form differs. The writer reproduces
//! that layout, so `write(read(f)) == f` byte for byte. A line whose model
//! content was changed after reading no longer matches its recorded
//! fingerprint and is written in canonical form instead; content added
//! after reading is written canonically next to its section.
//!
//! ```
//! let input = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tEX-1\n";
//! let parsed = mztab_m::parse(input);
//! assert_eq!(mztab_m::to_string(&parsed.mztab)?, input);
//!
//! let mut mztab = parsed.mztab;
//! mztab.layout = mztab_m::SourceLayout::default();
//! assert_eq!(mztab_m::to_string(&mztab)?, input);
//! # Ok::<(), mztab_m::WriteError>(())
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::id::{AssayId, InstrumentId, MsRunId, ProtocolId, SampleId, StudyVariableId};
use crate::meta::{MtdKey, ValueRef};
use crate::model::{
    ColUnit, Column, ExtraLine, Header, LineContent, LineEnding, MzTab, SmeId, SmfId, SmlId,
    SourceLine, SpectraRef, Table,
};
use crate::param::Param;
use crate::table::{Slot, TableRow};
use crate::typing::Splitter;
use crate::value::{Cell, Decimal, NULL, Raw, Text};
use crate::vocab::{AmbiguityCode, Datatype, Profile, PublicationItem, Section, Version};

/// Why a model could not be written.
///
/// # Examples
///
/// ```
/// use mztab_m::{Cell, MzTab, Raw, WriteError};
///
/// let mut mztab = MzTab::default();
/// mztab.metadata.title = Some(Cell::Invalid(Raw::new("two\tcells")));
/// assert_eq!(
///     mztab_m::to_string(&mztab),
///     Err(WriteError::Unwritable { line: 1, reason: "raw text with a tab or line break" })
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WriteError {
    /// The output (`fmt::Write`) failed.
    Fmt,
    /// A value cannot be written so that it reads back the same. Only
    /// models built or edited in code can hit this: raw text (in
    /// [`Cell::Invalid`], unknown columns or unknown metadata) that contains
    /// a tab or line break, a list item containing `|`, an empty list, or a
    /// unit column name containing `=`.
    Unwritable {
        /// 1-based output line being written.
        line: u32,
        /// What is wrong.
        reason: &'static str,
    },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fmt => f.write_str("the output failed"),
            Self::Unwritable { line, reason } => {
                write!(f, "cannot write output line {line}: {reason}")
            }
        }
    }
}

impl core::error::Error for WriteError {}

impl From<fmt::Error> for WriteError {
    fn from(_: fmt::Error) -> Self {
        Self::Fmt
    }
}

/// FNV-1a over everything written: the fingerprint of a line's canonical
/// form, recorded by the reader and checked by the writer.
#[derive(Debug)]
pub(crate) struct Fingerprint(u64);

impl Fingerprint {
    pub(crate) const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) const fn finish(&self) -> u64 {
        self.0
    }
}

impl fmt::Write for Fingerprint {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        Ok(())
    }
}

/// Compares what is written with a source line, fingerprinting it as well.
#[derive(Debug)]
pub(crate) struct Compare<'s> {
    rest: Option<&'s str>,
    pub(crate) fingerprint: Fingerprint,
}

impl<'s> Compare<'s> {
    pub(crate) const fn new(source: &'s str) -> Self {
        Self {
            rest: Some(source),
            fingerprint: Fingerprint::new(),
        }
    }

    /// Whether everything written so far equals the whole source line.
    pub(crate) fn matched(&self) -> bool {
        self.rest.is_some_and(str::is_empty)
    }
}

impl fmt::Write for Compare<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.rest = self.rest.and_then(|rest| rest.strip_prefix(s));
        self.fingerprint.write_str(s)
    }
}

/// The output, with the line being written (for error locations) and,
/// inside a list, the reader's list splitter.
pub(crate) struct Out<'o> {
    sink: &'o mut dyn fmt::Write,
    line: u32,
    /// Whether the text written on this line so far ends with `\r`.
    after_cr: bool,
    /// Inside a list: how the reader will split what is written.
    list: Option<Splitter>,
    /// An error raised while formatting through `fmt::Write`.
    failed: Option<WriteError>,
}

impl<'o> Out<'o> {
    pub(crate) fn new(out: &'o mut dyn fmt::Write) -> Self {
        Self {
            sink: out,
            line: 1,
            after_cr: false,
            list: None,
            failed: None,
        }
    }

    /// Writes text. Inside a list, a `|` that the reader would take as a
    /// separator is refused.
    pub(crate) fn str(&mut self, text: &str) -> Result<(), WriteError> {
        if let Some(splitter) = &mut self.list
            && text.bytes().any(|b| splitter.step(b))
        {
            return Err(self.fail("a list item containing `|`"));
        }
        if let Some(last) = text.as_bytes().last() {
            self.after_cr = *last == b'\r';
        }
        Ok(self.sink.write_str(text)?)
    }

    /// Writes a value's `Display` form, through [`Out::str`].
    fn display(&mut self, value: &dyn fmt::Display) -> Result<(), WriteError> {
        if fmt::write(self, format_args!("{value}")).is_ok() {
            return Ok(());
        }
        Err(self.failed.take().unwrap_or(WriteError::Fmt))
    }

    /// Writes the `|` between two list items.
    fn separator(&mut self) -> Result<(), WriteError> {
        if self.list.is_some_and(|splitter| !splitter.at_rest()) {
            return Err(self.fail("a list item that leaves `[` or `\"` open"));
        }
        self.after_cr = false;
        Ok(self.sink.write_str("|")?)
    }

    const fn fail(&self, reason: &'static str) -> WriteError {
        WriteError::Unwritable {
            line: self.line,
            reason,
        }
    }

    /// Raw text that must stay in one cell. A `\r` is allowed inside a
    /// line, not at its end (see [`Out::ending`]).
    fn checked(&mut self, text: &str) -> Result<(), WriteError> {
        if text.contains(['\t', '\n']) {
            return Err(self.fail("raw text with a tab or line break"));
        }
        self.str(text)
    }

    fn ending(&mut self, ending: LineEnding) -> Result<(), WriteError> {
        if self.after_cr && ending == LineEnding::Lf {
            return Err(self.fail("a line ending in `\\r`, which reads back as a CRLF ending"));
        }
        let text = match ending {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::None => "",
        };
        self.line = self.line.saturating_add(1);
        self.str(text)
    }
}

impl fmt::Write for Out<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.str(s).map_err(|error| {
            self.failed = Some(error);
            fmt::Error
        })
    }
}

/// A value that can be written as (part of) a cell.
pub(crate) trait Render {
    /// Whether a list of these splits like a parameter list.
    const BRACKETS: bool = false;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError>;
}

macro_rules! render_display {
    ($($ty:ty),+ $(,)?) => {$(
        impl Render for $ty {
            fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
                out.display(self)
            }
        }
    )+};
}

render_display! {
    Decimal<'_>, Version, Profile, Datatype, AmbiguityCode, u32, i32,
    SmlId, SmfId, SmeId, AssayId, InstrumentId, MsRunId, ProtocolId, SampleId, StudyVariableId,
}

impl Render for Param<'_> {
    const BRACKETS: bool = true;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.display(self)
    }
}

impl Render for Text<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.str(self.as_str())
    }
}

impl Render for PublicationItem<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.str(self.kind.as_str())?;
        out.str(":")?;
        out.str(self.accession.as_str())
    }
}

impl Render for SpectraRef<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.display(&self.ms_run)?;
        out.str(":")?;
        out.str(self.reference.as_str())
    }
}

impl Render for ColUnit<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        if self.column.as_str().contains('=') {
            return Err(out.fail("a unit column name containing `=`"));
        }
        self.column.render(out)?;
        out.str("=")?;
        out.display(&self.param)
    }
}

impl<T: Render> Render for Cell<'_, T> {
    const BRACKETS: bool = T::BRACKETS;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        match self {
            Self::Value(value) => value.render(out),
            Self::Null => out.str(NULL),
            Self::Invalid(raw) if raw.as_str() == NULL => {
                Err(out.fail("invalid text `null`, which reads back as null"))
            }
            Self::Invalid(raw) => raw.render(out),
        }
    }
}

impl Render for Raw<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.checked(self.as_str())
    }
}

/// Refuses lists that cannot be written: empty, or a single `null` (which
/// reads back as a null cell, not a list). Not generic, so covered once.
const fn list_shape(out: &Out<'_>, len: usize, first_is_null: bool) -> Result<(), WriteError> {
    match (len, first_is_null) {
        (0, _) => Err(out.fail("an empty list")),
        (1, true) => Err(out.fail("a list of one `null`, which reads back as a null cell")),
        _ => Ok(()),
    }
}

impl<T: Render> Render for Vec<Cell<'_, T>> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        list_shape(out, self.len(), self.first().is_some_and(Cell::is_null))?;
        let outer = out.list.replace(Splitter::new(T::BRACKETS));
        let written = self.iter().enumerate().try_for_each(|(i, item)| {
            if i > 0 {
                out.separator()?;
            }
            item.render(out)
        });
        out.list = outer;
        written
    }
}

/// Writes a metadata value.
fn render_value(value: ValueRef<'_, '_>, out: &mut Out<'_>) -> Result<(), WriteError> {
    match value {
        ValueRef::Version(c) => c.render(out),
        ValueRef::Profile(c) => c.render(out),
        ValueRef::Datatype(c) => c.render(out),
        ValueRef::Text(c) => c.render(out),
        ValueRef::Param(c) => c.render(out),
        ValueRef::ParamList(c) => c.render(out),
        ValueRef::Publication(c) => c.render(out),
        ValueRef::InstrumentRef(c) => c.render(out),
        ValueRef::SampleRef(c) => c.render(out),
        ValueRef::MsRunRefs(c) => c.render(out),
        ValueRef::ProtocolRefs(c) => c.render(out),
        ValueRef::AssayRefs(c) => c.render(out),
        ValueRef::StudyVariableRefs(c) => c.render(out),
        ValueRef::ColUnit(c) => c.render(out),
    }
}

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
pub(crate) fn header_line<S: fmt::Display>(
    out: &mut Out<'_>,
    section: Section,
    header: &Header<'_, S>,
) -> Result<(), WriteError> {
    out.str(section.header_prefix().unwrap_or_default())?;
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

/// The reader's slots for a header: what each column holds.
pub(crate) fn slots<S: Copy>(header: &Header<'_, S>) -> Vec<Slot<S>> {
    header
        .columns
        .iter()
        .map(|column| match column {
            Column::Stable(s) => Slot::Stable(*s),
            Column::Abundance(_) | Column::IdConfidence(_) => Slot::Number,
            Column::Opt(_) => Slot::Opt,
            Column::Unknown(_) => Slot::Unknown,
        })
        .collect()
}

/// Writes a row line (no line ending). Cells missing from a short row are
/// written as `null`.
pub(crate) fn row_line<'a, R: TableRow<'a>>(
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
fn extra_line(out: &mut Out<'_>, line: &ExtraLine<'_>) -> Result<(), WriteError> {
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

/// What a layout line refers to, resolved against the model.
enum Item<'r, 'a> {
    Metadata(MtdKey, ValueRef<'r, 'a>),
    Unknown(&'r (Raw<'a>, Raw<'a>)),
    Header(Section),
    Row(Section, usize),
    Extra(&'r ExtraLine<'a>),
}

/// The model's lines, consumed as the layout refers to them.
struct Pending<'r, 'a> {
    mztab: &'r MzTab<'a>,
    entries: Vec<(MtdKey, ValueRef<'r, 'a>)>,
    by_key: BTreeMap<MtdKey, VecDeque<usize>>,
    used: Vec<bool>,
    unknown: usize,
    headers: [bool; 3],
    rows: [usize; 3],
}

const fn table_index(section: Section) -> usize {
    match section {
        Section::Mtd | Section::Sml => 0,
        Section::Smf => 1,
        Section::Sme => 2,
    }
}

impl<'r, 'a> Pending<'r, 'a> {
    fn new(mztab: &'r MzTab<'a>) -> Self {
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

    fn row_count(&self, section: Section) -> usize {
        match section {
            Section::Mtd | Section::Sml => self.mztab.sml.as_ref().map_or(0, |t| t.rows.len()),
            Section::Smf => self.mztab.smf.as_ref().map_or(0, |t| t.rows.len()),
            Section::Sme => self.mztab.sme.as_ref().map_or(0, |t| t.rows.len()),
        }
    }

    const fn has_table(&self, section: Section) -> bool {
        match section {
            Section::Mtd | Section::Sml => self.mztab.sml.is_some(),
            Section::Smf => self.mztab.smf.is_some(),
            Section::Sme => self.mztab.sme.is_some(),
        }
    }

    /// The model item a layout line refers to, if it still exists.
    fn take(&mut self, content: &'r LineContent<'a>) -> Option<Item<'r, 'a>> {
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
                if !self.has_table(*section) {
                    return None;
                }
                let seen = self.headers.get_mut(table_index(*section))?;
                if *seen {
                    return None;
                }
                *seen = true;
                Some(Item::Header(*section))
            }
            LineContent::Row(section) => {
                let n = *self.rows.get(table_index(*section))?;
                if n >= self.row_count(*section) {
                    return None;
                }
                *self.rows.get_mut(table_index(*section))? += 1;
                Some(Item::Row(*section, n))
            }
            LineContent::Extra(line) => Some(Item::Extra(line)),
        }
    }
}

/// Writes one item's line (no line ending).
fn item_line(out: &mut Out<'_>, mztab: &MzTab<'_>, item: &Item<'_, '_>) -> Result<(), WriteError> {
    match item {
        Item::Metadata(key, value) => metadata_line(out, *key, *value),
        Item::Unknown((key, value)) => unknown_line(out, key, value),
        Item::Header(section) => match section {
            Section::Mtd | Section::Sml => table_header(out, *section, mztab.sml.as_ref()),
            Section::Smf => table_header(out, *section, mztab.smf.as_ref()),
            Section::Sme => table_header(out, *section, mztab.sme.as_ref()),
        },
        Item::Row(section, n) => match section {
            Section::Mtd | Section::Sml => table_row(out, *section, mztab.sml.as_ref(), *n),
            Section::Smf => table_row(out, *section, mztab.smf.as_ref(), *n),
            Section::Sme => table_row(out, *section, mztab.sme.as_ref(), *n),
        },
        Item::Extra(line) => extra_line(out, line),
    }
}

fn table_header<S: fmt::Display, R>(
    out: &mut Out<'_>,
    section: Section,
    table: Option<&Table<'_, S, R>>,
) -> Result<(), WriteError> {
    table.map_or(Ok(()), |t| header_line(out, section, &t.header))
}

fn table_row<'a, R: TableRow<'a>>(
    out: &mut Out<'_>,
    section: Section,
    table: Option<&Table<'a, R::Column, R>>,
    n: usize,
) -> Result<(), WriteError>
where
    R::Column: Copy,
{
    table
        .and_then(|t| t.rows.get(n).map(|row| (t, row)))
        .map_or(Ok(()), |(t, row)| {
            row_line(out, section, &slots(&t.header), row)
        })
}

/// The writer: the output plus whether the last line is still open (the
/// source's last line had no line break and more lines follow).
struct Writer<'o, 'r, 'a> {
    out: Out<'o>,
    mztab: &'r MzTab<'a>,
    open: bool,
}

impl<'r, 'a> Writer<'_, 'r, 'a> {
    /// Writes a canonical line (`\n`-terminated).
    fn canonical(&mut self, item: &Item<'r, 'a>) -> Result<(), WriteError> {
        self.close()?;
        item_line(&mut self.out, self.mztab, item)?;
        self.out.ending(LineEnding::Lf)
    }

    /// Writes a blank separator line.
    fn blank(&mut self) -> Result<(), WriteError> {
        self.close()?;
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
            item_line(&mut probe, self.mztab, item).is_ok() && fingerprint.finish() == v.fingerprint
        });
        match verbatim {
            Some(v) => self.out.str(v.text.as_str())?,
            None => item_line(&mut self.out, self.mztab, item)?,
        }
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
        section: Section,
    ) -> Result<(), WriteError> {
        let total = pending.row_count(section);
        let [sml, smf, sme] = &mut pending.rows;
        let written = match section {
            Section::Mtd | Section::Sml => sml,
            Section::Smf => smf,
            Section::Sme => sme,
        };
        let first = core::mem::replace(written, total);
        (first..total).try_for_each(|n| self.canonical(&Item::Row(section, n)))
    }
}

/// Writes `mztab` as mzTab-M TSV.
///
/// # Errors
///
/// [`WriteError::Fmt`] if `out` fails, [`WriteError::Unwritable`] if a value
/// cannot be written so that it reads back the same.
pub fn write(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    let mut pending = Pending::new(mztab);
    let mut w = Writer {
        out: Out::new(out),
        mztab,
        open: false,
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
    let last_of = |section: Section| {
        lines.iter().rposition(
            |l| matches!(l.content, LineContent::Header(s) | LineContent::Row(s) if s == section),
        )
    };
    let last_table = [
        last_of(Section::Sml),
        last_of(Section::Smf),
        last_of(Section::Sme),
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
        for (t, section) in [Section::Sml, Section::Smf, Section::Sme]
            .into_iter()
            .enumerate()
        {
            if last_table.get(t) == Some(&Some(i)) && pending.headers.get(t) == Some(&true) {
                w.rest_of_rows(&mut pending, section)?;
            }
        }
    }
    let mut anything =
        !lines.is_empty() || !pending.entries.is_empty() || !mztab.metadata.unknown.is_empty();
    for (t, section) in [Section::Sml, Section::Smf, Section::Sme]
        .into_iter()
        .enumerate()
    {
        if pending.has_table(section) && pending.headers.get(t) == Some(&false) {
            if anything {
                w.blank()?;
            }
            w.canonical(&Item::Header(section))?;
            w.rest_of_rows(&mut pending, section)?;
            anything = true;
        }
    }
    Ok(())
}

/// Writes `mztab` to a new string. See [`write()`].
///
/// # Errors
///
/// As [`write()`], except that writing to a `String` cannot fail.
pub fn to_string(mztab: &MzTab<'_>) -> Result<String, WriteError> {
    let mut out = String::new();
    write(mztab, &mut out)?;
    Ok(out)
}
