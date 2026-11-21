# Status

Last checked on 2026-11-21 on branch `main`. `./mk ci`, `./mk site`,
`./mk coverage`, and `./mk bindings` all pass locally, and GitHub Actions
(`CI Quality Gate`, `Bindings`, and `Docs and Demo Site`) pass on
`ubuntu-latest`. Everything described as verified was run on that date; the
commands are in [Verified how](#verified-how).

**In one line:** the Rust library and the `mztab-m` command work on every
example file of the specification, with a large test suite and passing CI on
Linux, but nothing is published, the specification itself is an unreleased
draft, and most of the choices about how to read that draft have not been
reviewed by anyone outside the project.

## Components

Labels: **Stable** (rely on it) · **Working** (works in verified scenarios,
tests exist, API and output may change) · **Working, needs rework** ·
**Prototype** · **Experimental** · **Broken / Known issues** · **Planned**.
Nothing here is Stable yet.

| Component | Status | Verified, and how | Known limitations | Next level needs |
|---|---|---|---|---|
| Core TSV reader (`mztab_m::parse`, `parse_bytes`) | Working | Workspace tests (154 unit/integration passed, 2 ignored, and 94 doc tests passed), including findings for all 23 spec example files pinned in a golden file; property tests; wasm32 tests | Rules follow a draft spec; 9 interpretation questions are OPEN in [DECISIONS.md](DECISIONS.md) | Outside review of the decisions; a released 2.1 spec; real-world files beyond the spec examples |
| Streaming reader (`StreamReader`) | Working | Tests and a property test that it equals a whole read; a 1 GB synthetic file was streamed on 2026-11-18 ([BENCHMARKS.md](docs/BENCHMARKS.md)) | Not re-run since | Same as above |
| Writer (canonical form, byte-exact round trip) | Working | Round-trip tests and properties; `mztab-m fmt --check` run on a spec example | Non-string cells with spaces around them are not written back byte-exact (D-056) | Same as above |
| Structure validator | Working | Tests per rule and per finding code; each code links an anchor that exists in the pinned spec (tested) | 37 decisions behind the rules are PROPOSED, 9 OPEN | Review of DECISIONS.md; comparison with jmzTab-m (see below) |
| Semantic validator with PSI-MS | Working | CV-mapping rules regenerated from the spec's mapping file in a test; all spec examples checked against PSI-MS 4.2.17 (`./mk local:vocab`, golden `psi-ms.txt`) | Vocabulary test is marked `#[ignore]` and runs only via `./mk local:vocab` (nightly gate) | — |
| Semantic validator with other vocabularies (UO, OBI, NCBITaxon, …) | Prototype | Terms are checked for form and declaration only | Never run against the real files | Download and pin them like PSI-MS |
| JSON reading and writing (feature `json`) | Working, needs rework | Tests, golden `full.json`, outcome for every spec example | Goes through TSV text (D-071), so it is 3–8× slower than TSV and cannot keep what TSV cannot hold (e.g. where JSON comments were); follows a draft JSON schema | A direct JSON ↔ model path |
| `mztab-m-obo` (OBO term lookup) | Working | Own tests; used for the PSI-MS run | Reads what lookups need, not the whole OBO 1.4 format | — |
| `mztab-m-api` | Working (internal) | Tests; used by the command and both bindings | An internal layer, not a public API | — |
| `mztab-m` command | Working | Binary tests; installed with `cargo install --path` from a fresh clone; `validate`, `validate --cv`, `convert --to json`, `inspect`, `fmt --check` run by hand on a spec example | Only installable from source | Publish |
| Python package (`mztab_m`) | Prototype | Wheel built with maturin 1.16.0 on macOS arm64 (Python 3.15, abi3-py3.9) and Linux `x86_64` in GitHub Actions (`Bindings`); 7 tests and doctests pass | Not on PyPI; no wheel built for Windows | CI matrix on macOS/Windows; publish |
| R package (`mztabm`) | Prototype | `R CMD INSTALL` on R 4.6.2 (macOS arm64) and Linux `x86_64` in GitHub Actions (`Bindings`); 27 testthat checks pass | Not on CRAN; CRAN needs the Rust sources vendored (D-076); Windows untested | Vendoring; `R CMD check` |
| Browser demo (`crates/mztab-m-wasm`) | Prototype | The wasm module builds (`./mk local:demo`), its 3 tests pass natively and on wasm32, and the accessible page includes a bundled example, multi-OBO support, and a client-side privacy note; live at [adafede.github.io/mztab-m-rs/demo/](https://adafede.github.io/mztab-m-rs/demo/) | Not yet tested across multiple browsers on the live URL | Cross-browser testing on the live URL |
| Docs site & findings reference (`docs/site/`, `docs/findings.md`) | Working | `./mk site` builds mdBook + WASM demo + rustdoc with `-D warnings`, verifies `docs/findings.md` matches `Code::ALL`, and passes `lychee --offline` with 0 errors; deployed via GitHub Pages at [adafede.github.io/mztab-m-rs/](https://adafede.github.io/mztab-m-rs/) | — | — |
| Benchmarks (`benches/`) | Working | One run on one laptop, 2026-11-18; builds (`cargo bench --no-run`) and checks in `./mk ci` | Generator changed after the numbers were taken (noted in BENCHMARKS.md) | Re-run, more machines |
| Fuzz targets (`fuzz/`) | unverified | `cargo check --manifest-path fuzz/Cargo.toml` passes in `./mk ci`; the JSON target found three bugs during development (D-071) | Fuzzing itself needs nightly Rust | Run in CI nightly |
| Comparison with jmzTab-m (`tests/oracle.rs`) | Broken / Known issues | — | **No outputs yet**: there are no jmzTab-m outputs in `crates/mztab-m/tests/oracle/`; the comparison test is marked `#[ignore]` so it does not pass vacuously. The 18 rows in `expected-disagreements.tsv` are predictions | Run `.github/workflows/oracle.yml` (or jmzTab-m locally), commit the outputs, explain each disagreement |
| GitHub Actions workflows | Working | Pinned by SHA; the `actions` gate (`zizmor`) passes; `ci.yml`, `bindings.yml`, and `pages.yml` pass on `ubuntu-latest` | `nightly.yml`, `oracle.yml`, and `release.yml` have not run on GitHub yet | Run `nightly.yml` and `oracle.yml` on GitHub |
| Publishing to crates.io, PyPI, CRAN | Planned | `cargo publish --dry-run` passed at M9 (D-074), not re-run | — | Maintainer decision |

## What you can rely on today

- Reading any of the spec's example files, TSV or JSON, without a panic, and
  getting a model plus findings with line, column and a link to the spec rule.
- `write(read(file)) == file` for those files (except non-string cells padded
  with spaces, D-056), and a stable canonical form
  (`mztab-m fmt`).
- The core crate has no dependencies, no `unsafe`, and no I/O.
- The command's exit codes (0 ok, 1 findings, 2 bad command line, 3 I/O).

## What you should not rely on yet

- **Which findings you get, and their severities.** They follow this
  project's reading of a draft spec. Finding codes, messages and severities
  may change.
- **Agreement with jmzTab-m.** It has not been measured.
- **The Rust API.** Version 0.1.0, nothing published, no semver promise.
- **The JSON shape**, which follows the draft JSON schema.
- **The Python and R packages** on Windows (untested).

## Known issues and sharp edges

Draft issues for these and other open gaps are in
[docs/proposed-issues.md](docs/proposed-issues.md) for the
[issue tracker](https://github.com/Adafede/mztab-m-rs/issues).

- The jmzTab-m comparison test has no committed outputs yet and is marked
  `#[ignore]` (above).
- `mutants.out/` from 2026-11-18 (before M5) listed 21 missed mutants. Every
  milestone was reported mutant-free on its own diff afterwards, but a full
  mutation run has not been done since. Treat "no surviving mutants" as
  unverified.

## Tests, coverage and CI

| What | Result (2026-11-21) |
|---|---|
| Workspace tests (`./mk test`) | 154 unit/integration tests passed, 2 ignored (`oracle` and PSI-MS `vocabulary`), and 94 doc tests passed |
| Line coverage (`./mk coverage`, cargo-llvm-cov) | 100% (7835 of 7835 lines) of the five workspace crates; bindings, benches and fuzz targets are not measured; branch coverage is not measured |
| wasm32 tests (`./mk wasm`) | 129 tests pass across `mztab-m`, `mztab-m-obo`, `mztab-m-api`, `mztab-m-wasm` |
| Full PR gate (`./mk ci`) | pass (lints, format, typos, licenses, audit, machete, MSRV, duplication, stale refs, wasm, demo, fuzz/bench check, mutants-diff) |
| Docs site & link check (`./mk site`) | pass (findings freshness, mdBook, WASM demo, rustdoc `-D warnings`, `lychee --offline` 0 errors) |
| Python / R package tests (`./mk bindings`) | pass locally on macOS arm64 and in CI on Linux `x86_64` (Python 7 tests + doctests; R 27 testthat checks) |
| Full mutation run, miri, feature powerset, fuzzing | not run |
| CI on GitHub | `CI Quality Gate`, `Bindings`, and `Docs and Demo Site` pass on `ubuntu-latest`; `Nightly`, `Oracle`, and `Release` not yet run |

## Platforms and toolchains tested

- macOS 27.2.1 on Apple Silicon (arm64)
- Linux (`ubuntu-latest`, `x86_64`) via GitHub Actions (`ci.yml`, `bindings.yml`, `pages.yml`)
- Rust 1.100.0 (pinned in `rust-toolchain.toml`; also the MSRV),
  target `wasm32-unknown-unknown` through `wasm-bindgen-test-runner` 0.2.130
- Python 3.15 with maturin 1.16.0 (abi3-py3.9 wheel)
- R 4.6.2 with testthat

Windows has not been tested.

## Verified how

```sh
./mk ci
./mk site
./mk coverage
./mk bindings
cargo bench -p mztab-m-bench --bench throughput --no-run --locked
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly-2026-11-03 doc -p mztab-m --all-features --no-deps --locked
cargo build -p mztab-m-cli --release
target/release/mztab-m validate third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
target/release/mztab-m validate --cv target/vocab/psi-ms-v4.2.17.obo third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
target/release/mztab-m convert --to json -o study.json third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
target/release/mztab-m inspect third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
```
