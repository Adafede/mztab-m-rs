# DECISIONS

A running log of spec ambiguities, conflicts between the schema and the prose,
deviations in jmzTab-m, and the choices this crate makes about them.

Status values:

- `OPEN`: waiting for a decision from the maintainer.
- `PROPOSED`: a default is recorded, but nothing is binding until it is approved.
- `DECIDED`: approved.

Each entry follows the same format: ID, status, the sources involved, a
description of the issue, the options, and the proposal.

Sources were pinned on 2026-11-17:

| Source | Commit |
|---|---|
| HUPO-PSI/mzTab-M | `e66c19e1d5c2e61cd721cb3bd6134389184cf778` (2026-10-15) |
| lifs-tools/jmzTab-m | `182e909a496f60d3d7866ecc2e1fad0e68d36e35` (2026-11-17) |
| Adafede/rusti-graben | `dae14b32c7057d0e79b42fd9afb95d1fc2187c39` (2026-11-16) |

Abbreviations used for the sources:

| Short name | File |
|---|---|
| SPEC | `specification_documents/mzTab_format_specification_2_1-M.adoc` |
| FREF | `specification_documents/mzTab_m_2_1_schema.adoc` (generated from JSON) |
| JSCH | `schema/mzTab_2_1-M.json` (JSON Schema 2020-12) |
| OAPI | `schema/mzTab_2_1-M_openapi.json` |
| CHG | `specification_documents/mzTab_m_2_0_to_2_1_changes.adoc` |

---

## Version and normativity

### D-001: Target version is a draft — DECIDED (2026-11-17)

> **Decision (maintainer):** target the pinned 2.1 draft (`e66c19e`) and
> also read and validate `2.0.0-M` files.

- **Sources:** SPEC `:document-version: 2.1.0-InDevelopment July 2025 (DEVELOPMENT)`;
  the README says "Current Version 2.1.0 (in development, 2026)"; the validation
  profiles page says "under active development … may change before the 2.1 release".
- **Issue:** you asked for v2.1. The repository covers 2.1, but only as an
  unreleased draft. The only released version is 2.0.0-M.
- **Proposal:** target `2.1.0-M` at the pinned commit. Also read `2.0.0-M` files
  and validate them under a 2.0 rule set, since 2.1 claims to be backwards
  compatible. Pin the spec commit in code (`SPEC_COMMIT`), and re-run the
  conformance tests whenever the pin is bumped.

### D-002: Which artefact is normative for structure — OPEN

- **Issue:** FREF is generated from JSCH by `generate_schema_adoc.py`, yet the two
  disagree (see D-010 to D-016). The SPEC prose also contradicts both in places.
  OAPI has only 3 component schemas: `MzTab` references JSCH by `$ref`, while
  `Error` and `ValidationMessage` are API types.
- **Proposal:** use this precedence: SPEC prose (RFC 2119 keywords), then FREF
  "Mandatory/Nullable" tables, then JSCH. Every override is logged here.

### D-003: Corpus files with non-2.x-conformant versions — PROPOSED

- `examples/2.0/masster_0.5.25_null.mztab` declares `mzTab-version 2.2.0-M` and
  uses CRLF line endings.
- `examples/2.0/progenesis_…json` declares `2.2.0-M`.
- Four files under `examples/2.1/` (`xcms+MsIO_*_onlySMF.mztab` and others)
  declare `2.0.0-M`.
- **Proposal:** dispatch the rule set on the declared version. An unknown minor
  version such as `2.2.0-M` gets a warning and falls back to the newest known
  rule set. Accept CRLF and record it so the writer can round-trip it.

### D-004: JSON representation has two incompatible dialects in the corpus — OPEN

- `progenesis_…json` follows JSCH keys (`mzTab-version`, `smallMoleculeSummary`,
  `small_molecule-quantification_unit`).
- `MetaboScape-…json` uses snake_case variants (`mz_tab_version`,
  `small_molecule_summary`, `colunit_small_molecule`) that are not in JSCH.
- JSCH table rows use `[n]`-index integer references, while OAPI uses embedded
  objects (§2.1 of the comparison report).
- **Proposal:** JSON support follows JSCH only. MetaboScape's dialect gets a
  structured "unknown key" error and no silent aliasing. JSON goes in an optional
  `json` feature, because a hand-rolled JSON parser in core would cost size.
  Decide in Phase 1.

