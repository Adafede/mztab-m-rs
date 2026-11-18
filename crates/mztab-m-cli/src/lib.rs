//! The `mztab-m` command: validate, format, convert and inspect mzTab-M
//! files (TSV or JSON).
//!
//! All the work is in [`run`], which takes its arguments and an [`Io`]; the
//! binary only supplies the real file system and standard streams. Exit
//! codes are [`OK`], [`FINDINGS`], [`USAGE`] and [`FAILED`].
//!
//! ```
//! use mztab_m_cli::{Io, OK, run};
//!
//! #[derive(Default)]
//! struct Capture(String);
//!
//! impl Io for Capture {
//!     fn read(&mut self, _path: &str) -> Result<Vec<u8>, String> {
//!         Ok(b"MTD\tmzTab-version\t2.1.0-M\n".to_vec())
//!     }
//!     fn write(&mut self, _path: &str, _text: &str) -> Result<(), String> {
//!         Ok(())
//!     }
//!     fn out(&mut self, text: &str) {
//!         self.0.push_str(text);
//!     }
//!     fn err(&mut self, _text: &str) {}
//! }
//!
//! let mut io = Capture::default();
//! assert_eq!(run(&["--version".to_owned()], &mut io), OK);
//! assert!(io.0.starts_with("mztab-m "));
//! ```

use mztab_m::cv::TermLookup;
use mztab_m::validate::{self, Config};
use mztab_m::{Code, Findings, IntoOwned as _, MzTab, Parsed, Severity, json};
use mztab_m_obo::Ontology;

/// Success: no finding at or above `--fail-on`; the file is formatted.
pub const OK: u8 = 0;
/// Validation found what `--fail-on` asks to fail on, or `fmt --check`
/// found a file that is not in canonical form.
pub const FINDINGS: u8 = 1;
/// The command line is wrong.
pub const USAGE: u8 = 2;
/// A file could not be read or written, or a model cannot be written in
/// the requested format.
pub const FAILED: u8 = 3;

/// The outside world, as the command sees it. `-` is standard input or
/// output.
pub trait Io {
    /// The bytes of a file (or of standard input, for `-`).
    ///
    /// # Errors
    ///
    /// A message saying why the file cannot be read.
    fn read(&mut self, path: &str) -> Result<Vec<u8>, String>;

    /// Writes a file (or standard output, for `-`).
    ///
    /// # Errors
    ///
    /// A message saying why the file cannot be written.
    fn write(&mut self, path: &str, text: &str) -> Result<(), String>;

    /// Writes to standard output.
    fn out(&mut self, text: &str);

    /// Writes to standard error.
    fn err(&mut self, text: &str);
}

const HELP: &str = "\
mztab-m: validate, format, convert and inspect mzTab-M files (TSV or JSON)

Usage:
  mztab-m validate [options] FILE
  mztab-m fmt [--check] [-o OUT] FILE
  mztab-m convert --to tsv|json [-o OUT] FILE
  mztab-m inspect [--format human|json] FILE
  mztab-m --help | --version

FILE may be `-` for standard input. JSON input is recognised by a `.json`
name or a leading `{`; `--input tsv|json` overrides.

validate options:
  --format human|json    findings as text lines (default) or a JSON array
  --layer syntax|structure|semantic
                         the last layer to run (default: semantic)
  --cv FILE.obo          a vocabulary for the semantic layer (repeatable)
  --fail-on error|warning|info|never
                         exit 1 if a finding is at least this severe
                         (default: error)
  --severity CODE=LEVEL  report CODE at LEVEL (error|warning|info|off);
                         repeatable

Exit codes: 0 ok, 1 findings (or `fmt --check` would change the file),
2 bad command line, 3 a file could not be read or written.
";

/// Runs the command line `args` (without the program name) and returns
/// the exit code.
pub fn run(args: &[String], io: &mut dyn Io) -> u8 {
    let mut args = args.iter().map(String::as_str);
    let result = match args.next() {
        None | Some("--help" | "-h" | "help") => {
            io.out(HELP);
            Ok(OK)
        }
        Some("--version" | "-V") => {
            io.out(&format!(
                "mztab-m {} (mzTab-M spec {})\n",
                env!("CARGO_PKG_VERSION"),
                mztab_m::SPEC_COMMIT
            ));
            Ok(OK)
        }
        Some(command @ ("validate" | "fmt" | "convert" | "inspect")) => {
            Options::parse(command, args).and_then(|options| options.run(io))
        }
        Some(other) => Err(Failure::Usage(format!("unknown command `{other}`"))),
    };
    match result {
        Ok(code) => code,
        Err(Failure::Usage(message)) => {
            io.err(&format!(
                "error: {message}\n\nRun `mztab-m --help` for usage.\n"
            ));
            USAGE
        }
        Err(Failure::Failed(message)) => {
            io.err(&format!("error: {message}\n"));
            FAILED
        }
    }
}

