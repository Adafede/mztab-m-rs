//! Closed vocabularies: [`Version`], [`Profile`], [`Section`],
//! [`AmbiguityCode`] and [`PublicationItem`].
//!
//! ```
//! use mztab_m::{Profile, RuleSet, Version};
//!
//! let version = Version::parse("2.1.0-M")?;
//! assert_eq!(version.rule_set(), RuleSet::V2_1);
//!
//! assert_eq!(Profile::parse("M+F+E"), Some(Profile::MFE));
//! assert_eq!(Profile::from_tables(true, true, true), Ok(Profile::MSFE));
//! # Ok::<(), mztab_m::VersionError>(())
//! ```

use core::fmt;

use crate::value::{Text, TextError};

/// Why a string is not an mzTab-M version.
///
/// # Examples
///
/// ```
/// use mztab_m::{Version, VersionError};
///
/// assert_eq!(Version::parse("2.1-M"), Err(VersionError { offset: 3 }));
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
/// let v = Version::parse("2.0.0-M")?;
/// assert_eq!((v.major, v.minor, v.patch), (2, 0, 0));
/// assert_eq!(v.to_string(), "2.0.0-M");
/// assert!(Version::parse("2.0.0-P").is_err());
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

    /// Parses `major.minor.patch-M`.
    ///
    /// # Errors
    ///
    /// Returns a [`VersionError`] with the offset of the first bad byte.
    pub fn parse(text: &str) -> Result<Self, VersionError> {
        let bytes = text.as_bytes();
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
    /// assert_eq!(Version::parse("2.0.1-M")?.rule_set(), RuleSet::V2_0);
    /// assert_eq!(Version::parse("2.2.0-M")?.rule_set(), RuleSet::V2_1);
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
    /// assert!(!Version::parse("2.2.0-M")?.is_known());
    /// # Ok::<(), mztab_m::VersionError>(())
    /// ```
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.major == 2 && self.patch == 0 && (self.minor == 0 || self.minor == 1)
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

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A section of an mzTab-M file, identified by its line prefixes.
///
/// # Examples
///
/// ```
/// use mztab_m::Section;
///
/// assert_eq!(Section::from_prefix("SFH"), Some((Section::Smf, true)));
/// assert_eq!(Section::from_prefix("SME"), Some((Section::Sme, false)));
/// assert_eq!(Section::from_prefix("COM"), None);
/// assert_eq!(Section::Sml.header_prefix(), Some("SMH"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    /// Metadata, `MTD`.
    Mtd,
    /// Small molecule summary, `SMH` / `SML`.
    Sml,
    /// Small molecule features, `SFH` / `SMF`.
    Smf,
    /// Small molecule evidence, `SEH` / `SME`.
    Sme,
}

impl Section {
    /// The sections in their mandatory file order.
    pub const ALL: [Self; 4] = [Self::Mtd, Self::Sml, Self::Smf, Self::Sme];

    /// The prefix of the section's data lines.
    #[must_use]
    pub const fn row_prefix(self) -> &'static str {
        match self {
            Self::Mtd => "MTD",
            Self::Sml => "SML",
            Self::Smf => "SMF",
            Self::Sme => "SME",
        }
    }

    /// The prefix of the section's header line; `None` for metadata.
    #[must_use]
    pub const fn header_prefix(self) -> Option<&'static str> {
        match self {
            Self::Mtd => None,
            Self::Sml => Some("SMH"),
            Self::Smf => Some("SFH"),
            Self::Sme => Some("SEH"),
        }
    }

    /// The section a line prefix belongs to, and whether it is the header.
    /// Case-sensitive (SPEC §6). `COM` belongs to no section.
    #[must_use]
    pub fn from_prefix(prefix: &str) -> Option<(Self, bool)> {
        Self::ALL.into_iter().find_map(|s| {
            if s.row_prefix() == prefix {
                Some((s, false))
            } else {
                (s.header_prefix() == Some(prefix)).then_some((s, true))
            }
        })
    }
}

/// `SME_ID_REF_ambiguity_code`, required when an SMF row has several
/// `SME_ID_REFS`.
///
/// # Examples
///
/// ```
/// use mztab_m::AmbiguityCode;
///
/// assert_eq!(AmbiguityCode::parse("3"), Some(AmbiguityCode::Both));
/// assert_eq!(AmbiguityCode::parse("4"), None);
/// assert_eq!(AmbiguityCode::Both.code(), 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AmbiguityCode {
    /// `1`: ambiguous identification.
    AmbiguousIdentification,
    /// `2`: only different evidence streams for the same molecule, no
    /// ambiguity.
    MultipleEvidenceStreams,
    /// `3`: both ambiguous identification and multiple evidence streams.
    Both,
}

impl AmbiguityCode {
    /// Parses `1`, `2` or `3`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "1" => Some(Self::AmbiguousIdentification),
            "2" => Some(Self::MultipleEvidenceStreams),
            "3" => Some(Self::Both),
            _ => None,
        }
    }

    /// The numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::AmbiguousIdentification => 1,
            Self::MultipleEvidenceStreams => 2,
            Self::Both => 3,
        }
    }
}

impl fmt::Display for AmbiguityCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// The qualifier of a publication item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PublicationKind {
    /// `pubmed:`, a `PubMed` id.
    Pubmed,
    /// `doi:`, a DOI.
    Doi,
    /// `uri:`, any URI.
    Uri,
}

impl PublicationKind {
    /// The qualifier as written, without the colon.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pubmed => "pubmed",
            Self::Doi => "doi",
            Self::Uri => "uri",
        }
    }
}

