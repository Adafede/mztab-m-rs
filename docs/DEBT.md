# Technical Debt & Ratchet Ledger

Every lint suppression, duplicate-dependency skip, echoes/coverage ratchet
exception, or mutation exclusion in this repository is recorded here with its
reason and the condition under which it can go. The reasons are copied from
the `reason = "..."` in the code; keep the two in sync.

Currently there are no duplicate-dependency skips (`deny.toml`), no mutation
exclusions (`.cargo/mutants.toml`) and no `refquiem: allow` markers.

## Active exceptions

| Location | Lint | Reason | Removal condition |
|---|---|---|---|
| `crates/mztab-m/src/id.rs` `IndexMap::get_or_insert_default` | `clippy::indexing_slicing` | `at` was just found or just inserted, so it is in bounds | A non-indexing way to return the slot without a second lookup |
| `crates/mztab-m/src/read/event.rs` `Event` | `clippy::large_enum_variant` | Events are consumed one at a time; boxing rows would cost an allocation per row | If a benchmark shows boxing is free |
| `crates/mztab-m/tests/cv_rules.rs` (crate) | `clippy::expect_used` | test helpers | Test code; stays |
| `crates/mztab-m/tests/json.rs` (crate) | `clippy::expect_used` | test helpers | Test code; stays |
| `crates/mztab-m/tests/oracle.rs` (crate) | `clippy::expect_used` | test setup: unreadable oracle files are a broken checkout | Test code; stays |
| `crates/mztab-m/tests/rules.rs` (crate) | `clippy::expect_used`, `clippy::indexing_slicing` | test helpers edit a known document; a miss is a broken test | Test code; stays |
| `crates/mztab-m/tests/rules.rs` `cases` | `clippy::too_many_lines` | One line per code | Test data; stays |
| `crates/mztab-m/tests/rules.rs` `more_cases` | `clippy::too_many_lines` | One line per branch | Test data; stays |
| `crates/mztab-m/tests/semantic.rs` (crate) | `clippy::expect_used` | test helpers | Test code; stays |
| `crates/mztab-m/tests/writer.rs` (crate) | `clippy::expect_used` | test helpers: a fixture that does not parse is a setup error | Test code; stays |
| `crates/mztab-m-api/tests/vocabulary.rs` (crate) | `clippy::expect_used` | test helpers | Test code; stays |
| `crates/mztab-m-wasm/src/lib.rs` `validate`, `convert` | `clippy::needless_pass_by_value` | wasm-bindgen passes JS string arrays and options by value | Stays while wasm-bindgen requires it |
| `bindings/python/src/lib.rs` `error` | `clippy::needless_pass_by_value` | a function for `map_err`, which passes the error by value | Stays while used with `map_err` |
| `bindings/r/src/rust/src/lib.rs` (crate) | `clippy::needless_pass_by_value`, `clippy::used_underscore_binding` | `#[extendr]` passes R vectors by value and names bindings with underscores | Stays while extendr requires it |
| `bindings/r/src/rust/src/lib.rs` `count_robj` | `clippy::cast_precision_loss` | R double numeric represents integer counts above i32::MAX up to 2^53 exactly | Stays while extendr lacks u64 scalar conversion |
| `benches/benches/heap.rs` (crate) | `clippy::print_stdout` | the bench prints its report table | Bench output; stays |
| `benches/benches/heap.rs` (missing corpus) | `clippy::expect_used` | benchmark setup: missing corpus file means submodule is uninitialized | Stays |
| `benches/benches/heap.rs` `row` | `clippy::cast_precision_loss` | a ratio for display | Stays |
| `benches/benches/throughput.rs` (missing corpus) | `clippy::expect_used` | benchmark setup: missing corpus file means submodule is uninitialized | Stays |
| `mk.local` `task_miri` | `miri` gate override (`mztab-m --lib` and `mztab-m-obo`) | Workspace forbids `unsafe_code`; `mztab-m-cli` spawns subprocesses unsupported by Miri and the 35 MB corpus / 2,000-case proptests exceed nightly time budgets under Miri | If Miri becomes fast enough for the corpus and proptests and `mztab-m-cli` is scoped via `MIRI_PACKAGES` |
| `mk.local` `task_outdated` | `outdated` gate override (`--ignore criterion`) | `criterion` is pinned to `0.5` because `0.8` builds `alloca`'s C object, which fails to link with Xcode clang 21 (DECISIONS D-073) | When `criterion` drops `alloca` or Xcode clang links it |

## Intentional exceptions

| Item | Exception | Justification |
|---|---|---|
| `crates/mztab-m/src/lib.rs` | Flat crate-root `pub use` exports and `pub(crate) use spec_commit` | Deliberate curated public entry point hiding internal module layout (`AGENTS.md` / design boundary) and macro scoping |
| `crates/mztab-m/Cargo.toml` | Optional `json` feature flag | Keeps the default `mztab-m` build zero-dependency while gating the JSON parser, writer, and schema |
| `crates/mztab-m-api` | Intermediate application crate between core/OBO and adapters | Shared stateless text-in/text-out API for `mztab-m-cli`, `mztab-m-wasm`, Python, and R adapters (DECISIONS D-076) |
| `crates/*/Cargo.toml` | Explicit path dependencies on sibling workspace crates | Required by `bindings/r/bootstrap.R`, which rejects `.workspace = true` inside `[dependencies]` when assembling the standalone R tarball |
| `Cargo.lock` | Dev-only duplicate `syn` version (`criterion 0.5`) | Bench dev-dependency only; zero runtime duplicate dependency versions (`DECISIONS D-073`) |