/// Why a command did not finish.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Usage(String),
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Tsv,
    Json,
}

/// The highest layer `validate` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    Syntax,
    Structure,
    Semantic,
}

#[derive(Debug)]
struct Options {
    command: String,
    file: Option<String>,
    output: String,
    input: Option<Format>,
    to: Option<Format>,
    json: bool,
    check: bool,
    layer: Layer,
    cv: Vec<String>,
    fail_on: Option<Severity>,
    config: Config,
}

fn value<'a>(flag: &str, args: &mut impl Iterator<Item = &'a str>) -> Result<&'a str, Failure> {
    args.next()
        .ok_or_else(|| Failure::Usage(format!("`{flag}` needs a value")))
}

fn format(flag: &str, text: &str) -> Result<Format, Failure> {
    match text {
        "tsv" => Ok(Format::Tsv),
        "json" => Ok(Format::Json),
        _ => Err(Failure::Usage(format!("`{flag}` takes `tsv` or `json`"))),
    }
}

fn severity(text: &str) -> Option<Severity> {
    match text {
        "error" => Some(Severity::Error),
        "warning" => Some(Severity::Warning),
        "info" => Some(Severity::Info),
        _ => None,
    }
}

impl Options {
    fn parse<'a>(command: &str, mut args: impl Iterator<Item = &'a str>) -> Result<Self, Failure> {
        let mut o = Self {
            command: command.to_owned(),
            file: None,
            output: "-".to_owned(),
            input: None,
            to: None,
            json: false,
            check: false,
            layer: Layer::Semantic,
            cv: Vec::new(),
            fail_on: Some(Severity::Error),
            config: Config::default(),
        };
        // Which flags each command takes.
        let allowed: &[&str] = match command {
            "validate" => &[
                "--format",
                "--input",
                "--layer",
                "--cv",
                "--fail-on",
                "--severity",
            ],
            "fmt" => &["--check", "-o", "--input"],
            "convert" => &["--to", "-o", "--input"],
            _ => &["--format", "--input"],
        };
        while let Some(arg) = args.next() {
            if arg.starts_with('-') && arg != "-" && !allowed.contains(&arg) {
                return Err(Failure::Usage(format!("`{command}` does not take `{arg}`")));
            }
            match arg {
                "--format" => {
                    o.json = match value(arg, &mut args)? {
                        "human" => false,
                        "json" => true,
                        _ => {
                            return Err(Failure::Usage(
                                "`--format` takes `human` or `json`".into(),
                            ));
                        }
                    };
                }
                "--input" => o.input = Some(format(arg, value(arg, &mut args)?)?),
                "--to" => o.to = Some(format(arg, value(arg, &mut args)?)?),
                "-o" => value(arg, &mut args)?.clone_into(&mut o.output),
                "--check" => o.check = true,
                "--layer" => {
                    o.layer = match value(arg, &mut args)? {
                        "syntax" => Layer::Syntax,
                        "structure" => Layer::Structure,
                        "semantic" => Layer::Semantic,
                        _ => {
                            return Err(Failure::Usage(
                                "`--layer` takes `syntax`, `structure` or `semantic`".into(),
                            ));
                        }
                    };
                }
                "--cv" => o.cv.push(value(arg, &mut args)?.to_owned()),
                "--fail-on" => {
                    let level = value(arg, &mut args)?;
                    o.fail_on = match (level, severity(level)) {
                        ("never", _) => None,
                        (_, Some(s)) => Some(s),
                        _ => {
                            return Err(Failure::Usage(
                                "`--fail-on` takes `error`, `warning`, `info` or `never`".into(),
                            ));
                        }
                    };
                }
                "--severity" => {
                    let setting = value(arg, &mut args)?;
                    o.config = configure(core::mem::take(&mut o.config), setting)?;
                }
                file if o.file.is_none() => o.file = Some(file.to_owned()),
                extra => return Err(Failure::Usage(format!("unexpected `{extra}`"))),
            }
        }
        if o.file.is_none() {
            return Err(Failure::Usage(format!("`{command}` needs a FILE")));
        }
        if command == "convert" && o.to.is_none() {
            return Err(Failure::Usage("`convert` needs `--to tsv|json`".into()));
        }
        Ok(o)
    }

    fn run(&self, io: &mut dyn Io) -> Result<u8, Failure> {
        let file = self.file.as_deref().unwrap_or("-");
        let bytes = io
            .read(file)
            .map_err(|e| Failure::Failed(format!("cannot read `{file}`: {e}")))?;
        let format = self.input.unwrap_or_else(|| sniff(file, &bytes));
        let parsed = read(&bytes, format);
        match self.command.as_str() {
            "validate" => self.validate(&parsed, io),
            "fmt" => self.canonical(&parsed, &bytes, format, io),
            "convert" => {
                let text = write(&parsed.mztab, self.to.unwrap_or(Format::Tsv))?;
                self.output(&text, io)?;
                Ok(OK)
            }
            _ => Ok(self.inspect(&parsed, io)),
        }
    }

    fn output(&self, text: &str, io: &mut dyn Io) -> Result<(), Failure> {
        io.write(&self.output, text)
            .map_err(|e| Failure::Failed(format!("cannot write `{}`: {e}", self.output)))
    }

    fn validate(&self, parsed: &Parsed<'static>, io: &mut dyn Io) -> Result<u8, Failure> {
        let mut texts = Vec::new();
        for path in &self.cv {
            let bytes = io
                .read(path)
                .map_err(|e| Failure::Failed(format!("cannot read `{path}`: {e}")))?;
            texts.push(String::from_utf8_lossy(&bytes).into_owned());
        }
        let vocabularies: Vec<Ontology<'_>> = texts.iter().map(|t| Ontology::parse(t)).collect();
        let terms: &[Ontology<'_>] = &vocabularies;
        let findings = findings(parsed, self.layer, &self.config, &terms);
        let mut out = String::new();
        if self.json {
            // Writing into a `String` cannot fail.
            json::write_findings(&findings, &mut out).unwrap_or_default();
            out.push('\n');
            io.out(&out);
        } else {
            out.extend(findings.iter().map(|f| format!("{f}\n")));
            io.out(&out);
            io.err(&summary(&findings));
        }
        let failing = self
            .fail_on
            .is_some_and(|level| findings.iter().any(|f| f.severity >= level));
        Ok(if failing { FINDINGS } else { OK })
    }

    fn canonical(
        &self,
        parsed: &Parsed<'static>,
        bytes: &[u8],
        format: Format,
        io: &mut dyn Io,
    ) -> Result<u8, Failure> {
        let text = write(&parsed.mztab, format)?;
        if self.check {
            if text.as_bytes() == bytes {
                return Ok(OK);
            }
            let file = self.file.as_deref().unwrap_or("-");
            io.err(&format!("{file}: not in canonical form\n"));
            return Ok(FINDINGS);
        }
        self.output(&text, io)?;
        Ok(OK)
    }

    fn inspect(&self, parsed: &Parsed<'static>, io: &mut dyn Io) -> u8 {
        let facts = facts(parsed);
        let out = if self.json {
            let members: Vec<String> = facts
                .iter()
                .map(|(key, value)| format!("\"{key}\":{value}"))
                .collect();
            format!("{{{}}}\n", members.join(","))
        } else {
            facts
                .iter()
                .flat_map(|(key, value)| [*key, ": ", value.trim_matches('"'), "\n"])
                .collect()
        };
        io.out(&out);
        OK
    }
}

