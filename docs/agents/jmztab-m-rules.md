> Provenance: inventory compiled by an AI coding agent from the jmzTab-m
> sources on 2026-11-17. Kept as a reference; not updated.

# jmzTab-m validation-rule inventory

This inventory was read at jmzTab-m commit `182e909` (2026-11-17). It
documents the parent version `2.1.0-SNAPSHOT`. The sources were read only;
nothing was built or run. We use jmzTab-m as a behavioural oracle, not as a
design template. In this document, **(F)** marks a fatal error: parsing stops
and no model is produced. Items marked ✔ were spot-checked by hand.

## Registry

Codes and messages live in
`api/src/main/resources/conf1_1/mztab_{format,logical,crosscheck}_error.properties`.
The severity of each code is fixed in Java: `FormatErrorType`,
`LogicalErrorType`, `CrossCheckErrorType`. The structural output has the
form `[Level-code] line N: msg`, where `N = -1` means the message has no
line.

| Category | Constants | Codes | Never raised or dead |
|---|---|---|---|
| Format | 38 | 1000–1038 (1006 is absent) | 1005, 1010, 1011, 1016, 1020, 1023, 1024, 1027, 1028×2, 1030 |
| Logical | 69 | 2000–2068 | 2011, 2013, 2015–2017, 2019, 2020, 2022–2027, 2029–2032, 2036–2038, 2042, 2043, 2045 (writer only) |
| CrossCheck (with `-s`) | 22 | 3000, 3005–3023 | 3000, 3001–3004 (properties file only), 3012 (dead) |

Two codes are duplicated: 1028 belongs to 2 constants and 3010 to 3.

## Live rules we must cover

### Lexical, line and section rules

| Code | Rule |
|---|---|
| 1000 (F) | Bad line prefix |
| 2002 (F) | Section order |
| 2004 (F) | Data line appears before its header |
| 1001 | Cell count differs from the header count |
| 2000 | Empty cell, or `null` in a non-nullable column |

### MTD rules

| Code | Rule |
|---|---|
| 1008 (F) | Not exactly 3 cells |
| 1009 (F) | Unknown key |
| 2014 (F) | Index is not a positive integer |
| 2010 (F) | Duplicate singleton key |
| 1031 (F) | Version regex `2\.\d\.\d-M` ✔ |
| 1032 (F) | Empty mzTab-ID |
| 1038 | Unknown profile |
| 1012 | Bad parameter |
| 1013 | Bad parameter list |
| 1014 | Bad publication |
| 1015 | Bad URI |
| 1017 | Bad email |
| 1037 | Bad ORCID |
| 2018 | Software parameter has no version (Warn) |
| 1002 (F) | Bad `element[n]` reference |
| 2028 | Duplicate reference |
| 1036 | Bad colunit |
| 2001 | `ms_run` location is `null` (Warn) |

### Metadata refinement

| Code | Rule |
|---|---|
| 2008 | Missing version, ID, software, assay, `assay-ms_run_ref`, SV name/description, `ms_run` location/`scan_polarity`, CV or its 4 sub-fields, or `database` version/uri |
| 2048, 2049, 2064 | Rules for the SV named "undefined" |
| 2012 | SV has no `assay_refs` |
| 2067 | `-factors` used in a non-2.0 file (Warn) |
| 2007, 2028 | Each SV must belong to exactly one group, once any group exists |
| (group checks) | Each group needs a name, a description, a STATO type, a datatype from the enum, and at least one ref that resolves |
| 2044 | "no database" must use the `null` prefix |
| 2068 | Forward reference |
| 2065 | Unreferenced instrument |
| 2066 | Unreferenced sample |

### Header rules

| Code | Rule |
|---|---|
| 2062 (F) | Duplicate column (case-insensitive) |
| 1003, 2006, 2007 (F) | Bad abundance column, or its assay/SV is undefined |
| 1007, 2005 (F) | Bad `opt_` column, or its referent is undefined |
| 2034 (F) | Unknown column |
| 1004 | Missing stable column. **All** stable columns are required, but column order is free. |
| 2039, 2040 (F) | Missing quantification unit for SML/SMF |
| 2041 | Missing reliability scheme (Info) |
| 2009 | Missing abundance column for an MTD assay/SV |
| (SEH) | `id_confidence_measure[n]` count is matched by list position |

### Data-row rules

