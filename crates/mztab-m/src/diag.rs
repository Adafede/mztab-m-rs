//! Findings: what the reader and the validators report.
//!
//! A [`Finding`] pairs a stable [`Code`] with a [`Severity`], a precise
//! [`Location`] and structured [`Detail`]. Nothing is pre-formatted: the
//! English message is rendered by `Display` only when asked for.
//!
//! ```
//! use mztab_m::{Code, Detail, Finding, Location, Severity};
//!
//! let finding = Finding::new(Code::ParamSyntax, Location::text(12, 5), Detail::None);
//! assert_eq!(finding.severity, Severity::Error);
//! assert_eq!(finding.code.id(), "MZT-S002");
//! assert_eq!(finding.to_string(), "error[MZT-S002] line 12, column 5: invalid parameter");
//! ```
//!
//! # Codes are stable
//!
//! A code's id is never reused or renumbered, and a retired code keeps its
//! number. jmzTab-m codes appear only as a mapping, [`Code::jmztab`], used by
//! the differential tests (DECISIONS J-17).

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

use crate::meta::MtdKey;
use crate::param::ParamError;
use crate::value::{NumberError, TextError};
use crate::vocab::{Profile, ProhibitedProfile, VersionError};

/// How serious a finding is. Ordered from least to most serious.
///
/// # Examples
///
/// ```
/// use mztab_m::Severity;
///
/// assert!(Severity::Error > Severity::Warning);
/// assert_eq!(Severity::Info.to_string(), "info");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth knowing; not a defect.
    Info,
    /// A SHOULD-level violation, or a likely mistake.
    Warning,
    /// A MUST-level violation: the file does not conform.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// The RFC 2119 level of the rule a code checks.
///
/// # Examples
///
/// ```
/// use mztab_m::{Requirement, Severity};
///
/// assert_eq!(Requirement::Should.severity(), Severity::Warning);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Requirement {
    /// MUST / MUST NOT / REQUIRED.
    Must,
    /// SHOULD / SHOULD NOT / RECOMMENDED.
    Should,
    /// MAY / OPTIONAL, or informational.
    May,
}

impl Requirement {
    /// The default severity of a violation: MUST gives an error, SHOULD a
    /// warning, MAY an informational note.
    #[must_use]
    pub const fn severity(self) -> Severity {
        match self {
            Self::Must => Severity::Error,
            Self::Should => Severity::Warning,
            Self::May => Severity::Info,
        }
    }
}

/// The validation layer a code belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// The lexical form of lines and cells, including each cell's grammar.
    /// Code prefix `S`.
    Syntax,
    /// Fields, types, cross-references and profiles on the typed model.
    /// Code prefix `T`.
    Structure,
    /// Controlled-vocabulary rules. Code prefix `X`.
    Semantic,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "syntax",
            Self::Structure => "structure",
            Self::Semantic => "semantic",
        })
    }
}

/// The path of a specification document: [`SpecDoc::path`] and the
/// generated docs of each [`Code`].
macro_rules! spec_path {
    (Format) => {
        "specification_documents/mzTab_format_specification_2_1-M.adoc"
    };
    (FieldReference) => {
        "specification_documents/mzTab_m_2_1_schema.adoc"
    };
}

/// A specification document at the pinned commit
/// ([`SPEC_COMMIT`](crate::SPEC_COMMIT)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecDoc {
    /// `mzTab_format_specification_2_1-M.adoc`, the prose specification.
    Format,
    /// `mzTab_m_2_1_schema.adoc`, the field reference.
    FieldReference,
}

impl SpecDoc {
    /// The document's path in the specification repository.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Format => spec_path!(Format),
            Self::FieldReference => spec_path!(FieldReference),
        }
    }
}

/// A reference to the section of the specification that states a rule.
///
/// `Display` renders a URL to the anchor at the pinned commit.
///
/// # Examples
///
/// ```
/// use mztab_m::Code;
///
/// let spec = Code::NumberSyntax.spec_ref();
/// assert!(spec.to_string().ends_with("#encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpecRef {
    /// The document.
    pub doc: SpecDoc,
    /// The `[[anchor]]` id in the document.
    pub anchor: &'static str,
}

impl fmt::Display for SpecRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "https://github.com/HUPO-PSI/mzTab-M/blob/{}/{}#{}",
            crate::SPEC_COMMIT,
            self.doc.path(),
            self.anchor
        )
    }
}

