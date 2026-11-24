//! Whole-document TSV entry points ([`parse`], [`parse_bytes`]) and event
//! collection into [`Parsed`].

use super::event::{Event, Parsed};
use super::typer::{Typer, strip_ending};
use crate::diag::finding::Findings;
use crate::model::document::MzTab;
use crate::model::layout::{ExtraLine, LineContent, SourceLine};
use crate::model::table::Table;
use crate::vocab::section::Section;

/// Collects events into a document.
#[derive(Debug, Default)]
struct Builder<'a> {
    mztab: MzTab<'a>,
    findings: Findings,
}

impl<'a> Builder<'a> {
    fn event(&mut self, event: Event<'a>) {
        let (info, content) = match event {
            Event::Bom => {
                self.mztab.layout.bom = true;
                return;
            }
            Event::Finding(finding) => {
                self.findings.push(finding);
                return;
            }
            Event::Verbatim(verbatim) => {
                if let Some(line) = self.mztab.layout.lines.last_mut() {
                    line.verbatim = Some(verbatim);
                }
                return;
            }
            Event::Blank(info, raw) => (info, LineContent::Extra(ExtraLine::Blank(raw))),
            Event::Comment(info, raw) => (info, LineContent::Extra(ExtraLine::Comment(raw))),
            Event::Unrecognized(info, raw) => {
                (info, LineContent::Extra(ExtraLine::Unrecognized(raw)))
            }
            Event::Metadata(info, entry) => {
                let metadata = &mut self.mztab.metadata;
                let placed = entry
                    .typed
                    .and_then(|(key, value)| metadata.insert(key, value).ok().map(|()| key));
                if let Some(key) = placed {
                    (info, LineContent::Metadata(key))
                } else {
                    metadata.unknown.push((entry.key, entry.value));
                    (info, LineContent::UnknownMetadata)
                }
            }
            Event::SmlHeader(info, header) => {
                self.mztab.sml.get_or_insert_with(Table::default).header = header;
                (info, LineContent::Header(Section::Sml))
            }
            Event::SmfHeader(info, header) => {
                self.mztab.smf.get_or_insert_with(Table::default).header = header;
                (info, LineContent::Header(Section::Smf))
            }
            Event::SmeHeader(info, header) => {
                self.mztab.sme.get_or_insert_with(Table::default).header = header;
                (info, LineContent::Header(Section::Sme))
            }
            Event::Sml(info, row) => {
                self.mztab
                    .sml
                    .get_or_insert_with(Table::default)
                    .rows
                    .push(row);
                (info, LineContent::Row(Section::Sml))
            }
            Event::Smf(info, row) => {
                self.mztab
                    .smf
                    .get_or_insert_with(Table::default)
                    .rows
                    .push(row);
                (info, LineContent::Row(Section::Smf))
            }
            Event::Sme(info, row) => {
                self.mztab
                    .sme
                    .get_or_insert_with(Table::default)
                    .rows
                    .push(row);
                (info, LineContent::Row(Section::Sme))
            }
        };
        self.mztab.layout.lines.push(SourceLine {
            content,
            ending: info.ending,
            verbatim: None,
        });
    }
}

/// Collects events, from [`StreamReader`](super::StreamReader) for example, into a document.
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
