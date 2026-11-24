> Provenance: authored by an AI coding agent during the structural refactor.

# Refactor Session Log

## Step 0 & Step 1 — Phase 0 Recon & Phase 1 Target Architecture

- **Branch:** `refactor/architecture` from `main` (`cfee822`)
- **Files added:** `docs/audit/AUDIT.md`, `docs/ARCHITECTURE.md`, `docs/REFACTOR_PLAN.md`, `SESSION_LOG.md`
- **Baseline metrics recorded:**
  - Rust LOC (`crates`, `benches`, `bindings`, `fuzz`): 18,788 (`tokei`: 17,448 code); max production file: `crates/mztab-m/src/meta.rs` (1,305 LOC); max function: `Metadata::entries` (189 LOC); cognitive complexity warnings: 1 (`validate/metadata.rs::mandatory`, 35/25); library `#[expect]` suppressions: 7 total (6 in `mztab-m`, 1 in `mztab-m-wasm`).
  - Release sizes & build time: CLI binary `1.6 MiB` (`953.8 KiB` `.text`, clean release build `28.93s`), WASM binary `967 KiB` (clean release build `50.54s`).
  - Criterion `--quick` median throughput (`small` / `medium` / `large`):
    - `parse`: `11.36` / `15.72` / `32.32` MiB/s (`530.3 µs` / `70.85 ms` / `400.4 ms`)
    - `write`: `36.46` / `220.47` / `149.85` MiB/s (`165.2 µs` / `5.05 ms` / `86.37 ms`)
    - `structure`: `46.46` / `38.99` / `424.48` MiB/s (`129.6 µs` / `28.56 ms` / `30.49 ms`)
    - `semantic`: `69.35` / `199.26` / `47,289` MiB/s (`86.8 µs` / `5.59 ms` / `267.3 µs`)
    - `json_write`: `5.31` / `5.79` / `21.69` MiB/s (`1.135 ms` / `192.3 ms` / `596.7 ms`)
    - `json_parse`: `6.53` / `2.89` / `14.21` MiB/s (`921.8 µs` / `384.7 ms` / `910.8 ms`)
- **Decisions:** Preserved `mztab-m`'s zero-dependency `#![no_std]` rule and JSON-through-TSV conformance semantics (`DECISIONS.md` D-071) while targeting elimination of full-model clones, per-row DOM/map allocations, and quadratic scans.

## Step 2 — Pin Characterization Tests

- **Files touched:** `crates/mztab-m/tests/writer.rs`, `crates/mztab-m-api/tests/api.rs`, `crates/mztab-m-cli/tests/run.rs`
- **Tests added:**
  - `unwritable_errors_report_exact_line_numbers` (`writer.rs`)
  - `write_preserves_comments_for_tsv_and_handles_lossy` (`api.rs`)
  - `flag_combinations_and_missing_values` (`run.rs`)
- **Verification:** `./mk ci-fast` passed all gates except `fmt` (fixed with `cargo fmt` and re-verified `fmt` + targeted tests).

## Step 3 — Split Core Primitives & Introduce `TableSection`

- **Files touched / created:**
  - `crates/mztab-m/src/id.rs`: moved `parse_index` here from `model.rs`.
  - `crates/mztab-m/src/value.rs` -> split into `value.rs`, `value/cell.rs`, `value/text.rs`, `value/decimal.rs` (max prod file ~200 LOC).
  - `crates/mztab-m/src/param.rs` -> split into `param.rs`, `param/text.rs`, `param/parse.rs` (max prod file ~175 LOC).
  - `crates/mztab-m/src/vocab.rs` -> split into `vocab.rs`, `vocab/version.rs`, `vocab/profile.rs`, `vocab/section.rs`, `vocab/field.rs`; added `FromStr` for `Version`, `Profile`, `AmbiguityCode`, `Datatype` (plus `VocabError`); added internal `TableSection` (`Sml`, `Smf`, `Sme`), removing `Section`'s silent `Mtd | Sml` aliasing from `write.rs` and `validate.rs`.
  - `crates/mztab-m/src/lib.rs`, `model.rs`, `meta.rs`, `table.rs`, `typing.rs`, `write.rs`, `validate.rs`.
