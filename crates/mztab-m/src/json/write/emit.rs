//! Streaming JSON encoder for metadata values, parameters, references, and table cells.

use alloc::vec::Vec;
use core::fmt;

use super::split_key;
use crate::json::lexer::is_number;
use crate::json::schema::{Col, Shape};
use crate::json::value::write_string;
use crate::param::Param;
use crate::tsv::splitter::split_list;

/// Writes JSON directly to `out` without allocating an intermediate DOM.
pub(super) struct JsonWriter<'a> {
    out: &'a mut dyn fmt::Write,
}

impl<'a> JsonWriter<'a> {
    pub(super) fn new(out: &'a mut dyn fmt::Write) -> Self {
        Self { out }
    }

    pub(super) fn out(&mut self) -> &mut dyn fmt::Write {
        self.out
    }

    pub(super) fn raw(&mut self, s: &str) -> fmt::Result {
        self.out.write_str(s)
    }

    pub(super) fn char(&mut self, c: char) -> fmt::Result {
        self.out.write_char(c)
    }

    pub(super) fn str(&mut self, s: &str) -> fmt::Result {
        write_string(self.out, s)
    }

    pub(super) fn write_param(&mut self, param: &Param<'_>, id: Option<u32>) -> fmt::Result {
        self.raw("{")?;
        if let Some(id) = id {
            write!(self.out, "\"id\":{id},")?;
        }
        self.raw("\"cv_label\":")?;
        match param {
            Param::Cv {
                label, accession, ..
            } => {
                self.str(label.as_str())?;
                self.raw(",\"cv_accession\":")?;
                self.str(accession.as_str())?;
            }
            Param::User { .. } => self.raw("null,\"cv_accession\":null")?,
        }
        self.raw(",\"name\":")?;
        self.str(param.name().as_str())?;
        self.raw(",\"value\":")?;
        match param.value() {
            Some(v) => self.str(v.as_str())?,
            None => self.raw("null")?,
        }
        self.char('}')
    }

    /// Writes a metadata value in its JSON form, or its text as a string.
    pub(super) fn shaped(&mut self, text: &str, shape: Shape) -> fmt::Result {
        // An explicit `null` value: JSON `null` means the key is absent.
        if text == "null" {
            return self.str(text);
        }
        match shape {
            Shape::Text | Shape::Indexed | Shape::Strings => self.str(text),
            Shape::Param => self.try_param(text),
            Shape::Params => self.try_params(text),
            Shape::Ref(element) => match parse_ref(text, element) {
                Some(n) => write!(self.out, "{n}"),
                None => self.str(text),
            },
            Shape::Refs(element) => self.try_refs(text, element),
            Shape::Publication => self.try_publications(text),
        }
    }

    pub(super) fn colunit(&mut self, text: &str) -> fmt::Result {
        if let Some((column, unit)) = text.split_once('=')
            && let Ok(param) = Param::parse(unit)
        {
            self.raw("{\"column_name\":")?;
            self.str(column)?;
            self.raw(",\"param\":")?;
            self.write_param(&param, None)?;
            return self.char('}');
        }
        self.str(text)
    }

    /// Writes a table cell in its JSON form, or its text as a string.
    pub(super) fn cell(&mut self, text: &str, col: Col) -> fmt::Result {
        if text == "null" {
            return self.raw("null");
        }
        match col {
            Col::Text => self.str(text),
            // Only plain numbers: the JSON reader writes exponents out, so a
            // number in another form stays a string (its TSV text).
            Col::Number | Col::Integer => {
                if is_number(text) && !text.contains(['e', 'E']) {
                    self.raw(text)
                } else {
                    self.str(text)
                }
            }
            Col::Param => self.try_param(text),
            Col::Texts | Col::Numbers | Col::Integers => {
                let item_col = if col == Col::Texts {
                    Col::Text
                } else {
                    Col::Number
                };
                self.char('[')?;
                for (i, item) in text.split('|').enumerate() {
                    if i > 0 {
                        self.char(',')?;
                    }
                    self.cell(item, item_col)?;
                }
                self.char(']')
            }
            Col::SpectraRefs => self.try_spectra_refs(text),
        }
    }

    fn try_param(&mut self, text: &str) -> fmt::Result {
        match Param::parse(text) {
            Ok(param) => self.write_param(&param, None),
            Err(_) => self.str(text),
        }
    }

    fn try_params(&mut self, text: &str) -> fmt::Result {
        let Ok(params) = split_list(text, true)
            .into_iter()
            .map(|(_, item)| Param::parse(item))
            .collect::<Result<Vec<_>, _>>()
        else {
            return self.str(text);
        };
        self.write_array(&params, |w, param| w.write_param(param, None))
    }

    fn try_refs(&mut self, text: &str, element: &str) -> fmt::Result {
        let Some(refs) = text
            .split('|')
            .map(|item| parse_ref(item, element))
            .collect::<Option<Vec<_>>>()
        else {
            return self.str(text);
        };
        self.write_array(&refs, |w, n| write!(w.out, "{n}"))
    }

    fn try_publications(&mut self, text: &str) -> fmt::Result {
        let Some(items) = text
            .split('|')
            .map(|item| item.split_once(':'))
            .collect::<Option<Vec<_>>>()
        else {
            return self.str(text);
        };
        self.write_array(&items, |w, (kind, accession)| {
            w.raw("{\"type\":")?;
            w.str(kind)?;
            w.raw(",\"accession\":")?;
            w.str(accession)?;
            w.char('}')
        })
    }

    fn try_spectra_refs(&mut self, text: &str) -> fmt::Result {
        let Some(refs) = text
            .split('|')
            .map(|item| {
                let (run, reference) = item.split_once(':')?;
                Some((parse_ref(run, "ms_run")?, reference))
            })
            .collect::<Option<Vec<_>>>()
        else {
            return self.str(text);
        };
        self.write_array(&refs, |w, (n, reference)| {
            write!(w.out, "{{\"ms_run\":{n},\"reference\":")?;
            w.str(reference)?;
            w.char('}')
        })
    }

    fn write_array<T>(
        &mut self,
        items: &[T],
        mut write_item: impl FnMut(&mut Self, &T) -> fmt::Result,
    ) -> fmt::Result {
        self.char('[')?;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.char(',')?;
            }
            write_item(self, item)?;
        }
        self.char(']')
    }
}

fn parse_ref(text: &str, element: &str) -> Option<u32> {
    let (name, n, rest) = split_key(text)?;
    (name == element && rest.is_empty()).then_some(n)
}
