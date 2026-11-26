# mztab-m-obo

[OBO 1.4](https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html)
vocabularies (such as `psi-ms.obo`) as a
[`TermLookup`](https://adafede.github.io/mztab-m-rs/api/mztab_m/cv/trait.TermLookup.html)
for semantic validation with
[`mztab-m`](https://github.com/Adafede/mztab-m-rs/tree/main/crates/mztab-m).

- `#![no_std]` + `alloc`, no dependencies besides `mztab-m`, no `unsafe`.
- Pure: parses caller-supplied text; never downloads anything.
- Zero-copy: an `Ontology` borrows the text it was parsed from.

## Quickstart

Parse an OBO document and inspect terms:

```rust
use mztab_m::cv::TermLookup;
use mztab_m_obo::Ontology;

let obo = "data-version: 4.1.195\n\n\
           [Term]\nid: MS:1000031\nname: instrument model\n\n\
           [Term]\nid: MS:1000483\nname: Thermo Fisher Scientific instrument model\n\
           is_a: MS:1000031 ! instrument model\n";
let ms = Ontology::parse(obo);
assert!(ms.covers("MS"));
assert!(ms.is_descendant("MS:1000483", "MS:1000031"));
assert_eq!(ms.version("MS"), Some("4.1.195"));
```

Read, semantically validate with an `Ontology`, and write back:

```rust
use mztab_m::{parse, to_string, validate};
use mztab_m_obo::Ontology;

let obo = "ontology: ms\n\n\
           [Term]\nid: MS:1001833\nname: quantitation analysis summary\n\n\
           [Term]\nid: MS:1001834\nname: LC-MS label-free quantitation analysis\n\
           is_a: MS:1001833 ! quantitation analysis summary\n";
let ms = Ontology::parse(obo);

let mztab = "MTD\tmzTab-version\t2.1.0-M\n\
             MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free quantitation analysis, ]\n";
let parsed = parse(mztab);
let findings = validate::semantic(&parsed.mztab, &validate::Config::default(), &ms);
assert!(findings.iter().any(|f| f.code.id() == "MZT-X001"), "MS CV not declared");
assert_eq!(to_string(&parsed.mztab)?, mztab);
# Ok::<(), mztab_m::WriteError>(())
```

Several vocabularies combine as an array or slice: `&[&psi_ms, &uo]` asks
each in turn for the prefixes it covers.
