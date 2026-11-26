> Provenance: drafted by an AI coding agent during development, with the
> maintainer's decisions quoted where they were made. Entries are
> addressed to the maintainer ("you").

# DECISIONS

A running log of spec ambiguities, conflicts between the schema and the prose,
deviations in jmzTab-m, and the choices this crate makes about them.

If you want to know why the validator reports (or ignores) something, look
up the finding's decision here. Most entries are still `PROPOSED`: the
code follows them, but they have not been formally approved or
reviewed by anyone outside the project. Disagreement with any of them is a welcome issue.

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

### D-004: JSON representation has two incompatible dialects in the corpus — DECIDED (M6, default; see D-071)

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
`docs/agents/jmztab-m-rules.md` §Quirks.

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
- **`.vscode/settings.json`:** initially skipped in the sandbox; adopted when
  adopting `rusti-graben` updates on the polish branch.
- **`CITATION.cff` and `codemeta.json`:** `CITATION.cff` was adopted once author
  contact and ORCID details were provided (`doi`/`version`/`date-released` remain
  marked `TODO(human)` until release); `codemeta.json` is not adopted yet.

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

## Semantic validator and CV (M5)

### D-067: Semantic checks, as built — PROPOSED (M5)

- API (PLAN §2): `validate::semantic(&MzTab, &Config, &dyn TermLookup)`,
  and `validate::all` now takes the lookup too. `cv::NoTerms` holds no
  vocabulary; then only the checks that need no terms run.
- `TermLookup` answers `covers(prefix)`, `term(accession)` (name,
  obsolete), `is_descendant(accession, ancestor)` (strict) and, optionally,
  `version(prefix)`. An accession whose prefix the lookup does not cover is
  never judged: its term checks are skipped, and a rule that would need it
  is skipped, not failed. Slices and arrays of lookups combine (the first
  that covers a prefix answers). No network I/O anywhere: vocabularies come
  from the caller.
