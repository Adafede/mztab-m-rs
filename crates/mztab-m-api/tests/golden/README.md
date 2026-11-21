# Golden files

`psi-ms.txt` holds the findings for every spec example validated against
PSI-MS 4.2.17 (DECISIONS D-077). It is written by `tests/vocabulary.rs`,
which needs the vocabulary file: `./mk local:vocab` downloads it and runs the
test. To rewrite after an intended change:

```sh
BLESS=1 PSI_MS_OBO=target/vocab/psi-ms-v4.2.17.obo cargo test -p mztab-m-api --test vocabulary -- --ignored
```
