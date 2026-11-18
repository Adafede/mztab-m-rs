//! The browser demo's functions: what the page in `www/` calls.
//!
//! Each takes text and returns text, so the page needs no glue beyond
//! `wasm-bindgen`'s. Natively they are plain functions, tested as such.

use mztab_m::validate::{self, Config};
use mztab_m::{Code, Findings, IntoOwned as _, Parsed, json};
use mztab_m_obo::Ontology;
use wasm_bindgen::prelude::wasm_bindgen;

/// Validates an mzTab-M document (TSV, or JSON if it starts with `{`)
/// against the vocabularies in `obo` (OBO text, may be empty) and returns
/// the findings as a JSON array (see `mztab_m::json::write_findings`).
#[wasm_bindgen]
#[must_use]
pub fn validate(input: &str, obo: &str) -> String {
    let parsed = read(input);
    let config = Config::default();
    let vocabulary = Ontology::parse(obo);
    let mut findings = Findings::new();
    findings.extend(parsed.findings.iter().cloned());
    findings.extend(
        validate::structure(&parsed.mztab, &config)
            .into_vec()
            .into_iter()
            .filter(|f| f.code != Code::InvalidValue),
    );
    findings.extend(validate::semantic(&parsed.mztab, &config, &vocabulary).into_vec());
    let mut out = String::new();
    // Writing into a `String` cannot fail.
    json::write_findings(&findings, &mut out).unwrap_or_default();
    out
}

/// Converts a document to `"json"` or `"tsv"`.
///
/// # Errors
///
/// Why the document cannot be written in that format.
#[wasm_bindgen]
pub fn convert(input: &str, to: &str) -> Result<String, String> {
    let mut model = read(input).mztab;
    model.layout = mztab_m::SourceLayout::default();
    let written = match to {
        "json" => json::to_string(&model),
        "tsv" => mztab_m::to_string(&model),
        other => return Err(format!("unknown format `{other}`")),
    };
    written.map_err(|e| e.to_string())
}

/// JSON if the text starts with `{`, TSV otherwise.
fn read(input: &str) -> Parsed<'static> {
    if input.trim_start().starts_with('{') {
        json::parse(input)
    } else {
        mztab_m::parse(input).into_owned()
    }
}
