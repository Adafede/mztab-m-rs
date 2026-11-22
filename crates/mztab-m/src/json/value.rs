//! A small RFC 8259 JSON parser and the string escaping of the writer.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Nesting deeper than this is refused (the parser recurses).
const MAX_DEPTH: usize = 128;

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

/// Parses one JSON text (surrounded by optional whitespace).
pub(crate) fn parse(text: &str) -> Result<Node<'_>, SyntaxError> {
    let mut p = Parser { text, at: 0 };
    let node = p.value(0)?;
    p.space();
    if p.at < text.len() {
        return Err(p.error("text after the JSON value"));
    }
    Ok(node)
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Parser<'a> {
    const fn error(&self, what: &'static str) -> SyntaxError {
        SyntaxError { at: self.at, what }
    }

    const fn offset(&self, rest: &str) -> usize {
        self.text.len() - rest.len()
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn space(&mut self) {
        let rest = self.text.get(self.at..).unwrap_or_default();
        self.at = self.offset(rest.trim_start_matches([' ', '\t', '\n', '\r']));
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.at += 1;
        }
        found
    }

    fn literal(&mut self, word: &str, value: Value<'a>) -> Result<Value<'a>, SyntaxError> {
        let rest = self.text.get(self.at..).unwrap_or_default();
        if rest.starts_with(word) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Node<'a>, SyntaxError> {
        self.space();
        let at = self.at;
        let value = match self.peek() {
            None => return Err(self.error("expected a value")),
            Some(b'n') => self.literal("null", Value::Null)?,
            Some(b't') => self.literal("true", Value::Bool(true))?,
            Some(b'f') => self.literal("false", Value::Bool(false))?,
            Some(b'"') => Value::String(self.string()?),
            Some(b'[' | b'{') if depth >= MAX_DEPTH => return Err(self.error("nesting too deep")),
            Some(b'[') => self.array(depth)?,
            Some(b'{') => self.object(depth)?,
            Some(_) => {
                let rest = self.text.get(self.at..).unwrap_or_default();
                let len = number_len(rest).ok_or_else(|| self.error("expected a value"))?;
                self.at += len;
                Value::Number(rest.get(..len).unwrap_or_default())
            }
        };
        Ok(Node { value, at })
    }

    fn array(&mut self, depth: usize) -> Result<Value<'a>, SyntaxError> {
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.eat(b']') {
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.space();
            if self.eat(b']') {
                return Ok(Value::Array(items));
            }
            if !self.eat(b',') {
                return Err(self.error("expected `,` or `]`"));
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value<'a>, SyntaxError> {
        self.at += 1;
        let mut members = Vec::new();
        self.space();
        if self.eat(b'}') {
            return Ok(Value::Object(members));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a member name"));
            }
            let key = self.string()?;
            self.space();
            if !self.eat(b':') {
                return Err(self.error("expected `:`"));
            }
            members.push((key, self.value(depth + 1)?));
            self.space();
            if self.eat(b'}') {
                return Ok(Value::Object(members));
            }
            if !self.eat(b',') {
                return Err(self.error("expected `,` or `}`"));
            }
        }
    }

    /// A string, at its opening quote.
    fn string(&mut self) -> Result<Cow<'a, str>, SyntaxError> {
        self.at += 1;
        let start = self.at;
        let mut owned: Option<String> = None;
        let mut chars = self.text.get(start..).unwrap_or_default().chars();
        loop {
            let at = self.offset(chars.as_str());
            let Some(c) = chars.next() else {
                return Err(SyntaxError {
                    at,
                    what: "unterminated string",
                });
            };
            match c {
                '"' => {
                    let plain = self.text.get(start..at).unwrap_or_default();
                    self.at = self.offset(chars.as_str());
                    return Ok(owned.map_or(Cow::Borrowed(plain), Cow::Owned));
                }
                '\\' => {
                    let buf = owned.get_or_insert_with(|| {
                        String::from(self.text.get(start..at).unwrap_or_default())
                    });
                    let escaped = self.escape(&mut chars)?;
                    buf.push(escaped);
                }
                c if u32::from(c) < 0x20 => {
                    return Err(SyntaxError {
                        at,
                        what: "control character in string",
                    });
                }
                c => {
                    if let Some(buf) = &mut owned {
                        buf.push(c);
                    }
                }
            }
        }
    }

    /// One escape, after its backslash.
    fn escape(&self, chars: &mut core::str::Chars<'a>) -> Result<char, SyntaxError> {
        let at = self.offset(chars.as_str());
        let Some(c) = chars.next() else {
            return Err(SyntaxError {
                at,
                what: "unterminated string",
            });
        };
        Ok(match c {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => {
                let high = self.hex4(chars)?;
                if (0xD800..0xDC00).contains(&high) {
                    let Some(rest) = chars.as_str().strip_prefix("\\u") else {
                        return Err(SyntaxError {
                            at: self.offset(chars.as_str()),
                            what: "unpaired surrogate",
                        });
                    };
                    *chars = rest.chars();
                    let low = self.hex4(chars)?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(SyntaxError {
                            at: self.offset(chars.as_str()),
                            what: "unpaired surrogate",
                        });
                    }
                    let code = 0x1_0000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                    // A valid pair is always a valid scalar value.
                    char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
                } else {
                    char::from_u32(high).ok_or_else(|| SyntaxError {
                        at: self.offset(chars.as_str()),
                        what: "unpaired surrogate",
                    })?
                }
            }
            _ => {
                return Err(SyntaxError {
                    at,
                    what: "invalid escape",
                });
            }
        })
    }

    fn hex4(&self, chars: &mut core::str::Chars<'a>) -> Result<u32, SyntaxError> {
        let rest = chars.as_str();
        let digits = rest
            .get(..4)
            .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| SyntaxError {
                at: self.offset(rest),
                what: "invalid `\\u` escape",
            })?;
        *chars = rest.get(4..).unwrap_or_default().chars();
        Ok(digits
            .chars()
            .filter_map(|c| c.to_digit(16))
            .fold(0, |acc, d| acc * 16 + d))
    }
}

/// The length of the JSON number at the start of `text`, if one is there.
fn number_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes
            .get(from..)
            .map_or(0, |b| b.iter().take_while(|c| c.is_ascii_digit()).count())
    };
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(at) {
        Some(b'0') => at += 1,
        Some(b'1'..=b'9') => at += digits(at),
        _ => return None,
    }
    if bytes.get(at) == Some(&b'.') {
        let n = digits(at + 1);
        if n == 0 {
            return None;
        }
        at += 1 + n;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let n = digits(at);
        if n == 0 {
            return None;
        }
        at += n;
    }
    Some(at)
}

/// Whether `text` is exactly one JSON number.
pub(crate) fn is_number(text: &str) -> bool {
    number_len(text) == Some(text.len())
}

/// Writes `text` as a JSON string.
pub(crate) fn write_string(out: &mut dyn fmt::Write, text: &str) -> fmt::Result {
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
