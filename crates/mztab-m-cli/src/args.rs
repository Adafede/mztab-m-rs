//! Subcommand argument parsing into a typed [`Command`].

use mztab_m_api::{Error, Format, Layer, Settings, Severity};

use super::Failure;

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Validate,
    Fmt,
    Convert,
    Inspect,
}

impl Kind {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "validate" => Some(Self::Validate),
            "fmt" => Some(Self::Fmt),
            "convert" => Some(Self::Convert),
            "inspect" => Some(Self::Inspect),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Fmt => "fmt",
            Self::Convert => "convert",
            Self::Inspect => "inspect",
        }
    }

    const fn allowed_flags(self) -> &'static [&'static str] {
        match self {
            Self::Validate => &[
                "--format",
                "--input",
                "--layer",
                "--cv",
                "--fail-on",
                "--severity",
            ],
            Self::Fmt => &["--check", "-o", "--input"],
            Self::Convert => &["--to", "-o", "--input", "--lossy"],
            Self::Inspect => &["--format", "--input"],
        }
    }
}

pub(crate) struct Source {
    pub(crate) file: String,
    pub(crate) input: Option<Format>,
}

pub(crate) struct ValidateOpts {
    pub(crate) output_json: bool,
    pub(crate) cv: Vec<String>,
    pub(crate) fail_on: Option<Severity>,
    pub(crate) settings: Settings,
}

pub(crate) struct FmtOpts {
    pub(crate) output: String,
    pub(crate) check: bool,
}

pub(crate) struct ConvertOpts {
    pub(crate) output: String,
    pub(crate) to: Format,
    pub(crate) lossy: bool,
}

pub(crate) enum Action {
    Validate(ValidateOpts),
    Fmt(FmtOpts),
    Convert(ConvertOpts),
    Inspect { output_json: bool },
}

pub(crate) struct Command {
    pub(crate) source: Source,
    pub(crate) action: Action,
}

struct Raw {
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

impl Default for Raw {
    fn default() -> Self {
        Self {
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
        }
    }
}

fn flag_value<'a>(
    flag: &str,
    args: &mut impl Iterator<Item = &'a str>,
) -> Result<&'a str, Failure> {
    args.next()
        .ok_or_else(|| Failure::Usage(format!("`{flag}` needs a value")))
}

fn usage<T>(res: Result<T, Error>) -> Result<T, Failure> {
    res.map_err(|err| Failure::Usage(err.to_string()))
}

impl Raw {
    fn apply<'a>(
        &mut self,
        arg: &'a str,
        args: &mut impl Iterator<Item = &'a str>,
    ) -> Result<(), Failure> {
        match arg {
            "--format" => {
                self.json = match flag_value(arg, args)? {
                    "human" => false,
                    "json" => true,
                    _ => return Err(Failure::Usage("`--format` takes `human` or `json`".into())),
                };
            }
            "--input" => self.input = Some(usage(flag_value(arg, args)?.parse::<Format>())?),
            "--to" => self.to = Some(usage(flag_value(arg, args)?.parse::<Format>())?),
            "-o" => flag_value(arg, args)?.clone_into(&mut self.output),
            "--check" => self.check = true,
            "--lossy" => self.lossy = true,
            "--layer" => {
                let layer = flag_value(arg, args)?;
                self.settings.layer = usage(
                    layer
                        .parse::<Layer>()
                        .map_err(|_| Error::Layer(layer.to_owned())),
                )?;
            }
            "--cv" => self.cv.push(flag_value(arg, args)?.to_owned()),
            "--fail-on" => {
                let level = flag_value(arg, args)?;
                self.fail_on = (level != "never")
                    .then(|| {
                        usage(
                            level
                                .parse::<Severity>()
                                .map_err(|_| Error::Severity(level.to_owned())),
                        )
                    })
                    .transpose()?;
            }
            "--severity" => {
                let setting = flag_value(arg, args)?;
                self.settings = usage(core::mem::take(&mut self.settings).set(setting))?;
            }
            path if self.file.is_none() => self.file = Some(path.to_owned()),
            extra => return Err(Failure::Usage(format!("unexpected `{extra}`"))),
        }
        Ok(())
    }

    fn finish(self, kind: Kind) -> Result<Command, Failure> {
        let name = kind.as_str();
        let file = self
            .file
            .ok_or_else(|| Failure::Usage(format!("`{name}` needs a FILE")))?;
        let source = Source {
            file,
            input: self.input,
        };
        let action = match kind {
            Kind::Validate => Action::Validate(ValidateOpts {
                output_json: self.json,
                cv: self.cv,
                fail_on: self.fail_on,
                settings: self.settings,
            }),
            Kind::Fmt => Action::Fmt(FmtOpts {
                output: self.output,
                check: self.check,
            }),
            Kind::Convert => {
                let to = self
                    .to
                    .ok_or_else(|| Failure::Usage("`convert` needs `--to tsv|json`".into()))?;
                Action::Convert(ConvertOpts {
                    output: self.output,
                    to,
                    lossy: self.lossy,
                })
            }
            Kind::Inspect => Action::Inspect {
                output_json: self.json,
            },
        };
        Ok(Command { source, action })
    }
}

impl Command {
    pub(crate) fn parse<'a>(
        kind: Kind,
        mut args: impl Iterator<Item = &'a str>,
    ) -> Result<Self, Failure> {
        let mut raw = Raw::default();
        let allowed = kind.allowed_flags();
        while let Some(arg) = args.next() {
            if arg.starts_with('-') && arg != "-" && !allowed.contains(&arg) {
                return Err(Failure::Usage(format!(
                    "`{}` does not take `{arg}`",
                    kind.as_str()
                )));
            }
            raw.apply(arg, &mut args)?;
        }
        raw.finish(kind)
    }
}
