//! Lenient OBO 1.4 stanza and tag parser.

use alloc::vec::Vec;

use mztab_m::cv::prefix;

use super::Ontology;

/// One `[Term]` stanza.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Entry<'a> {
    pub(super) name: &'a str,
    pub(super) obsolete: bool,
    /// The local name of the `xsd:` value type (`relationship:
    /// has_value_type`, or the older `xref: value-type:`).
    pub(super) value_type: Option<&'a str>,
    /// Exact synonyms (`synonym: "CID" EXACT []`).
    pub(super) synonyms: Vec<&'a str>,
    /// `is_a` and `relationship: part_of` targets.
    pub(super) parents: Vec<&'a str>,
}

pub(super) fn parse_ontology(text: &str) -> Ontology<'_> {
    let mut ontology = Ontology::default();
    let mut stanza: Option<(Option<&str>, Entry<'_>)> = None;
    let mut in_header = true;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            close(&mut ontology, stanza.take());
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
            parse_header_tag(&mut ontology, tag, value);
        } else if let Some((id, entry)) = stanza.as_mut() {
            parse_term_tag(id, entry, tag, value);
        }
    }
    close(&mut ontology, stanza);
    ontology
}

fn parse_header_tag<'a>(ontology: &mut Ontology<'a>, tag: &str, value: &'a str) {
    match tag {
        "data-version" => ontology.data_version = Some(value),
        "ontology" => ontology.namespace = Some(value),
        _ => {}
    }
}

fn parse_term_tag<'a>(id: &mut Option<&'a str>, entry: &mut Entry<'a>, tag: &str, value: &'a str) {
    match tag {
        "id" => *id = Some(target(value)),
        "name" => entry.name = value,
        "synonym" => {
            if let Some(synonym) = exact_synonym(value) {
                entry.synonyms.push(synonym);
            }
        }
        "is_obsolete" => entry.obsolete = value == "true",
        "xref" => {
            if let Some(kind) = value_type(value) {
                entry.value_type = Some(kind);
            }
        }
        "is_a" => entry.parents.push(target(value)),
        "relationship" => {
            if let Some(parent) = value.strip_prefix("part_of ") {
                entry.parents.push(target(parent));
            } else if let Some(kind) = value
                .strip_prefix("has_value_type ")
                .and_then(|t| target(t).strip_prefix("xsd:"))
            {
                // PSI-MS since 4.1: `relationship: has_value_type xsd:int`.
                entry.value_type = Some(kind);
            }
        }
        _ => {}
    }
}

fn close<'a>(ontology: &mut Ontology<'a>, stanza: Option<(Option<&'a str>, Entry<'a>)>) {
    if let Some((Some(id), entry)) = stanza {
        let own = ontology
            .namespace
            .is_none_or(|ns| ns.eq_ignore_ascii_case(prefix(id)));
        if own {
            ontology.covered.insert(prefix(id));
        }
        ontology.terms.entry(id).or_insert(entry);
    }
}

/// The text of an exact synonym: `"CID" EXACT []` gives `CID`.
fn exact_synonym(value: &str) -> Option<&str> {
    let rest = value.strip_prefix('"')?;
    let (text, scope) = rest.split_once('"')?;
    scope.trim_start().starts_with("EXACT").then_some(text)
}

/// The local name in `value-type:xsd\:int "…"` (`int`): the older PSI-MS
/// form, as an `xref`.
fn value_type(xref: &str) -> Option<&str> {
    let rest = xref.strip_prefix("value-type:")?;
    let rest = rest
        .strip_prefix("xsd\\:")
        .or_else(|| rest.strip_prefix("xsd:"))?;
    rest.split([' ', '"']).next().filter(|s| !s.is_empty())
}

/// The identifier in a tag value: before any `!` comment or `{`
/// qualifiers.
fn target(value: &str) -> &str {
    value.split(['!', '{']).next().map_or(value, str::trim)
}
