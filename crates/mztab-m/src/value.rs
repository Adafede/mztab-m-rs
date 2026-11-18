//! Cell-level values: [`Cell`], [`Text`], [`Raw`] and [`Decimal`].
//!
//! Every typed field the reader fills from a file is a [`Cell`]. A cell holds
//! a well-typed value, the `null` literal, or the raw text that failed to type.
//! The last case is what lets the reader collect *all* findings and still hand
//! the validator a typed model: nothing is lost, and nothing is guessed.
//!
//! ```
//! use mztab_m::{Cell, Decimal};
//!
//! let mass: Cell<'_, Decimal<'_>> = Cell::parse("1.50", Decimal::parse);
//! assert_eq!(mass.value().map(Decimal::as_str), Some("1.50"));
//!
//! let missing: Cell<'_, Decimal<'_>> = Cell::parse("null", Decimal::parse);
//! assert!(missing.is_null());
//!
//! let broken: Cell<'_, Decimal<'_>> = Cell::parse("1,5", Decimal::parse);
//! assert_eq!(broken.invalid().map(|raw| raw.as_str()), Some("1,5"));
//! ```

use alloc::borrow::Cow;
use alloc::string::{String, ToString as _};
use core::fmt;

/// The literal that encodes a missing value in a table cell (SPEC §5.7).
pub const NULL: &str = "null";

/// A typed cell: a value, the `null` literal, or text that failed to type.
///
/// A well-formed model contains no [`Cell::Invalid`]. The reader still
/// produces it for bad input, so that every problem in a file is reported and
/// the original text survives a round trip. The type system cannot rule
/// `Invalid` out, so the `structure` validator reports every one it finds.
///
/// # Examples
///
/// ```
/// use mztab_m::Cell;
///
/// let charge: Cell<'_, i32> = Cell::parse("-2", |s| s.parse().ok().ok_or(()));
/// assert_eq!(charge, Cell::Value(-2));
/// assert_eq!(charge.map(i32::abs), Cell::Value(2));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cell<'a, T> {
    /// A well-typed value.
    Value(T),
    /// The `null` literal: the property is not available (SPEC §5.7).
    Null,
    /// The original text, which is not a valid value of type `T`.
    Invalid(Raw<'a>),
}

impl<'a, T> Cell<'a, T> {
    /// Types `text` with `parse`, mapping the literal `null` to [`Cell::Null`]
    /// and any parse error to [`Cell::Invalid`].
    ///
    /// The parser's error is discarded here. Callers that need it (the reader
    /// does, to build a finding) call the parser themselves.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{Cell, Text};
    ///
    /// let name = Cell::parse("glucose", Text::new);
    /// assert_eq!(name.value().map(Text::as_str), Some("glucose"));
    /// ```
    #[must_use]
    pub fn parse<E>(text: &'a str, parse: impl FnOnce(&'a str) -> Result<T, E>) -> Self {
        if text == NULL {
            return Self::Null;
        }
        parse(text).map_or_else(|_| Self::Invalid(Raw::new(text)), Self::Value)
    }

    /// The value, if this cell holds one.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Cell;
    ///
    /// assert_eq!(Cell::<u8>::Value(3).value(), Some(&3));
    /// assert_eq!(Cell::<u8>::Null.value(), None);
    /// ```
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Null | Self::Invalid(_) => None,
        }
    }

    /// The raw text, if this cell failed to type.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{Cell, Raw};
    ///
    /// let cell = Cell::<u8>::Invalid(Raw::new("x"));
    /// assert_eq!(cell.invalid().map(Raw::as_str), Some("x"));
    /// assert_eq!(Cell::<u8>::Null.invalid(), None);
    /// ```
    #[must_use]
    pub const fn invalid(&self) -> Option<&Raw<'a>> {
        match self {
            Self::Invalid(raw) => Some(raw),
            Self::Value(_) | Self::Null => None,
        }
    }

    /// Whether this cell is the `null` literal.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Cell;
    ///
    /// assert!(Cell::<u8>::Null.is_null());
    /// assert!(!Cell::Value(1_u8).is_null());
    /// ```
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Applies `f` to the value, keeping `Null` and `Invalid` as they are.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Cell;
    ///
    /// assert_eq!(Cell::Value(2_u8).map(u16::from), Cell::Value(2_u16));
    /// assert_eq!(Cell::<u8>::Null.map(u16::from), Cell::Null);
    /// ```
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Cell<'a, U> {
        match self {
            Self::Value(value) => Cell::Value(f(value)),
            Self::Null => Cell::Null,
            Self::Invalid(raw) => Cell::Invalid(raw),
        }
    }
}

