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

use mztab_m::{Event, Findings, IntoOwned, Location, Parsed, StreamReader, parse_bytes};

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
/// line is accounted for, and chunked reading gives the same document.
fn check(name: &str, input: &[u8], chunk_sizes: &[usize]) -> Findings {
    let parsed = parse_bytes(input);
    let lines = u32::try_from(input.split(|&b| b == b'\n').count()).unwrap_or(u32::MAX);
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
    assert_eq!(
        rows,
        [count(b"SML"), count(b"SMF"), count(b"SME")],
        "{name}: every row is read"
    );
    let owned = parsed.clone().into_owned();
    for &size in chunk_sizes {
        assert!(
            streamed(input, size) == owned,
            "{name}: chunks of {size} bytes differ from a whole read"
        );
    }
    parsed.findings
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
    assert_eq!(checked, 79, "all 78 key forms, one colunit twice");
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
mod native {
    use alloc::collections::BTreeMap;
    use core::fmt::Write as _;
    use std::path::{Path, PathBuf};

    use super::check;

    fn corpus() -> Vec<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/mzTab-M/examples");
        let mut files = Vec::new();
        let mut dirs = vec![root];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("corpus directory (git submodule)") {
                let path = entry.expect("directory entry").path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("mztab"))
                {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }

    fn name(path: &Path) -> String {
        let text = path.to_string_lossy();
        text.split("examples/").last().unwrap_or(&text).to_owned()
    }

    /// Every file reads (see [`check`]), and the finding codes per file match
    /// `tests/golden/corpus.txt`. Set `BLESS=1` to rewrite the golden file
    /// after an intended change.
    #[test]
    fn every_corpus_file_reads_as_recorded() {
        let files = corpus();
        assert_eq!(files.len(), 21, "the pinned corpus has 21 TSV files");
        let mut report = String::new();
        for path in files {
            let input = std::fs::read(&path).expect("readable corpus file");
            // Chunked reads of the large files are slow in debug builds and add
            // nothing that the property tests and the fuzz targets miss.
            let sizes: &[usize] = match input.len() {
                0..65_536 => &[1, 7, 4096],
                65_536..1_048_576 => &[4093],
                _ => &[],
            };
            let findings = check(&name(&path), &input, sizes);
            let mut codes = BTreeMap::<&str, usize>::new();
            for finding in &findings {
                *codes.entry(finding.code.id()).or_default() += 1;
            }
            write!(report, "{}:", name(&path)).expect("writing to a String");
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
