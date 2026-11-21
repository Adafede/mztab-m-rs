# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
once `1.0.0` is reached (currently pre-`1.0` early development; see [STATUS.md](STATUS.md)).

## [Unreleased]

### Added

- Pure `#![no_std]` + `alloc`, zero-dependency core crate (`mztab-m`) supporting
  whole-document and streaming TSV reading (`parse`, `parse_bytes`, `StreamReader`),
  byte-exact round-trip writing (`write`, `to_string`), three-layer validation
  (`validate::syntax`, `validate::structure`, `validate::semantic`, `validate::all`),
  and optional `json` feature support targeting the pinned HUPO-PSI mzTab-M 2.1 draft
  (`e66c19e`) plus 2.0.0-M compatibility.
- Zero-copy OBO 1.4 term lookup crate (`mztab-m-obo`) with `is_a` and `part_of`
  ancestor traversal and exact-synonym matching.
- Shared text-in/text-out operations crate (`mztab-m-api`) and `mztab-m` CLI
  (`validate`, `fmt`, `convert`, `inspect`).
- Browser validator demo (`mztab-m-wasm`, WebAssembly) with local validation,
  conversion, and a bundled example file.
- Prototype Python (`mztab-m`) and R (`mztabm`) bindings built from source.
- Criterion throughput and dhat heap benchmarks (`benches/`) and `cargo-fuzz`
  targets (`fuzz/`).
- Generated validation findings reference (`docs/findings.md`) from `Code::ALL`,
  mdBook documentation site, and automated GitHub Pages deployment.
