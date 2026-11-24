//! [`IntoOwned`] implementations for model and metadata structs.

use super::IntoOwned;
use crate::model::{
    Assay, ColUnit, Contact, Cv, Database, Instrument, Metadata, MsRun, MtdEntry, MzTab, OptColumn,
    Protocol, Sample, SmeRow, SmfRow, SmlRow, Software, SourceLayout, SourceLine, SpectraRef,
    StudyVariable, StudyVariableGroup, Verbatim,
};
use crate::read::Parsed;
use crate::vocab::PublicationItem;

/// Implements [`IntoOwned`] for a struct by converting every field. A field
/// missing from the list is a compile error, so the impls stay complete.
macro_rules! owned_struct {
    ($($name:ident { $($field:ident),+ $(,)? })+) => {$(
        impl IntoOwned for $name<'_> {
            type Owned = $name<'static>;

            fn into_owned(self) -> $name<'static> {
                $name { $($field: self.$field.into_owned()),+ }
            }
        }
    )+};
}

owned_struct! {
    PublicationItem { kind, accession }
    ColUnit { column, param }
    SpectraRef { ms_run, reference }
    OptColumn { target, name }
    MtdEntry { key, value, trailing, typed }
    SourceLayout { bom, lines }
    SourceLine { content, ending, verbatim }
    Verbatim { text, fingerprint }
    Parsed { mztab, findings }
    MzTab { metadata, sml, smf, sme, layout }
    Instrument { name, source, analyzer, detector }
    Software { param, setting }
    Contact { name, affiliation, email, orcid }
    Sample { name, species, tissue, cell_type, disease, description, custom }
    MsRun {
        location, instrument_ref, format, id_format, fragmentation_method, scan_polarity, hash,
        hash_method, parameters, usi_identifier,
    }
    Assay { name, custom, external_uri, sample_ref, ms_run_ref, protocol_refs, parameters }
    StudyVariable {
        name, assay_refs, ms_run_refs, description, average_function, variation_function, factors,
    }
    StudyVariableGroup { param, description, type_, datatype, unit, study_variable_refs }
    Protocol { name, type_, description, parameters }
    Cv { label, full_name, version, uri }
    Database { param, prefix, version, uri }
    Metadata {
        version, id, profile, title, description, sample_processing, instrument, software,
        publication, contact, uri, external_study_uri, quantification_method, sample, ms_run,
        assay, study_variable, study_variable_group, protocol, custom, cv, database,
        derivatization_agent, small_molecule_quantification_unit,
        small_molecule_feature_quantification_unit, small_molecule_identification_reliability,
        id_confidence_measure, colunit_small_molecule, colunit_small_molecule_feature,
        colunit_small_molecule_evidence, unknown,
    }
    SmlRow {
        sml_id, smf_id_refs, database_identifier, chemical_formula, smiles, inchi, chemical_name,
        uri, theoretical_neutral_mass, adduct_ions, reliability, best_id_confidence_measure,
        best_id_confidence_value, abundance, opt, unknown, extra,
    }
    SmfRow {
        smf_id, sme_id_refs, sme_id_ref_ambiguity_code, adduct_ion, isotopomer,
        exp_mass_to_charge, charge, retention_time_in_seconds, retention_time_in_seconds_start,
        retention_time_in_seconds_end, abundance, opt, unknown, extra,
    }
    SmeRow {
        sme_id, evidence_input_id, database_identifier, chemical_formula, smiles, inchi,
        chemical_name, uri, derivatized_form, adduct_ion, exp_mass_to_charge, charge,
        theoretical_mass_to_charge, spectra_ref, identification_method, ms_level, rank,
        id_confidence, opt, unknown, extra,
    }
}
