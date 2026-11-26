//! Events and whole-document result produced by the TSV reader.

use crate::diag::finding::{Finding, Findings};
use crate::model::document::MzTab;
use crate::model::layout::{LineInfo, Verbatim};
use crate::model::meta_value::MtdEntry;
use crate::model::rows::{SmeRow, SmfRow, SmlRow};
use crate::model::table::{Header, SmeColumn, SmfColumn, SmlColumn};
use crate::value::text::Raw;

/// One step of reading: a typed line, a layout fact, or a finding.
///
/// Every line of the input produces one line event (all variants except
/// [`Event::Bom`], [`Event::Verbatim`], and [`Event::Finding`]), followed by
/// its [`Event::Verbatim`] if non-canonical and any [`Event::Finding`]s about it.
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
    /// The text of the preceding line, which differs from its canonical
    /// form; the writer reproduces it.
    Verbatim(Verbatim<'a>),
    /// A finding about the preceding line.
    Finding(Finding),
}

/// The result of reading a whole document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed<'a> {
    /// The document. Text that failed to type is kept in it.
    pub mztab: MzTab<'a>,
    /// Every finding, in input order.
    pub findings: Findings,
}
