//! Reading TSV: [`parse`], [`parse_bytes`] and the push-based
//! [`StreamReader`].
//!
//! Both entry points run the same line typer, which turns each line into
//! [`Event`]s. [`parse`] collects the events into an [`MzTab`] that borrows
//! the input; [`StreamReader`] hands them to a callback as complete lines
//! arrive, so memory stays proportional to one line.
//!
//! Reading never fails and never stops early: every problem becomes a
//! [`Finding`], and the text that caused it is kept in the model.
//!
//! ```
//! let input = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tEX-1\n";
//! let parsed = mztab_m::parse(input);
//! assert!(parsed.findings.is_empty());
//! assert_eq!(parsed.mztab.metadata.id.and_then(|c| c.value().map(|t| t.to_string())), Some("EX-1".into()));
//! ```

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use crate::diag::{Code, Detail, Finding, Findings};
use crate::meta::{MtdEntry, MtdKey};
use crate::model::{
    ExtraLine, Header, LineEnding, LineInfo, MzTab, SmeColumn, SmeRow, SmfColumn, SmfRow,
    SmlColumn, SmlRow, Table,
};
use crate::table::{self, Slot};
use crate::typing::{self, Cx, RawCell};
use crate::value::Raw;
use crate::vocab::Section;

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// One step of reading: a typed line, a layout fact, or a finding.
///
/// Every line of the input produces exactly one line event (all variants
/// but [`Event::Bom`] and [`Event::Finding`]), followed by the findings
/// about it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "events are consumed one at a time; boxing rows would cost an allocation per row"
)]
pub enum Event<'a> {
    /// The input started with a UTF-8 byte order mark, which was stripped.
    Bom,
    /// An empty or whitespace-only line.
    Blank(LineInfo, Raw<'a>),
    /// A `COM` line, as written.
    Comment(LineInfo, Raw<'a>),
    /// A line that could not be placed, as written.
    Unrecognized(LineInfo, Raw<'a>),
    /// An `MTD` line.
    Metadata(LineInfo, MtdEntry<'a>),
    /// The `SMH` line.
    SmlHeader(LineInfo, Header<'a, SmlColumn>),
    /// The `SFH` line.
    SmfHeader(LineInfo, Header<'a, SmfColumn>),
    /// The `SEH` line.
    SmeHeader(LineInfo, Header<'a, SmeColumn>),
    /// An `SML` row.
    Sml(LineInfo, SmlRow<'a>),
    /// An `SMF` row.
    Smf(LineInfo, SmfRow<'a>),
    /// An `SME` row.
    Sme(LineInfo, SmeRow<'a>),
    /// A finding about the preceding line.
    Finding(Finding),
}

/// The line typer: the state carried from line to line. It holds no
/// borrowed data, so it works across stream chunks.
#[derive(Debug, Default)]
struct Typer {
    number: u32,
    current: Option<Section>,
    furthest: Option<Section>,
    seen_keys: BTreeSet<MtdKey>,
    sml: Option<Vec<Slot<SmlColumn>>>,
    smf: Option<Vec<Slot<SmfColumn>>>,
    sme: Option<Vec<Slot<SmeColumn>>>,
    findings: Vec<Finding>,
}

/// The cells of `text` after the first (the prefix), numbered from 2.
fn cells(text: &str) -> impl Iterator<Item = RawCell<'_>> {
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

impl Typer {
    /// Types one line, given without its line break.
    fn line<'a>(&mut self, bytes: &'a [u8], ending: LineEnding, sink: &mut impl FnMut(Event<'a>)) {
        self.number = self.number.saturating_add(1);
        let info = LineInfo {
            number: self.number,
            ending,
        };
        let mut bytes = bytes;
        if let Some(rest) = bytes.strip_prefix(BOM).filter(|_| self.number == 1) {
            bytes = rest;
            sink(Event::Bom);
            let at = crate::diag::Location::text(1, 1);
            sink(Event::Finding(Finding::new(Code::Bom, at, Detail::None)));
        }
        match core::str::from_utf8(bytes) {
            Ok(text) => {
                let event = self.typed(text, info);
                sink(event);
            }
            Err(error) => {
                let valid = bytes.get(..error.valid_up_to()).unwrap_or_default();
                let column = String::from_utf8_lossy(valid).chars().count();
                let column = u32::try_from(column).map_or(u32::MAX, |n| n.saturating_add(1));
                let at = crate::diag::Location::text(info.number, column);
                self.findings
                    .push(Finding::new(Code::Utf8, at, Detail::None));
                let lossy = String::from_utf8_lossy(bytes).into_owned();
                sink(Event::Unrecognized(info, Raw::owned(lossy)));
            }
        }
        for finding in self.findings.drain(..) {
            sink(Event::Finding(finding));
        }
    }

    const fn cx<'l>(findings: &'l mut Vec<Finding>, line: &'l str, number: u32) -> Cx<'l> {
        Cx {
            line,
            number,
            findings,
        }
    }

    /// Types a decoded line into its line event, collecting findings.
    fn typed<'a>(&mut self, text: &'a str, info: LineInfo) -> Event<'a> {
        if text.bytes().all(|b| b == b' ' || b == b'\t') {
            return Event::Blank(info, Raw::new(text));
        }
        let prefix = text.split('\t').next().unwrap_or_default();
        if prefix == "COM" {
            return Event::Comment(info, Raw::new(text));
        }
        let Some((section, is_header)) = Section::from_prefix(prefix) else {
            let detail =
                known_prefix_ignoring_case(prefix).map_or(Detail::None, Detail::PrefixCase);
            let mut cx = Self::cx(&mut self.findings, text, info.number);
            cx.report(Code::LinePrefix, 0, 1, detail);
            return Event::Unrecognized(info, Raw::new(text));
        };
        self.check_order(section, text, info.number);
        let mut cx = Self::cx(&mut self.findings, text, info.number);
        let unrecognized = || Event::Unrecognized(info, Raw::new(text));
        match (section, is_header) {
            (Section::Mtd, _) => {
                Event::Metadata(info, metadata(&mut cx, &mut self.seen_keys, text))
            }
            (Section::Sml, true) => header_slot(
                &mut cx,
                &mut self.sml,
                Section::Sml,
                text,
                SmlColumn::from_label,
            )
            .map_or_else(unrecognized, |h| Event::SmlHeader(info, h)),
            (Section::Smf, true) => header_slot(
                &mut cx,
                &mut self.smf,
                Section::Smf,
                text,
                SmfColumn::from_label,
            )
            .map_or_else(unrecognized, |h| Event::SmfHeader(info, h)),
            (Section::Sme, true) => header_slot(
                &mut cx,
                &mut self.sme,
                Section::Sme,
                text,
                SmeColumn::from_label,
            )
            .map_or_else(unrecognized, |h| Event::SmeHeader(info, h)),
            (Section::Sml, false) => row::<SmlRow<'a>>(&mut cx, self.sml.as_deref(), text)
                .map_or_else(unrecognized, |r| Event::Sml(info, r)),
            (Section::Smf, false) => row::<SmfRow<'a>>(&mut cx, self.smf.as_deref(), text)
                .map_or_else(unrecognized, |r| Event::Smf(info, r)),
            (Section::Sme, false) => row::<SmeRow<'a>>(&mut cx, self.sme.as_deref(), text)
                .map_or_else(unrecognized, |r| Event::Sme(info, r)),
        }
    }

    /// Reports a line whose section comes before one already seen, once per
    /// change of section.
    fn check_order(&mut self, section: Section, text: &str, number: u32) {
        if self.current == Some(section) {
            return;
        }
        self.current = Some(section);
        if self.furthest.is_some_and(|furthest| section < furthest) {
            let mut cx = Self::cx(&mut self.findings, text, number);
            cx.report(Code::SectionOrder, 0, 1, Detail::None);
        } else {
            self.furthest = Some(section);
        }
    }
}

/// The known prefix that `prefix` matches ignoring ASCII case, if any.
fn known_prefix_ignoring_case(prefix: &str) -> Option<&'static str> {
    ["MTD", "SMH", "SML", "SFH", "SMF", "SEH", "SME", "COM"]
        .into_iter()
        .find(|known| known.eq_ignore_ascii_case(prefix))
}

/// Types a header line, unless the section already has one.
fn header_slot<'a, S: Copy + Ord>(
    cx: &mut Cx<'_>,
    slots: &mut Option<Vec<Slot<S>>>,
    section: Section,
    text: &'a str,
    stable: fn(&str) -> Option<S>,
) -> Option<Header<'a, S>> {
    if slots.is_some() {
        cx.report(Code::DuplicateHeader, 0, 1, Detail::None);
        return None;
    }
    let (header, typed) = table::header(cx, section, cells(text), stable);
    *slots = Some(typed);
    Some(header)
}

/// Types a row, unless its section has no header yet.
fn row<'a, R: table::TableRow<'a>>(
    cx: &mut Cx<'_>,
    slots: Option<&[Slot<R::Column>]>,
    text: &'a str,
) -> Option<R> {
    let Some(slots) = slots else {
        cx.report(Code::RowBeforeHeader, 0, 1, Detail::None);
        return None;
    };
    Some(table::row(cx, slots, cells(text)))
}

/// Types an `MTD` line.
fn metadata<'a>(cx: &mut Cx<'_>, seen: &mut BTreeSet<MtdKey>, text: &'a str) -> MtdEntry<'a> {
    let mut parts = cells(text);
    let Some(key) = parts.next() else {
        cx.report(Code::MtdCells, text.len(), 2, Detail::None);
        return MtdEntry {
            key: Raw::new(""),
            value: Raw::new(""),
            trailing: Raw::new(""),
            typed: None,
        };
    };
    let value = parts.next();
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
    let parsed = MtdKey::parse(key.text);
    match parsed {
        None => cx.report(Code::UnknownMtdKey, key.offset, key.number, Detail::None),
        Some(k) if !k.repeats() && !seen.insert(k) => {
            cx.report(Code::DuplicateMtdKey, key.offset, key.number, Detail::None);
        }
        Some(_) => {}
    }
    let typed = parsed.map(|k| {
        let raw = value.unwrap_or(RawCell {
            text: "",
            number: 3,
            offset: text.len(),
        });
        let value = if value.is_some() {
            typing::mtd_value(cx, k.kind(), raw)
        } else {
            // Already reported as a missing cell; typing the empty text would
            // report it a second time.
            let mut quiet = Vec::new();
            let mut silent = Cx {
                line: cx.line,
                number: cx.number,
                findings: &mut quiet,
            };
            typing::mtd_value(&mut silent, k.kind(), raw)
        };
        (k, value)
    });
    MtdEntry {
        key: Raw::new(key.text),
        value: Raw::new(value.map_or("", |v| v.text)),
        trailing: Raw::new(trailing),
        typed,
    }
}

/// Splits `line` (which includes its `\n`, or is the unterminated last line)
/// into its text and ending.
fn strip_ending(line: &[u8], terminated: bool) -> (&[u8], LineEnding) {
    if !terminated {
        return (line, LineEnding::None);
    }
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r")
        .map_or((line, LineEnding::Lf), |text| (text, LineEnding::CrLf))
}

/// The result of reading a whole document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed<'a> {
    /// The document. Text that failed to type is kept in it.
    pub mztab: MzTab<'a>,
    /// Every finding, in input order.
    pub findings: Findings,
}

/// Collects events into a document.
#[derive(Debug, Default)]
struct Builder<'a> {
    mztab: MzTab<'a>,
    findings: Findings,
}

impl<'a> Builder<'a> {
    fn event(&mut self, event: Event<'a>) {
        let layout = &mut self.mztab.layout;
        match event {
            Event::Bom => layout.bom = true,
            Event::Blank(info, raw) => layout.extra_lines.push((info, ExtraLine::Blank(raw))),
            Event::Comment(info, raw) => layout.extra_lines.push((info, ExtraLine::Comment(raw))),
            Event::Unrecognized(info, raw) => {
                layout
                    .extra_lines
                    .push((info, ExtraLine::Unrecognized(raw)));
            }
            Event::Metadata(_, entry) => {
                let metadata = &mut self.mztab.metadata;
                let placed = entry
                    .typed
                    .is_some_and(|(key, value)| metadata.insert(key, value).is_ok());
                if !placed {
                    metadata.unknown.push((entry.key, entry.value));
                }
            }
            Event::SmlHeader(_, header) => {
                self.mztab.sml = Some(Table {
                    header,
                    rows: Vec::new(),
                });
            }
            Event::SmfHeader(_, header) => {
                self.mztab.smf = Some(Table {
                    header,
                    rows: Vec::new(),
                });
            }
            Event::SmeHeader(_, header) => {
                self.mztab.sme = Some(Table {
                    header,
                    rows: Vec::new(),
                });
            }
            Event::Sml(_, row) => self
                .mztab
                .sml
                .get_or_insert_with(Table::default)
                .rows
                .push(row),
            Event::Smf(_, row) => self
                .mztab
                .smf
                .get_or_insert_with(Table::default)
                .rows
                .push(row),
            Event::Sme(_, row) => self
                .mztab
                .sme
                .get_or_insert_with(Table::default)
                .rows
                .push(row),
            Event::Finding(finding) => self.findings.push(finding),
        }
    }
}

/// Collects events, from [`StreamReader`] for example, into a document.
///
/// This is exactly how [`parse_bytes`] builds its result, so collecting the
/// owned events of a stream gives the owned result of [`parse_bytes`].
///
/// # Examples
///
/// ```
/// use mztab_m::{Event, IntoOwned, Parsed, StreamReader};
///
/// let input = b"MTD\tmzTab-ID\tEX-1\nMTD\ttitle\tT\n";
/// let mut events = Vec::new();
/// let mut reader = StreamReader::new();
/// for chunk in input.chunks(5) {
///     reader.push(chunk, &mut |e: Event<'_>| events.push(e.into_owned()));
/// }
/// reader.finish(&mut |e: Event<'_>| events.push(e.into_owned()));
/// let streamed: Parsed<'static> = events.into_iter().collect();
/// assert_eq!(streamed, mztab_m::parse_bytes(input).into_owned());
/// ```
impl<'a> FromIterator<Event<'a>> for Parsed<'a> {
    fn from_iter<I: IntoIterator<Item = Event<'a>>>(events: I) -> Self {
        let mut builder = Builder::default();
        events.into_iter().for_each(|event| builder.event(event));
        Self {
            mztab: builder.mztab,
            findings: builder.findings,
        }
    }
}

