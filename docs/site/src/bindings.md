# Python & R Bindings

Both bindings wrap `mztab-m-api` so Python and R users run the exact same
reader, validator, writer, and converter as the `mztab-m` command-line tool.
From a repository checkout, `./mk bindings` builds and tests both packages.

## Python (`mztab-m`)

{{#include ../../../bindings/python/README.md:3:}}

## R (`mztabm`)

{{#include ../../../bindings/r/README.md:3:}}
