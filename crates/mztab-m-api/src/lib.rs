#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod format;
mod ops;
mod report;
mod settings;

pub use mztab_m::{Findings, Layer, Parsed, SPEC_COMMIT, SPEC_VERSION, Severity, Stripped};

pub use format::{Error, Format};
pub use ops::{check, convert, read, validate, validate_json, vocabulary_size, write};
pub use report::{CodeRecord, Fact, Record, codes, facts, facts_json, findings_json};
pub use settings::Settings;
