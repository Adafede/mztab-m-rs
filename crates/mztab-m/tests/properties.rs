//! Property tests for the cell grammars: round trips and totality.
//!
//! Native targets only (DECISIONS D-052).
#![cfg(not(target_arch = "wasm32"))]

extern crate alloc;

use mztab_m::{
    Accession, Decimal, DecimalKind, MtdKey, Param, ParamText, Text, TextError, Version,
};
use mztab_m::{Event, IntoOwned, Parsed, StreamReader, parse_bytes};
use proptest::prelude::*;

/// Lines of the two reader fixtures: realistic material to shuffle.
fn fixture_lines() -> Vec<Vec<u8>> {
    let full: &[u8] = include_bytes!("fixtures/full.mztab");
    let broken: &[u8] = include_bytes!("fixtures/broken.mztab");
    full.split(|&b| b == b'\n')
        .chain(broken.split(|&b| b == b'\n'))
        .map(<[u8]>::to_vec)
        .collect()
}

/// A document: fixture lines in any order and number, mixed with arbitrary
/// bytes, joined by `\n` or `\r\n`.
fn document() -> impl Strategy<Value = Vec<u8>> {
    let line = prop_oneof![
        4 => proptest::sample::select(fixture_lines()),
        1 => proptest::collection::vec(any::<u8>(), 0..12),
    ];
    proptest::collection::vec((line, any::<bool>()), 0..40).prop_map(|lines| {
        let mut out = Vec::new();
        for (line, crlf) in lines {
            out.extend_from_slice(&line);
            out.extend_from_slice(if crlf { b"\r\n" } else { b"\n" });
        }
        out
    })
}

fn streamed(input: &[u8], cuts: &[usize]) -> Parsed<'static> {
    let mut events = Vec::new();
    let mut reader = StreamReader::new();
    let mut keep = |event: Event<'_>| events.push(event.into_owned());
    let mut rest = input;
    for &cut in cuts {
        let (chunk, tail) = rest.split_at(cut.min(rest.len()));
        reader.push(chunk, &mut keep);
        rest = tail;
    }
    reader.push(rest, &mut keep);
    reader.finish(&mut keep);
    events.into_iter().collect()
}

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

/// Every metadata key form; `#N` and `#M` stand for indices.
const KEYS: &[&str] = &[
    "mzTab-version",
    "mzTab-ID",
    "mzTab-profile",
    "title",
    "description",
    "sample_processing[#N]",
    "instrument[#N]-name",
    "instrument[#N]-source",
    "instrument[#N]-analyzer[#M]",
    "instrument[#N]-detector",
    "software[#N]",
    "software[#N]-setting[#M]",
    "publication[#N]",
    "contact[#N]-name",
    "contact[#N]-affiliation",
    "contact[#N]-email",
    "contact[#N]-orcid",
    "uri[#N]",
    "external_study_uri[#N]",
    "quantification_method",
    "sample[#N]",
    "sample[#N]-species[#M]",
    "sample[#N]-tissue[#M]",
    "sample[#N]-cell_type[#M]",
    "sample[#N]-disease[#M]",
    "sample[#N]-description",
    "sample[#N]-custom[#M]",
    "ms_run[#N]-location",
    "ms_run[#N]-instrument_ref",
    "ms_run[#N]-format",
    "ms_run[#N]-id_format",
    "ms_run[#N]-fragmentation_method[#M]",
    "ms_run[#N]-scan_polarity[#M]",
    "ms_run[#N]-hash",
    "ms_run[#N]-hash_method",
    "ms_run[#N]-parameters",
    "ms_run[#N]-usi_identifier",
    "assay[#N]",
    "assay[#N]-custom[#M]",
    "assay[#N]-external_uri",
    "assay[#N]-sample_ref",
    "assay[#N]-ms_run_ref",
    "assay[#N]-protocol_refs",
    "assay[#N]-parameters",
    "study_variable[#N]",
    "study_variable[#N]-assay_refs",
    "study_variable[#N]-ms_run_refs",
    "study_variable[#N]-description",
    "study_variable[#N]-average_function",
    "study_variable[#N]-variation_function",
    "study_variable[#N]-factors",
    "study_variable_group[#N]",
    "study_variable_group[#N]-description",
    "study_variable_group[#N]-type",
    "study_variable_group[#N]-datatype",
    "study_variable_group[#N]-unit",
    "study_variable_group[#N]-study_variable_refs",
    "protocol[#N]-name",
    "protocol[#N]-type",
    "protocol[#N]-description",
    "protocol[#N]-parameters",
    "custom[#N]",
    "cv[#N]-label",
    "cv[#N]-full_name",
    "cv[#N]-version",
    "cv[#N]-uri",
    "database[#N]",
    "database[#N]-prefix",
    "database[#N]-version",
    "database[#N]-uri",
    "derivatization_agent[#N]",
    "small_molecule-quantification_unit",
    "small_molecule_feature-quantification_unit",
    "small_molecule-identification_reliability",
    "id_confidence_measure[#N]",
    "colunit-small_molecule",
    "colunit-small_molecule_feature",
    "colunit-small_molecule_evidence",
];

#[test]
fn every_key_form_is_distinct() {
    let keys: alloc::collections::BTreeSet<_> = KEYS
        .iter()
        .map(|k| MtdKey::parse(&k.replace("#N", "1").replace("#M", "2")))
        .collect();
    assert_eq!(keys.len(), KEYS.len(), "each form parses to its own key");
    assert!(!keys.contains(&None), "every form parses");
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 2_000,
        // Bounded shrinking keeps a failure (under mutation testing, say)
        // from running into the harness timeout.
        max_shrink_time: 2_000,
        ..ProptestConfig::default()
    })]

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

    /// Every key form, with any indices, parses and writes back unchanged;
    /// a zero or non-canonical index, or a case change, is not a key.
    #[test]
    fn metadata_keys_round_trip(form in proptest::sample::select(KEYS), n in 1_u32.., m in 1_u32..) {
        let key = form.replace("#N", &n.to_string()).replace("#M", &m.to_string());
        prop_assert_eq!(MtdKey::parse(&key).map(|k| k.to_string()), Some(key.clone()));
        if form.contains("#N") {
            let zero = form.replace("#N", "0").replace("#M", "1");
            prop_assert_eq!(MtdKey::parse(&zero), None);
            let padded = form.replace("#N", &format!("0{n}")).replace("#M", "1");
            prop_assert_eq!(MtdKey::parse(&padded), None);
        }
        prop_assert_eq!(MtdKey::parse(&key.to_uppercase()), None);
    }

    /// `stream_vs_whole`: any document, cut anywhere, streams to the same
    /// result as a whole read (with or without a final line break).
    #[test]
    fn stream_matches_whole(
        mut input in document(),
        cuts in proptest::collection::vec(0_usize..64, 0..12),
        trim_last in any::<bool>(),
    ) {
        if trim_last {
            input.pop();
        }
        let whole = parse_bytes(&input).into_owned();
        prop_assert!(streamed(&input, &cuts) == whole);
    }
}
