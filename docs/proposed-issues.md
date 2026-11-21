> Provenance: drafted by an AI coding agent from the 2026-11-21 audit, for
> the maintainer to review and file as issues. Delete this file once they
> are filed.

# Proposed issues

Each is a real gap found while checking the repository. The ones marked
**good first issue** need no knowledge of the codebase beyond the files
named.

## 1. Run jmzTab-m over the corpus and commit its outputs

`crates/mztab-m/tests/oracle.rs` compares this project's findings with
jmzTab-m's, but `crates/mztab-m/tests/oracle/` holds no jmzTab-m outputs, so
the test checks nothing. `.github/workflows/oracle.yml` is written but has
never run.

Done when: the outputs are committed, the test compares real data, and
every disagreement is either fixed or listed in `expected-disagreements.tsv`
with a DECISIONS.md entry. Needs a JDK 25 and network access for Maven.

## 2. Pin and check UO the way PSI-MS is checked

Terms from UO (units) are only checked for form. Add a pinned download with
SHA-256 to `mk.local` next to `task_vocab`, run the corpus against it, and
commit a golden file like `crates/mztab-m-api/tests/golden/psi-ms.txt`.
Then the same for OBI and NCBITaxon if the corpus uses them.

## 3. Run `cargo deny` over the bindings' lockfiles — good first issue

`bindings/python` and `bindings/r/src/rust` have their own `Cargo.lock` and
are outside the workspace, so the `deny` gate never sees PyO3, extendr and
their dependencies. Add a step (in `mk.local` or `bindings.yml`) that runs
`cargo deny check` with the repository's `deny.toml` in each. (`bindings/python`
already passes; `bindings/r` currently triggers `extendr-api 0.9.0`'s
transitive duplicate `syn` and unmaintained `paste`, which need a `deny.toml`
exception or upstream update.)

## 4. Build and test the Python wheel on macOS and Windows in CI

`.github/workflows/bindings.yml` builds and tests on Ubuntu only; macOS arm64
has been built locally. Add a matrix over Linux, macOS, and Windows.

## 5. `R CMD check` the R package

Only `R CMD INSTALL` and the testthat suite have been run. `R CMD check
--as-cran` will raise notes (e.g. the Rust sources are not vendored, D-076).
List them and fix what can be fixed without vendoring.

## 6. Re-run the benchmarks with the current generator — good first issue

`docs/BENCHMARKS.md` says the synthetic generator changed after the numbers
were taken. Re-run `cargo bench -p mztab-m-bench --bench throughput` and
update the tables, with the machine and commit. (The heap benchmark takes
about 40 minutes; it can be a separate pass.)

## 7. Collect real-world mzTab-M files beyond the spec examples

All tests use the specification's 23 example files and hand-made fixtures.
Files exported by tools in current use (with licenses that allow
redistribution, or a script that downloads them by pinned URL and hash)
would show how this project behaves on what people actually produce.
