# Design

How `mztab-m` is built and why. The decisions behind each point are in
[DECISIONS.md](../DECISIONS.md) (referred to as D-nnn); the plan they came
from is [PLAN.md](agents/PLAN.md).

## Constraints

- **The specification is normative.** The HUPO-PSI mzTab-M 2.1 draft is
  pinned to one commit (`SPEC_COMMIT`). Where its parts disagree, the order
  of precedence is prose, then the field reference, then the JSON schema
  (D-002). The reference implementation, jmzTab-m, is a behavioural oracle
  only. Each deviation from it is recorded (the J- entries), and it is
  never copied (J-17).
- **The core is pure** (`mztab-m`):
  - `#![no_std]` + `alloc`, zero dependencies, no `unsafe`;
  - no I/O, no global state, no interior mutability, no clocks.
  - Everything that touches the outside world is a thin adapter: the CLI,
    the WASM demo, the OBO reader you feed text to.
- **It never fails and never panics on input.** Every function that reads
  returns a document and its findings. Panics are linted out
  (`unwrap_used`, `panic`, `indexing_slicing`) and fuzzed for.

## Crates

```
mztab-m        core: model, reader, writer, validator, JSON (feature)
mztab-m-obo    OBO 1.4 text -> TermLookup (no_std, zero-copy)
mztab-m-cli    the `mztab-m` command: files and streams around the core
mztab-m-wasm   browser demo (not published)
benches        generator and benchmarks (not published)
fuzz           cargo-fuzz targets (outside the workspace)
```

## The model

- **`MzTab`** holds the metadata (one typed field per key), the three
  tables, and a `SourceLayout`.
- **Every value read from a file is a `Cell<T>`:** `Value(T)`, `Null`, or
  `Invalid(Raw)`. An invalid cell keeps its text, so a broken document is
  still fully represented, validation can report on it, and writing can
  reproduce it. A well-formed model has no `Invalid` cells. That is checked
  by the structure layer, not by the type system (PLAN §3).
- **Lexemes are kept:** `Decimal` keeps `1.50` as written, so writing is
  exact (D-052).
- **Ids are typed** (`AssayId`, `MsRunId`, ...), one-based and never zero.
  An `IndexMap` keeps them sorted.
- **The model borrows from the input text.** `parse(&str)` copies nothing
  that needs no unescaping; `IntoOwned` makes a `'static` copy when needed.

## Reading

- **One line at a time.** The reader splits a line into cells, recognises
  the prefix (`MTD`, `SMH`, `SML`, ...), and types each cell with the
  grammar of its key or column. Everything is located: line, column, and
  cell.
- **`StreamReader`** takes chunks of any size and keeps only the
  incomplete last line. Streaming about 1 GB peaks at 0.87 MB of heap
  (BENCHMARKS.md).
- **`parse`** is the stream fed one whole text. A property test checks that
  any document, cut anywhere, streams to the same result.
- **Findings, not errors:**
  - A `Finding` has a `Code` (`MZT-S…` syntax, `MZT-T…` structure,
    `MZT-X…` semantic), a severity, a location and a structured `Detail`.
  - Each code knows its RFC 2119 level, the spec section that states its
    rule, and the equivalent jmzTab-m codes.
  - A test checks every spec anchor against the pinned text.

## Writing

- **Two modes, chosen by the layout** (D-060–D-063):
  - A model read from text records every line in its `SourceLayout`
    (order, line endings, comments, blank lines, and the exact text of any
    line that is not canonical). Writing it back reproduces the input byte
    for byte; only edited items are written canonically.
  - A model built in code has an empty layout and is written in canonical
    form: spec order, `\n` endings.
- **Refuse, never drop.** The writer refuses what would not read back the
  same, such as raw text containing a tab, with `WriteError::Unwritable`.

## Validation

Three independent layers, each callable on its own (`validate::syntax`,
`structure`, `semantic`, or `all`):

1. **Syntax** is what the reader reports: prefixes, cell grammars, cell
   counts, encodings.
2. **Structure** works on the typed model:
   - mandatory fields;
   - profiles (2.1);
   - index gaps;
   - references between elements and tables;
   - column sets and order;
   - nullability and value rules.
   The rule set is chosen by the model's `mzTab-version` (D-064, D-065).
3. **Semantic** covers controlled-vocabulary use:
   - declared and used CVs;
   - accession form;
   - terms (unknown, renamed, obsolete);
   - the 32 CV-mapping rules.
   The rules are **generated** from the spec's mapping file by a test that
   fails when the generated file is stale. Corrections are made where the
   spec's prose disagrees (D-067, D-068).

Terms come through the **`TermLookup`** trait. A lookup answers only for
the vocabularies it *covers*, and the validator skips what it cannot judge,
never guessing. `mztab-m-obo` builds lookups from OBO text. The core never
downloads a vocabulary.

Findings on a model read from text are located by line, column and cell,
even when the line was never kept verbatim: the writer renders the line to
find the column. A model built in code gets model paths instead
(`sml[2][14]`). `Config` overrides or switches off any code.

## JSON (feature `json`)

JSON goes **through TSV text** in both directions (D-071):

- The reader carries each JSON value into the TSV line that holds it, runs
  the TSV reader, and relocates its findings to JSON pointers.
- The writer writes canonical TSV and carries each line into JSON.
- So there is one grammar per value. A JSON string where a structured value
  is expected is that value's TSV text; that is how invalid values survive
  the trip.
- What JSON cannot carry is refused: unknown keys, unknown columns, ragged
  rows. The snake_case dialect (MetaboScape) is rejected.
- 10 corpus files read back from JSON unchanged, 2 differ only in column
  order, and 9 are refused with a reason (`tests/golden/json-corpus.txt`).

## Testing

Each milestone ended with all of these passing:

- **Conformance:**
  - every spec example in the corpus (golden finding counts per file);
  - one triggering edit per finding code, plus edits that must *not*
    trigger it (`tests/rules.rs`);
  - a base document that triggers nothing.
- **Round trips:** byte-exact on every corpus file; canonical
  write-then-read; JSON against canonical TSV.
- **Property tests (proptest):**
  - streaming equals a whole read;
  - writing what was read gives back the source;
  - parameter and key grammars.
- **Fuzzing (cargo-fuzz):**
  - `parse_bytes` and `stream_vs_whole` (never panic, and agree);
  - `roundtrip`;
  - `json`, which found three real bugs (D-071).
- **Mutation testing:** zero surviving mutants in each milestone's diff.
  Equivalent mutants were removed by simplifying the code, not by excluding
  them.
- **Coverage:** 100% of lines, enforced. Where a generic function counted
  only its best instantiation, the logic was made non-generic.
- **WASM:** the core, `mztab-m-obo` and the demo build and pass their tests
  on `wasm32-unknown-unknown`.
- **Oracle:** a workflow (`oracle.yml`) is set up to run jmzTab-m over the
  corpus. It has never run, so no outputs are committed; until then the comparison test has
  nothing to compare ([STATUS.md](../STATUS.md)).

## What it costs

Measured in BENCHMARKS.md:

- **Reading:** 120–210 MiB/s, 5–9× slower than a bare line-and-tab scan.
  The time goes to typing every cell and recording the layout.
- **Memory:** the typed, lossless model takes 2.5–7× the text it came
  from. Streaming is the way through files that do not fit.
- **JSON** is 3–8× slower than TSV, because of the detour through TSV text.
