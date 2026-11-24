//! Lexical [`Decimal`] numbers preserving exact source representation.

use alloc::borrow::Cow;
use alloc::string::ToString as _;
use core::fmt;

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

    use super::{Decimal, DecimalKind, NumberError, classify};
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
        assert_eq!(
            NumberError { offset: 2 }.to_string(),
            "not a number (unexpected input at byte 2)",
            "msg"
        );
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
}