## Structure: schema vs. prose conflicts

### D-010: Table column mandatoriness and order — OPEN

- **2.1 FREF section text:** "All columns are MANDATORY except for opt_" and "The
  order of columns MUST follow the order specified below."
- **The same FREF:** every SML/SMF/SME column says `Mandatory: False`.
- **The FREF minimal example:** `SMH SML_ID database_identifier chemical_name
  reliability abundance_assay[1]` omits most columns and is out of order.
- **Corpus:** `lipidcompass…v2.1.mztab` puts `chemical_name` before
  `database_identifier`. `manual_null_null_lipidomics_v2.1.mztab` places
  `opt_global_mass_error` in the middle of SEH, although SPEC §5.11 says
  optional columns are added "to the end of rows".
- **Proposal:**
  - The reader addresses columns by header name and accepts any order, so
    reading never fails on order.
  - The validator emits: missing mandatory column = ERROR, wrong column order =
    WARN, `opt_` column not at the end = WARN.
  - The writer emits canonical order. For the round-trip guarantee, it preserves
    the original order when one is recorded.

### D-011: SMF `charge` nullability — OPEN

- **FREF:** `Is Nullable: FALSE`, but the field's own description says "Is
  nullable in the SMF table".
- **JSCH:** `integer|null`, `required:false` (comparison report item #23).
- **examples/README:** lists "nullable SMF charge" as a 2.1 feature.
- **CHG:** says Nullable changed from True to False.
- **Proposal:** nullable in 2.1.

### D-012: SML `best_id_confidence_value` nullability — OPEN

- **CHG:** Nullable changed from True (2.0) to False (2.1).
- **JSCH:** `number|null`, marked `recommended`.
- **Corpus:** many files contain `null` here.
- **Proposal:** nullable. Report `null` as INFO, consistent with `recommended`.

### D-013: `study_variable_group` mandatory? — OPEN

- **SPEC §5.3:** "At least one `study_variable_group` MUST be defined."
- **FREF:** Mandatory True.
- **JSCH:** optional, no `minimum`.
- **SPEC (same version):** "each study variable MUST be referenced by exactly one
  study variable group".
- **Problem:** 2.0 files and most 2.1 examples (e.g. `manual_null_MTD-only`) have
  none.
- **Proposal:** for 2.1 files, absence is a WARN; for 2.0 files it is skipped. If
  groups are present, the "exactly one group per SV" rule is an ERROR.

### D-014: MTD mandatory set differs between FREF, JSCH, and 2.0 — OPEN

| Field | 2.0 FREF | 2.1 FREF | 2.1 JSCH | SPEC prose |
|---|---|---|---|---|
| `software[1-n]` | M | opt | REQ min 1 | – |
| `publication[1-n]` | opt | **M** (likely generator artefact) | opt | – |
| `quantification_method` | M | opt | opt | M if SML or SMF present |
| `small_molecule-quantification_unit` | M | opt | opt | M if SML present |
| `small_molecule_feature-quantification_unit` | M if SMF | opt | opt | M if SMF present |
| `id_confidence_measure[1-n]` | M | opt | opt | M if SME present |
| `database[1-n]` (+ `-prefix`/`-version`/`-uri`) | M | M | opt (`prefix` and `uri` "recommended") | M if SME or SML has identifiers |
| `ms_run[1-n]-scan_polarity` | M | opt | opt | – |
| `study_variable[1-n]-assay_refs` / `-description` | M | opt | opt | – |
| `cv[1-n]` | M | M (sub-fields) | REQ min 1 | – |

- **Proposal:**
  - Profile-conditional fields follow SPEC §5.18.1 (profile table).
  - `software`: required, per JSCH.
  - `publication`: optional. The FREF value is a generator bug; report it
    upstream.

### D-015: `reliability` nullability vs. prose — OPEN

- **SPEC §5.6:** "The SML element reliability MUST be assigned a value."
- **FREF and JSCH:** nullable.
- **FREF minimal example:** uses `[,,high,]`, a Param, for a String/integer
  field.
- **Proposal:** `null` = WARN. A non-integer value = ERROR only when no
  `small_molecule-identification_reliability` is declared. Otherwise, check the
  value against the declared scheme's levels.

### D-016: Regex defects in JSCH — PROPOSED

- **SMF `adduct_ion`:** `\[\d*M([-][\w\d])\]\d*[+-]` is broken: it allows only
  `-` and exactly one char. SML and SME use `^\[\d*M([+-][\w\d]+)*\]\d*[+-]$`.
  The comparison report §5.5 claims JSCH has no SMF pattern, which is wrong.
- **`OptColumnMapping.identifier`:** `^global|ms_run\[\d+\]|…` lacks grouping, so
  the `^` binds only to `global`.
- **`mzTab-version`:** 2.1 uses `^\d{1}\.\d{1}\.\d{1}-[A-Z]{1}$`, which allows
  single digits only and any capital letter. The 2.0 regex
  `\d{2}\.\d{0}\.\d{0}-M` is nonsensical.
- **`StudyVariable.name`:** has no type.
- **Proposal:**
  - Use the SML/SME adduct pattern everywhere.
  - Anchor the identifier pattern with grouping.
  - Version: accept `^\d+\.\d+\.\d+-M$`. A non-`M` suffix is an ERROR.
  - SV name: a string. When the group has `datatype=Parameter`, parse it as a
    Param (D-017).
  - Report these upstream.

### D-017: `study_variable[n]` value is string or Param depending on group datatype — PROPOSED

- **SPEC:** values must be consistent with the group `datatype`. Values are
  `Male`, `[,,Male,]`, or `[NCIT, …]`.
- **Proposal:** the model stores the raw string plus a lazily typed view. The
  validator checks consistency within each group.

### D-018: Empty cells, trailing tabs, whitespace-only lines — PROPOSED

- **SPEC §5.7:** "MUST NOT be any empty cells".
- **SPEC §6:** "Empty lines … ignored".
- **Corpus:**
  - Lines consisting only of `\t\t\t` appear in `manual_null_*`.
  - MTD lines carry trailing empty cells (`…\t\t\t`).
- **Proposal:**
  - A whitespace-only line is treated as an empty line. The validator reports
    INFO for it, and the reader keeps it for round-trip.
  - Trailing empty cells beyond the header width = WARN.
  - An empty cell inside the table width = ERROR.

### D-019: Header-only (empty) tables — PROPOSED

- **SPEC §5.18:** "An empty table (header line only, no rows) is NOT a
  substitute for omitting the section."
- **Corpus:**
  - `2.0/manual_null_null_minimal_example.mztab` has an SMH with no rows. It is
    valid under 2.0.
  - xcms `*_onlySMF.mztab` (2.0) contains a synthetic single SML row.
- **Proposal:** apply the rule only to 2.1 files: ERROR when `mzTab-profile` is
  declared, WARN when the profile is inferred.

### D-020: MTD field order — OPEN

- **FREF:** "fields in the metadata section MUST be reported in the order listed
  below".
- **Proposal:** the validator reports out-of-order fields as WARN (not ERROR),
  pending the jmzTab-m behaviour. The writer emits FREF order, but preserves the
  source order for round-trip of non-canonical input.

### D-021: Number grammar — PROPOSED

- **SPEC §5.7:** `xs:decimal` plus the literals `NaN` and `null`. "Scientific
  notation and infinity is explicitly not supported."
- **Proposal:**
  - The reader is lenient: it accepts `1e-5`, `INF`, and `Infinity` and records
    them losslessly.
  - The validator reports them as ERROR (SPEC wins). Any jmzTab-m acceptance is
    logged as a deviation.

### D-022: Profile `M+S+F` and inference — PROPOSED

- **SPEC §5.18.2:** `M+S+F` is permitted but NOT RECOMMENDED, yet it is absent
  from the `mzTab-profile` enum.
- **SPEC:** `M+E` and `M+S+E` are prohibited.
- **Proposal:** the inferred profile can be `M+S+F` (WARN), and is reported as
  INFO, as SPEC requires. Declaring `M+S+F` = ERROR, since it is not in the
  enum.

## jmzTab-m deviations

The spec wins over jmzTab-m. Every deviation is listed here, and the
differential test harness expects each one. Item numbers point to
`docs/recon/jmztab-m-rules.md` §Quirks.

| ID | jmzTab-m behaviour | Spec | Proposal | Status |
|---|---|---|---|---|
| J-01 | Trims whitespace around cells and drops trailing empty cells (q1). | Tab is the only delimiter. | Values are kept verbatim. Leading/trailing whitespace → WARN. Trailing empty cells on a table row → ERROR (empty cell). Trailing tabs on MTD lines → INFO (corpus). | PROPOSED |
| J-02 | Accepts `1e5`, `Infinity`, `INF`, hex floats, and the `d`/`f` suffixes (q2). | xs:decimal + `NaN` + `null`. No scientific notation, no infinity. | ERROR. Each disagreement in the differential tests is tagged J-02. | PROPOSED |
| J-03 | `NULL` passes as null (q3). | `null` (case-sensitive by §5.18 "case sensitivity"). | Only the exact string `null` is null. | PROPOSED |
| J-04 | Section prefixes are case-insensitive, long names are accepted, and MTD keys are lowercased (q4). | "All column labels and field names are case-sensitive." | Case-sensitive. A case-insensitive match → ERROR with a fix-it hint. | PROPOSED |
| J-05 | Uses `find()` partial regex matching (q5). | Full match is implied. | Full match. | PROPOSED |
| J-06 | Lenient `parseParam` (q6). | `[label, accession, name, value]` | Strict grammar. Recovery hints are given in the error. | PROPOSED |
| J-07 | Crashes on `assay[n]-custom[m]` and similar inputs (q7). | `assay[n]-custom[m]` is a valid spec field. | Parse it. Never panic (fuzzed). | DECIDED (hard constraint) |
| J-08 | Skips model-level checks once any Error is raised. | — | Collect all findings. Layers run independently on a partial model. | DECIDED (requirement) |
| J-09 | Table presence comes from rows, so a header-only table counts as absent (q9). | SPEC §5.18: empty table ≠ omitted. | A present header means the section is present. Header-only in 2.1 → see D-019. | OPEN |
| J-10 | No SMF→SME reference check in M+F+E files (q10). | M+F+E: SME_ID_REFS MUST resolve. | Always check. | PROPOSED |
| J-11 | assay, cv, software, scan_polarity and SV are required under profile `M` (q11). | Some MTD fields are profile-conditional. | Follow the spec. `scan_polarity` is optional in 2.1 (D-014). | OPEN |
| J-12 | Profile rules apply to 2.0.0-M files (q12). | Profiles were introduced in 2.1. | 2.0 files: profile inferred, INFO only. No ERROR for rules that exist only in 2.1. | PROPOSED |
| J-13 | A UTF-8 BOM gives a fatal 1000 (q13). | "parsers should be able to recognize commonly used encodings" | Strip the BOM and emit INFO. Record it for round-trip. Only UTF-8 is supported (core is `&str`/`&[u8]`). | PROPOSED |
| J-14 | All stable columns are required; column order is free. | FREF: "All MANDATORY", "order MUST". | See D-010. | OPEN |
| J-15 | Does not check uniqueness of SML/SMF/SME_ID, the ambiguity-code range, or duplicate indexed MTD keys. | IDs are "within file unique"; JSCH min 1 / max 3. | ERROR for each. | PROPOSED |
| J-16 | Adduct regex is not applied to SME `adduct_ion`. | JSCH has the pattern on SME. | Apply it. | PROPOSED |
| J-17 | Uses its own error codes (1000–3023). | No normative code registry exists in the spec repo. | Our own stable codes. Each one maps to its jmzTab-m code(s) where one exists, so differential tests can compare. | PROPOSED (adopted in M1: `Code::jmztab`) |
| J-18 | A parameter with a label but no accession (or the reverse) is accepted as a CV parameter. | `[CV label, accession, name, value]`; the spec does not define half-CV parameters. | Rejected by the grammar (D-050); the cell is kept as `Invalid` and reported (`MZT-S002`). The corpus has no such parameter. | PROPOSED |

## Licensing

### T6: License — DECIDED (2026-11-17)

> **Decision (maintainer):** `MIT OR Apache-2.0` for all crates. This
> deviates from the template's AGPL-3.0-or-later. The vendored test corpora
> keep their own licenses (CC-BY-4.0 for the mzTab-M examples, Apache-2.0 for
> the jmzTab-m fixtures), and NOTICE files carry the attribution.

## Tooling

### D-030: No local JVM — OPEN

- **Issue:** differential testing against jmzTab-m needs Java. None is installed
  here. The options are a bioconda or Docker `jmztab-m` container, or a CI-only
  job.
- **Proposal:** use a CI job that downloads the pinned release JAR and validates
  with a recorded checksum. Its JSON/XML output is committed as golden files, so
  local `cargo test` needs no JVM.

### D-031: Template `uses: $/…` — RETRACTED (2026-11-17)

- **Original claim (Phase 0):** every rusti-graben workflow had a bug, because
  it used `uses: $/.github/actions/setup` instead of `./`.
- **This was wrong.** `$/` is GitHub's self-repository syntax. zizmor 1.32
  (the `self-repository` audit) recommends it and flags `./` as a finding.
- **Action:** the template is kept as-is. No upstream report is needed.

## Repository

### D-040: Test corpora as pinned git submodules, not vendored copies — PROPOSED (M0)

- **Change from the plan:** PLAN.md said the corpora would be "vendored".
  Instead, `third_party/mzTab-M` (pinned to `e66c19e`) and
  `third_party/jmzTab-m` (pinned to `182e909`) are git submodules.
- **Why:**
  - Provenance is exact.
  - The upstream licences (CC-BY-4.0, Apache-2.0) stay intact.
  - The repository does not grow by about 37 MB of example files.
  - Nothing from the corpora can leak into the published crate (crates.io
    limit: 10 MB).
- **Cost:**
  - Contributors must clone with `--recurse-submodules`.
  - CI checkouts set `submodules: true`.
  - wasm tests embed corpus files with `include_str!` from `third_party/`.
- **Attribution:** `NOTICE` carries it.

### D-041: Template files not adopted — PROPOSED (M0)

- **Agent files:** `AGENTS.md`, `CLAUDE.md`, `.agents/` and `skills/` are not
  adopted (T8).
- **`.vscode/settings.json`:** not adopted. It is editor-only, and the sandbox
  used during development cannot write it.
- **`CITATION.cff` and `codemeta.json`:** deferred to M9. They need author
  metadata, which only the maintainer can supply.

## Model and grammars (M1)

### D-050: Parameter grammar details — PROPOSED (M1)

- **SPEC §6 "Params"** fixes the four-field shape, says unavailable fields
  are empty, and says a name containing commas MUST be quoted. It says
  nothing about escaping, whitespace, or half-CV parameters. Its own example
  uses typographic quotes (`“…”`); only ASCII `"` is treated as a quote.
- **Grammar adopted:** see the `Param` rustdoc. In short: exact `[`…`]`;
  four comma-separated fields, commas inside `"…"` do not split; spaces
  around a field are insignificant; field text cannot contain `"`, tab or
  line break; `name` is required; label and accession are both present (CV)
  or both absent (user).
- **Corpus check (e66c19e):** about 35 000 bracketed cells. Every one with
  more than three commas uses quotes; none is half-CV; none has a quote inside a
  field.
- **Writer form:** `[label, accession, name, value]` with `", "` separators,
  absent fields empty, quotes only when the text has a comma or a
  leading/trailing space. jmzTab-m writes the same shape.
- **Open consequence (M3):** the corpus mixes spacing styles (`[,,x,]`,
  `[, , x, x]`, `[x,x,x,]`). The typed `Param` does not keep them, so a
  byte-exact round trip of non-canonical files needs the lexeme kept
  elsewhere. Decided in M3 together with `SourceLayout` (D-054).

### D-051: Row ids are non-negative 32-bit integers — PROPOSED (M1)

- **FREF:** `SML_ID`, `SMF_ID`, `SME_ID` are "Integer", unique within the
  file. No range is given. JSCH gives `minimum: 1` on several indices but not
  on these ids.
- **Choice:** `u32`. A negative or larger value is kept as `Invalid` and
  reported. `0` is accepted by the type; whether it is legal is left to the
  validator (pending, no corpus file uses it).

### D-052: `proptest` is a native-only dev-dependency — PROPOSED (M1)

- **What:** property tests (round trips, totality of the grammars) in
  `crates/mztab-m/tests/properties.rs`.
- **Why not hand-rolled:** shrinking and regex-driven string strategies
  would cost more code than the tests themselves.
- **Cost:** 12 crates, dev-only, never in the user's graph
  (`default-features = false, features = ["std"]`; no fork/timeout).
- **wasm:** on `wasm32-unknown-unknown`, `getrandom` needs a JS backend. The
  property tests are therefore `cfg(not(target_arch = "wasm32"))`. Unit and
  other integration tests run on wasm; doctests run natively only, as for
  any crate.

### D-053: The model follows the field reference types literally — PROPOSED (M1)

- Every metadata field FREF types as "Parameter List" or "… List" is a
  `|`-separated list in the model, even where the corpus only ever has one
  item (`sample[n]-species[m]`, `id_confidence_measure[n]`). This is a
  lossless superset; the validator can narrow it.
- List items are cells, so `null|HMDB:1` is kept item by item.
- The two fields removed in 2.1 (`ms_run[n]-usi_identifier`,
  `study_variable[n]-factors`) are kept so 2.0 files read losslessly.
- Unknown metadata keys are kept in source order. The corpus has several
  (`mzTab-id`, `study_variable[n]_refs`, `instrument[n]-analyzer` without an
  index), which the validator will report.

### D-054: Deferred from M1 — PROPOSED (M1)

These §3 items depend on the reader's design and land with it:

- comment anchors and `SourceLayout` (line endings, BOM, blank lines, MTD
  and column order, parameter spacing): M2 (record) and M3 (honour);
- `MzTab::into_owned`: M2, together with the borrowed reader;
- byte ranges in `Location::Text`: dropped. Line, column and cell number
  locate every finding. Adding a byte range later is a breaking change,
  acceptable before 1.0 if it is ever needed.

### D-055: `allow-expect-in-tests` in `clippy.toml` — PROPOSED (M1)

- **Template:** `expect_used` is warned workspace-wide and CI denies
  warnings, so tests must avoid `.expect()`.
- **Problem:** the alternative, `.unwrap_or_else(|_| unreachable!())`, leaves
  the closure as an uncovered line, which the 100% line-coverage gate
  rejects.
- **Change:** `clippy.toml` sets `allow-expect-in-tests = true`. Library code
  is unaffected: `expect_used` still fires outside `#[test]` / `cfg(test)`.

## Reader (M2)

### D-056: Spaces around cells and list items — PROPOSED (M2)

- **Implements J-01.** jmzTab-m trims silently. SPEC defines no padding.
- **Rule:** a cell or `|`-list item with leading or trailing spaces gets
  `MZT-S013` (warning). Strings keep their spaces. Every other type is read
  from the trimmed text, so `assay[1]| assay[2]` (xcms, the 2.1 field
  reference's own example) types correctly.
- **Round trip:** the typed value of a trimmed cell has no spaces, so such
  cells are not written back byte-exact. PLAN §6 already excludes them.

### D-057: `study_variable_group[n]-datatype` is a closed vocabulary — PROPOSED (M2)

- **FREF** types it as "Parameter". Its own prose lists `xsd:string`,
  `xsd:integer`, `xsd:decimal`, `xsd:boolean`, `xsd:date`, `xsd:time`,
  `xsd:dateTime`, `xsd:anyURI` and `Parameter`, and the 2.1 example
  (`example_study_variable_group.mztab`) writes `xsd:integer`.
- **Choice:** prose wins (D-002). The model has a `Datatype` enum; any other
  value is reported (`MZT-S007`). To report upstream as an FREF defect.

### D-058: Reader design, as built — PROPOSED (M2)

- **One typer, two drivers.** `parse`/`parse_bytes` and `StreamReader` feed
  the same line typer. `parse_bytes` collects the events with the same
  builder as `Parsed: FromIterator<Event>`, so "streamed and whole reads give
  the same document" is a test (corpus, fixtures, proptest, fuzz target), not
  a hope.
- **Callback, not iterator** (PLAN §2 sketched `push -> Events`):
  `StreamReader::push(chunk, &mut impl FnMut(Event<'_>))`. The callback's
  higher-ranked lifetime stops a caller from keeping a borrowed event, so no
  internal buffering of events is needed; `Event::into_owned` keeps one.
- **Metadata events are typed.** `Event::Metadata` carries the key, the
  typed value and the raw text. `Metadata::insert` places a value; a
  duplicate or unknown key goes to `Metadata::unknown` with its finding
  (`MZT-T003`, `MZT-T002`), so nothing is lost.
- **Never stops.** Unknown prefixes, rows before their header, repeated
  headers and invalid UTF-8 lines become `ExtraLine::Unrecognized` in the
  layout, with a finding.
- **Codes added:** `MZT-S007`–`S013`, `MZT-T002`–`T008`. Each was checked on
  the corpus; `tests/golden/corpus.txt` records the counts per file.
- **Not yet in the layout** (M3): MTD key order, MTD trailing tabs, per-line
  endings outside extra lines. The events carry all of them.

### D-059: Line coverage of generic code — PROPOSED (M2)

- **Finding:** LLVM's line summary credits a generic function with its
  *best single* instantiation, not the union of all of them. A branch that
  only one table type can reach (`id_confidence_measure[n]` exists in SME
  only) therefore counts as uncovered.
- **Response:** table-independent logic lives in non-generic helpers
  (`typing::Pre`, `table::Special`), and the fixtures drive one
  instantiation of each remaining generic function through every branch. No
  coverage exclusions were needed.

## Writer (M3)

### D-060: Canonical form — PROPOSED (M3)

- **Metadata:** field-reference order, with each element's fields together
  (`assay[1]`, `assay[1]-…`, then `assay[2]`); the 2.0-only fields last
  within their element; `colunit-*` lines in model order; unknown keys
  after everything else.
- **Tables:** in the order SML, SMF, SME, each preceded by one blank line
  (SPEC §5.18: "a blank line separating each" section); columns in header
  order (the model keeps source order, D-010). Rows shorter than the header
  get `null` for their missing cells.
- **Cells:** parameters as `[label, accession, name, value]` with `", "`
  (D-050); numbers as their lexeme; everything else as its `Display`.
- **Line endings:** `\n`; the last line ends with one.
- `tests/golden/full.canonical.mztab` is the canonical form of the full
  fixture.

### D-061: `SourceLayout` and byte-exact round trips — PROPOSED (M3)

- **What is recorded:** for every source line, which model item it carries
  (a metadata key, an unknown key, a header, the next row of a table, or a
  blank/comment/unrecognised line), its line ending, and—only if its text
  differs from the canonical rendering—the text itself plus a 64-bit
  FNV-1a fingerprint of the canonical rendering. Plus the BOM.
- **Writing:** lines are written in source order. A recorded text is used
  only if the item still renders to the recorded fingerprint, so a value
  edited after reading is written canonically and never overwritten by
  stale text. Removed items are skipped; added metadata goes after the last
  metadata line, added rows after their table's last line, and a table with
  no header line in the layout is appended canonically.
- **Guarantee, as tested:** `write(read(f)) == f` byte for byte for every
  input, except lines that are not UTF-8, which come back decoded lossily
  (U+FFFD). Checked on all 21 corpus files, both fixtures, a proptest over
  generated documents, and the `roundtrip` fuzz target.
- **Cost:** reading renders each line once to compare it with its source
  (no allocation: a comparing `fmt::Write`). Measured in M8.
- **Replaces** the M2 list of extra lines in `SourceLayout` with `lines`.

### D-062: The writer refuses what would not read back — PROPOSED (M3)

- `write` returns `Result<(), WriteError>` instead of the `fmt::Result` in
  PLAN §2, because some models built or edited in code cannot be written
  faithfully. `WriteError::Unwritable` names the output line and the reason:
  raw text with a tab or newline, a line ending in `\r`, an invalid cell
  whose raw text is `null`, an empty list, a list of one `null`, a unit
  column name with `=`, a list item that the reader would split
  differently.
- **List splitting is shared.** The reader's splitter (`|` does not split
  inside `[…]` or `"…"` in parameter lists) is one type, `Splitter`; the
  writer checks everything it writes inside a list with it. Found by the
  `roundtrip` fuzz target: the reader kept `[|` as one invalid item, which
  the first writer refused.
- **No `WriteConfig`.** PLAN §2 sketched one; nothing needs configuring yet.

### D-063: Round trips of a model without layout — PROPOSED (M3)

- `read(write(m)) == m` (layouts ignored) holds for every model the reader
  produces, except rows shorter than their header (`MZT-S010`), whose
  missing cells are written as `null` and come back as values. Tested on
  the corpus, the fixtures and generated documents.

## Structure validator (M4)

### D-064: Validation API and locations — PROPOSED (M4)

- `validate::syntax(&str)`, `validate::structure(&MzTab)` and
  `validate::all(&str)`, each taking a `Config` (per-code severity
  overrides, disabled codes). `structure` never re-parses; `all` drops the
  structure layer's `MZT-T028 InvalidValue` because the reader already
  reported each such cell.
- **Locations.** On a model read from text, every structure finding gets
  line, column and cell from the `SourceLayout` (computing the column needs
  the line's text: its verbatim source or a canonical rendering of that one
  item, never a re-scan). An item added after reading points at the first
  metadata line. On a model built in code, findings carry a model path
  (`metadata.assay[1]`, `sml[3][14]`, `smf.header`).
- **Rule set** by `mzTab-version`: 2.0 for `2.0.x`, 2.1 otherwise (D-003).
  2.0 files get the 2.0 mandatory set (D-014) and no profile errors, only
  the inferred profile as information (J-12).

### D-065: Rules as implemented — PROPOSED (M4)

The OPEN decisions above are implemented on their proposals; each is one
place in `validate/` to change if you decide otherwise.

- D-010: every stable column is mandatory (`MZT-T024`, error), plus an
  abundance column for every assay and study variable (SML) or assay (SMF),
  and an SME `id_confidence_measure[n]` for every declared measure; order
  is a warning (`T025`); an `opt_` column before a defined one is a warning
  (`T026`).
- D-011: SMF `charge` is nullable. D-012: a null SML
  `best_id_confidence_value` is information. D-015: a null reliability is a
  warning (`T029`); a value outside `1`-`4` is an error only when no
  `small_molecule-identification_reliability` scheme is declared (`T030`).
- D-013: no group in a 2.1 file is a warning (`T021`); with groups, each
  study variable in exactly one is an error (`T020`).
- D-014: 2.1 mandatory set = `mzTab-version`, `mzTab-ID`, assays (name,
  `ms_run_ref`), `ms_run` locations, study variables (name), CVs (four
  fields), `software`, databases' four fields, protocols' name and type,
  group names; profile-dependent fields per SPEC §5.18.1, and present
  without their table = information (`T016`).
- D-016: one adduct pattern for SML, SMF and SME (`T033`).
- D-017: study-variable values are checked against their group's datatype
  (`T035`).
- D-019: a header-only table in 2.1 is an error with a declared profile,
  a warning with an inferred one (`T015`).
- D-022: an inferred `M+S+F` is a warning (`T014`).
- Cross-references per profile (SPEC table `table-profile-crossrefs`,
  `T022`): `M+S` rows have null `SMF_ID_REFS`; `M+S+F+E` rows have at least
  one; `M+F` rows have null `SME_ID_REFS`; with features and evidence, every
  SME row is referenced by an SMF row. "Identified" SMF rows (which MUST
  reference SME) are not checked: the spec does not define "identified".
- Not checked yet: `spectra_ref` against the run's `id_format`, URIs
  (RFC 3986) and emails. CV rules are M5.

### D-066: Oracle harness, as built — PROPOSED (M4)

- `.github/workflows/oracle.yml` builds the jmzTab-m CLI from the pinned
  submodule (Java 25, `./mvnw -pl cli -am package`), runs it over every
  corpus file and both reader fixtures with `-l Info`, uploads the outputs,
  and fails if committed outputs are stale. It runs on manual dispatch and
  on pull requests that touch it, the outputs or `third_party/`.
- `tests/oracle.rs` compares each committed output with `validate::all`
  by jmzTab-m code and line (`Code::jmztab`), and fails on any disagreement
  not listed in `tests/oracle/expected-disagreements.tsv` with its J-/D-
  reference.
- **No outputs are committed yet:** there is no JVM in this environment
  (D-030), so the first run of the workflow, and the review of its
  disagreements, is still to do. Until then the comparison test passes
  vacuously, and says so here. The seed rows of the disagreement list are
  the deviations already known from reading jmzTab-m (J-01, J-02, J-04).
