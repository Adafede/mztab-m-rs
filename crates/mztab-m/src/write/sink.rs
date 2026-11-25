//! Output sinks: FNV-1a fingerprinting, verbatim source comparison, and checked TSV emission.

use core::fmt;

use super::error::WriteError;
use crate::model::layout::LineEnding;
use crate::tsv::splitter::Splitter;

/// FNV-1a over everything written: the fingerprint of a line's canonical
/// form, recorded by the reader and checked by the writer.
#[derive(Debug)]
pub(crate) struct Fingerprint(u64);

impl Fingerprint {
    pub(crate) const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) const fn finish(&self) -> u64 {
        self.0
    }
}

impl fmt::Write for Fingerprint {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        Ok(())
    }
}

/// Compares what is written with a source line, fingerprinting it as well.
#[derive(Debug)]
pub(crate) struct Compare<'s> {
    rest: Option<&'s str>,
    pub(crate) fingerprint: Fingerprint,
}

impl<'s> Compare<'s> {
    pub(crate) const fn new(source: &'s str) -> Self {
        Self {
            rest: Some(source),
            fingerprint: Fingerprint::new(),
        }
    }

    /// Whether everything written so far equals the whole source line.
    pub(crate) fn matched(&self) -> bool {
        self.rest.is_some_and(str::is_empty)
    }
}

impl fmt::Write for Compare<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.rest = self.rest.and_then(|rest| rest.strip_prefix(s));
        self.fingerprint.write_str(s)
    }
}

/// The output, with the line being written (for error locations) and,
/// inside a list, the reader's list splitter.
pub(crate) struct Out<'o> {
    sink: &'o mut dyn fmt::Write,
    line: u32,
    /// Whether the text written on this line so far ends with `\r`.
    after_cr: bool,
    /// Inside a list: how the reader will split what is written.
    list: Option<Splitter>,
    /// An error raised while formatting through `fmt::Write`.
    failed: Option<WriteError>,
}

impl<'o> Out<'o> {
    pub(crate) fn new(out: &'o mut dyn fmt::Write) -> Self {
        Self {
            sink: out,
            line: 1,
            after_cr: false,
            list: None,
            failed: None,
        }
    }

    /// Writes text. Inside a list, a `|` that the reader would take as a
    /// separator is refused.
    pub(crate) fn str(&mut self, text: &str) -> Result<(), WriteError> {
        if let Some(splitter) = &mut self.list
            && text.bytes().any(|b| splitter.step(b))
        {
            return Err(self.fail("a list item containing `|`"));
        }
        if let Some(last) = text.as_bytes().last() {
            self.after_cr = *last == b'\r';
        }
        Ok(self.sink.write_str(text)?)
    }

    /// Writes a value's `Display` form, through [`Out::str`].
    pub(super) fn display(&mut self, value: &dyn fmt::Display) -> Result<(), WriteError> {
        if fmt::write(self, format_args!("{value}")).is_ok() {
            return Ok(());
        }
        Err(self.failed.take().unwrap_or(WriteError::Fmt))
    }

    /// Writes the `|` between two list items.
    pub(super) fn separator(&mut self) -> Result<(), WriteError> {
        if self.list.is_some_and(|splitter| !splitter.at_rest()) {
            return Err(self.fail("a list item that leaves `[` or `\"` open"));
        }
        self.after_cr = false;
        Ok(self.sink.write_str("|")?)
    }

    pub(super) const fn fail(&self, reason: &'static str) -> WriteError {
        WriteError::Unwritable {
            line: self.line,
            reason,
        }
    }

    /// Raw text that must stay in one cell. A `\r` is allowed inside a
    /// line, not at its end (see [`Out::ending`]).
    pub(super) fn checked(&mut self, text: &str) -> Result<(), WriteError> {
        if text.contains(['\t', '\n']) {
            return Err(self.fail("raw text with a tab or line break"));
        }
        self.str(text)
    }

    pub(super) fn ending(&mut self, ending: LineEnding) -> Result<(), WriteError> {
        if self.after_cr && ending == LineEnding::Lf {
            return Err(self.fail("a line ending in `\\r`, which reads back as a CRLF ending"));
        }
        let text = match ending {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::None => "",
        };
        self.line = self.line.saturating_add(1);
        self.str(text)
    }

    pub(super) fn with_list(
        &mut self,
        brackets: bool,
        write: impl FnOnce(&mut Self) -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        let outer = self.list.replace(Splitter::new(brackets));
        let res = write(self);
        self.list = outer;
        res
    }
}

impl fmt::Write for Out<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.str(s).map_err(|error| {
            self.failed = Some(error);
            fmt::Error
        })
    }
}
