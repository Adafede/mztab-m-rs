//! The generator of `src/cv/rules.rs`, and its freshness check.
//!
//! The CV-mapping rules are generated from the pinned mapping file
//! (`schema/mzTab_2_1-M_mapping.xml`) with the corrections of DECISIONS
//! D-068. This test regenerates them and fails if the committed file
//! differs; set `BLESS=1` to rewrite it.

#![expect(clippy::expect_used, reason = "test helpers")]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use core::fmt::Write as _;

const MAPPING: &str = include_str!("../../../third_party/mzTab-M/schema/mzTab_2_1-M_mapping.xml");
const COMMITTED: &str = include_str!("../src/cv/rules.rs");

/// The model path of each rule's `cvElementPath`, as a `Target` variant.
const TARGETS: &[(&str, &str)] = &[
    ("/metadata/msRun/@format", "MsRunFormat"),
    ("/metadata/msRun/@idFormat", "MsRunIdFormat"),
    (
        "/metadata/msRun/@fragmentationMethod",
        "MsRunFragmentationMethod",
    ),
    ("/metadata/msRun/@scanPolarity", "MsRunScanPolarity"),
    ("/metadata/msRun/@hashMethod", "MsRunHashMethod"),
    (
        "/metadata/sampleProcessing/@sampleProcessing",
        "SampleProcessing",
    ),
    ("/metadata/instrument/@name", "InstrumentName"),
    ("/metadata/instrument/@source", "InstrumentSource"),
    ("/metadata/instrument/@analyzer", "InstrumentAnalyzer"),
    ("/metadata/instrument/@detector", "InstrumentDetector"),
    ("/metadata/software/@parameter", "Software"),
    ("/metadata/@quantificationMethod", "QuantificationMethod"),
    ("/metadata/sample/@species", "SampleSpecies"),
    ("/metadata/sample/@tissue", "SampleTissue"),
    ("/metadata/sample/@cellType", "SampleCellType"),
    ("/metadata/sample/@disease", "SampleDisease"),
    ("/metadata/database/@param", "Database"),
    (
        "/metadata/studyVariable/@averageFunction",
        "StudyVariableAverageFunction",
    ),
    (
        "/metadata/studyVariable/@variationFunction",
        "StudyVariableVariationFunction",
    ),
    (
        "/metadata/studyVariableGroup/@parameter",
        "StudyVariableGroup",
    ),
    (
        "/metadata/studyVariableGroup/@type",
        "StudyVariableGroupType",
    ),
    (
        "/metadata/studyVariableGroup/@unit",
        "StudyVariableGroupUnit",
    ),
    ("/metadata/@derivatizationAgent", "DerivatizationAgent"),
    (
        "/metadata/@smallMoleculeQuantificationUnit",
        "SmallMoleculeQuantificationUnit",
    ),
    (
        "/metadata/@smallMoleculeFeatureQuantificationUnit",
        "SmallMoleculeFeatureQuantificationUnit",
    ),
    (
        "/metadata/@smallMoleculeIdentificationReliability",
        "SmallMoleculeIdentificationReliability",
    ),
    ("/metadata/@idConfidenceMeasure", "IdConfidenceMeasure"),
    (
        "/smallMoleculeSummary/@bestIdConfidenceMeasure",
        "SmlBestIdConfidenceMeasure",
    ),
    (
        "/smallMoleculeEvidence/@derivatizedForm",
        "SmeDerivatizedForm",
    ),
    ("/smallMoleculeEvidence/@msLevel", "SmeMsLevel"),
    (
        "/smallMoleculeEvidence/@identificationMethod",
        "SmeIdentificationMethod",
    ),
];

/// DECISIONS D-068: terms the field reference uses itself although the
/// mapping file sets `useTerm="false"` (the spec text wins, D-002).
const USE_TERM: &[(&str, &str)] = &[("ms_level_must", "MS:1000511")];

