//! The semantic layer: CV declarations and usage, terms against a
//! [`TermLookup`], and the CV-mapping rules (DECISIONS D-067, D-068).

use alloc::borrow::Cow;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::ToString as _;
use alloc::vec::Vec;

use super::tables::position;
use super::{Report, Site, label};
use crate::cv::{self, Logic, Rule, RuleTerm, Target, TermLookup};
use crate::diag::{Code, Detail};
use crate::meta::{
    CvKey, DatabaseKey, InstrumentKey, MsRunKey, MtdKey, SampleKey, SoftwareKey,
    StudyVariableGroupKey, StudyVariableKey, ValueRef,
};
use crate::model::{Column, Header, MzTab, SmeColumn, SmlColumn};
use crate::param::Param;
use crate::value::Cell;
use crate::vocab::Section;

/// One CV parameter in the model, where it is, and the rule field it
/// fills (with its scope: an element or row).
struct Use<'r, 'a> {
    site: Site,
    param: &'r Param<'a>,
    slot: Option<(Target, u32)>,
}

/// An accession named in an `opt_` column header (`opt_global_cv_MS:1002217_x`).
struct ColumnTerm<'r> {
    site: Site,
    accession: &'r str,
}

pub(super) fn check(mztab: &MzTab<'_>, terms: &dyn TermLookup, report: &mut Report<'_, '_>) {
    let uses = uses(mztab);
    let columns = column_terms(mztab);
    declarations(mztab, &uses, &columns, report);
    for u in &uses {
        if let Param::Cv {
            accession,
            name,
            value,
            ..
        } = u.param
        {
            if accession.prefix().is_none() {
                report.add(
                    Code::AccessionPrefix,
                    u.site,
                    term(accession.as_str(), None, None),
                );
            }
            term_checks(
                accession.as_str(),
                (
                    Some(name.as_str()),
                    value.as_ref().map(crate::param::ParamText::as_str),
                ),
                u.site,
                terms,
                report,
            );
        }
    }
    for c in &columns {
        term_checks(c.accession, (None, None), c.site, terms, report);
    }
    for rule in cv::rules() {
        check_rule(rule, &uses, terms, report);
    }
    versions(mztab, terms, report);
}

fn term(accession: &str, name: Option<&str>, rule: Option<&'static str>) -> Detail {
    Detail::Term {
        accession: Cow::Owned(accession.to_string()),
        name: name.map(|n| Cow::Owned(n.to_string())),
        rule,
    }
}

/// The CV parameters of the metadata and the tables.
fn uses<'r, 'a>(mztab: &'r MzTab<'a>) -> Vec<Use<'r, 'a>> {
    let mut out = Vec::new();
    let mut nth: BTreeMap<MtdKey, usize> = BTreeMap::new();
    for (key, value) in mztab.metadata.entries() {
        let slot = slot(key);
        match value {
            ValueRef::Param(Cell::Value(param)) => out.push(Use {
                site: Site::Value(key),
                param,
                slot,
            }),
            ValueRef::ParamList(Cell::Value(list)) => {
                out.extend(list.iter().filter_map(Cell::value).map(|param| Use {
                    site: Site::Value(key),
                    param,
                    slot,
                }));
            }
            ValueRef::ColUnit(cell) => {
                let n = nth.entry(key).or_default();
                if let Cell::Value(unit) = cell {
                    out.push(Use {
                        site: Site::Nth(key, *n),
                        param: &unit.param,
                        slot,
                    });
                }
                *n += 1;
            }
            _ => {}
        }
    }
    if let Some(table) = &mztab.sml {
        let at = position(&table.header, &SmlColumn::BestIdConfidenceMeasure);
        for (i, row) in table.rows.iter().enumerate() {
            cell_use(
                &mut out,
                &row.best_id_confidence_measure,
                (Section::Sml, i, at),
                Target::SmlBestIdConfidenceMeasure,
            );
        }
    }
    if let Some(table) = &mztab.smf {
        let at = position(&table.header, &crate::model::SmfColumn::Isotopomer);
        for (i, row) in table.rows.iter().enumerate() {
            if let Cell::Value(param) = &row.isotopomer {
                out.push(Use {
                    site: Site::Cell(Section::Smf, i, at),
                    param,
                    slot: None,
                });
            }
        }
    }
    if let Some(table) = &mztab.sme {
        let pos = |c| position(&table.header, &c);
        let cols = [
            (pos(SmeColumn::DerivatizedForm), Target::SmeDerivatizedForm),
            (
                pos(SmeColumn::IdentificationMethod),
                Target::SmeIdentificationMethod,
            ),
            (pos(SmeColumn::MsLevel), Target::SmeMsLevel),
        ];
        for (i, row) in table.rows.iter().enumerate() {
            let cells = [
                &row.derivatized_form,
                &row.identification_method,
                &row.ms_level,
            ];
            for (cell, (at, target)) in cells.into_iter().zip(cols) {
                cell_use(&mut out, cell, (Section::Sme, i, at), target);
            }
        }
    }
    out
}