| Code | Rule |
|---|---|
| 1018 | Bad Integer |
| 1019 | Bad Double |
| 1022 | Bad Double list |
| 1033 | Bad Integer list |
| 1025 | Bad boolean |
| 1035 | Accession has no namespace (Warn) |
| 1034 | Adduct regex, on SML and SMF only (not SME) |
| 2046 | SML `\|`-list lengths differ |
| 1026, 2021, 2050, 2051 | `spectra_ref` and `id_format` patterns |

Non-nullable columns:

- SML: `SML_ID`, `reliability`
- SMF: `SMF_ID`, `exp_mass_to_charge`
- SME: `SME_ID`, `evidence_input_id`, `exp_mass_to_charge`, `charge`, `theoretical_mass_to_charge`, `spectra_ref`, `identification_method`, `ms_level`, `rank`

### Model-level rules

These run **only if no Error was raised before**.

| Code | Rule |
|---|---|
| 1034 | Reliability is not in `[1234]` |
| 1029 | colunit names a column that is not in the header |
| 2047 | Dangling `SMF_ID_REFS` / `SME_ID_REFS` |
| 2052–2061 | Profile rules (infer, missing, forbidden, combination, M+S+F, field mismatch, cross-reference obligations) |
| 2033 | Hash without `hash_method` |
| 2063 | Bad `usi_identifier` |

### Semantic rules (CV mapping)

These need network I/O on first use; the vocabularies are cached afterwards.

| Code | Rule |
|---|---|
| 3005–3011 | CV-mapping rules from 28 rules in the bundled `mzTab-M-mapping.xml` |
| 3013 | CV used but not declared |
| 3014 | CV declared but unused |
| 3016 | Malformed accession |
| 3017–3023 | CV version resolution |

Terms are resolved from versioned OBO/OWL downloads or from OLS4.

### Not checked by jmzTab-m

- Uniqueness of `SML_ID`, `SMF_ID` and `SME_ID`
- Range of the ambiguity code
- Contiguous indices
- Duplicate indexed MTD keys: the later key silently overwrites the earlier one

## Quirks that bear on the differential tests

1. Cells are split with `\s*\t\s*` ✔. Values are trimmed, and trailing empty cells are dropped, so `…\t\t` gives 1001 instead of 2000.
2. Numbers are parsed with Java `Double.parseDouble`. It accepts `1e5`, `NaN`, `Infinity`, `INF`, hex floats and the `d`/`f` suffixes. It rejects `-INF`.
3. The non-null check compares against `null` case-sensitively, but the typed parsers treat `NULL` as null.
4. Line prefixes are case-insensitive, and the long section names are accepted. MTD keys are lower-cased.
5. Email, ORCID, `opt_` and `spectra_ref` patterns are tested with `find()`, i.e. partial matches.
6. `parseParam` takes the text from the first `[` to the last `]`. A missing leading `[` is accepted.
7. These inputs crash the CLI instead of producing a report: `assay[n]-custom[m]`, which the spec allows; a colunit value without `=`; proteomics sections; `database[n]-prefix` without a `database[n]` entry; `null` in a Double or Boolean `opt_` CV column.
8. Code 2003 is unreachable, because a repeated header gives 2002.
9. Table presence for profile checks is computed from rows. A header-only table therefore counts as absent.
10. The SMF→SME reference check is nested inside `if (sml && smf)` ✔ (MzTabFileParser.java:705–762). It is skipped for M+F+E files.
11. assay, cv, software, `scan_polarity` and SV are required even under profile `M`.
12. The profile rules also apply to files that declare `2.0.0-M`. There is no version gating apart from 2067.
13. A UTF-8 BOM is not stripped, which produces a fatal 1000.
14. Message placeholders are filled by position, so some messages are garbled. `ValidationMessage.code` holds the whole formatted string.

## Corpus and CLI

### Corpus

- Valid files, invalid files, profile files, `official-profile-*` files and JSON round-trip files are under `test-utils/src/main/resources/metabolomics/`.
- Location fixtures are under `io/src/test/resources/locations/`.
- `publication-samples/lipidomics-example-validation.txt` holds the reference CLI output for version 1.0.6.

### CLI

Invoke it as `java -jar jmztabm-cli.jar -c FILE -l Info`.

- The output is text. Parse it with `^\[(Info|Warn|Error)-(\d{4})\] line (-?\d+): `.
- There is no JSON output for messages.
- The exit code is 1 if any Warn or Error is reported.
- The README says Java 25 is required.