/// Text exactly as it appeared in the input, kept when it could not be typed.
///
/// `Raw` is unrestricted: it may hold anything, including text the writer
/// cannot emit. It only ever appears inside [`Cell::Invalid`] and in unknown
/// metadata entries.
///
/// # Examples
///
/// ```
/// use mztab_m::Raw;
///
/// let raw = Raw::new("1,5");
/// assert_eq!(raw.as_str(), "1,5");
/// assert_eq!(raw.to_string(), "1,5");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Raw<'a>(Cow<'a, str>);

impl<'a> Raw<'a> {
    /// Wraps borrowed text.
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self(Cow::Borrowed(text))
    }

    /// Wraps owned text.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Raw;
    ///
    /// assert_eq!(Raw::owned("x".to_owned()), Raw::new("x"));
    /// ```
    #[must_use]
    pub const fn owned(text: String) -> Raw<'static> {
        Raw(Cow::Owned(text))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_cow(self) -> Cow<'a, str> {
        self.0
    }
}

impl fmt::Display for Raw<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string cannot be a [`Text`].
///
/// # Examples
///
/// ```
/// use mztab_m::{Text, TextError};
///
/// assert_eq!(Text::new(""), Err(TextError::Empty));
/// assert_eq!(Text::new("null"), Err(TextError::NullLiteral));
/// assert_eq!(Text::new("a\tb"), Err(TextError::ControlChar { offset: 1 }));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextError {
    /// The string is empty. Cells MUST NOT be empty (SPEC §5.7).
    Empty,
    /// The string is the `null` literal, which a cell cannot carry as text.
    NullLiteral,
    /// The string contains a tab, carriage return or line feed at byte
    /// `offset`. These would split the cell or the line.
    ControlChar {
        /// Byte offset of the first offending character.
        offset: usize,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("text is empty"),
            Self::NullLiteral => f.write_str("text is the `null` literal"),
            Self::ControlChar { offset } => {
                write!(f, "tab or line break at byte {offset}")
            }
        }
    }
}

impl core::error::Error for TextError {}

impl TextError {
    /// The byte offset the error points at: the control character, or the
    /// start of the text.
    pub(crate) const fn offset(self) -> usize {
        match self {
            Self::ControlChar { offset } => offset,
            Self::Empty | Self::NullLiteral => 0,
        }
    }
}

/// The byte offset of the first tab, carriage return or line feed in `text`.
pub(crate) fn find_control(text: &str) -> Option<usize> {
    text.bytes()
        .position(|b| matches!(b, b'\t' | b'\r' | b'\n'))
}

/// A string that can be written as one mzTab cell and read back unchanged.
///
/// A `Text` is non-empty, is not the `null` literal, and contains no tab,
/// carriage return or line feed. Those are exactly the strings that survive
/// `read(write(x)) == x` as a single TSV cell, so a model built from `Text`
/// cannot make the writer emit an extra cell, an extra line, or a `null`.
///
/// List separators (`|`) are allowed: whether a `|` is legal depends on the
/// field, so the list types and the validator handle it.
///
/// # Examples
///
/// ```
/// use mztab_m::Text;
///
/// let id = Text::new("MTBLS263")?;
/// assert_eq!(id.as_str(), "MTBLS263");
/// assert!(Text::new("two\nlines").is_err());
/// # Ok::<(), mztab_m::TextError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text<'a>(Cow<'a, str>);