macro_rules! codes {
    ($(
        $(#[$doc:meta])*
        $name:ident = $id:literal, $layer:ident, $req:ident, $docf:ident # $anchor:literal,
            jmztab [$($j:literal),*], $msg:literal;
    )+) => {
        /// A stable finding code.
        ///
        /// Each code knows its id, layer, RFC 2119 level, the spec section
        /// that states its rule, and the equivalent jmzTab-m codes. Each
        /// variant's documentation links the section at the pinned commit.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum Code {
            $(
                $(#[$doc])*
                ///
                #[doc = concat!(
                    "`", $id, "`, ", stringify!($layer), " layer, ", stringify!($req),
                    ". Rule: [", $anchor, "](https://github.com/HUPO-PSI/mzTab-M/blob/",
                    crate::spec_commit!(), "/", spec_path!($docf), "#", $anchor, ")."
                )]
                $name,
            )+
        }

        impl Code {
            /// Every code, in id order.
            pub const ALL: &'static [Self] = &[$(Self::$name),+];

            /// The stable id, e.g. `MZT-S002`.
            #[must_use]
            pub const fn id(self) -> &'static str {
                match self { $(Self::$name => $id,)+ }
            }

            /// The layer that reports this code.
            #[must_use]
            pub const fn layer(self) -> Layer {
                match self { $(Self::$name => Layer::$layer,)+ }
            }

            /// The RFC 2119 level of the rule.
            #[must_use]
            pub const fn requirement(self) -> Requirement {
                match self { $(Self::$name => Requirement::$req,)+ }
            }

            /// Where the specification states the rule.
            #[must_use]
            pub const fn spec_ref(self) -> SpecRef {
                match self {
                    $(Self::$name => SpecRef { doc: SpecDoc::$docf, anchor: $anchor },)+
                }
            }

            /// The jmzTab-m codes that check the same rule, if any.
            #[must_use]
            pub const fn jmztab(self) -> &'static [u16] {
                match self { $(Self::$name => &[$($j),*],)+ }
            }

            /// A short, fixed English summary of the rule.
            #[must_use]
            pub const fn summary(self) -> &'static str {
                match self { $(Self::$name => $msg,)+ }
            }
        }
    };
}