- **Tests added:** `parse_index_checks_canonical_digits` (`id.rs`), `table_sections_exclude_metadata` (`vocab/section.rs`), `FromStr` assertions in `vocab/{version,profile,field}.rs`.
- **Verification:** `check`, `test-fast`, `wasm`, `echoes` (0 duplicated, 0 approximate, 0 reuse), `refquiem` pending re-run + `mutants-diff`.

## Step 4 — Break `meta <-> model` Cycle & Split `model/`, `meta/`, `diag/`, `cv/`, `owned/`

- **Files touched / created:**
  - `crates/mztab-m/src/model.rs` -> split into `model.rs`, `model/{document,layout,metadata,colunit,table,rows,meta_value,meta_insert,meta_entries}.rs`; kept `Metadata`'s single inherent `impl` (`insert`, `entries`) in `model/metadata.rs` delegating to `meta_insert`/`meta_entries`, eliminating both `#[expect(clippy::too_many_lines)]` attributes.
  - `crates/mztab-m/src/meta.rs` -> split into `meta.rs`, `meta/{subkey,key}.rs` (with `ValueKind` in `meta/key.rs`), removing every `crate::model` dependency from `meta` so `model -> meta` is strictly acyclic; added `FromStr` for `MtdKey` returning `VocabError`.
  - `crates/mztab-m/src/diag.rs` -> split into `diag.rs`, `diag/{severity,code,location,finding}.rs`; added `as_str()` on `Severity`, `Requirement`, `Layer` and `FromStr` on `Severity`, `Layer`, `Code`.
  - `crates/mztab-m/src/cv.rs` -> split into `cv.rs`, `cv/rule.rs` (kept generated `cv/rules.rs` unchanged).
  - `crates/mztab-m/src/owned.rs` -> split into `owned.rs`, `owned/model.rs`.
  - `crates/mztab-m/tests/findings_doc.rs`, `docs/findings.md`, `docs/DEBT.md`.
- **Verification:** `check`, `doc`, `test-fast`, `echoes` (0 duplicated, 0 approximate, 0 reuse), `refquiem` (no stale mentions), `wasm`, `mutants-diff` (`MK_BASE=HEAD`: 157 tested — 123 caught, 34 unviable, 0 missed).

## Step 5 — Break `table <-> write` Cycle; Split `tsv/`, `write/`, `read/`

- **Files touched / created / deleted:**
  - Deleted `crates/mztab-m/src/typing.rs` and `crates/mztab-m/src/table.rs`, replaced by `tsv.rs` + `tsv/{splitter,typing,mtd,table}.rs` with zero dependency on `write`.
  - `crates/mztab-m/src/write.rs` -> split into `write.rs`, `write/{error,sink,render,line,pending,layout}.rs`; moved `RenderRow`/`render_stable` into `write/render.rs`; added `TableSlots` precomputed once per write; added `to_canonical_string(mztab, keep_comments)` with zero `MzTab::clone()` and fast canonical empty-layout emission.
  - `crates/mztab-m/src/read.rs` -> split into `read.rs`, `read/{event,metadata,typer,builder,stream}.rs`.
  - Updated `crates/mztab-m/src/lib.rs`, `crates/mztab-m/src/json/write.rs`, and `crates/mztab-m/tests/writer.rs` (`canonical_string_matches_layout_variants` + BOM coverage).
- **Verification:** `check`, `doc`, `test-fast`, `echoes` (0 duplicated, 0 approximate, 0 reuse), `refquiem`, `wasm`, `mutants-diff` (`MK_BASE=HEAD`: 337 tested — 223 caught, 114 unviable, 0 missed, 0 timeouts).

