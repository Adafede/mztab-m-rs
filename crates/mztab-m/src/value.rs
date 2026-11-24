//! Cell-level values: [`Cell`], [`Text`], [`Raw`] and [`Decimal`].
//!
//! Every typed field the reader fills from a file is a [`Cell`]. A cell holds
//! a well-typed value, the `null` literal, or the raw text that failed to type.
//! The last case is what lets the reader collect *all* findings and still hand
//! the validator a typed model: nothing is lost, and nothing is guessed.
//!
//! ```
//! use mztab_m::{Cell, Decimal};
//!
//! let mass: Cell<'_, Decimal<'_>> = Cell::parse("1.50", Decimal::parse);
//! assert_eq!(mass.value().map(Decimal::as_str), Some("1.50"));
//!
//! let missing: Cell<'_, Decimal<'_>> = Cell::parse("null", Decimal::parse);
//! assert!(missing.is_null());
//!
//! let broken: Cell<'_, Decimal<'_>> = Cell::parse("1,5", Decimal::parse);
//! assert_eq!(broken.invalid().map(|raw| raw.as_str()), Some("1,5"));
//! ```

mod cell;
mod decimal;
mod text;

pub use cell::Cell;
pub use decimal::{Decimal, DecimalKind, NumberError};
pub(crate) use text::find_control;
pub use text::{Raw, Text, TextError};

/// The literal that encodes a missing value in a table cell (SPEC §5.7).
pub const NULL: &str = "null";
