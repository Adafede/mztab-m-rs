//! JSON (feature `json`): JSCH reading and writing, located findings, and
//! TSV ↔ JSON agreement on the corpus.

#![cfg(feature = "json")]
#![expect(clippy::expect_used, reason = "test helpers")]

#[cfg(not(target_arch = "wasm32"))]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::{Code, Column, Location, MzTab, Path, SourceLayout, json, parse};

const BASE: &str = include_str!("golden/full.canonical.mztab");
const BASE_JSON: &str = include_str!("golden/full.json");
const PROGENESIS: &str = include_str!(
    "../../../third_party/mzTab-M/examples/2.0/progenesis_2.4.6505.48857_MTBLS263.json"
);
const METABOSCAPE_HEAD: &str =
    r#"{"comment": null, "metadata": null, "small_molecule_summary": []}"#;

fn codes(findings: &mztab_m::Findings) -> Vec<Code> {
    findings.iter().map(|f| f.code).collect()
}

fn pointer(text: &str) -> Location {
    let mut path = Path::new();
    for segment in text.split('/').skip(1) {
        path = match segment.parse::<u32>() {
            Ok(n) => path.index(n),
            Err(_) => path.key(segment.to_owned()),
        };
    }
    Location::Json(path)
}

/// Set `BLESS=1` to rewrite `tests/golden/full.json`.
#[test]
fn the_base_document_round_trips_through_json() {
    let mut model = parse(BASE).mztab;
    model.layout = SourceLayout::default();
    let text = json::to_string(&model).expect("writable");
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BLESS").is_some() {
        let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/full.json");
        std::fs::write(golden, format!("{text}\n")).expect("write golden");
        return;
    }
    assert_eq!(format!("{text}\n"), BASE_JSON, "golden JSON");
    let back = json::parse(&text);
    assert!(back.findings.is_empty(), "{:?}", back.findings);
    assert_eq!(back.mztab, model, "same model");
}

#[test]
fn jmztab_output_reads_and_is_a_fixed_point() {
    let first = json::parse(PROGENESIS);
    assert!(first.findings.is_empty(), "{:?}", first.findings);
    let model = first.mztab;
    assert_eq!(
        model.sml.as_ref().map(|t| t.rows.len()),
        Some(17),
        "SML rows"
    );
    let text = json::to_string(&model).expect("writable");
    let again = json::parse(&text);
    assert!(again.findings.is_empty(), "{:?}", again.findings);
    assert_eq!(again.mztab, model, "fixed point");
    let tsv = mztab_m::to_string(&model).expect("writable");
    let mut from_tsv = parse(&tsv).mztab;
    from_tsv.layout = SourceLayout::default();
    assert_eq!(from_tsv, model, "TSV agrees across full model");
}

#[test]
fn snake_case_dialect_is_rejected() {
    let parsed = json::parse(METABOSCAPE_HEAD);
    assert_eq!(codes(&parsed.findings), [Code::JsonDialect], "dialect");
    assert_eq!(parsed.mztab, MzTab::default(), "nothing read");
}

#[test]
fn syntax_errors_are_located() {
    let cases: &[(&str, u32, u32)] = &[
        ("", 1, 1),
        ("{\n  \"a\": nil}", 2, 8),
        ("[1,]", 1, 4),
        ("{\"a\" 1}", 1, 6),
        ("{\"a\":1 \"b\":2}", 1, 8),
        ("{ 1:2}", 1, 3),
        ("[1 2]", 1, 4),
        ("01", 1, 2),
        ("1.", 1, 1),
        ("1e", 1, 1),
        ("-", 1, 1),
        ("\"abc", 1, 5),
        ("\"a\u{1}\"", 1, 3),
        ("\"\\x\"", 1, 3),
        ("\"\\u12g4\"", 1, 4),
        ("\"\\ud800\"", 1, 8),
        ("\"\\ud800\\u0041\"", 1, 14),
        ("\"\\udc00\"", 1, 8),
        ("\"\\", 1, 3),
        ("{} x", 1, 4),
        ("nul", 1, 1),
    ];
    for (input, line, column) in cases {
        let parsed = json::parse(input);
        let found: Vec<(Code, Location)> = parsed
            .findings
            .iter()
            .map(|f| (f.code, f.location.clone()))
            .collect();
        assert_eq!(
            found,
            [(Code::JsonSyntax, Location::text(*line, *column))],
            "{input:?}"
        );
    }
    let deep = "[".repeat(200);
    assert_eq!(
        codes(&json::parse(&deep).findings),
        [Code::JsonSyntax],
        "too deep"
    );
}

#[test]
fn valid_json_edge_cases_read() {
    let accepted = [
        "\u{feff}{}",
        " {\"metadata\" : null , \"comment\":[]} ",
        "{\"metadata\":{\"title\":\"\\\"q\\\" \\\\ \\/ \\b\\f \\u00e9 \\ud83d\\ude00\"}}",
        "{\"metadata\":{\"title\":-0.5e-3}}",
    ];
    let rejected_shapes = [
        "{\"metadata\":{\"title\":true}}",
        "{\"metadata\":{\"title\":false}}",
        "[]",
    ];
    let first = json::parse(accepted[0]);
    assert!(first.findings.is_empty(), "BOM and empty object");
    let second = json::parse(accepted[1]);
    assert!(second.findings.is_empty(), "spaces, nulls");
    let escapes = json::parse(accepted[2]).mztab;
    let title = escapes
        .metadata
        .title
        .and_then(|t| t.value().map(|t| t.as_str().to_owned()));
    assert_eq!(
        title.as_deref(),
        Some("\"q\" \\ / \u{8}\u{c} é 😀"),
        "escapes"
    );
    let number = json::parse(accepted[3]).mztab;
    let title = number
        .metadata
        .title
        .and_then(|t| t.value().map(|t| t.as_str().to_owned()));
    assert_eq!(title.as_deref(), Some("-0.0005"), "plain number");
    for input in rejected_shapes {
        assert_eq!(
            codes(&json::parse(input).findings),
            [Code::JsonShape],
            "{input}"
        );
    }
}