impl<'a> Text<'a> {
    /// Validates borrowed text.
    ///
    /// # Errors
    ///
    /// Returns a [`TextError`] if `text` is empty, is `null`, or contains a
    /// tab or line break.
    pub fn new(text: &'a str) -> Result<Self, TextError> {
        check_text(text)?;
        Ok(Self(Cow::Borrowed(text)))
    }

    /// Validates owned text.
    ///
    /// # Errors
    ///
    /// As [`Text::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Text;
    ///
    /// assert_eq!(Text::owned("x".to_owned()), Text::new("x"));
    /// ```
    pub fn owned(text: String) -> Result<Text<'static>, TextError> {
        check_text(&text)?;
        Ok(Text(Cow::Owned(text)))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_cow(self) -> Cow<'a, str> {
        self.0
    }

    /// Wraps text already known to be valid (it came from a `Text`).
    pub(crate) const fn from_cow_unchecked(text: Cow<'a, str>) -> Self {
        Self(text)
    }
}

fn check_text(text: &str) -> Result<(), TextError> {
    if text.is_empty() {
        return Err(TextError::Empty);
    }
    if text == NULL {
        return Err(TextError::NullLiteral);
    }
    find_control(text).map_or(Ok(()), |offset| Err(TextError::ControlChar { offset }))
}

impl fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How a [`Decimal`] lexeme relates to the mzTab number grammar.
///
/// SPEC §5.7: numbers MUST be `xs:decimal`, or the literal `NaN`.
/// "Scientific notation and infinity is explicitly not supported." The reader
/// still keeps the two non-conforming forms, because they are common in real
/// files (DECISIONS D-021); the validator reports them.
///
/// # Examples
///
/// ```
/// use mztab_m::{Decimal, DecimalKind};
///
/// assert_eq!(Decimal::parse("-0.5")?.kind(), DecimalKind::Decimal);
/// assert_eq!(Decimal::parse("NaN")?.kind(), DecimalKind::NaN);
/// assert_eq!(Decimal::parse("1E+03")?.kind(), DecimalKind::Scientific);
/// assert_eq!(Decimal::parse("-INF")?.kind(), DecimalKind::Infinity);
/// # Ok::<(), mztab_m::NumberError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecimalKind {
    /// An `xs:decimal`: `[+-]?(digits[.digits?] | .digits)`. Conforming.
    Decimal,
    /// The literal `NaN`, for incalculable numbers. Conforming.
    NaN,
    /// A decimal mantissa with an `e`/`E` exponent. **Not** conforming.
    Scientific,
    /// `INF` or `Infinity`, optionally signed. **Not** conforming.
    Infinity,
}

impl DecimalKind {
    /// Whether this form is allowed by SPEC §5.7.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::DecimalKind;
    ///
    /// assert!(DecimalKind::NaN.is_conforming());
    /// assert!(!DecimalKind::Scientific.is_conforming());
    /// ```
    #[must_use]
    pub const fn is_conforming(self) -> bool {
        matches!(self, Self::Decimal | Self::NaN)
    }
}

/// Why a string is not a number.
///
/// # Examples
///
/// ```
/// use mztab_m::{Decimal, NumberError};
///
/// assert_eq!(Decimal::parse("1,5"), Err(NumberError { offset: 1 }));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NumberError {
    /// Byte offset of the first character that does not fit the grammar. It
    /// equals the length of the input when the input ends too early.
    pub offset: usize,
}

impl fmt::Display for NumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a number (unexpected input at byte {})", self.offset)
    }
}

impl core::error::Error for NumberError {}

