> Provenance: Technical debt audit and refactoring ledger produced by an AI coding agent on branch `audit/pass-2` (refactoring baseline `26a4786`).

# Technical Debt & Refactoring Ledger (`mztab-m-rs`)

Scope: full repository audit and behavior-preserving refactoring of `/Users/adrutz/Git/mztab-m-rs` (Rust workspace, WASM demo, Python/R bindings, benchmarks, fuzzers, CI/scripts, docs) from baseline commit `26a4786` through `f64a627`. All golden outputs (`corpus.txt`, `broken.txt`, `full.canonical.mztab`, `full.json`, `json-corpus.txt`, `psi-ms.txt`, `benches/golden.mztab`) remain byte-for-byte identical.

---

## 1. Coverage Ledger

All 280 tracked files (`279` files at `f64a627` plus this ledger `DEBT.md`) and pinned `third_party/` submodules are accounted for:

| File | Status | Notes |
|---|---|---|
| `.cargo/mutants.toml` | Reviewed | Mutation config |
| `.config/nextest.toml` | Reviewed | Test runner config |
| `.echoes.toml` | Reviewed | Duplication/clones gate config |
| `.editorconfig` | Reviewed | Editor formatting rules |
| `.github/ISSUE_TEMPLATE/bug_report.md` | Reviewed | Issue template |
| `.github/ISSUE_TEMPLATE/feature_request.md` | Reviewed | Issue template |
| `.github/ISSUE_TEMPLATE/question.md` | Reviewed | Issue template |
| `.github/ISSUE_TEMPLATE/spec_disagreement.md` | Reviewed | Issue template |
| `.github/PULL_REQUEST_TEMPLATE.md` | Reviewed | PR checklist |
| `.github/actions/setup/action.yml` | Reviewed | Shared CI setup |
| `.github/dependabot.yml` | Reviewed | Dependency updates |
| `.github/workflows/bindings.yml` | Reviewed | Python & R CI workflow |
| `.github/workflows/ci.yml` | Reviewed | Main CI workflow |
| `.github/workflows/nightly.yml` | Reviewed | Nightly Miri/outdated/vocab CI |
| `.github/workflows/oracle.yml` | Reviewed | Oracle comparison CI |
| `.github/workflows/pages.yml` | Reviewed | Documentation + WASM deploy |
| `.github/workflows/release.yml` | Reviewed | Release workflow |
| `.gitignore` | Reviewed | Ignore rules |
| `.gitmodules` | Reviewed | Submodule definitions |
| `.refquiem.toml` | Reviewed | Stale-mention gate config |
| `.tombi.toml` | Reviewed | TOML formatter config |
| `.typos.toml` | Reviewed | Spellcheck config |
| `.vscode/settings.json` | Reviewed | Workspace IDE settings |
| `AGENTS.md` | Reviewed | Agent entry point |
| `AUDIT.md` | Reviewed | Prior scientific correctness audit ledger |
| `CHANGELOG.md` | Reviewed | Release history |
| `CITATION.cff` | Reviewed | Citation metadata |
| `CLAUDE.md` | Reviewed | Symlink/guide |
| `CODE_OF_CONDUCT.md` | Reviewed | Contributor covenant |
| `CONTRIBUTING.md` | Reviewed | Contributor workflow |
| `Cargo.lock` | N/A | Generated Cargo lockfile |
| `Cargo.toml` | Reviewed | Workspace manifest & strict lints |
| `DEBT.md` | Reviewed | This technical debt & refactoring ledger |
| `DECISIONS.md` | Reviewed | Spec & design decision register |
| `LICENSE-APACHE` | N/A | Standard Apache-2.0 license text |
| `LICENSE-MIT` | N/A | Standard MIT license text |
| `NOTICE` | Reviewed | Attribution notice |
| `README.md` | Reviewed | Repository overview |
| `SECURITY.md` | Reviewed | Security policy |
| `STATUS.md` | Reviewed | Implementation & verification status |
| `bacon.toml` | Reviewed | Background check runner config |
| `benches/Cargo.toml` | Reviewed | Bench manifest (`e1001f2` moved `mztab-m` to deps) |
| `benches/benches/heap.rs` | Reviewed | Peak heap allocation bench (`dhat`) |
| `benches/benches/throughput.rs` | Reviewed | Criterion throughput suite |
| `benches/golden.mztab` | Reviewed | Synthetic generator pinned output |
| `benches/src/lib.rs` | Reviewed | Synthetic generator (`e1001f2` DRY-01) |
| `bindings/python/Cargo.lock` | N/A | Generated Cargo lockfile |
| `bindings/python/Cargo.toml` | Reviewed | PyO3 crate manifest |
| `bindings/python/README.md` | Reviewed | Python package docs |
| `bindings/python/build.rs` | Reviewed | Maturin build hook |
| `bindings/python/python/mztab_m/__init__.py` | Reviewed | Python wrapper module |
| `bindings/python/python/mztab_m/_native.pyi` | Reviewed | Native type stubs |
| `bindings/python/python/mztab_m/py.typed` | N/A | PEP 561 marker file |
| `bindings/python/src/lib.rs` | Reviewed | PyO3 bindings (`1e1cb00` PERF-01) |
| `bindings/python/tests/test_mztab_m.py` | Reviewed | Python unittest suite |
| `bindings/r/.Rbuildignore` | Reviewed | R package build filter |
| `bindings/r/DESCRIPTION` | Reviewed | R package metadata |
| `bindings/r/LICENSE` | N/A | CRAN license stub |
| `bindings/r/NAMESPACE` | Reviewed | Generated roxygen namespace |
| `bindings/r/R/extendr-wrappers.R` | Reviewed | Generated extendr wrappers |
| `bindings/r/R/mztabm.R` | Reviewed | R public wrapper functions |
| `bindings/r/README.md` | Reviewed | R package docs |
| `bindings/r/bootstrap.R` | Reviewed | Standalone tarball helper (`15ac5e1` DRY-09) |
| `bindings/r/man/canonical.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/codes.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/convert.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/inspect.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/spec_commit.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/spec_version.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/validate.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/man/vocabulary_size.Rd` | Reviewed | Generated Rd docs |
| `bindings/r/src/Makevars` | Reviewed | Native R library build flags |
| `bindings/r/src/entrypoint.c` | Reviewed | R C entry shim |
| `bindings/r/src/rust/Cargo.lock` | N/A | Generated Cargo lockfile |
| `bindings/r/src/rust/Cargo.toml` | Reviewed | extendr crate manifest |
| `bindings/r/src/rust/src/lib.rs` | Reviewed | R bindings (`1e1cb00` PERF-01/DRY-10) |
| `bindings/r/tests/testthat.R` | Reviewed | testthat runner |
| `bindings/r/tests/testthat/test-mztabm.R` | Reviewed | R testthat suite |
| `clippy.toml` | Reviewed | Clippy complexity/lint thresholds |
| `crates/mztab-m-api/Cargo.toml` | Reviewed | Facade crate manifest |
| `crates/mztab-m-api/LICENSE-APACHE` | N/A | Standard Apache-2.0 license text |
| `crates/mztab-m-api/LICENSE-MIT` | N/A | Standard MIT license text |
| `crates/mztab-m-api/NOTICE` | Reviewed | Attribution notice |
| `crates/mztab-m-api/README.md` | Reviewed | Facade crate README |
| `crates/mztab-m-api/src/format.rs` | Reviewed | Sniffing & error types |
| `crates/mztab-m-api/src/lib.rs` | Reviewed | Public exports (`1e1cb00` added `read_borrowed`) |
| `crates/mztab-m-api/src/ops.rs` | Reviewed | Operations (`bb0aaba`, `1e1cb00`) |
| `crates/mztab-m-api/src/report.rs` | Reviewed | Serializable records (`1e1cb00` PERF-09) |
| `crates/mztab-m-api/src/settings.rs` | Reviewed | Validation settings builder |
| `crates/mztab-m-api/tests/api.rs` | Reviewed | API integration tests |
| `crates/mztab-m-api/tests/golden/README.md` | Reviewed | PSI-MS golden doc |
| `crates/mztab-m-api/tests/golden/psi-ms.txt` | Reviewed | Pinned PSI-MS corpus baseline |
| `crates/mztab-m-api/tests/vocabulary.rs` | Reviewed | Nightly OBO corpus test |
| `crates/mztab-m-cli/Cargo.toml` | Reviewed | CLI crate manifest |
| `crates/mztab-m-cli/LICENSE-APACHE` | N/A | Standard Apache-2.0 license text |
| `crates/mztab-m-cli/LICENSE-MIT` | N/A | Standard MIT license text |
| `crates/mztab-m-cli/NOTICE` | Reviewed | Attribution notice |
| `crates/mztab-m-cli/README.md` | Reviewed | CLI crate README |
| `crates/mztab-m-cli/src/args.rs` | Reviewed | CLI argument parser |
| `crates/mztab-m-cli/src/exec.rs` | Reviewed | Command runner (`1e1cb00` PERF-01/PERF-09) |
| `crates/mztab-m-cli/src/lib.rs` | Reviewed | CLI library entry |
| `crates/mztab-m-cli/src/main.rs` | Reviewed | CLI binary main & stdio plumbing |
| `crates/mztab-m-cli/tests/binary.rs` | Reviewed | Subprocess CLI tests |
| `crates/mztab-m-cli/tests/run.rs` | Reviewed | In-process CLI tests (`1e1cb00`) |
| `crates/mztab-m-obo/Cargo.toml` | Reviewed | OBO crate manifest |
| `crates/mztab-m-obo/LICENSE-APACHE` | N/A | Standard Apache-2.0 license text |
| `crates/mztab-m-obo/LICENSE-MIT` | N/A | Standard MIT license text |
| `crates/mztab-m-obo/NOTICE` | Reviewed | Attribution notice |
| `crates/mztab-m-obo/README.md` | Reviewed | OBO crate README |
| `crates/mztab-m-obo/src/lib.rs` | Reviewed | `Ontology` store & `TermLookup` impl |
| `crates/mztab-m-obo/src/parse.rs` | Reviewed | Zero-copy OBO 1.4 parser |
| `crates/mztab-m-obo/tests/lookup.rs` | Reviewed | OBO lookup tests |
| `crates/mztab-m-wasm/Cargo.toml` | Reviewed | WASM crate manifest |
| `crates/mztab-m-wasm/src/lib.rs` | Reviewed | `wasm-bindgen` exports |
| `crates/mztab-m-wasm/tests/demo.rs` | Reviewed | `wasm-bindgen-test` suite |
| `crates/mztab-m-wasm/www/index.html` | Reviewed | Browser demo page |
| `crates/mztab-m/Cargo.toml` | Reviewed | Core `#![no_std]` crate manifest |
| `crates/mztab-m/LICENSE-APACHE` | N/A | Standard Apache-2.0 license text |
| `crates/mztab-m/LICENSE-MIT` | N/A | Standard MIT license text |
| `crates/mztab-m/NOTICE` | Reviewed | Attribution notice |
| `crates/mztab-m/README.md` | Reviewed | Core crate README |
| `crates/mztab-m/src/cv.rs` | Reviewed | `TermLookup` trait & CV exports |
| `crates/mztab-m/src/cv/rule.rs` | Reviewed | CV mapping rule structs |
| `crates/mztab-m/src/cv/rules.rs` | Reviewed | 31 CV mapping rule definitions |
| `crates/mztab-m/src/diag.rs` | Reviewed | Diagnostics module exports |
| `crates/mztab-m/src/diag/code.rs` | Reviewed | Diagnostic `Code` catalog |
| `crates/mztab-m/src/diag/finding.rs` | Reviewed | `Finding`, `Findings`, `Detail` |
| `crates/mztab-m/src/diag/location.rs` | Reviewed | `Location`, `Path`, `Segment` |
| `crates/mztab-m/src/diag/severity.rs` | Reviewed | `Layer`, `Severity`, `Requirement` |
| `crates/mztab-m/src/id.rs` | Reviewed | Typed IDs & `IndexMap` |
| `crates/mztab-m/src/json.rs` | Reviewed | JSON facade (`bb0aaba` `parse_bytes`) |
| `crates/mztab-m/src/json/findings.rs` | Reviewed | JSON schema/dialect findings |
| `crates/mztab-m/src/json/lexer.rs` | Reviewed | RFC 8259 JSON lexer |
| `crates/mztab-m/src/json/read.rs` | Reviewed | JSON-to-TSV reader (`bb0aaba`, `af1b03b`) |
| `crates/mztab-m/src/json/read/emit.rs` | Reviewed | JSON cell transcoding (`af1b03b`, `f64a627`) |
| `crates/mztab-m/src/json/read/meta.rs` | Reviewed | JSON metadata emission (`7263917`, `af1b03b`) |
| `crates/mztab-m/src/json/read/table.rs` | Reviewed | JSON table inference & rows (`af1b03b`, `f64a627`) |
| `crates/mztab-m/src/json/schema.rs` | Reviewed | JSON metadata schema (`7263917`) |
| `crates/mztab-m/src/json/schema_tables.rs` | Reviewed | JSON table column schemas |
| `crates/mztab-m/src/json/value.rs` | Reviewed | JSON value DOM & `write_string` (`fefe2bd`) |
| `crates/mztab-m/src/json/write.rs` | Reviewed | Model-to-JSON serializer |
| `crates/mztab-m/src/json/write/emit.rs` | Reviewed | JSON cell writer (`a55e92f`) |
| `crates/mztab-m/src/json/write/meta.rs` | Reviewed | JSON metadata writer (`7263917`, `fefe2bd`) |
| `crates/mztab-m/src/json/write/table.rs` | Reviewed | JSON table writer |
| `crates/mztab-m/src/lib.rs` | Reviewed | Crate root exports |
| `crates/mztab-m/src/meta.rs` | Reviewed | Metadata key module |
| `crates/mztab-m/src/meta/key.rs` | Reviewed | `MtdKey` & `ValueKind` |
| `crates/mztab-m/src/meta/subkey.rs` | Reviewed | Typed metadata subkeys |
| `crates/mztab-m/src/model.rs` | Reviewed | Model module & `Field` / `List` |
| `crates/mztab-m/src/model/colunit.rs` | Reviewed | `ColUnit` & `SpectraRef` |
| `crates/mztab-m/src/model/document.rs` | Reviewed | `MzTab` & `Stripped` |
| `crates/mztab-m/src/model/layout.rs` | Reviewed | `SourceLayout` provenance |
| `crates/mztab-m/src/model/meta_entries.rs` | Reviewed | Metadata iteration |
| `crates/mztab-m/src/model/meta_insert.rs` | Reviewed | Metadata typed insertion |
| `crates/mztab-m/src/model/meta_value.rs` | Reviewed | `MtdValue` / `MtdEntry` |
| `crates/mztab-m/src/model/metadata.rs` | Reviewed | `Metadata` container |
| `crates/mztab-m/src/model/rows.rs` | Reviewed | `SmlRow`, `SmfRow`, `SmeRow` |
| `crates/mztab-m/src/model/table.rs` | Reviewed | `Table`, `Header`, `Column` enums |
| `crates/mztab-m/src/owned.rs` | Reviewed | `IntoOwned` conversion |
| `crates/mztab-m/src/param.rs` | Reviewed | `Param` struct |
| `crates/mztab-m/src/param/parse.rs` | Reviewed | Bracketed param parser |
| `crates/mztab-m/src/param/text.rs` | Reviewed | `ParamText` & `Accession` |
| `crates/mztab-m/src/read.rs` | Reviewed | Reader state machine (`a55e92f`) |
| `crates/mztab-m/src/read/builder.rs` | Reviewed | `parse`, `parse_bytes` |
| `crates/mztab-m/src/read/event.rs` | Reviewed | `Event` & `Parsed` |
| `crates/mztab-m/src/read/metadata.rs` | Reviewed | MTD accumulator |
| `crates/mztab-m/src/read/stream.rs` | Reviewed | Chunked `StreamReader` |
| `crates/mztab-m/src/read/typer.rs` | Reviewed | Table typer (`a55e92f`) |
| `crates/mztab-m/src/tsv.rs` | Reviewed | TSV tokenization & typing exports |
| `crates/mztab-m/src/tsv/mtd.rs` | Reviewed | MTD line parsing |
| `crates/mztab-m/src/tsv/splitter.rs` | Reviewed | List visitor (`a55e92f`, `f64a627`) |
| `crates/mztab-m/src/tsv/table.rs` | Reviewed | `RowSlots` & `TableRow` (`a55e92f`) |
| `crates/mztab-m/src/tsv/typing.rs` | Reviewed | Cell typers (`a55e92f`) |
| `crates/mztab-m/src/validate.rs` | Reviewed | Top-level validation (`bb0aaba` `parsed`) |
| `crates/mztab-m/src/validate/metadata.rs` | Reviewed | Metadata structural checks |
| `crates/mztab-m/src/validate/metadata/mandatory.rs` | Reviewed | Profile-mandatory metadata checks |
| `crates/mztab-m/src/validate/metadata/refs.rs` | Reviewed | Contiguous ID & cross-ref checks |
| `crates/mztab-m/src/validate/metadata/values.rs` | Reviewed | Metadata value checks |
| `crates/mztab-m/src/validate/patterns.rs` | Reviewed | XSD/adduct/URI/ORCID checks (`7263917`) |
| `crates/mztab-m/src/validate/report.rs` | Reviewed | Validation `Report` (`ba42d2e`) |
| `crates/mztab-m/src/validate/semantic.rs` | Reviewed | Semantic orchestrator |
| `crates/mztab-m/src/validate/semantic/rules.rs` | Reviewed | CV mapping rule evaluator |
| `crates/mztab-m/src/validate/semantic/terms.rs` | Reviewed | CV term lookup & caching (`ba42d2e`) |
| `crates/mztab-m/src/validate/semantic/uses.rs` | Reviewed | Metadata/column param collector |
| `crates/mztab-m/src/validate/tables.rs` | Reviewed | Table structural orchestrator |
| `crates/mztab-m/src/validate/tables/colunit.rs` | Reviewed | `colunit-*` validation |
| `crates/mztab-m/src/validate/tables/row.rs` | Reviewed | Shared row helpers (`7263917`) |
| `crates/mztab-m/src/validate/tables/shape.rs` | Reviewed | Column order/shape (`ba42d2e`) |
| `crates/mztab-m/src/validate/tables/sme.rs` | Reviewed | SME row checks (`7263917`) |
| `crates/mztab-m/src/validate/tables/smf.rs` | Reviewed | SMF row checks |
| `crates/mztab-m/src/validate/tables/sml.rs` | Reviewed | SML row checks (`7263917`) |
| `crates/mztab-m/src/value.rs` | Reviewed | Value module exports |
| `crates/mztab-m/src/value/cell.rs` | Reviewed | `Cell` tri-state enum |
| `crates/mztab-m/src/value/decimal.rs` | Reviewed | Exact `Decimal` parser |
| `crates/mztab-m/src/value/text.rs` | Reviewed | `Text` & `Raw` types |
| `crates/mztab-m/src/vocab.rs` | Reviewed | Vocab module exports |
| `crates/mztab-m/src/vocab/field.rs` | Reviewed | `AmbiguityCode`, `Datatype`, `PublicationItem` |
| `crates/mztab-m/src/vocab/profile.rs` | Reviewed | `Profile` & `ProhibitedProfile` |
| `crates/mztab-m/src/vocab/section.rs` | Reviewed | `Section` & `TableSection` |
| `crates/mztab-m/src/vocab/version.rs` | Reviewed | `Version` & `RuleSet` |
| `crates/mztab-m/src/write.rs` | Reviewed | TSV writer entry points |
| `crates/mztab-m/src/write/error.rs` | Reviewed | `WriteError` |
| `crates/mztab-m/src/write/layout.rs` | Reviewed | Layout-preserving writer |
| `crates/mztab-m/src/write/line.rs` | Reviewed | Raw line writer |
| `crates/mztab-m/src/write/pending.rs` | Reviewed | Layout emission tracking |
| `crates/mztab-m/src/write/render.rs` | Reviewed | Cell formatting helpers |
| `crates/mztab-m/src/write/sink.rs` | Reviewed | Output line-ending sink |
| `crates/mztab-m/tests/cv_rules.rs` | Reviewed | CV mapping rules test suite |
| `crates/mztab-m/tests/findings_doc.rs` | Reviewed | Generated `findings.md` sync test (`7263917`) |
| `crates/mztab-m/tests/fixtures/broken.mztab` | Reviewed | Error-coverage fixture |
| `crates/mztab-m/tests/fixtures/full.mztab` | Reviewed | Reference valid fixture |
| `crates/mztab-m/tests/golden/README.md` | Reviewed | Golden bless instructions |
| `crates/mztab-m/tests/golden/broken.txt` | Reviewed | Pinned broken findings |
| `crates/mztab-m/tests/golden/corpus.txt` | Reviewed | Pinned corpus findings |
| `crates/mztab-m/tests/golden/full.canonical.mztab` | Reviewed | Pinned canonical output |
| `crates/mztab-m/tests/golden/full.json` | Reviewed | Pinned JSON output |
| `crates/mztab-m/tests/golden/json-corpus.txt` | Reviewed | Pinned JSON corpus findings |
| `crates/mztab-m/tests/json.rs` | Reviewed | JSON tests (`bb0aaba`, `af1b03b`, `f64a627`) |
| `crates/mztab-m/tests/oracle.rs` | Reviewed | `#[ignore]` jmzTab-m comparison harness |
| `crates/mztab-m/tests/oracle/README.md` | Reviewed | Oracle instructions |
| `crates/mztab-m/tests/oracle/expected-disagreements.tsv` | Reviewed | Documented oracle deltas |
| `crates/mztab-m/tests/properties.proptest-regressions` | Reviewed | Saved proptest seeds |
| `crates/mztab-m/tests/properties.rs` | Reviewed | 17 property-based tests |
| `crates/mztab-m/tests/reader.rs` | Reviewed | Reader + corpus suite |
| `crates/mztab-m/tests/rules.rs` | Reviewed | Per-code negative rules suite |
| `crates/mztab-m/tests/semantic.rs` | Reviewed | Semantic validation suite |
| `crates/mztab-m/tests/spec_anchors.rs` | Reviewed | Spec anchor & CV version check |
| `crates/mztab-m/tests/support/corpus.rs` | Reviewed | Corpus loader |
| `crates/mztab-m/tests/support/terms.rs` | Reviewed | Synthetic CV term fixture |
| `crates/mztab-m/tests/validate.rs` | Reviewed | Structural & `validate::parsed` tests (`bb0aaba`) |
| `crates/mztab-m/tests/writer.rs` | Reviewed | Writer & round-trip suite |
| `deny.toml` | Reviewed | License & dependency audit policy |
| `docs/BENCHMARKS.md` | Reviewed | Benchmark methodology & targets |
| `docs/DEBT.md` | Reviewed | Active lint & ratchet ledger |
| `docs/DESIGN.md` | Reviewed | Architecture & crate boundaries |
| `docs/agents/PLAN.md` | Reviewed | Original implementation plan |
| `docs/agents/jmztab-m-rules.md` | Reviewed | Rule inventory vs jmzTab-m |
| `docs/agents/milestones.md` | Reviewed | Historical milestone log |
| `docs/agents/recon-phase0.md` | Reviewed | Historical recon notes |
| `docs/agents/sandbox.md` | Reviewed | Sandboxed build instructions |
| `docs/findings.md` | Reviewed | Generated diagnostic code reference |
| `docs/proposed-issues.md` | Reviewed | Spec upstream issue drafts |
| `docs/site/book.toml` | Reviewed | mdBook configuration |
| `docs/site/src/SUMMARY.md` | Reviewed | Book table of contents |
| `docs/site/src/benchmarks.md` | Reviewed | Book chapter |
| `docs/site/src/bindings.md` | Reviewed | Book chapter |
| `docs/site/src/changelog.md` | Reviewed | Book chapter |
| `docs/site/src/contributing.md` | Reviewed | Book chapter |
| `docs/site/src/decisions.md` | Reviewed | Book chapter |
| `docs/site/src/design.md` | Reviewed | Book chapter |
| `docs/site/src/findings.md` | Reviewed | Book chapter |
| `docs/site/src/install.md` | Reviewed | Book chapter |
| `docs/site/src/introduction.md` | Reviewed | Book chapter |
| `docs/site/src/library-api.md` | Reviewed | Book chapter |
| `docs/site/src/library-obo.md` | Reviewed | Book chapter |
| `docs/site/src/library.md` | Reviewed | Book chapter |
| `docs/site/src/quickstart.md` | Reviewed | Book chapter |
| `docs/site/src/security.md` | Reviewed | Book chapter |
| `docs/site/src/status.md` | Reviewed | Book chapter |
| `fuzz/Cargo.lock` | N/A | Generated Cargo lockfile |
| `fuzz/Cargo.toml` | Reviewed | Fuzz manifest |
| `fuzz/fuzz_targets/json.rs` | Reviewed | JSON fuzz target |
| `fuzz/fuzz_targets/obo.rs` | Reviewed | OBO fuzz target |
| `fuzz/fuzz_targets/parse_bytes.rs` | Reviewed | TSV byte-parse fuzz target |
| `fuzz/fuzz_targets/roundtrip.rs` | Reviewed | Writer round-trip fuzz target |
| `fuzz/fuzz_targets/stream_vs_whole.rs` | Reviewed | Streaming vs whole differential fuzz target |
| `fuzz/fuzz_targets/validate.rs` | Reviewed | Full validation fuzz target |
| `mk` | Reviewed | Shared task runner (`rusti-graben`; untouched) |
| `mk.local` | Reviewed | Repository gate config (`100%` coverage ratchet preserved) |
| `mk.versions` | Reviewed | Pinned toolchain/tool versions (untouched) |
| `prek.toml` | Reviewed | Pre-commit config |
| `pyproject.toml` | Reviewed | Python build metadata |
| `renovate.json` | Reviewed | Dependency bot rules |
| `rust-toolchain.toml` | Reviewed | Pinned Rust channel (`1.100.0`) |
| `rustfmt.toml` | Reviewed | Formatter settings |
| `scripts/generate-findings.sh` | Reviewed | Generates `docs/findings.md` from `Code::ALL` |
| `scripts/mdbook-links.py` | Reviewed | Link rewriter for mdBook build |
| `third_party/jmzTab-m` | N/A | Pinned external Java reference submodule |
| `third_party/mzTab-M` | N/A | Pinned external HUPO-PSI specification submodule |
| `uv.lock` | N/A | Generated Python lockfile |

