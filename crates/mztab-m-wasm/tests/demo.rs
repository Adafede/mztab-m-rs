//! The demo's functions, natively and on wasm32.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m_wasm::{convert, validate};

const BASE: &str = include_str!("../../mztab-m/tests/golden/full.canonical.mztab");
const BASE_JSON: &str = include_str!("../../mztab-m/tests/golden/full.json");

#[test]
fn validates_tsv_and_json() {
    assert_eq!(validate(BASE, ""), "[]", "the base document is valid");
    assert_eq!(validate(BASE_JSON, ""), "[]", "and so is its JSON");
    let bad = BASE.replace("\t2.1.0-M\n", "\t2.1\n");
    let findings = validate(&bad, "");
    assert!(
        findings.starts_with("[{\"code\":\"MZT-S006\""),
        "{findings}"
    );
    assert!(!findings.contains("MZT-T028"), "reported once");
    let obo = "[Term]\nid: MS:1\nname: x\n";
    assert!(
        validate(BASE, obo).contains("MZT-X004"),
        "MS terms unknown to this vocabulary"
    );
}

#[test]
fn converts_both_ways() {
    assert_eq!(
        convert(BASE, "json").as_deref(),
        Ok(BASE_JSON.trim_end()),
        "to JSON"
    );
    assert_eq!(convert(BASE_JSON, "tsv").as_deref(), Ok(BASE), "to TSV");
    assert!(convert(BASE, "xml").is_err(), "unknown format");
    let unknown = BASE.replace("MTD\ttitle\t", "MTD\tx[0]\ty\nMTD\ttitle\t");
    assert!(
        convert(&unknown, "json").is_err_and(|e| e.contains("no JSON form")),
        "unwritable"
    );
}
