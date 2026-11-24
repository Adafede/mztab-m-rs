> Provenance: authored by an AI coding agent during the structural refactor.

# Structural Refactor Plan

Status legend: `[x]` done, `[ ]` pending.

| Step | Scope | Verification Gate | Rollback Point | Status |
|---|---|---|---|---|
| 0 | Branch `refactor/architecture` & baseline criterion benchmarks | `cargo bench -p mztab-m-bench --bench throughput` | `main` (`cfee822`) | [x] |
| 1 | Write `docs/audit/AUDIT.md`, `docs/ARCHITECTURE.md`, `docs/REFACTOR_PLAN.md`, `SESSION_LOG.md` | `./mk typos && ./mk refquiem` | Step 0 | [x] |
| 2 | Pin characterization tests in `mztab-m`, `mztab-m-api`, `mztab-m-cli` | `./mk ci-fast` | Step 1 commit | [x] |
| 3 | Split core primitives (`value/`, `param/`, `vocab/`, `id.rs`), add `TableSection` & `FromStr` | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 2 commit | [x] |
| 4 | Break `meta <-> model` cycle; split `model/`, `meta/`, `diag/`, `cv/`, `owned/` | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 3 commit | [x] |
| 5 | Break `table <-> write` cycle; split `tsv/`, `write/`, `read/`; precompute writer slots & add canonical no-clone writer | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 4 commit | [x] |
| 6 | Split & optimize `validate/` (`patterns`, `report`, `metadata/*`, `tables/*`, `semantic/*`); zero cognitive-complexity warnings | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 5 commit | [x] |
| 7 | Split & optimize `json/` (eliminate `MzTab::clone`, DOM `J` allocation, and `O(K^2)` cell lookups) | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 6 commit | [ ] |
| 8 | Modularize `mztab-m-api` & optimize `mztab-m-obo` (`is_descendant`); remove duplicate JSON escaping | `./mk check && ./mk test-fast && ./mk wasm && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 7 commit | [ ] |
| 9 | Enforce Layer 3 -> Layer 2 boundary across `mztab-m-cli`, `mztab-m-wasm`, `bindings/python`, `bindings/r`; type-safe CLI `Command` | `./mk check && ./mk test-fast && ./mk wasm && ./mk bindings && ./mk machete && ./mk echoes && ./mk refquiem && ./mk mutants-diff` | Step 8 commit | [ ] |
| 10 | Add `FromStr` property tests, tune `[profile.release]`, clean `benches/src/lib.rs` | `./mk ci` | Step 9 commit | [ ] |
| 11 | Full re-audit, coverage/bindings/site/benchmarks verification, `REVIEW_NOTES.md`, final `SESSION_LOG.md` | `./mk ci && ./mk coverage && ./mk bindings && ./mk site` | Step 10 commit | [ ] |
