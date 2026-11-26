//! File format and API error types.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

use mztab_m::WriteError;

/// A file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Tab-separated mzTab-M.
    Tsv,
    /// JSON in the shape of the specification's JSON schema.
    Json,
}

impl Format {
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
        let rest = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        let first = rest.iter().find(|b| !b.is_ascii_whitespace());
        if named || first == Some(&b'{') {
            Self::Json
        } else {
            Self::Tsv
        }
    }
}

impl FromStr for Format {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tsv" => Ok(Self::Tsv),
            "json" => Ok(Self::Json),
            other => Err(Error::Format(other.to_owned())),
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
            Self::Severity(s) => write!(f, "unknown severity `{s}` (error, warning, info, off)"),
            Self::Code(s) => write!(f, "unknown code `{s}`"),
            Self::Setting(s) => write!(f, "expected CODE=LEVEL, not `{s}`"),
            Self::Write(e) => write!(f, "cannot write the file: {e}"),
        }
    }
}

impl core::error::Error for Error {}