/// Reads a whole document from text. The model borrows `input`.
///
/// Never fails: problems are reported in [`Parsed::findings`].
#[must_use]
pub fn parse(input: &str) -> Parsed<'_> {
    parse_bytes(input.as_bytes())
}

/// Reads a whole document from bytes, which should be UTF-8.
///
/// A leading byte order mark is stripped and reported. A line that is not
/// valid UTF-8 is reported and kept, decoded lossily, as an unrecognised
/// line.
///
/// # Examples
///
/// ```
/// let parsed = mztab_m::parse_bytes(b"\xEF\xBB\xBFMTD\tmzTab-ID\tx\nMTD\ttitle\t\xFF\n");
/// assert!(parsed.mztab.layout.bom);
/// let codes: Vec<_> = parsed.findings.iter().map(|f| f.code.id()).collect();
/// assert_eq!(codes, ["MZT-S009", "MZT-S008"]);
/// ```
#[must_use]
pub fn parse_bytes(input: &[u8]) -> Parsed<'_> {
    let mut typer = Typer::default();
    let mut builder = Builder::default();
    for line in input.split_inclusive(|&b| b == b'\n') {
        let (text, ending) = strip_ending(line, line.ends_with(b"\n"));
        typer.line(text, ending, &mut |event| builder.event(event));
    }
    Parsed {
        mztab: builder.mztab,
        findings: builder.findings,
    }
}

