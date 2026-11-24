//! Table header shape checks: emptiness, mandatory columns, canonical order,
//! `opt_` placement, and metadata cross-references.

use alloc::vec::Vec;

use crate::diag::{Code, Detail, Severity};
use crate::id::IdConfidenceMeasureId;
use crate::model::{AbundanceColumn, Column, Header, MzTab, OptTarget, Table};
use crate::validate::{Report, Site, label};
use crate::vocab::{Profile, RuleSet, TableSection};

/// A header column without its table's stable-column type.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A stable column, by canonical rank.
    Stable(usize),
    Abundance(AbundanceColumn),
    IdConfidence(IdConfidenceMeasureId),
    Opt(OptTarget),
    Unknown,
}

impl Kind {
    /// The canonical rank, for order checks; `None` for columns whose place
    /// is not specified (`opt_`, unknown).
    const fn rank(self) -> Option<usize> {
        match self {
            Self::Stable(rank) => Some(rank),
            Self::Abundance(AbundanceColumn::Assay(_)) => Some(1000),
            Self::Abundance(AbundanceColumn::StudyVariable(_)) => Some(1001),
            Self::Abundance(AbundanceColumn::VariationStudyVariable(_)) => Some(1002),
            // SME: after `ms_level`, before `rank` (field reference order).
            Self::IdConfidence(_) => Some(155),
            Self::Opt(_) | Self::Unknown => None,
        }
    }
}

/// The kinds of a header's columns.
fn kinds<S: Copy + Ord>(header: &Header<'_, S>, stable: &[S]) -> Vec<Kind> {
    header
        .columns
        .iter()
        .map(|column| match column {
            Column::Stable(s) => {
                Kind::Stable(stable.iter().position(|x| x == s).unwrap_or_default() * 10)
            }
            Column::Abundance(a) => Kind::Abundance(*a),
            Column::IdConfidence(i) => Kind::IdConfidence(*i),
            Column::Opt(o) => Kind::Opt(o.target),
            Column::Unknown(_) => Kind::Unknown,
        })
        .collect()
}

struct ShapeCtx<'r, 'a> {
    mztab: &'r MzTab<'a>,
    declared: Option<Profile>,
    section: TableSection,
    empty: bool,
}

/// Columns: emptiness, mandatory columns, order, `opt_` position, and
/// references from column names to metadata.
pub(super) fn table_shape<S: Copy + Ord + core::fmt::Display, R>(
    mztab: &MzTab<'_>,
    declared: Option<Profile>,
    section: TableSection,
    table: &Table<'_, S, R>,
    stable: &[S],
    report: &mut Report<'_, '_>,
) {
    for s in stable {
        if !table
            .header
            .columns
            .iter()
            .any(|c| matches!(c, Column::Stable(x) if x == s))
        {
            report.add(Code::MissingColumn, Site::Header(section), label(s));
        }
    }
    let header_kinds = kinds(&table.header, stable);
    let ctx = ShapeCtx {
        mztab,
        declared,
        section,
        empty: table.rows.is_empty(),
    };
    check_shape(&ctx, &header_kinds, report);
}

fn check_shape(ctx: &ShapeCtx<'_, '_>, header_kinds: &[Kind], report: &mut Report<'_, '_>) {
    if report.rule_set == RuleSet::V2_1 && ctx.empty {
        // DECISIONS D-019: an error when the profile is declared, a
        // warning when it is inferred.
        let severity = if ctx.declared.is_some() {
            Severity::Error
        } else {
            Severity::Warning
        };
        report.add_at(
            Code::EmptyTable,
            severity,
            Site::Header(ctx.section),
            Detail::None,
        );
    }
    for (kind, name) in expected_columns(ctx.mztab, ctx.section) {
        let present = header_kinds.iter().any(|k| match (k, kind) {
            (Kind::Abundance(a), Kind::Abundance(b)) => *a == b,
            (Kind::IdConfidence(a), Kind::IdConfidence(b)) => *a == b,
            _ => false,
        });
        if !present {
            report.add(Code::MissingColumn, Site::Header(ctx.section), name);
        }
    }
    check_order_and_refs(ctx, header_kinds, report);
}

fn expected_columns(mztab: &MzTab<'_>, section: TableSection) -> Vec<(Kind, Detail)> {
    let m = &mztab.metadata;
    let mut expected = Vec::new();
    let abundance = |column| (Kind::Abundance(column), label(column));
    if matches!(section, TableSection::Sml | TableSection::Smf) {
        expected.extend(
            m.assay
                .keys()
                .map(|id| abundance(AbundanceColumn::Assay(id))),
        );
    }
    if section == TableSection::Sml {
        expected.extend(
            m.study_variable
                .keys()
                .map(|id| abundance(AbundanceColumn::StudyVariable(id))),
        );
        expected.extend(
            m.study_variable
                .keys()
                .map(|id| abundance(AbundanceColumn::VariationStudyVariable(id))),
        );
    }
    if section == TableSection::Sme {
        expected.extend(
            m.id_confidence_measure
                .keys()
                .map(|id| (Kind::IdConfidence(id), label(id))),
        );
    }
    expected
}

fn check_order_and_refs(
    ctx: &ShapeCtx<'_, '_>,
    header_kinds: &[Kind],
    report: &mut Report<'_, '_>,
) {
    let m = &ctx.mztab.metadata;
    let mut highest = None;
    let mut out_of_order = false;
    let mut seen_opt = false;
    let mut opt_misplaced = false;
    for (i, kind) in header_kinds.iter().enumerate() {
        if let Some(r) = kind.rank() {
            if highest.is_some_and(|h| r < h) && !out_of_order {
                out_of_order = true;
                report.add(
                    Code::ColumnOrder,
                    Site::Column(ctx.section, i),
                    Detail::None,
                );
            }
            highest = highest.max(Some(r));
            if seen_opt && !opt_misplaced {
                opt_misplaced = true;
                report.add(
                    Code::OptColumnPosition,
                    Site::Column(ctx.section, i),
                    Detail::None,
                );
            }
        }
        seen_opt |= matches!(kind, Kind::Opt(_));
        let dangling = match *kind {
            Kind::Abundance(AbundanceColumn::Assay(id)) | Kind::Opt(OptTarget::Assay(id)) => {
                !m.assay.contains_key(id)
            }
            Kind::Abundance(
                AbundanceColumn::StudyVariable(id) | AbundanceColumn::VariationStudyVariable(id),
            )
            | Kind::Opt(OptTarget::StudyVariable(id)) => !m.study_variable.contains_key(id),
            Kind::IdConfidence(id) => !m.id_confidence_measure.contains_key(id),
            Kind::Opt(OptTarget::MsRun(id)) => !m.ms_run.contains_key(id),
            Kind::Opt(OptTarget::Global) | Kind::Stable(_) | Kind::Unknown => false,
        };
        if dangling {
            report.add(
                Code::DanglingReference,
                Site::Column(ctx.section, i),
                Detail::None,
            );
        }
    }
}
