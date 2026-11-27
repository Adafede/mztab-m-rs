//! The shared operations, as the bindings use them.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::{Code, Severity};
use mztab_m_api::{
    Error, Fact, Format, Layer, Settings, check, convert, facts, facts_json, read, read_borrowed,
};

const BASE: &str = include_str!("../../mztab-m/tests/golden/full.canonical.mztab");
const BASE_JSON: &str = include_str!("../../mztab-m/tests/golden/full.json");

#[test]
fn names_parse() {
    assert_eq!("json".parse::<Format>(), Ok(Format::Json), "json");
    assert_eq!("tsv".parse::<Format>(), Ok(Format::Tsv), "tsv");
    assert_eq!("syntax".parse::<Layer>(), Ok(Layer::Syntax), "syntax");
    assert_eq!(
        "structure".parse::<Layer>(),
        Ok(Layer::Structure),
        "structure"
    );
    assert_eq!("semantic".parse::<Layer>(), Ok(Layer::Semantic), "semantic");
    assert!("all".parse::<Layer>().is_err(), "unknown layer");
    let errors = [
        (
            "xml".parse::<Format>().map(|_| ()),
            "unknown format `xml` (tsv, json)",
        ),
        (
            Err(Error::Layer("all".to_owned())),
            "unknown layer `all` (syntax, structure, semantic)",
        ),
        (
            Settings::default().set("nonsense").map(|_| ()),
            "expected CODE=LEVEL, not `nonsense`",
        ),
        (
            Settings::default().set("MZT-Z999=error").map(|_| ()),
            "unknown code `MZT-Z999`",
        ),
        (
            Settings::default().set("MZT-T009=loud").map(|_| ()),
            "unknown severity `loud` (error, warning, info, off)",
        ),
    ];
    for (result, message) in errors {
        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(message.to_owned()),
            "{message}"
        );
    }
}

#[test]
fn settings_apply() {
    let settings = Settings::default()
        .set("MZT-T009=warning")
        .and_then(|s| s.set("MZT-S006=off"))
        .map(|s| s.layer(Layer::Structure));
    let settings = settings.unwrap_or_default();
    assert_eq!(
        settings.config.severity_of(Code::MissingField),
        Some(Severity::Warning),
        "override"
    );
    let bad = "MTD\tmzTab-version\t2.1\n";
    let records = check(bad.as_bytes(), Format::Tsv, &[], &settings);
    assert!(records.iter().all(|r| r.code != "MZT-S006"), "disabled");
    let syntax = Settings::default().layer(Layer::Syntax);
    let only = check(bad.as_bytes(), Format::Tsv, &[], &syntax);
    assert!(only.iter().all(|r| r.layer == "syntax"), "syntax only");
    let semantic = check(
        BASE.as_bytes(),
        Format::Tsv,
        &["[Term]\nid: MS:1\n"],
        &Settings::default(),
    );
    assert!(
        semantic
            .iter()
            .any(|r| r.code == "MZT-X004" && r.layer == "semantic"),
        "vocabulary used"
    );
}

#[test]
fn records_flatten_every_location() {
    let text = "MTD\tmzTab-version\t2.1\nMTD\tinstrument[1]-name\t[MS, MS:1, x, ]\n";
    let records = check(text.as_bytes(), Format::Tsv, &[], &Settings::default());
    let version = records.iter().find(|r| r.code == "MZT-S006");
    assert!(
        version.is_some_and(|r| {
            r.severity == "error"
                && r.line == Some(1)
                && r.column == Some(22)
                && r.cell == Some(3)
                && r.detail.is_some()
                && r.spec
                    .starts_with("https://github.com/HUPO-PSI/mzTab-M/blob/")
        }),
        "{version:?}"
    );
    assert!(
        records
            .iter()
            .any(|r| r.location == "document" && r.line.is_none() && r.severity == "info"),
        "document, info"
    );
    assert!(records.iter().any(|r| r.severity == "warning"), "a warning");
    let json = check(
        br#"{"metadata": {"title": 1, "x": 2}}"#,
        Format::Json,
        &[],
        &Settings::default(),
    );
    assert!(
        json.iter()
            .any(|r| r.location == "/metadata/x" && r.detail.is_none()),
        "pointer, no detail: {json:?}"
    );
}

