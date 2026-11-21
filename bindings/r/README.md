# mztabm (R)

Read, validate, convert and inspect [mzTab-M](https://github.com/HUPO-PSI/mzTab-M)
metabolomics files, as TSV or JSON, from R. It is the Rust library
[`mztab-m`](https://crates.io/crates/mztab-m), compiled through
[extendr](https://extendr.github.io): the same reader, validator and writer
as the `mztab-m` command.

```r
library(mztabm)
text <- paste(readLines("study.mztab"), collapse = "\n")
findings <- validate(text, vocabularies = paste(readLines("psi-ms.obo"), collapse = "\n"))
findings[findings$severity == "error", c("code", "location", "summary")]
json <- convert(text, "json", lossy = TRUE)
inspect(text)$sml_rows
```

`validate()` returns a data frame (`code`, `severity`, `layer`, `line`,
`column`, `cell`, `location`, `summary`, `detail`, `spec`); `convert()`,
`canonical()`, `inspect()`, `codes()` and `spec_commit()` complete it.

Install from the repository with `R CMD INSTALL bindings/r` (needs a Rust
toolchain, 1.100 or later).
