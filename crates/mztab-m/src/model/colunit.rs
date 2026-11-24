//! [`ColUnit`] (`colunit-*`) and [`SpectraRef`] (`ms_run[n]:ref`) field types.

use core::fmt;

use crate::id::{MsRunId, parse_index};
use crate::param::{Param, ParamError};
use crate::value::{Text, TextError};

/// Why a string is not a `column=[param]` unit mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColUnitError {
    /// There is no `=`.
    MissingEquals,
    /// The column name is not a valid [`Text`].
    Column(TextError),
    /// The unit is not a valid parameter. The offset is relative to the
    /// parameter, which starts after the first `=`.
    Param(ParamError),
}

impl fmt::Display for ColUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEquals => f.write_str("expected `column=[unit parameter]`"),
            Self::Column(e) => write!(f, "bad column name: {e}"),
            Self::Param(e) => write!(f, "bad unit: {e}"),
        }
    }
}

impl core::error::Error for ColUnitError {}

/// A `colunit-*` value: `column_name=[unit parameter]`.
///
/// # Examples
///
/// ```
/// use mztab_m::ColUnit;
///
/// let unit = ColUnit::parse("retention_time=[UO, UO:0000031, minute, ]")?;
/// assert_eq!(unit.column.as_str(), "retention_time");
/// assert_eq!(unit.param.name().as_str(), "minute");
/// assert_eq!(unit.to_string(), "retention_time=[UO, UO:0000031, minute, ]");
/// # Ok::<(), mztab_m::ColUnitError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColUnit<'a> {
    /// The column the unit applies to.
    pub column: Text<'a>,
    /// The unit.
    pub param: Param<'a>,
}

impl<'a> ColUnit<'a> {
    /// Parses `column=[param]`, splitting at the first `=`.
    ///
    /// # Errors
    ///
    /// Returns a [`ColUnitError`] if the `=` is missing or either side is
    /// malformed.
    pub fn parse(text: &'a str) -> Result<Self, ColUnitError> {
        let (column, param) = text.split_once('=').ok_or(ColUnitError::MissingEquals)?;
        Ok(Self {
            column: Text::new(column).map_err(ColUnitError::Column)?,
            param: Param::parse(param).map_err(ColUnitError::Param)?,
        })
    }
}

impl fmt::Display for ColUnit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.column, self.param)
    }
}

/// Why a string is not a spectra reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpectraRefError {
    /// It does not start with `ms_run[n]:` for a positive `n`.
    MsRun,
    /// The reference after the colon is not a valid [`Text`].
    Reference(TextError),
}

impl fmt::Display for SpectraRefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MsRun => f.write_str("expected `ms_run[n]:`"),
            Self::Reference(e) => write!(f, "bad spectrum reference: {e}"),
        }
    }
}

impl core::error::Error for SpectraRefError {}

/// One `ms_run[n]:{reference}` item of `spectra_ref`.
///
/// Whether `reference` matches the run's `id_format` is a validator rule.
///
/// # Examples
///
/// ```
/// use mztab_m::SpectraRef;
///
/// let r = SpectraRef::parse("ms_run[2]:index=7")?;
/// assert_eq!(r.ms_run.get(), 2);
/// assert_eq!(r.reference.as_str(), "index=7");
/// assert_eq!(r.to_string(), "ms_run[2]:index=7");
/// # Ok::<(), mztab_m::SpectraRefError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpectraRef<'a> {
    /// The run that holds the spectrum.
    pub ms_run: MsRunId,
    /// The spectrum's native id within the run.
    pub reference: Text<'a>,
}

impl<'a> SpectraRef<'a> {
    /// Parses `ms_run[n]:{reference}`.
    ///
    /// # Errors
    ///
    /// Returns a [`SpectraRefError`] if the run prefix or the reference is
    /// malformed.
    pub fn parse(text: &'a str) -> Result<Self, SpectraRefError> {
        let (run, reference) = text.split_once("]:").ok_or(SpectraRefError::MsRun)?;
        let ms_run = run
            .strip_prefix("ms_run[")
            .and_then(parse_index)
            .and_then(MsRunId::new)
            .ok_or(SpectraRefError::MsRun)?;
        let reference = Text::new(reference).map_err(SpectraRefError::Reference)?;
        Ok(Self { ms_run, reference })
    }
}

impl fmt::Display for SpectraRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ms_run, self.reference)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{ColUnit, ColUnitError, SpectraRef, SpectraRefError};
    use crate::param::{ParamError, ParamErrorKind};
    use crate::value::TextError;
    use alloc::string::ToString as _;

    #[test]
    fn colunits() {
        assert_eq!(
            ColUnit::parse("x"),
            Err(ColUnitError::MissingEquals),
            "no ="
        );
        assert_eq!(
            ColUnit::parse("=[,,n,]"),
            Err(ColUnitError::Column(TextError::Empty)),
            "no column"
        );
        let bad = ParamError {
            kind: ParamErrorKind::MissingOpenBracket,
            offset: 0,
        };
        assert_eq!(
            ColUnit::parse("a=b"),
            Err(ColUnitError::Param(bad)),
            "bad param"
        );
        assert_eq!(
            ColUnit::parse("a=[,,n=1,]").map(|u| u.to_string()),
            Ok("a=[, , n=1, ]".into()),
            "first = splits"
        );
        assert_eq!(
            ColUnitError::MissingEquals.to_string(),
            "expected `column=[unit parameter]`",
            "msg"
        );
        assert_eq!(
            ColUnitError::Column(TextError::Empty).to_string(),
            "bad column name: text is empty",
            "msg"
        );
        assert_eq!(
            ColUnitError::Param(bad).to_string(),
            "bad unit: expected `[` (at byte 0)",
            "msg"
        );
    }

    #[test]
    fn spectra_refs() {
        for bad in [
            "index=1",
            "ms_run[0]:x",
            "ms_run[01]:x",
            "run[1]:x",
            "ms_run[x]:y",
            "ms_run[]:y",
        ] {
            assert_eq!(SpectraRef::parse(bad), Err(SpectraRefError::MsRun), "{bad}");
        }
        assert_eq!(
            SpectraRef::parse("ms_run[1]:"),
            Err(SpectraRefError::Reference(TextError::Empty)),
            "empty"
        );
        assert_eq!(
            SpectraRefError::MsRun.to_string(),
            "expected `ms_run[n]:`",
            "msg"
        );
        assert_eq!(
            SpectraRefError::Reference(TextError::Empty).to_string(),
            "bad spectrum reference: text is empty",
            "msg"
        );
        let ok = SpectraRef::parse("ms_run[12]:scan=3:x").expect("valid fixture");
        assert_eq!(
            (ok.ms_run.get(), ok.reference.as_str()),
            (12, "scan=3:x"),
            "parts"
        );
        assert_eq!(ok.to_string(), "ms_run[12]:scan=3:x", "display");
    }
}
