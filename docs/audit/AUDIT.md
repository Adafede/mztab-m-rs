> Provenance: authored by an AI coding agent during the structural refactor.

# Phase 0 Audit (`mztab-m-rs`)

Baseline measurements and structural smell catalogue recorded on branch `refactor/architecture` (forked from `main` at `cfee822`).

## 1. Workspace & Target Map

| Crate / Package | Kind | Target / Env | Direct Workspace Deps | External Runtime Deps | Role |
|---|---|---|---|---|---|
| `crates/mztab-m` | lib (`#![no_std]` + `alloc`) | native, `wasm32` | — | none | Pure domain model, TSV/JSON codecs, three-layer validator |
| `crates/mztab-m-obo` | lib (`#![no_std]` + `alloc`) | native, `wasm32` | `mztab-m` | none | Zero-copy OBO 1.4 parser implementing `TermLookup` |
| `crates/mztab-m-api` | lib (`#![no_std]` + `alloc`) | native, `wasm32` | `mztab-m` (`json`), `mztab-m-obo` | none | Shared text-in/text-out application facade |
| `crates/mztab-m-cli` | lib + bin (`mztab-m`) | native | `mztab-m`, `mztab-m-api`, `mztab-m-obo` | none | CLI arg parser, `Io` trait, and `std` entrypoint |
| `crates/mztab-m-wasm` | cdylib + rlib | `wasm32` | `mztab-m`, `mztab-m-api`, `mztab-m-obo` | `wasm-bindgen = 0.2.130` | Browser demo wrapper |
| `benches` (`mztab-m-bench`) | lib + benches (excluded publish) | native | `mztab-m` (`dev-dep`) | `criterion`, `dhat`, `memchr` (`dev-dep`) | Synthetic generator, throughput & heap benchmarks |
| `bindings/python` | cdylib (`_native`) | native (outside workspace) | `mztab-m`, `mztab-m-api`, `mztab-m-obo` | `pyo3 = 0.30` (+ `pyo3-build-config` in `build.rs`) | Python extension module |
| `bindings/r/src/rust` | staticlib (`mztabm`) | native (outside workspace) | `mztab-m`, `mztab-m-api` | `extendr-api = 0.9` | R extension module |
| `fuzz` | fuzz bins | nightly + libFuzzer (outside workspace) | `mztab-m` (`json`) | `libfuzzer-sys` | 4 fuzz targets (`parse_bytes`, `stream_vs_whole`, `roundtrip`, `json`) |

- **Feature flags:** Only `mztab-m` defines an optional feature: `json` (off by default).
- **Duplicate dependency versions (`cargo tree -d`):** `syn` (`2.0.119` via `criterion` -> `zerocopy-derive` in `benches` only; `3.0.10` via `serde_derive` and `wasm-bindgen-macro-support`). Zero duplicate runtime dependencies in published crates.
- **Unused dependencies (`cargo machete`) / Advisories (`cargo deny check`):** 0 unused dependencies; advisories, bans, licenses, and sources all pass.
- **Declarative macros (`macro_rules!`):** `spec_commit!` (`lib.rs`), `sub_keys!` / `expect_kind!` (`meta.rs`), `stable_columns!` / `row_ids!` / `strip!` (`model.rs`), `spec_path!` / `codes!` (`diag.rs`), `ids!` (`id.rs`), `owned_copy!` / `owned_struct!` (`owned.rs`), `table_row!` (`table.rs`), `render_display!` (`write.rs`), `gap!` (`validate/metadata.rs`).

## 2. Quantitative Baseline Metrics

- **Total Rust LOC (`tokei` across `crates`, `benches`, `bindings`, `fuzz`):** 18,788 code+comment+blank (17,448 code, 110 comments, 1,230 blanks; plus 2,944 doc-markdown lines).
- **Files > 300 production LOC (`#[cfg(test)]` excluded):** 24 files in `crates/`, led by:
  - `crates/mztab-m/src/meta.rs`: 1,305 / 1,305
  - `crates/mztab-m/src/model.rs`: 1,029 / 1,214
  - `crates/mztab-m/src/diag.rs`: 839 / 1,031
  - `crates/mztab-m/src/validate/tables.rs`: 825 / 852
  - `crates/mztab-m/src/write.rs`: 794 / 794
  - `crates/mztab-m/src/json/read.rs`: 768 / 785
  - `crates/mztab-m/src/validate/metadata.rs`: 665 / 736
  - `crates/mztab-m/src/vocab.rs`: 659 / 849
  - `crates/mztab-m/src/json/write.rs`: 656 / 656
  - `crates/mztab-m/src/read.rs`: 615 / 715
  - `crates/mztab-m/src/value.rs`: 583 / 747
  - `crates/mztab-m/src/param.rs`: 512 / 719
  - `crates/mztab-m/src/validate/semantic.rs`: 476 / 505
  - `crates/mztab-m-api/src/lib.rs`: 423 / 423
  - `crates/mztab-m/src/json/schema.rs`: 389 / 389
  - `crates/mztab-m/src/typing.rs`: 382 / 413
  - `crates/mztab-m/src/validate.rs`: 381 / 381
  - `crates/mztab-m/src/owned.rs`: 353 / 399
  - `crates/mztab-m/src/table.rs`: 353 / 389
  - `crates/mztab-m/src/json/value.rs`: 341 / 341
  - `crates/mztab-m-cli/src/lib.rs`: 339 / 339
  - `crates/mztab-m/src/cv.rs`: 327 / 327
  - `crates/mztab-m/src/cv/rules.rs`: 310 / 310 (generated)
  - `crates/mztab-m/src/id.rs`: 306 / 376