codes! {
    /// A line does not start with a known three-letter prefix and a tab.
    LinePrefix = "MZT-S001", Syntax, Must, Format # "format-specification",
        jmztab [1000], "unknown line prefix";
    /// A cell that should hold a parameter does not follow the grammar.
    ParamSyntax = "MZT-S002", Syntax, Must, Format # "format-specification",
        jmztab [1012, 1013], "invalid parameter";
    /// A cell that should hold a number is not one.
    NumberSyntax = "MZT-S003", Syntax, Must, Format # "encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors",
        jmztab [1018, 1019, 1022, 1033], "invalid number";
    /// A number uses scientific notation or infinity, which mzTab forbids.
    NumberForm = "MZT-S004", Syntax, Must, Format # "encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors",
        jmztab [], "number in scientific notation or infinity";
    /// A cell is empty or cannot be carried as text.
    TextCell = "MZT-S005", Syntax, Must, Format # "encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors",
        jmztab [2000], "empty or malformed cell";
    /// `mzTab-version` is not `major.minor.patch-M`.
    VersionSyntax = "MZT-S006", Syntax, Must, FieldReference # "mztab-version",
        jmztab [1031], "invalid mzTab-version";
    /// A cell does not have the form its column or key requires (a
    /// reference, a publication item, a unit mapping, a spectrum reference,
    /// an ambiguity code).
    ValueSyntax = "MZT-S007", Syntax, Must, Format # "format-specification",
        jmztab [1014, 1015, 1026, 1034, 1036], "invalid value";
    /// A line is not valid UTF-8.
    Utf8 = "MZT-S008", Syntax, Must, Format # "format-specification",
        jmztab [], "line is not valid UTF-8";
    /// The input starts with a UTF-8 byte order mark. It is stripped.
    Bom = "MZT-S009", Syntax, May, Format # "format-specification",
        jmztab [1000], "byte order mark at start of input";
    /// A table row has a different number of cells than its header.
    CellCount = "MZT-S010", Syntax, Must, FieldReference # "sections",
        jmztab [1001], "cell count differs from the header";
    /// An `MTD` line has no value cell, or more than one non-empty cell after
    /// the key.
    MtdCells = "MZT-S011", Syntax, Must, FieldReference # "metadata-section",
        jmztab [1008], "metadata line must have exactly a key and a value";
    /// An `MTD` line ends with empty cells.
    TrailingEmptyCells = "MZT-S012", Syntax, May, FieldReference # "metadata-section",
        jmztab [], "empty cells after the metadata value";
    /// A cell or list item has leading or trailing spaces. Non-string values
    /// are read without them.
    CellWhitespace = "MZT-S013", Syntax, Should, Format # "format-specification",
        jmztab [], "leading or trailing spaces";
    /// The input is not JSON (RFC 8259). Feature `json`.
    JsonSyntax = "MZT-S014", Syntax, Must, Format # "format-specification",
        jmztab [], "not valid JSON";
    /// A JSON value does not have the form its place in the JSON schema
    /// requires. Feature `json`.
    JsonShape = "MZT-S015", Syntax, Must, Format # "format-specification",
        jmztab [], "JSON value of the wrong form";
    /// A JSON member that the JSON schema does not define. It is ignored.
    /// Feature `json`.
    JsonUnknownKey = "MZT-S016", Syntax, Should, Format # "format-specification",
        jmztab [], "unknown JSON member, ignored";
    /// The JSON uses another dialect than the JSON schema's (`snake_case`
    /// keys, as `MetaboScape` writes). Nothing is read. Feature `json`.
    JsonDialect = "MZT-S017", Syntax, Must, Format # "format-specification",
        jmztab [], "JSON dialect other than the JSON schema's";
    /// A JSON string holds a tab or line feed (or ends a line with a
    /// carriage return), which mzTab cannot carry. The line is dropped.
    /// Feature `json`.
    JsonText = "MZT-S018", Syntax, Must, Format # "format-specification",
        jmztab [], "JSON string with a tab or line break";
    /// `mzTab-profile` is not one of the declared profiles.
    ProfileValue = "MZT-T001", Structure, Must, FieldReference # "mztab-profile",
        jmztab [1038], "invalid mzTab-profile";
    /// A metadata key is not defined by the specification.
    UnknownMtdKey = "MZT-T002", Structure, Must, FieldReference # "metadata-section",
        jmztab [1009], "unknown metadata key";
    /// A metadata key appears more than once.
    DuplicateMtdKey = "MZT-T003", Structure, Must, FieldReference # "metadata-section",
        jmztab [2010], "duplicate metadata key";
    /// A table column is not defined for its table.
    UnknownColumn = "MZT-T004", Structure, Must, FieldReference # "sections",
        jmztab [2034], "unknown column";
    /// A column label appears twice in one header.
    DuplicateColumn = "MZT-T005", Structure, Must, FieldReference # "sections",
        jmztab [2062], "duplicate column";
    /// Sections are not in the order MTD, SML, SMF, SME.
    SectionOrder = "MZT-T006", Structure, Must, Format # "profiles",
        jmztab [2002], "section out of order";
    /// A table row appears before its header line.
    RowBeforeHeader = "MZT-T007", Structure, Must, Format # "format-specification",
        jmztab [2004], "row before its header";
    /// A table header line appears more than once.
    DuplicateHeader = "MZT-T008", Structure, Must, Format # "format-specification",
        jmztab [2003], "repeated header line";
    /// A metadata field the specification requires is missing.
    MissingField = "MZT-T009", Structure, Must, FieldReference # "metadata-section",
        jmztab [2008], "a mandatory metadata field is missing";
    /// `mzTab-version` names a version this crate has no rules for.
    UnknownVersion = "MZT-T010", Structure, Should, FieldReference # "mztab-version",
        jmztab [], "unknown mzTab-M version; newest rules applied";
    /// No `mzTab-profile` is declared; the profile is inferred and reported.
    ProfileInferred = "MZT-T011", Structure, May, Format # "profiles",
        jmztab [2052], "profile inferred from the tables present";
    /// The tables present do not match the declared `mzTab-profile`.
    ProfileMismatch = "MZT-T012", Structure, Must, FieldReference # "table-profiles",
        jmztab [2053, 2054, 2055], "tables present do not match the declared profile";
    /// The tables form a prohibited combination (`M+E` or `M+S+E`).
    ProhibitedProfile = "MZT-T013", Structure, Must, Format # "profiles-not-supported",
        jmztab [2056], "prohibited table combination";
    /// The tables form profile `M+S+F`, which is tolerated but not recommended.
    DiscouragedProfile = "MZT-T014", Structure, Should, Format # "profiles-not-supported",
        jmztab [2057], "profile M+S+F is not recommended";
    /// A table has a header and no rows (error when a profile is declared,
    /// warning when inferred).
    EmptyTable = "MZT-T015", Structure, Must, Format # "profiles",
        jmztab [], "header without rows; omit the section instead";
    /// A profile-dependent metadata field is present although its table is not.
    ProfileField = "MZT-T016", Structure, May, Format # "profile-metadata-requirements",
        jmztab [2058], "field describes a table that is not present";
    /// A reference names an element or row that is not defined.
    DanglingReference = "MZT-T017", Structure, Must, FieldReference # "metadata-section",
        jmztab [2005, 2006, 2007, 2047, 2068], "reference to an element that is not defined";
    /// A row id (`SML_ID`, `SMF_ID`, `SME_ID`) is not unique in its table.
    DuplicateRowId = "MZT-T018", Structure, Must, FieldReference # "sections",
        jmztab [], "duplicate row id";
    /// The indices of an element are not `1..n` without gaps.
    IndexGap = "MZT-T019", Structure, Should, FieldReference # "metadata-section",
        jmztab [], "element indices are not contiguous from 1";
    /// A study variable belongs to no group, or to more than one, while groups exist.
    StudyVariableGrouping = "MZT-T020", Structure, Must, FieldReference # "study_variable_group1-n",
        jmztab [2007, 2028], "study variable not in exactly one group";
    /// A 2.1 file defines no `study_variable_group`.
    NoStudyVariableGroup = "MZT-T021", Structure, Should, FieldReference # "study_variable_group1-n",
        jmztab [2008], "no study_variable_group defined";
    /// A row breaks the cross-reference obligations of its profile.
    CrossReference = "MZT-T022", Structure, Must, Format # "table-profile-crossrefs",
        jmztab [2059, 2060, 2061], "cross-reference required or forbidden by the profile";
    /// `SME_ID_REF_ambiguity_code` is set without several `SME_ID_REFS`, or missing with them.
    AmbiguityCode = "MZT-T023", Structure, Must, FieldReference # "smf-sme_id_ref_ambiguity_code",
        jmztab [], "ambiguity code must be set exactly when SME_ID_REFS has several entries";
    /// A table lacks a mandatory column.
    MissingColumn = "MZT-T024", Structure, Must, FieldReference # "sections",
        jmztab [1004, 2009], "a mandatory column is missing";
    /// Table columns are not in the specified order.
    ColumnOrder = "MZT-T025", Structure, Should, FieldReference # "sections",
        jmztab [], "columns are not in the specified order";
    /// An `opt_` column comes before a column that is not optional.
    OptColumnPosition = "MZT-T026", Structure, Should, Format # "adding-optional-columns",
        jmztab [], "optional column before a defined column";
    /// A column that is not nullable holds `null` (info for
    /// `best_id_confidence_value`).
    NullValue = "MZT-T027", Structure, Must, Format # "encoding-missing-values-zeroes-nulls-infinity-and-calculation-errors",
        jmztab [2000], "null in a column that is not nullable";
    /// A model value failed to type, or is a number in a form mzTab forbids.
    InvalidValue = "MZT-T028", Structure, Must, FieldReference # "sections",
        jmztab [], "value does not have the type its field requires";
    /// An SML row has no reliability.
    ReliabilityMissing = "MZT-T029", Structure, Should, FieldReference # "sml-reliability",
        jmztab [], "reliability is null";
    /// An SML reliability is not `1`-`4` and no other scheme is declared.
    ReliabilityValue = "MZT-T030", Structure, Must, FieldReference # "sml-reliability",
        jmztab [1034], "reliability is not a level of the default scheme (1-4)";
    /// An `ms_run` has a hash but no `hash_method`.
    HashMethod = "MZT-T031", Structure, Must, FieldReference # "ms_run1-n-hash_method",
        jmztab [2033], "hash without hash_method";
    /// A `colunit-*` entry names a column its table does not have.
    ColunitColumn = "MZT-T032", Structure, Must, FieldReference # "colunit-small_molecule",
        jmztab [1029], "colunit names a column that is not in the table";
    /// An adduct ion does not match the specified pattern.
    AdductIon = "MZT-T033", Structure, Must, FieldReference # "sml-adduct_ions",
        jmztab [1034], "adduct ion does not match `[nM+X]n+`";
    /// A contact ORCID does not match the specified pattern.
    Orcid = "MZT-T034", Structure, Must, FieldReference # "contact1-n-orcid",
        jmztab [1037], "ORCID does not match `0000-0000-0000-000X`";
    /// A study variable value does not match the `datatype` of its group.
    StudyVariableDatatype = "MZT-T035", Structure, Must, FieldReference # "study_variable_group1-n-datatype",
        jmztab [], "study variable value does not match its group datatype";
    /// A value typed URI is not an RFC 3986 URI (with a scheme, no spaces
    /// or backslashes, `%` only before two hex digits).
    Uri = "MZT-T036", Structure, Must, Format # "format-specification",
        jmztab [], "not a URI (RFC 3986)";
    /// A contact e-mail address is not of the form `local@domain.tld`.
    Email = "MZT-T037", Structure, Should, FieldReference # "contact1-n-email",
        jmztab [], "not an e-mail address";
    /// A spectrum reference does not have the form its run's `id_format`
    /// term prescribes (spec table 1, e.g. `index=N` for "multiple peak
    /// list nativeID format").
    SpectraRefFormat = "MZT-T038", Structure, Should, Format # "table-1",
        jmztab [], "spectrum reference does not match the run's id_format";
    /// A CV parameter's label is not declared by any `cv[n]-label`.
    CvUndeclared = "MZT-X001", Semantic, Should, FieldReference # "cv1-n-label",
        jmztab [3013], "CV label is not declared";
    /// A declared CV is not used by any parameter or column.
    CvUnused = "MZT-X002", Semantic, May, FieldReference # "cv1-n-label",
        jmztab [3014], "declared CV is not used";
    /// An accession has no `PREFIX:` part. (The prefix need not equal the
    /// CV label: the spec's own example is `[MIRIAM, MIR:00100079, HMDB, ]`.)
    AccessionPrefix = "MZT-X003", Semantic, Should, Format # "format-specification",
        jmztab [1035, 3016], "accession has no `PREFIX:` part";
    /// A vocabulary the lookup holds has no term with this accession.
    UnknownTerm = "MZT-X004", Semantic, Must, Format # "the-psi-mass-spectrometry-controlled-vocabulary-cv",
        jmztab [], "unknown CV term";
    /// A parameter's name differs from its term's name.
    TermName = "MZT-X005", Semantic, Should, Format # "the-psi-mass-spectrometry-controlled-vocabulary-cv",
        jmztab [], "parameter name differs from the CV term name";
    /// A parameter uses an obsolete term.
    ObsoleteTerm = "MZT-X006", Semantic, Should, Format # "the-psi-mass-spectrometry-controlled-vocabulary-cv",
        jmztab [], "obsolete CV term";
    /// A CV parameter is not allowed by its field's CV-mapping rule. The
    /// finding's severity is the rule's level.
    CvRule = "MZT-X007", Semantic, Must, Format # "format-specification",
        jmztab [3005, 3006, 3007, 3008, 3009, 3010, 3011], "CV term not allowed by the CV-mapping rule";
    /// A term that the CV-mapping rule does not let repeat appears twice in
    /// one element or row.
    CvRuleRepeat = "MZT-X008", Semantic, Should, Format # "format-specification",
        jmztab [], "CV term repeated where the CV-mapping rule forbids it";
    /// A CV parameter matches more than one term of an `XOR` CV-mapping rule.
    CvRuleXor = "MZT-X009", Semantic, Must, Format # "format-specification",
        jmztab [], "CV term matches several exclusive terms of the CV-mapping rule";
    /// A declared CV version differs from the version of the vocabulary
    /// used for checking.
    CvVersion = "MZT-X010", Semantic, May, FieldReference # "cv1-n-version",
        jmztab [], "declared CV version differs from the checked vocabulary";
    /// A CV parameter's value is not of the type its term declares
    /// (PSI-MS `value-type`, e.g. `xsd:int` for "ms level").
    TermValueType = "MZT-X011", Semantic, Must, Format # "the-psi-mass-spectrometry-controlled-vocabulary-cv",
        jmztab [], "parameter value is not of the term's value type";
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// One step of a [`Path`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    /// A named field or object key.
    Key(Cow<'static, str>),
    /// A position in a list or table. Zero-based in a JSON pointer, as JSON
    /// arrays are; one-based in a model path, as mzTab indices are.
    Index(u32),
}

/// A path into a JSON document or into the typed model.
///
/// # Examples
///
/// ```
/// use mztab_m::{Location, Path};
///
/// let path = Path::new().key("smallMoleculeSummary").index(3).key("chemical_name");
/// assert_eq!(Location::Json(path.clone()).to_string(), "/smallMoleculeSummary/3/chemical_name");
/// assert_eq!(Location::Model(path).to_string(), "smallMoleculeSummary[3].chemical_name");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Path(Vec<Segment>);

impl Path {
    /// The empty path: the document root.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends a key.
    #[must_use]
    pub fn key(mut self, key: impl Into<Cow<'static, str>>) -> Self {
        self.0.push(Segment::Key(key.into()));
        self
    }

    /// Appends an index.
    #[must_use]
    pub fn index(mut self, index: u32) -> Self {
        self.0.push(Segment::Index(index));
        self
    }

    /// The segments, from the root.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// Writes an RFC 6901 JSON pointer, escaping `~` and `/` in keys.
    fn fmt_pointer(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.0 {
            f.write_str("/")?;
            match segment {
                Segment::Key(key) => {
                    for c in key.chars() {
                        match c {
                            '~' => f.write_str("~0")?,
                            '/' => f.write_str("~1")?,
                            c => fmt::Write::write_char(f, c)?,
                        }
                    }
                }
                Segment::Index(i) => write!(f, "{i}")?,
            }
        }
        Ok(())
    }

    /// Writes `a.b[3].c`.
    fn fmt_model(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (n, segment) in self.0.iter().enumerate() {
            match segment {
                Segment::Key(key) if n == 0 => f.write_str(key)?,
                Segment::Key(key) => write!(f, ".{key}")?,
                Segment::Index(i) => write!(f, "[{i}]")?,
            }
        }
        Ok(())
    }
}

/// Where a finding applies.
///
/// # Examples
///
/// ```
/// use mztab_m::Location;
///
/// let at = Location::text(3, 1).with_cell(4);
/// assert_eq!(at.to_string(), "line 3, column 1, cell 4");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Location {
    /// A position in TSV text.
    Text {
        /// 1-based line number.
        line: u32,
        /// 1-based column, counted in Unicode scalar values.
        column: u32,
        /// 1-based cell number on the line (the prefix is cell 1), if the
        /// finding concerns one cell.
        cell: Option<u32>,
    },
    /// A position in a JSON document, as a JSON pointer.
    Json(Path),
    /// A position in a model built in code, which has no source text.
    Model(Path),
    /// The document as a whole, e.g. a missing mandatory field.
    Document,
}

