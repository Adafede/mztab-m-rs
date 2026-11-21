# Contributing

Thanks for looking. This project is early (see [STATUS.md](STATUS.md)), so
outside eyes are worth a lot, and small contributions are as welcome as big
ones.

## Most useful right now

- **Real mzTab-M files.** Files from tools or repositories (MetaboLights,
  xcms, MS-DIAL, lipid pipelines, …) that this project reads, flags or
  converts in a way you think is wrong. A file plus "I expected X" is a
  complete bug report.
- **Review of [DECISIONS.md](DECISIONS.md).** 37 entries are `PROPOSED` and
  9 `OPEN`: each is this project's reading of an ambiguous spot in the draft
  spec. If you know how the spec is meant to be read, say so in an issue.
- **The jmzTab-m comparison.** `tests/oracle.rs` has no jmzTab-m outputs to
  compare with yet. Running jmzTab-m over the corpus and explaining the
  differences is the single biggest gap.
- **Bindings on other platforms.** The Python and R packages have only been
  built on macOS arm64.
- **Other vocabularies** (UO, OBI, NCBITaxon, …), pinned and checked like
  PSI-MS.

Draft issues for these are in [docs/proposed-issues.md](docs/proposed-issues.md).

## Setup

```sh
git clone --recurse-submodules <repository URL>
cd mztab-m-rs
./mk setup        # installs the pinned toolchain and tools from mk.versions
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
./mk test-fast    # all workspace tests
./mk ci-fast      # what you should run before pushing
./mk ci           # the full pull-request gate, including mutation tests on your diff
```

What `./mk ci` enforces, so you are not surprised:

- the core crate (`crates/mztab-m`) has no dependencies and is `no_std`;
- no `unsafe`, and no `unwrap`, `panic!` or unchecked indexing in library
  code;
- no surviving mutants in the lines you changed (`cargo mutants --in-diff`);
- the core, `mztab-m-obo`, `mztab-m-api` and the demo build and pass their
  tests on `wasm32-unknown-unknown`;
- the minimum Rust version (the `rust-version` in `Cargo.toml`);
- dependency licenses and advisories (`cargo deny`).

The nightly run (`./mk ci-nightly`) adds the 100% line-coverage floor, a
full mutation run, miri and a feature-powerset build.

The Python and R packages are built and tested outside `./mk`; the commands
are in [AGENTS.md](AGENTS.md#build-and-test) and in
[`.github/workflows/bindings.yml`](.github/workflows/bindings.yml).

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
  is not obvious, add or update an entry in DECISIONS.md in the same pull
  request.

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