/// A number cell, kept as its original lexeme.
///
/// The lexeme is never reformatted, so `1.50` is written back as `1.50`. This
/// is what makes `write(read(f)) == f` possible without float-formatting
/// drift. Equality, ordering and hashing are **lexical**: `1.5 != 1.50`. Use
/// [`Decimal::to_f64`] to compare numerically.
///
/// # Examples
///
/// ```
/// use mztab_m::Decimal;
///
/// let a = Decimal::parse("1.50")?;
/// let b = Decimal::parse("1.5")?;
/// assert_ne!(a, b);
/// assert_eq!(a.to_f64(), b.to_f64());
/// assert_eq!(a.to_string(), "1.50");
/// # Ok::<(), mztab_m::NumberError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decimal<'a> {
    lexeme: Cow<'a, str>,
    kind: DecimalKind,
}

impl<'a> Decimal<'a> {
    /// Parses a number lexeme without copying it.
    ///
    /// Accepted: `xs:decimal`, `NaN`, scientific notation, and signed `INF` /
    /// `Infinity`. The last two are kept but marked non-conforming (see
    /// [`DecimalKind`]). Leading or trailing whitespace is not accepted.
    ///
    /// # Errors
    ///
    /// Returns a [`NumberError`] with the offset of the first bad character.
    pub fn parse(lexeme: &'a str) -> Result<Self, NumberError> {
        let kind = classify(lexeme)?;
        Ok(Self {
            lexeme: Cow::Borrowed(lexeme),
            kind,
        })
    }

    /// Builds a conforming decimal from a finite `f64`, or `NaN`.
    ///
    /// The lexeme is the shortest decimal that reads back as the same `f64`,
    /// never in scientific notation. Returns `None` for infinities, which
    /// mzTab cannot encode.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{Decimal, DecimalKind};
    ///
    /// assert_eq!(Decimal::from_f64(0.1).map(|d| d.to_string()), Some("0.1".to_owned()));
    /// assert_eq!(Decimal::from_f64(1e21).map(|d| d.kind()), Some(DecimalKind::Decimal));
    /// assert_eq!(Decimal::from_f64(f64::NAN).map(|d| d.to_string()), Some("NaN".to_owned()));
    /// assert_eq!(Decimal::from_f64(f64::INFINITY), None);
    /// ```
    #[must_use]
    pub fn from_f64(value: f64) -> Option<Decimal<'static>> {
        if value.is_nan() {
            return Some(Decimal {
                lexeme: Cow::Borrowed("NaN"),
                kind: DecimalKind::NaN,
            });
        }
        value.is_finite().then(|| Decimal {
            lexeme: Cow::Owned(value.to_string()),
            kind: DecimalKind::Decimal,
        })
    }

    /// The original lexeme.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.lexeme
    }

    pub(crate) fn into_static(self) -> Decimal<'static> {
        Decimal {
            lexeme: Cow::Owned(self.lexeme.into_owned()),
            kind: self.kind,
        }
    }

    /// Which form the lexeme takes.
    #[must_use]
    pub const fn kind(&self) -> DecimalKind {
        self.kind
    }

    /// The value as the nearest `f64`. Lexemes beyond the `f64` range
    /// round to an infinity or to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Decimal;
    ///
    /// assert_eq!(Decimal::parse("-.25")?.to_f64(), -0.25);
    /// assert_eq!(Decimal::parse("1E+03")?.to_f64(), 1000.0);
    /// assert!(Decimal::parse("NaN")?.to_f64().is_nan());
    /// assert_eq!(Decimal::parse("-Infinity")?.to_f64(), f64::NEG_INFINITY);
    /// # Ok::<(), mztab_m::NumberError>(())
    /// ```
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        // Every lexeme `classify` accepts is also accepted by core's float
        // parser (which is case-insensitive for `inf`, `infinity` and `nan`).
        self.lexeme.parse().unwrap_or(f64::NAN)
    }
}