impl Location {
    /// A text position with no cell.
    #[must_use]
    pub const fn text(line: u32, column: u32) -> Self {
        Self::Text {
            line,
            column,
            cell: None,
        }
    }

    /// Attaches a cell number to a text position; other locations are
    /// returned unchanged.
    #[must_use]
    pub fn with_cell(self, cell: u32) -> Self {
        match self {
            Self::Text { line, column, .. } => Self::Text {
                line,
                column,
                cell: Some(cell),
            },
            other => other,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { line, column, cell } => {
                write!(f, "line {line}, column {column}")?;
                cell.map_or(Ok(()), |cell| write!(f, ", cell {cell}"))
            }
            Self::Json(path) => path.fmt_pointer(f),
            Self::Model(path) => path.fmt_model(f),
            Self::Document => f.write_str("document"),
        }
    }
}

/// Structured detail for a finding. Rendered only on demand.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Detail {
    /// No detail beyond the code.
    None,
    /// A parameter grammar error.
    Param(ParamError),
    /// A number grammar error.
    Number(NumberError),
    /// A text cell error.
    Text(TextError),
    /// A version grammar error.
    Version(VersionError),
    /// What the cell should have been.
    Expected(&'static str),
    /// How many cells the header has and how many the row has.
    CellCount {
        /// Cells in the header, prefix included.
        expected: u32,
        /// Cells in the row, prefix included.
        found: u32,
    },
    /// A line prefix that matches a known one except for letter case.
    PrefixCase(&'static str),
    /// The metadata key concerned (for example, a missing field).
    Key(MtdKey),
    /// The element, column or row id concerned, as written.
    Label(Cow<'static, str>),
    /// The profile the tables form.
    Profile(Profile),
    /// The prohibited combination the tables form.
    Prohibited(ProhibitedProfile),
    /// A CV term: its accession, the name its vocabulary gives it, and the
    /// CV-mapping rule it was checked against.
    Term {
        /// The accession.
        accession: Cow<'static, str>,
        /// The vocabulary's name for the term, when it differs.
        name: Option<Cow<'static, str>>,
        /// The CV-mapping rule's id.
        rule: Option<&'static str>,
    },
}

impl fmt::Display for Detail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Param(e) => e.fmt(f),
            Self::Number(e) => e.fmt(f),
            Self::Text(e) => e.fmt(f),
            Self::Version(e) => e.fmt(f),
            Self::Expected(what) => write!(f, "expected {what}"),
            Self::CellCount { expected, found } => {
                write!(f, "expected {expected} cells, found {found}")
            }
            Self::PrefixCase(prefix) => {
                write!(f, "prefixes are case-sensitive; did you mean `{prefix}`?")
            }
            Self::Key(key) => write!(f, "`{key}`"),
            Self::Label(label) => write!(f, "`{label}`"),
            Self::Profile(profile) => write!(f, "profile `{profile}`"),
            Self::Prohibited(tables) => write!(f, "tables `{tables}`"),
            Self::Term {
                accession,
                name,
                rule,
            } => {
                write!(f, "`{accession}`")?;
                if let Some(name) = name {
                    write!(f, " is `{name}`")?;
                }
                rule.map_or(Ok(()), |rule| write!(f, " (rule `{rule}`)"))
            }
        }
    }
}

