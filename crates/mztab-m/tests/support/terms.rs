//! A small PSI-MS stand-in for tests: the terms the base document uses,
//! with their ancestors, plus a few made-up terms for negative cases.

use mztab_m::cv::{Term, TermLookup};

/// `(accession, name, obsolete, parents)`.
type Entry = (&'static str, &'static str, bool, &'static [&'static str]);

const TERMS: &[Entry] = &[
    ("MS:1000026", "detector type", false, &[]),
    ("MS:1000031", "instrument model", false, &[]),
    ("MS:1000044", "dissociation method", false, &[]),
    (
        "MS:1000073",
        "electrospray ionization",
        false,
        &["MS:1000458"],
    ),
    ("MS:1000129", "negative scan", false, &[]),
    ("MS:1000130", "positive scan", false, &[]),
    ("MS:1000133", "CID", false, &["MS:1000044"]),
    ("MS:1000253", "electron multiplier", false, &["MS:1000026"]),
    ("MS:1000451", "mass analyzer", false, &[]),
    ("MS:1000458", "source", false, &[]),
    (
        "MS:1000483",
        "Thermo Fisher Scientific instrument model",
        false,
        &["MS:1000031"],
    ),
    ("MS:1000484", "orbitrap", false, &["MS:1000451"]),
    ("MS:1000511", "ms level", false, &[]),
    ("MS:1000531", "software", false, &[]),
    ("MS:1000560", "mass spectrometer file format", false, &[]),
    ("MS:1000561", "data file checksum type", false, &[]),
    ("MS:1000569", "SHA-1", false, &["MS:1000561"]),
    ("MS:1000584", "mzML format", false, &["MS:1000560"]),
    (
        "MS:1000767",
        "native spectrum identifier format",
        false,
        &[],
    ),
    (
        "MS:1000774",
        "multiple peak list nativeID format",
        false,
        &["MS:1000767"],
    ),
    (
        "MS:1001058",
        "quality estimation by manual validation",
        false,
        &[],
    ),
    ("MS:1001060", "search type", false, &[]),
    ("MS:1001456", "analysis software", false, &["MS:1000531"]),
    ("MS:1001477", "SpectraST", false, &["MS:1001456"]),
    ("MS:1001833", "quantitation analysis summary", false, &[]),
    (
        "MS:1001834",
        "LC-MS label-free quantitation analysis",
        false,
        &["MS:1001833"],
    ),
    ("MS:1002879", "Progenesis QI", false, &["MS:1001456"]),
    ("MS:1002882", "study variable average function", false, &[]),
    ("MS:1002883", "median", false, &["MS:1002882"]),
    (
        "MS:1002884",
        "study variable variation function",
        false,
        &[],
    ),
    ("MS:1002885", "standard error", false, &["MS:1002884"]),
    (
        "MS:1002888",
        "small molecule confidence measure",
        false,
        &[],
    ),
    ("MS:1002890", "fragmentation score", false, &["MS:1002888"]),
    (
        "MS:1002895",
        "small molecule identification attribute",
        false,
        &[],
    ),
    (
        "MS:1002896",
        "compound identification confidence level",
        false,
        &["MS:1002895"],
    ),
    // Made up, for negative cases.
    (
        "MS:1009998",
        "search software",
        false,
        &["MS:1001060", "MS:1000531"],
    ),
    ("MS:1009999", "old term", true, &["MS:1001833"]),
    ("MS:1009997", "flag", false, &[]),
];

/// The value types of the stand-in's terms (as PSI-MS declares them).
fn value_type(accession: &str) -> Option<&'static str> {
    match accession {
        "MS:1000511" => Some("int"),
        "MS:1002890" => Some("double"),
        "MS:1009997" => Some("boolean"),
        _ => None,
    }
}

/// The PSI-MS stand-in: covers the `MS` prefix, version `4.1.0`.
#[derive(Debug)]
pub(crate) struct Mini;

fn entry(accession: &str) -> Option<&'static Entry> {
    TERMS.iter().find(|(a, ..)| *a == accession)
}

impl TermLookup for Mini {
    fn covers(&self, prefix: &str) -> bool {
        prefix == "MS"
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        entry(accession).map(|(accession, name, obsolete, _)| Term {
            name,
            obsolete: *obsolete,
            value_type: value_type(accession),
        })
    }

    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        entry(accession).is_some_and(|(_, _, _, parents)| {
            parents
                .iter()
                .any(|p| *p == ancestor || self.is_descendant(p, ancestor))
        })
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        self.covers(prefix).then_some("4.1.0")
    }
}
