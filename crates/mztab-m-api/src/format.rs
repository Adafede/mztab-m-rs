//! Format, validation layer, severity parsing, and API error types.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use core::fmt;

use mztab_m::{Severity, WriteError};

/// A file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Tab-separated mzTab-M.
    Tsv,
    /// JSON in the shape of the specification's JSON schema.
    Json,
}

impl Format {
    /// `"tsv"` or `"json"`.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] for anything else.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "tsv" => Ok(Self::Tsv),
            "json" => Ok(Self::Json),
            other => Err(Error::Format(other.to_owned())),
        }
    }

    /// JSON if the file name ends in `.json` or the text starts with `{`
    /// (after white space); TSV otherwise.
    ///
    /// ```
    /// use mztab_m_api::Format;
    ///
    /// assert_eq!(Format::sniff("a.JSON", b""), Format::Json);
    /// assert_eq!(Format::sniff("-", b"  {}"), Format::Json);
    /// assert_eq!(Format::sniff("a.mztab", b"MTD"), Format::Tsv);
    /// ```
    #[must_use]
    pub fn sniff(name: &str, bytes: &[u8]) -> Self {
        let named = name
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("json"));
        let first = bytes.iter().find(|b| !b.is_ascii_whitespace());
        if named || first == Some(&b'{') {
            Self::Json
        } else {
            Self::Tsv
        }
    }
}

/// The last validation layer to run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// What the reader finds.
    Syntax,
    /// And fields, profiles, references, columns and values.
    Structure,
    /// And the controlled vocabulary.
    #[default]
    Semantic,
}

impl Layer {
    /// `"syntax"`, `"structure"` or `"semantic"`.
    ///
    /// # Errors
    ///
    /// [`Error::Layer`] for anything else.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "syntax" => Ok(Self::Syntax),
            "structure" => Ok(Self::Structure),
            "semantic" => Ok(Self::Semantic),
            other => Err(Error::Layer(other.to_owned())),
        }
    }
}

/// What can go wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Not `tsv` or `json`.
    Format(String),
    /// Not `syntax`, `structure` or `semantic`.
    Layer(String),
    /// Not `error`, `warning`, `info` (or, for a code, `off`).
    Severity(String),
    /// Not a code id such as `MZT-T009`.
    Code(String),
    /// Not `CODE=LEVEL`.
    Setting(String),
    /// The model cannot be written in the requested format.
    Write(WriteError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(s) => write!(f, "unknown format `{s}` (tsv, json)"),
            Self::Layer(s) => write!(f, "unknown layer `{s}` (syntax, structure, semantic)"),
            Self::Severity(s) => write!(f, "unknown severity `{s}` (error, warning, info)"),
            Self::Code(s) => write!(f, "unknown code `{s}`"),
            Self::Setting(s) => write!(f, "expected CODE=LEVEL, not `{s}`"),
            Self::Write(e) => write!(f, "cannot write the file: {e}"),
        }
    }
}

impl core::error::Error for Error {}

/// `"error"`, `"warning"` or `"info"`.
///
/// # Errors
///
/// [`Error::Severity`] for anything else.
pub fn severity(name: &str) -> Result<Severity, Error> {
    name.parse().map_err(|_| Error::Severity(name.to_owned()))
}