/// One problem (or note) about a document.
///
/// `Display` renders `severity[id] location: summary: detail`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Finding {
    /// What rule is concerned.
    pub code: Code,
    /// How serious it is; defaults to the code's RFC 2119 level.
    pub severity: Severity,
    /// Where it applies.
    pub location: Location,
    /// Structured detail.
    pub detail: Detail,
}

impl Finding {
    /// A finding at the code's default severity.
    #[must_use]
    pub const fn new(code: Code, location: Location, detail: Detail) -> Self {
        Self {
            code,
            severity: code.requirement().severity(),
            location,
            detail,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = if matches!(self.detail, Detail::None) {
            ""
        } else {
            ": "
        };
        write!(
            f,
            "{}[{}] {}: {}{separator}{}",
            self.severity,
            self.code.id(),
            self.location,
            self.code.summary(),
            self.detail
        )
    }
}

/// Every finding about a document, in the order they were produced.
///
/// # Examples
///
/// ```
/// use mztab_m::{Code, Detail, Finding, Findings, Location, Severity};
///
/// let mut findings = Findings::new();
/// assert_eq!(findings.max_severity(), None);
/// findings.push(Finding::new(Code::LinePrefix, Location::text(1, 1), Detail::None));
/// assert!(findings.has_errors());
/// assert_eq!(findings.len(), 1);
/// assert_eq!(findings.iter().count(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Findings(Vec<Finding>);

impl Findings {
    /// No findings.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a finding.
    pub fn push(&mut self, finding: Finding) {
        self.0.push(finding);
    }

