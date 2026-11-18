# jmzTab-m oracle outputs

The files `*.txt` here are the output of the jmzTab-m validator
(`java -jar jmztabm-cli.jar -c FILE -l Info`), built from the pinned
`third_party/jmzTab-m` submodule, for each corpus example and reader
fixture. File names are the corpus paths with `/` replaced by `__`.

They are produced by `.github/workflows/oracle.yml` (no JVM is needed
locally; DECISIONS D-030). To adopt a run: download its `jmztab-m-oracle`
artifact, copy the files here, and commit.

`tests/oracle.rs` compares each output with this crate's findings, by
jmzTab-m code and line, through `Code::jmztab`. Every disagreement must be
listed in `expected-disagreements.tsv` with the DECISIONS.md entry that
explains it; anything unlisted fails the test.
