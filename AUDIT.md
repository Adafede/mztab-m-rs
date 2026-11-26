> Provenance: scientific audit and refactoring ledger produced by an AI coding agent on 2026-11-26.

# Scientific and Code Quality Audit (`mztab-m-rs`)

## Phase 0: Baseline & Intended Scientific Specification

### Baseline (pre-edit, HEAD `615ffb0`)

- `./mk check` (formatting, TOML, typos, clippy with `-D warnings`): exited `0`.
- `./mk test-fast` (workspace unit/integration tests via `cargo nextest`): exited `0`.
- `cargo machete`: exited `0` (zero unused workspace dependencies).
- Banned-vocabulary grep over tracked non-lock/non-submodule files: `0` hits.
- Tracked files (`git ls-files`): `268`.

### Intended Scientific Specification

- **Target standard:** HUPO-PSI / MSI mzTab-M `2.1.0-M` draft at pinned commit `e66c19e1d5c2e61cd721cb3bd6134389184cf778` (`third_party/mzTab-M`), plus backwards-compatible `2.0.0-M` validation (`RuleSet::V2_0` vs `RuleSet::V2_1`). Normative precedence (DECISIONS D-002): specification prose (`mzTab_format_specification_2_1-M.adoc`), then field reference (`mzTab_m_2_1_schema.adoc`), then JSON Schema 2020-12 (`schema/mzTab_2_1-M.json`).
- **Number grammar:** `xs:decimal` (`.` decimal separator, no thousands separators) plus `NaN` and `null`; scientific notation (`e`/`E`) and infinities (`INF`/`Infinity`) are explicitly prohibited by SPEC §5.7 (parsed losslessly by the reader and flagged with `MZT-S004`).
- **Study profiles:** `M` (MTD only), `M+S` (MTD + SML), `M+F` (MTD + SMF), `M+F+E` (MTD + SMF + SME), `M+S+F+E` (all four sections), and discouraged legacy `M+S+F` (inferred only, not declarable in `mzTab-profile`). Prohibited combinations: `M+E` and `M+S+E`. Sections MUST appear in order `MTD -> SML -> SMF -> SME`, separated by blank lines.
- **Validation layers:**
  1. `Syntax` (`MZT-S*`): line prefixes, UTF-8 / BOM, cell counts, trailing empty cells, surrounding spaces, cell grammars (`Decimal`, `Param`, `Text`, `Version`, `SpectraRef`, `ColUnit`, `PublicationItem`), RFC 8259 JSON syntax/shape.
  2. `Structure` (`MZT-T*`): mandatory and profile-dependent metadata, contiguous 1-based element indices, cross-element/cross-table references (`SMF_ID_REFS`, `SME_ID_REFS`, `SME_ID_REF_ambiguity_code` in `{1,2,3}`), table column presence/order/`opt_` placement, nullability, adduct pattern, ORCID pattern, URI (RFC 3986), e-mail, native spectrum ID formats (SPEC Table 1), `study_variable_group` membership and XSD/`Parameter` datatype conformance, `colunit-*` column matching.
  3. `Semantic` (`MZT-X*`): `cv[n]-label` declaration/usage, `PREFIX:LOCAL` accession syntax, vocabulary term existence, primary name / `EXACT` synonym match, obsolescence, `has_value_type` / `xref: value-type:` XSD type check, CV version check, and the 31 CV-mapping rules (`mzTab_2_1-M_mapping.xml` + D-068/D-077 prose corrections).
- **Controlled-vocabulary lookup (`mztab-m-obo`):** Zero-copy OBO 1.4 parser covering its declared `ontology:` namespace (or all prefixes when headerless), traversing `is_a` and `relationship: part_of` ancestors for strict-descendant checks (`is_descendant(a, a) == false`).
- **JSON encoding (`feature = "json"`):** JSCH 2020-12 dialect routed via canonical TSV text, preserving numeric lexemes via exact decimal expansion and relocating TSV findings to RFC 6901 JSON pointers.

### Canonical Domain Glossary

| Term | Canonical meaning | Domain / Units |
|---|---|---|
| `m/z` (`exp_mass_to_charge`, `theoretical_mass_to_charge`) | Mass-to-charge ratio of a precursor/adduct ion | Dimensionless (`m/z`, Thomson); positive decimal |
| `theoretical_neutral_mass` | Calculated monoisotopic neutral mass of the uncharged molecule | Dalton (Da, u); `\|`-separated list matching `database_identifier` |
| `charge` | Integer charge state of the observed/theoretical ion | Integer (spec prose states positive integer for both polarities; corpus also contains signed integers — see Open Questions) |
| `adduct_ion` / `adduct_ions` | IUPAC 2013 adduct notation matching `^\[\d*M([+-][\w\d]+)*\]\d*[+-]$` | String / `\|`-list |
| `retention_time_in_seconds` (`_start`, `_end`) | Chromatographic apex (and window bounds) | Seconds (s); `start <= apex <= end` |
| `abundance_assay` / `abundance_study_variable` / `abundance_variation_study_variable` | Quantitative signal, group aggregate (`average_function`), and dispersion (`variation_function`) | Unit declared in `small_molecule*-quantification_unit` |
| `reliability` | Identification confidence tier | Default MSI 1–4 (`"1"`–`"4"`) unless `small_molecule-identification_reliability` overrides |
| `SME_ID_REF_ambiguity_code` | Disambiguation code required iff `SME_ID_REFS` has > 1 entry | `1` (ambiguous ID), `2` (multiple evidence streams, one ID), `3` (both) |
| `Profile` | Presence of tables (`M`, `M+S`, `M+F`, `M+F+E`, `M+S+F+E`, tolerated `M+S+F`) | Enum `Profile`; prohibited `M+E`, `M+S+E` in `ProhibitedProfile` |
| `Section` / `TableSection` | Document section (`Mtd`, `Sml`, `Smf`, `Sme`) vs data tables only (`Sml`, `Smf`, `Sme`) | Enums |
| `Finding` / `Code` / `Layer` / `Severity` | Diagnostic `(Code, Severity, Location, Detail)` on one of `{Syntax, Structure, Semantic}` at `{Info, Warning, Error}` | Types in `mztab_m::diag` |
| `Param` | `[CV label, accession, name, value]` (both label+accession for `Cv`, neither for `User`, non-empty `name`) | Enum `Param<'a>` |
| `Accession` | Non-empty CV term identifier; split at first `:` into `(prefix, local)` | `Accession<'a>` |
| `Ontology` / `TermLookup` | OBO 1.4 term store (`mztab-m-obo`) implementing `mztab_m::cv::TermLookup` | Read-only lookup trait |
| `SpectraRef` | Spectrum reference linking `ms_run[n]` to a nativeID string (`ms_run[n]:nativeID`) | `SpectraRef<'a>` |
| `ColUnit` | Column-unit mapping (`column_name=[param]`) | `ColUnit<'a>` |
| `SourceLayout` | Per-line provenance (order, line endings, BOM, verbatim non-canonical lines with FNV-1a fingerprint) | `SourceLayout<'a>` |

