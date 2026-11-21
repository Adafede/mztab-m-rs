#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use alloc::borrow::ToOwned as _;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Write as _};

use mztab_m::cv::TermLookup;
use mztab_m::validate::{self, Config};
use mztab_m::{
    Code, Findings, IntoOwned as _, Location, MzTab, Parsed, Severity, Stripped, WriteError, json,
};
use mztab_m_obo::Ontology;

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
    match name {
        "error" => Ok(Severity::Error),
        "warning" => Ok(Severity::Warning),
        "info" => Ok(Severity::Info),
        other => Err(Error::Severity(other.to_owned())),
    }
}

/// How to validate: the last layer to run and the severity settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The last layer to run.
    pub layer: Layer,
    /// Severity overrides and disabled codes.
    pub config: Config,
}

impl Settings {
    /// With `layer` as the last layer.
    #[must_use]
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    /// With `CODE=LEVEL` applied: LEVEL is `error`, `warning`, `info` or
    /// `off` (not reported).
    ///
    /// ```
    /// use mztab_m::Code;
    /// use mztab_m_api::Settings;
    ///
    /// let settings = Settings::default().set("MZT-X002=off")?;
    /// assert_eq!(settings.config.severity_of(Code::CvUnused), None);
    /// # Ok::<(), mztab_m_api::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// If `setting` is not of that form, or names no code.
    pub fn set(mut self, setting: &str) -> Result<Self, Error> {
        let (id, level) = setting
            .split_once('=')
            .ok_or_else(|| Error::Setting(setting.to_owned()))?;
        let code = Code::ALL
            .iter()
            .copied()
            .find(|c| c.id() == id)
            .ok_or_else(|| Error::Code(id.to_owned()))?;
        self.config = if level == "off" {
            self.config.without(code)
        } else {
            self.config.with_severity(code, severity(level)?)
        };
        Ok(self)
    }
}

/// Reads a document. Never fails: problems are findings.
#[must_use]
pub fn read(bytes: &[u8], format: Format) -> Parsed<'static> {
    match format {
        Format::Tsv => mztab_m::parse_bytes(bytes).into_owned(),
        Format::Json => json::parse(&String::from_utf8_lossy(bytes)),
    }
}

/// The reader's findings and those of each layer up to the settings'
/// layer, with the settings' severities. A value that failed to type is
/// reported once, by the reader.
#[must_use]
pub fn validate(parsed: &Parsed<'_>, settings: &Settings, terms: &dyn TermLookup) -> Findings {
    let config = &settings.config;
    let mut out = Findings::new();
    for mut finding in parsed.findings.iter().cloned() {
        if let Some(severity) = config.severity_of(finding.code) {
            finding.severity = severity;
            out.push(finding);
        }
    }
    if settings.layer >= Layer::Structure {
        let structure = validate::structure(&parsed.mztab, config).into_vec();
        out.extend(
            structure
                .into_iter()
                .filter(|f| f.code != Code::InvalidValue),
        );
    }
    if settings.layer >= Layer::Semantic {
        out.extend(validate::semantic(&parsed.mztab, config, terms).into_vec());
    }
    out
}

/// The canonical text of a model in `format`. With `lossy`, what the
/// specification does not define is dropped first (and counted), so that
/// JSON can carry the rest.
///
/// # Errors
///
/// [`Error::Write`] if the model cannot be written in that format.
pub fn write(mztab: &MzTab<'_>, format: Format, lossy: bool) -> Result<(String, Stripped), Error> {
    let mut model = mztab.clone();
    let stripped = if lossy {
        model.strip_unknown()
    } else {
        Stripped::default()
    };
    let text = match format {
        // Comments are layout: a TSV writer keeps them in place, a model
        // without layout is canonical (D-075).
        Format::Tsv => {
            model.layout = model.layout.comments_only();
            mztab_m::to_string(&model)
        }
        Format::Json => json::to_string(&model),
    };
    text.map(|t| (t, stripped)).map_err(Error::Write)
}

/// One finding as flat fields, for bindings and tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The code's id, e.g. `MZT-T009`.
    pub code: &'static str,
    /// `error`, `warning` or `info`.
    pub severity: &'static str,
    /// `syntax`, `structure` or `semantic`.
    pub layer: &'static str,
    /// The 1-based line, for a text location.
    pub line: Option<u32>,
    /// The 1-based column, for a text location.
    pub column: Option<u32>,
    /// The 1-based cell, for a text location that has one.
    pub cell: Option<u32>,
    /// The location as text: `line 3, column 5, cell 2`, a JSON pointer,
    /// a model path, or `document`.
    pub location: String,
    /// The code's fixed summary.
    pub summary: &'static str,
    /// The finding's detail, if any.
    pub detail: Option<String>,
    /// The URL of the rule's section in the pinned specification.
    pub spec: String,
}

