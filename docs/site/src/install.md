# Install

Nothing is published to crates.io, PyPI, or CRAN yet. All targets build cleanly
from source, or you can try the validator directly in the
[Live Browser Demo](demo/index.html) without installing anything.

## Command-line tool (`mztab-m`)

You need Rust (the pinned toolchain `1.100.0` installs automatically through
`rust-toolchain.toml` if you use `rustup`).

```sh
git clone --recurse-submodules https://github.com/Adafede/mztab-m-rs
cd mztab-m-rs
cargo install --path crates/mztab-m-cli --locked
mztab-m --version
```

The command never goes to the network. To enable controlled-vocabulary term
checks (`--cv`), download an OBO file such as `psi-ms.obo` from
[HUPO-PSI/psi-ms-CV](https://github.com/HUPO-PSI/psi-ms-CV) (tested against
release `v4.2.17`).

## Rust libraries

While crates are unpublished, depend on the repository path or git URL in
`Cargo.toml`:

```toml
[dependencies]
mztab-m = { git = "https://github.com/Adafede/mztab-m-rs" }
# Optional OBO 1.4 vocabulary loader:
mztab-m-obo = { git = "https://github.com/Adafede/mztab-m-rs" }
```

To enable JSON schema read/write support on `mztab-m`, turn on the `json`
feature:

```toml
mztab-m = { git = "https://github.com/Adafede/mztab-m-rs", features = ["json"] }
```

## Python and R packages

Both bindings are prototypes outside the Rust workspace and build from source.
See [Python & R Bindings](bindings.md) for prerequisites and build commands.
