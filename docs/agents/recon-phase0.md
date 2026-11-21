> Provenance: survey of the spec and of jmzTab-m written by an AI coding
> agent on 2026-11-17, before any code. Kept as a record; not updated.

# Phase 0 – Recon report

This report was compiled on 2026-11-17. Source pins are listed in
`DECISIONS.md`. The companion documents are:

- [jmztab-m-rules.md](jmztab-m-rules.md) – the full inventory of jmzTab-m rules.
- [DECISIONS.md](../../DECISIONS.md) – the ambiguities (D-xxx) and the jmzTab-m deviations (J-xx).

## (a) Spec inventory — HUPO-PSI/mzTab-M @ e66c19e

### Normative or structural artefacts

| File | Version | Notes |
|---|---|---|
| `specification_documents/mzTab_format_specification_2_1-M.adoc` | **2.1.0, draft** (`2.1.0-InDevelopment July 2025`) | The prose spec: format rules, profiles (§5.18), cross-reference rules per profile, null/number encoding, Param syntax, and `opt_` columns. |
| `specification_documents/mzTab_m_2_1_schema.adoc` | 2.1 | Field reference. Generated from `mzTab_2_1-M.json` by `schema/generate_schema_adoc.py`. Gives Mandatory and Nullable flags for each field and the MTD order. |
| `specification_documents/mzTab_m_2_0_to_2_1_changes.adoc` | 2.0 vs 2.1 | Diff for each field. MTD: 16 fields added and 2 removed (`ms_run-usi_identifier`, `study_variable-factors`). |
| `specification_documents/mzTab_m_2_0_schema.adoc` | 2.0.0 | Field reference for 2.0. It is the only structural source for 2.0, because `mzTab_2_0-M.json` is referenced but **missing**. |
| `schema/mzTab_2_1-M.json` | 2.1.0-M | JSON Schema 2020-12 with 25 `$defs`. It carries custom keywords (`validation_policy`, `list_concatenation_str`, `multiple_columns`, …). It defines the **JSON representation**. |
| `schema/mzTab_2_1-M_openapi.json` | 2.1.0-M | OpenAPI 3.1.1 for the validator REST API. The `MzTab` schema `$ref`s the JSON Schema above. |
| `schema/mzTab_2_0-M_openapi_3_1.json` | labelled `2.1.0` (sic) | Its `$ref` to `mzTab_2_0-M.json` is dangling. |
| `schema/mzTab_2_1-M_mapping.xml` | 2.1 (says `modelVersion="2.0.0"`) | PSI CvMapping file with 33 rules. |
| `schema/mzTab_2_1-M_metabolights_mapping.xml` | 2.1 | MetaboLights profile, 29 rules. |
| `schema/mzTab_2_0-M_summary-mapping.xml` (+ `.html`) | 2.0 | 26 rules. |

### Informative artefacts

| File | Notes |
|---|---|
| `schema/mztabm-schema-comparison-report.md` | JSON Schema vs. OpenAPI. One of its claims is wrong (see D-016). |
| `schema/validation-profiles-openapi-todos.md` | Draft design for a `ValidationProfile` with structural, type, and semantic_cv checks. Not adopted. |
| `docs/…/validation/*.adoc` | Describes validation levels: Minimal, Structural, Semantic. Its "profiles" page still uses the 1.0-era `mzTab-mode` Summary/Complete, which is stale. |
| `.github/scripts/validate-examples.sh` | Validates the examples in CI with jmzTab-m. |

### Example corpus

There are 23 files: 21 TSV and 2 JSON, from 4.5 KB to 13.6 MB. Corpus
traits that drive design:

- Mixed versions:
  - `2.0/` contains `2.2.0-M` files (masster, progenesis.json).
  - `2.1/` contains `2.0.0-M` files (xcms `*_onlySMF`).
- Line endings and blank lines:
  - CRLF line endings (masster).
  - Lines made only of tabs.
  - Trailing empty cells on MTD lines.