### Module & Data-Flow Map

- **Layer 0 (`crates/mztab-m`, `#![no_std]` + `alloc`, zero deps, zero I/O):** `id`, `value`, `param`, `vocab` -> `meta`, `model`, `diag`, `cv` -> `tsv`, `write`, `read`, `owned`, `json` (optional) -> `validate` (`syntax`, `structure`, `semantic`, `all`).
- **Layer 1 (`crates/mztab-m-obo`, `#![no_std]` + `alloc`):** `Ontology::parse(&str)` -> `impl TermLookup`.
- **Layer 2 (`crates/mztab-m-api`, `#![no_std]` + `alloc`):** `Format::sniff`, `read`, `check`, `validate`, `validate_json`, `write`, `convert`, `facts`, `facts_json`, `codes`, `findings_json`, `vocabulary_size`, `Settings`.
- **Layer 3 (Adapters):** `crates/mztab-m-cli` (`mztab-m` binary), `crates/mztab-m-wasm` (`wasm-bindgen` + `www/index.html`), `bindings/python` (`mztab_m` via PyO3), `bindings/r` (`mztabm` via extendr).
- **Support crates (outside shipped libs):** `benches/` (`mztab-m-bench` synthetic generator + criterion/dhat), `fuzz/` (4 `cargo-fuzz` targets).

---

## 1. Coverage Ledger

All 279 files in the final repository (`git ls-files`, comprising 268 baseline files, 1 deleted slop file, and 12 new files including `AUDIT.md`):

