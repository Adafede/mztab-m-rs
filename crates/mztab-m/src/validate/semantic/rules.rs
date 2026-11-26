//! Target-indexed CV-mapping rule checking (`X007`–`X009`).

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use super::terms::term;
use super::uses::Use;
use crate::cv::{self, Logic, Rule, RuleTerm, Target, TermLookup};
use crate::diag::code::Code;
use crate::param::Param;
use crate::validate::report::{Report, Site};

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

struct RuleUse<'a> {
    site: Site,
    scope: u32,
    accession: &'a str,
}

/// Evaluates all CV-mapping rules against `uses` in canonical rule order,
/// indexing `uses` by `Target` once to avoid scanning empty targets.
pub(super) fn check_all(uses: &[Use<'_>], terms: &dyn TermLookup, report: &mut Report<'_, '_>) {
    let mut by_target: BTreeMap<Target, Vec<RuleUse<'_>>> = BTreeMap::new();
    for u in uses {
        if let (Some((target, scope)), Param::Cv { accession, .. }) = (u.slot, u.param.as_ref()) {
            by_target.entry(target).or_default().push(RuleUse {
                site: u.site,
                scope,
                accession: accession.as_str(),
            });
        }
    }
    if by_target.is_empty() {
        return;
    }
    for rule in cv::rules() {
        if let Some(target_uses) = by_target.get(&rule.target) {
            check_rule(rule, target_uses, terms, report);
        }
    }
}

/// `X007`-`X009` for one rule. User parameters are allowed in every field
/// (format specification, "Params"), so only CV parameters are checked;
/// a value the lookup cannot judge is skipped.
fn check_rule(
    rule: &Rule,
    uses: &[RuleUse<'_>],
    terms: &dyn TermLookup,
    report: &mut Report<'_, '_>,
) {
    let mut seen: BTreeSet<(u32, &str)> = BTreeSet::new();
    for u in uses {
        let mut hits = 0_usize;
        let mut any_unknown = false;
        let mut repeatable = false;
        for t in rule.terms {
            match matches(t, u.accession, terms) {
                Match::Yes => {
                    hits += 1;
                    repeatable |= t.repeatable;
                }
                Match::Unknown => any_unknown = true,
                Match::No => {}
            }
        }
        let detail = || term(u.accession, None, Some(rule.id));
        if hits == 0 {
            if !any_unknown {
                let severity = rule.requirement.severity();
                report.add_at(Code::CvRule, severity, u.site, detail());
            }
            continue;
        }
        if rule.logic == Logic::Xor && hits > 1 {
            report.add(Code::CvRuleXor, u.site, detail());
        }
        if !seen.insert((u.scope, u.accession)) && !repeatable {
            report.add(Code::CvRuleRepeat, u.site, detail());
        }
    }
}
