//! JSON reading/writing never panics, preserves models across TSV/JSON, and
//! emits valid JSON for findings.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::Code;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let parsed = mztab_m::json::parse(text);
    if let Ok(written) = mztab_m::json::to_string(&parsed.mztab) {
        let back = mztab_m::json::parse(&written);
        assert!(
            !back
                .findings
                .iter()
                .any(|f| matches!(f.code, Code::JsonSyntax | Code::JsonText)),
            "serialized JSON had syntax/text findings: {:?}",
            back.findings
        );
        assert!(back.mztab == parsed.mztab, "read(write(m)) != m");
    }
    let mut findings_json = String::new();
    mztab_m::json::write_findings(&parsed.findings, &mut findings_json).expect("writing to String");
    assert!(findings_json.starts_with('[') && findings_json.ends_with(']'));

    let mut model = mztab_m::parse(text).mztab;
    model.layout = mztab_m::SourceLayout::default();
    if let Ok(written) = mztab_m::json::to_string(&model) {
        let tsv = mztab_m::to_string(&model).expect("writable");
        let mut expected = mztab_m::parse(&tsv).mztab;
        expected.layout = mztab_m::SourceLayout::default();
        assert!(
            mztab_m::json::parse(&written).mztab == expected,
            "TSV -> JSON -> model"
        );
    }
});