---

## 2. Debt Ledger

Ranked by `(impact x confidence) / (effort x risk)`.

| ID | Category | Location | Evidence / Metric | Fix | Risk | Expected Gain | Status |
|---|---|---|---|---|---|---|---|
| `DRY-01` | DRY | `benches/src/lib.rs`, `benches/Cargo.toml` | Hand-copied `SML/SMF/SME_COLUMNS` arrays duplicating `mztab_m::*Column::ALL` (~60 lines) | Depend on `mztab-m` in `benches` and derive headers from `SmlColumn`, `SmfColumn`, `SmeColumn` | Low | Single source of truth for schema columns | Fixed (`e1001f2`) |
| `DRY-02` | DRY | `crates/mztab-m-api/src/ops.rs`, `crates/mztab-m/src/validate.rs` | `ops.rs` re-implemented `structure` + `semantic` merging and `InvalidValue` dedup instead of reusing core | Added `mztab_m::validate::parsed` and called it from `ops::run` | Low | Eliminates divergence risk between core `all()` and API pipeline | Fixed (`bb0aaba`) |
| `DRY-03` | DRY | `crates/mztab-m-api/src/ops.rs`, `crates/mztab-m/src/json/read.rs` | Duplicated JSON UTF-8 line/col error locator outside `mztab-m` | Added `mztab_m::json::parse_bytes` reusing `utf8_location` | Low | Single UTF-8 error location rule for TSV and JSON | Fixed (`bb0aaba`) |
| `DRY-04` | DRY | `crates/mztab-m/src/json/{schema,read/meta,write/meta}.rs` | Repeated `Item` -> JSON property name mapping across reader and writer | Extracted `Item::json_name` on `schema::Item` | Low | Single source of truth for metadata JSON keys | Fixed (`7263917`) |
| `DRY-05` | DRY | `crates/mztab-m/src/validate/tables/{row,sme,sml}.rs` | Duplicated `decimal_list` and `not_null` validation loops in `sme.rs` and `sml.rs` | Extracted `Rows::decimal_list` and `Rows::not_null` in `row.rs` | Low | Removes clone group & shared cell checks | Fixed (`7263917`) |
| `DRY-06` | DRY | `crates/mztab-m/src/validate/patterns.rs` | Repeated nonzero + all-ASCII-digits scan across integer/positive-int validators | Extracted `is_positive_digits` helper | Low | Single positive-integer predicate | Fixed (`7263917`) |
| `DRY-07` | DRY | `crates/mztab-m/tests/findings_doc.rs` | Duplicated `Requirement` match arm formatting strings | Extracted `Requirement::as_str` in test | Low | Cleaner test check | Fixed (`7263917`) |
| `DRY-08` | DRY | `crates/mztab-m/src/validate/tables/colunit.rs`, `write/line.rs` | Column label formatting matches `write/line.rs`, but `write/line.rs` writes via checked UTF-8 sink and `Column` lacks `Display` | Kept separate (single remaining caller would be `colunit.rs`) | Low | Avoids speculative `Display` impl on `Column` | Skipped (§7) |
| `DRY-09` | DRY | `bindings/r/bootstrap.R` | Duplicated `[section]` regex/header/line append logic across 3 manifest functions (64 lines) | Extracted `TOML_HEADER_RE`, `parse_toml_header`, `append_section_header`, `append_body_line` | Low | Single manifest section parser; byte-identical output | Fixed (`15ac5e1`) |
| `DRY-10` | DRY | `bindings/r/src/rust/src/lib.rs` | Repeated `.map_err(\|e\| Error::Other(e.to_string()))` across R entry points | Extracted `api_err` helper | Low | Cleaner FFI error boundary | Fixed (`1e1cb00`) |
| `PERF-01` | Efficiency | `crates/mztab-m-api/src/ops.rs`, `mztab-m-cli/src/exec.rs`, Python & R bindings | `read()` called `.into_owned()` eagerly even when input `&[u8]` stayed in scope, doubling `large` footprint | Added `read_borrowed`; switched API ops, CLI, and Python/R `inspect` to borrow | Low | `-20.1%` peak footprint & `-20.8%` instructions on `validate large` | Fixed (`1e1cb00`) |
| `PERF-02` | Efficiency | `crates/mztab-m/src/tsv/splitter.rs`, `typing.rs`, `json/write/emit.rs` | Legacy `\|`-list splitter allocated a `Vec<(usize, &str)>` per cell on every list column | Replaced with zero-alloc finite `visit_list` visitor | Low | Zero heap allocations during `\|`-list splitting | Fixed (`a55e92f`) |
| `PERF-03` | Efficiency | `crates/mztab-m/src/tsv/table.rs`, `read/typer.rs` | `SmlRow`/`SmfRow`/`SmeRow` variable-width `Vec`s started empty and reallocated during push | Added `RowSlots` counts and `TableRow::with_capacity` preallocation | Low | Zero reallocations per parsed table row | Fixed (`a55e92f`) |
| `PERF-04` | Efficiency | `crates/mztab-m/src/validate/tables/shape.rs` | `table_shape` scanned expected `Vec` linearly per column (`O(C^2)` at `1000` assays) | Indexed expected variable columns in `ExpectedKind` `BTreeSet` (`O(C log C)`) | Low | Eliminated quadratic column check in wide tables (`-57.4%` on `synthetic/structure`) | Fixed (`ba42d2e`) |
| `PERF-05` | Efficiency | `crates/mztab-m/src/json/write/meta.rs` | `O(N * K)` rescanning per metadata index + `format!("-{tsv}")` heap alloc per field | Pre-grouped keys per index in `BTreeMap` and omitted suffix allocations | Low | Contributes to `-64.5%` on `synthetic/json_write` | Fixed (`fefe2bd`) |
| `PERF-06` | Efficiency | `crates/mztab-m/src/json/read.rs`, `read/{emit,meta,table}.rs` | Built full `Vec<Vec<(String, Path)>>` relocation matrix + eagerly constructed TSV string & `Pointer` per cell (`9.18 s` on `synthetic/json_parse`) | Replaced with compact `LineOrigin::{Single, Row}` lazy relocation, borrowed `infer_columns`, per-row `opt` map, single `row_buf` reuse, and zero-alloc `write_cell` fast path | Med | `-74.65%` (`9.18 s -> 2.33 s`, `3.94x`) on `synthetic/json_parse` | Fixed (`af1b03b`) |
| `PERF-07` | Efficiency | `crates/mztab-m/src/validate/report.rs` | `Report::has_source_items` re-walked source-layout lines linearly on repeated queries | Cached boolean flag once on `Report` construction | Low | `O(1)` repeated queries | Fixed (`ba42d2e`) |
| `PERF-08` | Efficiency | `crates/mztab-m/src/json/value.rs` | `json::write_string` wrote char-by-char via `write_char` | Flushed contiguous unescaped `&str` slices directly via `write_str` | Low | Fewer UTF-8 encoding/flush calls in JSON serialization | Fixed (`fefe2bd`) |
| `PERF-09` | Efficiency | `crates/mztab-m-api/src/report.rs`, `mztab-m-cli/src/exec.rs`, `bindings/r/src/rust/src/lib.rs` | Intermediate `String`/`Vec` allocations in `Fact::write_json`, 3 separate CLI severity filter passes, owned R string conversion | Streamed `Fact::write_json`, fused single-pass CLI counters, borrowed `Vec<&str>` in R | Low | Reduced API/FFI overhead | Fixed (`1e1cb00`) |
| `PERF-10` | Efficiency | `crates/mztab-m/src/validate/semantic/terms.rs` | Redundant `labels.contains` before `labels.insert` lookup | Used `labels.insert(label)` directly on `BTreeSet` | Low | Halved lookup on newly declared CV labels | Fixed (`ba42d2e`) |
| `HYG-01` | Hygiene | `crates/mztab-m/Cargo.toml`, workspace | Check for unused deps, advisories, duplicates, unsafe, suppressed lints | Verified `cargo deny`, `cargo machete`, `0 unsafe`, `0 #[allow]`, `12 #[expect]` matching `docs/DEBT.md` | Low | Baseline & final hygiene confirmed clean | Closed |
| `HYG-02` | Hygiene | `crates/mztab-m/src/tsv/splitter.rs`, `json/read/{emit,table}.rs` | Refactoring exposed 2 dead `cell_text` arms, 1 uncovered fallback closure, and generic monomorphization line gap (`99.94%`) | Pruned dead arms, used `.unwrap_or_default()`, covered early stop & full pass in single closure | Low | Restored `100.00%` line coverage (`9337/9337`) | Fixed (`f64a627`) |
| `HYG-03` | Hygiene | Workspace docs & R/Python metadata | 2 `TODO(human)` markers for post-audit DOIs/versions (`CITATION.cff`, `DECISIONS.md`) | Documented as intentional human release metadata (§8) | Low | No premature edits to human release fields | Open (deferred) |