#[test]
fn exponents_are_written_out() {
    let cases = [
        ("1.5e3", "1500".to_owned()),
        ("-1.5E-2", "-0.015".to_owned()),
        ("12e0", "12".to_owned()),
        ("0.5e1", "5".to_owned()),
        ("1e+2", "100".to_owned()),
        ("123.456e1", "1234.56".to_owned()),
        ("5e-1", "0.5".to_owned()),
        ("1e1000", format!("1{}", "0".repeat(1000))),
        ("1e-1001", format!("0.{}1", "0".repeat(1000))),
        ("1e-1002", "1e-1002".to_owned()),
        ("1e2000", "1e2000".to_owned()),
        (
            "1e99999999999999999999",
            "1e99999999999999999999".to_owned(),
        ),
    ];
    for (lexeme, plain) in cases {
        let input = format!(
            "{{\"metadata\":{{\"assay\":[{{\"id\":1,\"name\":\"a\"}}]}},\
             \"smallMoleculeFeature\":[{{\"smf_id\":1,\"exp_mass_to_charge\":{lexeme}}}]}}"
        );
        let model = json::parse(&input).mztab;
        let row = model
            .smf
            .as_ref()
            .and_then(|t| t.rows.first())
            .expect("a row");
        let written = row
            .exp_mass_to_charge
            .value()
            .map(|d| d.as_str().to_owned())
            .or_else(|| {
                row.exp_mass_to_charge
                    .invalid()
                    .map(|r| r.as_str().to_owned())
            });
        assert_eq!(written.as_deref(), Some(plain.as_str()), "{lexeme}");
    }
    let empty_param_val =
        json::parse(r#"{"metadata":{"quantification_method":{"name":"x","value":""}}}"#);
    assert!(
        empty_param_val.findings.is_empty(),
        "empty parameter value normalizes to None: {:?}",
        empty_param_val.findings
    );
}

#[test]
fn shapes_and_unknown_members_are_reported_at_their_pointer() {
    let input = r#"{
        "metadata": {
            "mzTab-version": "2.1.0-M",
            "title": ["x"],
            "quantification_method": {"cv_label": "MS", "name": "half"},
            "sample": [{"id": 0, "name": "s"}, {"id": "x"}, 3],
            "ms_run": [{"id": 1, "name": "ignored", "location": "file:///a", "instrument_ref": 2,
                        "scan_polarity": [{"id": 1, "cv_label": "MS", "cv_accession": "MS:1000130", "name": "positive scan", "value": null}],
                        "format": {"cv_label": "MS", "cv_accession": "MS:1", "name": "x", "value": {"nested": 1}},
                        "id_format": {"cv_label": null, "cv_accession": null, "name": true, "value": null},
                        "hash_method": {"cv_label": null, "cv_accession": null, "name": "a\"b", "value": null}}],
            "assay": [{"id": 1, "name": "a", "ms_run_ref": [1, true], "sample_ref": [1]}],
            "publication": [{"id": 1, "publication_items": [{"type": "pubmed", "accession": "1"}, {"type": 1}]}],
            "software": [{"id": 1, "parameter": {"name": "tool", "value": "1"}, "setting": ["a", 2, true]}],
            "colunit-small_molecule": ["x=[,,y,]", {"column_name": "c", "param": {"name": "u"}}, {"column_name": 1}, 5],
            "custom": ["[,,d,]", {"id": 3, "name": "c"}],
            "cv": [{"id": 1, "label": "MS", "extra": 1}],
            "description": "tab\there",
            "nope": 1
        },
        "smallMoleculeFeature": [
            {"smf_id": 1, "sme_id_refs": [1, {}], "charge": "x", "isotopomer": 1, "abundance_assay": [1, 2],
             "opt": [{"identifier": "global_note", "param": null, "value": "n"}], "bogus": 1},
            {"smf_id": 2, "late": 1, "opt": [{"identifier": "global_other", "value": "o"}]},
            3
        ],
        "smallMoleculeSummary": {},
        "smallMoleculeEvidence": [{"sme_id": 1, "spectra_ref": [{"ms_run": 1, "reference": "index=1"}, {"ms_run": "x"}]}],
        "extra": true
    }"#;
    let parsed = json::parse(input);
    let found: Vec<(Code, String)> = parsed
        .findings
        .iter()
        .map(|f| (f.code, f.location.to_string()))
        .collect();
    let expect = |code: Code, at: &str| {
        assert!(
            found
                .iter()
                .any(|(c, l)| *c == code && *l == pointer(at).to_string()),
            "{code} at {at} in {found:#?}"
        );
    };
    expect(Code::JsonUnknownKey, "/extra");
    expect(Code::JsonUnknownKey, "/metadata/nope");
    expect(Code::JsonUnknownKey, "/metadata/cv/0/extra");
    expect(Code::JsonShape, "/metadata/title");
    expect(Code::JsonShape, "/metadata/quantification_method");
    expect(Code::JsonShape, "/metadata/sample/0/id");
    expect(Code::JsonShape, "/metadata/sample/1/id");
    expect(Code::JsonShape, "/metadata/sample/2");
    expect(Code::JsonShape, "/metadata/ms_run/0/format");
    expect(Code::JsonShape, "/metadata/ms_run/0/id_format");
    expect(Code::JsonShape, "/metadata/ms_run/0/hash_method");
    expect(Code::JsonShape, "/metadata/assay/0/ms_run_ref/1");
    expect(Code::JsonShape, "/metadata/assay/0/sample_ref");
    expect(
        Code::JsonShape,
        "/metadata/publication/0/publication_items/1",
    );
    expect(Code::JsonShape, "/metadata/software/0/setting/2");
    expect(Code::JsonShape, "/metadata/colunit-small_molecule/2");
    expect(Code::JsonShape, "/metadata/colunit-small_molecule/3");
    expect(Code::JsonText, "/metadata/description");
    expect(Code::JsonShape, "/smallMoleculeFeature/0/sme_id_refs/1");
    expect(Code::JsonShape, "/smallMoleculeFeature/0/isotopomer");
    expect(Code::JsonShape, "/smallMoleculeFeature/0/abundance_assay");
    expect(Code::JsonUnknownKey, "/smallMoleculeFeature/0/bogus");
    expect(Code::JsonUnknownKey, "/smallMoleculeFeature/1/late");
    expect(Code::JsonShape, "/smallMoleculeFeature/2");
    expect(Code::JsonShape, "/smallMoleculeSummary");
    expect(Code::JsonShape, "/smallMoleculeEvidence/0/spectra_ref/1");
    // The TSV reader's findings, relocated.
    expect(Code::NumberSyntax, "/smallMoleculeFeature/0/charge");
    assert_eq!(found.len(), 27, "exact finding count: {found:#?}");
    let model = &parsed.mztab;
    let run = model.metadata.ms_run.iter().next().map(|(_, r)| r);
    assert!(
        run.is_some_and(|r| r.scan_polarity.iter().count() == 1),
        "indexed parameters read"
    );
    assert_eq!(
        model.metadata.custom.iter().count(),
        2,
        "custom by id and position"
    );
    let smf = model.smf.as_ref().expect("SMF");
    assert!(
        smf.header
            .columns
            .iter()
            .any(|c| matches!(c, Column::Opt(o) if o.name.as_str() == "note")),
        "opt column from the first row"
    );
}

