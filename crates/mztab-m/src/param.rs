//! Parameters: `[CV label, accession, name, value]` (SPEC §6, "Params").
//!
//! ```
//! use mztab_m::Param;
//!
//! let cv = Param::parse("[MS, MS:1001477, SpectraST, ]")?;
//! assert_eq!(cv.name().as_str(), "SpectraST");
//! assert_eq!(cv.accession().map(|a| a.as_str()), Some("MS:1001477"));
//!
//! let user = Param::parse("[,,A user parameter, The value]")?;
//! assert_eq!(user.value().map(|v| v.as_str()), Some("The value"));
//!
//! let quoted = Param::parse(r#"[MOD, MOD:00648, "N,O-diacetylated L-serine",]"#)?;
//! assert_eq!(quoted.name().as_str(), "N,O-diacetylated L-serine");
//! assert_eq!(quoted.to_string(), r#"[MOD, MOD:00648, "N,O-diacetylated L-serine", ]"#);
//! # Ok::<(), mztab_m::ParamError>(())
//! ```

use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt;

use crate::value::find_control;

/// Why a string cannot be a [`ParamText`].
///
/// # Examples
///
/// ```
/// use mztab_m::{ParamText, ParamTextError};
///
/// assert_eq!(ParamText::new(""), Err(ParamTextError::Empty));
/// assert_eq!(ParamText::new("a\"b"), Err(ParamTextError::Quote { offset: 1 }));
/// assert_eq!(ParamText::new("a\nb"), Err(ParamTextError::ControlChar { offset: 1 }));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParamTextError {
    /// The text is empty. Absent fields are modelled as `None`.
    Empty,
    /// The text contains a double quote, which the grammar cannot escape.
    Quote {
        /// Byte offset of the quote.
        offset: usize,
    },
    /// The text contains a tab, carriage return or line feed.
    ControlChar {
        /// Byte offset of the character.
        offset: usize,
    },
}

impl fmt::Display for ParamTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("parameter field is empty"),
            Self::Quote { offset } => write!(f, "double quote at byte {offset}"),
            Self::ControlChar { offset } => write!(f, "tab or line break at byte {offset}"),
        }
    }
}

impl core::error::Error for ParamTextError {}

/// The text of one parameter field.
///
/// Non-empty, and free of double quotes, tabs and line breaks: exactly the
/// strings that a parameter can carry through a write and a read unchanged.
/// Commas and surrounding spaces are allowed; the writer quotes them.
///
/// # Examples
///
/// ```
/// use mztab_m::ParamText;
///
/// let name = ParamText::new("N,O-diacetylated L-serine")?;
/// assert_eq!(name.as_str(), "N,O-diacetylated L-serine");
/// # Ok::<(), mztab_m::ParamTextError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamText<'a>(Cow<'a, str>);

impl<'a> ParamText<'a> {
    /// Validates borrowed text.
    ///
    /// # Errors
    ///
    /// Returns a [`ParamTextError`] if `text` is empty or contains a double
    /// quote, a tab or a line break.
    pub fn new(text: &'a str) -> Result<Self, ParamTextError> {
        check_param_text(text)?;
        Ok(Self(Cow::Borrowed(text)))
    }

    /// Validates owned text.
    ///
    /// # Errors
    ///
    /// As [`ParamText::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::ParamText;
    ///
    /// assert_eq!(ParamText::owned("x".to_owned()), ParamText::new("x"));
    /// ```
    pub fn owned(text: String) -> Result<ParamText<'static>, ParamTextError> {
        check_param_text(&text)?;
        Ok(ParamText(Cow::Owned(text)))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the writer must quote this text.
    fn needs_quotes(&self) -> bool {
        self.0.contains(',') || self.0.starts_with(' ') || self.0.ends_with(' ')
    }
}

fn check_param_text(text: &str) -> Result<(), ParamTextError> {
    if text.is_empty() {
        return Err(ParamTextError::Empty);
    }
    if let Some(offset) = find_control(text) {
        return Err(ParamTextError::ControlChar { offset });
    }
    text.find('"')
        .map_or(Ok(()), |offset| Err(ParamTextError::Quote { offset }))
}

impl fmt::Display for ParamText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.needs_quotes() {
            write!(f, "\"{}\"", self.0)
        } else {
            f.write_str(&self.0)
        }
    }
}

