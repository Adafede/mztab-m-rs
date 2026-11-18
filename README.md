# mztab-m-rs

A reader, writer and validator for [mzTab-M](https://github.com/HUPO-PSI/mzTab-M),
the HUPO-PSI / MSI exchange format for mass-spectrometry-based metabolomics results.

> **Status:** under construction (Phase 2, milestone M0). Nothing is published yet.

## Guarantees (enforced by CI)

| Guarantee | Enforced by |
|---|---|
| Core crate has **zero runtime dependencies** | `cargo deny` + `cargo machete` gates; `Cargo.toml` review |
| Core is `#![no_std]` + `alloc` | crate attribute; `wasm` gate builds with `--no-default-features` |
| Core compiles **and passes its tests** on `wasm32-unknown-unknown` | `./mk wasm` (`mk.local`), via `wasm-bindgen-test-runner` |
| No `unsafe` code | `unsafe_code = "forbid"` in `[workspace.lints.rust]` |
| Pure API: no I/O, no global state in the core | design rule; I/O lives in the CLI adapter only |
| No panics on malformed input | `unwrap_used`/`panic`/`indexing_slicing` lints; fuzzing (nightly) |
| 100% line coverage, zero surviving mutants | `./mk coverage`, `./mk mutants` (ratcheted) |

## Specification

The spec is pinned to HUPO-PSI/mzTab-M commit `e66c19e` (mzTab-M **2.1.0-M**,
an unreleased draft); 2.0.0-M files are also read and validated. Every
ambiguity and every deviation from the reference implementation
[jmzTab-m](https://github.com/lifs-tools/jmzTab-m) is recorded in
[DECISIONS.md](DECISIONS.md).

## Development

```sh
git clone --recurse-submodules <repo>   # test corpora live in third_party/
./mk setup                              # pinned toolchain and tools
./mk ci                                 # every blocking gate
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/PLAN.md](docs/PLAN.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Test corpora under `third_party/`
keep their own licenses; see [NOTICE](NOTICE).
