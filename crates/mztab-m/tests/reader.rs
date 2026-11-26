//! The reader on every TSV example of the pinned specification, and on two
//! hand-written fixtures: one that uses every metadata key and column kind
//! correctly, and one with one problem per line, whose findings are checked
//! exactly against `tests/golden/broken.txt`.
//!
//! Native targets read the whole corpus from `third_party/`. wasm32 has no
//! file system, so it embeds a few small corpus files instead.
#![cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::expect_used,
        reason = "test helpers: a missing corpus file is a setup error, not a finding"
    )
)]

#[cfg(not(target_arch = "wasm32"))]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::{Code, Event, IntoOwned, Location, Parsed, SourceLayout, StreamReader, parse_bytes};

const FULL: &[u8] = include_bytes!("fixtures/full.mztab");
const BROKEN: &[u8] = include_bytes!("fixtures/broken.mztab");
const BROKEN_GOLDEN: &str = include_str!("golden/broken.txt");

/// Reads `input` through a [`StreamReader`] in chunks of `size` bytes.
fn streamed(input: &[u8], size: usize) -> Parsed<'static> {
    let mut events = Vec::new();
    let mut reader = StreamReader::new();
    let mut keep = |event: Event<'_>| events.push(event.into_owned());
    for chunk in input.chunks(size) {
        reader.push(chunk, &mut keep);
    }
    reader.finish(&mut keep);
    events.into_iter().collect()
}

/// Checks one file: every finding is located on a line of the file, every
/// line is accounted for, chunked reading gives the same document, the
/// writer reproduces the file byte for byte (invalid UTF-8 aside), and the
/// model without its layout survives a canonical write and read.
fn check<'a>(name: &str, input: &'a [u8], chunk_sizes: &[usize]) -> Parsed<'a> {
    let parsed = parse_bytes(input);
    let line_bytes = input.strip_suffix(b"\n").unwrap_or(input);
    let lines = if line_bytes.is_empty() {
        0
    } else {
        u32::try_from(line_bytes.split(|&b| b == b'\n').count()).unwrap_or(u32::MAX)
    };
    for finding in &parsed.findings {
        let line = match finding.location {
            Location::Text { line, .. } => line,
            Location::Json(_) | Location::Model(_) | Location::Document => 0,
        };
        assert!(
            (1..=lines).contains(&line),
            "{name}: finding outside the file: {finding}"
        );
    }
    let count = |prefix: &[u8]| {
        input
            .split(|&b| b == b'\n')
            .filter(|line| line.starts_with(prefix) && line.get(3) == Some(&b'\t'))
            .count()
    };
    let mztab = &parsed.mztab;
    let rows = [
        mztab.sml.as_ref().map_or(0, |t| t.rows.len()),
        mztab.smf.as_ref().map_or(0, |t| t.rows.len()),
        mztab.sme.as_ref().map_or(0, |t| t.rows.len()),
    ];
    // A row before its header is kept as an unrecognised line instead.
    if !parsed
        .findings
        .iter()
        .any(|f| f.code == Code::RowBeforeHeader)
    {
        assert_eq!(
            rows,
            [count(b"SML"), count(b"SMF"), count(b"SME")],
            "{name}: every row is read"
        );
    }
    let written = mztab_m::to_string(&parsed.mztab).expect("a read model is writable");
    assert!(
        written == String::from_utf8_lossy(input),
        "{name}: written text differs from the input"
    );
    let has_short_rows = parsed.findings.iter().any(|f| f.code == Code::CellCount);
    if !has_short_rows {
        let mut canonical = parsed.mztab.clone();
        canonical.layout = SourceLayout::default();
        let text = mztab_m::to_string(&canonical).expect("writable");
        let mut again = mztab_m::parse(&text).mztab;
        again.layout = SourceLayout::default();
        assert!(
            again == canonical,
            "{name}: canonical write and read changes the model"
        );
    }
    let owned = parsed.clone().into_owned();
    for &size in chunk_sizes {
        assert!(
            streamed(input, size) == owned,
            "{name}: chunks of {size} bytes differ from a whole read"
        );
    }
    parsed
}

fn rendered(parsed: &Parsed<'_>) -> String {
    parsed.findings.iter().fold(String::new(), |mut out, f| {
        out.push_str(&f.to_string());
        out.push('\n');
        out
    })
}