| File | Reviewed | Findings |
|---|---|---|
| `.cargo/mutants.toml` | y | 0 |
| `.config/nextest.toml` | y | 0 |
| `.echoes.toml` | y | 0 |
| `.editorconfig` | y | 0 |
| `.github/ISSUE_TEMPLATE/bug_report.md` | y | 0 |
| `.github/ISSUE_TEMPLATE/feature_request.md` | y | 0 |
| `.github/ISSUE_TEMPLATE/question.md` | y | 0 |
| `.github/ISSUE_TEMPLATE/spec_disagreement.md` | y | 0 |
| `.github/PULL_REQUEST_TEMPLATE.md` | y | 0 |
| `.github/actions/setup/action.yml` | y | 0 |
| `.github/dependabot.yml` | y | 1 |
| `.github/workflows/bindings.yml` | y | 1 |
| `.github/workflows/ci.yml` | y | 0 |
| `.github/workflows/nightly.yml` | y | 0 |
| `.github/workflows/oracle.yml` | y | 1 |
| `.github/workflows/pages.yml` | y | 0 |
| `.github/workflows/release.yml` | y | 0 |
| `.gitignore` | y | 0 |
| `.gitmodules` | y | 0 |
| `.refquiem.toml` | y | 0 |
| `.tombi.toml` | y | 0 |
| `.typos.toml` | y | 0 |
| `.vscode/settings.json` | y | 0 |
| `AGENTS.md` | y | 1 |
| `AUDIT.md` | y | 0 |
| `CHANGELOG.md` | y | 0 |
| `CITATION.cff` | y | 1 |
| `CLAUDE.md` | y | 0 |
| `CODE_OF_CONDUCT.md` | y | 0 |
| `CONTRIBUTING.md` | y | 1 |
| `Cargo.lock` | N/A (generated Cargo lockfile) | 0 |
| `Cargo.toml` | y | 1 |
| `DECISIONS.md` | y | 2 |
| `LICENSE-APACHE` | N/A (standard Apache-2.0 license text) | 0 |
| `LICENSE-MIT` | N/A (standard MIT license text) | 0 |
| `NOTICE` | y | 0 |
| `README.md` | y | 1 |
| `SECURITY.md` | y | 0 |
| `STATUS.md` | y | 4 |
| `bacon.toml` | y | 0 |
| `benches/Cargo.toml` | y | 0 |
| `benches/benches/heap.rs` | y | 3 |
| `benches/benches/throughput.rs` | y | 3 |
| `benches/golden.mztab` | y | 1 |
| `benches/src/lib.rs` | y | 5 |
| `bindings/python/Cargo.lock` | N/A (generated Cargo lockfile) | 0 |
| `bindings/python/Cargo.toml` | y | 0 |
| `bindings/python/README.md` | y | 1 |
| `bindings/python/build.rs` | y | 0 |
| `bindings/python/python/mztab_m/__init__.py` | y | 1 |
| `bindings/python/python/mztab_m/_native.pyi` | y | 3 |
| `bindings/python/python/mztab_m/py.typed` | y | 0 |
| `bindings/python/src/lib.rs` | y | 4 |
| `bindings/python/tests/test_mztab_m.py` | y | 1 |
| `bindings/r/.Rbuildignore` | y | 0 |
| `bindings/r/DESCRIPTION` | y | 0 |
| `bindings/r/LICENSE` | N/A (CRAN license file) | 0 |
| `bindings/r/NAMESPACE` | y | 2 |
| `bindings/r/R/extendr-wrappers.R` | y | 0 |
| `bindings/r/R/mztabm.R` | y | 5 |
| `bindings/r/README.md` | y | 1 |
| `bindings/r/bootstrap.R` | y | 1 |
| `bindings/r/man/canonical.Rd` | y | 0 |
| `bindings/r/man/codes.Rd` | y | 0 |
| `bindings/r/man/convert.Rd` | y | 0 |
| `bindings/r/man/inspect.Rd` | y | 0 |
| `bindings/r/man/spec_commit.Rd` | y | 0 |
| `bindings/r/man/spec_version.Rd` | y | 0 |
| `bindings/r/man/validate.Rd` | y | 0 |
| `bindings/r/man/vocabulary_size.Rd` | y | 0 |
| `bindings/r/src/Makevars` | y | 1 |
| `bindings/r/src/entrypoint.c` | y | 0 |
| `bindings/r/src/rust/Cargo.lock` | N/A (generated Cargo lockfile) | 0 |
| `bindings/r/src/rust/Cargo.toml` | y | 0 |
| `bindings/r/src/rust/src/lib.rs` | y | 4 |
| `bindings/r/tests/testthat.R` | y | 0 |
| `bindings/r/tests/testthat/test-mztabm.R` | y | 1 |
| `clippy.toml` | y | 0 |
| `crates/mztab-m-api/Cargo.toml` | y | 0 |
| `crates/mztab-m-api/LICENSE-APACHE` | N/A (standard Apache-2.0 license text) | 0 |
| `crates/mztab-m-api/LICENSE-MIT` | N/A (standard MIT license text) | 0 |
| `crates/mztab-m-api/NOTICE` | y | 0 |
| `crates/mztab-m-api/README.md` | y | 0 |
| `crates/mztab-m-api/src/format.rs` | y | 2 |
| `crates/mztab-m-api/src/lib.rs` | y | 0 |
| `crates/mztab-m-api/src/ops.rs` | y | 1 |
| `crates/mztab-m-api/src/report.rs` | y | 1 |
| `crates/mztab-m-api/src/settings.rs` | y | 1 |
| `crates/mztab-m-api/tests/api.rs` | y | 0 |
| `crates/mztab-m-api/tests/golden/README.md` | y | 0 |
| `crates/mztab-m-api/tests/golden/psi-ms.txt` | y | 0 |
| `crates/mztab-m-api/tests/vocabulary.rs` | y | 1 |
| `crates/mztab-m-cli/Cargo.toml` | y | 0 |
| `crates/mztab-m-cli/LICENSE-APACHE` | N/A (standard Apache-2.0 license text) | 0 |
| `crates/mztab-m-cli/LICENSE-MIT` | N/A (standard MIT license text) | 0 |
| `crates/mztab-m-cli/NOTICE` | y | 0 |
| `crates/mztab-m-cli/README.md` | y | 1 |
| `crates/mztab-m-cli/src/args.rs` | y | 2 |
| `crates/mztab-m-cli/src/exec.rs` | y | 1 |
| `crates/mztab-m-cli/src/lib.rs` | y | 2 |
| `crates/mztab-m-cli/src/main.rs` | y | 1 |
| `crates/mztab-m-cli/tests/binary.rs` | y | 1 |
| `crates/mztab-m-cli/tests/run.rs` | y | 0 |
| `crates/mztab-m-obo/Cargo.toml` | y | 0 |
| `crates/mztab-m-obo/LICENSE-APACHE` | N/A (standard Apache-2.0 license text) | 0 |
| `crates/mztab-m-obo/LICENSE-MIT` | N/A (standard MIT license text) | 0 |
| `crates/mztab-m-obo/NOTICE` | y | 0 |
| `crates/mztab-m-obo/README.md` | y | 1 |
| `crates/mztab-m-obo/src/lib.rs` | y | 1 |
| `crates/mztab-m-obo/src/parse.rs` | y | 3 |
| `crates/mztab-m-obo/tests/lookup.rs` | y | 3 |
| `crates/mztab-m-wasm/Cargo.toml` | y | 0 |
| `crates/mztab-m-wasm/src/lib.rs` | y | 1 |
| `crates/mztab-m-wasm/tests/demo.rs` | y | 1 |
| `crates/mztab-m-wasm/www/index.html` | y | 3 |
| `crates/mztab-m/Cargo.toml` | y | 0 |
| `crates/mztab-m/LICENSE-APACHE` | N/A (standard Apache-2.0 license text) | 0 |
| `crates/mztab-m/LICENSE-MIT` | N/A (standard MIT license text) | 0 |
| `crates/mztab-m/NOTICE` | y | 0 |
| `crates/mztab-m/README.md` | y | 2 |
| `crates/mztab-m/src/cv.rs` | y | 0 |
| `crates/mztab-m/src/cv/rule.rs` | y | 0 |
| `crates/mztab-m/src/cv/rules.rs` | y | 1 |
| `crates/mztab-m/src/diag.rs` | y | 0 |
| `crates/mztab-m/src/diag/code.rs` | y | 0 |
| `crates/mztab-m/src/diag/finding.rs` | y | 0 |
| `crates/mztab-m/src/diag/location.rs` | y | 0 |
| `crates/mztab-m/src/diag/severity.rs` | y | 0 |
| `crates/mztab-m/src/id.rs` | y | 0 |
| `crates/mztab-m/src/json.rs` | y | 0 |
| `crates/mztab-m/src/json/findings.rs` | y | 0 |
| `crates/mztab-m/src/json/lexer.rs` | y | 0 |
| `crates/mztab-m/src/json/read.rs` | y | 2 |
| `crates/mztab-m/src/json/read/emit.rs` | y | 6 |
| `crates/mztab-m/src/json/read/meta.rs` | y | 2 |
| `crates/mztab-m/src/json/read/table.rs` | y | 3 |
| `crates/mztab-m/src/json/schema.rs` | y | 2 |
| `crates/mztab-m/src/json/schema_tables.rs` | y | 0 |
| `crates/mztab-m/src/json/value.rs` | y | 1 |
| `crates/mztab-m/src/json/write.rs` | y | 2 |
| `crates/mztab-m/src/json/write/emit.rs` | y | 2 |
| `crates/mztab-m/src/json/write/meta.rs` | y | 0 |
| `crates/mztab-m/src/json/write/table.rs` | y | 0 |
| `crates/mztab-m/src/lib.rs` | y | 0 |
| `crates/mztab-m/src/meta.rs` | y | 0 |
| `crates/mztab-m/src/meta/key.rs` | y | 0 |
| `crates/mztab-m/src/meta/subkey.rs` | y | 0 |
| `crates/mztab-m/src/model.rs` | y | 0 |
| `crates/mztab-m/src/model/colunit.rs` | y | 2 |
| `crates/mztab-m/src/model/document.rs` | y | 0 |
| `crates/mztab-m/src/model/layout.rs` | y | 0 |
| `crates/mztab-m/src/model/meta_entries.rs` | y | 0 |
| `crates/mztab-m/src/model/meta_insert.rs` | y | 1 |
| `crates/mztab-m/src/model/meta_value.rs` | y | 0 |
| `crates/mztab-m/src/model/metadata.rs` | y | 1 |
| `crates/mztab-m/src/model/rows.rs` | y | 0 |
| `crates/mztab-m/src/model/table.rs` | y | 0 |
| `crates/mztab-m/src/owned.rs` | y | 1 |
| `crates/mztab-m/src/param.rs` | y | 0 |
| `crates/mztab-m/src/param/parse.rs` | y | 0 |
| `crates/mztab-m/src/param/text.rs` | y | 1 |
| `crates/mztab-m/src/read.rs` | y | 0 |
| `crates/mztab-m/src/read/builder.rs` | y | 1 |
| `crates/mztab-m/src/read/event.rs` | y | 1 |
| `crates/mztab-m/src/read/metadata.rs` | y | 1 |
| `crates/mztab-m/src/read/stream.rs` | y | 0 |
| `crates/mztab-m/src/read/typer.rs` | y | 3 |
| `crates/mztab-m/src/tsv.rs` | y | 0 |
| `crates/mztab-m/src/tsv/mtd.rs` | y | 1 |
| `crates/mztab-m/src/tsv/splitter.rs` | y | 0 |
| `crates/mztab-m/src/tsv/table.rs` | y | 3 |
| `crates/mztab-m/src/tsv/typing.rs` | y | 3 |
| `crates/mztab-m/src/validate.rs` | y | 0 |
| `crates/mztab-m/src/validate/metadata.rs` | y | 0 |
| `crates/mztab-m/src/validate/metadata/mandatory.rs` | y | 0 |
| `crates/mztab-m/src/validate/metadata/refs.rs` | y | 0 |
| `crates/mztab-m/src/validate/metadata/values.rs` | y | 0 |
| `crates/mztab-m/src/validate/patterns.rs` | y | 7 |
| `crates/mztab-m/src/validate/report.rs` | y | 0 |
| `crates/mztab-m/src/validate/semantic.rs` | y | 0 |
| `crates/mztab-m/src/validate/semantic/rules.rs` | y | 0 |
| `crates/mztab-m/src/validate/semantic/terms.rs` | y | 0 |
| `crates/mztab-m/src/validate/semantic/uses.rs` | y | 1 |
| `crates/mztab-m/src/validate/tables.rs` | y | 0 |
| `crates/mztab-m/src/validate/tables/colunit.rs` | y | 0 |
| `crates/mztab-m/src/validate/tables/row.rs` | y | 0 |
| `crates/mztab-m/src/validate/tables/shape.rs` | y | 0 |
| `crates/mztab-m/src/validate/tables/sme.rs` | y | 0 |
| `crates/mztab-m/src/validate/tables/smf.rs` | y | 1 |
| `crates/mztab-m/src/validate/tables/sml.rs` | y | 2 |
| `crates/mztab-m/src/value.rs` | y | 0 |
| `crates/mztab-m/src/value/cell.rs` | y | 0 |
| `crates/mztab-m/src/value/decimal.rs` | y | 0 |
| `crates/mztab-m/src/value/text.rs` | y | 0 |
| `crates/mztab-m/src/vocab.rs` | y | 0 |
| `crates/mztab-m/src/vocab/field.rs` | y | 0 |
| `crates/mztab-m/src/vocab/profile.rs` | y | 0 |
| `crates/mztab-m/src/vocab/section.rs` | y | 0 |
| `crates/mztab-m/src/vocab/version.rs` | y | 0 |
| `crates/mztab-m/src/write.rs` | y | 0 |
| `crates/mztab-m/src/write/error.rs` | y | 1 |
| `crates/mztab-m/src/write/layout.rs` | y | 3 |
| `crates/mztab-m/src/write/line.rs` | y | 0 |
| `crates/mztab-m/src/write/pending.rs` | y | 2 |
| `crates/mztab-m/src/write/render.rs` | y | 2 |
| `crates/mztab-m/src/write/sink.rs` | y | 1 |
| `crates/mztab-m/tests/cv_rules.rs` | y | 2 |
| `crates/mztab-m/tests/findings_doc.rs` | y | 2 |
| `crates/mztab-m/tests/fixtures/broken.mztab` | y | 0 |
| `crates/mztab-m/tests/fixtures/full.mztab` | y | 4 |
| `crates/mztab-m/tests/golden/README.md` | y | 1 |
| `crates/mztab-m/tests/golden/broken.txt` | y | 0 |
| `crates/mztab-m/tests/golden/corpus.txt` | y | 1 |
| `crates/mztab-m/tests/golden/full.canonical.mztab` | y | 4 |
| `crates/mztab-m/tests/golden/full.json` | y | 4 |
| `crates/mztab-m/tests/golden/json-corpus.txt` | y | 1 |
| `crates/mztab-m/tests/json.rs` | y | 4 |
| `crates/mztab-m/tests/oracle.rs` | y | 2 |
| `crates/mztab-m/tests/oracle/README.md` | y | 0 |
| `crates/mztab-m/tests/oracle/expected-disagreements.tsv` | y | 1 |
| `crates/mztab-m/tests/properties.proptest-regressions` | y | 0 |
| `crates/mztab-m/tests/properties.rs` | y | 4 |
| `crates/mztab-m/tests/reader.rs` | y | 3 |
| `crates/mztab-m/tests/rules.rs` | y | 3 |
| `crates/mztab-m/tests/semantic.rs` | y | 2 |
| `crates/mztab-m/tests/spec_anchors.rs` | y | 1 |
| `crates/mztab-m/tests/support/corpus.rs` | y | 0 |
| `crates/mztab-m/tests/support/terms.rs` | y | 2 |
| `crates/mztab-m/tests/validate.rs` | y | 1 |
| `crates/mztab-m/tests/writer.rs` | y | 1 |
| `deny.toml` | y | 0 |
| `docs/BENCHMARKS.md` | y | 1 |
| `docs/DEBT.md` | y | 1 |
| `docs/DESIGN.md` | y | 8 |
| `docs/agents/PLAN.md` | y | 1 |
| `docs/agents/jmztab-m-rules.md` | y | 1 |
| `docs/agents/milestones.md` | y | 1 |
| `docs/agents/recon-phase0.md` | y | 1 |
| `docs/agents/sandbox.md` | y | 1 |
| `docs/findings.md` | y | 0 |
| `docs/proposed-issues.md` | y | 1 |
| `docs/site/book.toml` | y | 1 |
| `docs/site/src/SUMMARY.md` | y | 0 |
| `docs/site/src/benchmarks.md` | y | 0 |
| `docs/site/src/bindings.md` | y | 0 |
| `docs/site/src/changelog.md` | y | 0 |
| `docs/site/src/contributing.md` | y | 0 |
| `docs/site/src/decisions.md` | y | 0 |
| `docs/site/src/design.md` | y | 0 |
| `docs/site/src/findings.md` | y | 0 |
| `docs/site/src/install.md` | y | 1 |
| `docs/site/src/introduction.md` | y | 2 |
| `docs/site/src/library-api.md` | y | 0 |
| `docs/site/src/library-obo.md` | y | 0 |
| `docs/site/src/library.md` | y | 0 |
| `docs/site/src/quickstart.md` | y | 1 |
| `docs/site/src/security.md` | y | 0 |
| `docs/site/src/status.md` | y | 0 |
| `fuzz/Cargo.lock` | N/A (generated Cargo lockfile) | 0 |
| `fuzz/Cargo.toml` | y | 1 |
| `fuzz/fuzz_targets/json.rs` | y | 1 |
| `fuzz/fuzz_targets/obo.rs` | y | 0 |
| `fuzz/fuzz_targets/parse_bytes.rs` | y | 1 |
| `fuzz/fuzz_targets/roundtrip.rs` | y | 1 |
| `fuzz/fuzz_targets/stream_vs_whole.rs` | y | 1 |
| `fuzz/fuzz_targets/validate.rs` | y | 0 |
| `mk` | y | 1 |
| `mk.local` | y | 2 |
| `mk.versions` | y | 0 |
| `prek.toml` | y | 0 |
| `pyproject.toml` | y | 1 |
| `renovate.json` | y | 1 |
| `rust-toolchain.toml` | y | 0 |
| `rustfmt.toml` | y | 0 |
| `scripts/generate-findings.sh` | y | 0 |
| `scripts/mdbook-links.py` | y | 0 |
| `third_party/jmzTab-m` | N/A (pinned external reference submodule) | 0 |
| `third_party/mzTab-M` | N/A (pinned external HUPO-PSI specification submodule) | 0 |
| `uv.lock` | N/A (generated Python lockfile) | 0 |

