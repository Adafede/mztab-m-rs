# Quickstart (CLI)

Validate an example file from the pinned specification:

```sh
mztab-m validate third_party/mzTab-M/examples/2.1/example_study_variable_group.mztab
```

Findings go to standard output and the summary line goes to standard error:

```text
warning[MZT-S013] line 65, column 67, cell 3: leading or trailing spaces
...
info[MZT-T011] document: profile inferred from the tables present: profile `M+S`
info[MZT-T016] line 108, column 5, cell 2: field describes a table that is not present: `small_molecule_feature-quantification_unit`
warning[MZT-X001] line 110, column 17, cell 3: CV label is not declared: `MIRIAM`
0 errors, 11 warnings, 2 infos
```

## Common tasks

```sh
# Check controlled-vocabulary terms against a local OBO file (e.g. PSI-MS v4.2.17)
mztab-m validate --cv psi-ms.obo study.mztab

# Machine-readable findings (JSON array) and strict failure threshold
mztab-m validate --format json --fail-on warning study.mztab

# Override or disable individual finding codes
mztab-m validate --severity MZT-S013=info --severity MZT-X002=off study.mztab

# Check canonical formatting or write canonical form
mztab-m fmt --check study.mztab
mztab-m fmt -o canonical.mztab study.mztab

# Convert between TSV and JSON (add --lossy to drop non-schema metadata/columns)
mztab-m convert --to json -o study.json study.mztab
mztab-m convert --to tsv -o study.mztab study.json

# Print a short summary (version, profile, element and row counts)
mztab-m inspect study.mztab
```

Pass `-` in place of `FILE` to read from standard input; JSON input is detected
from a `.json` suffix or a leading `{`, and `--input tsv|json` overrides detection.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success (no finding at or above `--fail-on`, and `fmt --check` found canonical input) |
| `1` | Validation found an issue at or above `--fail-on`, or `fmt --check` would modify the file |
| `2` | Invalid command line arguments |
| `3` | Input/output error, or the model cannot be represented in the requested format |

Run `mztab-m --help` for the full option list.
