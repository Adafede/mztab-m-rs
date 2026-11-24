//! Findings: what the reader and the validators report.
//!
//! A [`Finding`] pairs a stable [`Code`] with a [`Severity`], a precise
//! [`Location`] and structured [`Detail`]. Nothing is pre-formatted: the
//! English message is rendered by `Display` only when asked for.
//!
//! ```
//! use mztab_m::{Code, Detail, Finding, Location, Severity};
//!
//! let finding = Finding::new(Code::ParamSyntax, Location::text(12, 5), Detail::None);
//! assert_eq!(finding.severity, Severity::Error);
//! assert_eq!(finding.code.id(), "MZT-S002");
//! assert_eq!(finding.to_string(), "error[MZT-S002] line 12, column 5: invalid parameter");
//! ```
//!
//! # Codes are stable
//!
//! A code's id is never reused or renumbered, and a retired code keeps its
//! number. jmzTab-m codes appear only as a mapping, [`Code::jmztab`], used by
//! the differential tests (DECISIONS J-17).

/// The path of a specification document: [`SpecDoc::path`] and the
/// generated docs of each [`Code`].
macro_rules! spec_path {
    (Format) => {
        "specification_documents/mzTab_format_specification_2_1-M.adoc"
    };
    (FieldReference) => {
        "specification_documents/mzTab_m_2_1_schema.adoc"
    };
}

pub(crate) mod code;
pub(crate) mod finding;
pub(crate) mod location;
pub(crate) mod severity;
