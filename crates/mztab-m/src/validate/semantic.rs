//! The semantic layer: CV declarations and usage, terms against a
//! [`TermLookup`], and the CV-mapping rules (DECISIONS D-067, D-068).

mod rules;
mod terms;
mod uses;

use super::report::Report;
use crate::cv::TermLookup;
use crate::diag::code::Code;
use crate::model::document::MzTab;
use crate::param::Param;

pub(super) fn check(mztab: &MzTab<'_>, term_lookup: &dyn TermLookup, report: &mut Report<'_, '_>) {
    let uses = uses::collect(mztab);
    let columns = uses::column_terms(mztab);
    terms::declarations(mztab, &uses, &columns, report);
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
                    terms::term(accession.as_str(), None, None),
                );
            }
            terms::term_checks(
                accession.as_str(),
                (
                    Some(name.as_str()),
                    value.as_ref().map(crate::param::text::ParamText::as_str),
                ),
                u.site,
                term_lookup,
                report,
            );
        }
    }
    for c in &columns {
        terms::term_checks(c.accession, (None, None), c.site, term_lookup, report);
    }
    rules::check_all(&uses, term_lookup, report);
    terms::versions(mztab, term_lookup, report);
}
