//! The typed document model: [`MzTab`], [`Metadata`] and the three tables.
//!
//! The model follows the 2.1 field reference literally, plus the two fields
//! that 2.0 had and 2.1 removed (`ms_run[n]-usi_identifier` and
//! `study_variable[n]-factors`), so 2.0 files read losslessly.
//!
//! - A metadata field is a [`Field`]: `None` when the key is absent,
//!   otherwise a [`Cell`] (value, `null`, or invalid text).
//! - A field typed "… List" in the field reference holds a [`List`]: a
//!   `|`-separated list whose items are cells, so `null|HMDB:1` is kept.
//! - `element[n]` objects live in an [`IndexMap`](crate::id::IndexMap) keyed by their typed id.
//! - Unknown metadata keys are kept, in source order, in
//!   [`Metadata::unknown`].
//!
//! ```
//! use mztab_m::{Cell, IndexMap, Metadata, MzTab, Text, Version};
//!
//! let mut mztab = MzTab::default();
//! mztab.metadata.version = Some(Cell::Value(Version::V2_1_0));
//! mztab.metadata.id = Some(Cell::Value(Text::new("MTBLS263")?));
//! assert!(mztab.sml.is_none());
//! assert!(mztab.metadata.unknown.is_empty());
//! # Ok::<(), mztab_m::TextError>(())
//! ```
//!
//! # What the types do not enforce
//!
//! These rules are checked by `validate`, not by the type system:
//! cross-references resolve; row ids are unique; profile-dependent fields are
//! present; SML `|`-lists have equal lengths; study-variable values match
//! their group's `datatype`; CV terms exist; metadata order; contiguous
//! indices; `spectra_ref` matches the run's `id_format`; and no cell is
//! [`Cell::Invalid`].

mod colunit;
mod document;
mod layout;
mod meta_entries;
mod meta_insert;
mod meta_value;
mod metadata;
mod rows;
mod table;

use alloc::vec::Vec;

use crate::value::Cell;

pub use colunit::{ColUnit, ColUnitError, SpectraRef, SpectraRefError};
pub use document::{MzTab, Stripped};
pub use layout::{
    ExtraLine, LineContent, LineEnding, LineInfo, SourceLayout, SourceLine, Verbatim,
};
pub use meta_value::{InsertError, MtdEntry, MtdValue, ValueRef};
pub use metadata::{
    Assay, Contact, Cv, Database, Instrument, Metadata, MsRun, Protocol, Sample, Software,
    StudyVariable, StudyVariableGroup,
};
pub use rows::{SmeRow, SmfRow, SmlRow};
pub use table::{
    AbundanceColumn, Column, Header, OptColumn, OptTarget, SmeColumn, SmeId, SmfColumn, SmfId,
    SmlColumn, SmlId, Table,
};

/// A metadata field: absent (`None`) or present as a [`Cell`].
pub type Field<'a, T> = Option<Cell<'a, T>>;

/// A `|`-separated list. Each item is a cell, so `null` items and invalid
/// items are kept.
pub type List<'a, T> = Vec<Cell<'a, T>>;