/// A CV accession such as `MS:1001477`.
///
/// The type guarantees only what the grammar does: a non-empty
/// [`ParamText`]. Whether the accession is a well-formed `PREFIX:LOCAL` CURIE
/// is a validator rule (SPEC is not explicit, and jmzTab-m only warns, code
/// 1035), so [`Accession::prefix`] exposes it without enforcing it.
///
/// # Examples
///
/// ```
/// use mztab_m::{Accession, ParamText};
///
/// let curie = Accession::new(ParamText::new("MS:1001477")?);
/// assert_eq!(curie.prefix(), Some("MS"));
/// assert_eq!(curie.local(), Some("1001477"));
///
/// let bare = Accession::new(ParamText::new("1001477")?);
/// assert_eq!(bare.prefix(), None);
/// # Ok::<(), mztab_m::ParamTextError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Accession<'a>(ParamText<'a>);

impl<'a> Accession<'a> {
    /// Wraps the accession text.
    #[must_use]
    pub const fn new(text: ParamText<'a>) -> Self {
        Self(text)
    }

    /// The accession text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The part before the first `:`, if both sides are non-empty.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.split().map(|(prefix, _)| prefix)
    }

    /// The part after the first `:`, if both sides are non-empty.
    #[must_use]
    pub fn local(&self) -> Option<&str> {
        self.split().map(|(_, local)| local)
    }

    fn split(&self) -> Option<(&str, &str)> {
        self.as_str()
            .split_once(':')
            .filter(|(prefix, local)| !prefix.is_empty() && !local.is_empty())
    }
}

impl fmt::Display for Accession<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A parameter: either a CV term or a user-defined name/value pair.
///
/// A half-CV parameter (a label without an accession, or the reverse) and a
/// parameter without a name cannot be constructed.
///
/// # Grammar
///
/// The spec gives the four-field shape, says unavailable fields are empty,
/// and says a field containing commas MUST be quoted. It defines nothing
/// else, so this crate fixes the rest (DECISIONS D-050):
///
/// - The cell starts with `[` and ends with `]`. Nothing may surround them.
/// - Fields are separated by commas outside double quotes. There are exactly
///   four fields.
/// - Spaces around a field are insignificant. A field whose trimmed text
///   starts with `"` is quoted: it runs to the next `"`, and only spaces may
///   follow it. Quotes cannot be escaped, so field text never contains `"`.
/// - `name` is required. A CV parameter has both `label` and `accession`; a
///   user parameter has neither. Having exactly one of them is an error.
///
/// The writer quotes a field only when its text contains a comma or starts or
/// ends with a space, so every [`Param`] writes and reads back unchanged.
///
/// # Examples
///
/// ```
/// use mztab_m::{Accession, Param, ParamText};
///
/// let built = Param::Cv {
///     label: ParamText::new("MS")?,
///     accession: Accession::new(ParamText::new("MS:1002879")?),
///     name: ParamText::new("Progenesis QI")?,
///     value: Some(ParamText::new("3.0")?),
/// };
/// assert_eq!(built.to_string(), "[MS, MS:1002879, Progenesis QI, 3.0]");
/// assert_eq!(Param::parse("[MS,MS:1002879,Progenesis QI,3.0]"), Ok(built));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Param<'a> {
    /// A term from a controlled vocabulary.
    Cv {
        /// The CV label, e.g. `MS`. It should match a `cv[n]-label`.
        label: ParamText<'a>,
        /// The term accession, e.g. `MS:1001477`.
        accession: Accession<'a>,
        /// The term name.
        name: ParamText<'a>,
        /// The value, if any.
        value: Option<ParamText<'a>>,
    },
    /// A user parameter: a name and an optional value, no CV term.
    User {
        /// The parameter name.
        name: ParamText<'a>,
        /// The value, if any.
        value: Option<ParamText<'a>>,
    },
}

impl<'a> Param<'a> {
    /// Parses one parameter, borrowing from `input`.
    ///
    /// # Errors
    ///
    /// Returns a [`ParamError`] locating the first violation of the grammar
    /// described under [`Param`].
    pub fn parse(input: &'a str) -> Result<Self, ParamError> {
        parse_param(input)
    }

    /// The CV label, for a CV parameter.
    #[must_use]
    pub const fn label(&self) -> Option<&ParamText<'a>> {
        match self {
            Self::Cv { label, .. } => Some(label),
            Self::User { .. } => None,
        }
    }

    /// The accession, for a CV parameter.
    #[must_use]
    pub const fn accession(&self) -> Option<&Accession<'a>> {
        match self {
            Self::Cv { accession, .. } => Some(accession),
            Self::User { .. } => None,
        }
    }

    /// The name.
    #[must_use]
    pub const fn name(&self) -> &ParamText<'a> {
        match self {
            Self::Cv { name, .. } | Self::User { name, .. } => name,
        }
    }

    /// The value, if any.
    #[must_use]
    pub const fn value(&self) -> Option<&ParamText<'a>> {
        match self {
            Self::Cv { value, .. } | Self::User { value, .. } => value.as_ref(),
        }
    }
}