---

## 2. Findings Table

| ID | Severity | Category | Location | Description | Evidence | Fix | Commit |
|---|---|---|---|---|---|---|---|
| F-001 | CRITICAL | impl | `bindings/python/src/lib.rs:57`, `_native.pyi:31` | Passing single `str` to `vocabularies` iterates chars, giving silent false-clean validation | `validate(doc, vocabularies=obo_text)` parses char-by-char | Reject `PyString`/`PyBytes`/`PyByteArray` for `vocabularies` with `PyTypeError` | `95d05ea` |
| F-002 | CRITICAL | impl | `crates/mztab-m/src/json/read/table.rs:55,70,86` | Infers table columns from `rows[0]` only, dropping keys omitted or `null` on Row 0 | Row 1 `"charge":2` dropped with 0 findings | Union columns across all rows; allow `null` arrays | `5648acb` |
| F-003 | CRITICAL | impl | `crates/mztab-m/src/json/write.rs:84-88` | `split_key` accepts `0`, `01`, `+1` via `u32::from_str`, rewriting invalid TSV refs | `instrument[01]` becomes `"instrument_ref":1` | Use `crate::id::parse_index` | `5648acb` |
| F-004 | CRITICAL | impl | `crates/mztab-m/src/param/text.rs:112-114`, `write/layout.rs:53` | Valid `ParamList` with `[`, `]`, or `\|` fails `to_canonical_string` and `write` | `[,,"]\|",]` causes `WriteError::Unwritable` | Quote `[`, `]`, `\|` in `ParamText::needs_quotes`; align verbatim error handling | `1298978` |
| F-005 | CRITICAL | impl | `crates/mztab-m/tests/cv_rules.rs:196,228` | `AND` assertion runs before `EXTRA_TERMS`, emitting 2-term `Logic::And` for `msrun_id_format_may` | `cv::rules()` exposes `Logic::And` with 2 alternative terms | Promote `msrun_id_format_may` to `Logic::Or` and assert after `EXTRA_TERMS` | `4b88e0c` |
| F-006 | CRITICAL | math | `benches/src/lib.rs:187-191`, `benches/golden.mztab:71-78` | `(seed ^ row ^ table) \| 1` collides SMF (`2`) and SME (`3`) PRNG streams | SMF `rt` fractional digits match SME `id_confidence_measure[1]` | Replace with `SplitMix64` mixer and independent stream constants | `e9217d3` |
| F-007 | CRITICAL | math | `benches/src/lib.rs:260-291`, `benches/golden.mztab:66-78` | Synthetic documents have mismatched formula/mass/mz, unrelated SV means, and decoupled SML/SMF abundances | `C6H12O6` paired with random masses; `abundance_study_variable` < all assays | Use metabolite templates, CODATA 2022 proton mass, Welford mean & sample SE (`n-1`) | `e9217d3`, `9ddad30` |
| F-008 | CRITICAL | math | `crates/mztab-m/tests/fixtures/full.mztab:47,50,127` | Non-zero standard error (`0.1`) reported for `n = 1` assay | `study_variable[1]` has 1 assay (`assay[1]`) | Add `ms_run[2]` + `assay[2]` (`1234.4`, `1234.6`) yielding exact mean `1234.5` & SE `0.1` | `e9217d3` |
| F-009 | CRITICAL | semantic | `crates/mztab-m/tests/fixtures/full.mztab:34,131-136` | Polarity/charge/adduct/evidence contradictions (`203.0526` `[M+Na]+` marked `charge=-1`) | `ms_run[1]` is positive-only; `SMF 1` links to `SME 2` at different m/z | Align positive polarity, `[M+Na]+` charge `+1`, and SME 1–2 evidence m/z (`181.0709`) | `e9217d3` |
| F-010 | CRITICAL | impl | `benches/benches/heap.rs:83-97` | Streaming heap benchmark measures generator `Vec<String>` allocations under 64 MiB bound | `spec.metadata()` allocates ~6k strings inside `measure` | Stream metadata lazily (`436,354` B measured peak) and tighten bound to `< 2 MiB` | `e9217d3` |
| F-011 | CRITICAL | semantic | `docs/DESIGN.md:128-129` | Glossary defines `Profile` as mzTab 1.0 `Summary`/`Complete` | `vocab/profile.rs` defines `M`..`MSFE` | Rewrite glossary entry to match mzTab-M 2.1 profiles | `1ba5f02` |
| F-012 | MAJOR | semantic | `crates/mztab-m/src/validate/patterns.rs:58-120,159-226` | Calendar/clock bounds, `Datatype::Integer`/`Decimal`/`AnyUri`, `xsd_matches`, and Table 1 `PositiveInt` lax/inconsistent | `2025-02-29`, `25:00:00`, `NaN` as `xsd:decimal`, `scan=0` (Thermo) pass; `+12` fails `Datatype::Integer` | Enforce calendar/clock bounds, XSD types, and `PositiveInt` | `4b88e0c`, `9ddad30` |
| F-013 | MAJOR | semantic | `crates/mztab-m/src/validate/semantic/uses.rs:43-74` | `study_variable[n]` values in `Datatype::Parameter` groups are skipped by semantic CV checks | `X001`/`X004`/`X006` miss `study_variable[n]` CV params | Parse and collect `Parameter`-typed `study_variable[n]` in `uses.rs` | `4b88e0c` |
| F-014 | MAJOR | test | `crates/mztab-m-obo/src/lib.rs:100`, `parse.rs`, `lookup.rs` | `Ontology::is_descendant` returns `true` on cycle self-reachability; OBO BOM/inline comments/escaped quotes unhandled; false `part_of` in test | `is_descendant("MS:9000001", "MS:9000001") == true` breaks strict-descendant contract | Return `false` for `accession == ancestor`; strip BOM/comments; fix test | `4b88e0c` |
| F-015 | MAJOR | impl | `crates/mztab-m/src/model/meta_insert.rs:134-320` | `Metadata::insert` inserts default element on `Err(InsertError::WrongKind)` | Phantom empty element added before `expect_kind!` check | Validate `value.kind() == key.kind()` before mutating `IndexMap` | `1298978` |
| F-016 | MAJOR | impl | `crates/mztab-m/src/write/sink.rs:132-143`, `render.rs:92,108`, `layout.rs:152`, `pending.rs:107,181` | Double-counted error line after `LineEnding::None`; `Invalid(" null ")` and `[Invalid("")]` round-trip bugs; spurious blank/row order | `Out::ending` increments on `LineEnding::None`; `render.rs` only checks exact `"null"` | Only increment on non-`None` ending; trim spaces in check; track `wrote_any` | `1298978` |
| F-017 | MAJOR | impl | `crates/mztab-m/src/tsv/typing.rs:92-117`, `mtd.rs:54,81`, `read/metadata.rs:69`, `model/metadata.rs:84` | Error offsets hardcoded to `0` or off-by-1; `Metadata::unknown` docstring inaccurate; `Pre::of` interaction with spaces | `doi:abc\rdef` reports col 1; `TrailingEmptyCells` points at `\t` | Fix offsets and docstrings; test D-056 behavior | `1298978` |
| F-018 | MAJOR | impl | `crates/mztab-m/src/json/read/{table,emit,meta,read}.rs`, `schema.rs`, `write/emit.rs` | JSON row array/opt validation, `colunit` missing `param`, `0e10000`/`1e1`/`1.0`, empty `[]` metadata, `StudyVariable.name` `Parameter`, comments/ids ordering | Unchecked row/opt/colunit shapes and lossy scientific-notation/int coercion | Fix table/emit/meta/schema/comments/write-int coercion & kill boundary mutants | `5648acb`, `9ddad30` |
| F-019 | MAJOR | impl | `crates/mztab-m-api/src/{format,ops,settings}.rs`, `mztab-m-cli/src/*`, `mztab-m-wasm/*` | BOM JSON sniff fails; JSON invalid UTF-8 silent; CLI rejects `--`/subcommand `--help`/non-UTF-8 args and drops stdout errors; WASM init race & ignored filename | Bombed JSON sniffed as TSV; CLI fails without `--` handling or propagates lost stdout error | Fix BOM sniff, UTF-8 check, `set_code`, CLI args/IO, WASM init/name/lossy | `94d9306` |
| F-020 | MAJOR | impl | `bindings/python/*`, `bindings/r/*` | Python accepts `list[int]`/narrow dict, misreports lone surrogates; R `raw` uses `rawToChar`, `NA` sentinel bugs, `bootstrap.R` crash, self-contained test fail, missing `man/*.Rd` & parity exports | Type confusion in Python/R FFI and broken standalone R install/tests | Fix Python/R type checks, raw bytes, `NA` guards, `bootstrap.R`, `Makevars`, `roxygenise` | `95d05ea` |
| F-021 | MAJOR | semantic | `crates/mztab-m/tests/fixtures/full.mztab`, `support/terms.rs`, `tests/*`, `benches/*`, `fuzz/*` | Truncated InChI, mass-error mismatch, disallowed CV roots, double-counted `MZT-S004`/`T028` in `corpus.txt`, weak test/fuzz invariants | Inconsistent fixture values and double-counted `corpus.txt` | Fix fixture chemistry/CVs, test assertions, `corpus.txt`, benches, fuzz targets | `e9217d3`, `9ddad30` |
| F-022 | MAJOR | semantic | `docs/*`, `DECISIONS.md`, `STATUS.md`, `.github/workflows/{oracle,bindings}.yml`, `renovate.json`, `dependabot.yml` | Doc factual errors (`mztab-m-api`, R exports, `json::parse`, corpus counts, `OPEN`/`PROPOSED` counts), `oracle.yml` untracked-file bug, Renovate/Dependabot lockstep conflict, orphaned `decisions-review.md` | Verified against source/configs | Update docs/configs accurately; delete `decisions-review.md` | `1ba5f02` |
| F-023 | MINOR | slop | `README.md`, `docs/site/*`, `crates/*/README.md`, `Cargo.toml`, `crates/mztab-m/src/*` | Marketing/second-person prose, flat `introduction.md` heading hierarchy, minor dead fields/bounds, `indexing_slicing = "warn"` | Audited across workspace docs and sources | Rewrite prose tersely, split `status.md`, set `indexing_slicing = "deny"`, trim dead code | `4b88e0c`–`1ba5f02` |