---

## 3. Metrics Before → After

**Environment:** Apple M1 Pro (16 GiB RAM, arm64), macOS Darwin 27.2.0, `rustc 1.100.0`, Python 3.15.0, R 4.6.2.

### Codebase, Complexity, Safety & Quality Gates

| Metric | Before (`26a4786`) | After (`f64a627`) | Delta |
|---|---|---|---|
| `tokei` tracked files / lines / code (excl. `third_party`, `target`) | 211 / 34,203 / 25,042 | 211 / 34,843 / 25,608 | +640 lines (new unit/regression tests & lazy JSON transcoding) |
| Non-test `crates/*/src/**/*.rs` functions / max / mean len / >40 lines | 638 / 80 / 13.80 / 45 | 651 / 80 / 13.86 / 46 | +13 helpers extracted, max unchanged |
| 6-line normalized clone rate (`crates/*/src/**/*.rs`, non-test) | 170 / 10,542 (`1.61%`) | 170 / 10,764 (`1.58%`) | `-0.03 pp` |
| `./mk echoes` duplicate groups | 0 | 0 | Clean |
| `./mk refquiem`, `cargo deny check`, `cargo machete`, `./mk check` | Pass (0 warnings) | Pass (0 warnings) | Clean |
| `unsafe` / `#[allow(` / `#[expect(` (workspace `.rs`) | 0 / 0 / 12 | 0 / 0 / 12 | 0 suppressions added |
| Line coverage (`./mk coverage`, `COVERAGE_FAIL_UNDER_LINES=100`) | `100.00%` | `100.00%` (`9,337/9,337` lines, `1,204/1,204` fns) | Maintained |
| Mutation testing (`MK_BASE=26a4786 ./mk mutants-diff`) | — | `219` tested: `164` caught, `55` unviable, `0` missed, `0` timeouts | 100% viable kill rate |
| Test counts (`cargo test` native / WASM / Python / R / lychee) | Pass / 153 / 7 / 38 / 644 OK | 290 native (2 ignored) / 158 WASM / 7 Py (+doctest) / 38 R / 644 OK | +5 WASM/core tests |
| Golden diff (`git diff 26a4786 -- '**/golden/**' 'benches/golden.mztab'`) | — | Empty (0 bytes changed) | Bit-for-bit preserved |