fn cell_use<'r, 'a>(
    out: &mut Vec<Use<'r, 'a>>,
    cell: &'r Cell<'a, Param<'a>>,
    (section, row, at): (Section, usize, usize),
    target: Target,
) {
    if let Cell::Value(param) = cell {
        let scope = u32::try_from(row).unwrap_or(u32::MAX);
        out.push(Use {
            site: Site::Cell(section, row, at),
            param,
            slot: Some((target, scope)),
        });
    }
}

/// The rule field a metadata key fills, and its scope (the element's
/// index, or 0 for the metadata as a whole).
const fn slot(key: MtdKey) -> Option<(Target, u32)> {
    use Target as T;
    Some(match key {
        MtdKey::MsRun(id, sub) => (
            match sub {
                MsRunKey::Format => T::MsRunFormat,
                MsRunKey::IdFormat => T::MsRunIdFormat,
                MsRunKey::FragmentationMethod(_) => T::MsRunFragmentationMethod,
                MsRunKey::ScanPolarity(_) => T::MsRunScanPolarity,
                MsRunKey::HashMethod => T::MsRunHashMethod,
                _ => return None,
            },
            id.get(),
        ),
        MtdKey::SampleProcessing(n) => (T::SampleProcessing, n.get()),
        MtdKey::Instrument(id, sub) => (
            match sub {
                InstrumentKey::Name => T::InstrumentName,
                InstrumentKey::Source => T::InstrumentSource,
                InstrumentKey::Analyzer(_) => T::InstrumentAnalyzer,
                InstrumentKey::Detector => T::InstrumentDetector,
            },
            id.get(),
        ),
        MtdKey::Software(id, SoftwareKey::Software) => (T::Software, id.get()),
        MtdKey::QuantificationMethod => (T::QuantificationMethod, 0),
        MtdKey::Sample(id, sub) => (
            match sub {
                SampleKey::Species(_) => T::SampleSpecies,
                SampleKey::Tissue(_) => T::SampleTissue,
                SampleKey::CellType(_) => T::SampleCellType,
                SampleKey::Disease(_) => T::SampleDisease,
                _ => return None,
            },
            id.get(),
        ),
        MtdKey::Database(id, DatabaseKey::Database) => (T::Database, id.get()),
        MtdKey::StudyVariable(id, sub) => (
            match sub {
                StudyVariableKey::AverageFunction => T::StudyVariableAverageFunction,
                StudyVariableKey::VariationFunction => T::StudyVariableVariationFunction,
                _ => return None,
            },
            id.get(),
        ),
        MtdKey::StudyVariableGroup(id, sub) => (
            match sub {
                StudyVariableGroupKey::Group => T::StudyVariableGroup,
                StudyVariableGroupKey::Type => T::StudyVariableGroupType,
                StudyVariableGroupKey::Unit => T::StudyVariableGroupUnit,
                _ => return None,
            },
            id.get(),
        ),
        MtdKey::DerivatizationAgent(_) => (T::DerivatizationAgent, 0),
        MtdKey::SmallMoleculeQuantificationUnit => (T::SmallMoleculeQuantificationUnit, 0),
        MtdKey::SmallMoleculeFeatureQuantificationUnit => {
            (T::SmallMoleculeFeatureQuantificationUnit, 0)
        }
        MtdKey::SmallMoleculeIdentificationReliability => {
            (T::SmallMoleculeIdentificationReliability, 0)
        }
        MtdKey::IdConfidenceMeasure(_) => (T::IdConfidenceMeasure, 0),
        _ => return None,
    })
}

