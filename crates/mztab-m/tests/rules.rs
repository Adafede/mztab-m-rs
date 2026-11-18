//! The negative suite: for every finding code, one small edit of a valid
//! document that triggers it. The valid document itself triggers none, and
//! a meta-test checks that no code is left without a case.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test helpers edit a known document; a miss is a broken test"
)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[path = "support/terms.rs"]
mod terms;

use mztab_m::validate::{self, Config};
use mztab_m::{Code, Section, parse_bytes};
use terms::Mini;

/// A valid mzTab-M 2.1 document (M+S+F+E) using every key and column kind.
const BASE: &str = include_str!("golden/full.canonical.mztab");

/// A document under edit, line by line.
#[derive(Clone)]
struct Doc(Vec<String>);

impl Doc {
    fn base() -> Self {
        Self(BASE.lines().map(String::from).collect())
    }

    fn bytes(&self) -> Vec<u8> {
        let mut text = self.0.join("\n");
        text.push('\n');
        text.into_bytes()
    }

    fn key_line(&self, key: &str) -> Option<usize> {
        let prefix = format!("MTD\t{key}\t");
        self.0.iter().position(|line| line.starts_with(&prefix))
    }

    /// Replaces the value of the first `MTD` line with `key`.
    fn set(mut self, key: &str, value: &str) -> Self {
        let at = self.key_line(key).expect("the key is in the base document");
        self.0[at] = format!("MTD\t{key}\t{value}");
        self
    }

    /// Removes every `MTD` line whose key starts with `key`.
    fn drop_keys(mut self, key: &str) -> Self {
        let prefix = format!("MTD\t{key}");
        self.0.retain(|line| !line.starts_with(&prefix));
        self
    }

    /// Removes the `MTD` line with exactly this key.
    fn drop_key(mut self, key: &str) -> Self {
        let prefix = format!("MTD\t{key}\t");
        self.0.retain(|line| !line.starts_with(&prefix));
        self
    }

    /// Inserts metadata lines after the last `MTD` line.
    fn add_mtd(mut self, lines: &[&str]) -> Self {
        let last = self
            .0
            .iter()
            .rposition(|line| line.starts_with("MTD\t"))
            .unwrap_or(0);
        for (i, line) in lines.iter().enumerate() {
            self.0.insert(last + 1 + i, (*line).to_owned());
        }
        self
    }

    fn insert(mut self, before_prefix: &str, line: &str) -> Self {
        let at = self
            .0
            .iter()
            .position(|l| l.starts_with(before_prefix))
            .unwrap_or(0);
        self.0.insert(at, line.to_owned());
        self
    }

    fn push(mut self, line: &str) -> Self {
        self.0.push(line.to_owned());
        self
    }

    fn map(mut self, f: impl Fn(&str) -> String) -> Self {
        self.0 = self.0.iter().map(|line| f(line)).collect();
        self
    }

    fn prefixes(section: Section) -> (&'static str, &'static str) {
        let row = section.row_prefix();
        let header = section.header_prefix().unwrap_or_default();
        (header, row)
    }

    fn column(&self, section: Section, label: &str) -> usize {
        let (header, _) = Self::prefixes(section);
        let line = self
            .0
            .iter()
            .find(|l| l.starts_with(header))
            .expect("header");
        line.split('\t')
            .position(|c| c == label)
            .expect("the column is in the base document")
    }

