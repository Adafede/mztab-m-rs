//! [`Cell`]: typed values, `null` literals, or raw unparsable text.

use super::NULL;
use super::text::Raw;

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

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::Cell;
    use crate::value::text::{Raw, Text};

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
}