/// Accessions named by `opt_{target}_cv_{accession}_{name}` headers.
fn column_terms<'r>(mztab: &'r MzTab<'_>) -> Vec<ColumnTerm<'r>> {
    let mut out = Vec::new();
    let sml = mztab.sml.as_ref().map(|t| (Section::Sml, opts(&t.header)));
    let smf = mztab.smf.as_ref().map(|t| (Section::Smf, opts(&t.header)));
    let sme = mztab.sme.as_ref().map(|t| (Section::Sme, opts(&t.header)));
    for (section, names) in [sml, smf, sme].into_iter().flatten() {
        for (at, name) in names {
            let accession = name
                .strip_prefix("cv_")
                .and_then(|rest| rest.split_once('_'))
                .map(|(accession, _)| accession);
            if let Some(accession) = accession {
                out.push(ColumnTerm {
                    site: Site::Column(section, at),
                    accession,
                });
            }
        }
    }
    out
}

/// The `opt_` columns of a header: position and name after the target.
fn opts<'r, S>(header: &'r Header<'_, S>) -> Vec<(usize, &'r str)> {
    header
        .columns
        .iter()
        .enumerate()
        .filter_map(|(at, column)| match column {
            Column::Opt(opt) => Some((at, opt.name.as_str())),
            _ => None,
        })
        .collect()
}

/// `X001` (once per undeclared label, at its first use) and `X002`.
fn declarations(
    mztab: &MzTab<'_>,
    uses: &[Use<'_, '_>],
    columns: &[ColumnTerm<'_>],
    report: &mut Report<'_, '_>,
) {
    let declared: Vec<(crate::id::CvId, &str)> = mztab
        .metadata
        .cv
        .iter()
        .filter_map(|(id, cv)| Some((id, cv.label.as_ref()?.value()?.as_str())))
        .collect();
    let mut labels = BTreeSet::new();
    let param_labels = uses.iter().filter_map(|u| match u.param {
        Param::Cv { label, .. } => Some((label.as_str(), u.site)),
        Param::User { .. } => None,
    });
    let col_labels = columns.iter().map(|c| (cv::prefix(c.accession), c.site));
    for (cv_label, site) in param_labels.chain(col_labels) {
        let known = declared.iter().any(|(_, l)| *l == cv_label);
        if labels.insert(cv_label) && !known {
            report.add(Code::CvUndeclared, site, label(cv_label));
        }
    }
    for (id, cv_label) in declared {
        if !labels.contains(cv_label) {
            report.add(
                Code::CvUnused,
                Site::Value(MtdKey::Cv(id, CvKey::Label)),
                label(cv_label),
            );
        }
    }
}

/// `X004`-`X006` and `X011` for one accession (with the parameter's name
/// and value, if any), if the lookup covers it.
fn term_checks(
    accession: &str,
    (name, value): (Option<&str>, Option<&str>),
    site: Site,
    terms: &dyn TermLookup,
    report: &mut Report<'_, '_>,
) {
    if !terms.covers(cv::prefix(accession)) {
        return;
    }
    let Some(found) = terms.term(accession) else {
        report.add(Code::UnknownTerm, site, term(accession, None, None));
        return;
    };
    let known = |n: &str| {
        n.eq_ignore_ascii_case(found.name)
            || found.synonyms.iter().any(|s| n.eq_ignore_ascii_case(s))
    };
    if name.is_some_and(|n| !known(n)) {
        report.add(
            Code::TermName,
            site,
            term(accession, Some(found.name), None),
        );
    }
    if found.obsolete {
        report.add(Code::ObsoleteTerm, site, term(accession, None, None));
    }
    // A value, when given, has the term's type; a missing value is not
    // reported (mzTab names score types without values, D-075).
    if let (Some(kind), Some(value)) = (found.value_type, value)
        && xsd_matches(kind, value) == Some(false)
    {
        report.add(
            Code::TermValueType,
            site,
            label(alloc::format!("`{value}` is not xsd:{kind}")),
        );
    }
}

/// Whether `value` is a lexical value of XML Schema type `kind` (by local
/// name); `None` for a type not checked.
fn xsd_matches(kind: &str, value: &str) -> Option<bool> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let signed = |s: &str| digits(s.strip_prefix(['-', '+']).unwrap_or(s));
    Some(match kind {
        "int" | "integer" | "long" | "short" => signed(value),
        "nonNegativeInteger" | "unsignedInt" => digits(value.strip_prefix('+').unwrap_or(value)),
        "positiveInteger" => {
            digits(value.strip_prefix('+').unwrap_or(value))
                && value.bytes().any(|b| (b'1'..=b'9').contains(&b))
        }
        "float" | "double" | "decimal" => {
            matches!(value, "NaN" | "INF" | "-INF") || value.parse::<f64>().is_ok()
        }
        "boolean" => matches!(value, "true" | "false" | "1" | "0"),
        // `string`, `anyURI`, dates and the rest: not checked.
        _ => return None,
    })
}

