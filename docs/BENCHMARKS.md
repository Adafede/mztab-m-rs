# Benchmarks

Measured on 2026-11-18 (M8, [PLAN §7](agents/PLAN.md)). Raw numbers, including where this
library loses.

## Setup

- **Machine:** Apple M1 Pro laptop (8 cores), 16 GB, macOS 27.2.1, with
  other work running. One run each; criterion's median is reported.
- **Toolchain:** rustc 1.100.0, release profile (`cargo bench`), no LTO, no
  `target-cpu`.
- **Code:** the commit after `8b5439c` (M7) that adds `benches/`.
- **Run them:**
  - `cargo bench -p mztab-m-bench --bench throughput` (criterion 0.5);
  - `cargo bench -p mztab-m-bench --bench heap` (dhat; about 40 minutes,
    most of it dhat recording the 1 GB stream).

### Inputs

| Name | File | Size |
|---|---|---|
| small | `2.1/example_study_variable_group.mztab` | 6.3 KB |
| medium | `2.0/LDA_v2.11.1_MTBLS396.mzTab` | 1.17 MB |
| large | `2.1/xcms+MsIO_0.0.11_MTBLS4381_onlySMF.mztab` | 13.6 MB |
| synthetic | `Spec { assays: 1000, rows: 4800 }` (`benches/src/lib.rs`) | about 100 MB |
| synthetic (streamed) | `Spec { assays: 1000, rows: 48000 }`, generated chunk by chunk | about 1 GB |

The synthetic documents are valid mzTab-M 2.1 (`M+S+F+E`) with 1000 assays,
so each SML and SMF row is about 10 KB wide. They are seeded and
deterministic, and `benches/golden.mztab` pins a small one. (After these
numbers were taken, the generator's seed mixing and variation values were
changed so that its tests pin every expression. The row layout, column
count and value formats are the same, but the exact values differ.)

## Throughput

All rates are bytes of the **TSV input** per second, so the stages can be
compared with one another. Validation and writing work on a model already
parsed, so their rates say how much slower or faster they are than reading.

| Stage | small | medium | large | synthetic |
|---|---|---|---|---|
| `lines_std`: split lines and cells, std only (baseline) | 1.06 GiB/s | 1.03 GiB/s | 1.23 GiB/s | 833 MiB/s |
| `lines_memchr`: count tabs and newlines with `memchr` (baseline) | 1.93 GiB/s | 1.40 GiB/s | 1.71 GiB/s | 1.09 GiB/s |
| `parse` (borrowed model) | 126 MiB/s | 117 MiB/s | 213 MiB/s | 188 MiB/s |
| `parse_owned` (`'static` model) | 105 MiB/s | 94 MiB/s | 157 MiB/s | 133 MiB/s |
| `stream_64k` (`StreamReader`, 64 KiB chunks, events only) | 150 MiB/s | 122 MiB/s | 216 MiB/s | 190 MiB/s |
| `write` (byte-exact, with source layout) | 275 MiB/s | 413 MiB/s | 800 MiB/s | 618 MiB/s |
| `validate::structure` | 331 MiB/s | 288 MiB/s | 2.64 GiB/s | 8.30 GiB/s |
| `validate::semantic` (no vocabulary) | 503 MiB/s | 1.61 GiB/s | (see below) | 38 GiB/s |
| `json::to_string` | 59 MiB/s | 39 MiB/s | 132 MiB/s | 108 MiB/s |
| `json::parse` (rate over the TSV size) | 31 MiB/s | 15 MiB/s | 49 MiB/s | 38 MiB/s |

Notes:

- **Semantic validation on `large`** (253 GiB/s) is not meaningful. That
  file has only an SMF table, with almost no parameters to check, so the
  stage does nearly nothing.
- **Reading is 5–9× slower than the bare scans.** The baselines only find
  the tabs and newlines. Parsing also types every cell: numbers with their
  lexemes, parameters, references and lists. It also records the source
  layout, which is what makes byte-exact writing possible.
- **`memchr` would not close that gap.** It scans 1.3–1.8× faster than
  the std split, but a scan is under a fifth of the parse time. The plan's
  `memchr` feature (PLAN §5) would therefore gain at most a few percent;
  see D-073.
- **JSON is the slow path, by design.** Both directions go through TSV text
  (D-071), and the parser builds a tree before transcoding. It is 3–8× slower
  than TSV.
- **Not measured: lex-only.** The reader has no public lexing stage;
  `lines_std` stands in for it.

## Memory (dhat)

Peak heap during the stage, net of what existed before it, and the number of
allocations. The input text itself is not counted.

| Input | Stage | Peak heap | Peak / input | Allocations |
|---|---|---|---|---|
| medium | `parse` | 7.29 MB | 6.24 | 25 067 |
| medium | `parse_owned` | 8.04 MB | 6.88 | 99 313 |
| medium | `write` | 2.26 MB | 1.93 | 4 133 |
| medium | `validate::structure` | 0.89 MB | 0.77 | 22 821 |
| medium | `validate::semantic` | 0.51 MB | 0.44 | 6 157 |
| large | `parse` | 34.5 MB | 2.54 | 32 515 |
| large | `parse_owned` | 46.6 MB | 3.43 | 798 531 |
| large | `write` | 18.4 MB | 1.36 | 8 230 |
| large | `validate::structure` | 0.78 MB | 0.06 | 18 174 |
| large | `validate::semantic` | 0.14 MB | 0.01 | 487 |
| synthetic, about 1 GB, streamed | `StreamReader`, 64 KiB chunks | **0.87 MB** | **0.0009** | 195 019 834 |

- **Streaming memory is bounded by the line, not the file:** 0.87 MB peak
  while streaming about 1 GB. The bench asserts a bound of 64 MB, and the
  longest line is about 10 KB.
- **The model costs 2.5–7× its text.** The medium file is dense in short
  cells (each one a `Cell<T>` with its lexeme), the large file less so.
  That cost is the price of a typed, lossless model; streaming is the
  answer for files that do not fit.
- **`parse_owned` allocates once per owned string.** Avoiding that would
  need an arena, which the zero-dependency, `no_std` core does not have
  (PLAN §5).

## Not done

- **Speed against jmzTab-m (CLI against CLI, e.g. with hyperfine).** Not
  measured: building the jmzTab-m CLI needs a JDK and a Maven download, and
  that has not been done. No comparison is claimed. In-process JMH is out
  of scope (PLAN §7).
- **Repeated runs and variance across machines.** One machine, one run
  each.
