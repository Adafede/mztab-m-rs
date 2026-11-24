//! Typing raw TSV cells into [`Cell`]s, reporting a located finding for
//! every cell (or list item) that does not type.

use alloc::vec::Vec;
use core::str::FromStr;

use super::splitter::split_list;
use crate::diag::{Code, Detail, Finding, Location};
use crate::id::{ElementId, parse_index};
use crate::model::{List, SpectraRef, SpectraRefError};
use crate::param::{Param, ParamError};
use crate::value::{Cell, Decimal, NULL, Raw, Text, TextError};
use crate::vocab::AmbiguityCode;

/// A cell of a line: its text, its 1-based number, and its byte offset in
/// the line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawCell<'a> {
    pub(crate) text: &'a str,
    pub(crate) number: u32,
    pub(crate) offset: usize,
}

impl<'a> RawCell<'a> {
    /// A sub-slice of this cell starting `at` bytes in, as its own cell.
    pub(super) const fn part(self, text: &'a str, at: usize) -> Self {
        Self {
            text,
            number: self.number,
            offset: self.offset + at,
        }
    }
}

/// The line being typed and where its findings go.
pub(crate) struct Cx<'l> {
    pub(crate) line: &'l str,
    pub(crate) number: u32,
    pub(crate) findings: &'l mut Vec<Finding>,
}

impl Cx<'_> {
    /// The location of byte `offset` of the line, in cell `cell`.
    pub(crate) fn at(&self, offset: usize, cell: u32) -> Location {
        Location::text(self.number, self.column(offset)).with_cell(cell)
    }

    /// The 1-based column of byte `offset`, counted in characters.
    pub(crate) fn column(&self, offset: usize) -> u32 {
        let before = self.line.get(..offset).unwrap_or(self.line);
        u32::try_from(before.chars().count()).map_or(u32::MAX, |n| n.saturating_add(1))
    }

    /// Records a finding at byte `offset` of cell `cell`.
    pub(crate) fn report(&mut self, code: Code, offset: usize, cell: u32, detail: Detail) {
        let location = self.at(offset, cell);
        self.findings.push(Finding::new(code, location, detail));
    }
}

/// Whether [`typed`] parses the text with its surrounding spaces removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Spaces {
    /// Strings: spaces are part of the value (and still reported).
    Keep,
    /// Everything else: spaces are reported and ignored (DECISIONS J-01).
    Trim,
    /// A whole list cell: spaces are reported by its items, unless the cell
    /// is `null`.
    List,
}

/// What a cell is before its value is parsed: `null`, unusable, or text to
/// parse (with the byte offset where that text starts in the cell).
enum Pre<'a> {
    Null,
    Invalid(Raw<'a>),
    Parse(&'a str, usize),
}

impl<'a> Pre<'a> {
    /// Checks a cell's emptiness, surrounding spaces and `null`, reporting
    /// what it finds. Not generic, so every branch is covered once for all
    /// cell types.
    fn of(cx: &mut Cx<'_>, raw: RawCell<'a>, spaces: Spaces) -> Self {
        let trimmed = raw.text.trim_matches(' ');
        if trimmed.is_empty() {
            cx.report(
                Code::TextCell,
                raw.offset,
                raw.number,
                Detail::Text(TextError::Empty),
            );
            return Self::Invalid(Raw::new(raw.text));
        }
        let padded = trimmed.len() != raw.text.len();
        if padded && (spaces != Spaces::List || trimmed == NULL) {
            cx.report(Code::CellWhitespace, raw.offset, raw.number, Detail::None);
        }
        if trimmed == NULL {
            return Self::Null;
        }
        match spaces {
            Spaces::Keep => Self::Parse(raw.text, 0),
            Spaces::Trim | Spaces::List => Self::Parse(
                trimmed,
                raw.text.len() - raw.text.trim_start_matches(' ').len(),
            ),
        }
    }
}

/// Types `raw` with `parse`. `null` gives [`Cell::Null`]; an empty (or
/// all-space) cell or a parse error gives [`Cell::Invalid`] and a finding;
/// surrounding spaces give a warning. `explain` maps a parse error to a
/// code, a byte offset inside the parsed text, and the detail.
pub(super) fn typed<'a, T, E>(
    cx: &mut Cx<'_>,
    raw: RawCell<'a>,
    spaces: Spaces,
    parse: impl FnOnce(&'a str) -> Result<T, E>,
    explain: impl FnOnce(E) -> (Code, usize, Detail),
) -> Cell<'a, T> {
    let (input, lead) = match Pre::of(cx, raw, spaces) {
        Pre::Null => return Cell::Null,
        Pre::Invalid(raw) => return Cell::Invalid(raw),
        Pre::Parse(input, lead) => (input, lead),
    };
    match parse(input) {
        Ok(value) => Cell::Value(value),
        Err(error) => {
            let (code, at, detail) = explain(error);
            cx.report(code, raw.offset + lead + at, raw.number, detail);
            Cell::Invalid(Raw::new(raw.text))
        }
    }
}