/// Writes the canonical form: `[label, accession, name, value]`, fields
/// separated by `", "`, absent fields empty, and quotes only where needed.
impl fmt::Display for Param<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cv {
                label,
                accession,
                name,
                ..
            } => write!(f, "[{label}, {accession}, {name}, ")?,
            Self::User { name, .. } => write!(f, "[, , {name}, ")?,
        }
        if let Some(value) = self.value() {
            value.fmt(f)?;
        }
        f.write_str("]")
    }
}

/// What is wrong with a parameter.
///
/// # Examples
///
/// ```
/// use mztab_m::{ParamErrorKind, Param};
///
/// let err = Param::parse("[MS, , name, ]").unwrap_err();
/// assert_eq!(err.kind, ParamErrorKind::HalfCv);
/// assert_eq!(err.offset, 1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParamErrorKind {
    /// The input does not start with `[`.
    MissingOpenBracket,
    /// The input does not end with `]`.
    MissingCloseBracket,
    /// A quoted field is not closed.
    UnclosedQuote,
    /// Text other than spaces follows a closing quote, or a quote appears
    /// inside an unquoted field.
    StrayQuote,
    /// The number of fields is not four.
    FieldCount {
        /// The number of fields found.
        found: usize,
    },
    /// The name field is empty.
    MissingName,
    /// Exactly one of label and accession is present.
    HalfCv,
    /// A field contains a tab or a line break.
    ControlChar,
}

/// A parameter that does not follow the grammar, and where.
///
/// `offset` is a byte offset into the parsed input, suitable for turning into
/// a column number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamError {
    /// What is wrong.
    pub kind: ParamErrorKind,
    /// Byte offset of the problem in the input.
    pub offset: usize,
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ParamErrorKind::MissingOpenBracket => "expected `[`",
            ParamErrorKind::MissingCloseBracket => "expected `]` at the end",
            ParamErrorKind::UnclosedQuote => "unclosed double quote",
            ParamErrorKind::StrayQuote => "unexpected double quote or text after a quoted field",
            ParamErrorKind::FieldCount { found } => {
                return write!(
                    f,
                    "expected 4 comma-separated fields, found {found} (at byte {})",
                    self.offset
                );
            }
            ParamErrorKind::MissingName => "the name field is empty",
            ParamErrorKind::HalfCv => "a CV parameter needs both a label and an accession",
            ParamErrorKind::ControlChar => "tab or line break inside a parameter",
        };
        write!(f, "{what} (at byte {})", self.offset)
    }
}

impl core::error::Error for ParamError {}

const fn err(kind: ParamErrorKind, offset: usize) -> ParamError {
    ParamError { kind, offset }
}

/// One field: its text (if non-empty) and its start offset in the input.
type Field<'a> = (Option<ParamText<'a>>, usize);

fn parse_param(input: &str) -> Result<Param<'_>, ParamError> {
    let Some(rest) = input.strip_prefix('[') else {
        return Err(err(ParamErrorKind::MissingOpenBracket, 0));
    };
    let Some(inner) = rest.strip_suffix(']') else {
        return Err(err(ParamErrorKind::MissingCloseBracket, input.len()));
    };
    if let Some(at) = find_control(inner) {
        return Err(err(ParamErrorKind::ControlChar, at + 1));
    }
    let mut fields: [Field<'_>; 4] = [(None, 0), (None, 0), (None, 0), (None, 0)];
    let mut found = 0;
    let mut start = 0;
    // Every field but the last ends at a comma, so there are at most
    // `inner.len() + 1` fields. The bound makes termination evident rather
    // than a consequence of the offset arithmetic.
    for _ in 0..=inner.len() {
        let (field, end) = scan_field(inner, start)?;
        if let Some(slot) = fields.get_mut(found) {
            *slot = (field, start + 1);
        }
        found += 1;
        match end {
            Some(comma) => start = comma + 1,
            None => break,
        }
    }
    if found != 4 {
        return Err(err(ParamErrorKind::FieldCount { found }, input.len() - 1));
    }
    let [
        (label, label_at),
        (accession, _),
        (name, name_at),
        (value, _),
    ] = fields;
    let name = name.ok_or_else(|| err(ParamErrorKind::MissingName, name_at))?;
    match (label, accession) {
        (Some(label), Some(accession)) => Ok(Param::Cv {
            label,
            accession: Accession::new(accession),
            name,
            value,
        }),
        (None, None) => Ok(Param::User { name, value }),
        _ => Err(err(ParamErrorKind::HalfCv, label_at)),
    }
}

