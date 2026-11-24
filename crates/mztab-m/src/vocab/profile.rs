//! mzTab-M document [`Profile`]s and [`ProhibitedProfile`] table combinations.

use core::fmt;
use core::str::FromStr;

use super::VocabError;

/// A file profile: which tables are present (SPEC §5.18).
///
/// The two prohibited combinations, `M+E` and `M+S+E`, are not
/// representable. `M+S+F` is representable, because a validator must be able
/// to infer and report it, but it is tolerated rather than declarable (see
/// [`Profile::is_declarable`] and DECISIONS D-022).
///
/// # Examples
///
/// ```
/// use mztab_m::Profile;
///
/// assert_eq!(Profile::MSF.to_string(), "M+S+F");
/// assert_eq!(Profile::MF.tables(), (false, true, false));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Profile {
    /// `M`: metadata only.
    M,
    /// `M+S`: metadata and the small molecule summary.
    MS,
    /// `M+F`: metadata and features.
    MF,
    /// `M+F+E`: metadata, features and evidence.
    MFE,
    /// `M+S+F`: tolerated for backwards compatibility, NOT RECOMMENDED.
    MSF,
    /// `M+S+F+E`: all four sections.
    MSFE,
}

/// A table combination the spec prohibits (SPEC §5.18.2).
///
/// # Examples
///
/// ```
/// use mztab_m::{ProhibitedProfile, Profile};
///
/// assert_eq!(Profile::from_tables(false, false, true), Err(ProhibitedProfile::ME));
/// assert_eq!(ProhibitedProfile::MSE.to_string(), "M+S+E");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProhibitedProfile {
    /// `M+E`: evidence without features.
    ME,
    /// `M+S+E`: summary and evidence without features.
    MSE,
}

impl fmt::Display for ProhibitedProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ME => "M+E",
            Self::MSE => "M+S+E",
        })
    }
}

impl Profile {
    /// Every profile, from poorest to richest.
    pub const ALL: [Self; 6] = [
        Self::M,
        Self::MS,
        Self::MF,
        Self::MFE,
        Self::MSF,
        Self::MSFE,
    ];

    /// Parses the `mzTab-profile` value. Case-sensitive (SPEC §6).
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Profile;
    ///
    /// assert_eq!(Profile::parse("M+S"), Some(Profile::MS));
    /// assert_eq!(Profile::parse("m+s"), None);
    /// assert_eq!(Profile::parse("M+E"), None);
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == text)
    }

    /// The profile of a file with the given tables present.
    ///
    /// # Errors
    ///
    /// Returns the prohibited combination for evidence without features.
    pub const fn from_tables(sml: bool, smf: bool, sme: bool) -> Result<Self, ProhibitedProfile> {
        match (sml, smf, sme) {
            (false, false, false) => Ok(Self::M),
            (true, false, false) => Ok(Self::MS),
            (false, true, false) => Ok(Self::MF),
            (false, true, true) => Ok(Self::MFE),
            (true, true, false) => Ok(Self::MSF),
            (true, true, true) => Ok(Self::MSFE),
            (false, false, true) => Err(ProhibitedProfile::ME),
            (true, false, true) => Err(ProhibitedProfile::MSE),
        }
    }

    /// Which of the SML, SMF and SME tables this profile has.
    #[must_use]
    pub const fn tables(self) -> (bool, bool, bool) {
        match self {
            Self::M => (false, false, false),
            Self::MS => (true, false, false),
            Self::MF => (false, true, false),
            Self::MFE => (false, true, true),
            Self::MSF => (true, true, false),
            Self::MSFE => (true, true, true),
        }
    }

    /// Whether a file may declare this profile in `mzTab-profile`. Only
    /// `M+S+F` may not: it is absent from the field's enumeration.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::Profile;
    ///
    /// assert!(Profile::MSFE.is_declarable());
    /// assert!(!Profile::MSF.is_declarable());
    /// ```
    #[must_use]
    pub const fn is_declarable(self) -> bool {
        !matches!(self, Self::MSF)
    }

    /// The profile as written in the file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M => "M",
            Self::MS => "M+S",
            Self::MF => "M+F",
            Self::MFE => "M+F+E",
            Self::MSF => "M+S+F",
            Self::MSFE => "M+S+F+E",
        }
    }
}

impl FromStr for Profile {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| VocabError::new("an mzTab-M profile"))
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Profile, ProhibitedProfile};
    use alloc::string::ToString as _;

    #[test]
    fn profiles() {
        for p in Profile::ALL {
            assert_eq!(Profile::parse(p.as_str()), Some(p), "{p}");
            assert_eq!(p.as_str().parse::<Profile>(), Ok(p), "from_str {p}");
            assert_eq!(p.to_string(), p.as_str(), "{p}");
            let (s, f, e) = p.tables();
            assert_eq!(Profile::from_tables(s, f, e), Ok(p), "{p}");
        }
        assert!(
            "bogus"
                .parse::<Profile>()
                .is_err_and(|e| e.to_string() == "expected an mzTab-M profile"),
            "error"
        );
        assert_eq!(
            Profile::from_tables(true, false, true),
            Err(ProhibitedProfile::MSE),
            "M+S+E"
        );
        assert_eq!(ProhibitedProfile::ME.to_string(), "M+E", "msg");
        assert_eq!(ProhibitedProfile::MSE.to_string(), "M+S+E", "msg");
        assert!(Profile::M.is_declarable(), "M");
        assert!(!Profile::MSF.is_declarable(), "M+S+F");
    }
}
