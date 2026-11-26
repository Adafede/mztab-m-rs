//! OBO parsing and strict ancestor queries never panic or loop forever.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::cv::TermLookup;
use mztab_m_obo::Ontology;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let onto = Ontology::parse(text);
    for line in text.lines().take(16) {
        let id = line.trim();
        let _ = onto.term(id);
        assert!(!onto.is_descendant(id, id), "irreflexive is_descendant");
    }
});
