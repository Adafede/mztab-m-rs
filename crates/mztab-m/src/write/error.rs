//! Errors when writing mzTab-M TSV or JSON output.

use core::fmt;

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
    /// [`Cell::Invalid`](crate::Cell::Invalid), unknown columns or unknown metadata) that contains
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
