# mztab-m-obo

[OBO 1.4](https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html)
vocabularies (such as `psi-ms.obo`) as a
[`TermLookup`](https://docs.rs/mztab-m/latest/mztab_m/cv/trait.TermLookup.html)
for the semantic validation of [`mztab-m`](../mztab-m).

- `#![no_std]` + `alloc`, no dependencies besides `mztab-m`, no `unsafe`.
- Pure: parses text you supply; never downloads anything.
- Zero-copy: an `Ontology` borrows the text it was parsed from.

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

Several vocabularies combine as an array or slice: `&[&psi_ms, &uo]` asks
each in turn for the prefixes it covers.