    /// The number of findings.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The findings in order.
    pub fn iter(&self) -> core::slice::Iter<'_, Finding> {
        self.0.iter()
    }

    /// The most serious severity, or `None` if there are no findings.
    #[must_use]
    pub fn max_severity(&self) -> Option<Severity> {
        self.0.iter().map(|f| f.severity).max()
    }

    /// Whether any finding is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.max_severity() == Some(Severity::Error)
    }

    /// The findings as a `Vec`.
    #[must_use]
    pub fn into_vec(self) -> Vec<Finding> {
        self.0
    }
}

impl<'a> IntoIterator for &'a Findings {
    type Item = &'a Finding;
    type IntoIter = core::slice::Iter<'a, Finding>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Extend<Finding> for Findings {
    fn extend<I: IntoIterator<Item = Finding>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{
        Code, Detail, Finding, Findings, Layer, Location, Path, Requirement, Severity, SpecDoc,
    };
    use crate::param::{ParamError, ParamErrorKind};
    use crate::value::{NumberError, TextError};
    use crate::vocab::VersionError;
    use alloc::collections::BTreeSet;
    use alloc::string::ToString as _;
    use alloc::vec::Vec;

    #[test]
    fn codes_are_unique_sorted_and_prefixed() {
        let ids: Vec<_> = Code::ALL.iter().map(|c| c.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "Code::ALL is in id order");
        assert_eq!(
            ids.iter().collect::<BTreeSet<_>>().len(),
            ids.len(),
            "ids are unique"
        );
        for code in Code::ALL {
            let prefix = [
                (Layer::Syntax, "MZT-S"),
                (Layer::Structure, "MZT-T"),
                (Layer::Semantic, "MZT-X"),
            ]
            .into_iter()
            .find_map(|(layer, prefix)| (layer == code.layer()).then_some(prefix));
            let prefix = prefix.unwrap_or_default();
            assert!(code.id().starts_with(prefix), "{code} matches its layer");
            assert_eq!(code.id().len(), 8, "{code} has three digits");
            assert!(!code.summary().is_empty(), "{code} has a summary");
            assert!(!code.spec_ref().anchor.is_empty(), "{code} has an anchor");
            assert_eq!(code.to_string(), code.id(), "{code} displays as its id");
        }
    }

