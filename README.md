# mztab-m-rs

A reader, writer and validator for [mzTab-M](https://github.com/HUPO-PSI/mzTab-M),
the HUPO-PSI / MSI exchange format for mass-spectrometry-based metabolomics
results.

> **Status:** feature-complete for 2.1.0-M, with Python and R packages.
> **Nothing is published yet.** What is left is in
> [docs/STATUS.md](docs/STATUS.md).

## Crates

| Crate | What it is |
|---|---|
| [`mztab-m`](crates/mztab-m) | The core: model, TSV reader (streaming), byte-exact writer, three-layer validator, JSON (feature `json`). `no_std`, zero dependencies. |
| [`mztab-m-obo`](crates/mztab-m-obo) | OBO vocabularies (such as `psi-ms.obo`) as term lookups for semantic validation. |
| [`mztab-m-api`](crates/mztab-m-api) | Text-in, text-out operations shared by the command and the bindings. |
| [`mztab-m-cli`](crates/mztab-m-cli) | The `mztab-m` command: `validate`, `fmt`, `convert`, `inspect`. |
| [Python: `mztab-m`](bindings/python) | `import mztab_m`: validate, convert, canonical, inspect (PyO3, one wheel per platform). |
| [R: `mztabm`](bindings/r) | `library(mztabm)`: the same functions, with findings as data frames (extendr). |
| [`mztab-m-wasm`](crates/mztab-m-wasm) | A browser page that validates a dropped file (demo, not published). |

```sh
mztab-m validate --cv psi-ms.obo study.mztab
mztab-m convert --to json -o study.json study.mztab
```

The core's quickstart is in its [README](crates/mztab-m/README.md); the
architecture is in [docs/DESIGN.md](docs/DESIGN.md); the numbers are in
[docs/BENCHMARKS.md](docs/BENCHMARKS.md).

## Guarantees, and what enforces them

Every gate runs as `./mk <gate>`, defined in [`mk`](mk) and
[`mk.local`](mk.local). [CI](.github/workflows/ci.yml) runs them all
(`./mk ci`) on pushes to `main` and on pull requests;
[nightly](.github/workflows/nightly.yml) adds coverage, full mutation
testing and miri.

| Guarantee | Enforced by |
|---|---|
| The core has **zero runtime dependencies** | the core's [`Cargo.toml`](crates/mztab-m/Cargo.toml) has no `[dependencies]`; `deny` ([`deny.toml`](deny.toml)) and `machete` gates |
| The core is `#![no_std]` + `alloc` | the crate attribute (the compiler rejects any use of `std` in it) |
| The core, `mztab-m-obo`, `mztab-m-api` and the demo **build and pass their tests on `wasm32-unknown-unknown`** | the `wasm` gate (`task_wasm` in [`mk.local`](mk.local)), using `wasm-bindgen-test-runner` |
| No `unsafe` code | `unsafe_code = "forbid"` in the workspace lints ([`Cargo.toml`](Cargo.toml)); the `lints-inherit` gate checks every crate inherits them |
| **Pure core:** no I/O, no global state | the core is `no_std` (no file system, no stdin) and has no dependencies; I/O lives in `mztab-m-cli` and the demo only |
| No panics on malformed input | the `lint` gate denies `unwrap_used`, `panic` and `indexing_slicing`; four fuzz targets (`fuzz/`) |
| Every finding code links its rule in the pinned spec | `tests/spec_anchors.rs` checks each anchor exists in the pinned text |
| CV-mapping rules match the spec's mapping file | `tests/cv_rules.rs` regenerates them and fails if the committed file is stale |
| 100% line coverage | the `coverage` gate (floor in [`mk.local`](mk.local)); the `ratchet` gate stops it being lowered |
| No surviving mutants | the `mutants-diff` gate on every pull request; `mutants` nightly |
| Supported Rust version | the `msrv` gate (1.100.0) |

## Specification

The spec is pinned to HUPO-PSI/mzTab-M commit `e66c19e` (mzTab-M **2.1.0-M**,
an unreleased draft); 2.0.0-M files are also read and validated. Every
ambiguity, every deviation from the reference implementation
[jmzTab-m](https://github.com/lifs-tools/jmzTab-m), and every correction of
the spec's own examples or mapping file is recorded in
[DECISIONS.md](DECISIONS.md).

## Development

```sh
git clone --recurse-submodules <repo>   # spec and test corpora live in third_party/
./mk setup                              # pinned toolchain and tools
./mk ci                                 # every blocking gate
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/PLAN.md](docs/PLAN.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Test corpora under `third_party/`
keep their own licenses; see [NOTICE](NOTICE).
