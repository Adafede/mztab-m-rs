//! Differential testing against jmzTab-m (PLAN §6, DECISIONS D-030).
//!
//! For every committed jmzTab-m output in `tests/oracle/`, the findings of
//! this crate and of jmzTab-m are compared by jmzTab-m code and line. Each
//! disagreement must be explained in `tests/oracle/expected-disagreements.tsv`.
#![cfg(not(target_arch = "wasm32"))]
#![expect(
    clippy::expect_used,
    reason = "test setup: unreadable oracle files are a broken checkout"
)]

extern crate alloc;

use alloc::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mztab_m::cv::NoTerms;
use mztab_m::{Location, validate};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// One row of `expected-disagreements.tsv`.
#[derive(Debug)]
struct Expected {
    file: String,
    code: String,
    line: String,
    direction: String,
    reference: String,
}

impl Expected {
    fn covers(&self, file: &str, code: u16, line: i64, direction: &str) -> bool {
        (self.file == "*" || self.file == file)
            && (self.code == "*" || self.code == code.to_string())
            && (self.line == "*" || self.line == line.to_string())
            && self.direction == direction
    }
}

fn expected() -> Vec<Expected> {
    let text = std::fs::read_to_string(root().join("tests/oracle/expected-disagreements.tsv"))
        .expect("expected-disagreements.tsv");
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            assert_eq!(cells.len(), 6, "six columns: {line:?}");
            let cell = |i: usize| cells.get(i).copied().unwrap_or_default().to_owned();
            Expected {
                file: cell(0),
                code: cell(1),
                line: cell(2),
                direction: cell(3),
                reference: cell(4),
            }
        })
        .collect()
}

/// `[Level-code] line N: message` lines of a jmzTab-m output.
fn jmztab_reports(text: &str) -> BTreeSet<(u16, i64)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix('[')?;
            let (head, rest) = rest.split_once("] line ")?;
            let (_, code) = head.split_once('-')?;
            let (number, _) = rest.split_once(':')?;
            Some((code.parse().ok()?, number.trim().parse().ok()?))
        })
        .collect()
}

#[test]
fn every_reference_names_a_decision() {
    let decisions =
        std::fs::read_to_string(root().join("../../DECISIONS.md")).expect("DECISIONS.md");
    for row in expected() {
        assert!(
            ["jmztab-only", "ours-only"].contains(&row.direction.as_str()),
            "direction {:?}",
            row.direction
        );
        assert!(
            decisions.contains(&format!("| {} |", row.reference))
                || decisions.contains(&format!("### {}:", row.reference)),
            "{} is not in DECISIONS.md",
            row.reference
        );
    }
    assert_eq!(
        jmztab_reports("[Error-1019] line 12: x\n[Warn-2018] line -1: y\nnoise"),
        [(1019, 12), (2018, -1)].into(),
        "parser"
    );
}

#[test]
#[ignore = "no committed jmzTab-m outputs in tests/oracle/ yet; run .github/workflows/oracle.yml"]
fn disagreements_with_jmztab_are_explained() {
    let expected = expected();
    let mut used = vec![false; expected.len()];
    let dir = root().join("tests/oracle");
    let mut outputs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("tests/oracle")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "txt"))
        .collect();
    outputs.sort();
    assert!(
        !outputs.is_empty(),
        "expected at least one jmzTab-m output in tests/oracle/"
    );
    let mut unexplained = Vec::new();
    for output in outputs {
        let name = output
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("utf-8 name")
            .to_owned();
        let relative = name.replace("__", "/");
        let source = [
            root()
                .join("../../third_party/mzTab-M/examples")
                .join(&relative),
            root().join("tests/fixtures").join(&relative),
        ]
        .into_iter()
        .find(|p| p.exists())
        .expect("an oracle output names a corpus file or fixture");
        let input = std::fs::read(&source).expect("source");
        let text = String::from_utf8_lossy(&input);
        let ours: Vec<(Vec<u16>, i64)> =
            validate::all(&text, &validate::Config::default(), &NoTerms)
                .iter()
                .filter(|f| !f.code.jmztab().is_empty())
                .map(|f| {
                    let line = match f.location {
                        Location::Text { line, .. } => i64::from(line),
                        Location::Json(_) | Location::Model(_) | Location::Document => -1,
                    };
                    (f.code.jmztab().to_vec(), line)
                })
                .collect();
        let theirs = jmztab_reports(&std::fs::read_to_string(&output).expect("output"));
        for &(code, line) in &theirs {
            let matched = ours
                .iter()
                .any(|(codes, l)| codes.contains(&code) && (line == -1 || *l == line));
            if !matched {
                if let Some(pos) = expected
                    .iter()
                    .position(|e| e.covers(&name, code, line, "jmztab-only"))
                {
                    if let Some(flag) = used.get_mut(pos) {
                        *flag = true;
                    }
                } else {
                    unexplained.push(format!("{name}\t{code}\t{line}\tjmztab-only"));
                }
            }
        }
        for (codes, line) in &ours {
            let matched = theirs
                .iter()
                .any(|(c, l)| codes.contains(c) && (*l == -1 || l == line));
            if !matched {
                if let Some(pos) = expected.iter().position(|e| {
                    codes
                        .iter()
                        .any(|&c| e.covers(&name, c, *line, "ours-only"))
                }) {
                    if let Some(flag) = used.get_mut(pos) {
                        *flag = true;
                    }
                } else {
                    let joined: Vec<String> = codes.iter().map(ToString::to_string).collect();
                    unexplained.push(format!("{name}\t{}\t{line}\tours-only", joined.join("|")));
                }
            }
        }
    }
    unexplained.sort();
    unexplained.dedup();
    assert!(
        unexplained.is_empty(),
        "{} unexplained disagreements; add rows to expected-disagreements.tsv:\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );
    let unused: Vec<&Expected> = expected
        .iter()
        .zip(&used)
        .filter_map(|(entry, was_used)| (!was_used).then_some(entry))
        .collect();
    assert!(
        unused.is_empty(),
        "{} unused expected-disagreements rows: {unused:?}",
        unused.len()
    );
}
