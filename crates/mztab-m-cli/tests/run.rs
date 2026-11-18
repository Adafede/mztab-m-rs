//! Every command, option and exit code, through an in-memory [`Io`].

extern crate alloc;

use alloc::collections::BTreeMap;

use mztab_m_cli::{FAILED, FINDINGS, Io, OK, USAGE, run};

const BASE: &str = include_str!("../../mztab-m/tests/golden/full.canonical.mztab");
const BASE_JSON: &str = include_str!("../../mztab-m/tests/golden/full.json");
const MS: &str = "data-version: 4.1.0\n\n[Term]\nid: MS:1001834\nname: LC-MS label-free quantitation analysis\nis_a: MS:1001833\n\n[Term]\nid: MS:1001833\nname: quantitation analysis summary\n";

#[derive(Default)]
struct Fake {
    files: BTreeMap<String, String>,
    written: BTreeMap<String, String>,
    out: String,
    err: String,
    readonly: bool,
}

impl Io for Fake {
    fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        self.files
            .get(path)
            .map(|t| t.clone().into_bytes())
            .ok_or_else(|| "no such file".to_owned())
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        if self.readonly {
            return Err("read-only".to_owned());
        }
        if path == "-" {
            self.out.push_str(text);
        } else {
            self.written.insert(path.to_owned(), text.to_owned());
        }
        Ok(())
    }

    fn out(&mut self, text: &str) {
        self.out.push_str(text);
    }

    fn err(&mut self, text: &str) {
        self.err.push_str(text);
    }
}

fn fake() -> Fake {
    let mut io = Fake::default();
    io.files.insert("base.mztab".into(), BASE.into());
    io.files.insert("base.json".into(), BASE_JSON.into());
    io.files.insert("noname".into(), BASE_JSON.into());
    io.files
        .insert("bad.mztab".into(), BASE.replace("\t2.1.0-M\n", "\t2.1\n"));
    io.files.insert(
        "ugly.mztab".into(),
        BASE.replace("MTD\ttitle\tEverything", "MTD\ttitle\t Everything"),
    );
    io.files.insert("ms.obo".into(), MS.into());
    // A structure finding (no mzTab-ID) and a semantic one (CV `NEW`).
    io.files.insert(
        "layers.mztab".into(),
        BASE.replace("MTD\tmzTab-ID\tFULL-1\n", "").replace(
            "MTD\ttitle\t",
            "MTD\tcustom[9]\t[NEW, NEW:1, x, ]\nMTD\ttitle\t",
        ),
    );
    io.files
        .insert("commented.mztab".into(), format!("COM\tnote\n{BASE}"));
    io.files.insert(
        "unknown.mztab".into(),
        BASE.replace(
            "MTD\ttitle\t",
            "MTD\tinstrument[1]-analyzer\tx\nMTD\ttitle\t",
        ),
    );
    io
}

fn exec(io: &mut Fake, args: &[&str]) -> u8 {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    run(&args, io)
}

#[test]
fn help_and_version() {
    for args in [&[][..], &["--help"], &["-h"], &["help"]] {
        let mut io = fake();
        assert_eq!(exec(&mut io, args), OK, "{args:?}");
        assert!(io.out.contains("Usage:"), "help text");
    }
    for flag in ["--version", "-V"] {
        let mut io = fake();
        assert_eq!(exec(&mut io, &[flag]), OK, "{flag}");
        assert!(io.out.starts_with("mztab-m 0."), "{}", io.out);
    }
}