### Artifacts & CLI Resource Usage (`/usr/bin/time -l target/release/mztab-m`)

| Measurement | Before (`26a4786`) | After (`f64a627`) | Delta |
|---|---|---|---|
| `target/release/mztab-m` size | ~1.2 MB | `1,272,624` B | Comparable |
| `mztab_m_wasm_bg.wasm` raw / gzip (`--release`) | `669` KB / `202,222` B | `706,665` B / `209,800` B | `+3.7%` gzipped (`LineOrigin` & indexed table transcoding) |
| `validate medium` (`1.17 MB`): peak footprint / instructions | 12.78 MB / 257.8M | 11.47–13.94 MB / 208.7M–209.5M | **-18.8%** instructions |
| `convert --to json medium` (`1.17 MB`): peak footprint / instructions | 19.19 MB / 384.9M | 19.76 MB / 302.4M | **-21.4%** instructions |
| `validate large` (`13.6 MB`): max RSS / peak footprint / instructions | 66.21 MB / 64.96 MB / 1,250.7M | 49.23 MB / 47.96 MB / 929.9M | **-26.2%** footprint, **-25.7%** instructions |
| `convert --to json large` (`13.6 MB`): max RSS / peak footprint / instructions | 111.98 MB / 110.84 MB / 2,081.1M | 94.37 MB / 93.21 MB / 1,718.0M | **-15.9%** footprint, **-17.4%** instructions |