    #[test]
    fn layers_render() {
        let names: Vec<alloc::string::String> = [Layer::Syntax, Layer::Structure, Layer::Semantic]
            .iter()
            .map(alloc::string::ToString::to_string)
            .collect();
        assert_eq!(names, ["syntax", "structure", "semantic"], "names");
    }

    #[test]
    fn code_metadata() {
        assert_eq!(Code::ProfileValue.layer(), Layer::Structure, "layer");
        assert_eq!(Code::ProfileValue.jmztab(), &[1038], "jmztab");
        assert_eq!(
            Code::NumberForm.jmztab(),
            &[] as &[u16],
            "no jmztab equivalent"
        );
        assert_eq!(
            Code::VersionSyntax.spec_ref().doc,
            SpecDoc::FieldReference,
            "doc"
        );
        assert_eq!(
            Code::VersionSyntax.spec_ref().to_string(),
            "https://github.com/HUPO-PSI/mzTab-M/blob/e66c19e1d5c2e61cd721cb3bd6134389184cf778/specification_documents/mzTab_m_2_1_schema.adoc#mztab-version",
            "url"
        );
        assert_eq!(
            SpecDoc::Format.path(),
            "specification_documents/mzTab_format_specification_2_1-M.adoc",
            "path"
        );
    }