/// How one rule term judges an accession.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Match {
    Yes,
    No,
    Unknown,
}

fn matches(rule_term: &RuleTerm, accession: &str, terms: &dyn TermLookup) -> Match {
    if rule_term.use_term && accession == rule_term.accession {
        Match::Yes
    } else if !rule_term.allow_children {
        Match::No
    } else if !terms.covers(cv::prefix(accession)) {
        Match::Unknown
    } else if terms.is_descendant(accession, rule_term.accession) {
        Match::Yes
    } else {
        Match::No
    }
}

/// `X007`-`X009` for one rule. User parameters are allowed in every field
/// (format specification, "Params"), so only CV parameters are checked;
/// a value the lookup cannot judge is skipped.
fn check_rule(
    rule: &Rule,
    uses: &[Use<'_, '_>],
    terms: &dyn TermLookup,
    report: &mut Report<'_, '_>,
) {
    let mut seen: BTreeSet<(u32, &str)> = BTreeSet::new();
    for u in uses {
        let Some((target, scope)) = u.slot else {
            continue;
        };
        let Param::Cv { accession, .. } = u.param else {
            continue;
        };
        if target != rule.target {
            continue;
        }
        let accession = accession.as_str();
        let judged: Vec<(Match, &RuleTerm)> = rule
            .terms
            .iter()
            .map(|t| (matches(t, accession, terms), t))
            .collect();
        let hits: Vec<&RuleTerm> = judged
            .iter()
            .filter(|(m, _)| *m == Match::Yes)
            .map(|(_, t)| *t)
            .collect();
        let detail = || term(accession, None, Some(rule.id));
        if hits.is_empty() {
            if judged.iter().all(|(m, _)| *m == Match::No) {
                let severity = rule.requirement.severity();
                report.add_at(Code::CvRule, severity, u.site, detail());
            }
            continue;
        }
        if rule.logic == Logic::Xor && hits.len() > 1 {
            report.add(Code::CvRuleXor, u.site, detail());
        }
        let repeatable = hits.iter().any(|t| t.repeatable);
        if !seen.insert((scope, accession)) && !repeatable {
            report.add(Code::CvRuleRepeat, u.site, detail());
        }
    }
}

/// `X010`: a declared version that differs from the lookup's.
fn versions(mztab: &MzTab<'_>, terms: &dyn TermLookup, report: &mut Report<'_, '_>) {
    for (id, cv) in mztab.metadata.cv.iter() {
        let cv_label = cv.label.as_ref().and_then(Cell::value);
        let declared = cv.version.as_ref().and_then(Cell::value);
        let (Some(cv_label), Some(declared)) = (cv_label, declared) else {
            continue;
        };
        let Some(checked) = terms.version(cv_label.as_str()) else {
            continue;
        };
        if checked != declared.as_str() {
            report.add(
                Code::CvVersion,
                Site::Value(MtdKey::Cv(id, CvKey::Version)),
                label(checked),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::xsd_matches;

    #[test]
    fn xml_schema_types() {
        let cases = [
            ("int", "-3", Some(true)),
            ("integer", "x", Some(false)),
            ("nonNegativeInteger", "+0", Some(true)),
            ("nonNegativeInteger", "-1", Some(false)),
            ("positiveInteger", "0", Some(false)),
            ("positiveInteger", "07", Some(true)),
            ("double", "1.5e3", Some(true)),
            ("float", "INF", Some(true)),
            ("decimal", "one", Some(false)),
            ("boolean", "1", Some(true)),
            ("boolean", "yes", Some(false)),
            ("string", "anything", None),
            ("QName", "x", None),
        ];
        for (kind, value, expected) in cases {
            assert_eq!(xsd_matches(kind, value), expected, "{kind} {value}");
        }
    }
}