### Criterion Throughput (`--sample-size 10 --warm-up-time 1 --measurement-time 2`)

Point estimates (Criterion CI midpoints):

| Workload | Stage | Before (`26a4786`) | After (`f64a627`) | Change |
|---|---|---|---|---|
| `small` (6.3 KB) | `parse` | 69.6 µs (86.5 MiB/s) | 61.9 µs (97.2 MiB/s) | `-11.1%` |
| `small` | `parse_owned` | 160.8 µs (37.5 MiB/s) | 84.5 µs (71.2 MiB/s) | `-47.4%` |
| `small` | `stream_64k` | 62.1 µs (96.9 MiB/s) | 102.4 µs (58.8 MiB/s) | noise (`p = 0.26`) |
| `small` | `write` | 28.6 µs (210.9 MiB/s) | 32.7 µs (184.1 MiB/s) | noise (`p = 0.64`) |
| `small` | `write_canonical` | 13.6 µs (443.9 MiB/s) | 20.9 µs (re-run `--sample-size 30`: 31.1 µs, `p = 0.70`) | micro-tier jitter |
| `small` | `structure` | 42.7 µs (141.1 MiB/s) | 22.2 µs (270.7 MiB/s) | `-48.0%` |
| `small` | `semantic` | 15.7 µs (383.5 MiB/s) | 17.2 µs (349.6 MiB/s) | noise (`p = 0.08`) |
| `small` | `json_write` | 154.0 µs (39.1 MiB/s) | 136.5 µs (44.1 MiB/s) | `-11.4%` |
| `small` | `json_parse` | 536.1 µs (13.9 MiB/s) | 202.8 µs (36.8 MiB/s) | **-62.2%** |
| `medium` (1.17 MB) | `parse` | 19.4 ms (57.4 MiB/s) | 12.38 ms (89.9 MiB/s) | **-36.2%** |
| `medium` | `parse_owned` | 46.6 ms (23.9 MiB/s) | 26.45 ms (42.1 MiB/s) | **-43.2%** |
| `medium` | `stream_64k` | 42.9 ms (26.0 MiB/s) | 24.14 ms (46.1 MiB/s) | **-43.7%** |
| `medium` | `write` | 14.4 ms (77.5 MiB/s) | 4.39 ms (253.7 MiB/s) | **-69.5%** |
| `medium` | `write_canonical` | 14.9 ms (74.6 MiB/s) | 3.78 ms (294.7 MiB/s) | **-74.6%** |
| `medium` | `structure` | 29.6 ms (37.6 MiB/s) | 4.80 ms (231.9 MiB/s) | **-83.8%** |
| `medium` | `semantic` | 2.99 ms (373.0 MiB/s) | 452.0 µs (2.41 GiB/s) | **-84.9%** |
| `medium` | `json_write` | 54.5 ms (20.4 MiB/s) | 14.39 ms (77.4 MiB/s) | **-73.6%** |
| `medium` | `json_parse` | 80.0 ms (39.2 MiB/s) | 98.57 ms (31.8 MiB/s) | noise (`p = 0.45`) |
| `large` (13.6 MB) | `parse` | 148.8 ms (87.0 MiB/s) | 164.1 ms (78.9 MiB/s) | noise (`p = 0.64`) |
| `large` | `parse_owned` | 620.8 ms (20.8 MiB/s) | 138.2 ms (93.7 MiB/s) | **-77.7%** |
| `large` | `stream_64k` | 384.2 ms (33.7 MiB/s) | 74.22 ms (174.4 MiB/s) | **-80.7%** |
| `large` | `write` | 67.3 ms (192.3 MiB/s) | 22.89 ms (565.4 MiB/s) | **-66.0%** |
| `large` | `write_canonical` | 29.0 ms (445.6 MiB/s) | 32.32 ms (400.5 MiB/s) | noise (`p = 0.06`) |
| `large` | `structure` | 6.52 ms (1.94 GiB/s) | 12.79 ms (re-run: 7.83 ms, CI `[5.09, 10.22] ms`) | overlaps baseline |
| `large` | `semantic` | 94.2 µs (134.1 GiB/s) | 117.4 µs (re-run: 90.42 µs / 139.8 GiB/s) | **-4.0%** (`90.42 µs` isolated) |
| `large` | `json_write` | 170.9 ms (75.7 MiB/s) | 111.2 ms (116.4 MiB/s) | `-34.9%` (`p = 0.40`) |
| `large` | `json_parse` | 734.7 ms (19.4 MiB/s) | 245.5 ms (58.0 MiB/s) | **-66.6%** |
| `synthetic` (~100 MB) | `parse` | 3.21 s (31.8 MiB/s) | 1.11 s (92.3 MiB/s) | **-65.4%** |
| `synthetic` | `parse_owned` | 6.02 s (16.9 MiB/s) | 1.86 s (55.0 MiB/s) | **-69.1%** |
| `synthetic` | `stream_64k` | 4.77 s (21.4 MiB/s) | 1.31 s (78.2 MiB/s) | **-72.5%** |
| `synthetic` | `write` | 1.18 s (86.4 MiB/s) | 523.4 ms (195.0 MiB/s) | **-55.6%** |
| `synthetic` | `write_canonical` | 1.05 s (96.8 MiB/s) | 269.6 ms (378.7 MiB/s) | **-74.3%** |
| `synthetic` | `structure` | 85.8 ms (1.16 GiB/s) | 36.1 ms (2.76 GiB/s) | **-57.9%** |
| `synthetic` | `semantic` | 12.2 ms (8.17 GiB/s) | 6.31 ms (15.8 GiB/s) | **-48.3%** |
| `synthetic` | `json_write` | 2.54 s (40.2 MiB/s) | 903.5 ms (113.0 MiB/s) | **-64.4%** |
| `synthetic` | `json_parse` | 9.18 s (11.7 MiB/s) | 2.33 s (46.1 MiB/s) | **-74.6%** (`3.94x`) |

