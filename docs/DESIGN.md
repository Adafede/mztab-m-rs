# Design

How `mztab-m-rs` is structured and why. The decisions behind each point are in
[DECISIONS.md](../DECISIONS.md) (referred to as D-nnn); the original
implementation plan is [PLAN.md](agents/PLAN.md).

## Constraints

- **The specification is normative.** The HUPO-PSI mzTab-M 2.1 draft is
  pinned to one commit (`SPEC_COMMIT`). Where its parts disagree, the order
  of precedence is prose, then the field reference, then the JSON schema
  (D-002). The reference implementation, jmzTab-m, is a behavioural oracle
  only. Each deviation from it is recorded (the J- entries), and it is
  never copied (J-17).
- **The core is pure (`mztab-m`):**
  - `#![no_std]` + `alloc`, zero runtime dependencies, zero `unsafe`;
  - no I/O, no global state, no interior mutability, no clocks.
  - Everything touching the outside world is a thin adapter: the CLI,
    the WASM demo, Python/R bindings, and the OBO reader fed with text.
- **Inputs never fail or panic.** Every reader returns a document and its
  findings. Panics are denied at compile time (`unwrap_used`, `panic`,
  `indexing_slicing`) and verified with `cargo-fuzz`.

## Workspace architecture

Every workspace dependency arrow is strictly one-way (acyclic):

```text
Layer 3: Delivery Adapters (I/O, CLI, JS/Python/R FFI)
  mztab-m-cli   mztab-m-wasm   bindings/python   bindings/r
       \             |                |              /
        v            v                v             v
Layer 2: Application Facade (no_std + alloc, text/bytes in -> text/records out)
                       mztab-m-api
                      /           \
                     v             v
Layer 1: Vocabulary Adapter        |
       mztab-m-obo ----------------+
                     \             |
                      v            v
Layer 0: Domain Core (no_std + alloc, zero external dependencies, zero I/O)
                         mztab-m
```

- **Strict one-way rule:** Layer 3 adapters depend only on `mztab-m-api` (plus
  their runtime/FFI crates: `wasm-bindgen`, `pyo3`, `extendr-api`, or `std` in
  `mztab-m-cli`). They never import `mztab-m` or `mztab-m-obo` directly.
- **Core purity invariant:** `mztab-m` remains `#![no_std]` + `alloc` with
  zero runtime dependencies, zero I/O, zero `unsafe`, and zero panics on
  arbitrary input.

### Crates and packages

| Crate / Package | Responsibility | Public Surface |
|---|---|---|
| `mztab-m` | Pure mzTab-M domain model, lossless TSV/JSON codecs, and three-layer validator | Curated flat re-exports at crate root + `cv`, `validate`, `json` submodules; internal helpers `pub(crate)` |
| `mztab-m-obo` | Zero-copy OBO 1.4 text parser implementing `mztab_m::cv::TermLookup` | `Ontology` struct + `TermLookup` impl |
| `mztab-m-api` | Stateless application operations shared by CLI, WASM, and bindings (not published; D-074) | `read`, `validate`, `write`, `convert`, `facts`, `facts_json`, `codes`, `vocabulary_size`, `Format`, `Layer`, `Settings`, `Record`, `CodeRecord`, `Fact`, `Error` |
| `mztab-m-cli` | CLI parsing (`Command`, `Action`), `Io` abstraction, and `std` process entrypoint | `run`, `Io`, exit codes (`OK`, `FINDINGS`, `USAGE`, `FAILED`) |
| `mztab-m-wasm` | `wasm-bindgen` entrypoints for the browser validator demo (not published) | `validate`, `convert` |
| `benches/` | Criterion throughput and `dhat` heap benchmarks (not published) | — |
| `bindings/python` | PyO3 native module exposing `mztab-m-api` to Python (outside workspace) | `validate`, `convert`, `canonical`, `inspect`, `codes`, `vocabulary_size` |
| `bindings/r` | `extendr` native bindings exposing `mztab-m-api` to R (outside workspace) | `mztabm_*` R wrappers |
| `fuzz/` | `cargo-fuzz` targets (outside workspace) | — |

### Internal DAG of `mztab-m`

Modules inside `crates/mztab-m/src/` are organized into four strict acyclic
tiers:

1. **Tier 1 — Primitives** (`core` + `alloc` only):
   - `id`: 1-based non-zero typed element IDs, `Ix`, `parse_index`, and sorted
     `IndexMap`.
   - `value` (`cell`, `text`, `decimal`): `Cell<T>`, `Raw`, `Text`, `Decimal`,
     and lexical errors.
   - `param` (`text`, `parse`): `Param`, `ParamText`, `Accession`, and located
     parameter parser.
   - `vocab` (`version`, `profile`, `section`, `field`): Closed vocabularies
     (`Version`, `RuleSet`, `Profile`, `Section`, internal `TableSection`,
     `AmbiguityCode`, `Datatype`, `PublicationItem`).