- Table layout:
  - A header-only SMH (`minimal_example`).
  - Synthetic single-row SML (xcms 2.0).
  - Non-canonical column order (lipidcompass).
  - An `opt_` column in the middle of SEH (lipidomics v2.1).
- Two incompatible JSON dialects (D-004).
- **No intentionally invalid examples.** Negative tests must come from the jmzTab-m fixtures and from tests we write.
- Profiles declared in the corpus: `M`, `M+S`, `M+F`, `M+F+E`. **No file declares `M+S+F+E`.**
- The largest file is 13.6 MB. "Very large" benchmarks need a synthetic generator.

## (b) jmzTab-m rule inventory

The full inventory is in [jmztab-m-rules.md](jmztab-m-rules.md). In summary:

- Version 2.1.0-SNAPSHOT. It accepts `2.\d.\d-M`. The 2.1 code path exists (profiles, `study_variable_group`), but it is **not version-gated**.
- **About 95 live codes:**
  - Format: 1000–1038
  - Logical: 2000–2068
  - CrossCheck/CV: 3005–3023
- About 35 codes are dead, deprecated, or duplicated.
- The CV layer needs network I/O (versioned OBO/OWL downloads, or OLS4).
- The CLI emits text only, matching `[Level-code] line N: msg`. A JVM (Java 25) is needed, and there is none locally (D-030).
- Behaviour that matters for our design:
  - Fatal errors stop parsing.
  - Model-level checks are skipped once any Error has been raised.
  - Several valid inputs crash it, e.g. `assay[n]-custom[m]`.
  - Lenient lexing: values are trimmed, `find()` regex matching, case-insensitive prefixes, Java `Double` grammar.
  - 17 spec deviations are recorded as J-01 … J-17.

## (c) Template conventions — Adafede/rusti-graben @ dae14b3

### Adopt as-is

- **Workspace and toolchain:**
  - Workspace with `resolver = "3"`.
  - Edition 2024.
  - `rust-toolchain.toml` pinned to `1.100.0`; nightly `2026-11-03` for miri and udeps.
  - The `mk` bash task runner is the whole of CI (`./mk ci`, `ci-nightly`, `ci-release`).
  - `Cargo.lock` is committed.
- **Lints:**
  - All lints live in `[workspace.lints]`, and every member sets `lints.workspace = true`.
  - Clippy `all`, `pedantic` and `nursery` at warn, with CI running `-D warnings`.
  - Denied: `unwrap_used`, `panic`, `todo`, `allow_attributes*`, `dbg_macro`.
  - Warned: `indexing_slicing`, `string_slice`, `std_instead_of_core`, `std_instead_of_alloc`, `missing_assert_message`, `mod_module_files`, and others.
  - `unsafe_code = "forbid"`, `missing_docs`, `unreachable_pub`.
  - Rustdoc lints deny broken links.
  - No `#[allow]`; use `#[expect(…, reason)]`.
  - The `ratchet` gate rejects any new `allow`.
- **Gates (`./mk ci`):**
  - tools, zizmor, shellcheck
  - Conventional Commits
  - ratchet, fmt, tombi, typos
  - clippy, lints-inherit
  - test
  - doc (`RUSTDOCFLAGS=-D warnings`, `--document-private-items`)
  - cargo-deny (license allowlist, `multiple-versions = deny`, unknown sources denied)
  - cargo-audit (`--deny warnings`), machete
  - msrv (`cargo +<rust-version> check`)
  - echoes (duplicate code), refquiem (stale refs)
  - semver-checks (skipped below 1.0 or without a tag)
  - wasm (`cargo check` only)
  - mutants-diff (zero survivors)
- **Nightly gates:** a full mutants run, `llvm-cov` with **100%** line coverage (ratcheted), cargo-hack feature powerset, miri, and outdated.
- **Docs:**
  - The crate README is the crate doc (`#![doc = include_str!("../README.md")]`), so its examples are doctests.
  - Every public item has `///` docs with `# Examples` and `#[must_use]`.
  - Exceptions go in `docs/DEBT.md`.
  - ADRs live in `docs/adr/`.