- Codes (layer `Semantic`, prefix `X`):
  - `X001` a CV parameter's label is not declared by a `cv[n]-label`
    (warning, once per label at its first use). The spec implies it ("the
    labels of the controlled vocabularies/ontologies used in the mzTab
    file") without a MUST.
  - `X002` a declared CV is used by no parameter and no
    `opt_..._cv_<accession>_...` column (information).
  - `X003` an accession has no `PREFIX:` part (warning; jmzTab-m 1035).
    **The prefix is not required to equal the label:** the field
    reference's own example is `[MIRIAM, MIR:00100079, HMDB, ]`.
  - `X004` unknown term in a covered vocabulary (error), `X005` name
    differs from the vocabulary's, ignoring ASCII case (warning), `X006`
    obsolete term (warning), `X010` declared `cv[n]-version` differs from
    the lookup's (information).
  - `X007` a CV parameter breaks its field's CV-mapping rule, at the rule's
    level (MUST error, SHOULD warning, MAY information); `X008` a term the
    rule does not let repeat appears twice in one element or row; `X009` a
    value matches more than one term of an `XOR` rule.
- **User parameters pass every rule:** the format specification ("Params")
  allows a user parameter wherever a parameter is allowed.
- **Repeats (`X008`) are the same accession twice**, not two different
  children of a non-repeatable root: the field reference defines fields
  such as `instrument[n]-analyzer[1-n]` and `sample[n]-species[1-n]` as
  lists, which `isRepeatable="false"` read literally would forbid (D-002).
- A rule's absence check (a MUST field with no value) is the structure
  layer's (`T009`), not repeated here.
- Not checked: unit (`has_units`) and value-type (`value-type`) qualifiers
  of terms; `spectra_ref` against the `id_format` term (table-1).

### D-068: CV-mapping rules: generated, with corrections — PROPOSED (M5)

- `crates/mztab-m/src/cv/rules.rs` is generated from the pinned
  `schema/mzTab_2_1-M_mapping.xml` by `tests/cv_rules.rs`, which fails when
  the committed file is stale (`BLESS=1` rewrites it): 32 rules, one per
  field, mapped from `cvElementPath` to `cv::Target`. `cvIdentifierRef` and
  `scopePath` are not used (the scope is the element or row of the field;
  `ms_level_must` names `smallMoleculeSummary` as its scope, a slip).
  `useTerm="yes"` reads as true. An `AND` rule with several terms would
  fail generation (none exists).
- Corrections, each because the field reference says otherwise (D-002):
  - `ms_level_must`: `MS:1000511` itself is allowed (`useTerm` false in the
    mapping); the field reference's example is `[MS, MS:1000511, ms level,
    2]`.
  - `identification_method_must`: `MS:1000531` (software) and its
    descendants are added; the field reference says "the name of software,
    database or manual curation", with `[MS, MS:1001477, SpectraST,]` as
    its example.
- Not corrected: `best_id_confidence_measure_must` keeps "small molecule
  confidence measure", which matches the field's description ("the best
  confidence measure ... for this type of score"). Its example,
  `[MS, MS:1001477, SpectraST,]`, therefore breaks the rule: a defect in
  the example, reported as `X007` when the lookup holds PSI-MS.

### D-069: `mztab-m-obo` — PROPOSED (M5)

- A zero-dependency (besides `mztab-m`), `no_std + alloc`, zero-copy OBO
  1.4 reader: `Ontology::parse(&str)` never fails. It keeps `[Term]`
  stanzas' `id`, `name`, `is_obsolete`, `is_a` and `relationship:
  part_of`, and the header's `data-version`; everything else is skipped.
  A stanza without `id` is dropped; a repeated `id` keeps the first.
- Ancestors follow `is_a` **and `part_of`**, as the PSI semantic validator
  expands term subtrees; other relationships (`has_units`, ...) are not
  followed. Cycles end (each term is visited once).
- `version(prefix)` is the `data-version` for every prefix the file
  covers.
- The OBO files themselves are not vendored or downloaded by the crates
  (PLAN: no network I/O); the CLI (M7) will take them as paths.

### D-070: The base test document is semantically clean — PROPOSED (M5)

- `tests/fixtures/full.mztab` now declares every CV it uses (eleven more
  `cv[n]`), and its `sample_processing[1]` uses MSIO terms as the field
  reference does. `tests/support/terms.rs` is a small PSI-MS stand-in
  (`Mini`, the terms the document uses and their ancestors, plus made-up
  terms for negative cases); the rule suite runs the semantic layer with
  it, and the base document triggers nothing.
- The corpus golden (`tests/golden/corpus.txt`) records semantic codes
  with `NoTerms`: undeclared CVs in five files, and six `X007` in the
  MS-DIAL GC-MS file, whose `scan_polarity` accessions are `MS: 1000130`
  (with a space), which no allowed term matches.

## JSON (M6)

### D-071: JSON through TSV text — PROPOSED (M6)

- Feature `json` (off by default): `json::parse(&str) -> Parsed<'static>`,
  `json::write` and `json::to_string`. JSCH dialect only (D-004); a
  document with the snake_case table keys (MetaboScape) gets `S017` and
  nothing is read. A hand-rolled RFC 8259 parser (nesting limit 128; a
  leading BOM is skipped).
- **One grammar.** The reader carries each JSON value into the TSV line
  that holds it and lets the TSV reader type it; the writer writes the
  model as canonical TSV and carries each line into JSON. So a document
  reads the same from either format, and the TSV reader's findings are
  relocated to JSON pointers (`Location::Json`, e.g.
  `/smallMoleculeFeature/0/charge`). JSON syntax errors are located by
  line and column. The model owns its text (escaped strings), hence
  `'static`.
- **Strings carry TSV text.** Where a structured value is expected
  (parameter, reference, list, number), a JSON string is that value's TSV
  text. The writer uses this for what it cannot put in JSON form (an
  invalid cell, a number in scientific notation or infinity), so it reads
  back unchanged.
- **Nulls.** In metadata, JSON `null` (or a missing member) is an absent
  key and the string `"null"` is a key whose value is `null` (as in
  `database[2]-prefix null`); jmzTab-m writes `null` for absent members.
  In table rows every column has a cell, so `null` is a `null` cell.
