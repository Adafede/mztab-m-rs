//! Writing a read document reproduces it (invalid UTF-8 decoded lossily),
//! preserves findings on UTF-8 input, and round-trips canonical output.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::{Code, SourceLayout};

fuzz_target!(|data: &[u8]| {
    let parsed = mztab_m::parse_bytes(data);
    let written = mztab_m::to_string(&parsed.mztab).expect("a read model is writable");
    assert_eq!(
        written,
        String::from_utf8_lossy(data),
        "write(read(d)) != d"
    );
    if core::str::from_utf8(data).is_ok() {
        let reparsed = mztab_m::parse(&written);
        assert!(reparsed.mztab == parsed.mztab, "read(write(m)) != m");
        assert!(reparsed.findings == parsed.findings, "findings changed");
    }
    if !parsed.findings.iter().any(|f| f.code == Code::CellCount) {
        let mut canonical = parsed.mztab.clone();
        canonical.layout = SourceLayout::default();
        if let Ok(canon_text) = mztab_m::to_string(&canonical) {
            let mut back = mztab_m::parse(&canon_text).mztab;
            back.layout = SourceLayout::default();
            assert!(back == canonical, "canonical round-trip differed");
        }
    }
});