---

## 3. Behavior Changes

| Component | Before | After | Justification |
|---|---|---|---|
| Calendar & clock validation (`patterns.rs`) | Accepted `2025-02-29`, `25:00:00`, `24:00:00.1`, `+14:01`; rejected `+12` for `xsd:integer`; accepted `NaN` for `xsd:decimal` | Leap-year-aware calendar (`is_date`), strict ISO 8601 time/tz bounds (`is_time`), strict `xsd_matches` (`integer`, `nonNegativeInteger`, `positiveInteger`, `decimal`) | Spec XSD datatype conformance (`F-012`) |
| NativeID positive integers (`patterns.rs`) | `controllerNumber=0`, `scan=0` (Thermo/ABI/Bruker/Waters) accepted as non-negative | Enforces `PositiveInt` (`>= 1`) per mzTab-M Table 1 | Spec Table 1 nativeID definitions (`F-012`) |
| `study_variable[n]` `Parameter` CV check (`uses.rs`) | Skipped semantic CV checks when `study_variable_group` had `datatype = Parameter` | Parses and validates `Parameter`-typed `study_variable[n]` values (`MZT-X001`, `X004`, `X006`) | Spec §6.2.15 / `study_variable_group-datatype` (`F-013`) |
| `Ontology::is_descendant` & OBO parser (`mztab-m-obo`) | Self-reachability in cycle returned `true`; UTF-8 BOM, inline `!` comments on header/values, escaped quotes broke parsing | Strict descendant (`accession != ancestor`), BOM/comment/escape handling | `TermLookup::is_descendant` strict contract & OBO 1.4 (`F-014`) |
| `Metadata::insert` & `ParamList` round-trip | Failed `WrongKind` insert left default entry in `IndexMap`; `ParamText` containing `[`, `]`, `\|` failed to write canonically | Validates kind before mutating `IndexMap`; quotes `[`, `]`, `\|` inside `ParamText::needs_quotes` | Lossless round-trip and atomic API invariant (`F-004`, `F-015`) |
| `broken.txt` golden lines `9, 15, 47` | Sub-cell `doi:abc\rdef` pointed at byte 0 (`col 1`), trailing `\t` off by 1, line ending count off after `LineEnding::None` | Accurate 1-based line/column offsets (`broken.txt` blessed in `1298978`) | Accurate finding coordinates (`F-016`, `F-017`) |
| JSON codec (`json/`) | Dropped columns absent from row 0; accepted `[01]`/`[0]` refs; skipped `colunit.param` & `comment` prefix/`line_number` checks; incorrectly coerced `0e10000`/`1.0`/`1e1`; omitted `StudyVariable.name` `Parameter` | Unions columns across all rows, uses `parse_index`, validates `colunit`/`comment`/`opt`/bounds strictly, expands finite scientific lexemes faithfully | Schema & round-trip correctness (`F-002`, `F-003`, `F-018`) |
| API / CLI / WASM | JSON BOM sniffed as TSV; invalid UTF-8 JSON ignored; CLI rejected `--` and subcommand `--help` and masked stdout write errors; WASM init racy & ignored `.json` filename | Strips BOM before sniffing, emits `MZT-S014` on invalid UTF-8 JSON, supports `--` / `--help` / `--format` checking & propagates stdout errors; idempotent WASM init & filename sniffing | Boundary error handling parity (`F-019`) |
| Python & R bindings | Python iterated single `str` chars for `vocabularies` and accepted `list[int]`; R `raw` tripped on NUL bytes and used `NA` sentinel bugs | Python rejects non-sequence `vocabularies` and `list[int]` with `TypeError`; R preserves raw bytes & rejects `NA`, adds `spec_version()` and `vocabulary_size()`, adds `man/*.Rd` | Safe boundary validation and API parity (`F-001`, `F-020`) |
| `corpus.txt` golden (`tests/reader.rs`) | Chained raw `parse_bytes` + `validate::structure`, double-counting `MZT-S004` syntax errors as `MZT-T028` | Deduplicates `InvalidValue` when reader already flagged syntax error, matching `validate::all`; blessed in `e9217d3` | Consistent diagnostics (`F-021`) |
| `benches/golden.mztab` & `benches/src/lib.rs` | `Rng \| 1` collided SMF & SME streams; random m/z & SV stats divorced from `C6H12O6` & assays; `rt_start == rt == rt_end`; zero assays panicked | `SplitMix64` independent streams, 6 real metabolites + CODATA 2022 `PROTON_MASS_DA`, `±5 ppm` error, nonzero RT window, exact Welford mean & sample SE (`n-1`); blessed in `e9217d3` | Scientific & statistical consistency (`F-006`, `F-007`) |
| `full.mztab` / `full.canonical.mztab` / `full.json` | Positive scan with `203.0526` `charge=-1`, `n=1` SE `0.1`, truncated InChI, mismatched `1.2` ppm & `0.95`/`0.9` score, root/disallowed CV terms | Coherent positive-mode `ms_run[1..=2]` + `assay[1..=2]` (`1234.4`, `1234.6` -> mean `1234.5`, SE `0.1`), full Glucose InChI, consistent `1.1` ppm error & `0.9` confidence, leaf CV terms (`Q Exactive`, `inductive detector`, `liver`, `hepatocyte`, `Disease free`, `CHEBI:85064`/`51088`); blessed in `e9217d3` | Chemically & ontologically valid reference fixture (`F-008`, `F-009`, `F-021`) |