#[test]
fn validate_reports_and_exits_by_severity() {
    let mut io = fake();
    assert_eq!(exec(&mut io, &["validate", "base.mztab"]), OK, "valid base");
    assert_eq!(io.err, "0 errors, 0 warnings, 0 infos\n", "summary");

    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["validate", "bad.mztab"]),
        FINDINGS,
        "bad version"
    );
    assert!(io.out.contains("error[MZT-S006]"), "{}", io.out);
    assert!(io.err.starts_with("1 errors, "), "counted: {}", io.err);

    let mut io = fake();
    let args = ["validate", "--fail-on", "error", "bad.mztab"];
    assert_eq!(exec(&mut io, &args), FINDINGS, "explicit error");

    let mut io = fake();
    let args = ["validate", "--fail-on", "never", "bad.mztab"];
    assert_eq!(exec(&mut io, &args), OK, "never fails");

    let mut io = fake();
    let args = ["validate", "--severity", "MZT-S006=warning", "bad.mztab"];
    assert_eq!(exec(&mut io, &args), OK, "downgraded");
    assert!(io.out.contains("warning[MZT-S006]"), "{}", io.out);

    let mut io = fake();
    let args = [
        "validate",
        "--severity",
        "MZT-S006=off",
        "--fail-on",
        "info",
        "bad.mztab",
    ];
    assert_eq!(exec(&mut io, &args), OK, "disabled");
    assert!(!io.out.contains("MZT-S006"), "{}", io.out);

    let mut io = fake();
    let args = [
        "validate",
        "--severity",
        "MZT-S013=info",
        "--fail-on",
        "info",
        "ugly.mztab",
    ];
    assert_eq!(exec(&mut io, &args), FINDINGS, "info fails at info");
    let mut io = fake();
    let args = ["validate", "--fail-on", "warning", "ugly.mztab"];
    assert_eq!(exec(&mut io, &args), FINDINGS, "warning fails at warning");
}

#[test]
fn validate_layers_formats_and_vocabularies() {
    let mut io = fake();
    let args = [
        "validate",
        "--format",
        "json",
        "--layer",
        "syntax",
        "bad.mztab",
    ];
    assert_eq!(exec(&mut io, &args), FINDINGS, "syntax only");
    assert!(io.out.starts_with("[{\"code\":\"MZT-S006\""), "{}", io.out);
    assert!(
        !io.out.contains("MZT-T"),
        "no structure findings: {}",
        io.out
    );
    assert!(io.err.is_empty(), "no summary with JSON");

    let mut io = fake();
    let args = [
        "validate",
        "--layer",
        "structure",
        "--format",
        "human",
        "layers.mztab",
    ];
    assert_eq!(exec(&mut io, &args), FINDINGS, "structure, human");
    assert!(io.out.contains("MZT-T"), "structure ran: {}", io.out);
    assert!(!io.out.contains("MZT-X"), "semantic did not: {}", io.out);
    let mut io = fake();
    exec(&mut io, &["validate", "layers.mztab"]);
    assert!(
        io.out.contains("MZT-X001"),
        "semantic by default: {}",
        io.out
    );
    assert!(
        !io.out.contains("MZT-T028"),
        "an untyped value is reported once"
    );

    // With a vocabulary, the semantic layer checks MS terms.
    let mut io = fake();
    let args = [
        "validate",
        "--cv",
        "ms.obo",
        "--fail-on",
        "never",
        "base.mztab",
    ];
    assert_eq!(exec(&mut io, &args), OK, "with a vocabulary");
    assert!(io.out.contains("MZT-X004"), "unknown MS terms: {}", io.out);

    let mut io = fake();
    let args = ["validate", "--layer", "semantic", "base.json"];
    assert_eq!(exec(&mut io, &args), OK, "JSON input by name");
    let mut io = fake();
    assert_eq!(exec(&mut io, &["validate", "noname"]), OK, "JSON by `{{`");
    let mut io = fake();
    let args = ["validate", "--input", "tsv", "noname"];
    assert_eq!(exec(&mut io, &args), FINDINGS, "forced TSV");
}

