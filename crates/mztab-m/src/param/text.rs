//! [`ParamText`] and [`Accession`] primitives for parameters.

use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt;

use crate::value::text::find_control;

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

    pub(crate) fn into_cow(self) -> Cow<'a, str> {
        self.0
    }

    /// Wraps text already known to be valid (it came from a `ParamText`).
    pub(crate) const fn from_cow_unchecked(text: Cow<'a, str>) -> Self {
        Self(text)
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

    pub(crate) fn into_text(self) -> ParamText<'a> {
        self.0
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

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Accession, ParamText, ParamTextError};
    use alloc::string::ToString as _;

    fn text(s: &str) -> ParamText<'_> {
        ParamText::new(s).expect("valid test text")
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
}