- **Numbers.** JSON numbers with exponents are written out exactly in
  plain notation (mzTab forbids exponents; jmzTab-m writes them); the
  writer writes plain numbers as JSON numbers and anything else as a
  string.
- **Indexed keys.** `sample[1]-species[n]`, `instrument[1]-analyzer[n]`,
  `custom[n]`, `derivatization_agent[n]`, `id_confidence_measure[n]`, ...
  are arrays of `Parameter` objects whose `id` is `n` (JSCH has the
  member); an item without `id` takes its position + 1. String arrays
  (`software[n]-setting[m]`) are by position. So that an item that is not
  a `Parameter` object (a string carrying an invalid value) keeps its
  index, the writer pads the array with `null` items, which the reader
  skips; more than 1024 padding items make the model unwritable. (Both
  found by the `json` fuzz target, as was the rule that a key the reader
  keeps as unknown metadata, such as `x[0]`, is refused. Also found by
  fuzzing: a repeated member is read once, the first, with `S015`; a TSV
  header that repeats a column is refused.) `MsRun.name` (JSCH, no TSV
  key) is ignored; `ms_run[n]-usi_identifier` and
  `study_variable[n]-factors` (TSV keys missing from JSCH 2.1) are
  written under those names (the spec text wins, D-002).
- **Tables.** A row's keys in order define the header (taken from the
  first row; later rows' unknown keys are `S016`). The abundance arrays
  (`abundance_assay`, `abundance_study_variable`,
  `abundance_variation_study_variable`, SME `id_confidence_measure`) are
  indexed by the metadata ids of their element, in ascending order; `opt`
  items are columns `opt_<identifier>`. A table with no rows reads with
  the standard header (spec order, one array column per metadata id).
- **The writer refuses (`WriteError::Unwritable`) what JSON cannot
  carry**, never dropping it silently: unknown or repeated metadata keys,
  unknown columns, rows with more cells than their header, abundance
  columns that are not exactly the metadata ids in ascending order, and a
  table without rows whose header is not the standard one. The line
  number is in the canonical TSV. A lossy conversion, if wanted, is the
  CLI's (M7).
- **Comments** are not carried: the writer writes `"comment": null` and
  the reader ignores `comment` members (they are presentation, D-061).
- **Corpus agreement** (`tests/golden/json-corpus.txt`): of the 21 TSV
  files, 9 read back from JSON as the same model; the two LipidCompass
  files as the same up to column order (they interleave
  `abundance_study_variable[n]` and `abundance_variation_study_variable[n]`,
  which JSON groups); 10 are refused (unknown keys such as
  `instrument[1]-analyzer` without its index, unknown columns, rows with
  an extra trailing cell). The jmzTab-m JSON example (Progenesis) reads
  with no findings and is a fixed point of write-then-read.
- Codes `S014`-`S018` (syntax layer): not JSON, wrong JSON form, unknown
  member (warning, ignored), other dialect, a string with a tab or line
  feed (or ending a line with a carriage return), which TSV cannot carry.

## CLI and WASM demo (M7)

### D-072: The `mztab-m` command and the browser demo — PROPOSED (M7)

- **`mztab-m-cli`** (binary `mztab-m`): `validate`, `fmt`, `convert`,
  `inspect`. Dependencies: `mztab-m` (with `json`) and `mztab-m-obo` only;
  arguments are hand-parsed (PLAN §5). All logic is in the library's
  `run(args, &mut dyn Io) -> u8`; `main` only supplies files and standard
  streams, so the commands are tested in memory and the binary once.
- **Input:** a file or `-` (standard input). JSON if the name ends in
  `.json` or the text starts with `{`; `--input tsv|json` overrides.
- **`validate`:** the reader's findings and each layer up to `--layer`
  (default `semantic`), with `--severity CODE=LEVEL|off` applied to all of
  them. An untyped value is reported once, by the reader (as
  `validate::all` does). Vocabularies come only from `--cv FILE.obo`
  (repeatable); nothing is downloaded. Output: one finding per line on
  stdout and a count on stderr, or `--format json` (the core's
  `json::write_findings`, an array of objects with code, severity, layer,
  location, summary, detail, spec URL and jmzTab-m codes).