#[test]
fn fmt_and_convert() {
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["fmt", "--check", "base.mztab"]),
        OK,
        "canonical"
    );
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["fmt", "--check", "commented.mztab"]),
        FINDINGS,
        "not canonical"
    );
    assert_eq!(
        io.err, "commented.mztab: not in canonical form\n",
        "message"
    );
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["fmt", "-o", "out.mztab", "commented.mztab"]),
        OK,
        "formatted"
    );
    assert!(
        io.written.get("out.mztab").is_some_and(|t| t == BASE),
        "comments go; the canonical text stays"
    );
    let mut io = fake();
    assert_eq!(exec(&mut io, &["fmt", "base.json"]), OK, "JSON stays JSON");
    assert_eq!(io.out, BASE_JSON.trim_end(), "canonical JSON");

    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["convert", "--to", "json", "base.mztab"]),
        OK,
        "to JSON"
    );
    assert_eq!(io.out, BASE_JSON.trim_end(), "golden JSON");
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["convert", "--to", "tsv", "-", "--input", "json"]),
        FAILED,
        "stdin missing"
    );
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["convert", "--to", "tsv", "base.json"]),
        OK,
        "to TSV"
    );
    assert_eq!(io.out, BASE, "golden TSV");
    let mut io = fake();
    let args = ["convert", "--to", "json", "unknown.mztab"];
    assert_eq!(exec(&mut io, &args), FAILED, "unwritable");
    assert!(
        io.err.contains("metadata key has no JSON form"),
        "{}",
        io.err
    );
    let mut io = fake();
    io.readonly = true;
    let args = ["convert", "--to", "json", "-o", "x.json", "base.mztab"];
    assert_eq!(exec(&mut io, &args), FAILED, "cannot write");
    assert!(io.err.contains("cannot write `x.json`"), "{}", io.err);
}

#[test]
fn inspect_summarises() {
    let mut io = fake();
    assert_eq!(exec(&mut io, &["inspect", "base.mztab"]), OK, "human");
    assert!(io.out.contains("version: 2.1.0-M\n"), "{}", io.out);
    assert!(io.out.contains("sml_rows: 2\n"), "{}", io.out);
    let mut io = fake();
    let args = ["inspect", "--format", "json", "bad.mztab"];
    assert_eq!(exec(&mut io, &args), OK, "json");
    assert!(
        io.out.starts_with("{\"version\":null,\"id\":\"FULL-1\""),
        "{}",
        io.out
    );
    assert!(io.out.contains("\"reader_errors\":1"), "{}", io.out);
    let mut io = fake();
    io.files.insert(
        "quoted.mztab".into(),
        "MTD\ttitle\ta \"b\" \\ c\u{1}\n".into(),
    );
    exec(&mut io, &["inspect", "--format", "json", "quoted.mztab"]);
    assert!(
        io.out.contains(r#""title":"a \"b\" \\ c\u0001""#),
        "escaped: {}",
        io.out
    );
}

#[test]
fn usage_and_io_errors() {
    let usage: &[&[&str]] = &[
        &["bogus"],
        &["validate"],
        &["validate", "a", "b"],
        &["validate", "--format"],
        &["validate", "--format", "xml", "a"],
        &["validate", "--input", "xml", "a"],
        &["validate", "--layer", "all", "a"],
        &["validate", "--fail-on", "fatal", "a"],
        &["validate", "--severity", "nonsense", "a"],
        &["validate", "--severity", "MZT-Z999=error", "a"],
        &["validate", "--severity", "MZT-S006=loud", "a"],
        &["validate", "--check", "a"],
        &["fmt", "--cv", "x", "a"],
        &["convert", "a"],
        &["inspect", "-o", "x", "a"],
    ];
    for args in usage {
        let mut io = fake();
        assert_eq!(exec(&mut io, args), USAGE, "{args:?}");
        assert!(io.err.starts_with("error: "), "{args:?}: {}", io.err);
    }
    let mut io = fake();
    assert_eq!(
        exec(&mut io, &["validate", "missing.mztab"]),
        FAILED,
        "missing file"
    );
    assert!(io.err.contains("cannot read `missing.mztab`"), "{}", io.err);
    let mut io = fake();
    let args = ["validate", "--cv", "missing.obo", "base.mztab"];
    assert_eq!(exec(&mut io, &args), FAILED, "missing vocabulary");
    let mut io = fake();
    io.files.insert("-".into(), BASE.into());
    assert_eq!(exec(&mut io, &["validate", "-"]), OK, "standard input");
}
