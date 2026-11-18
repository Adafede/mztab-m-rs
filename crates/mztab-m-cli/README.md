# mztab-m-cli

The `mztab-m` command: validate, format, convert and inspect
[mzTab-M](https://github.com/HUPO-PSI/mzTab-M) files, as TSV or JSON.

```sh
mztab-m validate --cv psi-ms.obo study.mztab
mztab-m validate --format json --fail-on warning study.mztab
mztab-m fmt --check study.mztab
mztab-m convert --to json -o study.json study.mztab
mztab-m inspect study.mztab
```

Run `mztab-m --help` for every option. Exit codes: 0 ok, 1 findings (or
`fmt --check` would change the file), 2 bad command line, 3 a file could
not be read or written.

The command reads vocabularies only from the OBO files you name with
`--cv`; it never goes to the network.
