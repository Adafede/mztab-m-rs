# Phase 1: Plan

Status: **draft, waiting for your review.** This plan builds on
`docs/recon/PHASE0.md` and `DECISIONS.md`.

Where a decision is still open, the plan uses the proposal recorded in
DECISIONS.md and marks it **[default: D-xxx]**. You can override any of them
without touching the architecture, because every one of those choices lives in
validator rules or severities, not in the type model.

## 1. Workspace layout

```
Cargo.toml                    workspace (rusti-graben conventions, lints inherited)
crates/
  mztab-m/                    CORE. #![no_std] + alloc, zero deps, forbid(unsafe)
    src/lib.rs                re-exports + crate docs (README include)
    src/lex.rs  lex/          zero-copy line/cell lexer, chunked push reader
    src/param.rs              Param grammar ([label, accession, name, value])
    src/value.rs              Cell<T>, Decimal, Integer, lists, dates, URI-ish checks
    src/id.rs                 typed indices (AssayId, MsRunId, ...), references
    src/model.rs  model/      MzTab, Metadata (+ sub-objects), Sml/Smf/Sme tables
    src/read.rs   read/       events -> model builder (TSV)
    src/write.rs  write/      model -> canonical TSV
    src/json.rs   json/       [feature "json"] hand-rolled RFC 8259 reader/writer + JSCH mapping
    src/diag.rs   diag/       Finding, Code, Severity, Location, SpecRef, message rendering
    src/validate.rs validate/ syntax | structure | semantic layers, Config, profiles
    src/cv.rs     cv/         TermLookup trait, CV-mapping rules (generated, committed)
  mztab-m-obo/                optional: pure OBO 1.4 parser -> impl TermLookup (zero deps, no_std+alloc)
  mztab-m-cli/                bin `mztab-m`: std I/O adapter; validate | fmt | convert | inspect
  mztab-m-wasm/               wasm-bindgen demo (publish = false) + static HTML page
benches/                      criterion + dhat (publish = false, excluded from wasm)
fuzz/                         cargo-fuzz targets (outside workspace, nightly gate)
tests/corpus/                 vendored spec examples (CC-BY-4.0) + jmzTab-m fixtures (Apache-2.0), NOTICE
tests/oracle/                 committed jmzTab-m outputs + expected-disagreements.toml
tools/                        generator for cv rules + synthetic large-file generator (Rust, in benches or xtask-like bin)
docs/  DECISIONS.md  DESIGN.md  DEBT.md  adr/
```

The crate name `mztab-m` was available on crates.io when checked on
2026-11-17: the API returned 404 for `mztab-m`, `mztab_m` and `mztab`. **Nothing will be
published without your approval.**

## 2. Core API (public surface, kept small)

```rust
// Reading: never panics, never aborts early. You always get a document and its findings.
pub fn parse(input: &str) -> Parsed<'_>;                 // zero-copy model borrowing input
pub fn parse_bytes(input: &[u8]) -> Parsed<'_>;          // UTF-8 check + BOM strip -> findings
pub struct Parsed<'a> { pub mztab: MzTab<'a>, pub findings: Findings }
impl MzTab<'_> { pub fn into_owned(self) -> MzTab<'static>; }

// Streaming: push-based, no I/O in core. The caller feeds chunks.
pub struct StreamReader { /* owns only a partial-line buffer */ }
impl StreamReader {
    pub fn new(cfg: ReadConfig) -> Self;
    pub fn push<'s>(&'s mut self, chunk: &'s [u8]) -> Events<'s>;  // complete lines only
    pub fn finish(&mut self) -> Events<'_>;
}
pub enum Event<'a> { Metadata(MtdEntry<'a>), Header(Header<'a>), Sml(SmlRow<'a>),
                     Smf(SmfRow<'a>), Sme(SmeRow<'a>), Comment(Comment<'a>), Blank(Span),
                     Finding(Finding) }

// Writing: pure. Output goes to any fmt::Write target, including String.
pub fn write(mztab: &MzTab<'_>, out: &mut impl core::fmt::Write, cfg: &WriteConfig) -> fmt::Result;
pub fn to_string(mztab: &MzTab<'_>) -> String;

// Validation: three layers, each callable on its own, all pure. Each returns every finding it produces.
pub mod validate {
    pub fn syntax(input: &str, cfg: &Config) -> Findings;                // lexical, from raw text
    pub fn structure(m: &MzTab<'_>, cfg: &Config) -> Findings;            // fields, types, profiles
    pub fn semantic(m: &MzTab<'_>, cfg: &Config, cv: &dyn TermLookup) -> Findings;
    pub fn all(input: &str, cfg: &Config, cv: &dyn TermLookup) -> Findings;
}

#[cfg(feature = "json")] pub mod json { pub fn parse(..) -> Parsed<'_>; pub fn write(..); }
```