## 4. Renames

- `bindings/python/src/lib.rs`: renamed parameter/local to `array` to satisfy `typos` spellcheck without adding a false-positive suppression.
- `docs/site/`: split `# Status` out of `docs/site/src/introduction.md` into its own chapter `docs/site/src/status.md` (`scripts/mdbook-links.py` `STATUS.md` target updated accordingly) to fix the flattened H2 hierarchy.
- Unpushed local commit subjects normalized to match `./mk commits` Conventional Commits regex (`fix(core/obo):`, `fix(api/cli/wasm):`, `fix(tests/benches/fuzz):`, `docs:`).

## 5. Open Questions

1. **SMF/SME negative `charge` values:** Spec prose (`mzTab_m_2_1_schema.adoc:1725,2028`) requires positive integers for both positive and negative polarity modes, yet 5 real-world corpus files (`LDA`, `MTBLS263`, `msdial_*`) emit `-1` and `-2`. Kept `i32` without rejecting negative charge pending maintainer/spec decision.
2. **SML parallel `|`-list lengths (jmzTab-m 2046):** Spec requires equal `|`-separated list lengths across SML identification columns (`database_identifier`, `chemical_formula`, `smiles`, `inchi`, `chemical_name`, `theoretical_neutral_mass`). Not currently enforced by `sml.rs`.
3. **Bare `ms_run[n]` in `SpectraRef`:** Spec allows `ms_run[n]` without `:nativeID` when no fragmentation spectrum was used, but `SpectraRef::reference` is `pub Text<'a>` (breaking API change to make `Option<Text<'a>>`).
4. **`opt_` column charset & `opt_*_null`:** Spec restricts `opt_` names to `[A-Za-z0-9_\-\[\]:]`, and `tsv/table.rs` uses `Text::new` which rejects `opt_*_null`.
5. **`ExtendedParameter` nested `Parameter` values:** JSCH allows nested `Parameter` inside `ExtendedParameter.value`; TSV `Param` grammar does not support nested brackets.
6. **Strict JSCH `Parameter.value` (`""` vs `null`) and SML null list items (`"null"` vs `null`):** Kept jmzTab-m compatibility (`null`) pending maintainer choice.
7. **ORCID Mod 11-2 check digit & `colunit-*` without table:** Currently pattern-only (`patterns.rs`) and `colunit-*` is ignored if its table is absent (`colunit.rs:44`).
8. **Oracle outputs (`tests/oracle/`):** Needs Java 25 / Maven workflow run to populate committed jmzTab-m outputs and un-ignore `tests/oracle.rs`.
9. **Archiving `docs/proposed-issues.md` and frozen `docs/agents/{PLAN,recon-phase0,milestones}.md`:** Kept pending maintainer filing/archival.

