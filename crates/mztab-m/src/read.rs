//! Reading TSV: [`parse`], [`parse_bytes`] and the push-based
//! [`StreamReader`].
//!
//! Both entry points run the same line typer, which turns each line into
//! [`Event`]s. [`parse`] collects the events into an [`MzTab`](crate::MzTab) that borrows
//! the input; [`StreamReader`] hands them to a callback as complete lines
//! arrive, so memory stays proportional to one line.
//!
//! Reading never fails and never stops early: every problem becomes a
//! [`Finding`](crate::Finding), and the text that caused it is kept in the model.
//!
//! ```
//! let input = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tEX-1\n";
//! let parsed = mztab_m::parse(input);
//! assert!(parsed.findings.is_empty());
//! assert_eq!(parsed.mztab.metadata.id.and_then(|c| c.value().map(|t| t.to_string())), Some("EX-1".into()));
//! ```

mod builder;
mod event;
mod metadata;
mod stream;
mod typer;

pub use builder::{parse, parse_bytes};
pub use event::{Event, Parsed};
pub use stream::StreamReader;

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Event, Parsed, parse, parse_bytes};
    use crate::model::{LineEnding, LineInfo, SmeRow, SmfRow, SmlRow};
    use crate::tsv::TableRow as _;
    use alloc::vec;

    #[test]
    fn parse_is_parse_bytes() {
        let text = "MTD\tmzTab-ID\tx\r\nCOM\tlast";
        assert_eq!(parse(text), parse_bytes(text.as_bytes()), "same result");
        let endings: vec::Vec<_> = parse(text)
            .mztab
            .layout
            .lines
            .iter()
            .map(|line| line.ending)
            .collect();
        assert_eq!(
            endings,
            [LineEnding::CrLf, LineEnding::None],
            "endings kept"
        );
        assert!(parse("").findings.is_empty(), "empty input");
    }

    #[test]
    fn section_order_is_reported_once_per_backward_step() {
        let text = "SFH\tSMF_ID\nMTD\tmzTab-ID\tx\nSMF\t1\nSMH\tSML_ID\nSML\t1\n";
        let lines: vec::Vec<_> = parse(text)
            .findings
            .iter()
            .filter(|f| f.code == crate::diag::Code::SectionOrder)
            .map(|f| f.location.clone())
            .collect();
        let at = |line| crate::diag::Location::text(line, 1).with_cell(1);
        assert_eq!(
            lines,
            [at(2), at(4)],
            "MTD after SMF, SMH after SMF; not SMF again"
        );
    }

    #[test]
    fn verbatim_before_any_line_is_ignored() {
        use crate::model::Verbatim;
        use crate::value::Raw;
        let parsed: Parsed<'_> = vec![Event::Verbatim(Verbatim {
            text: Raw::new("x"),
            fingerprint: 0,
        })]
        .into_iter()
        .collect();
        assert!(
            parsed.mztab.layout.lines.is_empty(),
            "nothing to attach it to"
        );
        let spaced = parse("MTD\tquantification_method\t[,,a,]");
        let first = spaced.mztab.layout.lines.first();
        let verbatim = first.and_then(|line| line.verbatim.as_ref());
        assert_eq!(
            verbatim.map(|v| v.text.as_str()),
            Some("MTD\tquantification_method\t[,,a,]"),
            "attached to its line"
        );
    }

    #[test]
    fn rows_without_header_events_get_an_empty_table() {
        let info = LineInfo {
            number: 1,
            ending: LineEnding::Lf,
        };
        let parsed: Parsed<'_> = vec![
            Event::Sml(info, SmlRow::empty()),
            Event::Smf(info, SmfRow::empty()),
            Event::Sme(info, SmeRow::empty()),
        ]
        .into_iter()
        .collect();
        let counts = [
            parsed
                .mztab
                .sml
                .map(|t| (t.header.columns.len(), t.rows.len())),
            parsed
                .mztab
                .smf
                .map(|t| (t.header.columns.len(), t.rows.len())),
            parsed
                .mztab
                .sme
                .map(|t| (t.header.columns.len(), t.rows.len())),
        ];
        assert_eq!(counts, [Some((0, 1)); 3], "one row, no columns");
    }
}
