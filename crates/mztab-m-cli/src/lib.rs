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

use mztab_m::{Parsed, Severity, json};
use mztab_m_api::{Error, Fact, Format, Layer, Settings};
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
  mztab-m convert --to tsv|json [--lossy] [-o OUT] FILE
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

convert options:
  --lossy                drop what the specification does not define
                         (unknown metadata, unknown columns, extra cells)
                         instead of refusing; says what was dropped

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

/// A bad value on the command line.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a function for `map_err`, which passes the error by value"
)]
fn usage(error: Error) -> Failure {
    Failure::Usage(error.to_string())
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
    lossy: bool,
    cv: Vec<String>,
    fail_on: Option<Severity>,
    settings: Settings,
}

fn value<'a>(flag: &str, args: &mut impl Iterator<Item = &'a str>) -> Result<&'a str, Failure> {
    args.next()
        .ok_or_else(|| Failure::Usage(format!("`{flag}` needs a value")))
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
            lossy: false,
            cv: Vec::new(),
            fail_on: Some(Severity::Error),
            settings: Settings::default(),
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
            "convert" => &["--to", "-o", "--input", "--lossy"],
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
                "--input" => o.input = Some(Format::parse(value(arg, &mut args)?).map_err(usage)?),
                "--to" => o.to = Some(Format::parse(value(arg, &mut args)?).map_err(usage)?),
                "-o" => value(arg, &mut args)?.clone_into(&mut o.output),
                "--check" => o.check = true,
                "--lossy" => o.lossy = true,
                "--layer" => {
                    let layer = Layer::parse(value(arg, &mut args)?).map_err(usage)?;
                    o.settings = core::mem::take(&mut o.settings).layer(layer);
                }
                "--cv" => o.cv.push(value(arg, &mut args)?.to_owned()),
                "--fail-on" => {
                    let level = value(arg, &mut args)?;
                    o.fail_on = if level == "never" {
                        None
                    } else {
                        Some(mztab_m_api::severity(level).map_err(usage)?)
                    };
                }
                "--severity" => {
                    let setting = value(arg, &mut args)?;
                    o.settings = core::mem::take(&mut o.settings)
                        .set(setting)
                        .map_err(usage)?;
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
        let format = self.input.unwrap_or_else(|| Format::sniff(file, &bytes));
        let parsed = mztab_m_api::read(&bytes, format);
        match self.command.as_str() {
            "validate" => self.validate(&parsed, io),
            "fmt" => self.canonical(&parsed, &bytes, format, io),
            "convert" => {
                let to = self.to.unwrap_or(Format::Tsv);
                let (text, dropped) = mztab_m_api::write(&parsed.mztab, to, self.lossy)
                    .map_err(|e| Failure::Failed(e.to_string()))?;
                if dropped != mztab_m::Stripped::default() {
                    io.err(&format!(
                        "dropped {} unknown metadata lines, {} unknown columns, {} cells\n",
                        dropped.metadata, dropped.columns, dropped.cells
                    ));
                }
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
        let findings = mztab_m_api::validate(parsed, &self.settings, &terms);
        let mut out = String::new();
        if self.json {
            // Writing into a `String` cannot fail.
            json::write_findings(&findings, &mut out).unwrap_or_default();
            out.push('\n');
            io.out(&out);
        } else {
            out.extend(findings.iter().map(|f| format!("{f}\n")));
            io.out(&out);
            let count = |s| findings.iter().filter(|f| f.severity == s).count();
            io.err(&format!(
                "{} errors, {} warnings, {} infos\n",
                count(Severity::Error),
                count(Severity::Warning),
                count(Severity::Info)
            ));
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
        let (text, _) = mztab_m_api::write(&parsed.mztab, format, false)
            .map_err(|e| Failure::Failed(e.to_string()))?;
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
        let facts = mztab_m_api::facts(parsed);
        let out = if self.json {
            let members: Vec<String> = facts
                .iter()
                .map(|(key, value)| format!("\"{key}\":{}", value.to_json()))
                .collect();
            format!("{{{}}}\n", members.join(","))
        } else {
            facts
                .iter()
                .flat_map(|(key, value): &(&str, Fact)| {
                    [
                        (*key).to_owned(),
                        ": ".to_owned(),
                        value.to_string(),
                        "\n".to_owned(),
                    ]
                })
                .collect()
        };
        io.out(&out);
        OK
    }
}