/// A push-based reader: feed it chunks of bytes, get events for every
/// complete line.
///
/// It owns only a buffer for the line that straddles two chunks, so memory
/// stays proportional to the longest line. Events borrow from the chunk (or
/// that buffer) and live only for the callback; call `into_owned` on an
/// event to keep it.
///
/// Chunk boundaries never change the events: feeding a document in any
/// split produces the same events as [`parse_bytes`] sees.
///
/// # Examples
///
/// ```
/// use mztab_m::{Event, StreamReader};
///
/// let mut reader = StreamReader::new();
/// let mut rows = 0;
/// let mut count = |event: Event<'_>| rows += usize::from(matches!(event, Event::Smf(..)));
/// reader.push(b"MTD\tmzTab-version\t2.1.0-M\nSFH\tSMF_ID\nSMF\t1\nSM", &mut count);
/// reader.push(b"F\t2", &mut count);
/// reader.finish(&mut count);
/// assert_eq!(rows, 2);
/// ```
#[derive(Debug, Default)]
pub struct StreamReader {
    typer: Typer,
    pending: Vec<u8>,
}

impl StreamReader {
    /// A reader at the start of a document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads `chunk`, calling `on` for the events of every line it completes.
    pub fn push(&mut self, chunk: &[u8], on: &mut impl FnMut(Event<'_>)) {
        let mut rest = chunk;
        if !self.pending.is_empty() {
            let Some(at) = rest.iter().position(|&b| b == b'\n') else {
                self.pending.extend_from_slice(rest);
                return;
            };
            let (head, tail) = rest.split_at(at + 1);
            self.pending.extend_from_slice(head);
            let mut line = core::mem::take(&mut self.pending);
            let (text, ending) = strip_ending(&line, true);
            self.typer.line(text, ending, on);
            line.clear();
            self.pending = line;
            rest = tail;
        }
        while let Some(at) = rest.iter().position(|&b| b == b'\n') {
            let (line, tail) = rest.split_at(at + 1);
            let (text, ending) = strip_ending(line, true);
            self.typer.line(text, ending, on);
            rest = tail;
        }
        self.pending.extend_from_slice(rest);
    }

    /// Ends the document, reading a last line that has no line break.
    pub fn finish(mut self, on: &mut impl FnMut(Event<'_>)) {
        if !self.pending.is_empty() {
            let line = core::mem::take(&mut self.pending);
            self.typer.line(&line, LineEnding::None, on);
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Event, Parsed, parse, parse_bytes};
    use crate::model::{LineEnding, LineInfo, SmeRow, SmfRow, SmlRow};
    use crate::table::TableRow as _;
    use alloc::vec;

    #[test]
    fn parse_is_parse_bytes() {
        let text = "MTD\tmzTab-ID\tx\r\nCOM\tlast";
        assert_eq!(parse(text), parse_bytes(text.as_bytes()), "same result");
        let endings: vec::Vec<_> = parse(text)
            .mztab
            .layout
            .extra_lines
            .iter()
            .map(|(info, _)| info.ending)
            .collect();
        assert_eq!(endings, [LineEnding::None], "last line has no ending");
        assert!(parse("").findings.is_empty(), "empty input");
    }

    #[test]
    fn section_order_is_reported_once_per_backward_step() {
        let text = "SFH\tSMF_ID\nMTD\tmzTab-ID\tx\nSMF\t1\nSMH\tSML_ID\nSML\t1\n";
        let lines: vec::Vec<_> = parse(text)
            .findings
            .iter()
            .filter(|f| f.code == crate::diag::Code::SectionOrder)
            .map(|f| f.location.clone())
            .collect();
        let at = |line| crate::diag::Location::text(line, 1).with_cell(1);
        assert_eq!(
            lines,
            [at(2), at(4)],
            "MTD after SMF, SMH after SMF; not SMF again"
        );
    }

    #[test]
    fn rows_without_header_events_get_an_empty_table() {
        let info = LineInfo {
            number: 1,
            ending: LineEnding::Lf,
        };
        let parsed: Parsed<'_> = vec![
            Event::Sml(info, SmlRow::empty()),
            Event::Smf(info, SmfRow::empty()),
            Event::Sme(info, SmeRow::empty()),
        ]
        .into_iter()
        .collect();
        let counts = [
            parsed
                .mztab
                .sml
                .map(|t| (t.header.columns.len(), t.rows.len())),
            parsed
                .mztab
                .smf
                .map(|t| (t.header.columns.len(), t.rows.len())),
            parsed
                .mztab
                .sme
                .map(|t| (t.header.columns.len(), t.rows.len())),
        ];
        assert_eq!(counts, [Some((0, 1)); 3], "one row, no columns");
    }
}