- **Exit codes:** 0 ok; 1 a finding at or above `--fail-on` (default
  `error`; `never` disables), or `fmt --check` would change the file; 2 a
  bad command line; 3 a file that cannot be read or written, or a model
  that cannot be written in the requested format (a JSON conversion that
  D-071 refuses is an error, not a silent loss; no lossy option yet).
- **`fmt`:** the canonical form in the input's format (TSV stays TSV, JSON
  stays JSON); `--check` compares it with the input bytes. **`convert`:**
  `--to tsv|json`. **`inspect`:** version, profile (declared and implied
  by the tables), element counts, rows per table, and the reader's finding
  counts, as text or JSON.
- **`mztab-m-wasm`** (`publish = false`): `validate(text, obos)` returns
  the findings as JSON, `convert(text, to)` the converted text. Its one
  extra dependency is `wasm-bindgen`, pinned to the version of the
  test runner. `www/index.html` is a self-contained page (no external
  resources) that validates a dropped file in the browser, takes OBO files
  for the semantic layer, and downloads the JSON or TSV form;
  `./mk local:demo` builds `www/pkg` (not committed) and runs in CI. The
  crate is tested natively and on wasm32.

## Benchmarks (M8)

### D-073: Benchmark crate and what the numbers say — PROPOSED (M8)

- **`benches/` (`mztab-m-bench`, `publish = false`).** It holds a seeded
  synthetic generator (valid 2.1 `M+S+F+E` documents of any size, produced
  line by line), a criterion throughput bench and a dhat heap bench.
  `criterion`, `dhat` and `memchr` are dev-dependencies of this crate only.
