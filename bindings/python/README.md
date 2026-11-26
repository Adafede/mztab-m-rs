# mztab-m (Python)

**Status: prototype.** Builds from source and passes its tests on macOS
(arm64) and Linux (`x86_64` in CI); not on PyPI, and Windows is untested. See
[STATUS.md](https://github.com/Adafede/mztab-m-rs/blob/main/STATUS.md).

Read, validate, convert and inspect [mzTab-M](https://github.com/HUPO-PSI/mzTab-M)
metabolomics files, as TSV or JSON, from Python. It is the Rust library
[`mztab-m`](https://github.com/Adafede/mztab-m-rs/tree/main/crates/mztab-m),
compiled: the same reader, validator and writer as the `mztab-m` command.

```python
from pathlib import Path
import mztab_m

text = Path("study.mztab").read_text()
obo = Path("psi-ms.obo").read_text()
for f in mztab_m.validate(text, vocabularies=[obo]):
    print(f["severity"], f["code"], f["location"], f["summary"], f["detail"])

json_text = mztab_m.convert(text, "json")          # refuses what JSON cannot carry
json_text = mztab_m.convert(text, "json", lossy=True)
print(mztab_m.inspect(text)["sml_rows"])
```

- `validate(text, *, vocabularies=[], layer="semantic", severities={}, format=None)`
  returns a list of dicts (`code`, `severity`, `layer`, `line`, `column`,
  `cell`, `location`, `summary`, `detail`, `spec`).
- `convert(text, to, *, lossy=False, format=None)`, `canonical(text)`,
  `inspect(text)`, `codes()`, `vocabulary_size(text)`.
- `text` may be `str`, `bytes`, or `bytearray`; the format is detected
  (`{` means JSON) unless `format="tsv"` or `"json"` is given. Invalid
  argument values raise `ValueError`; wrong argument types raise `TypeError`.

Install directly from the repository root with `uv` or `pip` (uses
[maturin](https://www.maturin.rs) via `pyproject.toml`), or sync the project
environment for development:

```sh
uv pip install .
# or:
pip install .
# or for local development:
uv sync
```

Or build a wheel from the repository root with `maturin build --release`. One
`abi3` wheel per platform serves Python 3.9 and later, with PEP 484 type stubs
(`py.typed`) bundled in the package.