## Step 6 — Decompose & Optimize `validate/`

- **Files touched / created:**
  - `crates/mztab-m/src/validate.rs` -> split into `validate.rs`, `validate/{report,patterns}.rs`; tightened `Site::{Header,Column,Cell}` to carry `TableSection`; eliminated `Vec<&str>` allocation in `native_id_ok`.
  - `crates/mztab-m/src/validate/metadata.rs` -> split into `validate/metadata.rs`, `validate/metadata/{mandatory,refs,values}.rs`; decomposed `mandatory` into 3 focused helpers, removing `#[expect(clippy::too_many_lines)]` and reducing cognitive complexity from 35/25 to <= 25.
  - `crates/mztab-m/src/validate/tables.rs` -> split into `validate/tables.rs`, `validate/tables/{shape,row,sml,smf,sme,colunit}.rs`; bundled `ShapeCtx`, precomputed stable column `usize` offsets once per table via `.rev()` single scan, converted `refs()` to an iterator, skipped `colunit` header formatting when `units.is_empty()`, and removed `#[expect(clippy::too_many_lines)]` from `sml_rows`.
  - `crates/mztab-m/src/validate/semantic.rs` -> split into `validate/semantic.rs`, `validate/semantic/{uses,terms,rules}.rs`; replaced O(31 × |uses|) full-table scanning and per-use `Vec` allocations with single-pass `Target`-indexed `by_target` buckets and scalar counters (`hits`, `any_unknown`, `repeatable`).
  - Updated `docs/DEBT.md`, `crates/mztab-m/tests/validate.rs`, `crates/mztab-m/tests/rules.rs` (added repeatable CV term test case), and `crates/mztab-m/tests/writer.rs`.
- **Verification:** `check`, `clippy -W clippy::cognitive_complexity -D warnings` (0 warnings), `doc`, `test-fast`, `echoes`, `refquiem`, `wasm`, midpoint `./mk coverage` (**100.00% lines** across workspace), and targeted mutation tests (`validate/patterns.rs` + `validate/semantic/rules.rs`: 0 missed).

## Step 7 — Stream `json/` Writer Without Cloning & Split JSON Codec Modules

- **Files touched / created:**
  - `crates/mztab-m/src/write.rs` -> added `#[cfg(feature = "json")] pub(crate) fn write_canonical` (streams canonical TSV directly without cloning `MzTab`).
  - `crates/mztab-m/src/json.rs` + `json/schema.rs` + `json/schema_tables.rs` -> stored `(tsv_name, json_key, Col)` pairs statically, eliminating runtime lowercase `String` key allocations per cell.
  - `crates/mztab-m/src/json/value.rs` + `json/lexer.rs` -> separated AST types and `write_string` from JSON tokenizing/scanning.
  - `crates/mztab-m/src/json/read.rs` -> split into `json/read.rs`, `json/read/{emit,meta,table}.rs`; replaced `O(K^2)` linear member scans per row with a single-pass `BTreeMap<&str, &Node<'_>>` lookup map per row while preserving exact duplicate/unknown key diagnostics.
  - `crates/mztab-m/src/json/write.rs` -> split into `json/write.rs`, `json/write/{emit,meta,table}.rs`; eliminated `MzTab::clone()` and intermediate JSON DOM node allocations via `JsonWriter<'_>` direct streaming, pre-flight metadata buffer + `TablePlan` validation before writing to `out`, and precomputed `RowMember` plans per table.
  - Updated `docs/audit/AUDIT.md` and `docs/REFACTOR_PLAN.md` (Step 7 `[x]`).
- **Verification:** `check`, `doc`, `test-fast`, `echoes` (0 duplicated, 0 approximate, 0 reuse), `refquiem` (0 stale mentions), `typos`, `wasm`, and targeted `cargo mutants` on `json/write/emit.rs`, `json/write.rs`, `json/write/table.rs` (83 tested — 71 caught, 12 unviable, 0 missed).

