//! Metadata keys ([`MtdKey`]), subkeys, and their expected [`ValueKind`].
//!
//! This module sits strictly above Tier 1 primitives (`id`, `vocab`) and has
//! no dependency on `model`.
//!
//! ```
//! use mztab_m::{AssayKey, MtdKey};
//!
//! let key = MtdKey::parse("assay[2]-ms_run_ref").ok_or("unknown key")?;
//! assert!(matches!(key, MtdKey::Assay(id, AssayKey::MsRunRef) if id.get() == 2));
//! assert_eq!(key.to_string(), "assay[2]-ms_run_ref");
//! assert_eq!(MtdKey::parse("assay[0]"), None);
//! # Ok::<(), &str>(())
//! ```

pub(crate) mod key;
pub(crate) mod subkey;
