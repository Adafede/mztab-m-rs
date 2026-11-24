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

pub(crate) mod parse;
pub(crate) mod text;

use core::fmt;

use parse::ParamError;
use text::{Accession, ParamText};

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
        parse::parse_param(input)
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

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Accession, Param, ParamText};
    use alloc::string::ToString as _;

    fn text(s: &str) -> ParamText<'_> {
        ParamText::new(s).expect("valid test text")
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
}
