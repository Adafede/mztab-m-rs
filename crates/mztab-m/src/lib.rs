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
pub(crate) use spec_commit;

pub mod cv;
mod diag;
mod id;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
mod meta;
mod model;
mod owned;
mod param;
mod read;
mod table;
mod typing;
pub mod validate;
mod value;
mod vocab;
mod write;

pub use diag::{
    Code, Detail, Finding, Findings, Layer, Location, Path, Requirement, Segment, Severity,
    SpecDoc, SpecRef,
};
pub use id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
pub use meta::{
    AssayKey, ContactKey, CvKey, DatabaseKey, InsertError, InstrumentKey, MsRunKey, MtdEntry,
    MtdKey, MtdValue, ProtocolKey, SampleKey, SoftwareKey, StudyVariableGroupKey, StudyVariableKey,
    ValueKind, ValueRef,
};
pub use model::{
    AbundanceColumn, Assay, ColUnit, ColUnitError, Column, Contact, Cv, Database, ExtraLine, Field,
    Header, Instrument, LineContent, LineEnding, LineInfo, List, Metadata, MsRun, MzTab, OptColumn,
    OptTarget, Protocol, Sample, SmeColumn, SmeId, SmeRow, SmfColumn, SmfId, SmfRow, SmlColumn,
    SmlId, SmlRow, Software, SourceLayout, SourceLine, SpectraRef, SpectraRefError, Stripped,
    StudyVariable, StudyVariableGroup, Table, Verbatim,
};
pub use owned::IntoOwned;
pub use param::{Accession, Param, ParamError, ParamErrorKind, ParamText, ParamTextError};
pub use read::{Event, Parsed, StreamReader, parse, parse_bytes};
pub use value::{Cell, Decimal, DecimalKind, NULL, NumberError, Raw, Text, TextError};
pub use vocab::{
    AmbiguityCode, Datatype, Profile, ProhibitedProfile, PublicationItem, PublicationItemError,
    PublicationKind, RuleSet, Section, Version, VersionError,
};
pub use write::{WriteError, to_string, write};

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
