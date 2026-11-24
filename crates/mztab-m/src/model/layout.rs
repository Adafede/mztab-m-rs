//! Lossless source layout metadata for byte-exact TSV round trips.

use alloc::vec::Vec;

use crate::meta::key::MtdKey;
use crate::value::text::Raw;
use crate::vocab::section::Section;

/// How a line ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// `\n`.
    #[default]
    Lf,
    /// `\r\n`.
    CrLf,
    /// The last line of the input, with no line break.
    None,
}

/// Where a line was in the source: its 1-based number and its ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineInfo {
    /// 1-based line number.
    pub number: u32,
    /// The line break that ended it.
    pub ending: LineEnding,
}

/// A source line that carries no model data.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExtraLine<'a> {
    /// An empty or whitespace-only line, as written.
    Blank(Raw<'a>),
    /// A `COM` line, as written.
    Comment(Raw<'a>),
    /// A line the reader could not place, as written: an unknown prefix, a
    /// row with no header, a repeated header, or invalid UTF-8 (decoded
    /// lossily).
    Unrecognized(Raw<'a>),
}

/// The source text of a line whose canonical form differs from it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Verbatim<'a> {
    /// The line as written, without its line break.
    pub text: Raw<'a>,
    /// A fingerprint of the line's canonical form when it was read. The
    /// writer uses `text` only while the model still renders to it, so an
    /// edited value is never overwritten by stale source text.
    pub fingerprint: u64,
}

/// What a source line carried.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LineContent<'a> {
    /// The next value of this key (`colunit-*` keys repeat).
    Metadata(MtdKey),
    /// The next entry of [`Metadata::unknown`](super::Metadata::unknown).
    UnknownMetadata,
    /// The header of a table.
    Header(Section),
    /// The next row of a table.
    Row(Section),
    /// A line with no model data.
    Extra(ExtraLine<'a>),
}

/// One line of the source, in order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceLine<'a> {
    /// What the line carried.
    pub content: LineContent<'a>,
    /// How it ended.
    pub ending: LineEnding,
    /// Its text, when that differs from the canonical form.
    pub verbatim: Option<Verbatim<'a>>,
}

/// What the typed model does not capture about the source text: line order,
/// line endings, blank and comment lines, and the exact text of every line
/// that is not in canonical form.
///
/// The reader fills it and the writer honours it, so `write(read(f)) == f`
/// byte for byte (except lines that were not valid UTF-8). It is empty for
/// models built in code, which are written in canonical form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceLayout<'a> {
    /// Whether the input started with a UTF-8 byte order mark.
    pub bom: bool,
    /// Every source line, in order.
    pub lines: Vec<SourceLine<'a>>,
}

impl SourceLayout<'_> {
    /// Only the `COM` lines, in order, without line endings or a byte order
    /// mark. Writing a model with this layout gives its canonical form with
    /// its comments after the metadata, which is what a JSON-read model
    /// has (D-075).
    ///
    /// ```
    /// let mut model = mztab_m::parse("COM\tnote\r\nMTD\tmzTab-ID\tx\r\n").mztab;
    /// model.layout = model.layout.comments_only();
    /// assert_eq!(mztab_m::to_string(&model)?, "MTD\tmzTab-ID\tx\nCOM\tnote\n");
    /// # Ok::<(), mztab_m::WriteError>(())
    /// ```
    #[must_use]
    pub fn comments_only(self) -> Self {
        let lines = self
            .lines
            .into_iter()
            .filter(|l| matches!(l.content, LineContent::Extra(ExtraLine::Comment(_))))
            // Comment lines keep their text in the line itself, never as
            // verbatim source text, so only the ending changes.
            .map(|l| SourceLine {
                ending: LineEnding::Lf,
                ..l
            })
            .collect();
        Self { bom: false, lines }
    }
}
