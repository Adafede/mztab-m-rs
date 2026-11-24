//! mzTab-M as JSON, in the shape of the specification's JSON schema
//! (`schema/mzTab_2_1-M.json`, "JSCH"). Feature `json`.
//!
//! [`parse()`] reads JSCH JSON into the same model as the TSV reader, with
//! findings located by JSON pointer; [`write()`] and [`to_string`] write a
//! model as compact JSCH JSON. Both go through TSV text: a JSON value is
//! the TSV cell that holds it, typed by the TSV grammars, so a document
//! reads the same from either format (DECISIONS D-071).
//!
//! ```
//! use mztab_m::{json, parse};
//!
//! let tsv = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tx\n";
//! let model = parse(tsv).mztab;
//! let text = json::to_string(&model)?;
//! assert!(text.starts_with(r#"{"metadata":{"prefix":"MTD","mzTab-version":"2.1.0-M""#));
//! let back = json::parse(&text);
//! assert!(back.findings.is_empty());
//! assert_eq!(back.mztab.metadata, model.metadata);
//! # Ok::<(), mztab_m::WriteError>(())
//! ```

mod findings;
mod lexer;
mod read;
mod schema;
mod schema_tables;
mod value;
mod write;

use alloc::string::String;
use core::fmt;

use crate::diag::Findings;
use crate::model::MzTab;
use crate::read::Parsed;
use crate::write::WriteError;
pub use value::write_string;

/// Reads JSCH JSON. Never fails: a document that is not JSON gives an
/// empty model and a [`JsonSyntax`](crate::Code::JsonSyntax) finding.
///
/// The model owns its text (JSON strings carry escapes), so it is
/// `'static`.
#[must_use]
pub fn parse(input: &str) -> Parsed<'static> {
    read::parse(input)
}

/// Writes `mztab` as compact JSCH JSON.
///
/// # Errors
///
/// [`WriteError::Fmt`] if `out` fails; [`WriteError::Unwritable`] if the
/// model cannot be written as TSV, or holds what JSCH has no place for:
/// unknown metadata keys, unknown columns, or abundance columns that do
/// not match the metadata's elements one to one, in id order.
pub fn write(mztab: &MzTab<'_>, out: &mut dyn fmt::Write) -> Result<(), WriteError> {
    write::write(mztab, out)
}

/// [`write()`] into a new `String`.
///
/// # Errors
///
/// As [`write()`].
pub fn to_string(mztab: &MzTab<'_>) -> Result<String, WriteError> {
    let mut out = String::new();
    write(mztab, &mut out)?;
    Ok(out)
}

/// Writes findings as a JSON array.
///
/// One object per finding: `code`,
/// `severity`, `layer`, `location`, `summary`, `detail`, `spec` (the URL
/// of the rule's section at the pinned spec commit) and `jmztab` (the
/// equivalent jmzTab-m codes).
///
/// ```
/// use mztab_m::{json, validate};
///
/// let findings = validate::syntax("XYZ\tx\n", &validate::Config::default());
/// let mut out = String::new();
/// json::write_findings(&findings, &mut out)?;
/// assert!(out.starts_with(r#"[{"code":"MZT-S001","severity":"error","layer":"syntax","location":{"line":1"#));
/// # Ok::<(), core::fmt::Error>(())
/// ```
///
/// # Errors
///
/// If `out` fails.
pub fn write_findings(findings: &Findings, out: &mut dyn fmt::Write) -> fmt::Result {
    findings::write(findings, out)
}
