//! JSCH JSON table schemas, static column key mappings, and standard empty-table headers.

use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

/// The JSON form of a table column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Col {
    Text,
    Number,
    Integer,
    Param,
    Texts,
    Numbers,
    Integers,
    SpectraRefs,
}

/// A table: its JSON array, TSV prefixes, and stable columns `(tsv_name, json_key, form)`.
#[derive(Debug)]
pub(super) struct Table {
    pub(super) json: &'static str,
    pub(super) header: &'static str,
    pub(super) row: &'static str,
    pub(super) columns: &'static [(&'static str, &'static str, Col)],
    /// The abundance arrays: JSON name, the metadata element whose ids
    /// index it, the column name before `[n]`.
    pub(super) arrays: &'static [(&'static str, &'static str, &'static str)],
}

pub(super) const TABLES: [Table; 3] = [
    Table {
        json: "smallMoleculeSummary",
        header: "SMH",
        row: "SML",
        columns: &[
            ("SML_ID", "sml_id", Col::Integer),
            ("SMF_ID_REFS", "smf_id_refs", Col::Integers),
            ("database_identifier", "database_identifier", Col::Texts),
            ("chemical_formula", "chemical_formula", Col::Texts),
            ("smiles", "smiles", Col::Texts),
            ("inchi", "inchi", Col::Texts),
            ("chemical_name", "chemical_name", Col::Texts),
            ("uri", "uri", Col::Texts),
            (
                "theoretical_neutral_mass",
                "theoretical_neutral_mass",
                Col::Numbers,
            ),
            ("adduct_ions", "adduct_ions", Col::Texts),
            ("reliability", "reliability", Col::Text),
            (
                "best_id_confidence_measure",
                "best_id_confidence_measure",
                Col::Param,
            ),
            (
                "best_id_confidence_value",
                "best_id_confidence_value",
                Col::Number,
            ),
        ],
        arrays: &[
            ("abundance_assay", "assay", "abundance_assay"),
            (
                "abundance_study_variable",
                "study_variable",
                "abundance_study_variable",
            ),
            (
                "abundance_variation_study_variable",
                "study_variable",
                "abundance_variation_study_variable",
            ),
        ],
    },
    Table {
        json: "smallMoleculeFeature",
        header: "SFH",
        row: "SMF",
        columns: &[
            ("SMF_ID", "smf_id", Col::Integer),
            ("SME_ID_REFS", "sme_id_refs", Col::Integers),
            (
                "SME_ID_REF_ambiguity_code",
                "sme_id_ref_ambiguity_code",
                Col::Integer,
            ),
            ("adduct_ion", "adduct_ion", Col::Text),
            ("isotopomer", "isotopomer", Col::Param),
            ("exp_mass_to_charge", "exp_mass_to_charge", Col::Number),
            ("charge", "charge", Col::Integer),
            (
                "retention_time_in_seconds",
                "retention_time_in_seconds",
                Col::Number,
            ),
            (
                "retention_time_in_seconds_start",
                "retention_time_in_seconds_start",
                Col::Number,
            ),
            (
                "retention_time_in_seconds_end",
                "retention_time_in_seconds_end",
                Col::Number,
            ),
        ],
        arrays: &[("abundance_assay", "assay", "abundance_assay")],
    },
    Table {
        json: "smallMoleculeEvidence",
        header: "SEH",
        row: "SME",
        columns: &[
            ("SME_ID", "sme_id", Col::Integer),
            ("evidence_input_id", "evidence_input_id", Col::Text),
            ("database_identifier", "database_identifier", Col::Text),
            ("chemical_formula", "chemical_formula", Col::Text),
            ("smiles", "smiles", Col::Text),
            ("inchi", "inchi", Col::Text),
            ("chemical_name", "chemical_name", Col::Text),
            ("uri", "uri", Col::Text),
            ("derivatized_form", "derivatized_form", Col::Param),
            ("adduct_ion", "adduct_ion", Col::Text),
            ("exp_mass_to_charge", "exp_mass_to_charge", Col::Number),
            ("charge", "charge", Col::Integer),
            (
                "theoretical_mass_to_charge",
                "theoretical_mass_to_charge",
                Col::Number,
            ),
            ("spectra_ref", "spectra_ref", Col::SpectraRefs),
            ("identification_method", "identification_method", Col::Param),
            ("ms_level", "ms_level", Col::Param),
            ("rank", "rank", Col::Integer),
        ],
        arrays: &[(
            "id_confidence_measure",
            "id_confidence_measure",
            "id_confidence_measure",
        )],
    },
];

/// The header a table without rows reads back with: its stable columns in
/// the spec's order, with one array column per id of `ids(element)`
/// (SME's `id_confidence_measure[n]` before `rank`, abundance last).
pub(super) fn standard_header(table: &Table, ids: impl Fn(&str) -> Vec<u32>) -> Vec<String> {
    let split = table
        .columns
        .iter()
        .position(|(name, ..)| *name == "rank")
        .unwrap_or(table.columns.len());
    let (before, after) = table.columns.split_at(split);
    let mut out: Vec<String> = before
        .iter()
        .map(|(name, ..)| (*name).to_string())
        .collect();
    for (_, element, prefix) in table.arrays {
        out.extend(
            ids(element)
                .iter()
                .map(|id| alloc::format!("{prefix}[{id}]")),
        );
    }
    out.extend(after.iter().map(|(name, ..)| (*name).to_string()));
    out
}

/// The top-level keys of the `snake_case` dialect some tools write
/// (`MetaboScape`), which JSCH does not define (DECISIONS D-004).
pub(super) const SNAKE_DIALECT: &[&str] = &[
    "small_molecule_summary",
    "small_molecule_feature",
    "small_molecule_evidence",
];
