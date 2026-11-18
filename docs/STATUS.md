# Status and what is left

Last updated: 2026-11-19, after M9. Milestones are defined in
[PLAN.md §8](PLAN.md); decisions are in [DECISIONS.md](../DECISIONS.md).

## Done

| # | Milestone | Commit | Notes |
|---|---|---|---|
| M0 | Scaffold | `d35566b` | |
| M1 | Model and finding registry | `ba8cbed` | D-050–D-055 |
| M2 | TSV reader, streaming | `5b7d577` | D-056–D-059 |
| M3 | Writer, byte-exact layout | `2f332cc` | D-060–D-063 |
| M4 | Structure validator, oracle harness | `95bd5c4` | D-064–D-066 |
| M5 | Semantic validator, CV rules, `mztab-m-obo` | `b969cec` | D-067–D-070 |
| M6 | JSON (feature `json`) | `c5b7c29` | D-071 |
| M7 | CLI (`mztab-m`) and browser demo | `8b5439c` | D-072 |
| M8 | Benchmarks ([BENCHMARKS.md](BENCHMARKS.md)) | `570c987` | D-073 |
| M9 | Docs and release preparation | (pending commit) | D-074 |

All gates pass at each commit: 100% line coverage, no surviving mutants
in the milestone's diff, and wasm32 tests for `mztab-m` and `mztab-m-obo`.

## Left

### Milestones

All milestones are done. **Publishing is not**: `cargo publish --dry-run`
passes for `mztab-m`, `mztab-m-obo` and `mztab-m-cli` (D-074), and the
real publish waits for approval.

### Open items carried from earlier milestones

- **Oracle outputs (M4, D-066).** No JVM is available locally. The
  `oracle.yml` workflow has to run in CI, its outputs have to be committed,
  and every disagreement has to be reviewed into
  `tests/oracle/expected-disagreements.tsv`. Until then the comparison
  passes vacuously.
- **jmzTab-m comparison and oracle outputs.** A JDK 25 is installed
  (`~/.jenv/versions/25.0.4`), so both could run locally, but building the
  jmzTab-m CLI downloads Maven and its dependencies: needs approval.
- **Real vocabularies (M5).** The semantic layer has only been run with the
  test stand-in (`tests/support/terms.rs`). A run of the corpus against the
  real `psi-ms.obo` (and UO and others) needs those files downloaded, which
  has not been done because downloads need approval.
- **Not checked yet:**
  - URIs (RFC 3986) and emails (D-065);
  - `spectra_ref` against the run's `id_format` (spec table-1);
  - CV term units (`has_units`) and value types (D-067).
- **JSON comments (D-071).** `COM` lines are neither written to nor read
  from JSON.
- **Lossy conversion (D-071, D-072).** `mztab-m convert --to json` refuses
  what JSON cannot carry; there is no option to drop it instead.
- **Demo hosting.** `./mk local:demo` builds the page; it is not
  published anywhere (that would need approval).

### Local environment notes (not in CI)

- Source `target/env.sh` before `./mk`. It points `CARGO_HOME` and `PATH` at
  `target/`; `target/` was wiped once mid-M4, and the tools were
  reinstalled.
- Run `cargo fuzz` with `--target aarch64-apple-darwin`, because the
  installed cargo-fuzz is an x86_64 build.
- The `mutants-diff` gate is run by hand
  (`cargo mutants --in-diff … --jobs 2 --timeout 240`). Inside the sandbox
  its `mktemp` and memory guard misbehave.