- **"Pure":** the core has no globals, no I/O, no interior mutability, and no
  clocks. All configuration is an explicit `Config` value. `StreamReader::push`
  takes `&mut self` on a value the caller owns, so no mutation is hidden.
- **Layers are independent:**
  - `structure` and `semantic` take a typed model and never re-parse.
  - Models built in code have no spans. Their findings carry a model path
    instead of a line and column.

## 3. Type model

### Principles

**Make illegal states unrepresentable where the spec is unambiguous.** Where
the input is invalid, keep it losslessly so the validator can report it.

**The central device is `Cell<T>`.** Every typed field read from a file is a
`Cell<T>`:

```rust
pub enum Cell<'a, T> { Value(T), Null, Invalid(Raw<'a>) }
```

- A well-formed model contains no `Invalid` cells.
- The reader still produces `Invalid` cells for bad input. That is how
  "collect all findings" and "validate the typed model" can both hold.
- The cost: the type system alone cannot prove that a model is valid.
  `MzTab::is_well_formed()` and the `structure` layer enforce it instead
  (documented in DESIGN.md).

**Lexemes are preserved.** `Decimal<'a>` keeps the original lexeme (for
example, `1.50`) next to the lazily parsed value (`to_f64()`). `NaN` is its own
variant. This is what makes `write(read(f)) == f` possible without
float-formatting drift.

**`Param` is an enum, so a half-CV parameter cannot be constructed:**

```rust
pub enum Param<'a> {
    Cv { label: Str<'a>, accession: Accession<'a>, name: Str<'a>, value: Str<'a> },
    User { name: Str<'a>, value: Str<'a> },
}
```

- `Accession` is a validated `PREFIX:LOCAL` CURIE.
- An empty name cannot be constructed.
- Quoting is handled by the lexer and writer.

**Typed indices.** `AssayId`, `MsRunId`, `SampleId`, `StudyVariableId`,
`StudyVariableGroupId`, `InstrumentId`, `ProtocolId`, `CvId`, `DatabaseId` and
`IdConfidenceMeasureId` each wrap a `NonZeroU32`. Cross-references use the
target's own type (`assay.ms_run_refs: Vec<MsRunId>`), so pointing an assay at
a sample is a compile error.

**Indexed collections.** `IndexMap<Id, T>` is a small sorted vec map; no
dependency. Lookup is by id, and sparse or non-contiguous indices stay
representable so the validator can report them.

**Closed vocabularies are enums:** `Profile`, `Section`, `XsdDatatype`,
`AmbiguityCode` (1..=3), `PublicationItem` (`Doi` | `Pubmed` | `Uri`). Each has
a `Raw` fallback only where the reader needs to stay lossless.

**`Version { major, minor, patch }`** dispatches the rule set
(`RuleSet::V2_0` | `V2_1`); an unknown version uses the newest rule set. **[default: D-003]**

### Tables

- Each table keeps its `Header`: an ordered column list with
  `ColumnKind::{Stable(StableCol), Abundance(..), IdConfidence(n), Opt(OptColumn)}`.
  Rows are structs with named stable fields. Abundance and `opt_` columns are
  stored as vecs aligned to the header.
