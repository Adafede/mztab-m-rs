//! The binary itself: files, standard streams and exit codes.

use std::io::Write as _;
use std::process::{Command, Stdio};

const BASE: &str = include_str!("../../mztab-m/tests/golden/full.canonical.mztab");

fn mztab_m() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mztab-m"))
}

#[test]
fn reads_files_and_standard_input() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let input = dir.join("binary-base.mztab");
    let output = dir.join("binary-base.json");
    std::fs::write(&input, BASE).expect("write input");

    let status = mztab_m()
        .args(["convert", "--to", "json", "-o"])
        .arg(&output)
        .arg(&input)
        .status()
        .expect("runs");
    assert!(status.success(), "convert to a file");
    let json = std::fs::read_to_string(&output).expect("written");
    assert!(json.starts_with("{\"metadata\""), "JSON written");

    let mut child = mztab_m()
        .args(["fmt", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(BASE.as_bytes())
        .expect("piped");
    let out = child.wait_with_output().expect("finishes");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        BASE,
        "canonical on stdout"
    );

    let missing = mztab_m()
        .args(["validate", "/nonexistent/file.mztab"])
        .output()
        .expect("runs");
    assert_eq!(missing.status.code(), Some(3), "exit 3");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("cannot read")
            && String::from_utf8_lossy(&missing.stderr).contains("No such file"),
        "on stderr"
    );

    let unwritable = mztab_m()
        .args(["convert", "--to", "json", "-o", "/nonexistent/dir/x.json"])
        .arg(&input)
        .output()
        .expect("runs");
    assert_eq!(unwritable.status.code(), Some(3), "cannot write");
}