impl fmt::Display for Decimal<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.lexeme)
    }
}

/// Classifies a number lexeme, or locates the first byte that breaks it.
fn classify(s: &str) -> Result<DecimalKind, NumberError> {
    if s == "NaN" {
        return Ok(DecimalKind::NaN);
    }
    let bytes = s.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let unsigned = s.get(i..).unwrap_or_default();
    if unsigned == "INF" || unsigned == "Infinity" {
        return Ok(DecimalKind::Infinity);
    }
    let int_digits = count_digits(bytes, i);
    i += int_digits;
    let mut frac_digits = 0;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        frac_digits = count_digits(bytes, i);
        i += frac_digits;
    }
    if int_digits + frac_digits == 0 {
        return Err(NumberError { offset: i });
    }
    if i == bytes.len() {
        return Ok(DecimalKind::Decimal);
    }
    if !matches!(bytes.get(i), Some(b'e' | b'E')) {
        return Err(NumberError { offset: i });
    }
    i += 1;
    i += usize::from(matches!(bytes.get(i), Some(b'+' | b'-')));
    let exp_digits = count_digits(bytes, i);
    i += exp_digits;
    if exp_digits == 0 || i != bytes.len() {
        return Err(NumberError { offset: i });
    }
    Ok(DecimalKind::Scientific)
}

