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

pub(crate) mod colunit;
pub(crate) mod document;
pub(crate) mod layout;
mod meta_entries;
mod meta_insert;
pub(crate) mod meta_value;
pub(crate) mod metadata;
pub(crate) mod rows;
pub(crate) mod table;

use alloc::vec::Vec;

use crate::value::cell::Cell;

/// A metadata field: absent (`None`) or present as a [`Cell`].
pub type Field<'a, T> = Option<Cell<'a, T>>;

/// A `|`-separated list. Each item is a cell, so `null` items and invalid
/// items are kept.
pub type List<'a, T> = Vec<Cell<'a, T>>;
