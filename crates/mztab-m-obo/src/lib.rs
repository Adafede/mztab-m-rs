#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod parse;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use mztab_m::cv::{Term, TermLookup};
use parse::Entry;

/// The terms of one OBO 1.4 document, borrowed from its text.
///
/// Parsing is lenient and never fails: what it does not use (other
/// stanzas, other tags, comments, qualifiers) is skipped, a stanza without
/// an `id` is dropped, and a repeated `id` keeps its first stanza.
/// Ancestors are followed through `is_a` and `relationship: part_of`
/// (DECISIONS D-069).
///
/// An ontology *covers* (answers for) its own namespace only: the prefix
/// its header's `ontology:` names (`ms` covers `MS`), or, without that
/// header, every prefix its terms use. Terms of other vocabularies that a
/// file carries (PSI-MS holds some UO terms) are not enough to answer for
/// those vocabularies (D-077); [`cover`](Self::cover) widens it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ontology<'a> {
    terms: BTreeMap<&'a str, Entry<'a>>,
    covered: BTreeSet<&'a str>,
    namespace: Option<&'a str>,
    data_version: Option<&'a str>,
}

impl<'a> Ontology<'a> {
    /// Parses an OBO 1.4 document.
    ///
    /// ```
    /// let obo = mztab_m_obo::Ontology::parse("[Term]\nid: UO:0000010\nname: second\n");
    /// assert_eq!(obo.len(), 1);
    /// assert_eq!(obo.data_version(), None);
    /// ```
    #[must_use]
    pub fn parse(text: &'a str) -> Self {
        parse::parse_ontology(text)
    }

    /// Also covers `prefix`: answers for that vocabulary's terms from this
    /// file (for a file that is the whole of more than one namespace).
    ///
    /// ```
    /// use mztab_m::cv::TermLookup;
    ///
    /// let obo = "ontology: ms\n\n[Term]\nid: MS:1\n\n[Term]\nid: PEFF:1\n";
    /// let ms = mztab_m_obo::Ontology::parse(obo);
    /// assert!(ms.covers("MS") && !ms.covers("PEFF"));
    /// assert!(ms.cover("PEFF").covers("PEFF"));
    /// ```
    #[must_use]
    pub fn cover(mut self, prefix: &'a str) -> Self {
        self.covered.insert(prefix);
        self
    }

    /// The number of terms.
    #[must_use]
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether the document has no terms.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The header's `data-version`, if any.
    #[must_use]
    pub const fn data_version(&self) -> Option<&'a str> {
        self.data_version
    }
}

impl TermLookup for Ontology<'_> {
    fn covers(&self, prefix: &str) -> bool {
        self.covered.contains(prefix)
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        self.terms.get(accession).map(|entry| Term {
            name: entry.name,
            obsolete: entry.obsolete,
            value_type: entry.value_type,
            synonyms: &entry.synonyms,
        })
    }

    /// Follows `is_a` and `part_of` upwards; each term is visited once, so
    /// cycles end.
    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&[&str]> = self
            .terms
            .get(accession)
            .map_or_else(Vec::new, |e| alloc::vec![e.parents.as_slice()]);
        while let Some(parents) = stack.pop() {
            for &term in parents {
                if term == ancestor {
                    return true;
                }
                if seen.insert(term)
                    && let Some(entry) = self.terms.get(term)
                {
                    stack.push(&entry.parents);
                }
            }
        }
        false
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        self.data_version.filter(|_| self.covers(prefix))
    }
}
