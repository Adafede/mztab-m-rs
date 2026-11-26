//! Structure and semantic validation never panic on arbitrary TSV/JSON inputs.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::cv::NoTerms;
use mztab_m::validate::{self, Config};
use mztab_m_obo::Ontology;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let config = Config::default();
    let _ = validate::all(text, &config, &NoTerms);

    let (obo_src, doc_src) = data.split_at(data.len() / 2);
    let obo_text = String::from_utf8_lossy(obo_src);
    let obo = Ontology::parse(&obo_text);
    if let Ok(doc) = core::str::from_utf8(doc_src) {
        let _ = validate::all(doc, &config, &obo);
        let jp = mztab_m::json::parse(doc);
        let _ = validate::structure(&jp.mztab, &config);
        let _ = validate::semantic(&jp.mztab, &config, &obo);
    }
});
