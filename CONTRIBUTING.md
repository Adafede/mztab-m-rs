# Contributing

Thanks for looking. This project is early (see [STATUS.md](STATUS.md)), so
outside eyes are worth a lot, and small contributions are as welcome as big
ones.

## Most useful right now

- **Real mzTab-M files.** Files from tools or repositories (MetaboLights,
  xcms, MS-DIAL, lipid pipelines, …) that this project reads, flags or
  converts in a way you think is wrong. A file plus "I expected X" is a
  complete bug report.
- **Review of [DECISIONS.md](DECISIONS.md).** Many entries are `PROPOSED` or
  `OPEN`: each is this project's reading of an ambiguous spot in the draft
  spec. If you know how the spec is meant to be read, say so in an issue.
- **The jmzTab-m comparison.** `tests/oracle.rs` has no jmzTab-m outputs to
  compare with yet. Running jmzTab-m over the corpus and explaining the
  differences is the single biggest gap.
- **Bindings on other platforms.** The Python and R packages have mainly been
  tested on macOS arm64; Linux CI exists, Windows is untested.
- **Other vocabularies** (UO, OBI, NCBITaxon, …), pinned and checked like
  PSI-MS.

Draft issues for these are in [docs/proposed-issues.md](docs/proposed-issues.md).

### Label plan

- **`good first issue`**: self-contained tasks with clear files and checks
  (for example, items 2, 3, 5, 8, and 10 in [docs/proposed-issues.md](docs/proposed-issues.md)).
- **`help wanted`**: larger or domain-specific tasks where community input or
  platform access helps (such as real-world mzTab-M files, reviewing `DECISIONS.md`,
  running the jmzTab-m oracle, or testing Windows wheels).

## Setup

```sh
git clone --recurse-submodules https://github.com/Adafede/mztab-m-rs
cd mztab-m-rs
./mk setup        # installs the pinned toolchain and tools from mk.versions and mk.local
```

The tests read the pinned specification and jmzTab-m fixtures from the git
submodules in `third_party/`. If you cloned without them:
`git submodule update --init`.

`./mk setup` needs [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)
installed first (`cargo install cargo-binstall`, or see its README).
`./mk help` lists every task.

## Checks

```sh
./mk check        # format, TOML, typos, clippy (seconds)
./mk test-fast    # all workspace tests (nextest)
./mk ci-fast      # what you should run before pushing
./mk ci           # the full pull-request gate, including mutation tests on your diff
./mk site         # build the mdBook docs, WASM demo, API docs, and check links
./mk site-serve   # build and preview the site locally on 127.0.0.1:8000
./mk findings     # regenerate docs/findings.md from Code::ALL
./mk bindings     # build and test the Python and R bindings from source
```

What `./mk ci` enforces, so you are not surprised:

- the core crate (`crates/mztab-m`) has no dependencies and is `no_std`;
- no `unsafe`, and no `unwrap`, `panic!` or unchecked indexing in library
  code;
- no surviving mutants in the lines you changed (`cargo mutants --in-diff`);
- the core, `mztab-m-obo`, `mztab-m-api` and the demo build and pass their
  tests on `wasm32-unknown-unknown`, and the browser demo builds;
- the fuzz targets and benchmarks compile;
- the minimum Rust version (the `rust-version` in `Cargo.toml`);
- dependency licenses and advisories (`cargo deny` and `cargo audit`),
  unused dependencies (`cargo machete`), and rustdoc (`-D warnings`).

The nightly run (`./mk ci-nightly`) adds the 100% line-coverage floor, a
full mutation run, miri, a feature-powerset build, and the pinned PSI-MS
vocabulary check.

The Python and R packages sit outside the Cargo workspace and are run via
`./mk bindings` (and in [`.github/workflows/bindings.yml`](.github/workflows/bindings.yml));
see [AGENTS.md](AGENTS.md#build-and-test) for prerequisites.

Golden files are rewritten with `BLESS=1`; each `tests/golden/` directory
has a README saying which test owns which file.

## Conventions

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:` …). Keep each commit to one concern.
- **Branches and pull requests:** branch from `main`, open a pull request,
  and fill in the template. Say what you ran.
- **Lint exceptions** use `#[expect(lint, reason = "...")]` and get a line in
  [docs/DEBT.md](docs/DEBT.md). `#[allow]` is rejected by the lint gate.
- **Spec questions:** if your change depends on a reading of the spec that
  is not obvious, add or update an entry in [DECISIONS.md](DECISIONS.md) in
  the same pull request.

## AI-assisted contributions

Much of this project was written with AI coding agents, so they are not
unwelcome here. The rules are the same as for any contribution:

- say in the pull request that you used one, and roughly for what;
- you have read and understood every line you submit, and you can answer
  review questions about it;
- the same checks and the same review apply. Generated tests that only
  restate the code, or docs that only restate the function name, will be
  asked to go.

Agents themselves should start from [AGENTS.md](AGENTS.md).

## Conduct and security

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).
Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md),
not in public issues.