/// The header cells (after the prefix) of the table starting with `prefix`.
fn names(model: &MzTab<'_>, prefix: &str) -> Vec<String> {
    let text = mztab_m::to_string(model).expect("writable");
    text.lines()
        .find(|l| l.starts_with(prefix))
        .map_or_else(Vec::new, |l| {
            l.split('\t').skip(1).map(str::to_owned).collect()
        })
}

#[test]
fn empty_tables_get_the_full_header() {
    let input = r#"{"metadata": {"assay": [{"id": 1, "name": "a"}, {"id": 2, "name": "b"}],
        "id_confidence_measure": [{"id": 1, "cv_label": "MS", "cv_accession": "MS:1002890", "name": "fragmentation score", "value": null}]},
        "smallMoleculeEvidence": [], "smallMoleculeFeature": []}"#;
    let model = json::parse(input).mztab;
    let sme = names(&model, "SEH");
    assert_eq!(
        sme.get(16..18).map(<[String]>::to_vec),
        Some(vec![
            "id_confidence_measure[1]".to_owned(),
            "rank".to_owned()
        ]),
        "{sme:?}"
    );
    let smf = names(&model, "SFH");
    assert_eq!(
        smf.last().map(String::as_str),
        Some("abundance_assay[2]"),
        "{smf:?}"
    );
}

