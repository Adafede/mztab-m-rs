# mztab-m-api

**Internal.** This crate exists so the command and the bindings share one
implementation. It is not meant to be used directly and has no stability
promise.

The operations the `mztab-m` command and the Python, R and browser bindings
share, as plain functions over text: detect the format, read, validate up
to a layer with severities and vocabularies, write canonically (optionally
dropping what the specification does not define), list findings as flat
records, and summarise a document. Built on
[`mztab-m`](https://github.com/Adafede/mztab-m-rs/tree/main/crates/mztab-m) and
[`mztab-m-obo`](https://github.com/Adafede/mztab-m-rs/tree/main/crates/mztab-m-obo);
`no_std` + `alloc`.

## Quickstart

One-shot check returning flat finding records:

```rust
use mztab_m_api::{Format, Layer, Settings, check};

let text = b"MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tx\n";
let settings = Settings::default().layer(Layer::Structure);
let records = check(text, Format::Tsv, &[], &settings);
assert!(records.iter().any(|r| r.code == "MZT-T009"), "missing fields");
```

Read, validate, and write as canonical JSON:

```rust
use mztab_m_api::{Format, Settings, read, validate, write};

let tsv = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tAPI-1\n";
let parsed = read(tsv.as_bytes(), Format::Tsv);
let findings = validate(&parsed, &Settings::default(), &[]);
assert!(findings.has_errors(), "mandatory fields missing");
let (json_text, _) = write(&parsed.mztab, Format::Json, false)?;
assert!(json_text.contains("\"mzTab-ID\":\"API-1\""));
# Ok::<(), mztab_m_api::Error>(())
```
