#![doc = include_str!("../README.md")]
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

/// The pinned specification commit, as a literal for generated docs (see
/// [`SPEC_COMMIT`]).
macro_rules! spec_commit {
    () => {
        "e66c19e1d5c2e61cd721cb3bd6134389184cf778"
    };
}
// Macro export requires `use` to expose `spec_commit!` to child modules.
pub(crate) use spec_commit;

pub mod cv;
pub(crate) mod diag;
pub(crate) mod id;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
pub(crate) mod meta;
pub(crate) mod model;
pub(crate) mod owned;
pub(crate) mod param;
pub(crate) mod read;
pub(crate) mod tsv;
pub mod validate;
pub(crate) mod value;
pub(crate) mod vocab;
pub(crate) mod write;

pub use diag::code::Code;
pub use diag::finding::{Detail, Finding, Findings};
pub use diag::location::{Location, Path, Segment};
pub use diag::severity::{Layer, Requirement, Severity, SpecDoc, SpecRef};
pub use id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
pub use meta::key::{MtdKey, ValueKind};
pub use meta::subkey::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InstrumentKey, MsRunKey, ProtocolKey, SampleKey,
    SoftwareKey, StudyVariableGroupKey, StudyVariableKey,
};
pub use model::colunit::{ColUnit, ColUnitError, SpectraRef, SpectraRefError};
pub use model::document::{MzTab, Stripped};
pub use model::layout::{
    ExtraLine, LineContent, LineEnding, LineInfo, SourceLayout, SourceLine, Verbatim,
};
pub use model::meta_value::{InsertError, MtdEntry, MtdValue, ValueRef};
pub use model::metadata::{
    Assay, Contact, Cv, Database, Instrument, Metadata, MsRun, Protocol, Sample, Software,
    StudyVariable, StudyVariableGroup,
};
pub use model::rows::{SmeRow, SmfRow, SmlRow};
pub use model::table::{
    AbundanceColumn, Column, Header, OptColumn, OptTarget, SmeColumn, SmeId, SmfColumn, SmfId,
    SmlColumn, SmlId, Table,
};
pub use model::{Field, List};
pub use owned::IntoOwned;
pub use param::Param;
pub use param::parse::{ParamError, ParamErrorKind};
pub use param::text::{Accession, ParamText, ParamTextError};
pub use read::builder::{parse, parse_bytes};
pub use read::event::{Event, Parsed};
pub use read::stream::StreamReader;
pub use value::NULL;
pub use value::cell::Cell;
pub use value::decimal::{Decimal, DecimalKind, NumberError};
pub use value::text::{Raw, Text, TextError};
pub use vocab::VocabError;
pub use vocab::field::{
    AmbiguityCode, Datatype, PublicationItem, PublicationItemError, PublicationKind,
};
pub use vocab::profile::{Profile, ProhibitedProfile};
pub use vocab::section::Section;
pub use vocab::version::{RuleSet, Version, VersionError};
pub use write::error::WriteError;
pub use write::{to_canonical_string, to_string, write};

/// The mzTab-M specification commit this crate implements.
///
/// The rules and spec references in this crate are checked against the
/// specification text at this commit of
/// <https://github.com/HUPO-PSI/mzTab-M>.
///
/// # Examples
///
/// ```
/// assert_eq!(mztab_m::SPEC_COMMIT.len(), 40, "full git SHA expected");
/// ```
pub const SPEC_COMMIT: &str = spec_commit!();

/// The newest mzTab-M version this crate implements (draft, see `DECISIONS.md`).
///
/// # Examples
///
/// ```
/// assert_eq!(mztab_m::SPEC_VERSION, "2.1.0-M", "target version");
/// ```
pub const SPEC_VERSION: &str = "2.1.0-M";

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn spec_commit_is_hex() {
        assert!(
            super::SPEC_COMMIT.chars().all(|c| c.is_ascii_hexdigit()),
            "SPEC_COMMIT must be a hex SHA, got {}",
            super::SPEC_COMMIT
        );
    }
}