2. **Tier 2 — Domain Model & Diagnostics** (acyclic: `model -> meta`, Tier 2 -> Tier 1 only):
   - `meta` (`subkey`, `key`): Metadata key grammar (`MtdKey`, `*Key`,
     `ValueKind`) with zero dependency on `model`.
   - `model` (`document`, `layout`, `metadata`, `colunit`, `table`, `rows`,
     `meta_value`, `meta_insert`, `meta_entries`): `MzTab`, `SourceLayout`,
     `Metadata`, `MtdValue`/`ValueRef`, `ColUnit`, `SpectraRef`, and table
     rows/columns.
   - `diag` (`severity`, `code`, `location`, `finding`): `Severity`,
     `Requirement`, `Layer`, `SpecRef`, `Code` registry, `Path`/`Location`, and
     `Finding`/`Findings`.
   - `cv` (`rule`, `rules`): `Term`, `TermLookup`, `NoTerms`, and generated
     CV-mapping `Rule` definitions.
3. **Tier 3 — Serialization, Ownership & Event Streaming** (Tiers 1–2):
   - `tsv` (`splitter`, `typing`, `mtd`, `table`): List `Splitter`, raw cell
     typing, metadata value typing, and table header/row slot typing (zero
     dependency on `write`).
   - `write` (`error`, `sink`, `render`, `line`, `pending`, `layout`):
     `WriteError`, fingerprinting/checking `Out`, `Render`/`RenderRow`, and
     canonical & layout-preserving TSV emission.
   - `read` (`event`, `metadata`, `typer`, `builder`, `stream`): `Event`,
     `Parsed`, whole-input reader, and push-based `StreamReader`.
   - `owned`: `IntoOwned` trait detaching borrowed models and events to
     `'static`.
   - `json` (`schema`, `schema_tables`, `value`, `lexer`, `read/*`, `write/*`,
     `findings`): Feature-gated (`json`) RFC 8259 parser/writer, streaming JSON
     emitter, and JSON findings serializer.
4. **Tier 4 — Validation Rules** (Tiers 1–3):
   - `validate` (`report`, `patterns`, `metadata/*`, `tables/*`, `semantic/*`):
     `Config`, `Report` locator, shared format predicates (`is_orcid`, `is_uri`,
     `is_email`, `is_adduct`, `native_id_ok`, `xsd_matches`), and independent
     syntax/structure/semantic layers.

## Glossary

Canonical vocabulary enforced across code, CLI help, docs, and bindings:

- **Finding / `Finding`:** A diagnostic produced by reading or validating a
  document, carrying a `Code`, `Severity`, `Layer`, `Location`, and `Detail`.
  Unrecoverable output constraints during serialization instead return `Error`
  (`WriteError`).
- **`Code`:** Stable identifier (`MZT-S…` syntax, `MZT-T…` structure, `MZT-X…`
  semantic) carrying its RFC 2119 requirement level, spec anchor, and oracle
  codes.
- **`Layer`:** Validation stage (`Syntax`, `Structure`, `Semantic`). Shared
  between `mztab-m` and `mztab-m-api`.
- **`Severity`:** Diagnostic level (`Info`, `Warning`, `Error`).
- **`Format`:** Document wire encoding (`Tsv`, `Json`).
- **`Profile`:** Document study mode (`Summary`, `Complete`, plus tolerated
  `2.0` legacy tokens mapped internally).
- **`Section` vs `TableSection`:** `Section` spans all four mzTab-M sections
  (`Mtd`, `Sml`, `Smf`, `Sme`). Internal `TableSection` narrows table-only code
  paths to `{Sml, Smf, Sme}`, making metadata invalid by construction.
- **`Ontology` / `TermLookup`:** `Ontology` (`mztab-m-obo`) parses OBO 1.4
  text into an in-memory vocabulary implementing `mztab_m::cv::TermLookup`.
- **Parsing convention:** Owned/`Copy` enums and keys (`Version`, `Profile`,
  `AmbiguityCode`, `Datatype`, `MtdKey`, `Severity`, `Layer`, `Code`, `Format`)
  implement `core::str::FromStr`. Zero-copy borrowed parsers (`mztab_m::parse`,
  `mztab_m::json::parse`, `Ontology::parse`, `Param::parse`,
  `Accession::parse`) expose `parse(&str)` functions or associated methods.

## The model

- **`MzTab`** holds the metadata (one typed field per key), the three
  tables, and a `SourceLayout`.
- **Every value read from a file is a `Cell<T>`:** `Value(T)`, `Null`, or
  `Invalid(Raw)`. An invalid cell keeps its text, so a broken document is
  still fully represented, validation can report on it, and writing can
  reproduce it. A well-formed model has no `Invalid` cells, checked by the
  structure validation layer rather than the type system.
- **Lexemes are preserved:** `Decimal` keeps `1.50` as written, so writing is
  exact (D-021, D-060).
- **IDs are typed** (`AssayId`, `MsRunId`, ...), one-based and never zero.
  An `IndexMap` keeps them sorted.
- **Models borrow from input text:** `parse(&str)` copies nothing that needs
  no unescaping; `IntoOwned` makes a `'static` copy when needed.

## Reading

- **One line at a time:** The reader splits a line into cells, recognises
  the prefix (`MTD`, `SMH`, `SML`, ...), and types each cell with the
  grammar of its key or column. Everything is located: line, column, and
  cell.
