//! Collecting CV parameter sites and `opt_*_cv_*` column headers.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::cv::Target;
use crate::meta::key::MtdKey;
use crate::meta::subkey::{
    DatabaseKey, InstrumentKey, MsRunKey, SampleKey, SoftwareKey, StudyVariableGroupKey,
    StudyVariableKey,
};
use crate::model::document::MzTab;
use crate::model::meta_value::ValueRef;
use crate::model::table::{Column, Header, SmeColumn, SmfColumn, SmlColumn};
use crate::param::Param;
use crate::validate::report::Site;
use crate::validate::tables::position;
use crate::value::cell::Cell;
use crate::vocab::section::TableSection;

/// One CV parameter in the model, where it is, and the rule field it
/// fills (with its scope: an element or row).
pub(super) struct Use<'r, 'a> {
    pub(super) site: Site,
    pub(super) param: &'r Param<'a>,
    pub(super) slot: Option<(Target, u32)>,
}

/// An accession named in an `opt_` column header (`opt_global_cv_MS:1002217_x`).
pub(super) struct ColumnTerm<'r> {
    pub(super) site: Site,
    pub(super) accession: &'r str,
}

/// The CV parameters of the metadata and the tables.
pub(super) fn collect<'r, 'a>(mztab: &'r MzTab<'a>) -> Vec<Use<'r, 'a>> {
    let mut out = Vec::new();
    collect_metadata(mztab, &mut out);
    collect_tables(mztab, &mut out);
    out
}

fn collect_metadata<'r, 'a>(mztab: &'r MzTab<'a>, out: &mut Vec<Use<'r, 'a>>) {
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
}

fn collect_tables<'r, 'a>(mztab: &'r MzTab<'a>, out: &mut Vec<Use<'r, 'a>>) {
    if let Some(table) = &mztab.sml {
        let at = position(&table.header, &SmlColumn::BestIdConfidenceMeasure);
        for (i, row) in table.rows.iter().enumerate() {
            cell_use(
                out,
                &row.best_id_confidence_measure,
                (TableSection::Sml, i, at),
                Target::SmlBestIdConfidenceMeasure,
            );
        }
    }
    if let Some(table) = &mztab.smf {
        let at = position(&table.header, &SmfColumn::Isotopomer);
        for (i, row) in table.rows.iter().enumerate() {
            if let Cell::Value(param) = &row.isotopomer {
                out.push(Use {
                    site: Site::Cell(TableSection::Smf, i, at),
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
                cell_use(out, cell, (TableSection::Sme, i, at), target);
            }
        }
    }
}

fn cell_use<'r, 'a>(
    out: &mut Vec<Use<'r, 'a>>,
    cell: &'r Cell<'a, Param<'a>>,
    (section, row, at): (TableSection, usize, usize),
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
pub(super) fn column_terms<'r>(mztab: &'r MzTab<'_>) -> Vec<ColumnTerm<'r>> {
    let mut out = Vec::new();
    if let Some(t) = &mztab.sml {
        push_header_terms(TableSection::Sml, &t.header, &mut out);
    }
    if let Some(t) = &mztab.smf {
        push_header_terms(TableSection::Smf, &t.header, &mut out);
    }
    if let Some(t) = &mztab.sme {
        push_header_terms(TableSection::Sme, &t.header, &mut out);
    }
    out
}

fn push_header_terms<'r, S>(
    section: TableSection,
    header: &'r Header<'_, S>,
    out: &mut Vec<ColumnTerm<'r>>,
) {
    for (at, column) in header.columns.iter().enumerate() {
        let Column::Opt(opt) = column else { continue };
        let accession = opt
            .name
            .as_str()
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