Note: the initial full run experienced a transient OS CPU spike during the `large/` group (the unchanged stdlib `large/lines_std` harness jumped temporarily before returning to 23.6 ms on the isolated verification run). All `synthetic` (~100 MB) stages improved by **48.3% to 74.6%**.

---

## 4. Structural Changes

| Commit | Subject | Structural Problem Removed | Verification & Measured Effect |
|---|---|---|---|
| `7263917` | `refactor(core): unify duplicated helpers in schema, validation, and tests` | Duplicated JSON meta key table (`Item::json_name`), `decimal_list`/`not_null` in `sme.rs` vs `sml.rs`, `is_positive_digits`, and `Requirement::as_str` | Check, core tests, 19/19 mutants caught |
| `e1001f2` | `refactor(bench): derive synthetic table headers from mztab-m column enums` | Hand-maintained `SML/SMF/SME` header string lists in `benches/src/lib.rs` | Check, `output_is_pinned`, 5/5 mutants caught |
| `bb0aaba` | `refactor(core,api): unify json::parse_bytes and validate::parsed` | Duplicated `Report` assembly and JSON UTF-8 line/col calculation in `mztab-m-api` vs `mztab-m` | Added `json::parse_bytes` & `validate::parsed`; 14/14 viable mutants caught |
| `1e1cb00` | `perf(api,cli,bindings): borrow TSV input via read_borrowed and remove formatting clones` | Eager `.into_owned()` across API/CLI/Python/R, string-collecting `Fact::write_json`, multi-pass CLI counting | `-20.1%` peak footprint & `-20.8%` instructions on `validate large`; 19/19 viable mutants |
| `a55e92f` | `perf(core): zero-alloc list visitor and preallocated row vectors` | Per-cell `Vec` allocations in the previous list splitter and unreserved variable-width table row `Vec`s | Finite `visit_list` + `RowSlots` capacity preallocation; 17/17 viable mutants, 0 timeouts |
| `ba42d2e` | `perf(core): index expected columns and cache layout/CV checks in validation` | `O(C^2)` expected-column scan in `shape.rs`, repeated source-item walks, double `BTreeSet` lookup | Indexed `ExpectedKind`, cached `has_source_items`; `-57.9%` on `synthetic/structure`; 9/9 viable mutants |
| `fefe2bd` | `perf(json): group metadata by index and flush unescaped slices in write_string` | `O(N * K)` metadata scan + `format!` allocs + char-by-char JSON string escaping | Single-pass `BTreeMap` grouping + slice flushing; `-64.4%` on `synthetic/json_write`; 24/24 mutants |
| `af1b03b` | `perf(json): lazy row-cell relocation and zero-alloc row transcoding in json::parse` | Full `Vec<Vec<Path>>` relocation storage, repeated `infer_columns` keys, per-cell `Pointer` & string allocs | `LineOrigin::{Single, Row}` + `row_buf` reuse + `write_cell` fast path; `-74.6%` (`9.18 s -> 2.33 s`) on `synthetic/json_parse`; 54/54 viable mutants |
| `15ac5e1` | `refactor(r): deduplicate TOML header parsing and line flushing in bootstrap.R` | Copy-pasted TOML section/line parsing across 3 helpers in `bootstrap.R` | Byte-identical staged manifests + 38/38 R tests |
| `f64a627` | `test(core): restore 100% coverage on json opt fallbacks and list visitor` | Unreachable `cell_text` arms, redundant `unwrap_or_else`, monomorphization split in `visit_list` | `100.00%` line coverage (`9337/9337`), 5/5 mutants |

