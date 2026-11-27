//! The validation API: configuration, layers, and where findings point.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::cv::NoTerms;
use mztab_m::validate::{self, Config};
use mztab_m::{Cell, Code, Location, MzTab, Raw, Severity, SourceLayout, parse};

const BASE: &str = include_str!("golden/full.canonical.mztab");

fn with(code: Code, findings: &mztab_m::Findings) -> Vec<String> {
    findings
        .iter()
        .filter(|f| f.code == code)
        .map(ToString::to_string)
        .collect()
}

#[test]
fn config_overrides_and_disables() {
    let input = "MTD\tmzTab-version\t2.2.0-M\nMTD\ttitle\t t\n";
    let default = validate::all(input, &Config::default(), &NoTerms);
    assert_eq!(
        default
            .iter()
            .find(|f| f.code == Code::UnknownVersion)
            .map(|f| f.severity),
        Some(Severity::Warning),
        "default severity"
    );
    let config = Config::default()
        .with_severity(Code::UnknownVersion, Severity::Error)
        .without(Code::CellWhitespace);
    let configured = validate::all(input, &config, &NoTerms);
    assert_eq!(
        configured
            .iter()
            .find(|f| f.code == Code::UnknownVersion)
            .map(|f| f.severity),
        Some(Severity::Error),
        "overridden"
    );
    assert!(
        with(Code::CellWhitespace, &configured).is_empty(),
        "disabled"
    );
    assert_eq!(
        config.severity_of(Code::CellWhitespace),
        None,
        "disabled severity"
    );
    assert!(
        !with(
            Code::CellWhitespace,
            &validate::syntax(input, &Config::default())
        )
        .is_empty(),
        "syntax layer"
    );
    assert!(
        with(Code::CellWhitespace, &validate::syntax(input, &config)).is_empty(),
        "syntax configured"
    );
}

#[test]
fn all_reports_invalid_values_once() {
    let input = BASE.replace("MTD\tmzTab-version\t2.1.0-M", "MTD\tmzTab-version\t2.1");
    let findings = validate::all(&input, &Config::default(), &NoTerms);
    assert_eq!(
        with(Code::VersionSyntax, &findings).len(),
        1,
        "reader reports it"
    );
    assert!(
        with(Code::InvalidValue, &findings).is_empty(),
        "not reported again"
    );
    let model = parse(&input).mztab;
    let structure = validate::structure(&model, &Config::default());
    assert_eq!(
        with(Code::InvalidValue, &structure).len(),
        1,
        "structure alone reports it"
    );
}

#[test]
fn findings_on_read_models_point_at_text() {
    let input = BASE
        .replace(
            "MTD\tcontact[1]-orcid\t0000-0002-1825-0097",
            "MTD\tcontact[1]-orcid\tnope",
        )
        .replace(
            "MTD\tprotocol[1]-type\t[OBI, OBI:0302884, extraction, ]\n",
            "",
        );
    let findings = validate::all(&input, &Config::default(), &NoTerms);
    assert_eq!(
        with(Code::Orcid, &findings),
        ["error[MZT-T034] line 17, column 22, cell 3: ORCID does not match `0000-0000-0000-000X`"],
        "value cell"
    );
    let protocol = with(Code::MissingField, &findings);
    assert_eq!(protocol.len(), 1, "one missing field");
    assert!(
        protocol
            .iter()
            .all(|f| f.contains("cell 2") && f.contains("`protocol[1]-type`")),
        "element line: {protocol:?}"
    );
}

#[test]
fn findings_on_models_built_in_code_carry_model_paths() {
    let mut model = parse(BASE).mztab;
    model.layout = SourceLayout::default();
    if let Some(row) = model.sme.as_mut().and_then(|t| t.rows.first_mut()) {
        row.rank = Cell::Null;
    }
    model.metadata.title = Some(Cell::Invalid(Raw::new("x")));
    if let Some(table) = &mut model.smf {
        table.rows.clear();
    }
    let findings = validate::structure(&model, &Config::default());
    let at = |code| {
        findings
            .iter()
            .find(|f| f.code == code)
            .map(|f| f.location.to_string())
    };
    assert_eq!(at(Code::NullValue).as_deref(), Some("sme[1][18]"), "cell");
    assert_eq!(
        at(Code::InvalidValue).as_deref(),
        Some("metadata.title"),
        "key"
    );
    assert_eq!(
        at(Code::EmptyTable).as_deref(),
        Some("smf.header"),
        "header"
    );
    let empty = validate::structure(&MzTab::default(), &Config::default());
    assert!(
        empty.iter().all(|f| f.location == Location::Document),
        "missing fields of an empty model"
    );
}

