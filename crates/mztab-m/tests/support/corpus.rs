//! Discovers `.mztab` files under the pinned mzTab-M specification corpus.

use std::path::{Path, PathBuf};

/// Returns `(root, files)` for all `.mztab` files under `third_party/mzTab-M/examples`, sorted.
pub(crate) fn files() -> (PathBuf, Vec<PathBuf>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/mzTab-M/examples");
    let mut files = Vec::new();
    let mut dirs = vec![root.clone()];
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
    (root, files)
}
