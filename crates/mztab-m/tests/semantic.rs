//! The semantic layer with a vocabulary: every rule field is checked, and
//! lookups combine.

#![expect(clippy::expect_used, reason = "test helpers")]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[path = "support/terms.rs"]
mod terms;

use mztab_m::cv::{NoTerms, Term, TermLookup};
use mztab_m::validate::{self, Config};
use mztab_m::{Code, Location, parse};
use terms::Mini;

const BASE: &str = include_str!("golden/full.canonical.mztab");

/// The base document with the value of `key` replaced.
fn with_value(key: &str, value: &str) -> String {
    let prefix = format!("MTD\t{key}\t");
    let mut found = false;
    let out: String = BASE
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) && !found {
                found = true;
                format!("{prefix}{value}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    assert!(found, "{key} is in the base document");
    out
}

fn line_of(text: &str, start: &str) -> u32 {
    let at = text
        .lines()
        .position(|l| l.starts_with(start))
        .expect("line present");
    u32::try_from(at + 1).expect("small")
}

/// Every metadata field with a CV-mapping rule: a covered term outside
/// the rule is reported, on that field's line.
#[test]
fn every_rule_field_is_checked() {
    let polarity = "[MS, MS:1000130, positive scan, ]";
    let median = "[MS, MS:1002883, median, ]";
    let fields = [
        ("ms_run[1]-format", polarity),
        ("ms_run[1]-id_format", polarity),
        ("ms_run[1]-fragmentation_method[1]", polarity),
        ("ms_run[1]-scan_polarity[1]", median),
        ("ms_run[1]-hash_method", polarity),
        ("sample_processing[1]", polarity),
        ("instrument[1]-name", polarity),
        ("instrument[1]-source", polarity),
        ("instrument[1]-analyzer[1]", polarity),
        ("instrument[1]-detector", polarity),
        ("software[1]", polarity),
        ("quantification_method", polarity),
        ("sample[1]-species[1]", polarity),
        ("sample[1]-tissue[1]", polarity),
        ("sample[1]-cell_type[1]", polarity),
        ("sample[1]-disease[1]", polarity),
        ("database[1]", polarity),
        ("study_variable[1]-average_function", polarity),
        ("study_variable[1]-variation_function", polarity),
        ("study_variable_group[1]", polarity),
        ("study_variable_group[1]-type", polarity),
        ("study_variable_group[1]-unit", polarity),
        ("derivatization_agent[1]", polarity),
        ("small_molecule-quantification_unit", polarity),
        ("small_molecule_feature-quantification_unit", polarity),
        ("small_molecule-identification_reliability", polarity),
        ("id_confidence_measure[1]", polarity),
    ];
    for (key, value) in fields {
        let text = with_value(key, value);
        let line = line_of(&text, &format!("MTD\t{key}\t"));
        let model = parse(&text).mztab;
        let findings = validate::semantic(&model, &Config::default(), &Mini);
        let rule = findings
            .iter()
            .find(|f| f.code == Code::CvRule)
            .expect("a rule finding for every field");
        assert!(
            matches!(rule.location, Location::Text { line: l, .. } if l == line),
            "{key}: at line {line}, found {rule}"
        );
    }
}

/// Replaces `from` with `to` on the first line starting with `prefix`.
fn replace_on_line(prefix: &str, from: &str, to: &str) -> String {
    let mut replaced = false;
    let out: String = BASE
        .lines()
        .map(|line| {
            if line.starts_with(prefix) && !replaced {
                assert!(line.contains(from), "{from} is in line {prefix}");
                replaced = true;
                format!("{}\n", line.replacen(from, to, 1))
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    assert!(replaced, "{prefix} is in the base document");
    out
}

/// Table cells with semantic rules point at their exact row line and cell.
#[test]
fn table_semantic_findings_point_at_their_cell() {
    let cases = [
        (
            "SML\t1\t",
            "[MS, MS:1002890, fragmentation score, ]",
            "[MS, MS:1000130, positive scan, ]",
            Code::CvRule,
            13,
        ),
        (
            "SME\t2\t",
            "[CHEBI, CHEBI:51088, trimethylsilyl group, 3]",
            "[MS, MS:1000130, positive scan, ]",
            Code::CvRule,
            10,
        ),
        (
            "SME\t1\t",
            "[MS, MS:1001477, SpectraST, ]",
            "[MS, MS:1000130, positive scan, ]",
            Code::CvRule,
            16,
        ),
        (
            "SME\t1\t",
            "[MS, MS:1000511, ms level, 2]",
            "[MS, MS:1000130, positive scan, ]",
            Code::CvRule,
            17,
        ),
        (
            "SMF\t1\t",
            "[, , monoisotopic, ]",
            "[MS, MS:9999999, nothing, ]",
            Code::UnknownTerm,
            6,
        ),
        (
            "SME\t1\t",
            "[MS, MS:1000511, ms level, 2]",
            "[MS, MS:1000511, ms level, two]",
            Code::TermValueType,
            17,
        ),
    ];
    for (prefix, from, to, code, cell) in cases {
        let text = replace_on_line(prefix, from, to);
        let line = line_of(&text, prefix);
        let model = parse(&text).mztab;
        let findings = validate::semantic(&model, &Config::default(), &Mini);
        let finding = findings
            .iter()
            .find(|f| f.code == code)
            .expect("expected semantic finding");
        assert!(
            matches!(
                finding.location,
                Location::Text {
                    line: l,
                    cell: Some(c),
                    ..
                } if l == line && c == cell
            ),
            "{prefix}: expected line {line} cell {cell}, found {finding}"
        );
    }
}

/// Repeated `colunit` lines are told apart.
#[test]
fn findings_on_repeated_keys_point_at_their_line() {
    let unit = "MTD\tcolunit-small_molecule\tbest_id_confidence_value=";
    let text = BASE.replace(
        &format!("{unit}[UO, UO:0000186, dimensionless unit, ]"),
        &format!("{unit}[MS, MS:9999999, nothing, ]"),
    );
    let model = parse(&text).mztab;
    let findings = validate::semantic(&model, &Config::default(), &Mini);
    let unknown = findings
        .iter()
        .find(|f| f.code == Code::UnknownTerm)
        .expect("unknown term");
    assert!(
        matches!(unknown.location, Location::Text { line, .. } if line == line_of(&text, unit)),
        "second colunit line: {unknown}"
    );
}

/// A CV named only by a column header is declared and used.
#[test]
fn column_headers_use_declared_cvs() {
    let text = BASE
        .replace(
            "MTD\tcv[1]-label\tMS\n",
            "MTD\tcv[1]-label\tMS\nMTD\tcv[12]-label\tFOO\n",
        )
        .replace(
            "\topt_global_note\n",
            "\topt_global_note\topt_global_cv_FOO:1_flag\n",
        );
    let text: String = text
        .lines()
        .map(|l| {
            if l.starts_with("SML\t") {
                format!("{l}\tnull\n")
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    let model = parse(&text).mztab;
    let codes: Vec<Code> = validate::semantic(&model, &Config::default(), &Mini)
        .iter()
        .map(|f| f.code)
        .collect();
    assert!(
        !codes.contains(&Code::CvUndeclared) && !codes.contains(&Code::CvUnused),
        "FOO is declared and used: {codes:?}"
    );
}

/// References, slices and arrays of lookups answer as their parts do.
#[test]
fn lookups_combine() {
    struct OverrideMs;
    impl TermLookup for OverrideMs {
        fn covers(&self, prefix: &str) -> bool {
            prefix == "MS"
        }

        fn term(&self, accession: &str) -> Option<Term<'_>> {
            (accession == "MS:1000130").then_some(Term {
                name: "overridden scan",
                obsolete: false,
                value_type: None,
                synonyms: &[],
            })
        }

        fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
            accession == "MS:1000130" && ancestor == "MS:1000531"
        }

        fn version(&self, prefix: &str) -> Option<&str> {
            (prefix == "MS").then_some("9.9.9")
        }
    }
    let by_ref: &dyn TermLookup = &&Mini;
    let array = [&NoTerms as &dyn TermLookup, &Mini];
    let slice: &[&dyn TermLookup] = &array;
    let empty: [&dyn TermLookup; 1] = [&NoTerms];
    let scan = Term {
        name: "positive scan",
        obsolete: false,
        value_type: None,
        synonyms: &[],
    };
    for (name, lookup) in [
        ("reference", by_ref),
        ("array", &array as &dyn TermLookup),
        ("slice", &slice as &dyn TermLookup),
    ] {
        assert!(lookup.covers("MS"), "{name}: covers");
        assert!(!lookup.covers("UO"), "{name}: not UO");
        assert_eq!(lookup.term("MS:1000130"), Some(scan), "{name}: term");
        assert_eq!(lookup.term("UO:1"), None, "{name}: uncovered term");
        assert!(
            lookup.is_descendant("MS:1001477", "MS:1000531"),
            "{name}: descendant"
        );
        assert!(
            !lookup.is_descendant("MS:1000531", "MS:1001477"),
            "{name}: not descendant"
        );
        assert!(
            !lookup.is_descendant("UO:1", "UO:2"),
            "{name}: uncovered descendant"
        );
        assert_eq!(lookup.version("MS"), Some("4.1.0"), "{name}: version");
        assert_eq!(lookup.version("UO"), None, "{name}: uncovered version");
    }
    assert!(!empty.covers("MS"), "empty array");
    assert_eq!(NoTerms.version("MS"), None, "no version");

    let stacked: [&dyn TermLookup; 2] = [&OverrideMs, &Mini];
    assert_eq!(stacked.version("MS"), Some("9.9.9"), "first version wins");
    assert_eq!(
        stacked.term("MS:1000130").map(|t| t.name),
        Some("overridden scan"),
        "first term wins"
    );
    assert!(
        stacked.is_descendant("MS:1000130", "MS:1000531"),
        "first hierarchy wins"
    );
    assert!(
        stacked.term("MS:1001477").is_none(),
        "uncovered accession in first covering lookup does not fall through"
    );
}

#[test]
fn parameter_study_variables_are_semantically_checked() {
    let text = BASE
        .replace(
            "MTD\tstudy_variable_group[1]-datatype\txsd:string",
            "MTD\tstudy_variable_group[1]-datatype\tParameter",
        )
        .replace(
            "MTD\tstudy_variable[1]\tControl",
            "MTD\tstudy_variable[1]\t[MS, MS:9999999, missing, ]",
        );
    let model = parse(&text).mztab;
    let findings = validate::semantic(&model, &Config::default(), &Mini);
    assert!(
        findings.iter().any(|f| f.code == Code::UnknownTerm),
        "Parameter study_variable should be checked against TermLookup: {findings:?}"
    );
}