#[test]
fn more_model_paths() {
    let mut model = parse(BASE).mztab;
    model.layout = SourceLayout::default();
    let units = &mut model.metadata.colunit_small_molecule;
    if let Some(unit) = units.get_mut(1) {
        *unit = Cell::Invalid(Raw::new("x"));
    }
    if let Some(table) = &mut model.sml {
        let unknown = mztab_m::Column::Abundance(mztab_m::AbundanceColumn::Assay(
            mztab_m::AssayId::new(9).expect("non-zero"),
        ));
        table.header.columns.push(unknown);
    }
    model.metadata.cv = mztab_m::IndexMap::new();
    let cv = mztab_m::Cv {
        label: Some(Cell::Value(mztab_m::Text::new("MS").expect("valid"))),
        ..mztab_m::Cv::default()
    };
    model
        .metadata
        .cv
        .insert(mztab_m::CvId::new(1).expect("non-zero"), cv);
    let findings = validate::structure(&model, &Config::default());
    let at = |code| {
        findings
            .iter()
            .find(|f| f.code == code)
            .map(|f| f.location.to_string())
    };
    assert_eq!(
        at(Code::InvalidValue).as_deref(),
        Some("metadata.colunit-small_molecule[2]"),
        "nth"
    );
    assert_eq!(
        at(Code::DanglingReference).as_deref(),
        Some("sml.header[19]"),
        "column"
    );
    assert_eq!(
        at(Code::MissingField).as_deref(),
        Some("metadata.cv[1]"),
        "element"
    );
}

#[test]
fn findings_without_a_source_line_fall_back() {
    let mut model = parse(BASE).mztab;
    model.metadata.title = Some(Cell::Invalid(Raw::new("x")));
    model.metadata.uri = mztab_m::IndexMap::new();
    let findings = validate::structure(&model, &Config::default());
    let invalid = findings
        .iter()
        .find(|f| f.code == Code::InvalidValue)
        .map(|f| f.location.clone());
    assert!(
        matches!(invalid, Some(Location::Text { line: 4, .. })),
        "title is still on line 4: {invalid:?}"
    );

    let mut table_only = parse("SFH\tSMF_ID\n").mztab;
    table_only.metadata.title = Some(Cell::Invalid(Raw::new("x")));
    let findings = validate::structure(&table_only, &Config::default());
    let invalid = findings
        .iter()
        .find(|f| f.code == Code::InvalidValue)
        .map(|f| f.location.clone());
    assert_eq!(
        invalid,
        Some(Location::Document),
        "no metadata line to point at"
    );

    // Comments alone (as a JSON-read model keeps them) are no layout to
    // locate by: model paths.
    let mut comment_only = parse("COM\tx\n").mztab;
    comment_only.metadata.title = Some(Cell::Invalid(Raw::new("x")));
    let findings = validate::structure(&comment_only, &Config::default());
    let invalid = findings
        .iter()
        .find(|f| f.code == Code::InvalidValue)
        .map(|f| f.location.to_string());
    assert_eq!(invalid.as_deref(), Some("metadata.title"), "model path");

    let mut added = parse("MTD\tmzTab-ID\tx\n").mztab;
    added.metadata.description = Some(Cell::Invalid(Raw::new("y")));
    let findings = validate::structure(&added, &Config::default());
    let invalid = findings
        .iter()
        .find(|f| f.code == Code::InvalidValue)
        .map(|f| f.location.clone());
    assert_eq!(
        invalid,
        Some(Location::text(1, 1).with_cell(1)),
        "added after reading: first metadata line"
    );
}

