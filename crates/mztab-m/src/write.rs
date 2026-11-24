//! Writing TSV: [`write()`], [`to_string`], and [`to_canonical_string`].
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

mod error;
mod layout;
mod line;
mod pending;
mod render;
mod sink;

use alloc::string::String;
use core::fmt;

use crate::model::MzTab;

pub use error::WriteError;
pub(crate) use line::{metadata_line, row_line, unknown_line};
pub(crate) use pending::{header_text, row_text};
pub(crate) use sink::{Compare, Out};

/// Writes `mztab` as mzTab-M TSV, preserving its [`SourceLayout`](crate::SourceLayout).
///
/// # Errors
///
/// [`WriteError::Fmt`] if `out` fails, [`WriteError::Unwritable`] if a value
/// cannot be written so that it reads back the same.
pub fn write(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    layout::write_layout(mztab, out)
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

/// Writes `mztab` in canonical TSV form to a new string without cloning
/// `mztab`. When `keep_comments` is `true`, `COM` lines from
/// [`MzTab::layout`](crate::MzTab::layout) are preserved after metadata.
///
/// # Errors
///
/// [`WriteError::Unwritable`] if a value cannot be written so that it reads
/// back the same.
pub fn to_canonical_string(mztab: &MzTab<'_>, keep_comments: bool) -> Result<String, WriteError> {
    let mut out = String::new();
    layout::write_canonical_opts(mztab, &mut out, keep_comments)?;
    Ok(out)
}
