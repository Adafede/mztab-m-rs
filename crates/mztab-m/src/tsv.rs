//! Shared TSV tokenization, cell typing, metadata value typing, and table slots.
//!
//! This module depends only on Tiers 1–2 (`id`, `vocab`, `value`, `param`, `meta`,
//! `model`, `diag`) and has zero dependency on `write`.

pub(crate) mod mtd;
pub(crate) mod splitter;
pub(crate) mod table;
pub(crate) mod typing;