pub(super) fn text<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, Text<'a>> {
    typed(cx, raw, Spaces::Keep, Text::new, |e| {
        (Code::TextCell, e.offset(), Detail::Text(e))
    })
}

pub(super) fn param<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, Param<'a>> {
    typed(cx, raw, Spaces::Trim, Param::parse, |e: ParamError| {
        (Code::ParamSyntax, e.offset, Detail::Param(e))
    })
}

pub(super) fn decimal<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, Decimal<'a>> {
    let cell = typed(cx, raw, Spaces::Trim, Decimal::parse, |e| {
        (Code::NumberSyntax, e.offset, Detail::Number(e))
    });
    if cell.value().is_some_and(|d| !d.kind().is_conforming()) {
        cx.report(Code::NumberForm, raw.offset, raw.number, Detail::None);
    }
    cell
}

pub(super) fn integer<'a, T: FromStr>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, T> {
    typed(cx, raw, Spaces::Trim, str::parse, |_| {
        (Code::NumberSyntax, 0, Detail::Expected("an integer"))
    })
}

pub(super) fn ambiguity<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, AmbiguityCode> {
    typed(
        cx,
        raw,
        Spaces::Trim,
        |s| AmbiguityCode::parse(s).ok_or(()),
        |()| {
            (
                Code::ValueSyntax,
                0,
                Detail::Expected("an ambiguity code `1`, `2` or `3`"),
            )
        },
    )
}

pub(super) fn spectra_ref<'a>(cx: &mut Cx<'_>, raw: RawCell<'a>) -> Cell<'a, SpectraRef<'a>> {
    typed(cx, raw, Spaces::Trim, SpectraRef::parse, |e| match e {
        SpectraRefError::MsRun => (
            Code::ValueSyntax,
            0,
            Detail::Expected("`ms_run[n]:` followed by a spectrum reference"),
        ),
        SpectraRefError::Reference(t) => (Code::TextCell, 0, Detail::Text(t)),
    })
}

/// Parses `prefix[n]` for `I`.
pub(super) fn indexed<I: ElementId>(label: &str, prefix: &str) -> Option<I> {
    let inner = label
        .strip_prefix(prefix)?
        .strip_prefix('[')?
        .strip_suffix(']')?;
    I::from_index(parse_index(inner)?)
}

pub(super) fn reference<'a, I: ElementId>(
    cx: &mut Cx<'_>,
    raw: RawCell<'a>,
    expected: &'static str,
) -> Cell<'a, I> {
    typed(
        cx,
        raw,
        Spaces::Trim,
        |s| indexed::<I>(s, I::ELEMENT).ok_or(()),
        |()| (Code::ValueSyntax, 0, Detail::Expected(expected)),
    )
}

/// Types a `|`-separated list. A whole-cell `null` is [`Cell::Null`]; each
/// item is typed with `item`, which reports its own findings.
pub(super) fn list<'a, T>(
    cx: &mut Cx<'_>,
    raw: RawCell<'a>,
    brackets: bool,
    mut item: impl FnMut(&mut Cx<'_>, RawCell<'a>) -> Cell<'a, T>,
) -> Cell<'a, List<'a, T>> {
    match Pre::of(cx, raw, Spaces::List) {
        Pre::Null => return Cell::Null,
        Pre::Invalid(raw) => return Cell::Invalid(raw),
        Pre::Parse(..) => {}
    }
    let items = split_list(raw.text, brackets)
        .into_iter()
        .map(|(at, text)| item(cx, raw.part(text, at)))
        .collect();
    Cell::Value(items)
}