- **`StreamReader`** takes chunks of any size and keeps only the
  incomplete last line. Streaming about 1 GB peaks at 0.87 MB of heap
  (BENCHMARKS.md).
- **`parse`** feeds one whole text to the stream. A property test checks that
  any document, cut anywhere, streams to the same result.
- **Findings, not hard failures:** Syntax and schema issues become
  `Finding` values tied to a `Code` (`MZT-S…`, `MZT-T…`, `MZT-X…`), severity,
  location, and structured `Detail`. Each spec anchor is verified by a test
  against the pinned spec text.

## Writing

- **Two modes, chosen by the layout** (D-060–D-063):
  - A model read from text records every line in its `SourceLayout`
    (order, line endings, comments, blank lines, and the exact text of any
    non-canonical line). Writing reproduces the input byte for byte; only
    edited items are written canonically.
  - A model built in code has an empty layout and writes in canonical form:
    spec order, `\n` endings.
- **Refuse, never drop:** The writer refuses values that cannot round-trip
  (such as raw text containing a tab) with `WriteError::Unwritable`.

## Validation

Three independent layers, each callable on its own (`validate::syntax`,
`structure`, `semantic`, or `all`):

1. **Syntax** reports reader diagnostics: prefixes, cell grammars, cell
   counts, encodings.
2. **Structure** inspects the typed model: mandatory fields, profiles (2.1),
   index gaps, cross-element/table references, column sets and order, and
   nullability/value rules. The rule set follows `mzTab-version` (D-064,
   D-065).
3. **Semantic** checks controlled vocabularies: declared vs used CVs,
   accession syntax, term existence/renames/obsolescence, and the 32
   CV-mapping rules generated from the pinned spec mapping file (verified by a
   test against drift and prose overrides D-067, D-068).

Terms enter through **`TermLookup`**. A lookup answers only for vocabularies
it covers, and the validator skips uncovered CVs without guessing.
`mztab-m-obo` constructs lookups from OBO text; the core performs zero I/O.

Findings on parsed documents carry line, column, and cell coordinates; findings
on programmatically constructed models carry structural `Path` values
(`sml[2][14]`). `Config` overrides severities or disables individual codes.

## JSON (feature `json`)

JSON routes **through TSV text** in both directions (D-071):

- Reading lowers each JSON value into its TSV cell grammar, feeds the TSV
  reader, and relocates findings to JSON pointers.
- Writing emits canonical TSV and maps each line into JSON.
- A JSON string where a structured value is expected carries that value's TSV
  text so invalid values survive the round trip.
- Unsupported constructs (unknown keys, unknown columns, ragged rows, and
  the MetaboScape snake_case dialect) are refused.
- 10 corpus files round-trip unchanged through JSON, 2 differ only in column
  order, and 9 are refused with reasons (`tests/golden/json-corpus.txt`).

## Error strategy & concurrency

- `mztab-m` and `mztab-m-api` are zero-dependency `no_std`, implementing
  `core::fmt::Display` and `core::error::Error` directly without proc-macro
  crates.
- Readers collect `Finding`s instead of aborting on malformed input;
  serialization constraints return `WriteError` / `mztab_m_api::Error`.
- `mztab-m-cli` maps usage errors (`USAGE`, exit `2`) and I/O or write failures
  (`FAILED`, exit `3`) via typed `Failure` variants.
- All crates are synchronous and single-threaded over borrowed slices
  (`&str` / `&[u8]`) with no async runtime, clocks, or shared mutable state.
  Bounded-memory streaming uses the push callback `StreamReader`.

## Verification

- **Conformance:** Every spec example in the corpus (golden finding counts),
  positive and negative trigger cases per `Code` (`tests/rules.rs`), and a
  clean base document.
- **Round trips:** Byte-exact corpus round-trips, canonical write-then-read,
  and JSON vs canonical TSV.
- **Property tests (`proptest`):** Chunked stream equivalence, read-then-write
  fidelity, and parameter/key grammars.
- **Fuzzing (`cargo-fuzz`):** `parse_bytes`, `stream_vs_whole`, `roundtrip`,
  and `json` (D-071).
- **Mutation testing & coverage:** Zero surviving mutants in `mutants-diff`,
  and 100% line coverage enforced in CI (`mk.local`).
- **WASM:** `mztab-m`, `mztab-m-obo`, `mztab-m-api`, and `mztab-m-wasm`
  compile and pass tests on `wasm32-unknown-unknown`.
- **Oracle:** `oracle.yml` compares behaviour against jmzTab-m once oracle
  outputs are generated ([STATUS.md](../STATUS.md)).

## What it costs

Measured in `BENCHMARKS.md`:

- **Reading:** 120–210 MiB/s, 5–9× slower than a bare line-and-tab scan due to
  per-cell typing and layout capture.
- **Memory:** Lossless typed models use 2.5–7× input size; `StreamReader`
  processes ~1 GB at 0.87 MB peak heap.
- **JSON:** 3–8× slower than TSV because of the canonical TSV intermediate
  representation.