    /// Sets a cell of the `row`-th (1-based) row of a table.
    fn cell(mut self, section: Section, row: usize, label: &str, value: &str) -> Self {
        let column = self.column(section, label);
        let (_, prefix) = Self::prefixes(section);
        let at = self
            .0
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with(&format!("{prefix}\t")))
            .nth(row - 1)
            .map(|(i, _)| i)
            .expect("row");
        let mut cells: Vec<String> = self.0[at].split('\t').map(String::from).collect();
        value.clone_into(&mut cells[column]);
        self.0[at] = cells.join("\t");
        self
    }

    /// Appends a column to a table header and a value to each of its rows.
    fn add_column(self, section: Section, label: &str, value: &str) -> Self {
        let (header, row) = Self::prefixes(section);
        self.map(|line| {
            if line.starts_with(&format!("{header}\t")) {
                format!("{line}\t{label}")
            } else if line.starts_with(&format!("{row}\t")) {
                format!("{line}\t{value}")
            } else {
                line.to_owned()
            }
        })
    }

    /// Removes a column from a table header and rows.
    fn drop_column(self, section: Section, label: &str) -> Self {
        let column = self.column(section, label);
        let (header, row) = Self::prefixes(section);
        self.map(|line| {
            if line.starts_with(&format!("{header}\t")) || line.starts_with(&format!("{row}\t")) {
                let mut cells: Vec<&str> = line.split('\t').collect();
                cells.remove(column);
                cells.join("\t")
            } else {
                line.to_owned()
            }
        })
    }

    /// Removes a table's rows (`header` too if `all`).
    fn drop_table(mut self, section: Section, all: bool) -> Self {
        let (header, row) = Self::prefixes(section);
        self.0.retain(|l| {
            !(l.starts_with(&format!("{row}\t")) || all && l.starts_with(&format!("{header}\t")))
        });
        self
    }
}

/// Every code the reader and the structure and semantic validators (with
/// the test vocabulary) report on `input`.
fn codes(input: &[u8]) -> Vec<Code> {
    let parsed = parse_bytes(input);
    let structure = validate::structure(&parsed.mztab, &Config::default());
    let semantic = validate::semantic(&parsed.mztab, &Config::default(), &Mini);
    parsed
        .findings
        .iter()
        .chain(&structure)
        .chain(&semantic)
        .map(|f| f.code)
        .collect()
}

