# Contributing

- Run `./mk ci` before pushing; it runs every blocking gate (format, lints,
  tests, docs, licenses, MSRV, wasm build and tests, duplication, mutants on
  the diff). `./mk help` lists the gates.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org/).
- Lint exceptions use `#[expect(lint, reason = "...")]` and are recorded in
  [docs/DEBT.md](docs/DEBT.md); `#[allow]` is denied.
- Every spec ambiguity or deviation from jmzTab-m goes in
  [DECISIONS.md](DECISIONS.md) before code depends on it.
- Clone with `--recurse-submodules`: tests read the pinned spec corpora in
  `third_party/`.
- MSRV: the `rust-version` in the root `Cargo.toml` (checked by `./mk msrv`).