#[test]
fn conversions() {
    assert_eq!(
        convert(BASE.as_bytes(), Format::Tsv, Format::Json, false).as_deref(),
        Ok(BASE_JSON.trim_end()),
        "to JSON"
    );
    assert_eq!(
        convert(BASE_JSON.as_bytes(), Format::Json, Format::Tsv, false).as_deref(),
        Ok(BASE),
        "to TSV"
    );
    let unknown = BASE.replace("MTD\ttitle\t", "MTD\tnot-a-key\tx\nMTD\ttitle\t");
    let refused = convert(unknown.as_bytes(), Format::Tsv, Format::Json, false);
    assert!(
        matches!(refused, Err(Error::Write(_)))
            && refused.is_err_and(|e| e.to_string().starts_with("cannot write the file:")),
        "refused"
    );
    let lossy = mztab_m_api::write(
        &read(unknown.as_bytes(), Format::Tsv).mztab,
        Format::Json,
        true,
    );
    assert!(
        lossy.is_ok_and(|(text, dropped)| dropped.metadata == 1 && text == BASE_JSON.trim_end()),
        "dropped and written"
    );
}

#[test]
fn facts_render() {
    let parsed = read(BASE.as_bytes(), Format::Tsv);
    let facts = facts(&parsed);
    let get = |key: &str| {
        facts
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(
        get("version"),
        Some(Fact::Text(Some("2.1.0-M".to_owned()))),
        "version"
    );
    assert_eq!(get("sml_rows"), Some(Fact::Rows(Some(2))), "rows");
    let empty = read(b"MTD\ttitle\ta \"b\" \\ c\x01\n", Format::Tsv);
    let doc_facts = mztab_m_api::facts(&empty);
    let rendered: Vec<String> = doc_facts.iter().map(|(k, v)| format!("{k}={v}")).collect();
    for expected in [
        "version=null".to_owned(),
        "title=a \"b\" \\ c\u{1}".to_owned(),
        "sml_rows=null".to_owned(),
        "assays=0".to_owned(),
    ] {
        assert!(rendered.contains(&expected), "{expected:?} in {rendered:?}");
    }
    let json = facts_json(&empty);
    assert!(
        json.contains(r#""version":null,"#)
            && json.contains(r#""title":"a \"b\" \\ c\u0001","#)
            && json.contains(r#""sml_rows":null,"#),
        "facts json escaping: {json}"
    );
}

#[test]
fn formats_are_sniffed() {
    assert_eq!(Format::sniff("a.json", b"MTD"), Format::Json, "by name");
    assert_eq!(
        Format::sniff("a.mztab", b"\n \t{}"),
        Format::Json,
        "by text"
    );
    assert_eq!(Format::sniff("a.mztab", b"MTD\t{"), Format::Tsv, "TSV");
    assert_eq!(Format::sniff("json", b""), Format::Tsv, "no extension");
}

#[test]
fn reader_errors_count_errors_only() {
    // One error (version) and two warnings (spaces).
    let parsed = read(
        b"MTD\tmzTab-version\t2.1\nMTD\ttitle\t x\nMTD\tdescription\t y\n",
        Format::Tsv,
    );
    let facts = facts(&parsed);
    let get = |key: &str| {
        facts
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(get("reader_findings"), Some(Fact::Count(3)), "all three");
    assert_eq!(get("reader_errors"), Some(Fact::Count(1)), "the error");
}

#[test]
fn write_preserves_comments_for_tsv_and_handles_lossy() {
    let commented = format!("COM\theader note\r\n{BASE}");
    let parsed = read(commented.as_bytes(), Format::Tsv);
    let res = mztab_m_api::write(&parsed.mztab, Format::Tsv, false);
    assert!(
        res.as_ref().is_ok_and(|(tsv_out, stripped)| {
            *stripped == mztab_m::Stripped::default()
                && tsv_out.contains("COM\theader note\n")
                && !tsv_out.contains("\r\n")
        }),
        "comments preserved and line endings normalized: {res:?}"
    );

    let with_unknown = format!("MTD\textra_key\tval\n{BASE}");
    let parsed_unknown = read(with_unknown.as_bytes(), Format::Tsv);
    let lossy = mztab_m_api::write(&parsed_unknown.mztab, Format::Tsv, true);
    assert_eq!(
        lossy,
        Ok((
            BASE.to_owned(),
            mztab_m::Stripped {
                metadata: 1,
                columns: 0,
                cells: 0,
            }
        ))
    );
}

#[test]
fn facade_exposes_codes_facts_json_and_obo_validation() {
    assert!(!mztab_m_api::SPEC_COMMIT.is_empty(), "spec commit exposed");
    assert_eq!(mztab_m_api::SPEC_VERSION, "2.1.0-M", "spec version exposed");

    let catalog = mztab_m_api::codes();
    assert_eq!(catalog.len(), Code::ALL.len(), "every code listed");
    assert!(
        matches!(
            (catalog.first(), Code::ALL.first()),
            (Some(first), Some(code))
                if first.code == "MZT-S001"
                    && first.layer == "syntax"
                    && first.requirement == "MUST"
                    && first.summary == code.summary()
                    && first.spec.starts_with("https://github.com/HUPO-PSI/mzTab-M/blob/")
        ),
        "first code metadata: {:?}",
        catalog.first()
    );

    let parsed = read(BASE.as_bytes(), Format::Tsv);
    let json_facts = facts_json(&parsed);
    assert!(
        json_facts.starts_with(r#"{"version":"2.1.0-M","#)
            && json_facts.ends_with(r#""reader_errors":0}"#),
        "facts json object: {json_facts}"
    );

    let json_findings = mztab_m_api::validate_json(
        BASE.as_bytes(),
        Format::Tsv,
        &["[Term]\nid: MS:1\n"],
        &Settings::default(),
    );
    assert!(
        json_findings.starts_with('[')
            && json_findings.contains("MZT-X004")
            && json_findings.ends_with(']'),
        "validate_json output: {json_findings}"
    );

    assert_eq!(mztab_m_api::vocabulary_size(""), 0, "empty vocabulary");
    assert_eq!(
        mztab_m_api::vocabulary_size(
            "[Term]\nid: UO:0000010\nname: second\n\n[Term]\nid: UO:0000028\nname: millisecond\n"
        ),
        2,
        "vocabulary size"
    );
}

#[test]
fn bom_sniff_and_invalid_utf8_json_findings() {
    assert_eq!(
        Format::sniff("-", b"\xEF\xBB\xBF{}"),
        Format::Json,
        "UTF-8 BOM before JSON"
    );
    let bad = read(b"{\"a\":\"\xff\"}", Format::Json);
    let first = bad.findings.iter().next().expect("Utf8 finding");
    assert_eq!(first.code, Code::Utf8, "{:?}", bad.findings);
    assert_eq!(
        first.location,
        mztab_m::Location::text(1, 7),
        "column of first invalid UTF-8 byte"
    );
    let multiline = read(b"{\"a\":\n\"\xff\"}", Format::Json);
    assert_eq!(
        multiline
            .findings
            .iter()
            .next()
            .map(|f| (&f.code, &f.location)),
        Some((&Code::Utf8, &mztab_m::Location::text(2, 2))),
        "multiline UTF-8 column after newline"
    );

    let settings = Settings::default()
        .set_code("MZT-X002", "off")
        .expect("valid set_code");
    assert_eq!(settings.config.severity_of(Code::CvUnused), None, "off");
    assert_eq!(
        Settings::default().set_code("MZT-X002=off", "off"),
        Err(Error::Code("MZT-X002=off".to_owned())),
        "set_code does not split on `=`"
    );

    assert_eq!(
        read_borrowed(BASE.as_bytes(), Format::Tsv),
        read(BASE.as_bytes(), Format::Tsv),
        "read_borrowed matches read for TSV"
    );
    assert_eq!(
        read_borrowed(BASE_JSON.as_bytes(), Format::Json),
        read(BASE_JSON.as_bytes(), Format::Json),
        "read_borrowed matches read for JSON"
    );
}
