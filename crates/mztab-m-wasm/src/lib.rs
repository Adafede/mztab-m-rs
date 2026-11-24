//! The browser demo's functions: what the page in `www/` calls.
//!
//! Each takes text and returns text, so the page needs no glue beyond
//! `wasm-bindgen`'s. Natively they are plain functions, tested as such.

use mztab_m_api::{Format, Settings};
use wasm_bindgen::prelude::wasm_bindgen;

/// JSON if the text starts with `{`, TSV otherwise.
fn format(input: &str) -> Format {
    Format::sniff("", input.as_bytes())
}

/// Validates an mzTab-M document and returns findings as a JSON array.
///
/// Accepts TSV (or JSON if it starts with `{`) and checks terms against the
/// OBO vocabularies in `obos` (one file text per entry); see
/// `mztab_m_api::validate_json` / `findings_json` for the JSON shape.
#[wasm_bindgen]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen passes JS string arrays by value"
)]
pub fn validate(input: &str, obos: Vec<String>) -> String {
    let obo: Vec<&str> = obos.iter().map(String::as_str).collect();
    mztab_m_api::validate_json(input.as_bytes(), format(input), &obo, &Settings::default())
}

/// Converts a document to `"json"` or `"tsv"`.
///
/// # Errors
///
/// Why the document cannot be written in that format.
#[wasm_bindgen]
pub fn convert(input: &str, to: &str) -> Result<String, String> {
    Format::parse(to)
        .and_then(|to| mztab_m_api::convert(input.as_bytes(), format(input), to, false))
        .map_err(|e| e.to_string())
}
