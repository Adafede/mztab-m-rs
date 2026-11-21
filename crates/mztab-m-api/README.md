# mztab-m-api

The operations the `mztab-m` command and the Python, R and browser bindings
share, as plain functions over text: detect the format, read, validate up
to a layer with severities and vocabularies, write canonically (optionally
dropping what the specification does not define), list findings as flat
records, and summarise a document. Built on
[`mztab-m`](https://crates.io/crates/mztab-m) and
[`mztab-m-obo`](https://crates.io/crates/mztab-m-obo); `no_std` + `alloc`.

```rust
use mztab_m_api::{Format, Layer, Settings, check};

let text = b"MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tx\n";
let settings = Settings::default().layer(Layer::Structure);
let records = check(text, Format::Tsv, &[], &settings)?;
assert!(records.iter().any(|r| r.code == "MZT-T009"), "missing fields");
# Ok::<(), mztab_m_api::Error>(())
```
