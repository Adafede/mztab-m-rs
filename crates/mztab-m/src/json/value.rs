//! JSON AST types and string escaping helper.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

pub(crate) use super::lexer::{is_number, parse};

/// A parsed JSON value. Strings borrow the input unless they had escapes;
/// numbers keep their lexeme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value<'a> {
    Null,
    Bool(bool),
    Number(&'a str),
    String(Cow<'a, str>),
    Array(Vec<Node<'a>>),
    /// Members in input order; duplicates are kept.
    Object(Vec<(Cow<'a, str>, Node<'a>)>),
}

/// A value and the byte offset where it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node<'a> {
    pub(crate) value: Value<'a>,
    pub(crate) at: usize,
}

impl Node<'_> {
    /// The member `key` of an object (its first occurrence).
    pub(crate) fn get(&self, key: &str) -> Option<&Self> {
        match &self.value {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) const fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }
}

/// Why the input is not JSON, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SyntaxError {
    pub(crate) at: usize,
    pub(crate) what: &'static str,
}

/// Writes `text` as a JSON-escaped quoted string to `out`.
///
/// ```
/// use mztab_m::json;
///
/// let mut out = String::new();
/// json::write_string(&mut out, "a \"b\"\x01")?;
/// assert_eq!(out, "\"a \\\"b\\\"\\u0001\"");
/// # Ok::<(), core::fmt::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`fmt::Error`] if writing to `out` fails.
pub fn write_string(out: &mut dyn fmt::Write, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            // TSV cells hold no tab or line feed; a carriage return can be
            // mid-cell. Other control characters as `\u00XX`.
            '\r' => out.write_str("\\r")?,
            c if u32::from(c) < 0x20 => write!(out, "\\u{:04x}", u32::from(c))?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}