#[test]
fn what_json_cannot_carry_is_refused() {
    let unwritable = |text: &str| {
        let mut model = parse(text).mztab;
        model.layout = SourceLayout::default();
        matches!(
            json::to_string(&model),
            Err(mztab_m::WriteError::Unwritable { .. })
        )
    };
    let unknown_key = BASE.replace(
        "MTD\ttitle\t",
        "MTD\tinstrument[1]-analyzer\tx\nMTD\ttitle\t",
    );
    assert!(unwritable(&unknown_key), "unknown metadata key");
    let zero = BASE.replace(
        "MTD\ttitle\t",
        "MTD\tms_run[1]-scan_polarity[0]\tx\nMTD\ttitle\t",
    );
    assert!(unwritable(&zero), "index 0 is not a key (fuzz)");
    let unknown_column = BASE.replace("\topt_global_note", "\tnot_a_column");
    assert!(unwritable(&unknown_column), "unknown column");
    let extra_cell: String = BASE
        .lines()
        .map(|l| {
            if l.starts_with("SML\t1\t") {
                format!("{l}\textra\n")
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    assert!(unwritable(&extra_cell), "row with an extra cell");
    let gap = BASE.replace("abundance_assay[1]", "abundance_assay[7]");
    assert!(unwritable(&gap), "abundance ids differ from the metadata");
    let mut failing = FailAfter(0);
    let model = parse(BASE).mztab;
    assert_eq!(
        json::write(&model, &mut failing),
        Err(mztab_m::WriteError::Fmt),
        "output failure"
    );
}

/// A `fmt::Write` that fails after `n` writes.
struct FailAfter(usize);

impl core::fmt::Write for FailAfter {
    fn write_str(&mut self, _: &str) -> core::fmt::Result {
        if self.0 == 0 {
            return Err(core::fmt::Error);
        }
        self.0 -= 1;
        Ok(())
    }
}

/// What the writer cannot put in JSON form travels as its TSV text.
#[test]
fn invalid_values_travel_as_strings() {
    let edits = BASE
        .replace("MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free quantitation analysis, ]", "MTD\tquantification_method\t[MS, MS:1001834")
        .replace("MTD\tsample_processing[1]\t[MSIO, MSIO:0000146, centrifugation, ]", "MTD\tsample_processing[1]\t[MSIO")
        .replace("MTD\tassay[1]-ms_run_ref\tms_run[1]", "MTD\tassay[1]-ms_run_ref\tms_run[x]")
        .replace("MTD\tms_run[1]-instrument_ref\tinstrument[1]", "MTD\tms_run[1]-instrument_ref\tbad")
        .replace("MTD\tpublication[1]\t", "MTD\tpublication[1]\tnocolon|");
    let mut model = parse(&edits).mztab;
    model.layout = SourceLayout::default();
    let text = json::to_string(&model).expect("writable");
    let back = json::parse(&text);
    assert_eq!(back.mztab, model, "same model, invalid cells included");
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "support/corpus.rs"]
mod corpus_files;

/// TSV → JSON → model on every corpus file: the same model, the same up
/// to the order of columns (JSON groups each abundance kind in one array),
/// or refused with the reason. Set `BLESS=1` to rewrite
/// `tests/golden/json-corpus.txt`.
#[cfg(not(target_arch = "wasm32"))]
mod corpus {
    use alloc::collections::BTreeMap;
    use core::fmt::Write as _;
    use std::path::Path;

    use mztab_m::{SourceLayout, json, parse_bytes};

    use super::corpus_files;

    /// Each table as column → cells, ignoring column order.
    fn by_column(tsv: &str) -> BTreeMap<String, Vec<String>> {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut header: Vec<String> = Vec::new();
        for line in tsv.lines() {
            let cells: Vec<&str> = line.split('\t').collect();
            match cells.first().copied() {
                Some(prefix @ ("SMH" | "SFH" | "SEH")) => {
                    header = cells.iter().map(|c| format!("{prefix}:{c}")).collect();
                }
                Some("SML" | "SMF" | "SME") => {
                    for (name, cell) in header.iter().zip(&cells) {
                        out.entry(name.clone())
                            .or_default()
                            .push((*cell).to_owned());
                    }
                }
                Some(_) | None => {
                    out.entry(String::from("<meta>"))
                        .or_default()
                        .push(line.to_owned());
                }
            }
        }
        if let Some(meta) = out.get_mut("<meta>") {
            meta.sort();
        }
        out
    }

    #[test]
    fn corpus_round_trips_through_json_as_recorded() {
        let (root, paths) = corpus_files::files();
        let mut report = String::new();
        for path in paths {
            let bytes = std::fs::read(&path).expect("readable");
            let mut model = parse_bytes(&bytes).mztab;
            model.layout = SourceLayout::default();
            let name = path.strip_prefix(&root).expect("under root").display();
            let outcome = match json::to_string(&model) {
                Err(e) => format!("refused: {e}"),
                Ok(text) => {
                    let parsed_back = json::parse(&text);
                    assert!(
                        !parsed_back.findings.iter().any(|f| matches!(
                            f.code,
                            crate::Code::JsonSyntax | crate::Code::JsonText
                        )),
                        "{name}: serialized JSON triggered findings: {:?}",
                        parsed_back.findings
                    );
                    let back = parsed_back.mztab;
                    let tsv = |m| mztab_m::to_string(m).expect("writable");
                    if back == model {
                        "same model".to_owned()
                    } else if back.metadata == model.metadata
                        && by_column(&tsv(&back)) == by_column(&tsv(&model))
                    {
                        "same up to column order".to_owned()
                    } else {
                        "DIFFERENT".to_owned()
                    }
                }
            };
            writeln!(report, "{name}: {outcome}").expect("writing to a String");
        }
        let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/json-corpus.txt");
        if std::env::var_os("BLESS").is_some() {
            std::fs::write(&golden, &report).expect("write golden");
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_default();
        assert!(!report.contains("DIFFERENT"), "a file changed:\n{report}");
        assert!(
            expected == report,
            "outcomes changed; BLESS=1 if intended:\n{report}"
        );
    }
}

/// Every JSON form, given a value of the wrong kind.
#[test]
fn every_form_reports_the_wrong_kind() {
    let cases: &[(&str, &str)] = &[
        (r#"{"metadata": 1}"#, "/metadata"),
        (
            r#"{"metadata": {"quantification_method": 1}}"#,
            "/metadata/quantification_method",
        ),
        (
            r#"{"metadata": {"ms_run": [{"id": 1, "parameters": 1}]}}"#,
            "/metadata/ms_run/0/parameters",
        ),
        (
            r#"{"metadata": {"assay": [{"id": 1, "ms_run_ref": 1}]}}"#,
            "/metadata/assay/0/ms_run_ref",
        ),
        (
            r#"{"metadata": {"publication": [{"id": 1, "publication_items": 1}]}}"#,
            "/metadata/publication/0/publication_items",
        ),
        (
            r#"{"metadata": {"publication": [{"id": 1, "publication_items": [5]}]}}"#,
            "/metadata/publication/0/publication_items/0",
        ),
        (
            r#"{"smallMoleculeFeature": [1]}"#,
            "/smallMoleculeFeature/0",
        ),
        (
            r#"{"smallMoleculeFeature": [{"smf_id": true}]}"#,
            "/smallMoleculeFeature/0/smf_id",
        ),
        (
            r#"{"smallMoleculeFeature": [{"adduct_ion": true}]}"#,
            "/smallMoleculeFeature/0/adduct_ion",
        ),
        (
            r#"{"smallMoleculeFeature": [{"exp_mass_to_charge": true}]}"#,
            "/smallMoleculeFeature/0/exp_mass_to_charge",
        ),
        (
            r#"{"smallMoleculeFeature": [{"sme_id_refs": 5}]}"#,
            "/smallMoleculeFeature/0/sme_id_refs",
        ),
        (
            r#"{"smallMoleculeSummary": [{"database_identifier": 5}]}"#,
            "/smallMoleculeSummary/0/database_identifier",
        ),
        (
            r#"{"smallMoleculeSummary": [{"database_identifier": [true]}]}"#,
            "/smallMoleculeSummary/0/database_identifier/0",
        ),
        (
            r#"{"smallMoleculeSummary": [{"theoretical_neutral_mass": 5}]}"#,
            "/smallMoleculeSummary/0/theoretical_neutral_mass",
        ),
        (
            r#"{"smallMoleculeEvidence": [{"spectra_ref": 5}]}"#,
            "/smallMoleculeEvidence/0/spectra_ref",
        ),
        (
            r#"{"metadata": {"assay": [{"id": 1, "name": "a"}]}, "smallMoleculeFeature": [{"abundance_assay": 5}]}"#,
            "/smallMoleculeFeature/0/abundance_assay",
        ),
    ];
    for (input, at) in cases {
        let parsed = json::parse(input);
        assert!(
            parsed
                .findings
                .iter()
                .any(|f| f.code == Code::JsonShape && f.location == pointer(at)),
            "{input}: {:?}",
            parsed.findings
        );
    }
}

/// Lenient readings: numbers as text, nulls in arrays, odd `opt` members.
#[test]
fn lenient_forms_read() {
    let input = r#"{
        "metadata": {
            "software": [{"id": 1, "parameter": {"name": "tool", "value": 2}, "setting": [null, "a"]}],
            "sample": [{"id": 1, "species": [null]}],
            "description": "a\nb"
        },
        "smallMoleculeSummary": [
            {"sml_id": 1, "database_identifier": [1, null, "x"], "opt": [{"identifier": "global_a", "value": "1"}]},
            {"sml_id": 2, "opt": 5},
            {"sml_id": 3, "opt": [{"identifier": "global_b", "value": "2"}]}
        ],
        "smallMoleculeFeature": [{"smf_id": 1, "opt": [{"identifier": 1, "value": "v"}]}]
    }"#;
    let parsed = json::parse(input);
    let text = mztab_m::to_string(&parsed.mztab).expect("writable");
    assert!(
        text.contains("MTD\tsoftware[1]\t[, , tool, 2]"),
        "numeric value: {text}"
    );
    assert!(
        text.contains("SML\t1\t") && text.contains("1|null|x"),
        "list items: {text}"
    );
    assert!(
        parsed.findings.iter().any(|f| f.code == Code::JsonText),
        "line feed in a string"
    );
    assert!(
        parsed
            .findings
            .iter()
            .any(|f| f.code == Code::JsonShape
                && f.location == pointer("/smallMoleculeSummary/1/opt")),
        "invalid opt container"
    );
    assert!(
        parsed.findings.iter().any(|f| {
            f.code == Code::JsonShape
                && f.location == pointer("/smallMoleculeFeature/0/opt/0/identifier")
        }),
        "non-string opt identifier"
    );
}

/// Items that are not parameter objects keep their index by position,
/// padded with `null` (found by fuzzing); too much padding is refused.
#[test]
fn indices_survive_without_ids() {
    let edits = BASE
        .replace(
            "MTD\tsoftware[1]-setting[1]\tMass tolerance = 5 ppm",
            "MTD\tsoftware[1]-setting[3]\tMass tolerance = 5 ppm\nMTD\tsoftware[1]-setting[4]\tRT tolerance = 10 s",
        )
        .replace(
            "MTD\tms_run[1]-scan_polarity[1]\t[MS, MS:1000130, positive scan, ]",
            "MTD\tms_run[1]-scan_polarity[2]\t[MS-broken",
        );
    let mut model = parse(&edits).mztab;
    model.layout = SourceLayout::default();
    let text = json::to_string(&model).expect("writable");
    assert!(
        text.contains(r#""setting":[null,null,"Mass tolerance = 5 ppm","RT tolerance = 10 s"]"#),
        "consecutive positioned strings: {text}"
    );
    assert!(
        text.contains(r#""scan_polarity":[null,"[MS-broken"]"#),
        "padded: {text}"
    );
    assert_eq!(json::parse(&text).mztab, model, "indices kept");
    let far = BASE.replace(
        "MTD\tsoftware[1]-setting[1]\t",
        "MTD\tsoftware[1]-setting[5000]\t",
    );
    let mut model = parse(&far).mztab;
    model.layout = SourceLayout::default();
    assert!(
        matches!(
            json::to_string(&model),
            Err(mztab_m::WriteError::Unwritable { .. })
        ),
        "too far apart"
    );
}

/// Repeated members are read once (found by fuzzing); a TSV header that
/// repeats a column has no JSON form.
#[test]
fn repeated_members_and_columns() {
    let parsed = json::parse(
        r#"{"metadata": {"title": "a", "title": "b"},
            "smallMoleculeFeature": [{"smf_id": 1, "smf_id": 2, "charge": 1,
              "opt": [{"identifier": "global_x", "value": "1"}, {"identifier": "global_x", "value": "2"}]}]}"#,
    );
    let shapes: Vec<String> = parsed
        .findings
        .iter()
        .filter(|f| f.code == Code::JsonShape)
        .map(|f| f.location.to_string())
        .collect();
    assert_eq!(
        shapes,
        ["/metadata/title", "/smallMoleculeFeature/0/smf_id"],
        "{:?}",
        parsed.findings
    );
    let text = json::to_string(&parsed.mztab).expect("writable");
    assert_eq!(json::parse(&text).mztab, parsed.mztab, "stable");
    let sml_repeat = BASE.replace("SMH\tSML_ID\t", "SMH\tSML_ID\tSML_ID\t");
    let mut twice = parse(&sml_repeat).mztab;
    twice.layout = SourceLayout::default();
    assert!(
        matches!(
            json::to_string(&twice),
            Err(mztab_m::WriteError::Unwritable { .. })
        ),
        "repeated column"
    );
}

/// The edges mutation testing found untested.
#[test]
fn mutation_edges() {
    // A carriage return may end any cell but the last of a line.
    let cr = json::parse(
        r#"{"metadata": {"title": "a\r"},
            "smallMoleculeFeature": [{"smf_id": 1, "adduct_ion": "x\r", "charge": 1}]}"#,
    );
    let text: Vec<String> = cr
        .findings
        .iter()
        .filter(|f| f.code == Code::JsonText)
        .map(|f| f.location.to_string())
        .collect();
    assert_eq!(text, ["/metadata/title"], "only the line's last cell");

    // `id` wins over position; `null` items only pad; strings in lists
    // are TSV text.
    let ids = json::parse(
        r#"{"metadata": {"custom": [null, {"id": 3, "name": "c"}],
            "ms_run": [{"id": 1, "parameters": [null, "[, , p, ]"]}]}}"#,
    );
    assert!(ids.findings.is_empty(), "{:?}", ids.findings);
    let tsv = mztab_m::to_string(&ids.mztab).expect("writable");
    assert!(tsv.contains("MTD\tcustom[3]\t[, , c, ]"), "{tsv}");
    assert!(
        tsv.contains("MTD\tms_run[1]-parameters\tnull|[, , p, ]"),
        "{tsv}"
    );

    // Arrays longer than the metadata's ids are indexed by position.
    let long = json::parse(
        r#"{"metadata": {"assay": [{"id": 4, "name": "a"}]},
            "smallMoleculeFeature": [{"smf_id": 1, "abundance_assay": [1, 2, 3]}]}"#,
    );
    let header = names(&long.mztab, "SFH");
    assert_eq!(
        header.get(1..),
        Some(
            &[
                "abundance_assay[4]".to_owned(),
                "abundance_assay[2]".to_owned(),
                "abundance_assay[3]".to_owned()
            ][..]
        ),
        "{header:?}"
    );

    // Messages say what was expected.
    let rendered = |input: &str| -> Vec<String> {
        json::parse(input)
            .findings
            .iter()
            .map(ToString::to_string)
            .collect()
    };
    assert!(
        rendered(r#"{"metadata": {"quantification_method": 1}}"#)
            .iter()
            .any(|m| m.ends_with("expected a parameter")),
        "metadata form"
    );
    assert!(
        rendered(r#"{"smallMoleculeFeature": [{"exp_mass_to_charge": true}]}"#)
            .iter()
            .any(|m| m.ends_with("expected a number")),
        "column form"
    );
    assert!(
        rendered(&"[".repeat(200))
            .iter()
            .any(|m| m.ends_with("nesting too deep")),
        "depth"
    );
    assert!(
        rendered(&format!("{}{}", "{\"a\":".repeat(130), "1"))
            .iter()
            .any(|m| m.ends_with("nesting too deep")),
        "object depth"
    );

    // Control characters are escaped; numeric-looking text stays text.
    let mut model = parse(BASE).mztab;
    model.layout = SourceLayout::default();
    model.metadata.title = Some(mztab_m::Cell::Value(
        mztab_m::Text::new("a\u{1}b").expect("a control character is text"),
    ));
    let edited = BASE.replace("\tGlucose|null\t", "\t123|null\t");
    let mut numeric = parse(&edited).mztab;
    numeric.layout = SourceLayout::default();
    // A carriage return mid-cell is not text, but TSV carries it.
    model.metadata.description = Some(mztab_m::Cell::Invalid(mztab_m::Raw::new("q\" \\ c\rd")));
    let text = json::to_string(&model).expect("writable");
    assert!(text.contains(r#""title":"a\u0001b""#), "escaped");
    assert!(
        text.contains(r#""description":"q\" \\ c\rd""#),
        "quote, backslash, carriage return: {text}"
    );
    assert_eq!(json::parse(&text).mztab, model, "control character");
    let text = json::to_string(&numeric).expect("writable");
    assert!(text.contains(r#""chemical_name":["123",null]"#), "{text}");

    // Padding up to the limit is written; one more is refused.
    for (index, writable) in [(1025, true), (1026, false)] {
        let edited = BASE.replace(
            "MTD\tsoftware[1]-setting[1]\t",
            &format!("MTD\tsoftware[1]-setting[{index}]\t"),
        );
        let mut model = parse(&edited).mztab;
        model.layout = SourceLayout::default();
        assert_eq!(json::to_string(&model).is_ok(), writable, "{index}");
    }
}

/// Findings as JSON: every kind of location, with and without detail, and
/// a failing output at every point.
#[test]
fn findings_as_json() {
    use mztab_m::{Detail, Finding, Findings};
    let mut findings = Findings::new();
    let at = |l| Finding::new(Code::JsonShape, l, Detail::None);
    findings.push(at(Location::text(1, 2)));
    findings.push(at(Location::text(1, 2).with_cell(3)));
    findings.push(at(pointer("/a/0")));
    findings.push(at(Location::Model(Path::new().key("sml").index(1))));
    findings.push(Finding::new(
        Code::NumberSyntax,
        Location::Document,
        Detail::Expected("x"),
    ));
    let mut out = String::new();
    json::write_findings(&findings, &mut out).expect("written");
    assert!(
        out.starts_with("[{") && out.ends_with("}]") && out.matches("},{").count() == 4,
        "five objects, comma-separated: {out}"
    );
    for part in [
        r#""location":{"line":1,"column":2,"cell":null}"#,
        r#""location":{"line":1,"column":2,"cell":3}"#,
        r#""location":{"pointer":"/a/0"}"#,
        r#""location":{"path":"sml[1]"}"#,
        r#""location":null"#,
        r#""detail":null"#,
        r#""detail":"expected x""#,
        r#""jmztab":[1018,1019,1022,1033]"#,
    ] {
        assert!(out.contains(part), "{part} in {out}");
    }
    for n in 0.. {
        let mut failing = FailAfter(n);
        if json::write_findings(&findings, &mut failing).is_ok() {
            break;
        }
    }
}

/// `COM` lines travel as `comment` objects; read back, they follow the
/// metadata (D-075).
#[test]
fn comments_travel() {
    let source = format!("COM\tfirst\n{BASE}COM\tlast\n");
    let model = parse(&source).mztab;
    let text = json::to_string(&model).expect("writable");
    assert!(
        text.ends_with(r#""comment":[{"prefix":"COM","msg":"first","line_number":1},{"prefix":"COM","msg":"last","line_number":141}]}"#),
        "{}",
        text.get(text.len().saturating_sub(160)..).unwrap_or_default()
    );
    let back = json::parse(&text);
    assert!(back.findings.is_empty(), "{:?}", back.findings);
    let tsv = mztab_m::to_string(&back.mztab).expect("writable");
    assert_eq!(
        tsv.replace("COM\tfirst\nCOM\tlast\n", ""),
        BASE,
        "the same document, comments after the metadata"
    );
    assert!(tsv.contains("\nCOM\tfirst\nCOM\tlast\n\nSMH"), "{tsv}");
    let shapes = json::parse(r#"{"comment": [{"msg": null}, {"msg": 3}, {"msg": "a\nb"}]}"#);
    let codes: Vec<Code> = shapes.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, [Code::JsonShape, Code::JsonText], "bad comments");
    assert_eq!(
        shapes.mztab.layout.lines.len(),
        1,
        "an empty comment is kept"
    );
}

/// `strip_unknown` drops what each table and the metadata could not place.
#[test]
fn stripping_unknown_content() {
    let edited: String = BASE
        .lines()
        .map(|l| {
            let tail = if l.starts_with("SMH\t") || l.starts_with("SFH\t") || l.starts_with("SEH\t")
            {
                "\tbogus\n"
            } else if l.starts_with("SML\t") || l.starts_with("SMF\t") || l.starts_with("SME\t") {
                "\tx\textra\n"
            } else {
                "\n"
            };
            [l, tail].concat()
        })
        .collect::<String>()
        .replace("MTD\ttitle\t", "MTD\tnot-a-key\tx\nMTD\ttitle\t");
    let mut model = parse(&edited).mztab;
    let dropped = model.strip_unknown();
    assert_eq!(
        (dropped.metadata, dropped.columns, dropped.cells),
        (1, 3, 12),
        "one key, a column per table, two cells per row"
    );
    model.layout = SourceLayout::default();
    let mut base = parse(BASE).mztab;
    base.layout = SourceLayout::default();
    assert_eq!(model, base, "what remains is the base document");
}

#[test]
fn malformed_params_and_spectra_refs_fall_back_to_json_strings() {
    let input = concat!(
        "MTD\tmzTab-version\t2.1.0-M\n",
        "MTD\tmzTab-ID\tX\n",
        "MTD\tsample_processing[1]\tnot-a-param\n",
        "SEH\tSME_ID\tspectra_ref\n",
        "SME\t1\tbad-ref\n",
    );
    let mut model = parse(input).mztab;
    model.layout = SourceLayout::default();
    let text = json::to_string(&model).expect("writable");
    assert!(
        text.contains(r#""sample_processing":"not-a-param""#),
        "param list fallback: {text}"
    );
    assert!(
        text.contains(r#""spectra_ref":"bad-ref""#),
        "spectra_ref fallback: {text}"
    );
    let back = json::parse(&text);
    assert_eq!(back.mztab, model, "round-trips via fallback string");
}

#[test]
fn json_union_columns_and_row_validation() {
    let union = json::parse(
        r#"{"metadata": {"assay": [{"id": 1, "name": "a"}]},
            "smallMoleculeSummary": [
                {"sml_id": 1, "abundance_assay": null, "opt": null},
                {"sml_id": 2, "chemical_name": ["Caffeine"], "abundance_assay": [42.0],
                 "opt": [{"identifier": "global_tag", "value": "x"}]}
            ],
            "smallMoleculeFeature": [3, {"smf_id": 1}],
            "smallMoleculeEvidence": [3, 4]}"#,
    );
    assert_eq!(
        codes(&union.findings),
        [Code::JsonShape, Code::JsonShape, Code::JsonShape],
        "expected three non-object row findings: {:?}",
        union.findings
    );
    let sml_cols = names(&union.mztab, "SMH");
    assert!(
        sml_cols.contains(&"chemical_name".to_owned())
            && sml_cols.contains(&"abundance_assay[1]".to_owned())
            && sml_cols.contains(&"opt_global_tag".to_owned()),
        "union columns missing: {sml_cols:?}"
    );
    let sml = union.mztab.sml.as_ref().expect("SML");
    assert_eq!(sml.rows.len(), 2, "both SML rows kept");
    assert!(
        sml.rows
            .get(1)
            .is_some_and(|r| r.chemical_name.value().is_some()),
        "row 1 chemical_name kept"
    );
    let smf = union.mztab.smf.as_ref().expect("SMF");
    assert_eq!(smf.rows.len(), 1, "object row after non-object row kept");

    let row_checks = json::parse(
        r#"{"metadata": {"assay": [{"id": 1, "name": "a"}]},
            "smallMoleculeFeature": [
                {"smf_id": 1, "abundance_assay": [1.0],
                 "opt": [{"identifier": "a\nb", "value": "1"}, {"identifier": "global_ok", "value": "ok"}]},
                {"smf_id": 2, "abundance_assay": [1.0, 2.0], "opt": 5},
                {"smf_id": 3, "abundance_assay": "bad", "opt": [123, {"identifier": 1, "value": "v"}]}
            ]}"#,
    );
    let check_locs: Vec<(Code, String)> = row_checks
        .findings
        .iter()
        .map(|f| (f.code, f.location.to_string()))
        .collect();
    for (code, path) in [
        (Code::JsonText, "/smallMoleculeFeature/0/opt/0/identifier"),
        (Code::JsonShape, "/smallMoleculeFeature/1/abundance_assay"),
        (Code::JsonShape, "/smallMoleculeFeature/1/opt"),
        (Code::JsonShape, "/smallMoleculeFeature/2/abundance_assay"),
        (Code::JsonShape, "/smallMoleculeFeature/2/opt/0"),
        (Code::JsonShape, "/smallMoleculeFeature/2/opt/1/identifier"),
    ] {
        assert!(
            check_locs
                .iter()
                .any(|(c, l)| *c == code && l == &pointer(path).to_string()),
            "missing ({code}, {path}) in {check_locs:#?}"
        );
    }
    let cols = names(&row_checks.mztab, "SFH");
    assert!(
        cols.contains(&"opt_global_ok".to_owned()) && !cols.iter().any(|c| c.contains('\n')),
        "header kept without bad opt column: {cols:?}"
    );

    let bad_colunit =
        json::parse(r#"{"metadata": {"colunit-small_molecule": [{"column_name": "c"}]}}"#);
    assert!(
        bad_colunit.findings.iter().any(|f| {
            f.code == Code::JsonShape
                && f.location == pointer("/metadata/colunit-small_molecule/0/param")
        }),
        "missing colunit param: {:?}",
        bad_colunit.findings
    );
}

#[test]
fn json_numeric_coercion_and_integer_serialization() {
    for col in [
        "abundance_assay[0]",
        "abundance_assay[01]",
        "abundance_assay[+1]",
    ] {
        let bad_tsv = format!("SMH\tSML_ID\t{col}\nSML\t1\t1.0\n");
        let mut bad_col = parse(&bad_tsv).mztab;
        bad_col.layout = SourceLayout::default();
        assert!(
            matches!(
                json::to_string(&bad_col),
                Err(mztab_m::WriteError::Unwritable { .. })
            ),
            "non-canonical index {col} must be refused"
        );
    }

    let nums = json::parse(
        r#"{"metadata": {"ms_run": [{"id": 1.0, "location": "file:///a"}],
                         "sample": [{"id": 1e0, "name": "s"}],
                         "assay": [{"id": 1.50e1, "name": "a", "sample_ref": 1.0, "ms_run_ref": [1.0]}]},
            "smallMoleculeFeature": [{"smf_id": 1e1, "charge": -0, "exp_mass_to_charge": 0e10000, "retention_time_in_seconds": -0e9999999}],
            "smallMoleculeEvidence": [{"sme_id": 1.0, "spectra_ref": [{"ms_run": 1e0, "reference": "index=1"}]}]}"#,
    );
    assert!(
        nums.findings.is_empty(),
        "numeric coercion findings: {:?}",
        nums.findings
    );
    let smf_row = nums
        .mztab
        .smf
        .as_ref()
        .and_then(|t| t.rows.first())
        .expect("SMF row");
    assert_eq!(
        smf_row.smf_id.value().map(ToString::to_string).as_deref(),
        Some("10")
    );
    assert_eq!(smf_row.charge.value().copied(), Some(0));
    assert_eq!(
        smf_row
            .exp_mass_to_charge
            .value()
            .map(mztab_m::Decimal::as_str),
        Some("0")
    );
    assert_eq!(
        smf_row
            .retention_time_in_seconds
            .value()
            .map(mztab_m::Decimal::as_str),
        Some("-0")
    );
    let big_digits = format!(
        r#"{{"smallMoleculeFeature":[{{"smf_id":1,"exp_mass_to_charge":0.{}1e1100}}]}}"#,
        "0".repeat(1200)
    );
    let big = json::parse(&big_digits);
    let big_val = big
        .mztab
        .smf
        .as_ref()
        .and_then(|t| t.rows.first())
        .and_then(|r| r.exp_mass_to_charge.value())
        .map(|d| d.as_str().to_owned());
    assert_eq!(
        big_val.as_deref(),
        Some(format!("0.{}1", "0".repeat(100)).as_str()),
        "no padding needed inside digits"
    );

    let bad_ints = json::parse(
        r#"{"metadata": {"assay": [{"id": 1.5, "name": "a", "sample_ref": 1.5, "ms_run_ref": [1.5]}]},
            "smallMoleculeFeature": [{"smf_id": 1.5, "charge": 1e2000}],
            "smallMoleculeEvidence": [{"sme_id": 1, "spectra_ref": [{"ms_run": 1.5, "reference": "index=1"}]}]}"#,
    );
    assert!(
        bad_ints
            .findings
            .iter()
            .any(|f| f.code == Code::JsonShape && f.location == pointer("/metadata/assay/0/id")),
        "fractional id must fail: {:?}",
        bad_ints.findings
    );
}

#[test]
fn json_integer_round_trip_escapes_non_integers() {
    let mut non_int = parse(
        "SMH\tSML_ID\nSML\t1\n\n\
         SFH\tSMF_ID\tSME_ID_REFS\tcharge\n\
         SMF\t1\t1.5\t1.0\n\
         SMF\t2\t1\t1e2\n",
    )
    .mztab;
    non_int.layout = SourceLayout::default();
    let non_int_json = json::to_string(&non_int).expect("writable");
    assert!(
        non_int_json.contains(r#""sme_id_refs":["1.5"]"#)
            && non_int_json.contains(r#""charge":"1.0""#)
            && non_int_json.contains(r#""charge":"1e2""#),
        "non-integer invalid integers must serialize as JSON strings: {non_int_json}"
    );
    assert_eq!(json::parse(&non_int_json).mztab, non_int, "round-trips");

    if let Some(smf) = non_int.smf.as_mut()
        && let Some(row) = smf.rows.get_mut(1)
    {
        row.charge = mztab_m::Cell::Invalid(mztab_m::Raw::new("-0"));
    }
    let neg_zero_json = json::to_string(&non_int).expect("writable");
    assert!(
        neg_zero_json.contains(r#""charge":"-0""#),
        "-0 invalid integer must serialize as JSON string: {neg_zero_json}"
    );
}

#[test]
fn json_metadata_lists_comments_and_ids() {
    let empty_meta = json::parse(
        r#"{"metadata": {"ms_run": [{"id": 1, "location": "file:///a"}],
                         "assay": [{"id": 1, "ms_run_ref": []}],
                         "publication": [{"id": 1, "publication_items": []}],
                         "protocol": [{"id": 1, "parameters": []}]},
            "smallMoleculeSummary": [{"sml_id": 1, "database_identifier": []}]}"#,
    );
    assert!(empty_meta.findings.is_empty(), "{:?}", empty_meta.findings);
    let empty_tsv = mztab_m::to_string(&empty_meta.mztab).expect("writable");
    assert!(
        !empty_tsv.contains("MTD\tassay[1]-ms_run_ref")
            && !empty_tsv.contains("MTD\tpublication[1]")
            && !empty_tsv.contains("MTD\tprotocol[1]-parameters"),
        "empty metadata arrays omitted: {empty_tsv}"
    );
    assert!(
        empty_meta
            .mztab
            .sml
            .as_ref()
            .and_then(|t| t.rows.first())
            .is_some_and(|r| r.database_identifier.is_null()),
        "table [] still becomes Cell::Null"
    );

    let list_bad = json::parse(
        r#"{"metadata": {"publication": [{"id": 1, "publication_items": [{"type": "doi", "accession": "1|2"}]}],
                         "protocol": [{"id": 1, "parameters": ["null"]}]},
            "smallMoleculeSummary": [{"sml_id": 1, "database_identifier": ["a|b"], "chemical_formula": ["null"]}],
            "smallMoleculeEvidence": [{"sme_id": 1, "spectra_ref": [{"ms_run": 1, "reference": "x|y"}]}]}"#,
    );
    let list_text_locs: Vec<String> = list_bad
        .findings
        .iter()
        .filter(|f| f.code == Code::JsonText)
        .map(|f| f.location.to_string())
        .collect();
    for p in [
        "/metadata/publication/0/publication_items/0",
        "/metadata/protocol/0/parameters/0",
        "/smallMoleculeSummary/0/database_identifier/0",
        "/smallMoleculeSummary/0/chemical_formula/0",
        "/smallMoleculeEvidence/0/spectra_ref/0",
    ] {
        assert!(
            list_text_locs.contains(&pointer(p).to_string()),
            "missing JsonText at {p} in {list_text_locs:?}"
        );
    }

    let sv_param = json::parse(
        r#"{"metadata": {"study_variable": [{"id": 1, "name": {"cv_label": "NCIT", "cv_accession": "NCIT:C20197", "name": "Male"}},
                                            {"id": 2, "name": true}]}}"#,
    );
    assert_eq!(
        codes(&sv_param.findings),
        [Code::JsonShape],
        "{:?}",
        sv_param.findings
    );
    let sv_name = sv_param
        .mztab
        .metadata
        .study_variable
        .iter()
        .next()
        .and_then(|(_, sv)| sv.name.as_ref())
        .and_then(|c| c.value().map(|t| t.as_str().to_owned()));
    assert_eq!(sv_name.as_deref(), Some("[NCIT, NCIT:C20197, Male, ]"));

    let num_fields = json::parse(
        r#"{"metadata": {"quantification_method": {"cv_label": 1, "cv_accession": "MS:1", "name": "x"},
                         "small_molecule-quantification_unit": {"name": 1}}}"#,
    );
    let num_shapes: Vec<String> = num_fields
        .findings
        .iter()
        .filter(|f| f.code == Code::JsonShape)
        .map(|f| f.location.to_string())
        .collect();
    assert!(
        num_shapes.contains(&pointer("/metadata/quantification_method").to_string())
            && num_shapes
                .contains(&pointer("/metadata/small_molecule-quantification_unit").to_string()),
        "{num_shapes:?}"
    );
}

#[test]
fn json_comments_and_sorted_ids() {
    let bad_comments = json::parse(
        r#"{"comment": [
            123,
            true,
            {"prefix": "BAD", "msg": "x", "line_number": 0, "extra": 1},
            {"prefix": "BAD", "msg": "bad prefix only", "line_number": 1},
            {"prefix": "COM", "msg": "bad line only", "line_number": 0},
            {"prefix": null, "msg": "null optional fields", "line_number": null}
        ]}"#,
    );
    assert_eq!(
        bad_comments.mztab.layout.lines.len(),
        1,
        "only the valid comment with null optional fields becomes a COM line"
    );
    for (code, path) in [
        (Code::JsonShape, "/comment/0"),
        (Code::JsonShape, "/comment/1"),
        (Code::JsonShape, "/comment/2/prefix"),
        (Code::JsonShape, "/comment/2/line_number"),
        (Code::JsonUnknownKey, "/comment/2/extra"),
        (Code::JsonShape, "/comment/3/prefix"),
        (Code::JsonShape, "/comment/4/line_number"),
    ] {
        assert!(
            bad_comments
                .findings
                .iter()
                .any(|f| f.code == code && f.location == pointer(path)),
            "{code:?} at {path} in {:?}",
            bad_comments.findings
        );
    }
    assert_eq!(
        bad_comments.findings.len(),
        7,
        "exact finding count: {:?}",
        bad_comments.findings
    );
    let non_array_comment = json::parse(r#"{"comment": 5}"#);
    assert_eq!(
        codes(&non_array_comment.findings),
        [Code::JsonShape],
        "non-array comment"
    );
    let snake_with_comment = json::parse(r#"{"comment": [123], "small_molecule_summary": []}"#);
    assert_eq!(
        codes(&snake_with_comment.findings),
        [Code::JsonDialect],
        "comments() must not run after SNAKE_DIALECT"
    );

    let sorted_ids = json::parse(
        r#"{"metadata": {"assay": [{"id": 2, "name": "b"}, {"id": 1, "name": "a"}, {"id": 2, "name": "dup"}]},
            "smallMoleculeFeature": [{"smf_id": 1, "abundance_assay": [10.0, 20.0]}]}"#,
    );
    let sfh = names(&sorted_ids.mztab, "SFH");
    assert_eq!(
        sfh,
        ["SMF_ID", "abundance_assay[1]", "abundance_assay[2]"],
        "ids sorted & deduped: {sfh:?}"
    );
}
