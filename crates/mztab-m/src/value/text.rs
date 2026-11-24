//! [`Raw`] and validated cell [`Text`].

use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt;

use super::NULL;

/// Text exactly as it appeared in the input, kept when it could not be typed.
///
/// `Raw` is unrestricted: it may hold anything, including text the writer
/// cannot emit. It only ever appears inside [`Cell::Invalid`](super::Cell::Invalid)
/// and in unknown metadata entries.
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

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Raw, Text, TextError};
    use alloc::string::ToString as _;

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
    }
}
