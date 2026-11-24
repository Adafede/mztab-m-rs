//! Parameter grammar parser and located error types.

use core::fmt;

use super::{Accession, Param, ParamText};
use crate::value::find_control;

/// What is wrong with a parameter.
///
/// # Examples
///
/// ```
/// use mztab_m::{ParamErrorKind, Param};
///
/// let err = Param::parse("[MS, , name, ]").unwrap_err();
/// assert_eq!(err.kind, ParamErrorKind::HalfCv);
/// assert_eq!(err.offset, 1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParamErrorKind {
    /// The input does not start with `[`.
    MissingOpenBracket,
    /// The input does not end with `]`.
    MissingCloseBracket,
    /// A quoted field is not closed.
    UnclosedQuote,
    /// Text other than spaces follows a closing quote, or a quote appears
    /// inside an unquoted field.
    StrayQuote,
    /// The number of fields is not four.
    FieldCount {
        /// The number of fields found.
        found: usize,
    },
    /// The name field is empty.
    MissingName,
    /// Exactly one of label and accession is present.
    HalfCv,
    /// A field contains a tab or a line break.
    ControlChar,
}

/// A parameter that does not follow the grammar, and where.
///
/// `offset` is a byte offset into the parsed input, suitable for turning into
/// a column number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamError {
    /// What is wrong.
    pub kind: ParamErrorKind,
    /// Byte offset of the problem in the input.
    pub offset: usize,
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ParamErrorKind::MissingOpenBracket => "expected `[`",
            ParamErrorKind::MissingCloseBracket => "expected `]` at the end",
            ParamErrorKind::UnclosedQuote => "unclosed double quote",
            ParamErrorKind::StrayQuote => "unexpected double quote or text after a quoted field",
            ParamErrorKind::FieldCount { found } => {
                return write!(
                    f,
                    "expected 4 comma-separated fields, found {found} (at byte {})",
                    self.offset
                );
            }
            ParamErrorKind::MissingName => "the name field is empty",
            ParamErrorKind::HalfCv => "a CV parameter needs both a label and an accession",
            ParamErrorKind::ControlChar => "tab or line break inside a parameter",
        };
        write!(f, "{what} (at byte {})", self.offset)
    }
}

impl core::error::Error for ParamError {}

const fn err(kind: ParamErrorKind, offset: usize) -> ParamError {
    ParamError { kind, offset }
}

/// One field: its text (if non-empty) and its start offset in the input.
type Field<'a> = (Option<ParamText<'a>>, usize);

pub(super) fn parse_param(input: &str) -> Result<Param<'_>, ParamError> {
    let inner = input
        .strip_prefix('[')
        .ok_or_else(|| err(ParamErrorKind::MissingOpenBracket, 0))?
        .strip_suffix(']')
        .ok_or_else(|| err(ParamErrorKind::MissingCloseBracket, input.len()))?;
    if let Some(at) = find_control(inner) {
        return Err(err(ParamErrorKind::ControlChar, at + 1));
    }
    let [
        (label, label_at),
        (accession, _),
        (name, name_at),
        (value, _),
    ] = split_four_fields(inner, input.len())?;
    let name = name.ok_or_else(|| err(ParamErrorKind::MissingName, name_at))?;
    match (label, accession) {
        (Some(label), Some(accession)) => Ok(Param::Cv {
            label,
            accession: Accession::new(accession),
            name,
            value,
        }),
        (None, None) => Ok(Param::User { name, value }),
        _ => Err(err(ParamErrorKind::HalfCv, label_at)),
    }
}

