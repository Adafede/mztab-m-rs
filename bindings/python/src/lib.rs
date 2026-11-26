//! The native part of the `mztab_m` Python package: thin wrappers over
//! `mztab-m-api`. Everything is plain data on the Python side: findings are
//! dicts, documents are strings.

use mztab_m_api::{Error, Fact, Format, Layer, SPEC_COMMIT, SPEC_VERSION, Settings};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyDict, PyMapping, PyString};

/// A Python `ValueError` for a bad argument or an unwritable document.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a function for `map_err`, which passes the error by value"
)]
fn error(e: Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// The bytes of a `str`, `bytes`, or `bytearray` argument.
fn bytes(text: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(s) = text.cast::<PyString>() {
        return s.extract::<String>().map(String::into_bytes);
    }
    if let Ok(b) = text.cast::<PyBytes>() {
        return Ok(b.as_bytes().to_vec());
    }
    if let Ok(array) = text.cast::<PyByteArray>() {
        return Ok(array.to_vec());
    }
    Err(PyTypeError::new_err("expected str, bytes, or bytearray"))
}

/// The format: given (`"tsv"` or `"json"`), or detected from the text.
fn format(given: Option<&str>, bytes: &[u8]) -> PyResult<Format> {
    given.map_or_else(
        || Ok(Format::sniff("", bytes)),
        |f| f.parse::<Format>().map_err(error),
    )
}

/// Validates a document and returns its findings, as dicts with `code`,
/// `severity`, `layer`, `line`, `column`, `cell`, `location`, `summary`,
/// `detail` and `spec`.
///
/// `vocabularies` are OBO texts (such as the contents of `psi-ms.obo`) for
/// the semantic layer; `layer` is the last layer to run (`"syntax"`,
/// `"structure"` or `"semantic"`); `severities` maps codes to `"error"`,
/// `"warning"`, `"info"` or `"off"`.
#[pyfunction]
#[pyo3(signature = (text, *, vocabularies = None, layer = "semantic", severities = None, format = None))]
fn validate<'py>(
    py: Python<'py>,
    text: &Bound<'py, PyAny>,
    vocabularies: Option<&Bound<'py, PyAny>>,
    layer: &str,
    severities: Option<&Bound<'py, PyMapping>>,
    format: Option<&str>,
) -> PyResult<Vec<Bound<'py, PyDict>>> {
    let bytes = bytes(text)?;
    let format = self::format(format, &bytes)?;
    let parsed_layer = layer
        .parse::<Layer>()
        .map_err(|_| error(Error::Layer(layer.to_owned())))?;
    let mut settings = Settings::default().layer(parsed_layer);
    if let Some(map) = severities {
        for item in map.items()?.iter() {
            let (code, level) = item.extract::<(String, String)>()?;
            settings = settings.set_code(&code, &level).map_err(error)?;
        }
    }
    let obos: Vec<String> = match vocabularies {
        None => Vec::new(),
        Some(v) => {
            if v.is_instance_of::<PyString>()
                || v.is_instance_of::<PyBytes>()
                || v.is_instance_of::<PyByteArray>()
            {
                return Err(PyTypeError::new_err(
                    "`vocabularies` must be a sequence of OBO strings, not str or bytes",
                ));
            }
            v.extract::<Vec<String>>()?
        }
    };
    let obo: Vec<&str> = obos.iter().map(String::as_str).collect();
    let records = mztab_m_api::check(&bytes, format, &obo, &settings);
    records
        .iter()
        .map(|r| {
            let d = PyDict::new(py);
            d.set_item("code", r.code)?;
            d.set_item("severity", r.severity)?;
            d.set_item("layer", r.layer)?;
            d.set_item("line", r.line)?;
            d.set_item("column", r.column)?;
            d.set_item("cell", r.cell)?;
            d.set_item("location", &r.location)?;
            d.set_item("summary", r.summary)?;
            d.set_item("detail", r.detail.as_deref())?;
            d.set_item("spec", &r.spec)?;
            Ok(d)
        })
        .collect()
}

/// Converts a document to `"tsv"` or `"json"`. With `lossy`, what the
/// specification does not define is dropped instead of refused.
#[pyfunction]
#[pyo3(signature = (text, to, *, lossy = false, format = None))]
fn convert(
    text: &Bound<'_, PyAny>,
    to: &str,
    lossy: bool,
    format: Option<&str>,
) -> PyResult<String> {
    let bytes = bytes(text)?;
    let from = self::format(format, &bytes)?;
    let to = to.parse::<Format>().map_err(error)?;
    mztab_m_api::convert(&bytes, from, to, lossy).map_err(error)
}

/// The canonical form of a document, in its own format.
#[pyfunction]
#[pyo3(signature = (text, *, format = None))]
fn canonical(text: &Bound<'_, PyAny>, format: Option<&str>) -> PyResult<String> {
    let bytes = bytes(text)?;
    let format = self::format(format, &bytes)?;
    mztab_m_api::convert(&bytes, format, format, false).map_err(error)
}

/// A summary of a document: version, profile, element counts, rows per
/// table and the reader's finding counts.
#[pyfunction]
#[pyo3(signature = (text, *, format = None))]
fn inspect<'py>(
    py: Python<'py>,
    text: &Bound<'py, PyAny>,
    format: Option<&str>,
) -> PyResult<Bound<'py, PyDict>> {
    let bytes = bytes(text)?;
    let format = self::format(format, &bytes)?;
    let parsed = mztab_m_api::read(&bytes, format);
    let d = PyDict::new(py);
    for (key, fact) in mztab_m_api::facts(&parsed) {
        match fact {
            Fact::Text(text) => d.set_item(key, text)?,
            Fact::Count(n) => d.set_item(key, n)?,
            Fact::Rows(n) => d.set_item(key, n)?,
        }
    }
    Ok(d)
}

/// The finding codes: dicts with `code`, `layer`, `requirement`,
/// `summary` and `spec`.
#[pyfunction]
fn codes(py: Python<'_>) -> PyResult<Vec<Bound<'_, PyDict>>> {
    mztab_m_api::codes()
        .into_iter()
        .map(|c| {
            let d = PyDict::new(py);
            d.set_item("code", c.code)?;
            d.set_item("layer", c.layer)?;
            d.set_item("requirement", c.requirement)?;
            d.set_item("summary", c.summary)?;
            d.set_item("spec", c.spec)?;
            Ok(d)
        })
        .collect()
}

/// Checks that an OBO text parses into terms (returns how many).
#[pyfunction]
fn vocabulary_size(text: &str) -> usize {
    mztab_m_api::vocabulary_size(text)
}

/// The `mztab_m._native` extension module.
#[pymodule]
#[pyo3(name = "_native")]
fn native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(canonical, m)?)?;
    m.add_function(wrap_pyfunction!(inspect, m)?)?;
    m.add_function(wrap_pyfunction!(codes, m)?)?;
    m.add_function(wrap_pyfunction!(vocabulary_size, m)?)?;
    m.add("SPEC_COMMIT", SPEC_COMMIT)?;
    m.add("SPEC_VERSION", SPEC_VERSION)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
