[![CI Quality Gate](https://github.com/Adafede/mztab-m-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Adafede/mztab-m-rs/actions/workflows/ci.yml)
[![Docs & Demo](https://github.com/Adafede/mztab-m-rs/actions/workflows/pages.yml/badge.svg)](https://adafede.github.io/mztab-m-rs/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)

# mztab-m-rs

Read, write, validate, format, and convert
[mzTab-M](https://github.com/HUPO-PSI/mzTab-M) metabolomics files in pure,
`no_std` Rust, on the command line, in Python/R, or directly in your browser.

**[Documentation](https://adafede.github.io/mztab-m-rs/)** ·
**[Live Browser Demo](https://adafede.github.io/mztab-m-rs/demo/)** ·
**[Findings Reference](https://adafede.github.io/mztab-m-rs/findings.html)** ·
**[Rust API Docs](https://adafede.github.io/mztab-m-rs/api/mztab_m/)**

> **Early development — feedback welcome.**
> `mztab-m-rs` passes on every example file in the specification, but the target
> draft (`2.1.0-M` at `e66c19e`) is unreleased, crates are not yet published on
> crates.io, and several spec interpretations in [DECISIONS.md](DECISIONS.md)
> await community review. See [STATUS.md](STATUS.md) for what is verified today.

## 30-second quickstart

Requires Rust (the pinned toolchain installs via `rust-toolchain.toml` if you use `rustup`):

```sh
git clone --recurse-submodules https://github.com/Adafede/mztab-m-rs
cd mztab-m-rs
cargo install --path crates/mztab-m-cli --locked
mztab-m validate third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
```

Findings go to standard output and the count summary goes to standard error:

```text
warning[MZT-S013] line 65, column 67, cell 3: leading or trailing spaces
...
info[MZT-T011] document: profile inferred from the tables present: profile `M+S`
info[MZT-T016] line 108, column 5, cell 2: field describes a table that is not present: `small_molecule_feature-quantification_unit`
warning[MZT-X001] line 110, column 17, cell 3: CV label is not declared: `MIRIAM`
0 errors, 11 warnings, 2 infos
```

Prefer zero install? Try the
**[Live Browser Demo](https://adafede.github.io/mztab-m-rs/demo/)** —
validation runs locally in WebAssembly so files never leave your computer.

## What you can do

| Command | What it does | Example |
|---|---|---|
| `validate` | Check syntax, structure, profiles, and OBO terms; exit `1` on errors (configurable with `--fail-on`) | `mztab-m validate --cv psi-ms.obo study.mztab` |
| `fmt` | Emit canonical TSV/JSON or check formatting (`--check`) | `mztab-m fmt --check study.mztab` |
| `convert` | Convert between TSV and JSON (add `--lossy` to strip unknown fields) | `mztab-m convert --to json -o study.json study.mztab` |
| `inspect` | Print version, profile, element counts, and table row counts | `mztab-m inspect study.mztab` |

Exit codes: `0` ok, `1` findings at or above `--fail-on` (default: `error`) or
`fmt --check` differs, `2` bad command line, `3` I/O or unwritable output error. Run
`mztab-m --help` for all options. The CLI never makes network requests; pass
local OBO files such as `psi-ms.obo` from
[HUPO-PSI/psi-ms-CV](https://github.com/HUPO-PSI/psi-ms-CV) via `--cv`.

## Why mztab-m-rs

The Java reference implementation is
[jmzTab-m](https://github.com/lifs-tools/jmzTab-m). `mztab-m-rs` provides a
tiny, dependency-free core that embeds anywhere while keeping files lossless:

- **Full-document diagnostics:** never stops at the first error; returns a typed
  model plus every finding with a stable code, location, and spec link.
- **Byte-exact round trips:** writing an unmodified file returns the original bytes.
- **2.1.0-M & 2.0.0-M:** targets the pinned 2.1 draft (`e66c19e`) and validates
  2.0.0-M files under 2.0 rules.

## Libraries and bindings

- **Rust core (`#![no_std]` + `alloc`, zero deps):** [`crates/mztab-m`](crates/mztab-m/README.md)
- **OBO 1.4 term lookup:** [`crates/mztab-m-obo`](crates/mztab-m-obo/README.md)
- **CLI crate:** [`crates/mztab-m-cli`](crates/mztab-m-cli/README.md)
- **Python (`mztab-m` via PyO3, prototype):** [`bindings/python`](bindings/python/README.md)
- **R (`mztabm` via extendr, prototype):** [`bindings/r`](bindings/r/README.md)
- **Browser demo (WebAssembly):** [`crates/mztab-m-wasm`](crates/mztab-m-wasm)

## Contributing and help

Open an issue on the [issue tracker](https://github.com/Adafede/mztab-m-rs/issues)
for bugs, real-world files read differently from jmzTab-m, or feedback on
[DECISIONS.md](DECISIONS.md). See [CONTRIBUTING.md](CONTRIBUTING.md) for local
checks (`./mk ci`, `./mk site`, `./mk bindings`) and [SECURITY.md](SECURITY.md)
for private vulnerability reports.

<details>
<summary>Repository layout and development workflow</summary>

| Path | What it is |
|---|---|
| [`crates/mztab-m`](crates/mztab-m) | Core model, whole/streaming reader, writer, validator, optional `json` feature. |
| [`crates/mztab-m-obo`](crates/mztab-m-obo) | Zero-copy OBO 1.4 vocabulary reader (`TermLookup`). |
| [`crates/mztab-m-cli`](crates/mztab-m-cli) | The `mztab-m` command-line binary. |
| [`crates/mztab-m-api`](crates/mztab-m-api) | Internal text-in/text-out API shared by CLI and bindings. |
| [`crates/mztab-m-wasm`](crates/mztab-m-wasm) | WebAssembly wrapper and browser demo page. |
| [`bindings/`](bindings) | Python (`mztab-m`) and R (`mztabm`) source packages. |
| [`benches/`](benches), [`fuzz/`](fuzz) | Criterion/dhat benchmarks and `cargo-fuzz` targets. |
| [`third_party/`](third_party) | Pinned HUPO-PSI mzTab-M spec and jmzTab-m submodules. |
| [`docs/`](docs) | [Site source](docs/site), [findings reference](docs/findings.md), [design](docs/DESIGN.md), [benchmarks](docs/BENCHMARKS.md), [debt ledger](docs/DEBT.md). |
| [`DECISIONS.md`](DECISIONS.md) | Recorded spec ambiguities and jmzTab-m deviations. |

Most code and documents were written with AI coding agents directed by the
maintainer; checks described in [STATUS.md](STATUS.md) were actually run, and
agent-authored internal docs carry a provenance header ([AGENTS.md](AGENTS.md)).

</details>

## License and citation

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT License](LICENSE-MIT) at your option. Submodules under `third_party/` keep
their own licenses (CC-BY-4.0 for the specification, Apache-2.0 for jmzTab-m);
see [NOTICE](NOTICE).

See [CITATION.cff](CITATION.cff) for citation metadata. The validation rules
come from the HUPO-PSI / MSI mzTab-M specification, and
[jmzTab-m](https://github.com/lifs-tools/jmzTab-m) by Nils Hoffmann and
contributors served as the behavioural reference.
