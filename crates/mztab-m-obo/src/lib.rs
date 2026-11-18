#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use mztab_m::cv::{Term, TermLookup, prefix};

/// One `[Term]` stanza.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Entry<'a> {
    name: &'a str,
    obsolete: bool,
    /// `is_a` and `relationship: part_of` targets.
    parents: Vec<&'a str>,
}

/// The terms of one OBO 1.4 document, borrowed from its text.
///
/// Parsing is lenient and never fails: what it does not use (other
/// stanzas, other tags, comments, qualifiers) is skipped, a stanza without
/// an `id` is dropped, and a repeated `id` keeps its first stanza.
/// Ancestors are followed through `is_a` and `relationship: part_of`
/// (DECISIONS D-069).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ontology<'a> {
    terms: BTreeMap<&'a str, Entry<'a>>,
    prefixes: BTreeSet<&'a str>,
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
        let mut ontology = Self::default();
        let mut stanza: Option<(Option<&'a str>, Entry<'a>)> = None;
        let mut in_header = true;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                ontology.close(stanza.take());
                in_header = false;
                if line == "[Term]" {
                    stanza = Some((None, Entry::default()));
                }
                continue;
            }
            let Some((tag, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            if in_header {
                if tag == "data-version" {
                    ontology.data_version = Some(value);
                }
                continue;
            }
            let Some((id, entry)) = stanza.as_mut() else {
                continue;
            };
            match tag {
                "id" => *id = Some(target(value)),
                "name" => entry.name = value,
                "is_obsolete" => entry.obsolete = value == "true",
                "is_a" => entry.parents.push(target(value)),
                "relationship" => {
                    if let Some(parent) = value.strip_prefix("part_of ") {
                        entry.parents.push(target(parent));
                    }
                }
                _ => {}
            }
        }
        ontology.close(stanza);
        ontology
    }

    fn close(&mut self, stanza: Option<(Option<&'a str>, Entry<'a>)>) {
        if let Some((Some(id), entry)) = stanza {
            self.prefixes.insert(prefix(id));
            self.terms.entry(id).or_insert(entry);
        }
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

/// The identifier in a tag value: before any `!` comment or `{`
/// qualifiers.
fn target(value: &str) -> &str {
    value.split(['!', '{']).next().map_or(value, str::trim)
}

impl TermLookup for Ontology<'_> {
    fn covers(&self, prefix: &str) -> bool {
        self.prefixes.contains(prefix)
    }

    fn term(&self, accession: &str) -> Option<Term<'_>> {
        self.terms.get(accession).map(|entry| Term {
            name: entry.name,
            obsolete: entry.obsolete,
        })
    }

    /// Follows `is_a` and `part_of` upwards; each term is visited once, so
    /// cycles end.
    fn is_descendant(&self, accession: &str, ancestor: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = self
            .terms
            .get(accession)
            .map_or_else(Vec::new, |e| e.parents.clone());
        while let Some(term) = stack.pop() {
            if term == ancestor {
                return true;
            }
            if seen.insert(term) {
                stack.extend(self.terms.get(term).into_iter().flat_map(|e| &e.parents));
            }
        }
        false
    }

    fn version(&self, prefix: &str) -> Option<&str> {
        self.data_version.filter(|_| self.covers(prefix))
    }
}