/// DECISIONS D-068: terms the field reference allows that the mapping file
/// lacks. `identification_method` is "the name of software, database or
/// manual curation", and its example is `[MS, MS:1001477, SpectraST,]`.
/// `(rule, accession, name, use_term, allow_children)`.
const EXTRA_TERMS: &[(&str, &str, &str, bool, bool)] = &[
    (
        "identification_method_must",
        "MS:1000531",
        "software",
        false,
        true,
    ),
    // Spec table 1 and the field reference's own example
    // (`ms_run[1]-id_format [MS, MS:1001530, mzML unique identifier, ]`).
    (
        "msrun_id_format_may",
        "MS:1001530",
        "mzML unique identifier",
        true,
        false,
    ),
    // "Defines what type of units are reported in the ... abundance
    // fields", with "peak area" (a quantification datatype) as the example.
    (
        "small_molecule_quantification_unit_must",
        "MS:1001805",
        "quantification datatype",
        false,
        true,
    ),
    (
        "small_molecule_feature_quantification_unit_must",
        "MS:1001805",
        "quantification datatype",
        false,
        true,
    ),
];

/// Removes `<!-- ... -->` comments.
fn strip_comments(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some((before, after)) = rest.split_once("<!--") {
        out.push_str(before);
        rest = after.split_once("-->").map_or("", |(_, tail)| tail);
    }
    out.push_str(rest);
    out
}

/// Unescapes standard XML character entities inside an attribute value.
fn unescape_xml(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The value of attribute `name` in a start tag.
fn attr(tag: &str, name: &str) -> String {
    let tag = tag.trim_end_matches('/');
    let mut found = None;
    let mut rest = tag;
    while !rest.is_empty() {
        rest = rest.trim_start();
        if let Some(after_name) = rest.strip_prefix(name) {
            let after_eq = after_name.trim_start();
            if let Some(after_eq) = after_eq.strip_prefix('=') {
                let after_ws = after_eq.trim_start();
                if let Some(inner) = after_ws.strip_prefix('"') {
                    found = Some(unescape_xml(
                        inner.split_once('"').expect("closed attribute").0,
                    ));
                    break;
                }
                if let Some(inner) = after_ws.strip_prefix('\'') {
                    found = Some(unescape_xml(
                        inner.split_once('\'').expect("closed attribute").0,
                    ));
                    break;
                }
            }
        }
        // Skip past the next quoted attribute value (or whitespace token).
        if let Some((_, after_dq)) = rest.split_once('"')
            && let Some((_, tail)) = after_dq.split_once('"')
        {
            rest = tail;
        } else if let Some((_, after_sq)) = rest.split_once('\'')
            && let Some((_, tail)) = after_sq.split_once('\'')
        {
            rest = tail;
        } else {
            break;
        }
    }
    found.expect("attribute present")
}

/// The value `table` pairs with `key`; every key of the mapping is known.
fn pick<T: Copy>(table: &[(&str, T)], key: &str) -> T {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .expect("a value the generator knows")
}

/// `true`/`yes` (the mapping uses both) or `false`.
fn flag(value: &str) -> bool {
    pick(&[("true", true), ("yes", true), ("false", false)], value)
}

fn generate(xml: &str) -> String {
    let xml = strip_comments(xml);
    let mut out = String::from(
        "//! @generated by `tests/cv_rules.rs` from the pinned\n\
         //! `schema/mzTab_2_1-M_mapping.xml` (DECISIONS D-068). Do not edit:\n\
         //! rerun that test with `BLESS=1`.\n\n\
         use super::{Logic, Rule, RuleTerm, Target};\n\
         use crate::diag::severity::Requirement;\n\n\
         pub(super) const RULES: &[Rule] = &[\n",
    );
    let mut used_overrides = Vec::new();
    let mut used_extra = Vec::new();
    for chunk in xml.split("<CvMappingRule ").skip(1) {
        let (tag, body) = chunk.split_once('>').expect("rule start tag");
        let body = body.split_once("</CvMappingRule>").expect("rule end").0;
        let id = attr(tag, "id");
        let path = attr(tag, "cvElementPath");
        let path = path.as_str();
        let target = pick(TARGETS, path);
        let requirement = pick(
            &[("MUST", "Must"), ("SHOULD", "Should"), ("MAY", "May")],
            &attr(tag, "requirementLevel"),
        );
        let mut logic = pick(
            &[("AND", "And"), ("OR", "Or"), ("XOR", "Xor")],
            &attr(tag, "cvTermsCombinationLogic"),
        );
        let mut rule_terms = Vec::new();
        let mut seen = Vec::new();
        for raw_term in body.split("<CvTerm ").skip(1) {
            let term = raw_term.split_once('>').expect("term tag end").0;
            let accession = attr(term, "termAccession");
            // The same term listed for two vocabularies (sample_processing)
            // is one term here: `cvIdentifierRef` is not checked (D-068).
            if seen.contains(&accession) {
                continue;
            }
            let override_match = USE_TERM.contains(&(id.as_str(), accession.as_str()));
            if override_match && !used_overrides.contains(&(id.clone(), accession.clone())) {
                used_overrides.push((id.clone(), accession.clone()));
            }
            let use_term = flag(&attr(term, "useTerm")) || override_match;
            rule_terms.push((
                accession.clone(),
                attr(term, "termName"),
                use_term,
                flag(&attr(term, "allowChildren")),
                flag(&attr(term, "isRepeatable")),
            ));
            seen.push(accession);
        }
        let mut added_extra = false;
        for &(rule, accession, name, use_term, children) in
            EXTRA_TERMS.iter().filter(|(rule, ..)| *rule == id)
        {
            if !used_extra.contains(&(rule, accession)) {
                used_extra.push((rule, accession));
            }
            if !seen.iter().any(|s| s == accession) {
                rule_terms.push((
                    accession.to_owned(),
                    name.to_owned(),
                    use_term,
                    children,
                    false,
                ));
                seen.push(accession.to_owned());
                added_extra = true;
            }
        }
        if added_extra && logic == "And" {
            logic = "Or";
        }
        assert!(
            logic != "And" || rule_terms.len() == 1,
            "{id}: an AND rule with several terms is not supported"
        );
        writeln!(
            out,
            "    Rule {{\n        id: \"{id}\",\n        target: Target::{target},\n        \
             requirement: Requirement::{requirement},\n        logic: Logic::{logic},\n        \
             terms: &["
        )
        .expect("writing to a String");
        for (accession, name, use_term, allow_children, repeatable) in rule_terms {
            writeln!(
                out,
                "            RuleTerm {{ accession: {accession:?}, name: {name:?}, use_term: {use_term}, \
                 allow_children: {allow_children}, repeatable: {repeatable} }},",
            )
            .expect("writing to a String");
        }
        out.push_str("        ],\n    },\n");
    }
    assert_eq!(used_overrides.len(), USE_TERM.len(), "all USE_TERM applied");
    assert_eq!(
        used_extra.len(),
        EXTRA_TERMS.len(),
        "all EXTRA_TERMS applied"
    );
    out.push_str("];\n");
    out
}

#[test]
fn rules_are_generated_from_the_pinned_mapping() {
    let generated = generate(MAPPING);
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BLESS").is_some() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/cv/rules.rs");
        std::fs::write(path, &generated).expect("write rules");
        return;
    }
    assert!(
        generated == COMMITTED,
        "src/cv/rules.rs is stale; rerun with BLESS=1"
    );
}