/// One case per code: the edit that triggers it.
#[expect(clippy::too_many_lines, reason = "one line per code")]
fn cases() -> Vec<(Code, Vec<u8>)> {
    let d = Doc::base;
    let (sml, smf, sme) = (Section::Sml, Section::Smf, Section::Sme);
    let mut bom = b"\xEF\xBB\xBF".to_vec();
    bom.extend(d().bytes());
    let mut utf8 = d().bytes();
    utf8.extend(b"COM\t\xFF\n");
    vec![
        (Code::LinePrefix, d().push("XYZ\tx").bytes()),
        (
            Code::ParamSyntax,
            d().set("quantification_method", "[x]").bytes(),
        ),
        (
            Code::NumberSyntax,
            d().cell(smf, 1, "exp_mass_to_charge", "abc").bytes(),
        ),
        (
            Code::NumberForm,
            d().cell(smf, 1, "exp_mass_to_charge", "1e5").bytes(),
        ),
        (
            Code::TextCell,
            d().cell(sml, 1, "opt_global_note", "").bytes(),
        ),
        (Code::VersionSyntax, d().set("mzTab-version", "2.1").bytes()),
        (
            Code::ValueSyntax,
            d().set("ms_run[1]-instrument_ref", "instr[1]").bytes(),
        ),
        (Code::Utf8, utf8),
        (Code::Bom, bom),
        (
            Code::CellCount,
            d().map(|l| {
                if l.starts_with("SMF\t1\t") {
                    format!("{l}\tx")
                } else {
                    l.to_owned()
                }
            })
            .bytes(),
        ),
        (Code::MtdCells, d().set("description", "d\textra").bytes()),
        (
            Code::TrailingEmptyCells,
            d().set("description", "d\t\t").bytes(),
        ),
        (
            Code::CellWhitespace,
            d().set("title", " Everything").bytes(),
        ),
        (Code::ProfileValue, d().set("mzTab-profile", "M+E").bytes()),
        (
            Code::UnknownMtdKey,
            d().add_mtd(&["MTD\tnot-a-key\tv"]).bytes(),
        ),
        (
            Code::DuplicateMtdKey,
            d().add_mtd(&["MTD\ttitle\tagain"]).bytes(),
        ),
        (
            Code::UnknownColumn,
            d().add_column(sml, "bogus", "x").bytes(),
        ),
        (
            Code::DuplicateColumn,
            d().add_column(sml, "SML_ID", "9").bytes(),
        ),
        (
            Code::SectionOrder,
            d().push("MTD\tcustom[2]\t[,,x,]").bytes(),
        ),
        (Code::RowBeforeHeader, d().insert("SMH\t", "SML\t9").bytes()),
        (Code::DuplicateHeader, d().push("SEH\tSME_ID").bytes()),
        (Code::MissingField, d().drop_keys("mzTab-ID").bytes()),
        (
            Code::UnknownVersion,
            d().set("mzTab-version", "2.2.0-M").bytes(),
        ),
        (
            Code::ProfileInferred,
            d().drop_keys("mzTab-profile").bytes(),
        ),
        (
            Code::ProfileMismatch,
            d().set("mzTab-profile", "M+S").bytes(),
        ),
        (Code::ProhibitedProfile, d().drop_table(smf, true).bytes()),
        (
            Code::DiscouragedProfile,
            d().drop_keys("mzTab-profile").drop_table(sme, true).bytes(),
        ),
        (Code::EmptyTable, d().drop_table(sme, false).bytes()),
        (Code::ProfileField, d().drop_table(sme, true).bytes()),
        (
            Code::DanglingReference,
            d().set("assay[1]-sample_ref", "sample[9]").bytes(),
        ),
        (
            Code::DuplicateRowId,
            d().cell(sml, 2, "SML_ID", "1").bytes(),
        ),
        (
            Code::IndexGap,
            d().drop_keys("custom[1]")
                .add_mtd(&["MTD\tcustom[2]\t[,,x,]"])
                .bytes(),
        ),
        (
            Code::StudyVariableGrouping,
            d().add_mtd(&[
                "MTD\tstudy_variable_group[2]\t[,,g,]",
                "MTD\tstudy_variable_group[2]-study_variable_refs\tstudy_variable[1]",
            ])
            .bytes(),
        ),
        (
            Code::NoStudyVariableGroup,
            d().drop_keys("study_variable_group").bytes(),
        ),
        (
            Code::CrossReference,
            d().cell(sml, 1, "SMF_ID_REFS", "null").bytes(),
        ),
        (
            Code::AmbiguityCode,
            d().cell(smf, 1, "SME_ID_REF_ambiguity_code", "null")
                .bytes(),
        ),
        (Code::MissingColumn, d().drop_column(sml, "smiles").bytes()),
        (
            Code::ColumnOrder,
            d().drop_column(sml, "smiles")
                .add_column(sml, "smiles", "null")
                .bytes(),
        ),
        (
            Code::OptColumnPosition,
            d().add_column(sme, "rank", "1")
                .drop_column(sme, "rank")
                .add_column(sme, "rank", "1")
                .bytes(),
        ),
        (Code::NullValue, d().cell(sme, 1, "rank", "null").bytes()),
        (
            Code::InvalidValue,
            d().cell(smf, 1, "exp_mass_to_charge", "abc").bytes(),
        ),
        (
            Code::ReliabilityMissing,
            d().cell(sml, 1, "reliability", "null").bytes(),
        ),
        (
            Code::ReliabilityValue,
            d().drop_keys("small_molecule-identification_reliability")
                .cell(sml, 1, "reliability", "7")
                .bytes(),
        ),
        (
            Code::HashMethod,
            d().drop_keys("ms_run[1]-hash_method").bytes(),
        ),
        (
            Code::ColunitColumn,
            d().set("colunit-small_molecule", "nope=[UO, UO:0000221, dalton, ]")
                .bytes(),
        ),
        (
            Code::AdductIon,
            d().cell(smf, 1, "adduct_ion", "M+H").bytes(),
        ),
        (Code::Orcid, d().set("contact[1]-orcid", "123").bytes()),
        (
            Code::StudyVariableDatatype,
            d().set("study_variable_group[1]-datatype", "xsd:integer")
                .bytes(),
        ),
        (
            Code::CvUndeclared,
            d().set("quantification_method", "[NEW, NEW:1, new method, ]")
                .bytes(),
        ),
        (
            Code::CvUnused,
            d().add_mtd(&[
                "MTD\tcv[13]-label\tFOO",
                "MTD\tcv[13]-full_name\tFoo",
                "MTD\tcv[13]-version\t1",
                "MTD\tcv[13]-uri\thttps://example.org/foo",
            ])
            .bytes(),
        ),
        (
            Code::AccessionPrefix,
            d().set(
                "quantification_method",
                "[MS, 1001834, LC-MS label-free quantitation analysis, ]",
            )
            .bytes(),
        ),
        (
            Code::UnknownTerm,
            d().set("quantification_method", "[MS, MS:9999999, nothing, ]")
                .bytes(),
        ),
        (
            Code::TermName,
            d().set("quantification_method", "[MS, MS:1001834, label free, ]")
                .bytes(),
        ),
        (
            Code::ObsoleteTerm,
            d().set("quantification_method", "[MS, MS:1009999, old term, ]")
                .bytes(),
        ),
        (
            Code::CvRule,
            d().set("quantification_method", "[MS, MS:1002883, median, ]")
                .bytes(),
        ),
        (
            Code::CvRuleRepeat,
            d().add_mtd(&["MTD\tms_run[1]-scan_polarity[2]\t[MS, MS:1000130, positive scan, ]"])
                .bytes(),
        ),
        (
            Code::CvRuleXor,
            d().cell(
                sme,
                1,
                "identification_method",
                "[MS, MS:1009998, search software, ]",
            )
            .bytes(),
        ),
        (Code::CvVersion, d().set("cv[1]-version", "4.0.0").bytes()),
    ]
}

