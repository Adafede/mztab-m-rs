# mztab-m-cli

The `mztab-m` command: validate, format, convert and inspect
[mzTab-M](https://github.com/HUPO-PSI/mzTab-M) files, as TSV or JSON.

## Command-line usage

```sh
mztab-m validate --cv psi-ms.obo study.mztab
mztab-m validate --format json --fail-on warning study.mztab
mztab-m fmt --check study.mztab
mztab-m convert --to json -o study.json study.mztab
mztab-m inspect study.mztab
```

Run `mztab-m --help` for every option.

- **Exit codes:** `0` ok, `1` findings at or above `--fail-on` (or `fmt --check`
  would change the file), `2` bad command line, `3` a file could not be read
  or written (or the model cannot be represented in the target format).
- **Vocabularies:** the command never goes to the network; pass local OBO files
  with `--cv` (for example `psi-ms.obo` from
  [HUPO-PSI/psi-ms-CV](https://github.com/HUPO-PSI/psi-ms-CV), tested against
  release `v4.2.17`).

## Library entry point

All command logic lives in `run`, which takes arguments and an `Io`
implementation; the `mztab-m` binary only connects the file system and standard
streams.

```rust
use mztab_m_cli::{FINDINGS, Io, OK, run};

#[derive(Default)]
struct MemoryIo {
    out: String,
    err: String,
}

impl Io for MemoryIo {
    fn read(&mut self, _path: &str) -> Result<Vec<u8>, String> {
        Ok(b"MTD\tmzTab-version\t2.1.0-M\nMTD\tmzTab-ID\tCLI-1\n".to_vec())
    }
    fn write(&mut self, _path: &str, text: &str) -> Result<(), String> {
        self.out.push_str(text);
        Ok(())
    }
    fn out(&mut self, text: &str) {
        self.out.push_str(text);
    }
    fn err(&mut self, text: &str) {
        self.err.push_str(text);
    }
}

let mut io = MemoryIo::default();
assert_eq!(run(&["validate".to_owned(), "in.mztab".to_owned()], &mut io), FINDINGS);
assert!(io.out.contains("error[MZT-T009]"));

io.out.clear();
assert_eq!(run(&["fmt".to_owned(), "in.mztab".to_owned()], &mut io), OK);
assert!(io.out.starts_with("MTD\tmzTab-version\t2.1.0-M\n"));
```
