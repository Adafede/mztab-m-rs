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
