# AGENTS.md

Entry point for AI coding agents working in this repository. Humans do not
need to read this; start with [README.md](README.md).

This file links to the human docs instead of repeating them. If something
here disagrees with a linked doc, the linked doc wins; fix this file.

## Read first

- [README.md](README.md): what the project is, crate layout.
- [STATUS.md](STATUS.md): what works, what is verified, known issues.
  Keep it truthful when you change behaviour or verify something new.
- [CONTRIBUTING.md](CONTRIBUTING.md): setup, gates, commit style, the rules
  for lint exceptions and spec decisions. They apply to agents too.
- [docs/DESIGN.md](docs/DESIGN.md): architecture and its constraints.
- [DECISIONS.md](DECISIONS.md): every spec ambiguity and deviation from
  jmzTab-m. Record a new one here before code depends on it.

## Build and test

Clone with submodules; tests read the pinned spec in `third_party/`.
**In a sandbox** (read-only `~/.cargo` or `~/.rustup`, filtered network),
read [docs/agents/sandbox.md](docs/agents/sandbox.md) before `./mk setup`.

```sh
git submodule update --init      # if cloned without --recurse-submodules
./mk setup                       # pinned toolchain and tools (mk.versions)
./mk check [-p <crate>]          # fast: fmt, toml, typos, clippy (optionally scoped)
./mk check-changed               # inner loop scoped to affected packages + dependents
./mk test-fast [-p <crate>]      # all workspace tests (nextest, optionally scoped)
./mk test-changed                # workspace tests scoped to affected packages
./mk mutants-diff [--iterate]    # mutation testing on branch Rust diff
./mk mutants-affected [base]     # mutation testing on affected packages + dependents
./mk mutants-plan [--mode diff|affected|full] # JSON mutant/shard plan without mutating
./mk ci-fast                     # pre-push gates, skips mutants-diff and the network audit
./mk ci-core                     # blocking PR gates excluding mutants-diff
./mk ci                          # full PR gate, includes mutants on the diff, demo, fuzz/bench check
./mk bindings                    # build and test Python and R bindings (outside the workspace)
./mk findings                    # regenerate docs/findings.md from Code::ALL
./mk site                        # build mdBook docs + WASM demo + API docs and check links
./mk site-serve                  # build and preview the site locally
```

`./mk bindings` (or `./mk bindings python` / `./mk bindings r`) runs the Python and R builds and tests; to run them manually:

```sh
# Python (from repo root; needs maturin 1.16 in a venv, or use uv sync)
uv pip install .   # or: uv sync
maturin build --release --out target/wheels
pip install target/wheels/*.whl && python -m unittest discover -s bindings/python/tests -v

# R (direct checkout or self-contained tarball via bootstrap.R / devtools)
R CMD INSTALL --no-docs --library=target/rlib bindings/r
R_LIBS=target/rlib Rscript -e 'library(testthat); library(mztabm); test_dir("bindings/r/tests/testthat", stop_on_failure = TRUE)'
```

Golden files are rewritten with `BLESS=1`; see the README in each
`tests/golden/` directory.

## Hard rules

- The core crate `mztab-m` has **zero dependencies**, is `#![no_std]` +
  `alloc`, and does no I/O. Do not add a dependency to it.
- No `unsafe`. No `#[allow]`; use `#[expect(lint, reason = "...")]` and
  record it in [docs/DEBT.md](docs/DEBT.md).
- No panics on input: `unwrap_used`, `panic`, `indexing_slicing` are denied.
- 100% line coverage is a ratchet; do not lower `COVERAGE_FAIL_UNDER_LINES`
  in `mk.local`.
- `mk` and `mk.versions` come from the rusti-graben template. Put
  repository-specific gates in `mk.local`.
- Conventional Commits. Commits are GPG-signed by the user's config.
- Never publish (crates.io, PyPI, CRAN), push, or create remotes without
  the maintainer's explicit approval.
- Do not change a DECISIONS.md status to `DECIDED`; only the maintainer does.

## Agent-only material

[docs/agents/](docs/agents/) holds what agents wrote for themselves or for
the maintainer: the original plan, recon reports, the milestone log, and
[sandbox.md](docs/agents/sandbox.md) for sandboxed sessions.

Every AI-written document that stays in the repository starts with a
one-line `> Provenance:` note. Add one to anything new you write outside
code.