#[test]
fn every_rule_has_a_distinct_target() {
    let rules = mztab_m::cv::rules();
    assert_eq!(rules.len(), TARGETS.len(), "one rule per target");
    for (i, rule) in rules.iter().enumerate() {
        assert!(
            rules.iter().skip(i + 1).all(|r| r.target != rule.target),
            "{} shares its target",
            rule.id
        );
    }
}

#[test]
fn corrections_apply() {
    let ms_level = mztab_m::cv::rules()
        .iter()
        .find(|r| r.id == "ms_level_must")
        .expect("rule");
    assert!(
        ms_level.terms.iter().all(|t| t.use_term),
        "MS:1000511 itself is allowed"
    );
    let id_format = mztab_m::cv::rules()
        .iter()
        .find(|r| r.id == "msrun_id_format_may")
        .expect("rule");
    assert_eq!(
        id_format.logic,
        mztab_m::cv::Logic::Or,
        "multi-term alternative rule must not be Logic::And"
    );
    assert!(
        id_format.terms.iter().any(|t| t.accession == "MS:1001530"),
        "mzML unique identifier added"
    );
    for (rule_id, accession) in [
        ("identification_method_must", "MS:1000531"),
        ("small_molecule_quantification_unit_must", "MS:1001805"),
        (
            "small_molecule_feature_quantification_unit_must",
            "MS:1001805",
        ),
    ] {
        let rule = mztab_m::cv::rules()
            .iter()
            .find(|r| r.id == rule_id)
            .expect("rule");
        assert!(
            rule.terms.iter().any(|t| t.accession == accession),
            "{rule_id} contains {accession}"
        );
    }
    assert!(flag("yes"), "yes");
    assert_eq!(strip_comments("a<!-- b -->c<!-- d"), "ac", "comments");
}
