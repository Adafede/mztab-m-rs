> Provenance: progress log kept by the AI coding agent that built the
> milestones in [PLAN.md](PLAN.md). For the current state of the project,
> read [STATUS.md](../../STATUS.md) instead.

# Milestones and local environment notes

## Milestones

Milestones are defined in [PLAN.md §8](PLAN.md); decisions are in
[DECISIONS.md](../../DECISIONS.md).

| # | Milestone | Commit | Decisions |
|---|---|---|---|
| M0 | Scaffold | `d35566b` | |
| M1 | Model and finding registry | `ba8cbed` | D-050–D-055 |
| M2 | TSV reader, streaming | `5b7d577` | D-056–D-059 |
| M3 | Writer, byte-exact layout | `2f332cc` | D-060–D-063 |
| M4 | Structure validator, oracle harness | `95bd5c4` | D-064–D-066 |
| M5 | Semantic validator, CV rules, `mztab-m-obo` | `b969cec` | D-067–D-070 |
| M6 | JSON (feature `json`) | `c5b7c29` | D-071 |
| M7 | CLI (`mztab-m`) and browser demo | `8b5439c` | D-072 |
| M8 | Benchmarks ([BENCHMARKS.md](../BENCHMARKS.md)) | `570c987` | D-073 |
| M9 | Docs and release preparation | `9378f48` | D-074 |
| — | Remaining checks, API crate, Python and R packages | `5eb36a0` | D-075, D-076 |
| — | The corpus against PSI-MS 4.2.17 | `4a0af59` | D-077 |

The agent recorded that every gate passed at each of these commits. That was
local only: no CI run exists for any of them.

## Local environment notes (macOS, sandboxed agent sessions)

- When `~/.cargo` is not writable (sandboxed sessions), tools live in
  `target/`. Create `target/env.sh` once:

  ```sh
  export CARGO_HOME="$PWD/target/.cargo-home"
  export PATH="$PWD/target/tools/bin:$CARGO_HOME/bin:$PATH"
  ```

  then `source target/env.sh` before `./mk`. `./mk setup` reinstalls the
  tools after `target/` is wiped (tombi, cargo-audit and cargo-fuzz may need
  installing separately).
- Same for Python: set `UV_CACHE_DIR` to a writable directory.
- `deny` and `audit` fetch the RustSec advisory database from github.com;
  without network access they fail, which says nothing about the code.
- Commits are GPG-signed by the user's git config; the gpg agent is not
  reachable from inside the sandbox.
- Run `cargo fuzz` with `--target aarch64-apple-darwin` when the installed
  cargo-fuzz is an x86_64 build.
- The `mutants-diff` gate has been run by hand
  (`cargo mutants --in-diff … --jobs 2 --timeout 240`); inside the sandbox its
  `mktemp` and memory guard misbehave.
- The jmzTab-m oracle needs a JDK 25 and a Maven download; it has never been
  run (see [STATUS.md](../../STATUS.md)).