/// The findings as records.
#[must_use]
pub fn records(findings: &Findings) -> Vec<Record> {
    findings
        .iter()
        .map(|f| {
            let (line, column, cell) = match f.location {
                Location::Text { line, column, cell } => (Some(line), Some(column), cell),
                _ => (None, None, None),
            };
            let detail = f.detail.to_string();
            Record {
                code: f.code.id(),
                severity: match f.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                },
                layer: match f.code.layer() {
                    mztab_m::Layer::Syntax => "syntax",
                    mztab_m::Layer::Structure => "structure",
                    mztab_m::Layer::Semantic => "semantic",
                },
                line,
                column,
                cell,
                location: f.location.to_string(),
                summary: f.code.summary(),
                detail: (!detail.is_empty()).then_some(detail),
                spec: f.code.spec_ref().to_string(),
            }
        })
        .collect()
}

/// Reads `bytes`, validates with `settings` and the OBO vocabularies in
/// `obo`, and returns the records: the one call a binding needs.
///
/// # Errors
///
/// None at present; the `Result` leaves room for input errors.
pub fn check(
    bytes: &[u8],
    format: Format,
    obo: &[&str],
    settings: &Settings,
) -> Result<Vec<Record>, Error> {
    let parsed = read(bytes, format);
    let vocabularies: Vec<Ontology<'_>> = obo.iter().map(|t| Ontology::parse(t)).collect();
    let terms: &[Ontology<'_>] = &vocabularies;
    Ok(records(&validate(&parsed, settings, &terms)))
}

/// Converts `bytes` (in `from`) to the canonical text in `to`; with
/// `lossy`, drops what the specification does not define first.
///
/// # Errors
///
/// [`Error::Write`] if the document cannot be written in `to`.
pub fn convert(bytes: &[u8], from: Format, to: Format, lossy: bool) -> Result<String, Error> {
    write(&read(bytes, from).mztab, to, lossy).map(|(text, _)| text)
}

/// A fact about a document, for `inspect`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// A text value, if present.
    Text(Option<String>),
    /// A count.
    Count(usize),
    /// A row count, or `None` if the table is absent.
    Rows(Option<usize>),
}

impl Fact {
    /// The fact as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> String {
        match self {
            Self::Text(None) | Self::Rows(None) => "null".to_owned(),
            Self::Text(Some(text)) => {
                let mut out = String::from("\"");
                for c in text.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        c if u32::from(c) < 0x20 => {
                            write!(out, "\\u{:04x}", u32::from(c)).unwrap_or_default();
                        }
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Self::Count(n) | Self::Rows(Some(n)) => n.to_string(),
        }
    }
}

impl fmt::Display for Fact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(None) | Self::Rows(None) => f.write_str("null"),
            Self::Text(Some(text)) => f.write_str(text),
            Self::Count(n) | Self::Rows(Some(n)) => write!(f, "{n}"),
        }
    }
}

/// What `inspect` reports about a document.
#[must_use]
pub fn facts(parsed: &Parsed<'_>) -> Vec<(&'static str, Fact)> {
    let m = &parsed.mztab;
    let meta = &m.metadata;
    let text = |c: Option<&mztab_m::Cell<'_, mztab_m::Text<'_>>>| {
        Fact::Text(c.and_then(|c| c.value()).map(|t| t.as_str().to_owned()))
    };
    let version = meta.version.as_ref().and_then(|c| c.value());
    let profile = meta.profile.as_ref().and_then(|c| c.value());
    let errors = parsed
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    Vec::from([
        ("version", Fact::Text(version.map(ToString::to_string))),
        ("id", text(meta.id.as_ref())),
        ("title", text(meta.title.as_ref())),
        ("profile", Fact::Text(profile.map(ToString::to_string))),
        (
            "tables_profile",
            Fact::Text(m.inferred_profile().ok().map(|p| p.to_string())),
        ),
        ("ms_runs", Fact::Count(meta.ms_run.len())),
        ("assays", Fact::Count(meta.assay.len())),
        ("study_variables", Fact::Count(meta.study_variable.len())),
        ("samples", Fact::Count(meta.sample.len())),
        ("cvs", Fact::Count(meta.cv.len())),
        ("sml_rows", Fact::Rows(m.sml.as_ref().map(|t| t.rows.len()))),
        ("smf_rows", Fact::Rows(m.smf.as_ref().map(|t| t.rows.len()))),
        ("sme_rows", Fact::Rows(m.sme.as_ref().map(|t| t.rows.len()))),
        ("reader_findings", Fact::Count(parsed.findings.len())),
        ("reader_errors", Fact::Count(errors)),
    ])
}