## Step 8 — Modularize `mztab-m-api`, Optimize `mztab-m-obo`, & Share `write_string`

- **Files touched / created:**
  - `crates/mztab-m/src/json.rs` + `json/value.rs` -> exposed `pub use value::write_string` with doc and doctest.
  - `crates/mztab-m-obo/src/lib.rs` + `src/parse.rs` -> separated stanza/tag parsing into `parse.rs` and replaced `e.parents.clone()` in `is_descendant` with a `Vec<&[&str]>` slice stack.
  - `crates/mztab-m-api/src/lib.rs` -> decomposed into `format.rs`, `settings.rs`, `ops.rs`, `report.rs`:
    - `format.rs` + `settings.rs`: reused `Severity::from_str` and `Code::from_str` instead of manual lookups.
    - `ops.rs`: eliminated `mztab.clone()` on `!lossy` writes via `mztab_m::to_canonical_string(mztab, true)` / `json::to_string`; added `validate_obo`, `validate_json`, `vocabulary_size`.
    - `report.rs`: reused `mztab_m::json::write_string` in `Fact::to_json`, `Layer::as_str()`/`Severity::as_str()`/`Requirement::as_str()`, and added `CodeRecord`, `codes`, `facts_json`, and `findings_json`.
  - `crates/mztab-m-api/tests/api.rs` -> added `facade_exposes_codes_facts_json_and_obo_validation`.
  - Updated `docs/REFACTOR_PLAN.md` (Step 8 `[x]`).
- **Verification:** `check`, `doc`, `test-fast`, `echoes`, `refquiem`, `typos`, `wasm`, and `cargo mutants` on `mztab-m-api` + `mztab-m-obo` (93 tested: 76 caught, 16 unviable; fixed 1 missed `vocabulary_size` mutant, re-verified `ops.rs`: 18 tested — 15 caught, 3 unviable, 0 missed).

## Step 9 — Route All Layer 3 Adapters Through `mztab-m-api` & Split CLI

- **Files touched / created:**
  - `crates/mztab-m-cli/Cargo.toml`, `crates/mztab-m-wasm/Cargo.toml`, `bindings/python/Cargo.toml`, `bindings/r/src/rust/Cargo.toml` (and updated lockfiles) -> removed direct `mztab-m` and `mztab-m-obo` dependencies so all four Layer 3 adapters depend strictly on Layer 2 `mztab-m-api`.
  - `crates/mztab-m-cli/src/lib.rs` -> split into `lib.rs`, `args.rs`, `exec.rs`, replacing the stringly-typed `Options` struct with a typed `Command` + `Action` enum (`Validate`, `Fmt`, `Convert`, `Inspect`) and `ValidateOpts`/`FmtOpts`/`ConvertOpts`.
  - `crates/mztab-m-wasm/src/lib.rs`, `bindings/python/src/lib.rs`, `bindings/r/src/rust/src/lib.rs` -> switched to `mztab_m_api::{validate_json, codes, vocabulary_size, SPEC_COMMIT, SPEC_VERSION, CodeRecord}`.
  - `crates/mztab-m-cli/tests/run.rs` -> added assertions on exact subcommand names in `USAGE` error messages.
  - Updated `docs/audit/AUDIT.md`, `docs/ARCHITECTURE.md`, and `docs/REFACTOR_PLAN.md` (Step 9 `[x]`).
- **Verification:** `check`, `doc`, `test-fast`, `wasm`, `bindings` (Python wheel + unittest + R tarball/testthat 27/27 PASS), `machete` (0 unused), `echoes` (0 duplicated, 0 approximate, 0 reuse), `refquiem` (0 stale), `typos`, `cargo mutants` on `mztab-m-cli` + `mztab-m-wasm` (57 tested initially: 2 missed `Kind::as_str`, fixed via `run.rs` tests and re-verified `args.rs`: 26 tested — 19 caught, 7 unviable, 0 missed).