- **Release:**
  - A `v*` tag runs `ci` plus `cargo publish --dry-run`.
  - **Nothing publishes or pushes automatically.** The workflow permissions are `contents: read`.
  - Dependabot and Renovate open PRs; nothing auto-merges.

### Planned deviations and adaptations (proposed)

| # | Template | Change | Reason |
|---|---|---|---|
| T1 | ~~`uses: $/.github/actions/setup`~~ | **Retracted:** `$/` is GitHub's self-repository syntax, which zizmor recommends (see D-031). Kept as-is. | — |
| T2 | The wasm gate only runs `cargo check` on the whole workspace. | Add a `mk.local` override: `cargo test -p mztab-m --target wasm32-unknown-unknown` through a pinned `wasm-bindgen-test-runner` (dev-dep only), plus a `--no-default-features` build. Exclude CLI and bench crates from the wasm target. | Your constraint "CI must build **and test** for wasm". |
| T3 | No fuzz support. | Add a `fuzz/` crate with cargo-fuzz and libfuzzer-sys, excluded from the workspace, as a bounded nightly gate (`-max_total_time`). | Fuzz target required. Nightly-only toolchain. |
| T4 | Benches are compile-only. | Add a separate `benches` crate with `publish = false` and criterion as a dev-dep. Expect cargo-deny `skip` entries for criterion's duplicate dependency versions, recorded in DEBT.md. | Keeps the core dependency graph untouched. |
| T5 | MSRV = pinned stable 1.100.0. | Library `rust-version` to be decided in Phase 1. I suggest keeping 1.100.0 unless you want a lower floor. | Edition 2024 needs ≥ 1.85. |
| T6 | License AGPL-3.0-or-later. | **Your call.** AGPL on a library will deter adoption: Luca's crates are MIT. I suggest MIT OR Apache-2.0. | Adoption goal. |
| T7 | 100% line coverage and zero surviving mutants, including the CLI. | Keep for the core. For the CLI I/O glue, proposed: keep it too, with a thin I/O shim. Otherwise any exemption goes in DEBT.md. | Strict but feasible for a parser. |
| T8 | Agent files (`AGENTS.md`, `CLAUDE.md`, `.agents/`). | Adopt only if you want them. | Not a code concern. |
| T9 | `CONTRIBUTING.md` states MSRV 1.85. | Fix it. | Stale. |

## (d) Ambiguities and version mismatches (needs your decision)

The full list, with proposals, is in `DECISIONS.md`. These are the blocking ones:

1. **D-001: v2.1 is an unreleased draft.** The repo has v2.1 content, but the spec itself says `2.1.0-InDevelopment`, and the profile docs say "may change before the 2.1 release". The only released version is 2.0.0-M. Proposal: target the pinned draft and also read and validate 2.0.0-M.
2. **D-002: normative precedence.** The prose, the generated field reference, and the JSON Schema disagree. Proposal: prose > field reference > JSON Schema.
3. **D-010: column rules.** "All columns mandatory, order MUST" contradicts the field flags (all `Mandatory: False`), the spec's own example, and the corpus.
4. **D-011, D-012, D-015: nullability conflicts** for SMF `charge`, SML `best_id_confidence_value`, and `reliability`.
5. **D-013: `study_variable_group`.** The prose says MUST (≥1); the JSON Schema says optional; most 2.1 examples have none.
6. **D-014: the MTD mandatory set** differs across 2.0, the 2.1 field reference, and the 2.1 JSON Schema. `publication` is probably a generator bug.
7. **D-004: JSON.** The corpus has two incompatible dialects, and JSON support costs either a dependency or a hand-written parser.
8. **J-09, J-11, J-14:** places where jmzTab-m and the spec disagree in ways that change which files pass.
9. **T6: license.**
10. **D-030: JVM for the differential tests.** Proposal: a CI-only job; golden outputs are committed.
