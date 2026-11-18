//! JSON reading never panics; a model written as JSON reads back the same.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let parsed = mztab_m::json::parse(text);
    if let Ok(written) = mztab_m::json::to_string(&parsed.mztab) {
        let back = mztab_m::json::parse(&written);
        assert!(back.mztab == parsed.mztab, "read(write(m)) != m");
    }
    // TSV input too: JSON agrees with the canonical TSV.
    let mut model = mztab_m::parse(text).mztab;
    model.layout = mztab_m::SourceLayout::default();
    if let Ok(written) = mztab_m::json::to_string(&model) {
        let tsv = mztab_m::to_string(&model).expect("writable");
        let mut expected = mztab_m::parse(&tsv).mztab;
        expected.layout = mztab_m::SourceLayout::default();
        assert!(mztab_m::json::parse(&written).mztab == expected, "TSV -> JSON -> model");
    }
});