- **Top functions by LOC:** `Metadata::entries` (189 LOC, `meta.rs:1017`), `mandatory` (185 LOC, `validate/metadata.rs:119`), `Metadata::insert` (125 LOC, `meta.rs:744`), `sml_rows` (115 LOC, `validate/tables.rs:389`), `sme_rows` (102 LOC, `validate/tables.rs:585`), `check_shape` (97 LOC, 6 params, `validate/tables.rs:115`), `mtd_value` (88 LOC, `typing.rs:294`), `table_lines` (80 LOC, `json/read.rs:541`), `kind` (80 LOC, `meta.rs:489`), `smf_rows` (79 LOC, `validate/tables.rs:505`), `Options::parse` (78 LOC, `mztab-m-cli/src/lib.rs:156`), `uses` (78 LOC, `validate/semantic.rs:85`).
- **Functions > 4 params (excluding `&self`):** `check_shape` (6), `row_cell` (6), `bindings/python::validate` (6, PyO3 signature), `row_line` (6), `table_shape` (6).
- **Clippy cognitive complexity (> 25):** `crates/mztab-m/src/validate/metadata.rs:119` `mandatory` (35/25).
- **Lint exceptions in production library code (`crates/`):** 6 in `mztab-m` (`id.rs:265`, `meta.rs:740`, `meta.rs:1013`, `read.rs:43`, `validate/metadata.rs:115`, `validate/tables.rs:385`), 1 in `mztab-m-wasm` (`lib.rs:22`).
- **Build timings & binary sizes:**
  - `cargo clippy --workspace --all-features --lib --bins`: 48.46s (clean check).
  - `cargo build --release -p mztab-m-cli`: 28.93s (clean release); binary size **1.6 MiB** (`953.8 KiB` `.text`: `620.5 KiB` `mztab_m`, `298.3 KiB` `std`, `13.2 KiB` `mztab_m_api`, `12.3 KiB` `mztab_m_cli`, `10.6 KiB` `mztab_m_obo`).
  - `cargo build -p mztab-m-wasm --target wasm32-unknown-unknown --release`: 50.54s; WASM size **967 KiB**.
- **Criterion `--quick` baseline throughput (same machine, Apple Silicon, 2026-11-24):**
  - `small`: `parse` 11.36 MiB/s (530.3 µs), `write` 36.46 MiB/s (165.2 µs), `structure` 46.46 MiB/s (129.6 µs), `semantic` 69.35 MiB/s (86.8 µs), `json_write` 5.31 MiB/s (1.135 ms), `json_parse` 6.53 MiB/s (921.8 µs).
  - `medium`: `parse` 15.72 MiB/s (70.85 ms), `write` 220.47 MiB/s (5.05 ms), `structure` 38.99 MiB/s (28.56 ms), `semantic` 199.26 MiB/s (5.59 ms), `json_write` 5.79 MiB/s (192.3 ms), `json_parse` 2.89 MiB/s (384.7 ms).
  - `large`: `parse` 32.32 MiB/s (400.4 ms), `write` 149.85 MiB/s (86.37 ms), `structure` 424.48 MiB/s (30.49 ms), `semantic` 47.29 GiB/s (267.3 µs), `json_write` 21.69 MiB/s (596.7 ms), `json_parse` 14.21 MiB/s (910.8 ms).

## 3. Architectural Smells Ranked by Impact x Effort

