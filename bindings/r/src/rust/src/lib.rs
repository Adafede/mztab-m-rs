//! The native part of the `mztabm` R package: thin wrappers over
//! `mztab-m-api`. The R functions in `R/mztabm.R` add defaults and turn the
//! columns into data frames.
#![expect(
    clippy::needless_pass_by_value,
    clippy::used_underscore_binding,
    reason = "`#[extendr]` passes R vectors by value and names bindings with underscores"
)]

use extendr_api::prelude::*;
use mztab_m_api::{Fact, Format, Layer, Settings};

/// A result; the R wrapper turns its error into an R error.
type Result<T> = std::result::Result<T, Error>;

/// `list(value = ..., error = NULL)` or `list(value = NULL, error = "...")`:
/// an `Err` returned through extendr would print a Rust panic before the R
/// error, so the R side raises it instead (`R/extendr-wrappers.R`).
fn outcome<T: Into<Robj>>(result: Result<T>) -> List {
    match result {
        Ok(value) => list!(value = value.into(), error = NULL),
        Err(e) => list!(value = NULL, error = e.to_string()),
    }
}

fn api_err(e: impl std::fmt::Display) -> Error {
    Error::Other(e.to_string())
}

/// Extracts UTF-8 string or raw bytes directly, preserving invalid UTF-8 and NUL.
fn extract_bytes(text: &Robj) -> Result<&[u8]> {
    if let Some(raw) = text.as_raw_slice() {
        return Ok(raw);
    }
    if let Some(s) = text.as_str() {
        return Ok(s.as_bytes());
    }
    Err(Error::Other(
        "`text` must be one non-NA character string or a raw vector".to_owned(),
    ))
}

/// The format: `"tsv"`, `"json"`, or `NULL` to detect it from the bytes.
fn format(given: Nullable<&str>, bytes: &[u8]) -> Result<Format> {
    match given {
        Nullable::Null => Ok(Format::sniff("", bytes)),
        Nullable::NotNull(s) => s.parse::<Format>().map_err(api_err),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "R double numeric represents integer counts above i32::MAX up to 2^53 exactly"
)]
fn count_robj(n: usize) -> Robj {
    i32::try_from(n).map_or_else(|_| Robj::from(n as f64), Robj::from)
}

/// Text or NA.
fn text<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Strings {
    values
        .map(|v| v.map_or_else(Rstr::na, Rstr::from))
        .collect()
}

/// Integers or NA.
fn integers(values: impl Iterator<Item = Option<u32>>) -> Integers {
    values
        .map(|v| {
            v.and_then(|n| i32::try_from(n).ok())
                .map_or_else(Rint::na, Rint::from)
        })
        .collect()
}

/// Findings of `text`, as named columns.
#[extendr]
fn mztabm_validate(
    text: Robj,
    vocabularies: Strings,
    layer: &str,
    severity_codes: Strings,
    severity_levels: Strings,
    format: Nullable<&str>,
) -> List {
    outcome(validate(
        &text,
        &vocabularies,
        layer,
        &severity_codes,
        &severity_levels,
        format,
    ))
}

fn validate(
    text: &Robj,
    vocabularies: &Strings,
    layer: &str,
    severity_codes: &Strings,
    severity_levels: &Strings,
    format: Nullable<&str>,
) -> Result<List> {
    let bytes = extract_bytes(text)?;
    let format = self::format(format, bytes)?;
    let parsed_layer = layer
        .parse::<Layer>()
        .map_err(|_| Error::Other(mztab_m_api::Error::Layer(layer.to_owned()).to_string()))?;
    if severity_codes.len() != severity_levels.len() {
        return Err(Error::Other(
            "`severity_codes` and `severity_levels` must have the same length".to_owned(),
        ));
    }
    let mut settings = Settings::default().layer(parsed_layer);
    for (code, level) in severity_codes.iter().zip(severity_levels.iter()) {
        if code.is_na() || level.is_na() || code.as_ref().is_empty() {
            return Err(Error::Other(
                "`severities` must have non-NA, non-empty names and non-NA values".to_owned(),
            ));
        }
        settings = settings
            .set_code(code.as_ref(), level.as_ref())
            .map_err(api_err)?;
    }
    let mut obo = Vec::with_capacity(vocabularies.len());
    for entry in vocabularies.iter() {
        if entry.is_na() {
            return Err(Error::Other(
                "`vocabularies` must not contain NA".to_owned(),
            ));
        }
        obo.push(entry.as_ref());
    }
    let records = mztab_m_api::check(bytes, format, &obo, &settings);
    let column = |f: fn(&mztab_m_api::Record) -> Option<&str>| text_column(&records, f);
    Ok(list!(
        code = column(|r| Some(r.code)),
        severity = column(|r| Some(r.severity)),
        layer = column(|r| Some(r.layer)),
        line = integers(records.iter().map(|r| r.line)),
        column = integers(records.iter().map(|r| r.column)),
        cell = integers(records.iter().map(|r| r.cell)),
        location = column(|r| Some(r.location.as_str())),
        summary = column(|r| Some(r.summary)),
        detail = column(|r| r.detail.as_deref()),
        spec = column(|r| Some(r.spec.as_str()))
    ))
}