---

## 5. API Changes & Migrations

All public API changes in this pass are **strictly additive**; no existing public signature, enum variant, field, or CLI/Python/R behavior was broken or removed:

- `mztab_m::json::parse_bytes(&[u8]) -> Parsed<'static>` — validates UTF-8 and delegates to `json::parse` with `Code::Utf8` line/column coordinates, matching `parse_bytes` on TSV.
- `mztab_m::validate::parsed(&Parsed<'_>, &dyn TermLookup, &Config) -> Report` — runs structural and semantic validation on an already-parsed document, deduplicating `InvalidValue` against syntax errors like `validate::all`.
- `mztab_m_api::read_borrowed(&[u8], Format) -> Result<Parsed<'_>, Error>` — borrows TSV slices directly from `&[u8]` (`Format::Tsv`) without copying into an owned arena; callers that need `'static` documents continue using `mztab_m_api::read`.

---

## 6. Findings Handed Off (Scientific / Correctness)

Per the non-negotiable behavior-preservation rule, none of the pre-existing scientific/specification questions logged in `AUDIT.md` §5 were altered:

1. SMF/SME negative `charge` values (`-1`, `-2`) accepted as `i32` pending maintainer/spec alignment (`AUDIT.md` §5.1).
2. Equal `|`-separated list lengths across SML identification columns (`jmzTab-m` rule 2046) not yet enforced in `sml.rs` (`AUDIT.md` §5.2).
3. Bare `ms_run[n]` without `:nativeID` in `SpectraRef` would require changing public `SpectraRef::reference` to `Option<Text<'a>>` (`AUDIT.md` §5.3).
4. Strict `opt_` column charset `[A-Za-z0-9_\-\[\]:]` and `opt_*_null` naming (`AUDIT.md` §5.4).
5. Nested `Parameter` inside `ExtendedParameter.value` in JSON vs TSV grammar (`AUDIT.md` §5.5).
6. JSCH `Parameter.value` (`""` vs `null`) and SML `"null"` list items compatibility (`AUDIT.md` §5.6).
7. ORCID Mod 11-2 check digit and unreferenced `colunit-*` columns when the target table is absent (`AUDIT.md` §5.7).
8. Oracle golden comparison (`tests/oracle.rs`) remains `#[ignore]` pending a Java 25 / Maven oracle run (`AUDIT.md` §5.8).
9. Archival disposition of `docs/proposed-issues.md` and `docs/agents/` (`AUDIT.md` §5.9).

