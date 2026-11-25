//! Stable finding [`Code`] registry.

use core::fmt;
use core::str::FromStr;

use super::severity::{Layer, Requirement, SpecDoc, SpecRef};
use crate::vocab::VocabError;

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

impl FromStr for Code {
    type Err = VocabError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|c| c.id() == s)
            .ok_or_else(|| VocabError::new("a finding code such as `MZT-T009`"))
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::Code;
    use crate::diag::severity::{Layer, SpecDoc};
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
            assert_eq!(code.id().parse::<Code>(), Ok(*code), "from_str {code}");
        }
        assert!("MZT-Z999".parse::<Code>().is_err());
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
    }
}