- Source column order is preserved. **[default: D-010]**
- `opt_` columns are typed by identifier: `Global`, `Assay(id)`,
  `StudyVariable(id)` or `MsRun(id)`, plus an optional `cv_` Param. Values stay
  raw strings, and the spec gives them no meaning.

### Extensibility and losslessness

- `custom[n]` and `colunit-*` are typed.
- Unknown MTD keys are kept as `Metadata::unknown: Vec<(Key, Raw)>` in source
  order and flagged by the validator.
- Comments are anchored to their position (`before: Anchor`) so the writer can
  put them back.
- `SourceLayout` records MTD key order, line-ending style, BOM, and blank-line
  placement. The writer honours it when present.

### What the type system cannot enforce

The type system cannot enforce:

- cross-reference existence
- id uniqueness
- profile-conditional mandatory fields
- `|`-list arity equality in SML
- `study_variable` values matching their group's `datatype`
- CV term validity
- MTD order
- non-contiguous indices
- `spectra_ref` matching its `id_format` pattern

Each of these is a named rule in `validate`, and DESIGN.md lists every one
with its rule code.

## 4. Error / finding model

```rust
pub struct Finding {
    pub code: Code,              // #[non_exhaustive] enum, stable string id
    pub severity: Severity,      // Error | Warning | Info (default derived from RFC-2119 level)
    pub location: Location,
    pub detail: Detail,          // structured args (enum per code family), no preformatted strings
}
pub enum Location {
    Text { line: u32, column: u32, byte: Range<u32>, cell: Option<u16> }, // 1-based line/col
    Json(JsonPointer),
    Model(ModelPath),            // e.g. sml[12].abundance_assay[3], metadata.assay[2].ms_run_ref
}
impl Code {
    pub const fn id(self) -> &'static str;        // "MZT-S001", "MZT-T042", "MZT-X007"
    pub const fn layer(self) -> Layer;            // Syntax | Structure | Semantic
    pub const fn spec_ref(self) -> SpecRef;       // section number + anchor + URL at pinned commit
    pub const fn requirement(self) -> Requirement;// Must | Should | May (RFC 2119)
    pub const fn jmztab(self) -> &'static [u16];  // equivalent jmzTab-m code(s), for differential tests
}
```

- **Codes are our own.** Prefixes: `S` = syntax, `T` = structure, `X` =
  semantic. A code is never reused or renumbered. jmzTab-m numbers appear only
  as a mapping. **[default: J-17]**
- **Spec references:** every code links to a section of the spec at the pinned
  commit. A test checks that each anchor exists in the vendored spec text.
- **Severity override:** `Config` can override severities per code, which is
  how validation profiles plug in later.
- **Messages:** `Display for Finding` renders English from `detail`. No strings
  are allocated during validation unless they are rendered.
- **No panics:**
  - The workspace already denies `unwrap_used`, `expect_used`, `panic` and
    `indexing_slicing`.
  - Fuzzing checks that `parse` never panics on arbitrary bytes.
  - Resource bounds (maximum line length and maximum cells) are configurable
    and default to generous values. Exceeding one produces a finding, not an
    abort.

## 5. Dependency budget

| Crate | Runtime deps | Dev / build deps | Justification |
|---|---|---|---|
| `mztab-m` core | **0** | `proptest` (dev), `wasm-bindgen-test` (dev, wasm only) | Core is pure `no_std + alloc`. `core::error::Error` has been stable since 1.81, so `std` is not needed. Lexing, decimals, regex-like checks and JSON are hand-rolled; see below. |
| `mztab-m-obo` | 0 | – | OBO is a simple line format; a hand-rolled parser is about 300 LOC. |
| `mztab-m-cli` | 0 (proposed) | – | Arguments are hand-parsed: four subcommands and fewer than 10 flags. clap would add about 10 crates and noticeable compile time. |
| `mztab-m-wasm` | `wasm-bindgen` | – | Exporting without it needs `unsafe` (raw pointers across the ABI). The crate is demo-only and is not published. |
| `benches` | – | `criterion`, `dhat` | Measurement only. Duplicate-version skips go in `deny.toml` and are recorded in DEBT.md. |
| `fuzz` | – | `libfuzzer-sys`, `arbitrary` | Standard. Kept outside the workspace. |