fn split_four_fields(inner: &str, total_len: usize) -> Result<[Field<'_>; 4], ParamError> {
    let mut fields: [Field<'_>; 4] = [(None, 0), (None, 0), (None, 0), (None, 0)];
    let mut found = 0;
    let mut start = 0;
    // Every field but the last ends at a comma, so there are at most
    // `inner.len() + 1` fields. The bound makes termination evident rather
    // than a consequence of the offset arithmetic.
    for _ in 0..=inner.len() {
        let (field, end) = scan_field(inner, start)?;
        if let Some(slot) = fields.get_mut(found) {
            *slot = (field, start + 1);
        }
        found += 1;
        match end {
            Some(comma) => start = comma + 1,
            None => break,
        }
    }
    if found != 4 {
        return Err(err(ParamErrorKind::FieldCount { found }, total_len - 1));
    }
    Ok(fields)
}

/// Scans one field of `inner` starting at `start`.
///
/// Returns the field text (`None` when empty) and the offset of the comma
/// that ends it (`None` at the end of `inner`). Offsets in errors are relative
/// to the whole input, which has one leading `[`.
fn scan_field(
    inner: &str,
    start: usize,
) -> Result<(Option<ParamText<'_>>, Option<usize>), ParamError> {
    let rest = inner.get(start..).unwrap_or_default();
    let lead = rest.len() - rest.trim_start_matches(' ').len();
    let body_at = start + lead;
    let body = rest.get(lead..).unwrap_or_default();
    let (text, after_at) = if let Some(quoted) = body.strip_prefix('"') {
        let close = quoted
            .find('"')
            .ok_or_else(|| err(ParamErrorKind::UnclosedQuote, body_at + 1))?;
        (quoted.get(..close).unwrap_or_default(), body_at + close + 2)
    } else {
        let len = body.find(',').unwrap_or(body.len());
        let text = body.get(..len).unwrap_or_default().trim_end_matches(' ');
        if let Some(q) = text.find('"') {
            return Err(err(ParamErrorKind::StrayQuote, body_at + q + 1));
        }
        (text, body_at + text.len())
    };
    let after = inner.get(after_at..).unwrap_or_default();
    let end_at = after_at + (after.len() - after.trim_start_matches(' ').len());
    let end = match inner.as_bytes().get(end_at) {
        None => None,
        Some(b',') => Some(end_at),
        Some(_) => return Err(err(ParamErrorKind::StrayQuote, end_at + 1)),
    };
    // `text` has no quote (it ended at one, or we checked) and no control
    // character (the caller checked), so only emptiness can fail.
    Ok((ParamText::new(text).ok(), end))
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{ParamError, ParamErrorKind};
    use crate::param::Param;
    use alloc::string::ToString as _;

    fn fail(input: &str) -> Option<(ParamErrorKind, usize)> {
        Param::parse(input)
            .err()
            .map(|ParamError { kind, offset }| (kind, offset))
    }

    #[test]
    fn errors_are_located() {
        let cases: &[(&str, ParamErrorKind, usize)] = &[
            ("", ParamErrorKind::MissingOpenBracket, 0),
            (" [,,n,]", ParamErrorKind::MissingOpenBracket, 0),
            ("[,,n,", ParamErrorKind::MissingCloseBracket, 5),
            ("[,,n,] ", ParamErrorKind::MissingCloseBracket, 7),
            ("[,,n]", ParamErrorKind::FieldCount { found: 3 }, 4),
            ("[,,n,,]", ParamErrorKind::FieldCount { found: 5 }, 6),
            ("[]", ParamErrorKind::FieldCount { found: 1 }, 1),
            ("[,,,]", ParamErrorKind::MissingName, 3),
            ("[,, ,v]", ParamErrorKind::MissingName, 3),
            ("[MS,,n,]", ParamErrorKind::HalfCv, 1),
            ("[,MS:1,n,]", ParamErrorKind::HalfCv, 1),
            ("[,,\"n,]", ParamErrorKind::UnclosedQuote, 3),
            ("[,,\"n\"x,]", ParamErrorKind::StrayQuote, 6),
            ("[,,\"n\" x,]", ParamErrorKind::StrayQuote, 7),
            ("[,,a\"b,]", ParamErrorKind::StrayQuote, 4),
            ("[,,n\t,]", ParamErrorKind::ControlChar, 4),
            ("[,,n,\r]", ParamErrorKind::ControlChar, 5),
        ];
        for &(input, kind, offset) in cases {
            assert_eq!(fail(input), Some((kind, offset)), "{input:?}");
        }
    }

    #[test]
    fn error_messages() {
        let msg = |kind, offset| ParamError { kind, offset }.to_string();
        assert_eq!(
            msg(ParamErrorKind::MissingOpenBracket, 0),
            "expected `[` (at byte 0)",
            "open"
        );
        assert_eq!(
            msg(ParamErrorKind::MissingCloseBracket, 3),
            "expected `]` at the end (at byte 3)",
            "close"
        );
        assert_eq!(
            msg(ParamErrorKind::UnclosedQuote, 1),
            "unclosed double quote (at byte 1)",
            "quote"
        );
        assert_eq!(
            msg(ParamErrorKind::StrayQuote, 1),
            "unexpected double quote or text after a quoted field (at byte 1)",
            "stray"
        );
        assert_eq!(
            msg(ParamErrorKind::FieldCount { found: 2 }, 5),
            "expected 4 comma-separated fields, found 2 (at byte 5)",
            "count"
        );
        assert_eq!(
            msg(ParamErrorKind::MissingName, 1),
            "the name field is empty (at byte 1)",
            "name"
        );
        assert_eq!(
            msg(ParamErrorKind::HalfCv, 1),
            "a CV parameter needs both a label and an accession (at byte 1)",
            "half"
        );
        assert_eq!(
            msg(ParamErrorKind::ControlChar, 1),
            "tab or line break inside a parameter (at byte 1)",
            "control"
        );
    }
}