#[test]
fn full_fixture_reads_without_findings() {
    let parsed = parse_bytes(FULL);
    assert_eq!(rendered(&parsed), "", "no findings");
    let metadata = &parsed.mztab.metadata;
    assert!(
        metadata.unknown.is_empty(),
        "every key placed: {:?}",
        metadata.unknown
    );
    assert_eq!(metadata.colunit_small_molecule.len(), 2, "colunit repeats");
    let rows = |t: Option<usize>| t.unwrap_or_default();
    assert_eq!(
        rows(parsed.mztab.sml.as_ref().map(|t| t.rows.len())),
        2,
        "SML rows"
    );
    assert_eq!(
        rows(parsed.mztab.smf.as_ref().map(|t| t.rows.len())),
        2,
        "SMF rows"
    );
    assert_eq!(
        rows(parsed.mztab.sme.as_ref().map(|t| t.rows.len())),
        2,
        "SME rows"
    );
}

/// Set `BLESS=1` to rewrite the golden file after an intended change.
#[test]
fn broken_fixture_findings_match_golden() {
    let parsed = parse_bytes(BROKEN);
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BLESS").is_some() {
        let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/broken.txt");
        std::fs::write(golden, rendered(&parsed)).expect("write golden");
        return;
    }
    assert_eq!(rendered(&parsed), BROKEN_GOLDEN, "findings");
    assert!(parsed.mztab.layout.bom, "BOM recorded");
}

/// Events collect into a document however they arrive: a verbatim event
/// with no line before it has nothing to attach to and is dropped.
#[test]
fn stray_verbatim_events_are_dropped() {
    let verbatim = mztab_m::Verbatim {
        text: mztab_m::Raw::new("x"),
        fingerprint: 0,
    };
    let parsed: Parsed<'_> = core::iter::once(Event::Verbatim(verbatim)).collect();
    assert!(parsed == parse_bytes(b""), "same as an empty document");
}

#[test]
fn fixtures_round_trip() {
    let _full = check("full.mztab", FULL, &[1, 7]);
    let _broken = check("broken.mztab", BROKEN, &[1, 7]);
}

/// Every placed metadata value occupies its key, and only values of the
/// key's kind are accepted.
#[test]
fn metadata_keys_hold_one_value_of_their_kind() {
    use mztab_m::{Cell, InsertError, MtdValue, Text};
    let mut entries = Vec::new();
    let mut reader = StreamReader::new();
    let mut keep = |event: Event<'_>| {
        if let Event::Metadata(_, entry) = event {
            entries.push(entry.into_owned());
        }
    };
    reader.push(FULL, &mut keep);
    reader.finish(&mut keep);
    let mut metadata = parse_bytes(FULL).into_owned().mztab.metadata;
    let text = || MtdValue::Text(Cell::Value(Text::new("t").expect("valid text")));
    let profile = || MtdValue::Profile(Cell::Null);
    let mut checked = 0;
    for (key, value) in entries.into_iter().filter_map(|e| e.typed) {
        let wrong = if matches!(value, MtdValue::Text(_)) {
            profile()
        } else {
            text()
        };
        assert_eq!(
            metadata.insert(key, wrong),
            Err(InsertError::WrongKind),
            "{key}: wrong kind"
        );
        let again = if key.repeats() {
            Ok(())
        } else {
            Err(InsertError::Occupied)
        };
        assert_eq!(metadata.insert(key, value), again, "{key}: second value");
        checked += 1;
    }
    assert!(
        metadata.insert(
            mztab_m::MtdKey::Software(
                mztab_m::SoftwareId::new(99).expect("non-zero"),
                mztab_m::SoftwareKey::Software,
            ),
            text(),
        ) == Err(InsertError::WrongKind)
            && !metadata
                .software
                .contains_key(mztab_m::SoftwareId::new(99).expect("non-zero")),
        "WrongKind insert must not mutate IndexMap"
    );
    assert_eq!(
        checked, 127,
        "all 78 key forms, one colunit twice, ten more CVs (four keys each), plus ms_run[2] and assay[2]"
    );
}

