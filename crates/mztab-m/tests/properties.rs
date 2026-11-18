//! Property tests for the cell grammars: round trips and totality.
//!
//! Native targets only (DECISIONS D-052).
#![cfg(not(target_arch = "wasm32"))]

use mztab_m::{Accession, Decimal, DecimalKind, Param, ParamText, Text, TextError, Version};
use proptest::prelude::*;

/// Field text that a parameter can carry: no `"`, tab, CR or LF; non-empty.
/// Commas and spaces are deliberately over-represented.
fn param_text() -> impl Strategy<Value = String> {
    "[ ,a-zA-Z0-9:_.()\\[\\]|=\u{e9}\u{3bc}-]{1,12}"
}

fn param() -> impl Strategy<Value = (Option<(String, String)>, String, Option<String>)> {
    (
        proptest::option::of((param_text(), param_text())),
        param_text(),
        proptest::option::of(param_text()),
    )
}

fn build<'a>(
    cv: Option<&'a (String, String)>,
    name: &'a str,
    value: Option<&'a String>,
) -> Option<Param<'a>> {
    let name = ParamText::new(name).ok()?;
    let value = match value {
        Some(v) => Some(ParamText::new(v).ok()?),
        None => None,
    };
    Some(match cv {
        Some((label, accession)) => Param::Cv {
            label: ParamText::new(label).ok()?,
            accession: Accession::new(ParamText::new(accession).ok()?),
            name,
            value,
        },
        None => Param::User { name, value },
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// `parse(write(p)) == p` for every constructible parameter.
    #[test]
    fn param_round_trips((cv, name, value) in param()) {
        let param = build(cv.as_ref(), &name, value.as_ref());
        prop_assume!(param.is_some());
        let param = param.expect("valid fixture");
        let written = param.to_string();
        prop_assert_eq!(Param::parse(&written), Ok(param), "written as {:?}", written);
    }

    /// The parser is total, and anything it accepts is canonicalised to a
    /// fixed point after one write.
    #[test]
    fn param_parse_is_total_and_canonical(input in "\\[[ ,\"a-z:\t]{0,16}\\]|.{0,20}") {
        if let Ok(param) = Param::parse(&input) {
            let once = param.to_string();
            let reparsed = Param::parse(&once);
            prop_assert_eq!(reparsed.as_ref(), Ok(&param), "reparse of {:?}", once);
            let twice = reparsed.map(|p| p.to_string());
            prop_assert_eq!(twice, Ok(once));
        }
    }

    /// Decimal parsing is total and keeps the lexeme; accepted lexemes
    /// convert without loss of kind.
    #[test]
    fn decimal_parse_is_total(input in "[-+]?[0-9]{0,4}(\\.[0-9]{0,4})?([eE][-+]?[0-9]{0,3})?|.{0,8}") {
        if let Ok(d) = Decimal::parse(&input) {
            prop_assert_eq!(d.as_str(), input.as_str());
            match d.kind() {
                // Finite lexemes may still overflow to infinity (`1e310`).
                DecimalKind::Decimal | DecimalKind::Scientific => prop_assert!(!d.to_f64().is_nan()),
                DecimalKind::NaN => prop_assert!(d.to_f64().is_nan()),
                DecimalKind::Infinity => prop_assert!(d.to_f64().is_infinite()),
            }
        }
    }

    /// Every finite `f64` becomes a conforming decimal that reads back to
    /// the same bits.
    #[test]
    fn decimal_from_f64_round_trips(bits in any::<u64>()) {
        let value = f64::from_bits(bits);
        prop_assume!(value.is_finite());
        let d = Decimal::from_f64(value).expect("valid fixture");
        prop_assert_eq!(d.kind(), DecimalKind::Decimal);
        let reparsed = Decimal::parse(d.as_str()).map(|r| r.to_f64().to_bits());
        prop_assert_eq!(reparsed, Ok(bits));
    }

    /// `Text` accepts exactly the non-empty, non-`null` strings without
    /// tabs or line breaks.
    #[test]
    fn text_accepts_exactly_cell_safe_strings(input in "[a-z\t\r\n ]{0,6}|null") {
        let expected = if input.is_empty() {
            Err(TextError::Empty)
        } else if input == "null" {
            Err(TextError::NullLiteral)
        } else if let Some(offset) = input.find(['\t', '\r', '\n']) {
            Err(TextError::ControlChar { offset })
        } else {
            Ok(())
        };
        prop_assert_eq!(Text::new(&input).map(|_| ()), expected);
    }

    /// Versions print and parse back.
    #[test]
    fn version_round_trips(major in any::<u16>(), minor in any::<u16>(), patch in any::<u16>()) {
        let version = Version { major, minor, patch };
        prop_assert_eq!(Version::parse(&version.to_string()), Ok(version));
    }
}
