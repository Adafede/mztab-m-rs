//! Stateful line typer: turns decoded lines into [`Event`]s with section-order
//! and verbatim-source checks.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use super::event::Event;
use super::metadata::{cells, metadata};
use crate::diag::code::Code;
use crate::diag::finding::{Detail, Finding};
use crate::diag::location::Location;
use crate::meta::key::MtdKey;
use crate::model::layout::{LineEnding, LineInfo, Verbatim};
use crate::model::rows::{SmeRow, SmfRow, SmlRow};
use crate::model::table::{Header, SmeColumn, SmfColumn, SmlColumn};
use crate::tsv::table::{self, Slot, TableRow};
use crate::tsv::typing::Cx;
use crate::value::text::Raw;
use crate::vocab::section::{Section, TableSection};
use crate::write::line::{metadata_line, row_line, unknown_line};
use crate::write::sink::{Compare, Out};

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The line typer: the state carried from line to line. It holds no
/// borrowed data, so it works across stream chunks.
#[derive(Debug, Default)]
pub(super) struct Typer {
    number: u32,
    current: Option<Section>,
    furthest: Option<Section>,
    seen_keys: BTreeSet<MtdKey>,
    /// Whether the last metadata line is placed in the model (not unknown,
    /// not a duplicate).
    placed: bool,
    sml: Option<Vec<Slot<SmlColumn>>>,
    smf: Option<Vec<Slot<SmfColumn>>>,
    sme: Option<Vec<Slot<SmeColumn>>>,
    findings: Vec<Finding>,
}

impl Typer {
    /// Types one line, given without its line break.
    pub(super) fn line<'a>(
        &mut self,
        bytes: &'a [u8],
        ending: LineEnding,
        sink: &mut dyn FnMut(Event<'a>),
    ) {
        self.number = self.number.saturating_add(1);
        let info = LineInfo {
            number: self.number,
            ending,
        };
        let mut bytes = bytes;
        if let Some(rest) = bytes.strip_prefix(BOM).filter(|_| self.number == 1) {
            bytes = rest;
            sink(Event::Bom);
            sink(Event::Finding(Finding::new(
                Code::Bom,
                Location::text(1, 1),
                Detail::None,
            )));
        }
        match core::str::from_utf8(bytes) {
            Ok(text) => {
                let event = self.typed(text, info);
                let verbatim = self.verbatim(&event, text);
                sink(event);
                if let Some(verbatim) = verbatim {
                    sink(Event::Verbatim(verbatim));
                }
            }
            Err(error) => self.push_invalid_utf8(bytes, error.valid_up_to(), info, sink),
        }
        for finding in self.findings.drain(..) {
            sink(Event::Finding(finding));
        }
    }

    fn push_invalid_utf8<'a>(
        &mut self,
        bytes: &'a [u8],
        valid_up_to: usize,
        info: LineInfo,
        sink: &mut dyn FnMut(Event<'a>),
    ) {
        let valid = bytes.get(..valid_up_to).unwrap_or_default();
        let column = String::from_utf8_lossy(valid).chars().count();
        let column = u32::try_from(column).map_or(u32::MAX, |n| n.saturating_add(1));
        self.findings.push(Finding::new(
            Code::Utf8,
            Location::text(info.number, column),
            Detail::None,
        ));
        let lossy = String::from_utf8_lossy(bytes).into_owned();
        sink(Event::Unrecognized(info, Raw::owned(lossy)));
    }

    /// The line's text, if it differs from the canonical form of its event.
    fn verbatim<'a>(&self, event: &Event<'a>, text: &'a str) -> Option<Verbatim<'a>> {
        let mut compare = Compare::new(text);
        let mut out = Out::new(&mut compare);
        let rendered = match event {
            Event::Metadata(_, entry) => match (&entry.typed, self.placed) {
                (Some((key, value)), true) => metadata_line(&mut out, *key, value.as_ref()),
                _ => unknown_line(&mut out, &entry.key, &entry.value),
            },
            Event::Sml(_, row) => row_line(&mut out, TableSection::Sml, self.sml.as_deref()?, row),
            Event::Smf(_, row) => row_line(&mut out, TableSection::Smf, self.smf.as_deref()?, row),
            Event::Sme(_, row) => row_line(&mut out, TableSection::Sme, self.sme.as_deref()?, row),
            // Header lines always render as written; blank, comment and
            // unrecognised lines are stored as written.
            _ => return None,
        };
        (rendered.is_err() || !compare.matched()).then(|| Verbatim {
            text: Raw::new(text),
            fingerprint: compare.fingerprint.finish(),
        })
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
        match (section, is_header) {
            (Section::Mtd, _) => {
                let (entry, placed) = metadata(&mut cx, &mut self.seen_keys, text);
                self.placed = placed;
                Event::Metadata(info, entry)
            }
            (Section::Sml, true) => header_slot(
                &mut cx,
                &mut self.sml,
                Section::Sml,
                text,
                SmlColumn::from_label,
            )
            .map_or_else(
                || Event::Unrecognized(info, Raw::new(text)),
                |h| Event::SmlHeader(info, h),
            ),
            (Section::Smf, true) => header_slot(
                &mut cx,
                &mut self.smf,
                Section::Smf,
                text,
                SmfColumn::from_label,
            )
            .map_or_else(
                || Event::Unrecognized(info, Raw::new(text)),
                |h| Event::SmfHeader(info, h),
            ),
            (Section::Sme, true) => header_slot(
                &mut cx,
                &mut self.sme,
                Section::Sme,
                text,
                SmeColumn::from_label,
            )
            .map_or_else(
                || Event::Unrecognized(info, Raw::new(text)),
                |h| Event::SmeHeader(info, h),
            ),
            (Section::Sml, false) => row::<SmlRow<'a>>(&mut cx, self.sml.as_deref(), text)
                .map_or_else(
                    || Event::Unrecognized(info, Raw::new(text)),
                    |r| Event::Sml(info, r),
                ),
            (Section::Smf, false) => row::<SmfRow<'a>>(&mut cx, self.smf.as_deref(), text)
                .map_or_else(
                    || Event::Unrecognized(info, Raw::new(text)),
                    |r| Event::Smf(info, r),
                ),
            (Section::Sme, false) => row::<SmeRow<'a>>(&mut cx, self.sme.as_deref(), text)
                .map_or_else(
                    || Event::Unrecognized(info, Raw::new(text)),
                    |r| Event::Sme(info, r),
                ),
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
fn row<'a, R: TableRow<'a>>(
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

/// Splits `line` (which includes its `\n`, or is the unterminated last line)
/// into its text and ending.
pub(super) fn strip_ending(line: &[u8], terminated: bool) -> (&[u8], LineEnding) {
    if !terminated {
        return (line, LineEnding::None);
    }
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r")
        .map_or((line, LineEnding::Lf), |text| (text, LineEnding::CrLf))
}
