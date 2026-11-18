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
    assert_eq!(parse(&tsv).mztab.sml, model.sml, "TSV agrees");
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
    for input in &accepted[4..6] {
        assert_eq!(
            codes(&json::parse(input).findings),
            [Code::JsonShape],
            "{input}"
        );
    }
    assert_eq!(
        codes(&json::parse(accepted[6]).findings),
        [Code::JsonShape],
        "not an object"
    );
}

#[test]
fn exponents_are_written_out() {
    let cases = [
        ("1.5e3", "1500"),
        ("-1.5E-2", "-0.015"),
        ("12e0", "12"),
        ("0.5e1", "5"),
        ("1e+2", "100"),
        ("123.456e1", "1234.56"),
        ("5e-1", "0.5"),
        ("1e2000", "1e2000"),
        ("1e99999999999999999999", "1e99999999999999999999"),
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
        assert_eq!(written.as_deref(), Some(plain), "{lexeme}");
    }
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

/// TSV → JSON → model on every corpus file: the same model, the same up
/// to the order of columns (JSON groups each abundance kind in one array),
/// or refused with the reason. Set `BLESS=1` to rewrite
/// `tests/golden/json-corpus.txt`.
#[cfg(not(target_arch = "wasm32"))]
mod corpus {
    use alloc::collections::BTreeMap;
    use core::fmt::Write as _;
    use std::path::{Path, PathBuf};

    use mztab_m::{SourceLayout, json, parse_bytes};

    fn files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("corpus directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                files(&path, out);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mztab"))
            {
                out.push(path);
            }
        }
    }

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
                    out.entry(line.to_owned()).or_default();
                }
            }
        }
        out
    }

    #[test]
    fn corpus_round_trips_through_json_as_recorded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/mzTab-M/examples");
        let mut paths = Vec::new();
        files(&root, &mut paths);
        paths.sort();
        let mut report = String::new();
        for path in paths {
            let bytes = std::fs::read(&path).expect("readable");
            let mut model = parse_bytes(&bytes).mztab;
            model.layout = SourceLayout::default();
            let name = path.strip_prefix(&root).expect("under root").display();
            let outcome = match json::to_string(&model) {
                Err(e) => format!("refused: {e}"),
                Ok(text) => {
                    let back = json::parse(&text).mztab;
                    let tsv = |m| mztab_m::to_string(m).expect("writable");
                    if back == model {
                        "same model".to_owned()
                    } else if by_column(&tsv(&back)) == by_column(&tsv(&model)) {
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
}

/// Items that are not parameter objects keep their index by position,
/// padded with `null` (found by fuzzing); too much padding is refused.
#[test]
fn indices_survive_without_ids() {
    let edits = BASE
        .replace(
            "MTD\tsoftware[1]-setting[1]\t",
            "MTD\tsoftware[1]-setting[3]\t",
        )
        .replace(
            "MTD\tms_run[1]-scan_polarity[1]\t[MS, MS:1000130, positive scan, ]",
            "MTD\tms_run[1]-scan_polarity[2]\t[MS-broken",
        );
    let mut model = parse(&edits).mztab;
    model.layout = SourceLayout::default();
    let text = json::to_string(&model).expect("writable");
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
