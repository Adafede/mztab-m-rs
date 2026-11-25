//! Value coercion from JSON AST nodes into TSV cell/metadata strings.

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::{Tsv, items, shape_error};
use crate::diag::location::Path;
use crate::json::schema::{Col, Shape};
use crate::json::value::{Node, Value};
use crate::param::Param;
use crate::param::text::{Accession, ParamText};

/// The TSV text of a metadata value.
pub(super) fn text(node: &Node<'_>, shape: Shape, at: &Path, tsv: &mut Tsv) -> Option<String> {
    match (&node.value, shape) {
        // JSON `null` is an absent key; the string `"null"` is a `null` value.
        (Value::Null, _) => None,
        (Value::String(s), _) => Some(s.to_string()),
        (Value::Number(n), Shape::Text | Shape::Strings) => Some(plain(n)),
        (Value::Number(n), Shape::Ref(element)) => Some(format!("{element}[{n}]")),
        (Value::Object(_), Shape::Param | Shape::Indexed) => param_text(node, at, tsv),
        (Value::Array(_), Shape::Params) => joined(node, at, tsv, param_text),
        (Value::Array(_), Shape::Refs(element)) => {
            joined(node, at, tsv, |item, here, tsv| match &item.value {
                Value::Number(n) => Some(format!("{element}[{n}]")),
                _ => shape_error(tsv, here, "an integer"),
            })
        }
        (Value::Array(_), Shape::Publication) => joined(node, at, tsv, |item, here, tsv| {
            let field = |key: &str| match item.get(key).map(|n| &n.value) {
                Some(Value::String(s)) => Some(s.to_string()),
                _ => None,
            };
            match (field("type"), field("accession")) {
                (Some(kind), Some(accession)) => Some(format!("{kind}:{accession}")),
                _ => shape_error(tsv, here, "a publication item"),
            }
        }),
        _ => shape_error(tsv, at, expected(shape)),
    }
}

const fn expected(shape: Shape) -> &'static str {
    match shape {
        Shape::Text | Shape::Strings => "a string",
        Shape::Param | Shape::Indexed => "a parameter",
        Shape::Params => "an array of parameters",
        Shape::Ref(_) => "an integer",
        Shape::Refs(_) => "an array of integers",
        Shape::Publication => "an array of publication items",
    }
}

/// Items joined with `|`; an empty array is `null`.
pub(super) fn joined(
    node: &Node<'_>,
    at: &Path,
    tsv: &mut Tsv,
    mut item: impl FnMut(&Node<'_>, &Path, &mut Tsv) -> Option<String>,
) -> Option<String> {
    let parts: Vec<Option<String>> = items(node, at, tsv)
        .into_iter()
        .map(|(_, node, here)| match &node.value {
            Value::Null => Some("null".to_string()),
            Value::String(s) => Some(s.to_string()),
            _ => item(node, &here, tsv),
        })
        .collect();
    if parts.is_empty() {
        return Some("null".to_string());
    }
    let parts: Option<Vec<String>> = parts.into_iter().collect();
    parts.map(|p| p.join("|"))
}

/// The TSV text of a `Parameter` object.
pub(super) fn param_text(node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Option<String> {
    let field = |key: &str| -> Result<Option<&str>, ()> {
        match node.get(key).map(|n| &n.value) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) if s.is_empty() => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(Value::Number(n)) => Ok(Some(n)),
            Some(_) => Err(()),
        }
    };
    let (Ok(label), Ok(accession), Ok(Some(name)), Ok(value)) = (
        field("cv_label"),
        field("cv_accession"),
        field("name"),
        field("value"),
    ) else {
        return shape_error(tsv, at, "a parameter");
    };
    let built = (|| {
        let value = value.map(ParamText::new).transpose().ok()?;
        let name = ParamText::new(name).ok()?;
        Some(match (label, accession) {
            (None, None) => Param::User { name, value },
            (Some(label), Some(accession)) => Param::Cv {
                label: ParamText::new(label).ok()?,
                accession: Accession::new(ParamText::new(accession).ok()?),
                name,
                value,
            },
            _ => return None,
        })
    })();
    built.map_or_else(
        || shape_error(tsv, at, "a parameter"),
        |param| Some(param.to_string()),
    )
}

pub(super) fn colunit_text(node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Option<String> {
    let column = match node.get("column_name").map(|n| &n.value) {
        Some(Value::String(s)) => s.to_string(),
        _ => return shape_error(tsv, at, "a column unit mapping"),
    };
    let param = node.get("param")?;
    let unit = param_text(param, &at.clone().key("param"), tsv)?;
    Some(format!("{column}={unit}"))
}

/// A JSON number in plain decimal notation (mzTab forbids exponents): the
/// digits of `1.5e3` are moved to `1500`, exactly.
pub(super) fn plain(lexeme: &str) -> String {
    let Some((mantissa, exponent)) = lexeme.split_once(['e', 'E']) else {
        return lexeme.to_string();
    };
    let Ok(exponent) = exponent.parse::<i64>() else {
        return lexeme.to_string();
    };
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |m| ("-", m));
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{int}{frac}");
    let point = i64::try_from(int.len())
        .unwrap_or(i64::MAX)
        .saturating_add(exponent);
    let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    // Bound the padding: a pathological exponent stays as written.
    if !(-1000..=1000).contains(&point) {
        return lexeme.to_string();
    }
    let (int, frac) = if point <= 0 {
        let zeros = "0".repeat(usize::try_from(-point).unwrap_or(0));
        ("0".to_string(), format!("{zeros}{digits}"))
    } else if point >= len {
        let zeros = "0".repeat(usize::try_from(point - len).unwrap_or(0));
        (format!("{digits}{zeros}"), String::new())
    } else {
        let at = usize::try_from(point).unwrap_or(0);
        let (i, f) = digits.split_at(at);
        (i.to_string(), f.to_string())
    };
    let int = int.trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    if frac.is_empty() {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    }
}

/// The TSV text of a table cell.
pub(super) fn cell_text(node: &Node<'_>, col: Col, at: &Path, tsv: &mut Tsv) -> Option<String> {
    match (&node.value, col) {
        (Value::Null, _) => Some("null".to_string()),
        (Value::String(s), _) => Some(s.to_string()),
        (Value::Number(n), Col::Text | Col::Number | Col::Integer) => Some(plain(n)),
        (Value::Object(_), Col::Param) => param_text(node, at, tsv),
        (Value::Array(_), Col::Texts) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Text, here, tsv)
        }),
        (Value::Array(_), Col::Numbers) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Number, here, tsv)
        }),
        (Value::Array(_), Col::Integers) => joined(node, at, tsv, |item, here, tsv| {
            cell_text(item, Col::Integer, here, tsv)
        }),
        (Value::Array(_), Col::SpectraRefs) => joined(node, at, tsv, |item, here, tsv| {
            match (
                item.get("ms_run").map(|n| &n.value),
                item.get("reference").map(|n| &n.value),
            ) {
                (Some(Value::Number(run)), Some(Value::String(reference))) => {
                    Some(format!("ms_run[{run}]:{reference}"))
                }
                _ => shape_error(tsv, here, "a spectra reference"),
            }
        }),
        _ => shape_error(tsv, at, col_expected(col)),
    }
}

const fn col_expected(col: Col) -> &'static str {
    match col {
        Col::Text => "a string",
        Col::Number => "a number",
        Col::Integer => "an integer",
        Col::Param => "a parameter",
        Col::Texts => "an array of strings",
        Col::Numbers => "an array of numbers",
        Col::Integers => "an array of integers",
        Col::SpectraRefs => "an array of spectra references",
    }
}
