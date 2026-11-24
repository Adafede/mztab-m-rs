//! High-level read, validate, convert, and vocabulary operations.

use alloc::string::String;
use alloc::vec::Vec;

use mztab_m::cv::TermLookup;
use mztab_m::validate;
use mztab_m::{Code, Findings, IntoOwned as _, MzTab, Parsed, Stripped, json};
use mztab_m_obo::Ontology;

use super::{Error, Format, Layer, Record, Settings, findings_json, records};

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

/// Validates a parsed document against raw OBO 1.4 vocabulary texts.
#[must_use]
pub fn validate_obo(parsed: &Parsed<'_>, settings: &Settings, obo: &[&str]) -> Findings {
    let vocabularies: Vec<Ontology<'_>> = obo.iter().map(|t| Ontology::parse(t)).collect();
    let terms: &[Ontology<'_>] = &vocabularies;
    validate(parsed, settings, &terms)
}

/// Reads and validates `bytes`, formatting findings as a JSON array.
#[must_use]
pub fn validate_json(bytes: &[u8], format: Format, obo: &[&str], settings: &Settings) -> String {
    let parsed = read(bytes, format);
    findings_json(&validate_obo(&parsed, settings, obo))
}

/// The canonical text of a model in `format`. With `lossy`, what the
/// specification does not define is dropped first (and counted), so that
/// JSON can carry the rest.
///
/// # Errors
///
/// [`Error::Write`] if the model cannot be written in that format.
pub fn write(mztab: &MzTab<'_>, format: Format, lossy: bool) -> Result<(String, Stripped), Error> {
    if lossy {
        let mut model = mztab.clone();
        let stripped = model.strip_unknown();
        write_canonical(&model, format).map(|t| (t, stripped))
    } else {
        write_canonical(mztab, format).map(|t| (t, Stripped::default()))
    }
}

fn write_canonical(mztab: &MzTab<'_>, format: Format) -> Result<String, Error> {
    let text = match format {
        // Comments are layout: canonical TSV keeps COM lines, JSON encodes them (D-075).
        Format::Tsv => mztab_m::to_canonical_string(mztab, true),
        Format::Json => json::to_string(mztab),
    };
    text.map_err(Error::Write)
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
    Ok(records(&validate_obo(&parsed, settings, obo)))
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

/// Parses an OBO 1.4 text and returns the number of unique terms it defines.
#[must_use]
pub fn vocabulary_size(text: &str) -> usize {
    Ontology::parse(text).len()
}
