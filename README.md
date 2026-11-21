# mztab-m-rs

Read, write, validate and convert [mzTab-M](https://github.com/HUPO-PSI/mzTab-M)
files: the HUPO-PSI / MSI exchange format for mass-spectrometry-based
metabolomics results.

> **Status: early development. Not production grade yet.**
> The library and the command work on every example file of the
> specification, but the target spec (mzTab-M 2.1.0-M) is itself an
> unreleased draft, nothing is published, and many of the choices about how
> to read that draft have not been reviewed outside this project. Expect
> breaking changes. Feedback is very welcome, especially from people who
> produce or consume mzTab-M files. [STATUS.md](STATUS.md) says exactly
> what works and how it was checked.

## What and why

The reference implementation, [jmzTab-m](https://github.com/lifs-tools/jmzTab-m),
is a Java library. This project is a small, dependency-free implementation
in Rust that can be embedded anywhere: a command-line tool, Python and R
packages, and the browser (WebAssembly). It never stops at the first problem:
a broken file still gives you a document and every finding, each with a
stable code, a line and column, and a link to the sentence of the spec that
states the rule. Writing a file you read gives back the same bytes.

It targets mzTab-M **2.1.0-M** at HUPO-PSI/mzTab-M commit `e66c19e` and
also reads and validates **2.0.0-M** files.

## Quickstart

You need Rust (the pinned version installs itself through
`rust-toolchain.toml` if you use rustup).

```sh
git clone --recurse-submodules https://github.com/Adafede/mztab-m-rs   # TODO(human): confirm the final URL
cd mztab-m-rs
cargo install --path crates/mztab-m-cli --locked
mztab-m validate third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
```

```text
0 errors, 11 warnings, 2 infos
warning[MZT-S013] line 65, column 67, cell 3: leading or trailing spaces
warning[MZT-S013] line 72, column 67, cell 3: leading or trailing spaces
...
warning[MZT-X001] line 110, column 17, cell 3: CV label is not declared: `MIRIAM`
```

More of what the command does:

```sh
mztab-m validate --cv psi-ms.obo study.mztab      # also check terms against PSI-MS
mztab-m validate --format json study.mztab        # findings as JSON
mztab-m fmt --check study.mztab                   # is it in canonical form?
mztab-m convert --to json -o study.json study.mztab
mztab-m inspect study.mztab                       # a short summary
```

`mztab-m --help` lists every option. The command never downloads anything:
vocabularies are only read from the OBO files you pass with `--cv` (for
example `psi-ms.obo` from [HUPO-PSI/psi-ms-CV](https://github.com/HUPO-PSI/psi-ms-CV)).

From Rust, see the [core crate's README](crates/mztab-m/README.md). From
Python or R, see [bindings/python](bindings/python) and
[bindings/r](bindings/r); both are prototypes and build from source only.

## Project layout

| Path | What it is |
|---|---|
| [`crates/mztab-m`](crates/mztab-m) | The core: model, reader (whole or streaming), writer, validator, JSON (feature `json`). `no_std`, no dependencies. |
| [`crates/mztab-m-obo`](crates/mztab-m-obo) | Reads OBO vocabularies such as `psi-ms.obo` for term checks. |
| [`crates/mztab-m-cli`](crates/mztab-m-cli) | The `mztab-m` command. |
| [`crates/mztab-m-api`](crates/mztab-m-api) | Internal layer shared by the command and the bindings. |
| [`crates/mztab-m-wasm`](crates/mztab-m-wasm) | A browser page that validates a dropped file (demo). |
| [`bindings/`](bindings) | Python (PyO3) and R (extendr) packages. |
| [`benches/`](benches), [`fuzz/`](fuzz) | Benchmarks and fuzz targets. |
| [`third_party/`](third_party) | The pinned spec and jmzTab-m, as git submodules (test data). |
| [`docs/`](docs) | [Design](docs/DESIGN.md), [benchmarks](docs/BENCHMARKS.md), [lint exceptions](docs/DEBT.md). |
| [`DECISIONS.md`](DECISIONS.md) | Every place the spec is ambiguous or this project differs from jmzTab-m, and what it does. |

## Getting help

Open an issue for bugs, questions about a finding, or disagreement with a
decision in [DECISIONS.md](DECISIONS.md).
`TODO(human): link the issue tracker once the repository is public.`
Security problems go through [SECURITY.md](SECURITY.md).

## Contributing

Contributions are welcome. The most useful ones right now are real-world
mzTab-M files that this project reads differently from jmzTab-m or from what
you expected, and reviews of the decisions in DECISIONS.md.
[CONTRIBUTING.md](CONTRIBUTING.md) explains the setup and the checks.

## How this project is developed

Most of the code and documents were written with AI coding agents, directed
by the maintainer. The checks described in STATUS.md were actually run, and
the documents an agent wrote for itself carry a provenance note.
Instructions for those agents are in
[AGENTS.md](AGENTS.md) and [docs/agents/](docs/agents/). You do not need
to read them to use or contribute to the project.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. The submodules under
`third_party/` keep their own licenses (CC-BY-4.0 for the specification,
Apache-2.0 for jmzTab-m); see [NOTICE](NOTICE).

## Citation and acknowledgements

`TODO(human): add a CITATION.cff (authors, ORCID, and a DOI once released)
and any funding or institutional acknowledgement.`

The rules come from the mzTab-M specification by the HUPO Proteomics
Standards Initiative and the Metabolomics Standards Initiative. jmzTab-m by
Nils Hoffmann and contributors served as the behavioural reference.
