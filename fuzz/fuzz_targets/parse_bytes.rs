//! `parse_bytes` never panics, and every finding it reports is located on a
//! line of the input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::Location;

fuzz_target!(|data: &[u8]| {
    let parsed = mztab_m::parse_bytes(data);
    let lines = data.split(|&b| b == b'\n').count();
    for finding in &parsed.findings {
        let Location::Text { line, .. } = finding.location else {
            panic!("reader finding without a text location: {finding}");
        };
        assert!(line >= 1 && line as usize <= lines, "finding outside the input: {finding}");
        let _ = finding.to_string();
    }
});
