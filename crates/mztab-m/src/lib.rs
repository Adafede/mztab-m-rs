#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod diag;
mod id;
mod model;
mod param;
mod value;
mod vocab;

pub use diag::{
    Code, Detail, Finding, Findings, Layer, Location, Path, Requirement, Segment, Severity,
    SpecDoc, SpecRef,
};
pub use id::{
    AssayId, ContactId, CvId, DatabaseId, IdConfidenceMeasureId, IndexMap, InstrumentId, Ix,
    MsRunId, ProtocolId, PublicationId, SampleId, SoftwareId, StudyVariableGroupId,
    StudyVariableId,
};
pub use model::{
    AbundanceColumn, Assay, ColUnit, ColUnitError, Column, Contact, Cv, Database, Field, Header,
    Instrument, List, Metadata, MsRun, MzTab, OptColumn, OptTarget, Protocol, Sample, SmeColumn,
    SmeId, SmeRow, SmfColumn, SmfId, SmfRow, SmlColumn, SmlId, SmlRow, Software, SpectraRef,
    SpectraRefError, StudyVariable, StudyVariableGroup, Table,
};
pub use param::{Accession, Param, ParamError, ParamErrorKind, ParamText, ParamTextError};
pub use value::{Cell, Decimal, DecimalKind, NULL, NumberError, Raw, Text, TextError};
pub use vocab::{
    AmbiguityCode, Profile, ProhibitedProfile, PublicationItem, PublicationItemError,
    PublicationKind, RuleSet, Section, Version, VersionError,
};

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
pub const SPEC_COMMIT: &str = "e66c19e1d5c2e61cd721cb3bd6134389184cf778";

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