### Rank 1 (High Impact, Medium Effort) — Hot-Path Allocations & Quadratic Scans in JSON, Validation, and Writer
- `crates/mztab-m/src/json/write.rs:80` and `crates/mztab-m-api/src/lib.rs:234`: Both `json::write` and `mztab_m_api::write` unconditionally clone the entire `MzTab` document (`mztab.clone()`) solely to clear `layout` before calling canonical TSV writing.
- `crates/mztab-m/src/json/write.rs:26-69, 566-618`: `json::write` parses canonical TSV back into a full in-memory DOM (`enum J`), allocates a `BTreeMap<Cow<'static, str>, J>` and `Vec<Cow<'static, str>>` per row, and allocates lowercase `String` keys via `schema::key(name)` (`crates/mztab-m/src/json/schema.rs:379`) on every cell.
- `crates/mztab-m/src/json/read.rs:639-644`: `row_cell` allocates `schema::key(name)` (`String`) and executes `row.get(&key)` (linear scan over `Value::Object` members) on every single cell of every row (`O(K^2)` per row).
- `crates/mztab-m/src/write.rs:615`: `table_row` calls `slots(&t.header)`, allocating a fresh `Vec<Slot<R::Column>>` on every row written.
- `crates/mztab-m/src/validate/tables.rs:236-265, 375`: `Rows` runs a linear scan `position(header, column)` (~20 times per row) instead of precomputing column indices once per table; `refs()` allocates a `Vec` per row; `native_id_ok` (`crates/mztab-m/src/validate/tables.rs:779`) allocates `Vec<&str>` per check.
- `crates/mztab-m/src/validate/semantic.rs:70-72, 410-454`: Iterates all 31 rules over all uses (`O(rules * uses)`) and allocates two temporary `Vec`s (`judged`, `hits`) on every CV parameter use.
- `crates/mztab-m-obo/src/lib.rs:206-209`: `is_descendant` clones `e.parents` (`Vec<&str>`) on every call.

### Rank 2 (High Impact, Medium Effort) — God-Files & Internal Circular Module Dependencies
- **God-files:** 14 production source files exceed 300 LOC (`meta.rs` 1,305; `model.rs` 1,029; `diag.rs` 839; `validate/tables.rs` 825; `write.rs` 794; `json/read.rs` 768; `validate/metadata.rs` 665; `vocab.rs` 659; `json/write.rs` 656; `read.rs` 615; `value.rs` 583; `param.rs` 512; `validate/semantic.rs` 476; `mztab-m-api/src/lib.rs` 423).
- **Circular module dependencies inside `mztab-m`:**
  - `meta.rs:22` imports `model::{ColUnit, Field, List, Metadata, parse_index}`, while `model.rs:129` imports `meta::MtdKey`.
  - `table.rs:15` imports `write::{Out, Render, WriteError}`, while `write.rs:39` imports `table::{Slot, TableRow}`.

### Rank 3 (Medium-High Impact, Low-Medium Effort) — Layering Leaks Across Adapter Crates
- `crates/mztab-m-cli/Cargo.toml:28-30`, `crates/mztab-m-wasm/Cargo.toml:15-17`, `bindings/python/Cargo.toml:15-17`, and `bindings/r/src/rust/Cargo.toml:16-17` bypass the application facade (`mztab-m-api`) to depend directly on `mztab-m` and `mztab-m-obo` (for `Ontology::parse`, `json::write_findings`, `Code::ALL`, `SPEC_COMMIT`, `Stripped`, `Severity`).
- `crates/mztab-m-api/src/lib.rs:355-375` (`Fact::to_json`) duplicates JSON string escaping logic already implemented in `crates/mztab-m/src/json/value.rs:327-341` (`write_string`).

### Rank 4 (Medium Impact, Low-Medium Effort) — Type-Driven Smells & Ad-Hoc Parsing
- `crates/mztab-m/src/vocab.rs:410-430` (`Section::pick` / `pick_mut`), `crates/mztab-m/src/write.rs:520-590`, and `crates/mztab-m/src/validate.rs:211-217` use `Section` (4 variants including `Mtd`) in table-only contexts, silently aliasing `Section::Mtd | Section::Sml => sml`. A dedicated `TableSection` (`Sml`, `Smf`, `Sme`) eliminates invalid states.
- `crates/mztab-m-cli/src/lib.rs:136-260` uses a flat stringly-typed `Options { command: String, to: Option<Format>, ... }` god-struct with `_ =>` catch-all match arms instead of a per-subcommand `Command` enum.
- Domain and API value types expose ad-hoc `parse` methods (`Version::parse`, `Profile::parse`, `AmbiguityCode::parse`, `Datatype::parse`, `MtdKey::parse`, `mztab_m_api::Format::parse`, `mztab_m_api::Layer::parse`) without implementing `core::str::FromStr`.

### Rank 5 (Medium Impact, Low Effort) — Complexity Hotspots, Lint Suppressions, & Release Profile
- `validate/metadata.rs:119` (`mandatory`, 185 LOC) triggers `clippy::cognitive_complexity` (35/25) and `clippy::too_many_lines`.
- Four `#[expect(clippy::too_many_lines)]` suppressions (`meta.rs:740`, `meta.rs:1013`, `validate/metadata.rs:115`, `validate/tables.rs:385`) exist solely because functions were not decomposed into clean per-section/per-family helpers.
- Root `Cargo.toml` defines `[profile.dev.package."*"]` and `[profile.test.package."*"]` but omits `[profile.release]` tuning (`lto`, `codegen-units`, `strip`), leaving the WASM artifact at 967 KiB and CLI binary at 1.6 MiB.