/// `CODE=LEVEL`: report CODE at LEVEL, or not at all (`off`).
fn configure(config: Config, setting: &str) -> Result<Config, Failure> {
    let bad = || Failure::Usage(format!("`--severity` takes CODE=LEVEL, not `{setting}`"));
    let (id, level) = setting.split_once('=').ok_or_else(bad)?;
    let code = Code::ALL
        .iter()
        .copied()
        .find(|c| c.id() == id)
        .ok_or_else(|| Failure::Usage(format!("unknown code `{id}`")))?;
    match (level, severity(level)) {
        ("off", _) => Ok(config.without(code)),
        (_, Some(s)) => Ok(config.with_severity(code, s)),
        _ => Err(bad()),
    }
}

/// JSON if the name says so or the text starts with `{`.
fn sniff(file: &str, bytes: &[u8]) -> Format {
    let first = bytes.iter().find(|b| !b.is_ascii_whitespace());
    let named = std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if named || first == Some(&b'{') {
        Format::Json
    } else {
        Format::Tsv
    }
}

fn read(bytes: &[u8], format: Format) -> Parsed<'static> {
    match format {
        Format::Tsv => mztab_m::parse_bytes(bytes).into_owned(),
        Format::Json => json::parse(&String::from_utf8_lossy(bytes)),
    }
}

