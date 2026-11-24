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

/// The format: `"tsv"`, `"json"`, or `""` to detect it from the text.
fn format(given: &str, text: &str) -> Result<Format> {
    if given.is_empty() {
        Ok(Format::sniff("", text.as_bytes()))
    } else {
        given
            .parse::<Format>()
            .map_err(|e| Error::Other(e.to_string()))
    }
}

/// Text or NA.
fn text(values: impl Iterator<Item = Option<String>>) -> Strings {
    values
        .map(|v| v.map_or_else(Rstr::na, |s| Rstr::from(s.as_str())))
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
    text: &str,
    vocabularies: Strings,
    layer: &str,
    severities: Strings,
    format: &str,
) -> List {
    outcome(validate(text, &vocabularies, layer, &severities, format))
}

fn validate(
    text: &str,
    vocabularies: &Strings,
    layer: &str,
    severities: &Strings,
    format: &str,
) -> Result<List> {
    let format = self::format(format, text)?;
    let parsed_layer = layer
        .parse::<Layer>()
        .map_err(|_| Error::Other(mztab_m_api::Error::Layer(layer.to_owned()).to_string()))?;
    let mut settings = Settings::default().layer(parsed_layer);
    for setting in severities.iter() {
        settings = settings
            .set(setting.as_ref())
            .map_err(|e| Error::Other(e.to_string()))?;
    }
    let obo: Vec<&str> = vocabularies.iter().map(AsRef::as_ref).collect();
    let records = mztab_m_api::check(text.as_bytes(), format, &obo, &settings);
    let column = |f: fn(&mztab_m_api::Record) -> Option<String>| text_column(&records, f);
    Ok(list!(
        code = column(|r| Some(r.code.to_owned())),
        severity = column(|r| Some(r.severity.to_owned())),
        layer = column(|r| Some(r.layer.to_owned())),
        line = integers(records.iter().map(|r| r.line)),
        column = integers(records.iter().map(|r| r.column)),
        cell = integers(records.iter().map(|r| r.cell)),
        location = column(|r| Some(r.location.clone())),
        summary = column(|r| Some(r.summary.to_owned())),
        detail = column(|r| r.detail.clone()),
        spec = column(|r| Some(r.spec.clone()))
    ))
}

fn text_column(
    records: &[mztab_m_api::Record],
    f: fn(&mztab_m_api::Record) -> Option<String>,
) -> Strings {
    text(records.iter().map(f))
}

/// `text` converted to `to`; with `lossy`, unknown content is dropped.
#[extendr]
fn mztabm_convert(text: &str, to: &str, lossy: bool, format: &str) -> List {
    outcome(convert(text, to, lossy, format))
}

fn convert(text: &str, to: &str, lossy: bool, format: &str) -> Result<String> {
    let from = self::format(format, text)?;
    let to = to
        .parse::<Format>()
        .map_err(|e| Error::Other(e.to_string()))?;
    mztab_m_api::convert(text.as_bytes(), from, to, lossy).map_err(|e| Error::Other(e.to_string()))
}

/// The canonical form of `text`, in its own format.
#[extendr]
fn mztabm_canonical(text: &str, format: &str) -> List {
    outcome(canonical(text, format))
}

fn canonical(text: &str, format: &str) -> Result<String> {
    let format = self::format(format, text)?;
    mztab_m_api::convert(text.as_bytes(), format, format, false)
        .map_err(|e| Error::Other(e.to_string()))
}

/// A summary of `text`, as a named list.
#[extendr]
fn mztabm_inspect(text: &str, format: &str) -> List {
    outcome(inspect(text, format))
}

fn inspect(text: &str, format: &str) -> Result<List> {
    let format = self::format(format, text)?;
    let parsed = mztab_m_api::read(text.as_bytes(), format);
    let facts = mztab_m_api::facts(&parsed);
    let names: Vec<&str> = facts.iter().map(|(k, _)| *k).collect();
    let values: Vec<Robj> = facts
        .into_iter()
        .map(|(_, fact)| match fact {
            Fact::Text(t) => t.map_or_else(|| Robj::from(Rstr::na()), Robj::from),
            Fact::Count(n) => Robj::from(i32::try_from(n).unwrap_or(i32::MAX)),
            Fact::Rows(n) => n.map_or_else(
                || Robj::from(Rint::na()),
                |n| Robj::from(i32::try_from(n).unwrap_or(i32::MAX)),
            ),
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

extendr_module! {
    mod mztabm;
    fn mztabm_validate;
    fn mztabm_convert;
    fn mztabm_canonical;
    fn mztabm_inspect;
    fn mztabm_codes;
    fn mztabm_spec_commit;
}
