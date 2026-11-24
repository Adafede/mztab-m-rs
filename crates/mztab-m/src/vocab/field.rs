//! Closed field vocabularies: [`AmbiguityCode`], [`Datatype`], and [`PublicationItem`].

use core::fmt;
use core::str::FromStr;

use super::VocabError;
use crate::value::text::{Text, TextError};

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

impl FromStr for AmbiguityCode {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| VocabError::new("`1`, `2` or `3`"))
    }
}

impl fmt::Display for AmbiguityCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// `study_variable_group[n]-datatype`: how the group's study-variable
/// values are encoded.
///
/// The field reference types this field as a parameter, but its prose lists
/// a closed set of plain names, and the 2.1 example uses them (`xsd:integer`).
/// The prose wins (DECISIONS D-002, D-057).
///
/// # Examples
///
/// ```
/// use mztab_m::Datatype;
///
/// assert_eq!(Datatype::parse("xsd:dateTime"), Some(Datatype::DateTime));
/// assert_eq!(Datatype::Parameter.as_str(), "Parameter");
/// assert_eq!(Datatype::parse("xsd:float"), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Datatype {
    /// `xsd:string`.
    String,
    /// `xsd:integer`.
    Integer,
    /// `xsd:decimal`.
    Decimal,
    /// `xsd:boolean`.
    Boolean,
    /// `xsd:date`.
    Date,
    /// `xsd:time`.
    Time,
    /// `xsd:dateTime`.
    DateTime,
    /// `xsd:anyURI`.
    AnyUri,
    /// `Parameter`: values are parameters.
    Parameter,
}

impl Datatype {
    /// Every datatype.
    pub const ALL: [Self; 9] = [
        Self::String,
        Self::Integer,
        Self::Decimal,
        Self::Boolean,
        Self::Date,
        Self::Time,
        Self::DateTime,
        Self::AnyUri,
        Self::Parameter,
    ];

    /// Parses the datatype name. Case-sensitive.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.as_str() == text)
    }

    /// The name as written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "xsd:string",
            Self::Integer => "xsd:integer",
            Self::Decimal => "xsd:decimal",
            Self::Boolean => "xsd:boolean",
            Self::Date => "xsd:date",
            Self::Time => "xsd:time",
            Self::DateTime => "xsd:dateTime",
            Self::AnyUri => "xsd:anyURI",
            Self::Parameter => "Parameter",
        }
    }
}

impl FromStr for Datatype {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| VocabError::new("an `xsd:` datatype or `Parameter`"))
    }
}

impl fmt::Display for Datatype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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

    use super::{AmbiguityCode, Datatype, PublicationItem, PublicationItemError, PublicationKind};
    use crate::value::text::TextError;
    use alloc::string::ToString as _;

    #[test]
    fn ambiguity_codes() {
        for (text, code) in [
            ("1", AmbiguityCode::AmbiguousIdentification),
            ("2", AmbiguityCode::MultipleEvidenceStreams),
            ("3", AmbiguityCode::Both),
        ] {
            assert_eq!(AmbiguityCode::parse(text), Some(code), "{text}");
            assert_eq!(text.parse::<AmbiguityCode>(), Ok(code), "from_str {text}");
            assert_eq!(code.to_string(), text, "{text}");
        }
        assert_eq!(AmbiguityCode::parse("0"), None, "0");
        assert!("0".parse::<AmbiguityCode>().is_err());
    }

    #[test]
    fn datatypes() {
        for d in Datatype::ALL {
            assert_eq!(Datatype::parse(d.as_str()), Some(d), "{d}");
            assert_eq!(d.as_str().parse::<Datatype>(), Ok(d), "from_str {d}");
            assert_eq!(d.to_string(), d.as_str(), "{d}");
        }
        assert_eq!(
            Datatype::DateTime.as_str(),
            "xsd:dateTime",
            "camel case kept"
        );
        assert!("xsd:float".parse::<Datatype>().is_err());
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
