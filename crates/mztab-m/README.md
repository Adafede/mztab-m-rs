# mztab-m

Reader, writer, and validator for [mzTab-M](https://github.com/HUPO-PSI/mzTab-M),
the HUPO-PSI / MSI exchange format for mass-spectrometry metabolomics results.

- Targets mzTab-M **2.1.0-M** (draft, pinned spec commit) and reads 2.0.0-M.
- `#![no_std]` + `alloc`, zero runtime dependencies, no `unsafe`.
- Pure: no I/O, no global state. Bring your own bytes.

This crate is under construction; the API below is a placeholder.

```rust
// The spec commit every rule in this crate is checked against.
let pinned = mztab_m::SPEC_COMMIT;
assert!(pinned.starts_with("e66c19e"), "pinned to HUPO-PSI/mzTab-M e66c19e");
```
