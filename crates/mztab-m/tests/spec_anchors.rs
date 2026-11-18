//! Every finding code links to an anchor that exists in the pinned spec.
//!
//! The specification text is embedded at compile time, so this test runs
//! unchanged on wasm32 (which has no file system).

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::{Code, SpecDoc};

const FORMAT: &str = include_str!(
    "../../../third_party/mzTab-M/specification_documents/mzTab_format_specification_2_1-M.adoc"
);
const FIELD_REFERENCE: &str =
    include_str!("../../../third_party/mzTab-M/specification_documents/mzTab_m_2_1_schema.adoc");

/// Whether `doc` declares `[[anchor]]` or `[[anchor, title]]`.
fn has_anchor(doc: &str, anchor: &str) -> bool {
    doc.match_indices("[[")
        .filter_map(|(at, _)| doc.get(at + 2..))
        .filter_map(|rest| rest.split_once("]]").map(|(id, _)| id))
        .any(|id| id.split(',').next() == Some(anchor))
}

#[test]
fn every_code_anchor_exists_in_the_pinned_spec() {
    for code in Code::ALL {
        let spec = code.spec_ref();
        let doc = match spec.doc {
            SpecDoc::Format => FORMAT,
            SpecDoc::FieldReference => FIELD_REFERENCE,
        };
        assert!(
            has_anchor(doc, spec.anchor),
            "{code}: anchor `{}` not found in {}",
            spec.anchor,
            spec.doc.path()
        );
    }
}

#[test]
fn anchor_matcher_is_exact() {
    let doc = "[[a-b]]\n[[table-1, CV Terms]]\n[[[biblio]]]";
    assert!(has_anchor(doc, "a-b"), "plain anchor");
    assert!(has_anchor(doc, "table-1"), "anchor with title");
    assert!(!has_anchor(doc, "a"), "no prefix match");
    assert!(!has_anchor(doc, "CV Terms"), "title is not an id");
    assert!(
        !has_anchor("[[unterminated", "unterminated"),
        "needs closing brackets"
    );
}
