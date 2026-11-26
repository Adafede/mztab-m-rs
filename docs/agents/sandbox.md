> Provenance: notes written by AI coding agents from their own sandboxed
> sessions (macOS, Claude Code). Checked by a fresh agent on 2026-11-21.

# Running the tools in a sandboxed session

Outside a sandbox, `./mk setup` and the commands in [AGENTS.md](../../AGENTS.md)
are enough. In a sandbox where `~/.cargo`, `~/.rustup` and `~/.cache` are
read-only and the network is filtered, do this instead.

## Before `./mk`

1. Check the pinned toolchain is already installed:
   `rustup run 1.100.0 rustc --version`. If it is not, a human has to run
   `rustup toolchain install` (rustup cannot write `~/.rustup` from the
   sandbox, and `./mk setup` stops at that step).
2. Put the tools under `target/`, ahead of anything in `~/.cargo/bin`
   (which may hold older versions, e.g. a tombi that `./mk tools` rejects):

   ```sh
   cat > target/env.sh <<'SH'
   export CARGO_HOME="$PWD/target/.cargo-home"
   export PATH="$PWD/target/tools/bin:$CARGO_HOME/bin:$PATH"
   SH
   source target/env.sh
   ```

   `./mk setup` installs pinned tools under `$MK_TOOLS_DIR` (defaults to
   `$CARGO_HOME/mk-tools/<tool>/<ver>/bin`), and `./mk` prepends each tool's
   directory to `PATH` automatically via `refresh_tool_path`.
   `target/tools/bin` is only a place for tools added by hand; it may not
   exist.
3. If another checkout of this repository already has these tools in its
   `target/`, putting that checkout's two directories on `PATH` instead is
   the quickest way; `./mk tools` checks the versions either way.

## Network

- Crates: allow `index.crates.io` and `static.crates.io`.
- `deny` and `audit` fetch the RustSec advisory database from `github.com`;
  without it they fail, which says nothing about the code.
- `./mk setup` downloads tombi and shellcheck from `github.com` release
  assets. cargo-binstall has failed to reach crates.io from the sandbox even
  when cargo could; setup then builds from source, which is slower but works.
- Python builds with `uv` need `UV_CACHE_DIR` set to a writable directory.

## Other

- Commits are GPG-signed by the user's git config; the gpg agent is not
  reachable from inside the sandbox, so committing needs to run outside it.
- `./mk ci-fast` takes about 3–4 minutes on an M1 Pro (wasm alone about
  100 s). `./mk ci` adds mutation testing on the diff; its `mktemp` and
  memory guard misbehave in the sandbox, so it has been run by hand
  (`cargo mutants --in-diff … --jobs 2 --timeout 240`).
- Run `cargo fuzz` with `--target aarch64-apple-darwin` when the installed
  cargo-fuzz is an x86_64 build.
- The jmzTab-m oracle needs a JDK 25 and a Maven download; it has never been
  run (see [STATUS.md](../../STATUS.md)).