Hand-rolled instead of a dependency:

| Instead of | Reason | Risk and mitigation |
|---|---|---|
| `regex` | Only 6 small fixed patterns are needed: adduct, ORCID, version, CURIE, `opt_` header, `spectra_ref` id_formats. `regex` alone adds about 1 MB to the wasm build. | Each matcher is written by hand and checked against the JSCH regex with a dev-only differential test. |
| `serde_json` | The JSCH mapping is bespoke: hyphenated keys, `[n]`-index references. Derive would need custom implementations everywhere, and `syn` is costly to compile. | Fuzzed, and tested against JSONTestSuite cases vendored as data. |
| `memchr` | Tab and newline scanning are simple. | **Expected loss** of perhaps 1.5–3× in raw lexing throughput on large files. To be measured; an optional `memchr` feature can be added later if you want it. |

`proptest` is a dev-dependency only and does not affect users. The
alternative is to hand-roll generators; my recommendation is proptest.

## 6. Test strategy

### Conformance

- All 23 vendored spec examples are run.
- Each file gets: parse with no panic; an expected-findings snapshot
  (`tests/golden/*.findings`, a stable text format of code + location);
  canonicalisation reaching a fixed point; and JSON ↔ TSV agreement for the
  progenesis example.
- The jmzTab-m fixtures, valid and invalid, are vendored with expected
  findings.

### Negative suite

- One minimal failing input per rule code, generated as table-driven tests.
- A meta-test asserts that every `Code` variant has at least one triggering
  test and one non-triggering test, so a code cannot go untested.

### Differential testing against jmzTab-m

- `./mk oracle` runs the pinned jmzTab-m CLI JAR (sha256-checked) in CI only,
  with `-l Info`, over the corpus.
- The outputs are committed under `tests/oracle/`.
- `cargo test` maps codes through `Code::jmztab()` and compares by (code, line).
- Every disagreement must be listed in `tests/oracle/expected-disagreements.toml`
  with a J-xx or D-xxx reference. Anything unlisted fails the build.
- This is reported in DESIGN.md.
- **Dependency:** a JVM in CI (D-030).

### Properties (proptest)

- `read(write(x)) == x` for arbitrary well-formed models.
- `write(read(write(x))) == write(x)`.
- `parse(any bytes)` never panics.
- Validation is deterministic and independent of finding order.
- Every finding has a location.

### Round-trip guarantees

| Input | Guarantee |
|---|---|
| Canonical file (writer output; LF; canonical MTD and column order) | `write(read(f)) == f`, byte for byte. |
| Non-canonical file with `SourceLayout` preserved | `write(read(f)) == f` for LF/CRLF, BOM, MTD order, column order, comments, and blank lines. |
| Any input | Idempotent: `write(read(write(read(f)))) == write(read(f))`. |
| Leading/trailing cell whitespace | Byte-exactness is **not** promised; it is reported as a finding. |

### Fuzzing

Targets: `lex`, `parse_bytes`, `json::parse`, `roundtrip` (parse → write →
parse must give an equal model), and `stream_vs_whole` (chunked parsing
produces the same events as whole-input parsing, for arbitrary chunk splits).
The nightly gate runs each target for a bounded time.

### WebAssembly

The core test suite runs under `wasm32-unknown-unknown` through
`wasm-bindgen-test-runner` (Node, no browser). The corpus is embedded with
`include_str!`, since a wasm test has no file I/O.

### Template gates

All template gates pass: 100% line coverage and zero surviving mutants,
including in the CLI.

## 7. Benchmark strategy

- **Inputs:**
  - Corpus files: small (4.5 KB), medium (1.2 MB) and large (13.6 MB, xcms MTBLS4381).
  - A synthetic generator (seeded, deterministic) produces 100 MB and 1 GB files,
    with a mix of profiles and wide SMF tables (1000 assays).
