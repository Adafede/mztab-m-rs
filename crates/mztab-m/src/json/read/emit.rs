//! Value coercion from JSON AST nodes into TSV cell/metadata strings.

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use super::{Tsv, items, shape_error};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
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
        (Value::Number(n), Shape::Ref(element)) => {
            let id = plain_int(n).unwrap_or_else(|| (*n).to_string());
            Some(format!("{element}[{id}]"))
        }
        (Value::Object(_), Shape::Param | Shape::Indexed | Shape::TextOrParam) => {
            param_text(node, at, tsv)
        }
        (Value::Array(_), Shape::Params) => joined(node, at, tsv, None, false, param_text),
        (Value::Array(_), Shape::Refs(element)) => {
            joined(node, at, tsv, None, true, |item, here, tsv| {
                match &item.value {
                    Value::Number(n) => {
                        let id = plain_int(n).unwrap_or_else(|| (*n).to_string());
                        Some(format!("{element}[{id}]"))
                    }
                    _ => shape_error(tsv, here, "an integer"),
                }
            })
        }
        (Value::Array(_), Shape::Publication) => {
            joined(node, at, tsv, None, true, |item, here, tsv| {
                let field = |key: &str| match item.get(key).map(|n| &n.value) {
                    Some(Value::String(s)) => Some(s.to_string()),
                    _ => None,
                };
                match (field("type"), field("accession")) {
                    (Some(kind), Some(accession)) => Some(format!("{kind}:{accession}")),
                    _ => shape_error(tsv, here, "a publication item"),
                }
            })
        }
        _ => shape_error(tsv, at, expected(shape)),
    }
}

const fn expected(shape: Shape) -> &'static str {
    match shape {
        Shape::Text | Shape::Strings => "a string",
        Shape::TextOrParam => "a string or a parameter",
        Shape::Param | Shape::Indexed => "a parameter",
        Shape::Params => "an array of parameters",
        Shape::Ref(_) => "an integer",
        Shape::Refs(_) => "an array of integers",
        Shape::Publication => "an array of publication items",
    }
}

/// Items joined with `|`; an empty array uses `empty`.
pub(super) fn joined(
    node: &Node<'_>,
    at: &Path,
    tsv: &mut Tsv,
    empty: Option<&str>,
    check_pipe: bool,
    mut item: impl FnMut(&Node<'_>, &Path, &mut Tsv) -> Option<String>,
) -> Option<String> {
    let raw_items = items(node, at, tsv);
    if raw_items.is_empty() {
        return empty.map(str::to_string);
    }
    let single = raw_items.len() == 1;
    let parts: Vec<Option<String>> = raw_items
        .into_iter()
        .map(|(_, node, here)| {
            let text = match &node.value {
                Value::Null => Some("null".to_string()),
                Value::String(s) => {
                    if single && s == "null" {
                        tsv.report(Code::JsonText, &here, Detail::None);
                        return None;
                    }
                    Some(s.to_string())
                }
                _ => item(node, &here, tsv),
            }?;
            if check_pipe && text.contains('|') {
                tsv.report(Code::JsonText, &here, Detail::None);
                return None;
            }
            Some(text)
        })
        .collect();
    let parts: Option<Vec<String>> = parts.into_iter().collect();
    parts.map(|p| p.join("|"))
}

/// The TSV text of a `Parameter` object.
pub(super) fn param_text(node: &Node<'_>, at: &Path, tsv: &mut Tsv) -> Option<String> {
    let str_field = |key: &str| -> Result<Option<&str>, ()> {
        match node.get(key).map(|n| &n.value) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) if s.is_empty() => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(_) => Err(()),
        }
    };
    let value = match node.get("value").map(|n| &n.value) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_ref())),
        Some(Value::Number(n)) => Ok(Some(*n)),
        Some(_) => Err(()),
    };
    let (Ok(label), Ok(accession), Ok(Some(name)), Ok(value)) = (
        str_field("cv_label"),
        str_field("cv_accession"),
        str_field("name"),
        value,
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
    let param_at = at.clone().key("param");
    let Some(param) = node.get("param") else {
        return shape_error(tsv, &param_at, "a parameter");
    };
    let unit = param_text(param, &param_at, tsv)?;
    Some(format!("{column}={unit}"))
}

/// Parses a JSON number as a positive `u32` integer (accepting `1.0`, `1e1`, `1.50e1`).
pub(super) fn parse_positive_int(lexeme: &str) -> Option<u32> {
    let int = plain_int(lexeme)?;
    let n = int.parse::<u32>().ok()?;
    (n > 0).then_some(n)
}

