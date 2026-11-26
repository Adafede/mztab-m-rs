# Golden files

Expected outputs the tests compare against. All are generated; rewrite them
with `BLESS=1` after an intended change and review the diff.

| File | Written by | Regenerate |
|---|---|---|
| `corpus.txt` | `tests/reader.rs` (findings for every spec example) | `BLESS=1 cargo test -p mztab-m --test reader` |
| `broken.txt` | `tests/reader.rs` (findings for `fixtures/broken.mztab`) | same |
| `full.json` | `tests/json.rs` | `BLESS=1 cargo test -p mztab-m --all-features --test json` |
| `json-corpus.txt` | `tests/json.rs` (JSON outcome per spec example) | same |
| `full.canonical.mztab` | `tests/writer.rs` (canonical form of `fixtures/full.mztab`, used as base document across crates) | `BLESS=1 cargo test -p mztab-m --test writer` |