#[test]
fn keep_spaces_text_and_sub_cell_error_columns() {
    // Spaces::Keep preserves `" null"` and `"   "` with a whitespace warning.
    let padded_null = parse_bytes(b"MTD\ttitle\t null\n");
    assert_eq!(
        padded_null.mztab.metadata.title,
        Some(mztab_m::Cell::Value(
            mztab_m::Text::new(" null").expect("valid")
        ))
    );
    assert_eq!(padded_null.findings.len(), 1);
    assert_eq!(
        padded_null.findings.iter().next().map(|f| f.code),
        Some(Code::CellWhitespace)
    );

    let spaces_only = parse_bytes(b"MTD\ttitle\t   \n");
    assert_eq!(
        spaces_only.mztab.metadata.title,
        Some(mztab_m::Cell::Value(
            mztab_m::Text::new("   ").expect("valid")
        ))
    );
    assert_eq!(
        spaces_only.findings.iter().next().map(|f| f.code),
        Some(Code::CellWhitespace)
    );

    // Sub-cell TextError offsets include prefix (`doi:`, `ms_run[1]:`, colunit column).
    let pub_err = parse_bytes(b"MTD\tpublication[1]\tdoi:abc\rdef\n");
    assert_eq!(
        pub_err.findings.iter().next().map(|f| f.location.clone()),
        Some(Location::text(1, 27).with_cell(3))
    );
    let col_err = parse_bytes(b"MTD\tcolunit-small_molecule\tx\ry=[,,u,]\n");
    assert_eq!(
        col_err.findings.iter().next().map(|f| f.location.clone()),
        Some(Location::text(1, 29).with_cell(3))
    );
    let spec_err = parse_bytes(b"SEH\tspectra_ref\nSME\tms_run[1]:a\rb\n");
    assert_eq!(
        spec_err.findings.iter().next().map(|f| f.location.clone()),
        Some(Location::text(2, 16).with_cell(2))
    );
}

#[test]
fn chunked_reads_match_whole_reads() {
    for input in [FULL, BROKEN] {
        let whole = parse_bytes(input).into_owned();
        for size in 1..=9 {
            assert!(streamed(input, size) == whole, "chunks of {size}");
        }
        assert!(streamed(input, input.len()) == whole, "one chunk");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "support/corpus.rs"]
mod corpus_files;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use alloc::collections::BTreeMap;
    use core::fmt::Write as _;
    use std::path::Path;

    use super::{check, corpus_files};
    use mztab_m::{Code, validate};

    fn name(root: &Path, path: &Path) -> String {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// Every file reads (see [`check`]), and the finding codes per file (the
    /// reader's, the structure validator's and the semantic validator's with
    /// no vocabulary) match `tests/golden/corpus.txt`. Set `BLESS=1` to
    /// rewrite the golden file after an intended change.
    #[test]
    fn every_corpus_file_reads_as_recorded() {
        let (root, files) = corpus_files::files();
        assert_eq!(files.len(), 21, "the pinned corpus has 21 TSV files");
        let mut report = String::new();
        for path in files {
            let input = std::fs::read(&path).expect("readable corpus file");
            let sizes: &[usize] = match input.len() {
                0..65_536 => &[1, 7, 4096],
                65_536..1_048_576 => &[4093],
                _ => &[65_536],
            };
            let rel = name(&root, &path);
            let parsed = check(&rel, &input, sizes);
            let config = validate::Config::default();
            let structure = validate::structure(&parsed.mztab, &config);
            let structure = structure.iter().filter(|f| f.code != Code::InvalidValue);
            let semantic = validate::semantic(&parsed.mztab, &config, &mztab_m::cv::NoTerms);
            let mut codes = BTreeMap::<&str, usize>::new();
            for finding in parsed.findings.iter().chain(structure).chain(&semantic) {
                *codes.entry(finding.code.id()).or_default() += 1;
            }
            write!(report, "{rel}:").expect("writing to a String");
            for (code, n) in codes {
                write!(report, " {code}={n}").expect("writing to a String");
            }
            report.push('\n');
        }
        let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/corpus.txt");
        if std::env::var_os("BLESS").is_some() {
            std::fs::write(&golden, &report).expect("write golden");
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_default();
        assert!(
            expected == report,
            "corpus findings changed; rerun with BLESS=1 if intended:\n{report}"
        );
    }
}

#[cfg(target_arch = "wasm32")]
#[test]
fn embedded_corpus_files_read() {
    const FILES: [(&str, &str); 3] = [
        (
            "2.0/manual_null_null_minimal_example.mztab",
            include_str!(
                "../../../third_party/mzTab-M/examples/2.0/manual_null_null_minimal_example.mztab"
            ),
        ),
        (
            "2.1/manual_null_MTD-only.mztab",
            include_str!("../../../third_party/mzTab-M/examples/2.1/manual_null_MTD-only.mztab"),
        ),
        (
            "2.1/example_study_variable_group.mztab",
            include_str!(
                "../../../third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab"
            ),
        ),
    ];
    for (name, input) in FILES {
        let _findings = check(name, input.as_bytes(), &[1, 7, 4096]);
    }
}
