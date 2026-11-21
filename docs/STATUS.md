# Status and what is left

Last updated: 2026-11-21, after the PSI-MS run (D-077). Milestones are defined in
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
| M9 | Docs and release preparation | `9378f48` | D-074 |
| — | Remaining checks, API crate, Python and R packages | `5eb36a0` | D-075, D-076 |
| — | The corpus against PSI-MS 4.2.17 | `4a0af59` | D-077 |

All gates pass at each commit: 100% line coverage, no surviving mutants
in the milestone's diff, and wasm32 tests for `mztab-m` and `mztab-m-obo`.

## Left

### Milestones

All milestones are done. **Publishing is not**: `cargo publish --dry-run`
passes for `mztab-m`, `mztab-m-obo` and `mztab-m-cli` (D-074), and the
real publish waits for approval.

### Open items

- **Publishing** (crates.io, PyPI, CRAN) waits for approval. CRAN would
  also need the Rust sources vendored (D-076).
- **jmzTab-m comparison and oracle outputs.** A JDK 25 is installed
  (`~/.jenv/versions/25.0.4`), so both could run locally, but building the
  jmzTab-m CLI downloads Maven and its dependencies, which needs approval.
- **Other vocabularies.** PSI-MS is checked (`./mk local:vocab`, D-077);
  UO, OBI, NCBITaxon and the rest are not downloaded, so terms from them
  are only checked for form and declaration.
- **Deliberately not done:** CV term units (D-075 explains why mzTab has
  nothing to check them against); the original position of JSON comments
  (D-075).

### Local environment notes (not in CI)

- Source `target/env.sh` before `./mk`. It points `CARGO_HOME` and `PATH` at
  `target/`. `target/` has been wiped twice (mid-M4 and before the
  post-M9 work); `./mk setup` plus tombi, cargo-audit and cargo-fuzz
  reinstall the tools.
- Run `cargo fuzz` with `--target aarch64-apple-darwin`, because the
  installed cargo-fuzz is an x86_64 build.
- The `mutants-diff` gate is run by hand
  (`cargo mutants --in-diff … --jobs 2 --timeout 240`). Inside the sandbox
  its `mktemp` and memory guard misbehave.