/// Any table type can hold any column kind (a model built in code can put
/// an SME confidence column in SML); the validator checks them all.
#[test]
fn every_column_kind_is_checked_in_any_table() {
    use mztab_m::{AbundanceColumn, AssayId, Column, IdConfidenceMeasureId};
    let mut model = parse(BASE).mztab;
    if let Some(table) = &mut model.sml {
        table
            .header
            .columns
            .push(Column::Abundance(AbundanceColumn::Assay(
                AssayId::new(1).expect("non-zero"),
            )));
        table.header.columns.push(Column::IdConfidence(
            IdConfidenceMeasureId::new(9).expect("non-zero"),
        ));
        table.header.columns.push(Column::Unknown(Raw::new("odd")));
    }
    let findings = validate::structure(&model, &Config::default());
    let codes: Vec<Code> = findings.iter().map(|f| f.code).collect();
    assert_eq!(
        codes,
        [
            Code::ColumnOrder,
            Code::OptColumnPosition,
            Code::DanglingReference,
        ],
        "pushed columns trigger order, opt position, and dangling id_confidence_measure[9]: {codes:?}"
    );
}

#[test]
fn version_two_files_skip_profile_errors() {
    let input = BASE
        .replace("MTD\tmzTab-version\t2.1.0-M", "MTD\tmzTab-version\t2.0.0-M")
        .replace("MTD\tmzTab-profile\tM+S+F+E\n", "");
    let without_smf: String = input
        .lines()
        .filter(|l| !l.starts_with("SFH\t") && !l.starts_with("SMF\t"))
        .flat_map(|l| [l, "\n"])
        .collect();
    let findings = validate::all(&without_smf, &Config::default(), &NoTerms);
    let codes: Vec<Code> = findings.iter().map(|f| f.code).collect();
    assert!(
        !codes.contains(&Code::ProhibitedProfile),
        "2.0 has no profiles: {codes:?}"
    );
    assert!(
        !codes.contains(&Code::ProfileInferred),
        "nothing to infer: {codes:?}"
    );
    assert_eq!(
        Config::default().severity_of(Code::MissingField),
        Some(Severity::Error),
        "default"
    );
}

#[test]
fn details_render() {
    use mztab_m::{Detail, Finding, Profile, ProhibitedProfile};
    let render =
        |detail| Finding::new(Code::ProfileInferred, Location::Document, detail).to_string();
    assert!(
        render(Detail::Profile(Profile::MF)).ends_with(": profile `M+F`"),
        "profile"
    );
    assert!(
        render(Detail::Prohibited(ProhibitedProfile::ME)).ends_with(": tables `M+E`"),
        "prohibited"
    );
    assert!(
        render(Detail::Label("x[1]".into())).ends_with(": `x[1]`"),
        "label"
    );
    assert!(
        render(Detail::Key(mztab_m::MtdKey::Title)).ends_with(": `title`"),
        "key"
    );
    assert!(
        render(Detail::Term {
            accession: "MS:1".into(),
            name: Some("x".into()),
            rule: Some("r"),
        })
        .ends_with(": `MS:1` is `x` (rule `r`)"),
        "term"
    );
}

/// Table findings on a read model point into the header and row lines
/// (rendered by the writer, as canonical lines keep no source text).
#[test]
fn table_findings_on_read_models_point_at_text() {
    let lines: Vec<String> = BASE
        .lines()
        .map(|l| {
            let l = if l.starts_with("SMF\t2\t") {
                l.replacen("203.0526", "null", 1)
            } else if l.starts_with("SME\t2\t") {
                l.replacen("181.0709\t1\t181.0707", "181.0709\tnull\t181.0707", 1)
            } else {
                l.to_owned()
            };
            if l.starts_with("SFH\t") {
                format!("{l}\topt_assay[9]_x")
            } else if l.starts_with("SMF\t") {
                format!("{l}\tnull")
            } else {
                l
            }
        })
        .collect();
    let input = lines.join("\n") + "\n";
    let findings = validate::all(&input, &Config::default(), &NoTerms);
    let at = |code| -> Vec<Location> {
        findings
            .iter()
            .filter(|f| f.code == code)
            .map(|f| f.location.clone())
            .collect()
    };
    assert_eq!(
        at(Code::DanglingReference),
        [Location::text(133, 242).with_cell(15)],
        "SMF header column"
    );
    assert_eq!(
        at(Code::NullValue),
        [
            Location::text(135, 27).with_cell(7),
            Location::text(139, 65).with_cell(13),
        ],
        "SMF and SME cells"
    );
}