fn count_digits(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count()
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Cell, Decimal, DecimalKind, NumberError, Raw, Text, TextError, classify};
    use alloc::string::ToString as _;

    #[test]
    fn classify_accepts_xs_decimal() {
        for ok in ["0", "-0", "+1", "1.", ".5", "-.5", "12.340", "007"] {
            assert_eq!(classify(ok), Ok(DecimalKind::Decimal), "{ok}");
        }
    }

    #[test]
    fn classify_marks_nonconforming_forms() {
        for sci in ["1e5", "1E+03", "-9.4E-05", ".5e0", "1.e-1"] {
            assert_eq!(classify(sci), Ok(DecimalKind::Scientific), "{sci}");
        }
        for inf in ["INF", "-INF", "+INF", "Infinity", "-Infinity"] {
            assert_eq!(classify(inf), Ok(DecimalKind::Infinity), "{inf}");
        }
        assert_eq!(classify("NaN"), Ok(DecimalKind::NaN), "NaN");
    }

    #[test]
    fn classify_locates_errors() {
        let cases = [
            ("", 0),
            ("-", 1),
            (".", 1),
            ("+.", 2),
            ("1,5", 1),
            (" 1", 0),
            ("1 ", 1),
            ("1e", 2),
            ("1e+", 3),
            ("1e5x", 3),
            ("e5", 0),
            ("nan", 0),
            ("-NaN", 1),
            ("inf", 0),
            ("0x1p3", 1),
            ("1.2.3", 3),
            ("١", 0),
        ];
        for (input, offset) in cases {
            assert_eq!(classify(input), Err(NumberError { offset }), "{input:?}");
        }
    }

    #[test]
    fn decimal_keeps_lexeme_and_converts() {
        let d = Decimal::parse("+001.500").expect("valid fixture");
        assert_eq!(d.as_str(), "+001.500", "lexeme kept");
        assert_eq!(d.to_string(), "+001.500", "display is the lexeme");
        assert!((d.to_f64() - 1.5).abs() < f64::EPSILON, "value");
        assert_eq!(d.kind(), DecimalKind::Decimal, "kind");
        assert!(d.kind().is_conforming(), "conforming");
        assert!(!DecimalKind::Infinity.is_conforming(), "infinity");
        assert!(DecimalKind::Decimal.is_conforming(), "decimal");
    }

    #[test]
    fn decimal_from_f64_round_trips() {
        for v in [0.0, -0.0, 1.5, -1234.5678, 1e-7, 1e300, f64::MIN_POSITIVE] {
            let d = Decimal::from_f64(v).expect("valid fixture");
            assert_eq!(d.kind(), DecimalKind::Decimal, "{v} is xs:decimal");
            assert_eq!(
                classify(d.as_str()),
                Ok(DecimalKind::Decimal),
                "{v} re-classifies"
            );
            assert_eq!(d.to_f64().to_bits(), v.to_bits(), "{v} round-trips");
        }
        assert_eq!(Decimal::from_f64(f64::NEG_INFINITY), None, "-inf");
        let nan = Decimal::from_f64(f64::NAN).expect("valid fixture");
        assert_eq!((nan.as_str(), nan.kind()), ("NaN", DecimalKind::NaN), "nan");
    }

    #[test]
    fn cell_parse_dispatches() {
        assert_eq!(Cell::parse("null", Text::new), Cell::Null, "null literal");
        assert_eq!(
            Cell::parse("NULL", Text::new),
            Cell::Value(Text::new("NULL").expect("valid fixture")),
            "case-sensitive"
        );
        assert_eq!(
            Cell::<Text<'_>>::parse("", Text::new),
            Cell::Invalid(Raw::new("")),
            "empty"
        );
        assert_eq!(
            Cell::<u8>::Invalid(Raw::new("x")).map(u16::from),
            Cell::Invalid(Raw::new("x")),
            "map keeps invalid"
        );
        assert_eq!(Cell::<u8>::Invalid(Raw::new("x")).value(), None, "no value");
        assert_eq!(Cell::Value(1_u8).value(), Some(&1), "value");
        assert_eq!(Cell::Value(1_u8).map(u16::from), Cell::Value(1_u16), "map");
        assert_eq!(Cell::<u8>::Null.map(u16::from), Cell::Null, "map null");
        let invalid = Cell::<u8>::Invalid(Raw::new("x"));
        assert_eq!(invalid.invalid().map(Raw::as_str), Some("x"), "invalid");
        assert!(
            !Cell::<u8>::Invalid(Raw::new("x")).is_null(),
            "invalid is not null"
        );
        assert!(Cell::<u8>::Null.is_null(), "null");
        assert_eq!(Cell::Value(1_u8).invalid(), None, "value is not invalid");
    }

    #[test]
    fn text_rules() {
        assert_eq!(
            Text::new("a\rb"),
            Err(TextError::ControlChar { offset: 1 }),
            "cr"
        );
        assert_eq!(
            Text::new("ab\n"),
            Err(TextError::ControlChar { offset: 2 }),
            "lf"
        );
        assert_eq!(
            Text::owned("null".into()),
            Err(TextError::NullLiteral),
            "owned null"
        );
        assert_eq!(Text::owned("ok".into()), Text::new("ok"), "owned ok");
        assert!(Text::new(" null").is_ok(), "only the exact literal is null");
        assert!(Text::new("a|b").is_ok(), "pipes allowed");
        assert_eq!(TextError::Empty.to_string(), "text is empty", "msg");
        assert_eq!(TextError::Empty.offset(), 0, "empty offset");
        assert_eq!(TextError::NullLiteral.offset(), 0, "null offset");
        assert_eq!(
            TextError::ControlChar { offset: 4 }.offset(),
            4,
            "control offset"
        );
        assert_eq!(
            TextError::NullLiteral.to_string(),
            "text is the `null` literal",
            "msg"
        );
        assert_eq!(
            TextError::ControlChar { offset: 3 }.to_string(),
            "tab or line break at byte 3",
            "msg"
        );
        assert_eq!(
            Text::new("x").map(|t| t.to_string()),
            Ok("x".into()),
            "display"
        );
        assert_eq!(Raw::owned("y".into()).to_string(), "y", "raw display");
        assert_eq!(
            NumberError { offset: 2 }.to_string(),
            "not a number (unexpected input at byte 2)",
            "msg"
        );
    }
}
