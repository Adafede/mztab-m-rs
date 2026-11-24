//! Metadata keys ([`MtdKey`]), subkeys, and their expected [`ValueKind`].
//!
//! This module sits strictly above Tier 1 primitives (`id`, `vocab`) and has
//! no dependency on `model`.
//!
//! ```
//! use mztab_m::{AssayKey, MtdKey};
//!
//! let key: MtdKey = "assay[2]-ms_run_ref".parse()?;
//! assert!(matches!(key, MtdKey::Assay(id, AssayKey::MsRunRef) if id.get() == 2));
//! assert_eq!(key.to_string(), "assay[2]-ms_run_ref");
//! assert!("assay[0]".parse::<MtdKey>().is_err());
//! # Ok::<(), mztab_m::VocabError>(())
//! ```

pub(crate) mod key;
pub(crate) mod subkey;