/// The semantic layer on a model built in code: a CV with no version is
/// not compared, and the empty lookup judges nothing.
#[test]
fn semantic_on_models_built_in_code() {
    use mztab_m::cv::{NoTerms, TermLookup, rules};
    let mut model = MzTab::default();
    let cv = mztab_m::Cv {
        label: Some(Cell::Value(mztab_m::Text::new("MS").expect("valid"))),
        ..mztab_m::Cv::default()
    };
    model
        .metadata
        .cv
        .insert(mztab_m::CvId::new(1).expect("non-zero"), cv);
    let findings = validate::semantic(&model, &Config::default(), &NoTerms);
    let codes: Vec<Code> = findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, [Code::CvUnused], "only the unused CV");
    assert_eq!(NoTerms.term("MS:1"), None, "no term");
    assert!(!NoTerms.is_descendant("MS:1", "MS:2"), "no ancestors");
    let names: Vec<String> = rules().iter().map(|r| r.target.to_string()).collect();
    for (i, name) in names.iter().enumerate() {
        assert!(
            !name.is_empty() && !names.iter().skip(i + 1).any(|n| n == name),
            "{name}: distinct field names"
        );
    }
}

#[test]
fn synthetic_mtd_table_lines_are_ignored_in_location_index() {
    use mztab_m::{LineContent, LineEnding, Location, Section, SourceLine};
    let mut mztab = MzTab::default();
    mztab.layout.lines.push(SourceLine {
        content: LineContent::Header(Section::Mtd),
        ending: LineEnding::Lf,
        verbatim: None,
    });
    mztab.layout.lines.push(SourceLine {
        content: LineContent::Row(Section::Mtd),
        ending: LineEnding::Lf,
        verbatim: None,
    });
    let findings = validate::structure(&mztab, &Config::default());
    assert!(
        findings.iter().all(|f| f.location == Location::Document),
        "non-table layout sections do not map to table lines"
    );
}

#[test]
fn theoretical_neutral_mass_and_missing_ambiguity_col() {
    // Mixed invalid + scientific item in SML theoretical_neutral_mass emits one InvalidValue.
    let mixed = BASE.replace("180.0634|null", "bad|1e5");
    let model = parse(&mixed).mztab;
    let findings = validate::structure(&model, &Config::default());
    let t028_on_mass = findings
        .iter()
        .filter(|f| f.code == Code::InvalidValue && f.location.to_string().contains("cell 10"))
        .count();
    assert_eq!(t028_on_mass, 1, "one InvalidValue per cell: {findings:?}");

    // Missing SME_ID_REF_ambiguity_code column does not emit AmbiguityCode past the header.
    let no_amb = BASE
        .replace("\tSME_ID_REF_ambiguity_code\t", "\t")
        .replace("SMF\t1\t1|2\t1\t", "SMF\t1\t1|2\t")
        .replace("SMF\t2\tnull\tnull\t", "SMF\t2\tnull\t");
    let model = parse(&no_amb).mztab;
    let findings = validate::structure(&model, &Config::default());
    assert!(
        !findings.iter().any(|f| f.code == Code::AmbiguityCode),
        "missing ambiguity column should not emit AmbiguityCode: {findings:?}"
    );
}

#[test]
fn validate_parsed_matches_layer_functions() {
    use mztab_m::Layer;
    use mztab_m::cv::NoTerms;

    let broken = "MTD\tmzTab-version\t2.1\nMTD\tinstrument[1]-name\t[MS, MS:1, x, ]\n";
    let doc = parse(broken);
    let config = Config::default().with_severity(Code::MissingField, Severity::Warning);

    assert_eq!(
        validate::parsed(&doc, Layer::Syntax, &config, &NoTerms),
        validate::syntax(broken, &config),
        "syntax layer"
    );
    let structure_parsed = validate::parsed(&doc, Layer::Structure, &config, &NoTerms);
    let mut expected_structure = validate::syntax(broken, &config).into_vec();
    expected_structure.extend(
        validate::structure(&doc.mztab, &config)
            .into_vec()
            .into_iter()
            .filter(|f| f.code != Code::InvalidValue),
    );
    assert_eq!(
        structure_parsed.into_vec(),
        expected_structure,
        "structure layer"
    );
    assert_eq!(
        validate::parsed(&doc, Layer::Semantic, &config, &NoTerms),
        validate::all(broken, &config, &NoTerms),
        "semantic layer matches all"
    );
}
