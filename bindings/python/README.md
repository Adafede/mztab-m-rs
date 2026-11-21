# mztab-m (Python)

**Status: prototype.** Builds from source and passes its tests on macOS
(arm64); not on PyPI, and not yet built or tested on other platforms. See
[STATUS.md](https://github.com/Adafede/mztab-m-rs/blob/main/STATUS.md).

Read, validate, convert and inspect [mzTab-M](https://github.com/HUPO-PSI/mzTab-M)
metabolomics files, as TSV or JSON, from Python. It is the Rust library
[`mztab-m`](https://github.com/Adafede/mztab-m-rs/tree/main/crates/mztab-m),
compiled: the same reader, validator and writer as the `mztab-m` command.

```python
import mztab_m

text = open("study.mztab").read()
for f in mztab_m.validate(text, vocabularies=[open("psi-ms.obo").read()]):
    print(f["severity"], f["code"], f["location"], f["summary"], f["detail"])

json_text = mztab_m.convert(text, "json")          # refuses what JSON cannot carry
json_text = mztab_m.convert(text, "json", lossy=True)
print(mztab_m.inspect(text)["sml_rows"])
```

- `validate(text, *, vocabularies=[], layer="semantic", severities={}, format=None)`
  returns a list of dicts (`code`, `severity`, `layer`, `line`, `column`,
  `cell`, `location`, `summary`, `detail`, `spec`).
- `convert(text, to, *, lossy=False, format=None)`, `canonical(text)`,
  `inspect(text)`, `codes()`.
- `text` may be `str` or `bytes`; the format is detected (`{` means JSON)
  unless `format="tsv"` or `"json"` is given. Bad arguments raise
  `ValueError`.

Build from source with [maturin](https://www.maturin.rs): `maturin build
--release` in `bindings/python`. One `abi3` wheel per platform serves
Python 3.9 and later.