- **Measured:**
  - Throughput (MB/s) for lex-only, parse to the borrowed model, parse to the
    owned model, write, `validate::structure`, and streaming end to end.
  - Peak heap and allocation count via dhat.
  - Streaming memory on the 1 GB file, which must stay O(line) and not O(file).
- **Comparison:**
  - jmzTab-m is timed with hyperfine on the same files, CLI against CLI, with
    JVM start-up reported separately.
  - In-process JMH would be fairer to the JVM but is out of scope. I will say
    so explicitly.
- **Reporting:** `docs/BENCHMARKS.md` records the machine, versions and raw
  numbers, including where we lose (for example, no SIMD scanning and no
  memchr).

## 8. Milestones and acceptance criteria

Each milestone ends with `./mk ci` green, a short report, and a DECISIONS.md
update. I continue to the next milestone unless I am blocked.

| # | Milestone | Acceptance criteria |
|---|---|---|
| M0 | Scaffold | Template adopted (T1 retracted, see D-031), with T2 (a wasm *test* gate in `mk.local`). License per your T6 decision. An empty core crate is `no_std`, has zero deps, and passes every gate on CI, including the wasm test gate. Corpus vendored with a NOTICE (CC-BY-4.0 / Apache-2.0). |
| M1 | Model + diagnostics types | All §3 types documented, with doctests. `Code` registry skeleton in place, with the spec-anchor test. `Param` and `Decimal` grammars complete, with unit and property tests. |
| M2 | Reader (TSV) | Lexer, `StreamReader`, and model builder. All 21 TSV corpus files parse without panic, and every syntax-layer finding is located. `stream_vs_whole` property passes. Fuzz `parse_bytes` runs 30 minutes clean. |
| M3 | Writer | Canonical writer plus `SourceLayout`. Round-trip properties pass. All corpus files round-trip byte-exact, except documented whitespace cases. |
| M4 | Structure validator + oracle | MTD, table, profile, cross-reference and type rules, with the version-dispatched rule set. Oracle harness in place, and every disagreement explained. Negative suite covers every `T` code. |
| M5 | Semantic validator + CV | CV declaration/usage and accession rules; CV-mapping rules generated from `mzTab_2_1-M_mapping.xml` (generator committed; CI checks freshness); `TermLookup` trait; `mztab-m-obo`. No network I/O anywhere. |
| M6 | JSON (feature `json`) | Read and write JSCH-shaped JSON. JSON Pointer locations. TSV ↔ JSON agreement on the corpus. MetaboScape dialect rejected with a structured error. **[default: D-004]** |
| M7 | CLI + WASM demo | `mztab-m validate/fmt/convert/inspect`, with human and JSON output and exit codes. WASM demo: a static page that validates a dropped file in the browser. |
| M8 | Benchmarks | §7 complete, and `docs/BENCHMARKS.md` written. |
| M9 | Docs + release prep | Crate quickstart; every rule's rustdoc links to its spec section; `DESIGN.md`; README states and links the CI gates enforcing zero dependencies, purity, and WASM. Dry-run publish only. |

## 9. Still needed from you

The following remain open. Phase 2 starts on the defaults unless you say
otherwise.

- **T6: license.** This blocks M0. Recommendation: `MIT OR Apache-2.0`.
- **D-002: precedence.** Prose > field reference > JSCH.
- **D-010 / D-011 / D-012 / D-013 / D-014 / D-015:** field-level conflicts, with
  defaults as listed in DECISIONS.md.
- **D-004: JSON.** JSCH dialect only, hand-rolled, behind a feature flag.
- **D-030: JVM.** The oracle runs in CI only; outputs are committed. That means
  a GitHub Actions job installing Temurin 25 and downloading the jmzTab-m JAR.
- **Dependencies:** `proptest` (dev), `wasm-bindgen` (demo crate only),
  `criterion` + `dhat` (bench crate only).
- **MSRV:** 1.100.0, which equals the pinned toolchain (T5).
