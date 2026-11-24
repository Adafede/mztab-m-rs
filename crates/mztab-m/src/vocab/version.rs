//! mzTab-M version numbers and validation rule-set selection.

use core::fmt;
use core::str::FromStr;

/// Why a string is not an mzTab-M version.
///
/// # Examples
///
/// ```
/// use mztab_m::{Version, VersionError};
///
/// assert_eq!("2.1-M".parse::<Version>(), Err(VersionError { offset: 3 }));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VersionError {
    /// Byte offset of the first character that does not fit
    /// `major.minor.patch-M`.
    pub offset: usize,
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "not an mzTab-M version `major.minor.patch-M` (unexpected input at byte {})",
            self.offset
        )
    }
}

impl core::error::Error for VersionError {}

/// An mzTab-M version, `major.minor.patch-M` (SPEC §6, "Versioning").
///
/// The JSON Schema's pattern allows one digit per part and any capital
/// suffix; this type accepts any number of digits and only `-M`, the
/// metabolomics suffix (DECISIONS D-016).
///
/// # Examples
///
/// ```
/// use mztab_m::Version;
///
/// let v: Version = "2.0.0-M".parse()?;
/// assert_eq!((v.major, v.minor, v.patch), (2, 0, 0));
/// assert_eq!(v.to_string(), "2.0.0-M");
/// assert!("2.0.0-P".parse::<Version>().is_err());
/// # Ok::<(), mztab_m::VersionError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Patch version.
    pub patch: u16,
}

impl Version {
    /// mzTab-M 2.0.0, the released version.
    pub const V2_0_0: Self = Self {
        major: 2,
        minor: 0,
        patch: 0,
    };

    /// mzTab-M 2.1.0, the pinned draft this crate targets.
    pub const V2_1_0: Self = Self {
        major: 2,
        minor: 1,
        patch: 0,
    };

    /// The rule set used to validate a file of this version.
    ///
    /// `2.0.x` uses the 2.0 rules. Everything else, including versions this
    /// crate does not know, uses the newest rules (DECISIONS D-003); the
    /// validator reports unknown versions.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{RuleSet, Version};
    ///
    /// assert_eq!("2.0.1-M".parse::<Version>()?.rule_set(), RuleSet::V2_0);
    /// assert_eq!("2.2.0-M".parse::<Version>()?.rule_set(), RuleSet::V2_1);
    /// # Ok::<(), mztab_m::VersionError>(())
    /// ```
    #[must_use]
    pub const fn rule_set(self) -> RuleSet {
        if self.major == 2 && self.minor == 0 {
            RuleSet::V2_0
        } else {
            RuleSet::V2_1
        }
    }

    /// Whether this is a version the crate has rules for: `2.0.0` or `2.1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Version;
    ///
    /// assert!(Version::V2_1_0.is_known());
    /// assert!(!"2.2.0-M".parse::<Version>()?.is_known());
    /// # Ok::<(), mztab_m::VersionError>(())
    /// ```
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.major == 2 && self.patch == 0 && (self.minor == 0 || self.minor == 1)
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        let (major, at) = number(bytes, 0)?;
        let at = expect(bytes, at, b'.')?;
        let (minor, at) = number(bytes, at)?;
        let at = expect(bytes, at, b'.')?;
        let (patch, at) = number(bytes, at)?;
        let at = expect(bytes, at, b'-')?;
        let at = expect(bytes, at, b'M')?;
        if at == bytes.len() {
            Ok(Self {
                major,
                minor,
                patch,
            })
        } else {
            Err(VersionError { offset: at })
        }
    }
}

fn number(bytes: &[u8], from: usize) -> Result<(u16, usize), VersionError> {
    let mut value: u16 = 0;
    let mut at = from;
    while let Some(digit) = bytes.get(at).filter(|b| b.is_ascii_digit()) {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(u16::from(digit - b'0')))
            .ok_or(VersionError { offset: at })?;
        at += 1;
    }
    if at == from {
        Err(VersionError { offset: at })
    } else {
        Ok((value, at))
    }
}

fn expect(bytes: &[u8], at: usize, byte: u8) -> Result<usize, VersionError> {
    if bytes.get(at) == Some(&byte) {
        Ok(at + 1)
    } else {
        Err(VersionError { offset: at })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}-M", self.major, self.minor, self.patch)
    }
}

/// The rule set a file is validated against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleSet {
    /// mzTab-M 2.0.0-M (released).
    V2_0,
    /// mzTab-M 2.1.0-M (the pinned draft).
    V2_1,
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{RuleSet, Version, VersionError};
    use alloc::string::ToString as _;

    #[test]
    fn versions() {
        let ok = [
            ("2.1.0-M", (2, 1, 0)),
            ("10.20.30-M", (10, 20, 30)),
            ("0.0.0-M", (0, 0, 0)),
        ];
        for (text, (major, minor, patch)) in ok {
            let expected = Version {
                major,
                minor,
                patch,
            };
            assert_eq!(text.parse::<Version>(), Ok(expected), "from_str {text}");
        }
        let bad = [
            ("", 0),
            ("2", 1),
            ("2.", 2),
            ("2.1", 3),
            ("2.1.", 4),
            ("2.1.0", 5),
            ("2.1.0-", 6),
            ("2.1.0-MM", 7),
            ("2.1.0-m", 6),
            ("x.1.0-M", 0),
            ("65536.0.0-M", 4),
            ("2.1.0 -M", 5),
        ];
        for (text, offset) in bad {
            assert_eq!(
                text.parse::<Version>(),
                Err(VersionError { offset }),
                "{text:?}"
            );
        }
        assert_eq!(
            "65535.0.0-M".parse::<Version>().map(|v| v.major),
            Ok(65535),
            "max"
        );
        assert_eq!(
            VersionError { offset: 1 }.to_string(),
            "not an mzTab-M version `major.minor.patch-M` (unexpected input at byte 1)",
            "msg"
        );
    }

    #[test]
    fn rule_sets() {
        let v = |major, minor, patch| Version {
            major,
            minor,
            patch,
        };
        assert_eq!(v(2, 0, 0).rule_set(), RuleSet::V2_0, "2.0");
        assert_eq!(v(2, 1, 0).rule_set(), RuleSet::V2_1, "2.1");
        assert_eq!(
            v(1, 0, 0).rule_set(),
            RuleSet::V2_1,
            "1.0 unknown -> newest"
        );
        assert_eq!(
            v(3, 0, 0).rule_set(),
            RuleSet::V2_1,
            "3.0 unknown -> newest"
        );
        assert!(Version::V2_0_0.is_known(), "2.0.0");
        for unknown in [v(2, 0, 1), v(2, 2, 0), v(1, 0, 0), v(3, 1, 0)] {
            assert!(!unknown.is_known(), "{unknown}");
        }
    }
}