- **`criterion` is pinned to 0.5.** 0.8 builds the `alloca` crate's C
  object, which this toolchain (Xcode clang 21) fails to link ("invalid
  value for 'frame-pointer' attribute"). Revisit when either side moves.
  `criterion_group!` is not used, because its generated function has no
  docs; the bench writes its `main` instead.
- **No `memchr` feature in the core (PLAN §5 asked to measure first).** A
  `memchr` scan is 1.3–1.8× faster than the std split. But the scan is
  under a fifth of parse time, which goes to typing cells and recording the
  layout, so the feature would gain a few percent at the cost of the core's
  first dependency. Recorded in `docs/BENCHMARKS.md`.
- **The streaming bound is enforced:** the heap bench asserts a peak under
  64 MB while streaming a ~1 GB synthetic document. It measured 0.87 MB.
- **The jmzTab-m comparison is not done.** It needs the jmzTab-m CLI built
  (a Maven download, not approved yet) and hyperfine.

## Release preparation (M9)

### D-074: What would be published, and how — PROPOSED (M9)

- **Three crates would be published:** `mztab-m`, `mztab-m-obo` and
  `mztab-m-cli`. The names were free on crates.io on 2026-11-19. The demo
  and the benches are `publish = false`.
- **Packages carry sources and the README only** (`include`). The test
  suites read the pinned spec and the corpora from `third_party/`, which a
  package cannot carry; the repository is where they run.
- **`cargo publish --dry-run --workspace`** (excluding the two unpublished
  crates) packages and verifies all three in dependency order, through a
  temporary local registry. It was run, it passed, and it uploaded
  nothing. **Publishing needs your approval** and would go in that order:
  `mztab-m`, then `mztab-m-obo`, then `mztab-m-cli`.
- **Each finding code's rustdoc links its rule** at the pinned spec
  commit. The link is generated by the `codes!` macro from the same anchor
  that `Code::spec_ref` returns and `tests/spec_anchors.rs` checks. The
  commit hash is one macro (`spec_commit!`) behind both `SPEC_COMMIT` and
  the links.
- **Documentation:** `README.md` links each guarantee to the gate that
  enforces it; the core's README is its crate documentation, with doctested
  examples; `docs/DESIGN.md` explains the architecture.

## Closing the gaps (after M9)

### D-075: URIs, e-mails, spectrum references, value types, comments, lossy conversion — PROPOSED

- **URIs (`T036`, error):** values of the six fields the field reference
  types as URI must be RFC 3986 URIs (format specification, "URIs ...
  MUST follow RFC 3986 and RFC 8089"). The six are `uri[n]`,
  `external_study_uri[n]`, `ms_run[n]-location`, `assay[n]-external_uri`,
  `cv[n]-uri` and SME `uri`. A URI needs:
  - a scheme;
  - only the characters RFC 3986 allows;
  - `%` followed by two hex digits;
  - at most one `#`.
  A Windows path (`D:\data`) or a space fails. `database[n]-uri` and SML
  `uri` are typed String and String List, so they are not checked. No corpus
  file triggers this.
- **E-mail (`T037`, warning):** `contact[n]-email` must look like
  `local@domain.tld`. The field is typed String but described as an e-mail
  address, hence a warning.
- **Spectrum references (`T038`, warning):** each `spectra_ref` of an SME
  row must have the `key=value` form that spec table 1 gives for its run's
  `id_format` term:
  - Thermo: `controllerType=… controllerNumber=… scan=…`;
  - multiple peak lists: `index=…`;
  - ... ten formats in all.
  An `id_format` the table does not constrain (`MS:1001530` mzML ids, user
  parameters) is not checked. The table says "should", hence a warning.
- **Value types (`X011`, error):** a CV parameter's value, when present,
  must match its term's XML Schema `value-type` (PSI-MS
  `xref: value-type:xsd\:int`, read by `mztab-m-obo` into
  `Term::value_type`). A missing value is not reported, because mzTab names
  score types without values (`id_confidence_measure[n]`).
- **Units are not checked, and this is deliberate.** PSI-MS `has_units`
  says which unit a value of a term is in. An mzTab parameter
  `[label, accession, name, value]` has no unit slot, and units of columns
  are given by `colunit-*` lines, whose columns are not CV terms. There is
  nothing to check a unit against.
- **JSON comments:**
  - The writer turns `COM` lines into the top-level `comment` array:
    `prefix`, `msg`, and the source `line_number`.
  - The reader keeps them as a layout of comment lines only. Written as
    TSV, they follow the metadata; their original position is not kept.
  - A layout that holds only comments locates findings by model path, as
    an empty one does.
  - Canonical TSV (`fmt`, `convert`) now keeps comments, after the
    metadata, instead of dropping them.
- **Lossy conversion:** `MzTab::strip_unknown` drops unknown metadata lines,
  unknown or repeated columns with their cells, and cells beyond a row's
  header. It reports the counts. `mztab-m convert --lossy` (and the
  bindings' `lossy`) use it before writing. Abundance columns that do not
  match the metadata still make JSON refuse.

### D-076: A shared API crate, and Python and R packages — PROPOSED

- **`mztab-m-api`** (`no_std`, published with the others) holds what every
  front end needs, as plain functions over text:
  - detect the format;
  - read;
  - validate up to a layer, with `CODE=LEVEL` settings and OBO
    vocabularies;
  - write canonically (optionally lossy);
  - flat finding `Record`s;
  - `inspect` facts.
  The CLI and the WASM demo now use it, as do the bindings.
- **Python (`bindings/python`, root `pyproject.toml` + `uv.lock`, package `mztab-m`, module `mztab_m`):**
  - PyO3 0.30 with the stable ABI from Python 3.9, so one wheel per
    platform covers every Python version;
  - built by maturin from the repository root (`manifest-path =
    "bindings/python/Cargo.toml"`, `uv pip install .`, `pip install .`,
    `uv sync`);
  - `validate`, `convert`, `canonical`, `inspect` and `codes`, returning
    dicts and strings and raising `ValueError`;
  - hybrid package (`__init__.py`, type stubs `_native.pyi`, `py.typed`)
    plus doctests.
- **R (`bindings/r`, package `mztabm`):**
  - extendr 0.9, built by `R CMD INSTALL` or `devtools::install("bindings/r")`
    from `src/rust`;
  - `bootstrap.R` + `.Rbuildignore` + `Config/build/bootstrap: TRUE` +
    `Config/build/copy-method: link` and hardened `Makevars` stage the
    workspace crates (`mztab-m`, `mztab-m-obo`, `mztab-m-api`) for
    self-contained source tarballs (`./mk bindings r`), while direct
    checkouts build against `../../../../crates/`;
  - the same functions, with `validate` and `codes` returning data frames;
  - errors come back as `list(value, error)` and are raised by R, because
    an extendr `Err` would first print a Rust panic message.
- **Both sit outside the Rust workspace, like `fuzz/`.** They need a
  Python interpreter or R headers, which the workspace gates do not have.
  `.github/workflows/bindings.yml` builds and tests them: unittest and
  doctests for Python, testthat for R. Locally, Python is tested by
  building with maturin from the repository root, and R by building a
  staged source tarball and installing it into a scratch library.
- **Not done:**
  - **CRAN:** workspace crates now stage into self-contained R source
    tarballs, though CRAN would still need external `crates.io`
    dependencies (`extendr-api`, etc.) vendored offline.
  - **PyPI:** wheels are not uploaded. Publishing either needs approval.

### D-077: The corpus against the real PSI-MS — PROPOSED

- **Pinned release:** PSI-MS `v4.2.17`
  (`HUPO-PSI/psi-ms-CV`, SHA-256 `354b666d…6ffcf8`, 6120 terms).
  `./mk local:vocab` downloads it into `target/vocab/` (not committed),
  checks the hash and runs `crates/mztab-m-api/tests/vocabulary.rs`, which
  compares the semantic codes per corpus file with
  `tests/golden/psi-ms.txt`. It is a nightly gate. It is marked `#[ignore]`
  so normal test runs report it honestly instead of passing without checking.
- **What the real file changed in the code:**
  - PSI-MS now declares value types as `relationship: has_value_type
    xsd:int`, not `xref: value-type:`; `mztab-m-obo` reads both.
  - An OBO file covers **its own namespace only**: the header's
    `ontology:` (`ms` covers `MS`). `psi-ms.obo` also carries 53 UO and 16
    NCIT terms, and covering those prefixes made every other UO term
    "unknown" and compared UO's declared version with PSI-MS's.
    `Ontology::cover` widens coverage on request.
  - **EXACT synonyms** count as the term's name (`[MS, MS:1000133, CID, ]`
    is right: "CID" is an exact synonym of "collision-induced
    dissociation"). There are 359 such synonyms.
- **More mapping-file corrections, from the spec text** (D-002, as in
  D-068):
  - `msrun_id_format_may` also allows `MS:1001530` (mzML unique
    identifier), which spec table 1 lists and the field reference uses as
    its example.
  - Both quantification-unit rules also allow descendants of `MS:1001805`
    (quantification datatype). The field reference describes the field as
    "what type of units are reported in the ... abundance fields" and gives
    "peak area", a quantification datatype, as its example.
  - That example's accession, `MS:1001113`, is "c-terminal flanking
    residue"; peak area is `MS:1001845` (now obsolete). The example itself
    is a defect.
- **What it finds:**
  - Each file gets one `X010`: its declared PSI-MS version is not 4.2.17.
  - The rest are real problems in the files:
    - riken: `MS:1001582` (the software XCMS) labelled "accurate mass",
      1238 times in one file;
    - masster: the generic "small molecule confidence measure" carrying
      the tool's own score names, which the rules also reject as an
      identification method;
    - LDA: descriptions in place of term names;
    - MTBLS263: a spectral-similarity term as a confidence measure;
    - MS-DIAL GC-MS: `MS: 1000130` with a space;
    - names PSI-MS has since changed (`mzML file` → `mzML format`).
  - The base test document finds one name (the fixture's "CID", now
    accepted), one information (its group unit, "dimensionless unit", is
    not a time, mass or concentration unit) and its old declared version.
