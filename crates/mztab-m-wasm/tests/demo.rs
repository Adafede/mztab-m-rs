//! The demo's functions, natively and on wasm32.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m_wasm::{convert, validate};

const BASE: &str = include_str!("../../mztab-m/tests/golden/full.canonical.mztab");
const BASE_JSON: &str = include_str!("../../mztab-m/tests/golden/full.json");

#[test]
fn validates_tsv_and_json() {
    assert_eq!(
        validate(BASE, Vec::new(), None),
        "[]",
        "the base document is valid"
    );
    assert_eq!(
        validate(BASE_JSON, Vec::new(), None),
        "[]",
        "and so is its JSON"
    );
    assert!(
        validate("not-json", Vec::new(), Some("bad.json".to_owned())).contains("MZT-S014"),
        "filename `.json` sniffs JSON even when invalid"
    );
    let bad = BASE.replace("\t2.1.0-M\n", "\t2.1\n");
    let findings = validate(&bad, Vec::new(), None);
    assert!(
        findings.starts_with("[{\"code\":\"MZT-S006\""),
        "{findings}"
    );
    assert!(!findings.contains("MZT-T028"), "reported once");
    let obo = "[Term]\nid: MS:1\nname: x\n";
    assert!(
        validate(BASE, vec![obo.to_owned()], None).contains("MZT-X004"),
        "MS terms unknown to this vocabulary"
    );
}

#[test]
fn validates_multiple_vocabularies_separately() {
    let ms = "ontology: ms\n\n[Term]\nid: MS:1\nname: x\n";
    let uo = "ontology: uo\n\n[Term]\nid: UO:1\nname: y\n";
    let doc = "MTD\tmzTab-version\t2.1.0-M\nMTD\tcustom[1]\t[UO, UO:999, bad, ]\n";
    let findings = validate(doc, vec![ms.to_owned(), uo.to_owned()], None);
    assert!(
        findings.contains("MZT-X004") && findings.contains("UO:999"),
        "second vocabulary must keep its own ontology header: {findings}"
    );
}

#[test]
fn converts_both_ways() {
    assert_eq!(
        convert(BASE, "json", None, false).as_deref(),
        Ok(BASE_JSON.trim_end()),
        "to JSON"
    );
    assert_eq!(
        convert(BASE_JSON, "tsv", None, false).as_deref(),
        Ok(BASE),
        "to TSV"
    );
    let unknown = BASE.replace("MTD\ttitle\t", "MTD\tx[0]\ty\nMTD\ttitle\t");
    assert!(
        convert(&unknown, "json", None, false).is_err_and(|e| e.contains("no JSON form")),
        "unwritable without lossy"
    );
    assert_eq!(
        convert(&unknown, "json", None, true).as_deref(),
        Ok(BASE_JSON.trim_end()),
        "lossy conversion drops unknown keys"
    );
    assert!(
        convert(BASE, "xml", None, false).is_err_and(|e| e.contains("unknown format")),
        "unknown target format"
    );
}