/// The canonical text of a model in `format`.
fn write(mztab: &MzTab<'_>, format: Format) -> Result<String, Failure> {
    let written = match format {
        Format::Tsv => {
            let mut canonical = mztab.clone();
            canonical.layout = mztab_m::SourceLayout::default();
            mztab_m::to_string(&canonical)
        }
        Format::Json => json::to_string(mztab),
    };
    written.map_err(|e| Failure::Failed(format!("cannot write the file: {e}")))
}

/// The reader's findings and those of each layer up to `layer`, with
/// `config` applied. Values that failed to type are reported once, by the
/// reader.
fn findings(
    parsed: &Parsed<'static>,
    layer: Layer,
    config: &Config,
    terms: &dyn TermLookup,
) -> Findings {
    let mut out = Findings::new();
    for mut finding in parsed.findings.iter().cloned() {
        if let Some(severity) = config.severity_of(finding.code) {
            finding.severity = severity;
            out.push(finding);
        }
    }
    if layer >= Layer::Structure {
        let structure = validate::structure(&parsed.mztab, config);
        out.extend(
            structure
                .into_vec()
                .into_iter()
                .filter(|f| f.code != Code::InvalidValue),
        );
    }
    if layer >= Layer::Semantic {
        out.extend(validate::semantic(&parsed.mztab, config, terms).into_vec());
    }
    out
}

fn summary(findings: &Findings) -> String {
    let count = |s| findings.iter().filter(|f| f.severity == s).count();
    format!(
        "{} errors, {} warnings, {} infos\n",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    )
}

/// What `inspect` reports: key and JSON value.
fn facts(parsed: &Parsed<'static>) -> Vec<(&'static str, String)> {
    let m = &parsed.mztab;
    let meta = &m.metadata;
    let text = |v: Option<String>| v.map_or_else(|| "null".to_owned(), |v| format!("\"{v}\""));
    let cell = |c: Option<&mztab_m::Cell<'_, mztab_m::Text<'_>>>| {
        text(c.and_then(|c| c.value()).map(|t| escape(t.as_str())))
    };
    let version = meta
        .version
        .as_ref()
        .and_then(|c| c.value())
        .map(ToString::to_string);
    let profile = meta
        .profile
        .as_ref()
        .and_then(|c| c.value())
        .map(ToString::to_string);
    let inferred = m.inferred_profile().ok().map(|p| p.to_string());
    let rows = |n: Option<usize>| n.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let errors = parsed
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    Vec::from([
        ("version", text(version)),
        ("id", cell(meta.id.as_ref())),
        ("title", cell(meta.title.as_ref())),
        ("profile", text(profile)),
        ("tables_profile", text(inferred)),
        ("ms_runs", meta.ms_run.iter().count().to_string()),
        ("assays", meta.assay.iter().count().to_string()),
        (
            "study_variables",
            meta.study_variable.iter().count().to_string(),
        ),
        ("samples", meta.sample.iter().count().to_string()),
        ("cvs", meta.cv.iter().count().to_string()),
        ("sml_rows", rows(m.sml.as_ref().map(|t| t.rows.len()))),
        ("smf_rows", rows(m.smf.as_ref().map(|t| t.rows.len()))),
        ("sme_rows", rows(m.sme.as_ref().map(|t| t.rows.len()))),
        ("reader_findings", parsed.findings.len().to_string()),
        ("reader_errors", errors.to_string()),
    ])
}

/// Escapes text for a JSON string body.
fn escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            c if u32::from(c) < 0x20 => format!("\\u{:04x}", u32::from(c)),
            c => c.to_string(),
        })
        .collect()
}