fn text_column(
    records: &[mztab_m_api::Record],
    f: fn(&mztab_m_api::Record) -> Option<&str>,
) -> Strings {
    text(records.iter().map(f))
}

/// `text` converted to `to`; with `lossy`, unknown content is dropped.
#[extendr]
fn mztabm_convert(text: Robj, to: &str, lossy: bool, format: Nullable<&str>) -> List {
    outcome(convert(&text, to, lossy, format))
}

fn convert(text: &Robj, to: &str, lossy: bool, format: Nullable<&str>) -> Result<String> {
    let bytes = extract_bytes(text)?;
    let from = self::format(format, bytes)?;
    let to = to.parse::<Format>().map_err(api_err)?;
    mztab_m_api::convert(bytes, from, to, lossy).map_err(api_err)
}

/// The canonical form of `text`, in its own format.
#[extendr]
fn mztabm_canonical(text: Robj, format: Nullable<&str>) -> List {
    outcome(canonical(&text, format))
}

fn canonical(text: &Robj, format: Nullable<&str>) -> Result<String> {
    let bytes = extract_bytes(text)?;
    let format = self::format(format, bytes)?;
    mztab_m_api::convert(bytes, format, format, false).map_err(api_err)
}

/// A summary of `text`, as a named list.
#[extendr]
fn mztabm_inspect(text: Robj, format: Nullable<&str>) -> List {
    outcome(inspect(&text, format))
}

fn inspect(text: &Robj, format: Nullable<&str>) -> Result<List> {
    let bytes = extract_bytes(text)?;
    let format = self::format(format, bytes)?;
    let parsed = mztab_m_api::read_borrowed(bytes, format);
    let facts = mztab_m_api::facts(&parsed);
    let names: Vec<&str> = facts.iter().map(|(k, _)| *k).collect();
    let values: Vec<Robj> = facts
        .into_iter()
        .map(|(_, fact)| match fact {
            Fact::Text(t) => t.map_or_else(|| Robj::from(Rstr::na()), Robj::from),
            Fact::Count(n) => count_robj(n),
            Fact::Rows(n) => n.map_or_else(|| Robj::from(Rint::na()), count_robj),
        })
        .collect();
    List::from_names_and_values(names, values)
}

/// The finding codes, as named columns.
#[extendr]
fn mztabm_codes() -> List {
    let all = mztab_m_api::codes();
    let col = |f: fn(&mztab_m_api::CodeRecord) -> &str| -> Strings {
        all.iter().map(|c| Rstr::from(f(c))).collect()
    };
    list!(
        code = col(|c| c.code),
        layer = col(|c| c.layer),
        requirement = col(|c| c.requirement),
        summary = col(|c| c.summary),
        spec = col(|c| c.spec.as_str())
    )
}

/// The pinned specification commit.
#[extendr]
fn mztabm_spec_commit() -> &'static str {
    mztab_m_api::SPEC_COMMIT
}

/// The pinned specification version.
#[extendr]
fn mztabm_spec_version() -> &'static str {
    mztab_m_api::SPEC_VERSION
}

/// Number of unique terms parsed from OBO 1.4 `text`.
#[extendr]
fn mztabm_vocabulary_size(text: Robj) -> List {
    outcome(vocabulary_size(&text))
}

fn vocabulary_size(text: &Robj) -> Result<Robj> {
    let Some(s) = text.as_str() else {
        return Err(Error::Other(
            "`text` must be one non-NA character string".to_owned(),
        ));
    };
    Ok(count_robj(mztab_m_api::vocabulary_size(s)))
}

extendr_module! {
    mod mztabm;
    fn mztabm_validate;
    fn mztabm_convert;
    fn mztabm_canonical;
    fn mztabm_inspect;
    fn mztabm_codes;
    fn mztabm_spec_commit;
    fn mztabm_spec_version;
    fn mztabm_vocabulary_size;
}
