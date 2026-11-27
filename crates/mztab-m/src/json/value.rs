//! JSON AST types and string escaping helper.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

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

/// A parsed JSON value with object member lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node<'a> {
    pub(crate) value: Value<'a>,
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
    let mut start = 0;
    for (idx, byte) in text.bytes().enumerate() {
        let esc = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            // TSV cells hold no tab or line feed; a carriage return can be
            // mid-cell. Other control characters as `\u00XX`.
            b'\r' => "\\r",
            0..=0x1f => {
                if idx > start {
                    out.write_str(text.get(start..idx).unwrap_or_default())?;
                }
                write!(out, "\\u{byte:04x}")?;
                start = idx + 1;
                continue;
            }
            _ => continue,
        };
        if idx > start {
            out.write_str(text.get(start..idx).unwrap_or_default())?;
        }
        out.write_str(esc)?;
        start = idx + 1;
    }
    if start < text.len() {
        out.write_str(text.get(start..).unwrap_or_default())?;
    }
    out.write_char('"')
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use alloc::string::String;
    use core::fmt;

    use super::write_string;

    struct NonEmptyWrite(String);

    impl fmt::Write for NonEmptyWrite {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            assert!(!s.is_empty(), "empty write_str call");
            self.0.push_str(s);
            Ok(())
        }
    }

    #[test]
    fn write_string_flushes_slices_and_escapes_boundaries() {
        let cases = [
            ("", "\"\""),
            ("plain", "\"plain\""),
            ("é 😀", "\"é 😀\""),
            ("\"", "\"\\\"\""),
            ("\"\\", "\"\\\"\\\\\""),
            ("a\"b\\c\rd\x01\x1fe", "\"a\\\"b\\\\c\\rd\\u0001\\u001fe\""),
            ("\x00x\x1f", "\"\\u0000x\\u001f\""),
        ];
        for (input, expected) in cases {
            let mut out = NonEmptyWrite(String::new());
            assert!(
                write_string(&mut out, input).is_ok(),
                "write into String: {input:?}"
            );
            assert_eq!(out.0, expected, "{input:?}");
        }
    }
}