/// More edits, one per branch of each rule.
#[expect(clippy::too_many_lines, reason = "one line per branch")]
fn more_cases() -> Vec<(Code, Vec<u8>)> {
    let d = Doc::base;
    let (sml, smf, sme) = (Section::Sml, Section::Smf, Section::Sme);
    let v20 = || {
        d().set("mzTab-version", "2.0.0-M")
            .drop_keys("mzTab-profile")
    };
    let mut cases = Vec::new();
    for key in [
        "assay[1]",
        "assay[1]-ms_run_ref",
        "ms_run[1]-location",
        "study_variable[1]",
        "cv[1]-uri",
        "database[1]",
        "database[1]-version",
        "protocol[1]-name",
        "protocol[1]-type",
        "study_variable_group[1]",
        "quantification_method",
        "small_molecule-quantification_unit",
        "small_molecule_feature-quantification_unit",
        "id_confidence_measure[1]",
    ] {
        cases.push((Code::MissingField, d().drop_key(key).bytes()));
    }
    for prefix in [
        "assay[",
        "ms_run[",
        "study_variable[",
        "cv[",
        "software[",
        "database[",
    ] {
        cases.push((Code::MissingField, d().drop_keys(prefix).bytes()));
    }
    for key in [
        "ms_run[1]-scan_polarity[1]",
        "study_variable[1]-assay_refs",
        "study_variable[1]-description",
    ] {
        cases.push((Code::MissingField, v20().drop_key(key).bytes()));
    }
    cases.push((Code::ProfileInferred, v20().bytes()));
    cases.push((
        Code::ProfileField,
        d().drop_table(sml, true).drop_keys("mzTab-profile").bytes(),
    ));
    for (key, value) in [
        ("ms_run[1]-instrument_ref", "instrument[9]"),
        ("assay[1]-ms_run_ref", "ms_run[9]"),
        ("assay[1]-protocol_refs", "protocol[9]"),
        ("study_variable[1]-assay_refs", "assay[9]"),
        ("study_variable[1]-ms_run_refs", "ms_run[9]"),
        (
            "study_variable_group[1]-study_variable_refs",
            "study_variable[9]",
        ),
    ] {
        cases.push((Code::DanglingReference, d().set(key, value).bytes()));
    }
    for (section, label) in [
        (sml, "abundance_assay[9]"),
        (sml, "abundance_study_variable[9]"),
        (sml, "abundance_variation_study_variable[9]"),
        (sme, "id_confidence_measure[9]"),
        (sml, "opt_assay[9]_x"),
        (sml, "opt_study_variable[9]_x"),
        (sml, "opt_ms_run[9]_x"),
    ] {
        cases.push((
            Code::DanglingReference,
            d().add_column(section, label, "null").bytes(),
        ));
    }
    cases.push((
        Code::DanglingReference,
        d().cell(sml, 1, "SMF_ID_REFS", "9").bytes(),
    ));
    cases.push((
        Code::DanglingReference,
        d().cell(smf, 2, "SME_ID_REFS", "9").bytes(),
    ));
    cases.push((
        Code::DanglingReference,
        d().cell(sme, 1, "spectra_ref", "ms_run[9]:x").bytes(),
    ));
    let ms = d()
        .drop_table(smf, true)
        .drop_table(sme, true)
        .set("mzTab-profile", "M+S");
    cases.push((Code::CrossReference, ms.bytes()));
    let mf = d()
        .drop_table(sml, true)
        .drop_table(sme, true)
        .set("mzTab-profile", "M+F");
    cases.push((Code::CrossReference, mf.bytes()));
    cases.push((
        Code::CrossReference,
        d().cell(smf, 1, "SME_ID_REFS", "1").bytes(),
    ));
    cases.push((
        Code::EmptyTable,
        d().drop_keys("mzTab-profile")
            .drop_table(sme, false)
            .bytes(),
    ));
    cases.push((
        Code::MissingColumn,
        d().drop_column(smf, "abundance_assay[1]").bytes(),
    ));
    cases.push((
        Code::MissingColumn,
        d().drop_column(sme, "id_confidence_measure[1]").bytes(),
    ));
    cases.push((Code::MissingColumn, d().drop_column(sme, "rank").bytes()));
    cases.push((
        Code::MissingColumn,
        d().drop_column(sml, "reliability").bytes(),
    ));
    let late = d().drop_column(sme, "id_confidence_measure[1]").add_column(
        sme,
        "id_confidence_measure[1]",
        "0.9",
    );
    cases.push((Code::ColumnOrder, late.bytes()));
    for (section, label, value) in [
        (sml, "SMF_ID_REFS", "x"),
        (sml, "database_identifier", "a||b"),
        (sml, "theoretical_neutral_mass", "1e5"),
        (sml, "theoretical_neutral_mass", "x"),
        (sml, "best_id_confidence_measure", "x"),
        (sml, "best_id_confidence_value", "1e5"),
        (sml, "abundance_assay[1]", "x"),
        (sml, "abundance_assay[1]", "1e5"),
        (smf, "SME_ID_REF_ambiguity_code", "9"),
        (smf, "isotopomer", "x"),
        (smf, "charge", "x"),
        (smf, "opt_assay[1]_area", ""),
        (sme, "SME_ID", "x"),
        (sme, "charge", "x"),
        (sme, "rank", "x"),
        (sme, "spectra_ref", "x"),
        (sme, "derivatized_form", "x"),
        (sme, "exp_mass_to_charge", "x"),
        (sme, "evidence_input_id", ""),
    ] {
        cases.push((
            Code::InvalidValue,
            d().cell(section, 1, label, value).bytes(),
        ));
    }
    for (key, value) in [
        ("custom[1]", "x"),
        ("publication[1]", "x"),
        ("colunit-small_molecule", "x"),
        ("title", ""),
    ] {
        cases.push((Code::InvalidValue, d().set(key, value).bytes()));
    }
    cases.push((
        Code::AdductIon,
        d().cell(sml, 1, "adduct_ions", "M+H").bytes(),
    ));
    cases.push((
        Code::AdductIon,
        d().cell(sme, 1, "adduct_ion", "M+H").bytes(),
    ));
    cases.push((
        Code::CvUndeclared,
        d().add_column(sml, "opt_global_cv_NEW:1_flag", "null")
            .bytes(),
    ));
    cases.push((
        Code::UnknownTerm,
        d().add_column(sml, "opt_global_cv_MS:9999999_flag", "null")
            .bytes(),
    ));
    for (datatype, value) in [("xsd:boolean", "yes"), ("Parameter", "Control")] {
        let typed = d()
            .set("study_variable_group[1]-datatype", datatype)
            .set("study_variable[1]", value);
        cases.push((Code::StudyVariableDatatype, typed.bytes()));
    }
    cases.push((
        Code::NullValue,
        d().cell(sml, 1, "best_id_confidence_value", "null").bytes(),
    ));
    for value in ["2020-01-01Tnoon", "today\u{54}12:00:00"] {
        let typed = d()
            .set("study_variable_group[1]-datatype", "xsd:dateTime")
            .set("study_variable[1]", value);
        cases.push((Code::StudyVariableDatatype, typed.bytes()));
    }
    cases
}