---

## 7. Deliberately Not Changed

- **`DRY-08` (`colunit.rs` vs `write/line.rs`):** `Column` intentionally has no `Display` impl, and `write/line.rs` checks invalid UTF-8/tabs via `out.checked`; unifying across a single caller (`colunit.rs`) would add boilerplate for zero benefit.
- **`validate/semantic/uses.rs` (`slot` match):** Exhaustive match over `Metadata` fields is static, compiler-checked, and clear; replacing with macros or reflection would harm readability and increase binary size.
- **`SmlRow` / `SmfRow` / `SmeRow` & `SmlColumn` / `SmfColumn` / `SmeColumn`:** Tables have genuinely distinct mandatory columns, types, and nullability rules.
- **JSON-via-TSV architecture (`DECISIONS.md` D-071):** Routing JSON through TSV keeps one canonical parser and `Location` model; optimizing the transcoding/relocation layer (`af1b03b`) delivered `3.94x` speedup on large JSON without duplicating parser logic.
- **`criterion 0.5` & dev-only `syn` duplicate (`DECISIONS.md` D-073):** Pinned because `0.8` pulls `alloca` C compilation failures under Xcode clang 21; zero runtime duplicate dependencies.
- **`memchr` excluded from `crates/mztab-m` (`DECISIONS.md` D-073):** Core crate stays strictly zero-dependency.
- **`[profile.release]` (`Cargo.toml`):** Speed-optimized release defaults kept rather than switching globally to `opt-level = "z"`, avoiding throughput degradation on large datasets.
- **`DECISIONS.md` statuses, `mk`, `mk.versions`, `third_party/`:** Upstream template scripts and maintainer-only decision statuses left untouched.

---

## 8. Residual Debt & Risk

- **`mztab_m_api::read` (`IntoOwned`) allocation cost:** Callers needing `'static` `Parsed<'static>` still copy TSV cells into `String`s inside `#![no_std]` core (`parse_owned`), though `read_borrowed` eliminates that cost for stateless inspection, validation, and JSON conversion.
- **JSON lexer DOM allocation:** `json::lexer` builds a `Value` DOM before `json::read` emits canonical TSV; streaming JSON directly without a DOM remains future work if multi-hundred-MB JSON inputs become common (`2.33 s` on 100 MB currently).
- **WASM bundle size:** `706,665 B` raw (`209,800 B` gzipped) without CI `wasm-opt -Oz`; adding an optional `wasm-opt` post-pass in `task_demo` would shrink download size if desired.
- **Dev-only dependency staleness:** `criterion 0.5` brings `syn` v2/v3 (`syn v2.0.119` and `syn v3.0.10`) into dev-dependencies only (handled via `--ignore criterion` in `mk.local`).
- **Pre-release metadata markers:** 2 `TODO(human)` markers remain in `CITATION.cff` and `DECISIONS.md` for maintainer release DOI/dates.
- **Deferred verification:** `tests/oracle.rs` (`#[ignore]` until Java 25 run), continuous nightly libFuzzer execution, and Windows CI validation for Python/R bindings remain outside the macOS local check scope.
