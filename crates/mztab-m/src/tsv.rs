//! Shared TSV tokenization, cell typing, metadata value typing, and table slots.
//!
//! This module depends only on Tiers 1–2 (`id`, `vocab`, `value`, `param`, `meta`,
//! `model`, `diag`) and has zero dependency on `write`.

mod mtd;
mod splitter;
mod table;
mod typing;

pub(crate) use mtd::mtd_value;
pub(crate) use splitter::Splitter;
#[cfg(feature = "json")]
pub(crate) use splitter::split_list;
pub(crate) use table::{Slot, TableRow, header, row, slots};
pub(crate) use typing::{Cx, RawCell};
