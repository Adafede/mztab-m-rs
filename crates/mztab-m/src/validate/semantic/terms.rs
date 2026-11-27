//! CV declaration, term-level checks, and declared CV version checks.

use alloc::borrow::Cow;
use alloc::collections::BTreeSet;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use super::uses::{ColumnTerm, Use};
use crate::cv::{self, TermLookup};
use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::id::CvId;
use crate::meta::key::MtdKey;
use crate::meta::subkey::CvKey;
use crate::model::document::MzTab;
use crate::param::Param;
use crate::validate::label;
use crate::validate::patterns::xsd_matches;
use crate::validate::report::{Report, Site};
use crate::value::cell::Cell;

pub(super) fn term(accession: &str, name: Option<&str>, rule: Option<&'static str>) -> Detail {
    Detail::Term {
        accession: Cow::Owned(accession.to_string()),
        name: name.map(|n| Cow::Owned(n.to_string())),
        rule,
    }
}

/// `X001` (once per undeclared label, at its first use) and `X002`.
pub(super) fn declarations(
    mztab: &MzTab<'_>,
    uses: &[Use<'_>],
    columns: &[ColumnTerm<'_>],
    report: &mut Report<'_, '_>,
) {
    let declared: Vec<(CvId, &str)> = mztab
        .metadata
        .cv
        .iter()
        .filter_map(|(id, cv)| Some((id, cv.label.as_ref()?.value()?.as_str())))
        .collect();
    let mut labels = BTreeSet::new();
    let param_labels = uses.iter().filter_map(|u| match u.param.as_ref() {
        Param::Cv { label, .. } => Some((label.as_str(), u.site)),
        Param::User { .. } => None,
    });
    let col_labels = columns.iter().map(|c| (cv::prefix(c.accession), c.site));
    for (cv_label, site) in param_labels.chain(col_labels) {
        if labels.insert(cv_label) && !declared.iter().any(|(_, l)| *l == cv_label) {
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
pub(super) fn term_checks(
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

/// `X010`: a declared version that differs from the lookup's.
pub(super) fn versions(mztab: &MzTab<'_>, terms: &dyn TermLookup, report: &mut Report<'_, '_>) {
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
