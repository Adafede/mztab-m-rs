//! The corpus against the real PSI-MS vocabulary (D-077).
//!
//! Marked `#[ignore]` so regular workspace test runs do not report a vacuous
//! pass without `PSI_MS_OBO`. `./mk local:vocab` downloads the pinned
//! `psi-ms.obo`, verifies its SHA-256, and runs this test with `-- --ignored`.
//! The semantic codes per corpus file must match `tests/golden/psi-ms.txt`;
//! set `BLESS=1` to rewrite it.

#![cfg(not(target_arch = "wasm32"))]
#![expect(clippy::expect_used, reason = "test helpers")]

extern crate alloc;

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use std::path::{Path, PathBuf};

use mztab_m_api::{Format, Settings, check};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("corpus directory") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| {
            e.eq_ignore_ascii_case("mztab")
                || (e == "json" && path.to_string_lossy().contains("progenesis"))
        }) {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "requires PSI_MS_OBO; run ./mk local:vocab"]
fn corpus_against_psi_ms() {
    let obo = std::env::var_os("PSI_MS_OBO").expect("set PSI_MS_OBO or run ./mk local:vocab");
    let vocabulary = std::fs::read_to_string(obo).expect("readable OBO");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/mzTab-M/examples");
    let mut paths = Vec::new();
    files(&root, &mut paths);
    paths.sort();
    let mut report = String::new();
    for path in paths {
        let bytes = std::fs::read(&path).expect("readable");
        let name = path
            .strip_prefix(&root)
            .expect("under root")
            .display()
            .to_string();
        let format = Format::sniff(&name, &bytes);
        let records = check(&bytes, format, &[&vocabulary], &Settings::default());
        let mut codes = BTreeMap::<&str, usize>::new();
        for r in records.iter().filter(|r| r.layer == "semantic") {
            *codes.entry(r.code).or_default() += 1;
        }
        write!(report, "{name}:").expect("writing to a String");
        for (code, n) in codes {
            write!(report, " {code}={n}").expect("writing to a String");
        }
        report.push('\n');
    }
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/psi-ms.txt");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&golden, &report).expect("write golden");
    }
    let expected = std::fs::read_to_string(&golden).unwrap_or_default();
    assert!(
        expected == report,
        "findings changed; BLESS=1 if intended:\n{report}"
    );
}