## 6. Residual Risk

- **Oracle comparison (`tests/oracle.rs`):** Remains `#[ignore]`d until `.github/workflows/oracle.yml` (JDK 25 + Maven) is run to populate committed `crates/mztab-m/tests/oracle/*.txt` outputs.
- **Mutation & coverage scope:** `mutants-diff` passed across all 539 branch-diff Rust mutants (`0` missed) across `crates/*` and `benches/`; `./mk coverage` holds `100%` line coverage (`COVERAGE_FAIL_UNDER_LINES="100"`) across `crates/*`. Non-workspace crates (`bindings/python`, `bindings/r`, `fuzz/`) are verified by unit/integration tests, clippy `-D warnings`, and `cargo check --manifest-path fuzz/Cargo.toml --locked`.
- **Fuzz execution:** All 6 `cargo-fuzz` targets (`parse_bytes`, `stream_vs_whole`, `roundtrip`, `json`, `validate`, `obo`) compile (`./mk local:fuzz`), but continuous libFuzzer runs require nightly Rust.
- **Draft specification & Windows bindings:** mzTab-M `2.1.0-M` remains an unreleased HUPO-PSI draft; Python/R bindings remain untested on Windows.

## 7. Slop Removal

- **Deleted files:** Removed orphaned `docs/agents/decisions-review.md` (`1ba5f02`; 0 incoming links, stale claims on repo status and counts).
- **Removed dead code, duplicates & redundant checks:** Removed dead `Pre::Unknown` / redundant fields and tightened workspace `[workspace.lints.clippy]` `indexing_slicing = "deny"`; simplified redundant `sv_count > 1 &&` branch in `benches/src/lib.rs` (`9ddad30`); extracted `text_after_colon` in `crates/mztab-m/src/tsv/typing.rs` to deduplicate colon-offset mapping in `mtd.rs` and `typing.rs` and removed stale backticked J-row code-span references in `STATUS.md` (`31ad9d9`).
- **Rewritten documentation:** Stripped marketing and second-person phrasing across `README.md`, `STATUS.md`, `docs/site/src/{introduction,install}.md`, `crates/mztab-m/README.md`, and `crates/mztab-m-obo/README.md`; fixed factual counts and export lists in `docs/DESIGN.md`, `STATUS.md`, `DECISIONS.md`, `docs/DEBT.md`, `docs/BENCHMARKS.md`, and `docs/agents/sandbox.md`.
- **Final banned-vocabulary sweep:** Case-insensitive regex sweep over documentation, code, scripts, and configs (`README.md`, `STATUS.md`, `DECISIONS.md`, `docs/`, `crates/`, `bindings/`, `benches/`, `fuzz/`, `mk.local`, `pyproject.toml`, `renovate.json`, `.github/`) returned `0` hits.
