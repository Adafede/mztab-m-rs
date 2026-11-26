//! `parse_bytes` never panics, every finding points to a valid line/column,
//! and valid UTF-8 inputs match `mztab_m::parse`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::Location;

fuzz_target!(|data: &[u8]| {
    let parsed = mztab_m::parse_bytes(data);
    let line_bytes = data.strip_suffix(b"\n").unwrap_or(data);
    let lines = if line_bytes.is_empty() {
        1
    } else {
        line_bytes.split(|&b| b == b'\n').count()
    };
    for finding in &parsed.findings {
        let Location::Text { line, column, .. } = finding.location else {
            panic!("reader finding without a text location: {finding}");
        };
        assert!(
            line >= 1 && line as usize <= lines,
            "finding outside the input: {finding}"
        );
        assert!(column >= 1, "column is 1-based: {finding}");
        let _ = finding.to_string();
    }
    if let Ok(text) = core::str::from_utf8(data) {
        assert!(mztab_m::parse(text) == parsed, "parse != parse_bytes");
    }
});