/// Edits that are valid for one code: each must not report it.
fn clean_cases() -> Vec<(Code, Vec<u8>)> {
    let d = Doc::base;
    let (sml, smf, sme) = (Section::Sml, Section::Smf, Section::Sme);
    vec![
        // The default scheme accepts levels 1-4 (the base has 2 and 4).
        (
            Code::ReliabilityValue,
            d().drop_keys("small_molecule-identification_reliability")
                .bytes(),
        ),
        (
            Code::StudyVariableDatatype,
            d().set("study_variable_group[1]-datatype", "xsd:dateTime")
                .set("study_variable[1]", "2020-01-01T12:00:00")
                .bytes(),
        ),
        // A column missing from the header has no null cells.
        (Code::NullValue, d().drop_column(sme, "rank").bytes()),
        (
            Code::ReliabilityMissing,
            d().drop_column(sml, "reliability").bytes(),
        ),
        // Without SMF, SME rows need no reference.
        (Code::CrossReference, d().drop_table(smf, true).bytes()),
        // User parameters are allowed in every field; a vocabulary the
        // lookup does not hold cannot be judged.
        (
            Code::CvRule,
            d().set("quantification_method", "[,, my method, ]").bytes(),
        ),
        (
            Code::CvRule,
            d().set("quantification_method", "[UO, UO:0000001, x, ]")
                .bytes(),
        ),
        // Two different terms may fill a non-repeatable field's list.
        (
            Code::CvRuleRepeat,
            d().add_mtd(&["MTD\tms_run[1]-scan_polarity[2]\t[MS, MS:1000129, negative scan, ]"])
                .bytes(),
        ),
    ]
}

#[test]
fn more_cases_trigger_their_codes() {
    for (i, (code, input)) in more_cases().into_iter().enumerate() {
        let found = codes(&input);
        assert!(
            found.contains(&code),
            "case {i}: {code} not reported; found {found:?}"
        );
    }
}

#[test]
fn clean_cases_do_not_trigger_their_codes() {
    for (i, (code, input)) in clean_cases().into_iter().enumerate() {
        let found = codes(&input);
        assert!(
            !found.contains(&code),
            "case {i}: {code} reported; found {found:?}"
        );
    }
}

#[test]
fn the_base_document_triggers_nothing() {
    assert_eq!(codes(BASE.as_bytes()), [], "valid base");
}

#[test]
fn every_case_triggers_its_code() {
    for (code, input) in cases() {
        let found = codes(&input);
        assert!(
            found.contains(&code),
            "{code}: not reported; found {found:?}"
        );
    }
}

#[test]
fn every_code_has_a_case() {
    let covered: Vec<Code> = cases().into_iter().map(|(code, _)| code).collect();
    for code in Code::ALL {
        assert!(
            covered.contains(code),
            "{code} ({}) has no negative case",
            code.summary()
        );
    }
}
