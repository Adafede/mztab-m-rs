# mztab-m

Reader, writer and validator for [mzTab-M](https://github.com/HUPO-PSI/mzTab-M),
the HUPO-PSI / MSI exchange format for mass-spectrometry metabolomics results.

- Implements mzTab-M **2.1.0-M**, a draft pinned to one commit of the
  specification. It also reads and validates **2.0.0-M**.
- `#![no_std]` + `alloc`, **zero dependencies**, no `unsafe`. Builds and
  tests on `wasm32-unknown-unknown`.
- **Pure.** No I/O, no global state: you supply the bytes, and the
  vocabularies if you want them.
- **Never fails, never panics on input.** A malformed document still
  gives a model plus findings, each with a stable code, a severity, a
  location and a link to the spec section that states its rule.
- **Lossless.** Writing what was read reproduces it byte for byte, and
  invalid cells are kept, not dropped.

## Quickstart

Read, validate, write:

```rust
use mztab_m::{cv::NoTerms, parse, to_string, validate};

let text = "MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tEXAMPLE\n";

// Reading never fails: a model and the reader's findings.
let parsed = parse(text);
assert_eq!(parsed.mztab.metadata.id.as_ref().and_then(|c| c.value()).map(|t| t.as_str()), Some("EXAMPLE"));

// All three layers: syntax, structure (fields, profiles, references)
// and semantic (controlled vocabulary; `NoTerms` holds no vocabulary).
let findings = validate::all(text, &validate::Config::default(), &NoTerms);
for finding in &findings {
    // e.g. "error[MZT-T009] document: a mandatory metadata field is missing: `assay[1]`"
    let _line = finding.to_string();
    let _spec = finding.code.spec_ref().to_string(); // URL of the rule
}
assert!(findings.has_errors(), "no assays, study variables or software yet");

// Writing reproduces the input byte for byte.
assert_eq!(to_string(&parsed.mztab)?, text);
# Ok::<(), mztab_m::WriteError>(())
```

Change severities, or switch codes off:

```rust
use mztab_m::{Code, Severity, validate::Config};

let config = Config::default()
    .with_severity(Code::ColumnOrder, Severity::Error)
    .without(Code::CvUnused);
assert_eq!(config.severity_of(Code::CvUnused), None);
```

Stream a large file in chunks, holding only the current line:

```rust
use mztab_m::{Event, StreamReader};

let mut reader = StreamReader::new();
let mut rows = 0;
for chunk in [&b"MTD\tmzTab-version\t2.1.0-M\nSFH\tSMF_ID\nSMF\t1\nSM"[..], b"F\t2\n"] {
    reader.push(chunk, &mut |event| {
        if matches!(event, Event::Smf(..)) {
            rows += 1;
        }
    });
}
reader.finish(&mut |_| {});
assert_eq!(rows, 2);
```

JSON in the shape of the specification's JSON schema (feature `json`):

```rust
# #[cfg(feature = "json")] {
use mztab_m::{json, parse};

let model = parse("MTD\tmzTab-version\t2.1.0-M\n").mztab;
let text = json::to_string(&model)?;
let back = json::parse(&text); // findings located by JSON pointer
assert!(back.findings.is_empty());
# }
# Ok::<(), mztab_m::WriteError>(())
```

To check terms against real vocabularies, parse OBO files with
[`mztab-m-obo`](https://crates.io/crates/mztab-m-obo) and pass them to
`validate::semantic` or `validate::all`. The command-line tool is
[`mztab-m-cli`](https://crates.io/crates/mztab-m-cli).

## Features

| Feature | Default | What it adds |
|---|---|---|
| `json` | off | `json::parse`, `json::write`, `json::write_findings` |

## Where the rules come from

- Every finding code's documentation links the section of the pinned
  specification that states its rule.
- Where the specification is ambiguous, or where this crate deviates from
  the reference implementation [jmzTab-m](https://github.com/lifs-tools/jmzTab-m),
  the choice is recorded in
  [DECISIONS.md](https://github.com/Adafede/mztab-m-rs/blob/main/DECISIONS.md).
- [DESIGN.md](https://github.com/Adafede/mztab-m-rs/blob/main/docs/DESIGN.md)
  explains the architecture.

## License

MIT OR Apache-2.0.