    #[test]
    fn severities() {
        assert_eq!(Requirement::Must.severity(), Severity::Error, "must");
        assert_eq!(Requirement::May.severity(), Severity::Info, "may");
        assert_eq!(Requirement::Should.severity(), Severity::Warning, "should");
        assert_eq!(Severity::Info.to_string(), "info", "info");
        assert_eq!(Severity::Warning.to_string(), "warning", "warning");
        assert_eq!(Severity::Error.to_string(), "error", "error");
        assert!(Severity::Warning > Severity::Info, "order");
    }

    #[test]
    fn locations_render() {
        assert_eq!(Location::text(2, 9).to_string(), "line 2, column 9", "text");
        assert_eq!(
            Location::text(2, 9).with_cell(4).to_string(),
            "line 2, column 9, cell 4",
            "cell"
        );
        assert_eq!(
            Location::Document.with_cell(3),
            Location::Document,
            "cell ignored"
        );
        assert_eq!(Location::Document.to_string(), "document", "document");
        let path = Path::new().key("a/b~c").index(0).key("d");
        assert_eq!(path.segments().len(), 3, "segments");
        assert_eq!(
            Location::Json(path.clone()).to_string(),
            "/a~1b~0c/0/d",
            "pointer escapes"
        );
        assert_eq!(Location::Model(path).to_string(), "a/b~c[0].d", "model");
        assert_eq!(Location::Json(Path::new()).to_string(), "", "root pointer");
        assert_eq!(
            Location::Model(Path::new().index(2)).to_string(),
            "[2]",
            "leading index"
        );
    }

    #[test]
    fn findings_render_with_detail() {
        let at = Location::text(1, 2);
        let render = |code, detail| Finding::new(code, at.clone(), detail).to_string();
        assert_eq!(
            render(
                Code::ParamSyntax,
                Detail::Param(ParamError {
                    kind: ParamErrorKind::MissingName,
                    offset: 3
                })
            ),
            "error[MZT-S002] line 1, column 2: invalid parameter: the name field is empty (at byte 3)",
            "param"
        );
        assert_eq!(
            render(
                Code::NumberSyntax,
                Detail::Number(NumberError { offset: 0 })
            ),
            "error[MZT-S003] line 1, column 2: invalid number: not a number (unexpected input at byte 0)",
            "number"
        );
        assert_eq!(
            render(Code::TextCell, Detail::Text(TextError::Empty)),
            "error[MZT-S005] line 1, column 2: empty or malformed cell: text is empty",
            "text"
        );
        assert_eq!(
            render(
                Code::VersionSyntax,
                Detail::Version(VersionError { offset: 0 })
            ),
            "error[MZT-S006] line 1, column 2: invalid mzTab-version: not an mzTab-M version `major.minor.patch-M` (unexpected input at byte 0)",
            "version"
        );
        assert_eq!(Detail::None.to_string(), "", "none");
        assert_eq!(
            render(Code::LinePrefix, Detail::None),
            "error[MZT-S001] line 1, column 2: unknown line prefix",
            "no detail"
        );
    }

    #[test]
    fn findings_collect() {
        let mut findings = Findings::new();
        assert_eq!(findings, Findings::default(), "new is default");
        assert!(findings.is_empty(), "empty");
        assert_eq!(findings.len(), 0, "len 0");
        assert!(!findings.has_errors(), "no errors");
        let mut warn = Finding::new(Code::NumberForm, Location::Document, Detail::None);
        warn.severity = Severity::Warning;
        findings.push(warn.clone());
        assert_eq!(findings.len(), 1, "len");
        assert_eq!(findings.iter().next(), Some(&warn), "iter");
        assert_eq!(findings.max_severity(), Some(Severity::Warning), "max");
        assert!(!findings.has_errors(), "warnings only");
        assert!(!findings.is_empty(), "not empty");
        findings.extend([warn.clone()]);
        assert_eq!((&findings).into_iter().count(), 2, "into_iter");
        assert_eq!(findings.len(), 2, "len 2");
        findings.push(Finding::new(
            Code::LinePrefix,
            Location::Document,
            Detail::None,
        ));
        assert!(findings.has_errors(), "error present");
        let all = findings.into_vec();
        assert_eq!(all.get(..2), Some(&[warn.clone(), warn][..]), "into_vec");
        assert_eq!(all.len(), 3, "into_vec len");
    }
}
