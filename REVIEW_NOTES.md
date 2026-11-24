> Provenance: authored by an AI coding agent at completion of the structural refactor.

# Review Notes

Human reviewer checklist covering architectural decisions, rejected alternatives,
intentional behavior preservation, remaining technical debt, and high-priority
inspection targets for the `refactor/architecture` branch.

## 1. Key Decisions & Rejected Alternatives

1. **Keep `crates/mztab-m` as a single zero-dependency `#![no_std]` + `alloc` crate (rejected: splitting into multiple workspace crates).**
   - *Decision:* Enforced strict 4-tier DAG submodules (`vocab`/`value`/`param`/`id` -> `model`/`meta`/`cv`/`diag`/`owned` -> `tsv`/`write`/`read`/`json` -> `validate`) inside the existing `mztab-m` crate.
   - *Why:* Preserves zero external dependencies, avoids breaking downstream crate imports / `cargo semver-checks` paths, and avoids duplicate `no_std` crate boilerplate.

2. **Preserve JSON-through-TSV conformance while eliminating `MzTab::clone` and DOM allocation (rejected: rewriting a direct model-to-JSON serializer bypassing TSV).**
   - *Decision:* Added `crate::write::write_canonical` and streamed JSON directly via `JsonWriter<'_>` after pre-flight metadata buffer and `TablePlan` schema checks (`DECISIONS.md` D-071).
   - *Why:* Guaranteeing byte-for-byte parity on malformed/extension TSV values embedded as fallback JSON strings preserves 100% conformance without duplicating TSV formatting grammars, while still delivering a ~3.4x to ~5.8x throughput gain on `json_write`.

3. **Enforce strict 3-layer workspace dependency rule (rejected: leaving adapters calling `mztab-m` and `mztab-m-obo` directly).**
   - *Decision:* Routed `mztab-m-cli`, `mztab-m-wasm`, `bindings/python`, and `bindings/r` exclusively through `mztab-m-api` (`validate_obo`, `validate_json`, `codes`, `vocabulary_size`, `facts_json`, `findings_json`, and re-exports).
   - *Why:* Eliminates duplicated JSON string escaping (`Fact::to_json`), code-catalog formatting loops, and severity parsing across adapters, and shrinks release binaries. (Left `bindings/r/bootstrap.R` staging all three workspace crates for self-contained R source tarballs.)

4. **Type-driven invalid-state elimination (`TableSection`, CLI `Command` / `Action`, `FromStr`).**
   - *Decision:* Introduced crate-internal `TableSection` (`Sml`, `Smf`, `Sme`) for table-only contexts, replaced the CLI's stringly-typed option bag with `Command { source, action: Action::{Validate, Fmt, Convert, Inspect} }`, added standard `FromStr` impls on domain vocab/diagnostic types (`VocabError`), and tuned `[profile.release]` (`lto = "thin"`, `codegen-units = 1`).
   - *Rejected:* Adding `thiserror` (violates zero-dep `mztab-m` rule and adds proc-macro compile time to `mztab-m-api`) or `strip = true` in `[profile.release]` (prevents `cargo bloat` crate attribution).

## 2. Intentional Behavior Changes

**None.** No parsing rule, validation code, severity, CLI exit code, CLI error message, TSV canonical layout, JSON output, or binding API behavior was changed. All changes outside internal restructuring are strictly additive (`mztab_m::to_canonical_string`, `mztab_m::VocabError`, `mztab_m::json::write_string`, standard `FromStr` impls, and `mztab-m-api` facade helpers).

## 3. Code Size & Complexity Exceptions

All production files outside data/macro tables are at or below ~300 non-comment LOC, and `clippy::cognitive_complexity` reports **0 warnings**. Justified structural exceptions:

- `crates/mztab-m/src/meta/key.rs` (438 total / 348 non-comment LOC): declarative metadata key enumeration (`MtdKey`) + canonical string mapping (`kind` 53 LOC, linear dispatch).
- `crates/mztab-m/src/model/meta_insert.rs` (335 total / 312 non-comment LOC): 1:1 typed routing from `MtdKey` into `Metadata` fields (`insert` 53 LOC, flat match).
- `crates/mztab-m/src/validate/patterns.rs` (345 total / 305 non-comment LOC) & `crates/mztab-m/src/cv/rules.rs` (310 total / 305 non-comment LOC, build/spec-generated CV mapping rules).
- Flat dispatch / exhaustive field matchers (`tsv/mtd.rs::mtd_value` 88 LOC, `read/builder.rs::event` 76 LOC, `read/typer.rs::typed` 73 LOC, `validate/metadata/mandatory.rs::mandatory_core_elements` 71 LOC, `validate/semantic/uses.rs::slot` 66 LOC): each is a single-level exhaustive `match` over all spec fields with low branching complexity (`cognitive_complexity <= 25`).

## 4. Remaining Technical Debt (Ranked)

All workspace lint exceptions live in `docs/DEBT.md` and dropped from **7** to **3** in production `crates/`:

1. **[Low / Micro-opt] JSON reader intermediate TSV buffer (`crates/mztab-m/src/json/read/`):** `json_parse` still serializes transcoded TSV text and re-parses it (`19.3–21.8 MiB/s` after optimizations vs `6.5–14.2 MiB/s` baseline). Direct AST-to-`Event` streaming could accelerate `json_parse` further if JSON ingestion becomes a bottleneck (`DECISIONS.md` D-071).
2. **[Low / Recorded in `docs/DEBT.md`] `crates/mztab-m/src/read/event.rs` (`#[expect(clippy::large_enum_variant)]`):** Keeping `Event` unboxed avoids per-row heap allocation during streaming reads.
3. **[Low / Recorded in `docs/DEBT.md`] `crates/mztab-m/src/id.rs` (`#[expect(clippy::indexing_slicing)]`):** `IndexMap::get_or_insert_default` indexes a just-located slot to avoid a double lookup.
4. **[Low / External FFI] `crates/mztab-m-wasm/src/lib.rs` (`#[expect(clippy::needless_pass_by_value)]`):** Required by `wasm-bindgen` for JS string arrays.

## 5. What a Human Reviewer Should Scrutinize First

1. **`crates/mztab-m/src/json/write/` (`write.rs`, `emit.rs`, `meta.rs`, `table.rs`):** Verify pre-flight buffer + `TablePlan` validation happens before any byte is written to `out`, and verify composite fallback string emission (`try_param`, `try_params`, `try_refs`, `try_publications`, `try_spectra_refs`) writes nothing before falling back to `JsonWriter::str`.
2. **`crates/mztab-m/src/write/layout.rs` (`write_canonical_opts`) & `crates/mztab-m-api/src/ops.rs` (`write`):** Confirm `to_canonical_string(mztab, true)` matches the former `layout = layout.comments_only(); to_string(&model)` clone behavior across TSV and JSON outputs.
3. **`crates/mztab-m/src/validate/semantic/` (`uses.rs`, `rules.rs`, `terms.rs`):** Confirm `by_target` bucket indexing and scalar counters preserve the exact `RuleSet` diagnostic emission order and multiplicity.
4. **Layering & FFI crates (`crates/mztab-m-cli`, `crates/mztab-m-wasm`, `bindings/python`, `bindings/r`):** Confirm zero direct `mztab-m` or `mztab-m-obo` coupling and check R self-contained tarball staging via `bindings/r/bootstrap.R`.
