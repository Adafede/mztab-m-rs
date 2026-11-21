//! OBO parsing and the term lookup, alone and in semantic validation.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::cv::{Term, TermLookup};
use mztab_m::validate::{self, Config};
use mztab_m::{Code, parse};
use mztab_m_obo::Ontology;

const MS: &str = "format-version: 1.2\r
data-version: 4.1.195\r
ontology: ms\r
\r
[Term]\r
id: MS:1000031\r
name: instrument model\r
def: \"An instrument model.\" [PSI:MS]\r
\r
[Term]\r
id: MS:1000483 ! a comment\r
name: Thermo Fisher Scientific instrument model\r
is_a: MS:1000031 ! instrument model\r
\r
[Term]\r
id: MS:1001911\r
name: Q Exactive\r
relationship: part_of MS:1000483 {source=\"x\"} ! Thermo\r
relationship: has_units UO:0000221 ! dalton\r
\r
[Term]\r
id: MS:1000001\r
name: sample number\r
is_obsolete: true\r
xref: value-type:xsd\\:string \"The allowed value-type for this CV term.\"\r
xref: PMID:12345\r
\r
[Term]\r
name: no id: dropped\r
\r
[Term]\r
id: MS:1000031\r
name: duplicate: first wins\r
\r
[Typedef]\r
id: part_of\r
name: part of\r
\r
[Term]\r
id: MS:9000001\r
name: loop a\r
is_a: MS:9000002\r
\r
[Term]\r
id: MS:9000002\r
name: loop b\r
is_a: MS:9000001\r
";

#[test]
fn parses_terms_and_header() {
    let ms = Ontology::parse(MS);
    assert_eq!(ms.len(), 6, "five unique ids with a name, one loop pair");
    assert!(!ms.is_empty(), "not empty");
    assert_eq!(ms.data_version(), Some("4.1.195"), "header");
    assert_eq!(ms.version("MS"), Some("4.1.195"), "version of MS");
    assert_eq!(ms.version("UO"), None, "UO not covered");
    assert!(ms.covers("MS") && !ms.covers("UO"), "prefixes");
    assert_eq!(
        ms.term("MS:1000031"),
        Some(Term {
            name: "instrument model",
            obsolete: false,
            value_type: None,
        }),
        "first stanza wins"
    );
    assert_eq!(
        ms.term("MS:1000483").map(|t| t.name),
        Some("Thermo Fisher Scientific instrument model"),
        "comment stripped from id"
    );
    assert!(
        ms.term("MS:1000001").is_some_and(|t| t.obsolete),
        "obsolete"
    );
    assert_eq!(
        ms.term("MS:1000001").and_then(|t| t.value_type),
        Some("string"),
        "value type"
    );
    let plain = Ontology::parse(
        "[Term]\nid: X:1\nxref: value-type:xsd:int\nxref: value-type:other\nxref: value-type:xsd\\:\n",
    );
    assert_eq!(
        plain.term("X:1").and_then(|t| t.value_type),
        Some("int"),
        "unescaped form; others ignored"
    );
    assert_eq!(ms.term("part_of"), None, "typedefs are not terms");
    assert!(Ontology::parse("").is_empty(), "empty text");
}

#[test]
fn ancestors_follow_is_a_and_part_of() {
    let ms = Ontology::parse(MS);
    assert!(ms.is_descendant("MS:1000483", "MS:1000031"), "is_a");
    assert!(
        ms.is_descendant("MS:1001911", "MS:1000031"),
        "part_of then is_a"
    );
    assert!(
        !ms.is_descendant("MS:1001911", "UO:0000221"),
        "other relationships are not followed"
    );
    assert!(
        !ms.is_descendant("MS:1000031", "MS:1000031"),
        "strict descendants only"
    );
    assert!(!ms.is_descendant("MS:404", "MS:1000031"), "unknown term");
    assert!(
        !ms.is_descendant("MS:9000001", "MS:1000031"),
        "a cycle ends"
    );
    assert!(
        ms.is_descendant("MS:9000001", "MS:9000001"),
        "via the cycle"
    );
}

#[test]
fn drives_semantic_validation() {
    let ms = Ontology::parse(MS);
    let uo = Ontology::parse("[Term]\nid: UO:0000010\nname: second\n");
    let input = "MTD\tmzTab-version\t2.1.0-M\n\
                 MTD\tinstrument[1]-name\t[MS, MS:1001911, Q Exactive, ]\n\
                 MTD\tinstrument[1]-source\t[MS, MS:1000001, sample number, ]\n\
                 MTD\tinstrument[1]-detector\t[MS, MS:1000483, wrong name, ]\n\
                 MTD\tcv[1]-label\tMS\n\
                 MTD\tcv[1]-version\t4.1.0\n\
                 MTD\tcolunit-small_molecule\tx=[UO, UO:0000010, second, ]\n";
    let model = parse(input).mztab;
    let lookups = [&ms, &uo];
    let findings = validate::semantic(&model, &Config::default(), &lookups);
    let codes: Vec<Code> = findings.iter().map(|f| f.code).collect();
    for code in [
        Code::ObsoleteTerm,
        Code::TermName,
        Code::CvRule,
        Code::CvVersion,
        Code::CvUndeclared,
    ] {
        assert!(codes.contains(&code), "{code} in {codes:?}");
    }
    assert!(
        !findings
            .iter()
            .any(|f| f.code == Code::CvRule && f.to_string().contains("MS:1001911")),
        "Q Exactive is an instrument model: {findings:?}"
    );
}
