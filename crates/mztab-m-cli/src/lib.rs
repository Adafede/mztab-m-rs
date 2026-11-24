#![doc = include_str!("../README.md")]

mod args;
mod exec;

use args::{Command, Kind};

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
  mztab-m fmt [--check] [-o OUT] [--input tsv|json] FILE
  mztab-m convert --to tsv|json [--lossy] [-o OUT] [--input tsv|json] FILE
  mztab-m inspect [--format human|json] [--input tsv|json] FILE
  mztab-m --help | --version

FILE may be `-` for standard input. JSON input is recognised by a `.json`
name or a leading `{`; `--input tsv|json` overrides.

Common options:
  --input tsv|json       force input format (default: sniff from name or `{`)

validate options:
  --format human|json    findings as text lines (default) or a JSON array
  --layer syntax|structure|semantic
                         the last layer to run (default: semantic)
  --cv FILE.obo          OBO vocabulary file for semantic checks (repeatable)
  --fail-on error|warning|info|never
                         exit 1 if a finding is at least this severe
                         (default: error)
  --severity CODE=LEVEL  override CODE to error|warning|info|off (repeatable)

fmt options:
  --check                exit 1 if the file is not in canonical form
  -o OUT                 write output to OUT (default: `-`, standard output)

convert options:
  --to tsv|json          target format (required)
  --lossy                drop metadata, columns and cells outside the schema
                         instead of refusing; reports counts to stderr
  -o OUT                 write output to OUT (default: `-`, standard output)

inspect options:
  --format human|json    summary as key/value lines (default) or JSON object

Exit codes:
  0  ok (no finding at or above `--fail-on`, or file is canonical)
  1  findings at or above `--fail-on`, or `fmt --check` would change the file
  2  bad command line arguments
  3  file could not be read or written, or model cannot be written in format

Status: early development. The rules follow a draft of mzTab-M 2.1.0-M;
finding codes, messages and severities may change between versions.
";

/// Why a command did not finish.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Usage(String),
    Failed(String),
}

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
                mztab_m_api::SPEC_COMMIT
            ));
            Ok(OK)
        }
        Some(name) => Kind::from_name(name)
            .ok_or_else(|| Failure::Usage(format!("unknown command `{name}`")))
            .and_then(|kind| Command::parse(kind, args))
            .and_then(|cmd| exec::execute(&cmd, io)),
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
