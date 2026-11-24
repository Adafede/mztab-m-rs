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