/// Why a string is not a publication item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PublicationItemError {
    /// The qualifier is not `pubmed`, `doi` or `uri`, or the colon is missing.
    Qualifier,
    /// The accession after the colon is not a valid [`Text`].
    Accession(TextError),
}

impl fmt::Display for PublicationItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Qualifier => f.write_str("expected `pubmed:`, `doi:` or `uri:`"),
            Self::Accession(e) => write!(f, "bad publication accession: {e}"),
        }
    }
}

impl core::error::Error for PublicationItemError {}

/// One `qualifier:accession` item of a `publication[n]` list.
///
/// # Examples
///
/// ```
/// use mztab_m::{PublicationItem, PublicationKind};
///
/// let item = PublicationItem::parse("doi:10.1007/978-1-60761-987-1_6")?;
/// assert_eq!(item.kind, PublicationKind::Doi);
/// assert_eq!(item.accession.as_str(), "10.1007/978-1-60761-987-1_6");
/// assert_eq!(item.to_string(), "doi:10.1007/978-1-60761-987-1_6");
/// # Ok::<(), mztab_m::PublicationItemError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PublicationItem<'a> {
    /// The qualifier.
    pub kind: PublicationKind,
    /// The accession, after the first colon.
    pub accession: Text<'a>,
}

impl<'a> PublicationItem<'a> {
    /// Parses `qualifier:accession`. Qualifiers are case-sensitive.
    ///
    /// # Errors
    ///
    /// Returns a [`PublicationItemError`] for an unknown qualifier or an
    /// accession that is not a valid [`Text`].
    pub fn parse(text: &'a str) -> Result<Self, PublicationItemError> {
        let (qualifier, accession) = text
            .split_once(':')
            .ok_or(PublicationItemError::Qualifier)?;
        let kind = [
            PublicationKind::Pubmed,
            PublicationKind::Doi,
            PublicationKind::Uri,
        ]
        .into_iter()
        .find(|k| k.as_str() == qualifier)
        .ok_or(PublicationItemError::Qualifier)?;
        let accession = Text::new(accession).map_err(PublicationItemError::Accession)?;
        Ok(Self { kind, accession })
    }
}

impl fmt::Display for PublicationItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.accession)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{
        AmbiguityCode, Profile, ProhibitedProfile, PublicationItem, PublicationItemError,
        PublicationKind, RuleSet, Section, Version, VersionError,
    };
    use crate::value::TextError;
    use alloc::string::ToString as _;

    #[test]
    fn versions() {
        let ok = [
            ("2.1.0-M", (2, 1, 0)),
            ("10.20.30-M", (10, 20, 30)),
            ("0.0.0-M", (0, 0, 0)),
        ];
        for (text, (major, minor, patch)) in ok {
            assert_eq!(
                Version::parse(text),
                Ok(Version {
                    major,
                    minor,
                    patch
                }),
                "{text}"
            );
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
                Version::parse(text),
                Err(VersionError { offset }),
                "{text:?}"
            );
        }
        assert_eq!(
            Version::parse("65535.0.0-M").map(|v| v.major),
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

    #[test]
    fn profiles() {
        for p in Profile::ALL {
            assert_eq!(Profile::parse(p.as_str()), Some(p), "{p}");
            assert_eq!(p.to_string(), p.as_str(), "{p}");
            let (s, f, e) = p.tables();
            assert_eq!(Profile::from_tables(s, f, e), Ok(p), "{p}");
        }
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

    #[test]
    fn sections() {
        for s in Section::ALL {
            assert_eq!(
                Section::from_prefix(s.row_prefix()),
                Some((s, false)),
                "{s:?}"
            );
            if let Some(h) = s.header_prefix() {
                assert_eq!(Section::from_prefix(h), Some((s, true)), "{s:?}");
            }
        }
        assert_eq!(Section::from_prefix("mtd"), None, "case-sensitive");
        assert_eq!(Section::Mtd.header_prefix(), None, "no MTD header");
        assert_eq!(Section::Sml.header_prefix(), Some("SMH"), "SML header");
    }

    #[test]
    fn ambiguity_codes() {
        for (text, code) in [
            ("1", AmbiguityCode::AmbiguousIdentification),
            ("2", AmbiguityCode::MultipleEvidenceStreams),
            ("3", AmbiguityCode::Both),
        ] {
            assert_eq!(AmbiguityCode::parse(text), Some(code), "{text}");
            assert_eq!(code.to_string(), text, "{text}");
        }
        assert_eq!(AmbiguityCode::parse("0"), None, "0");
    }

    #[test]
    fn publication_items() {
        let item = PublicationItem::parse("uri:https://x.org/a:b");
        assert_eq!(
            item.as_ref().map(|i| (i.kind, i.accession.as_str())),
            Ok((PublicationKind::Uri, "https://x.org/a:b")),
            "first colon"
        );
        assert_eq!(
            PublicationItem::parse("pubmed:1").map(|i| i.to_string()),
            Ok("pubmed:1".into()),
            "pubmed"
        );
        assert_eq!(
            PublicationItem::parse("PMID:1"),
            Err(PublicationItemError::Qualifier),
            "unknown"
        );
        assert_eq!(
            PublicationItem::parse("doi"),
            Err(PublicationItemError::Qualifier),
            "no colon"
        );
        assert_eq!(
            PublicationItem::parse("doi:"),
            Err(PublicationItemError::Accession(TextError::Empty)),
            "empty"
        );
        assert_eq!(
            PublicationItemError::Qualifier.to_string(),
            "expected `pubmed:`, `doi:` or `uri:`",
            "msg"
        );
        assert_eq!(
            PublicationItemError::Accession(TextError::Empty).to_string(),
            "bad publication accession: text is empty",
            "msg"
        );
    }
}