/// Converts a JSON number with an integer value (e.g. `1e1`, `1.0`, `1.50e1`)
/// to its integer string, returning `None` if fractional or overflowed.
pub(super) fn plain_int(lexeme: &str) -> Option<String> {
    let p = plain(lexeme);
    if p.contains(['e', 'E']) {
        return None;
    }
    let int = if let Some((int, frac)) = p.split_once('.') {
        if !frac.bytes().all(|b| b == b'0') {
            return None;
        }
        int
    } else {
        p.as_str()
    };
    Some(if int == "-0" {
        "0".to_string()
    } else {
        int.to_string()
    })
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
    if digits.bytes().all(|b| b == b'0') {
        return format!("{sign}0");
    }
    let point = i64::try_from(int.len())
        .unwrap_or(i64::MAX)
        .saturating_add(exponent);
    let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    // Bound the padding: a pathological exponent stays as written.
    if point < -1000 || point.saturating_sub(len) > 1000 {
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

/// Appends a table cell's TSV text directly to `buf` without allocating on
/// plain literals (`null`, strings, plain numbers/integers).
pub(super) fn write_cell(
    node: &Node<'_>,
    col: Col,
    at: impl FnOnce() -> Path,
    tsv: &mut Tsv,
    buf: &mut String,
) {
    match (&node.value, col) {
        (Value::Null, _) => buf.push_str("null"),
        (Value::String(s), _) => buf.push_str(s),
        (Value::Number(n), Col::Text | Col::Number) if !n.contains(['e', 'E']) => buf.push_str(n),
        (Value::Number(n), Col::Integer) if !n.contains(['.', 'e', 'E']) && *n != "-0" => {
            buf.push_str(n);
        }
        _ => match cell_text(node, col, &at(), tsv) {
            Some(s) => buf.push_str(&s),
            None => buf.push_str("null"),
        },
    }
}

/// The TSV text of a structured or non-trivial table cell.
pub(super) fn cell_text(node: &Node<'_>, col: Col, at: &Path, tsv: &mut Tsv) -> Option<String> {
    match (&node.value, col) {
        (Value::Number(n), Col::Text | Col::Number) => Some(plain(n)),
        (Value::Number(n), Col::Integer) => Some(plain_int(n).unwrap_or_else(|| plain(n))),
        (Value::Object(_), Col::Param) => param_text(node, at, tsv),
        (Value::Array(_), Col::Texts) => {
            joined(node, at, tsv, Some("null"), true, |item, here, tsv| {
                cell_text(item, Col::Text, here, tsv)
            })
        }
        (Value::Array(_), Col::Numbers) => {
            joined(node, at, tsv, Some("null"), true, |item, here, tsv| {
                cell_text(item, Col::Number, here, tsv)
            })
        }
        (Value::Array(_), Col::Integers) => {
            joined(node, at, tsv, Some("null"), true, |item, here, tsv| {
                cell_text(item, Col::Integer, here, tsv)
            })
        }
        (Value::Array(_), Col::SpectraRefs) => {
            joined(node, at, tsv, Some("null"), true, |item, here, tsv| match (
                item.get("ms_run").map(|n| &n.value),
                item.get("reference").map(|n| &n.value),
            ) {
                (Some(Value::Number(run)), Some(Value::String(reference))) => {
                    let run = plain_int(run).unwrap_or_else(|| (*run).to_string());
                    Some(format!("ms_run[{run}]:{reference}"))
                }
                _ => shape_error(tsv, here, "a spectra reference"),
            })
        }
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

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;
    use alloc::string::String;
    use core::cell::Cell;

    use super::super::Tsv;
    use super::write_cell;
    use crate::diag::code::Code;
    use crate::diag::location::Path;
    use crate::json::schema::Col;
    use crate::json::value::{Node, Value};

    #[test]
    fn write_cell_fast_path_skips_pointer_construction() {
        let check = |value, col, expected: &str, expect_at: bool| {
            let mut tsv = Tsv::default();
            let mut buf = String::new();
            let called = Cell::new(false);
            let node = Node { value };
            write_cell(
                &node,
                col,
                || {
                    called.set(true);
                    Path::new().key("x")
                },
                &mut tsv,
                &mut buf,
            );
            assert_eq!(buf, expected, "value mismatch");
            assert_eq!(
                called.get(),
                expect_at,
                "path laziness mismatch for {expected}"
            );
            tsv
        };

        check(Value::Null, Col::Number, "null", false);
        check(Value::String(Cow::Borrowed("abc")), Col::Text, "abc", false);
        check(Value::Number("1.5"), Col::Number, "1.5", false);
        check(Value::Number("1.5"), Col::Text, "1.5", false);
        check(Value::Number("42"), Col::Integer, "42", false);
        check(Value::Number("-0"), Col::Number, "-0", false);

        check(Value::Number("1e2"), Col::Number, "100", true);
        check(Value::Number("1.0"), Col::Integer, "1", true);
        check(Value::Number("-0"), Col::Integer, "0", true);
        let bad = check(Value::Bool(true), Col::Number, "null", true);
        assert!(bad.findings.iter().any(|f| f.code == Code::JsonShape));
    }
}
