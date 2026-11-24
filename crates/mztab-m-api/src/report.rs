//! Flat records for findings, code catalog metadata, and document facts.

use alloc::borrow::ToOwned as _;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use mztab_m::{Code, Findings, Location, Parsed, Severity, json};

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

/// One registered finding code as flat fields, for bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRecord {
    /// The stable code id, e.g. `MZT-S001`.
    pub code: &'static str,
    /// `syntax`, `structure` or `semantic`.
    pub layer: &'static str,
    /// `MUST`, `SHOULD` or `MAY`.
    pub requirement: &'static str,
    /// Fixed English summary of the rule.
    pub summary: &'static str,
    /// URL of the rule's section in the pinned specification.
    pub spec: String,
}

/// All registered finding codes in canonical id order.
#[must_use]
pub fn codes() -> Vec<CodeRecord> {
    Code::ALL
        .iter()
        .map(|c| CodeRecord {
            code: c.id(),
            layer: c.layer().as_str(),
            requirement: c.requirement().as_str(),
            summary: c.summary(),
            spec: c.spec_ref().to_string(),
        })
        .collect()
}

/// The findings as records.
#[must_use]
pub(crate) fn records(findings: &Findings) -> Vec<Record> {
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
                severity: f.severity.as_str(),
                layer: f.code.layer().as_str(),
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
    fn to_json(&self) -> String {
        match self {
            Self::Text(None) | Self::Rows(None) => "null".to_owned(),
            Self::Text(Some(text)) => {
                let mut out = String::new();
                // Writing into a `String` cannot fail.
                json::write_string(&mut out, text).unwrap_or_default();
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

/// Formats `findings` as a compact JSON array of diagnostic objects.
#[must_use]
pub fn findings_json(findings: &Findings) -> String {
    let mut out = String::new();
    // Writing into a `String` cannot fail.
    json::write_findings(findings, &mut out).unwrap_or_default();
    out
}

/// Formats `inspect` facts as a compact JSON object.
#[must_use]
pub fn facts_json(parsed: &Parsed<'_>) -> String {
    let mut out = String::from("{");
    for (i, (key, value)) in facts(parsed).iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(key);
        out.push_str("\":");
        out.push_str(&value.to_json());
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::records;
    use mztab_m::validate;
    use mztab_m::{Cell, MzTab, Raw};

    #[test]
    fn records_flatten_model_path_locations() {
        let mut model = MzTab::default();
        model.metadata.title = Some(Cell::Invalid(Raw::new("x")));
        let findings = validate::structure(&model, &validate::Config::default());
        assert!(
            records(&findings)
                .iter()
                .any(|r| r.location == "metadata.title"),
            "model path"
        );
    }
}
