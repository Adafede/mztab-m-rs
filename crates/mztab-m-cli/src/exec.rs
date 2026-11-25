//! Subcommand execution via [`mztab_m_api`].

use mztab_m_api::{Format, Parsed, Severity, Stripped};

use super::args::{Action, Command, ConvertOpts, FmtOpts, ValidateOpts};
use super::{FINDINGS, Failure, Io, OK};

struct Loaded<'a> {
    file: &'a str,
    bytes: Vec<u8>,
    format: Format,
    parsed: Parsed<'static>,
}

pub(crate) fn execute(cmd: &Command, io: &mut dyn Io) -> Result<u8, Failure> {
    let bytes = io
        .read(&cmd.source.file)
        .map_err(|e| Failure::Failed(format!("cannot read `{}`: {e}", cmd.source.file)))?;
    let format = cmd
        .source
        .input
        .unwrap_or_else(|| Format::sniff(&cmd.source.file, &bytes));
    let parsed = mztab_m_api::read(&bytes, format);
    let loaded = Loaded {
        file: &cmd.source.file,
        bytes,
        format,
        parsed,
    };
    match &cmd.action {
        Action::Validate(opts) => run_validate(&loaded.parsed, opts, io),
        Action::Fmt(opts) => run_fmt(&loaded, opts, io),
        Action::Convert(opts) => run_convert(&loaded.parsed, opts, io),
        Action::Inspect { output_json } => Ok(run_inspect(&loaded.parsed, *output_json, io)),
    }
}

fn write_output(path: &str, text: &str, io: &mut dyn Io) -> Result<(), Failure> {
    io.write(path, text)
        .map_err(|e| Failure::Failed(format!("cannot write `{path}`: {e}")))
}

fn run_validate(
    parsed: &Parsed<'static>,
    opts: &ValidateOpts,
    io: &mut dyn Io,
) -> Result<u8, Failure> {
    let mut texts = Vec::new();
    for path in &opts.cv {
        let bytes = io
            .read(path)
            .map_err(|e| Failure::Failed(format!("cannot read `{path}`: {e}")))?;
        texts.push(String::from_utf8_lossy(&bytes).into_owned());
    }
    let obo: Vec<&str> = texts.iter().map(String::as_str).collect();
    let findings = mztab_m_api::validate(parsed, &opts.settings, &obo);
    if opts.output_json {
        let mut out = mztab_m_api::findings_json(&findings);
        out.push('\n');
        io.out(&out);
    } else {
        let mut out = String::new();
        for f in &findings {
            out.push_str(&f.to_string());
            out.push('\n');
        }
        io.out(&out);
        let count = |s| findings.iter().filter(|f| f.severity == s).count();
        io.err(&format!(
            "{} errors, {} warnings, {} infos\n",
            count(Severity::Error),
            count(Severity::Warning),
            count(Severity::Info)
        ));
    }
    let failing = opts
        .fail_on
        .is_some_and(|level| findings.iter().any(|f| f.severity >= level));
    Ok(if failing { FINDINGS } else { OK })
}

fn run_fmt(loaded: &Loaded<'_>, opts: &FmtOpts, io: &mut dyn Io) -> Result<u8, Failure> {
    let (text, _) = mztab_m_api::write(&loaded.parsed.mztab, loaded.format, false)
        .map_err(|e| Failure::Failed(e.to_string()))?;
    if opts.check {
        if text.as_bytes() == loaded.bytes {
            return Ok(OK);
        }
        io.err(&format!("{}: not in canonical form\n", loaded.file));
        return Ok(FINDINGS);
    }
    write_output(&opts.output, &text, io)?;
    Ok(OK)
}

fn run_convert(
    parsed: &Parsed<'static>,
    opts: &ConvertOpts,
    io: &mut dyn Io,
) -> Result<u8, Failure> {
    let (text, dropped) = mztab_m_api::write(&parsed.mztab, opts.to, opts.lossy)
        .map_err(|e| Failure::Failed(e.to_string()))?;
    if dropped != Stripped::default() {
        io.err(&format!(
            "dropped {} unknown metadata lines, {} unknown columns, {} cells\n",
            dropped.metadata, dropped.columns, dropped.cells
        ));
    }
    write_output(&opts.output, &text, io)?;
    Ok(OK)
}

fn run_inspect(parsed: &Parsed<'static>, output_json: bool, io: &mut dyn Io) -> u8 {
    let out = if output_json {
        format!("{}\n", mztab_m_api::facts_json(parsed))
    } else {
        let mut out = String::new();
        for (key, value) in mztab_m_api::facts(parsed) {
            out.push_str(key);
            out.push_str(": ");
            out.push_str(&value.to_string());
            out.push('\n');
        }
        out
    };
    io.out(&out);
    OK
}
