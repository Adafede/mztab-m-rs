//! Typed row structs for SML, SMF, and SME tables.

use alloc::vec::Vec;

use super::{List, SmeId, SmfId, SmlId, SpectraRef};
use crate::param::Param;
use crate::value::{Cell, Decimal, Raw, Text};
use crate::vocab::AmbiguityCode;

/// One SML row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmlRow<'a> {
    /// `SML_ID`.
    pub sml_id: Cell<'a, SmlId>,
    /// `SMF_ID_REFS`.
    pub smf_id_refs: Cell<'a, List<'a, SmfId>>,
    /// `database_identifier`.
    pub database_identifier: Cell<'a, List<'a, Text<'a>>>,
    /// `chemical_formula`.
    pub chemical_formula: Cell<'a, List<'a, Text<'a>>>,
    /// `smiles`.
    pub smiles: Cell<'a, List<'a, Text<'a>>>,
    /// `inchi`.
    pub inchi: Cell<'a, List<'a, Text<'a>>>,
    /// `chemical_name`.
    pub chemical_name: Cell<'a, List<'a, Text<'a>>>,
    /// `uri`.
    pub uri: Cell<'a, List<'a, Text<'a>>>,
    /// `theoretical_neutral_mass`.
    pub theoretical_neutral_mass: Cell<'a, List<'a, Decimal<'a>>>,
    /// `adduct_ions`.
    pub adduct_ions: Cell<'a, List<'a, Text<'a>>>,
    /// `reliability`.
    pub reliability: Cell<'a, Text<'a>>,
    /// `best_id_confidence_measure`.
    pub best_id_confidence_measure: Cell<'a, Param<'a>>,
    /// `best_id_confidence_value`.
    pub best_id_confidence_value: Cell<'a, Decimal<'a>>,
    /// The `abundance_*` columns, in header order.
    pub abundance: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
    /// Cells beyond the last header column, kept as written.
    pub extra: Vec<Raw<'a>>,
}

/// One SMF row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmfRow<'a> {
    /// `SMF_ID`.
    pub smf_id: Cell<'a, SmfId>,
    /// `SME_ID_REFS`.
    pub sme_id_refs: Cell<'a, List<'a, SmeId>>,
    /// `SME_ID_REF_ambiguity_code`.
    pub sme_id_ref_ambiguity_code: Cell<'a, AmbiguityCode>,
    /// `adduct_ion`.
    pub adduct_ion: Cell<'a, Text<'a>>,
    /// `isotopomer`.
    pub isotopomer: Cell<'a, Param<'a>>,
    /// `exp_mass_to_charge`.
    pub exp_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `charge`.
    pub charge: Cell<'a, i32>,
    /// `retention_time_in_seconds`.
    pub retention_time_in_seconds: Cell<'a, Decimal<'a>>,
    /// `retention_time_in_seconds_start`.
    pub retention_time_in_seconds_start: Cell<'a, Decimal<'a>>,
    /// `retention_time_in_seconds_end`.
    pub retention_time_in_seconds_end: Cell<'a, Decimal<'a>>,
    /// The `abundance_assay[n]` columns, in header order.
    pub abundance: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
    /// Cells beyond the last header column, kept as written.
    pub extra: Vec<Raw<'a>>,
}

/// One SME row.
///
/// Values of columns missing from the header are [`Cell::Null`] and are not
/// written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmeRow<'a> {
    /// `SME_ID`.
    pub sme_id: Cell<'a, SmeId>,
    /// `evidence_input_id`.
    pub evidence_input_id: Cell<'a, Text<'a>>,
    /// `database_identifier`.
    pub database_identifier: Cell<'a, Text<'a>>,
    /// `chemical_formula`.
    pub chemical_formula: Cell<'a, Text<'a>>,
    /// `smiles`.
    pub smiles: Cell<'a, Text<'a>>,
    /// `inchi`.
    pub inchi: Cell<'a, Text<'a>>,
    /// `chemical_name`.
    pub chemical_name: Cell<'a, Text<'a>>,
    /// `uri`.
    pub uri: Cell<'a, Text<'a>>,
    /// `derivatized_form`.
    pub derivatized_form: Cell<'a, Param<'a>>,
    /// `adduct_ion`.
    pub adduct_ion: Cell<'a, Text<'a>>,
    /// `exp_mass_to_charge`.
    pub exp_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `charge`.
    pub charge: Cell<'a, i32>,
    /// `theoretical_mass_to_charge`.
    pub theoretical_mass_to_charge: Cell<'a, Decimal<'a>>,
    /// `spectra_ref`.
    pub spectra_ref: Cell<'a, List<'a, SpectraRef<'a>>>,
    /// `identification_method`.
    pub identification_method: Cell<'a, Param<'a>>,
    /// `ms_level`.
    pub ms_level: Cell<'a, Param<'a>>,
    /// `rank`.
    pub rank: Cell<'a, u32>,
    /// The `id_confidence_measure[n]` columns, in header order.
    pub id_confidence: Vec<Cell<'a, Decimal<'a>>>,
    /// The `opt_` columns, in header order.
    pub opt: Vec<Cell<'a, Text<'a>>>,
    /// The unknown columns, in header order.
    pub unknown: Vec<Raw<'a>>,
    /// Cells beyond the last header column, kept as written.
    pub extra: Vec<Raw<'a>>,
}