/// Scans one field of `inner` starting at `start`.
///
/// Returns the field text (`None` when empty) and the offset of the comma
/// that ends it (`None` at the end of `inner`). Offsets in errors are relative
/// to the whole input, which has one leading `[`.
fn scan_field(
    inner: &str,
    start: usize,
) -> Result<(Option<ParamText<'_>>, Option<usize>), ParamError> {
    let rest = inner.get(start..).unwrap_or_default();
    let lead = rest.len() - rest.trim_start_matches(' ').len();
    let body_at = start + lead;
    let body = rest.get(lead..).unwrap_or_default();
    let (text, after_at) = if let Some(quoted) = body.strip_prefix('"') {
        let close = quoted
            .find('"')
            .ok_or_else(|| err(ParamErrorKind::UnclosedQuote, body_at + 1))?;
        let text = quoted.get(..close).unwrap_or_default();
        (text, body_at + 1 + close + 1)
    } else {
        let len = body.find(',').unwrap_or(body.len());
        let text = body.get(..len).unwrap_or_default().trim_end_matches(' ');
        if let Some(q) = text.find('"') {
            return Err(err(ParamErrorKind::StrayQuote, body_at + q + 1));
        }
        (text, body_at + text.len())
    };
    let after = inner.get(after_at..).unwrap_or_default();
    let trailing = after.len() - after.trim_start_matches(' ').len();
    let end_at = after_at + trailing;
    let end = match inner.as_bytes().get(end_at) {
        None => None,
        Some(b',') => Some(end_at),
        Some(_) => return Err(err(ParamErrorKind::StrayQuote, end_at + 1)),
    };
    // `text` has no quote (it ended at one, or we checked) and no control
    // character (the caller checked), so only emptiness can fail.
    Ok((ParamText::new(text).ok(), end))
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Accession, Param, ParamError, ParamErrorKind, ParamText, ParamTextError};
    use alloc::string::ToString as _;

    fn text(s: &str) -> ParamText<'_> {
        ParamText::new(s).expect("valid test text")
    }

    fn fail(input: &str) -> Option<(ParamErrorKind, usize)> {
        Param::parse(input)
            .err()
            .map(|ParamError { kind, offset }| (kind, offset))
    }

    #[test]
    fn parses_cv_and_user() {
        let cv = Param::parse("[MS, MS:1, a b, ]");
        assert_eq!(
            cv,
            Ok(Param::Cv {
                label: text("MS"),
                accession: Accession::new(text("MS:1")),
                name: text("a b"),
                value: None
            }),
            "cv"
        );
        let user = Param::parse("[, ,n,v]");
        assert_eq!(
            user,
            Ok(Param::User {
                name: text("n"),
                value: Some(text("v"))
            }),
            "user"
        );
        let user = user.expect("valid fixture");
        assert_eq!(user.label(), None, "user has no label");
        let cv = cv.expect("valid fixture");
        assert_eq!(
            cv.accession().map(Accession::as_str),
            Some("MS:1"),
            "cv accession"
        );
        assert_eq!(user.accession(), None, "user has no accession");
        assert_eq!(user.to_string(), "[, , n, v]", "canonical user");
    }

    #[test]
    fn quoted_fields() {
        let p = Param::parse(r#"[,, "no database", null ]"#).expect("valid fixture");
        assert_eq!(p.name().as_str(), "no database", "quoted name");
        assert_eq!(p.value().map(ParamText::as_str), Some("null"), "value");
        assert_eq!(
            p.to_string(),
            "[, , no database, null]",
            "quotes dropped when not needed"
        );
        let p = Param::parse(r#"[a,b:c,"x, y"," v "]"#).expect("valid fixture");
        assert_eq!(
            p.to_string(),
            r#"[a, b:c, "x, y", " v "]"#,
            "quotes kept when needed"
        );
        assert_eq!(p.label().map(ParamText::as_str), Some("a"), "label");
        let p = Param::parse(r#"["",,n,]"#).expect("valid fixture");
        assert_eq!(p.label(), None, "empty quoted field is absent");
        assert_eq!(
            Param::parse("[,,a\u{a0}b,]").map(|p| p.to_string()),
            Ok("[, , a\u{a0}b, ]".into()),
            "nbsp is text"
        );
    }

    #[test]
    fn errors_are_located() {
        let cases: &[(&str, ParamErrorKind, usize)] = &[
            ("", ParamErrorKind::MissingOpenBracket, 0),
            (" [,,n,]", ParamErrorKind::MissingOpenBracket, 0),
            ("[,,n,", ParamErrorKind::MissingCloseBracket, 5),
            ("[,,n,] ", ParamErrorKind::MissingCloseBracket, 7),
            ("[,,n]", ParamErrorKind::FieldCount { found: 3 }, 4),
            ("[,,n,,]", ParamErrorKind::FieldCount { found: 5 }, 6),
            ("[]", ParamErrorKind::FieldCount { found: 1 }, 1),
            ("[,,,]", ParamErrorKind::MissingName, 3),
            ("[,, ,v]", ParamErrorKind::MissingName, 3),
            ("[MS,,n,]", ParamErrorKind::HalfCv, 1),
            ("[,MS:1,n,]", ParamErrorKind::HalfCv, 1),
            ("[,,\"n,]", ParamErrorKind::UnclosedQuote, 3),
            ("[,,\"n\"x,]", ParamErrorKind::StrayQuote, 6),
            ("[,,\"n\" x,]", ParamErrorKind::StrayQuote, 7),
            ("[,,a\"b,]", ParamErrorKind::StrayQuote, 4),
            ("[,,n\t,]", ParamErrorKind::ControlChar, 4),
            ("[,,n,\r]", ParamErrorKind::ControlChar, 5),
        ];
        for &(input, kind, offset) in cases {
            assert_eq!(fail(input), Some((kind, offset)), "{input:?}");
        }
    }

    #[test]
    fn param_text_rules() {
        assert_eq!(
            ParamText::new("a\"b"),
            Err(ParamTextError::Quote { offset: 1 }),
            "quote"
        );
        assert_eq!(
            ParamText::owned("a\tb".into()),
            Err(ParamTextError::ControlChar { offset: 1 }),
            "tab"
        );
        assert_eq!(
            ParamText::owned(alloc::string::String::new()),
            Err(ParamTextError::Empty),
            "empty"
        );
        assert_eq!(ParamText::owned("a b".into()), Ok(text("a b")), "owned");
        assert_eq!(text(" x").to_string(), "\" x\"", "leading space quoted");
        assert_eq!(text("x ").to_string(), "\"x \"", "trailing space quoted");
        assert_eq!(text("x,y").to_string(), "\"x,y\"", "comma quoted");
        assert_eq!(text("x y").to_string(), "x y", "inner space bare");
        assert_eq!(
            ParamTextError::Empty.to_string(),
            "parameter field is empty",
            "msg"
        );
        assert_eq!(
            ParamTextError::Quote { offset: 2 }.to_string(),
            "double quote at byte 2",
            "msg"
        );
        assert_eq!(
            ParamTextError::ControlChar { offset: 2 }.to_string(),
            "tab or line break at byte 2",
            "msg"
        );
    }

    #[test]
    fn accession_parts() {
        let acc = Accession::new(text("a:b:c"));
        assert_eq!(
            (acc.prefix(), acc.local()),
            (Some("a"), Some("b:c")),
            "first colon splits"
        );
        for bare in [":x", "x:", "x"] {
            let acc = Accession::new(text(bare));
            assert_eq!((acc.prefix(), acc.local()), (None, None), "{bare}");
        }
        assert_eq!(
            Accession::new(text("a, b")).to_string(),
            "\"a, b\"",
            "display quotes"
        );
    }

    #[test]
    fn error_messages() {
        let msg = |kind, offset| ParamError { kind, offset }.to_string();
        assert_eq!(
            msg(ParamErrorKind::MissingOpenBracket, 0),
            "expected `[` (at byte 0)",
            "open"
        );
        assert_eq!(
            msg(ParamErrorKind::MissingCloseBracket, 3),
            "expected `]` at the end (at byte 3)",
            "close"
        );
        assert_eq!(
            msg(ParamErrorKind::UnclosedQuote, 1),
            "unclosed double quote (at byte 1)",
            "quote"
        );
        assert_eq!(
            msg(ParamErrorKind::StrayQuote, 1),
            "unexpected double quote or text after a quoted field (at byte 1)",
            "stray"
        );
        assert_eq!(
            msg(ParamErrorKind::FieldCount { found: 2 }, 5),
            "expected 4 comma-separated fields, found 2 (at byte 5)",
            "count"
        );
        assert_eq!(
            msg(ParamErrorKind::MissingName, 1),
            "the name field is empty (at byte 1)",
            "name"
        );
        assert_eq!(
            msg(ParamErrorKind::HalfCv, 1),
            "a CV parameter needs both a label and an accession (at byte 1)",
            "half"
        );
        assert_eq!(
            msg(ParamErrorKind::ControlChar, 1),
            "tab or line break inside a parameter (at byte 1)",
            "control"
        );
    }
}
