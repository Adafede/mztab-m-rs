//! Generic [`Table`], [`Header`], [`Column`] definitions, and row IDs.

use alloc::vec::Vec;
use core::fmt;

use crate::id::{AssayId, IdConfidenceMeasureId, MsRunId, StudyVariableId};
use crate::value::{Raw, Text};

/// A table: its header and its rows.
///
/// `S` is the table's stable-column enum and `R` its row type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table<'a, S, R> {
    /// The header line, in source order.
    pub header: Header<'a, S>,
    /// The rows, in source order.
    pub rows: Vec<R>,
}

impl<S, R> Default for Table<'_, S, R> {
    /// A table with no columns and no rows.
    fn default() -> Self {
        Self {
            header: Header {
                columns: Vec::new(),
            },
            rows: Vec::new(),
        }
    }
}

/// A table header: the ordered list of columns after the line prefix.
///
/// Source order is kept (DECISIONS D-010). A row stores its stable columns
/// by name, and every other column in the vector of its kind, in the order
/// those columns appear here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header<'a, S> {
    /// The columns, in source order.
    pub columns: Vec<Column<'a, S>>,
}

/// One column of a header.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Column<'a, S> {
    /// A column defined by the field reference, such as `SML_ID`.
    Stable(S),
    /// An `abundance_*` column.
    Abundance(AbundanceColumn),
    /// SME `id_confidence_measure[n]`.
    IdConfidence(IdConfidenceMeasureId),
    /// An `opt_` column.
    Opt(OptColumn<'a>),
    /// A column this crate does not recognise, kept as written.
    Unknown(Raw<'a>),
}

/// The kind of an `abundance_*` column and the element it refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbundanceColumn {
    /// `abundance_assay[n]` (SML and SMF).
    Assay(AssayId),
    /// `abundance_study_variable[n]` (SML).
    StudyVariable(StudyVariableId),
    /// `abundance_variation_study_variable[n]` (SML).
    VariationStudyVariable(StudyVariableId),
}

impl fmt::Display for AbundanceColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assay(id) => write!(f, "abundance_{id}"),
            Self::StudyVariable(id) => write!(f, "abundance_{id}"),
            Self::VariationStudyVariable(id) => write!(f, "abundance_variation_{id}"),
        }
    }
}

/// What an `opt_` column refers to: `opt_{identifier}_…`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptTarget {
    /// `global`: the value relates to all replicates.
    Global,
    /// `assay[n]`.
    Assay(AssayId),
    /// `study_variable[n]`.
    StudyVariable(StudyVariableId),
    /// `ms_run[n]`.
    MsRun(MsRunId),
}

impl fmt::Display for OptTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => f.write_str("global"),
            Self::Assay(id) => id.fmt(f),
            Self::StudyVariable(id) => id.fmt(f),
            Self::MsRun(id) => id.fmt(f),
        }
    }
}

/// An `opt_{identifier}_{name}` column.
///
/// `name` is everything after the identifier's trailing `_`. For a
/// CV-typed column it reads `cv_{accession}_{parameter_name}`; splitting it
/// is left to the validator, because accessions may themselves contain `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptColumn<'a> {
    /// What the column refers to.
    pub target: OptTarget,
    /// The rest of the column name.
    pub name: Text<'a>,
}

impl fmt::Display for OptColumn<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "opt_{}_{}", self.target, self.name)
    }
}

macro_rules! stable_columns {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $col:literal,)+ }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$vdoc])* #[doc = concat!("`", $col, "`.")] $variant,)+
        }

        impl $name {
            /// The stable columns in canonical order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The column label as written in the header. Case-sensitive.
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self { $(Self::$variant => $col,)+ }
            }

            /// The column with this label, if it is a stable column.
            #[must_use]
            pub fn from_label(label: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|c| c.label() == label)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.label())
            }
        }
    };
}

stable_columns! {
    /// The stable columns of the small molecule summary.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmlColumn;
    ///
    /// assert_eq!(SmlColumn::from_label("SMF_ID_REFS"), Some(SmlColumn::SmfIdRefs));
    /// assert_eq!(SmlColumn::ALL.first().map(|c| c.label()), Some("SML_ID"));
    /// assert_eq!(SmlColumn::from_label("sml_id"), None);
    /// ```
    SmlColumn {
        SmlId = "SML_ID",
        SmfIdRefs = "SMF_ID_REFS",
        DatabaseIdentifier = "database_identifier",
        ChemicalFormula = "chemical_formula",
        Smiles = "smiles",
        Inchi = "inchi",
        ChemicalName = "chemical_name",
        Uri = "uri",
        TheoreticalNeutralMass = "theoretical_neutral_mass",
        AdductIons = "adduct_ions",
        Reliability = "reliability",
        BestIdConfidenceMeasure = "best_id_confidence_measure",
        BestIdConfidenceValue = "best_id_confidence_value",
    }
}

stable_columns! {
    /// The stable columns of the small molecule feature table.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmfColumn;
    ///
    /// assert_eq!(SmfColumn::Charge.to_string(), "charge");
    /// ```
    SmfColumn {
        SmfId = "SMF_ID",
        SmeIdRefs = "SME_ID_REFS",
        SmeIdRefAmbiguityCode = "SME_ID_REF_ambiguity_code",
        AdductIon = "adduct_ion",
        Isotopomer = "isotopomer",
        ExpMassToCharge = "exp_mass_to_charge",
        Charge = "charge",
        RetentionTimeInSeconds = "retention_time_in_seconds",
        RetentionTimeInSecondsStart = "retention_time_in_seconds_start",
        RetentionTimeInSecondsEnd = "retention_time_in_seconds_end",
    }
}

stable_columns! {
    /// The stable columns of the small molecule evidence table.
    ///
    /// SME's `id_confidence_measure[n]` columns are not stable columns:
    /// they are [`Column::IdConfidence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::SmeColumn;
    ///
    /// assert_eq!(SmeColumn::ALL.last(), Some(&SmeColumn::Rank));
    /// ```
    SmeColumn {
        SmeId = "SME_ID",
        EvidenceInputId = "evidence_input_id",
        DatabaseIdentifier = "database_identifier",
        ChemicalFormula = "chemical_formula",
        Smiles = "smiles",
        Inchi = "inchi",
        ChemicalName = "chemical_name",
        Uri = "uri",
        DerivatizedForm = "derivatized_form",
        AdductIon = "adduct_ion",
        ExpMassToCharge = "exp_mass_to_charge",
        Charge = "charge",
        TheoreticalMassToCharge = "theoretical_mass_to_charge",
        SpectraRef = "spectra_ref",
        IdentificationMethod = "identification_method",
        MsLevel = "ms_level",
        Rank = "rank",
    }
}

macro_rules! row_ids {
    ($($(#[$doc:meta])* $name:ident;)+) => {$(
        $(#[$doc])*
        ///
        /// The field reference types it as an integer; this crate reads it as
        /// a non-negative 32-bit integer.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

row_ids! {
    /// An `SML_ID`.
    SmlId;
    /// An `SMF_ID`, also used in `SMF_ID_REFS`.
    SmfId;
    /// An `SME_ID`, also used in `SME_ID_REFS`.
    SmeId;
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{AbundanceColumn, OptColumn, OptTarget, SmeColumn, SmfColumn, SmlColumn, SmlId};
    use crate::id::{AssayId, MsRunId, StudyVariableId};
    use crate::value::Text;
    use alloc::string::ToString as _;

    #[test]
    fn stable_columns_round_trip_labels() {
        for c in SmlColumn::ALL {
            assert_eq!(SmlColumn::from_label(c.label()), Some(*c), "{c}");
        }
        for c in SmfColumn::ALL {
            assert_eq!(SmfColumn::from_label(&c.to_string()), Some(*c), "{c}");
        }
        for c in SmeColumn::ALL {
            assert_eq!(SmeColumn::from_label(c.label()), Some(*c), "{c}");
        }
        assert_eq!(
            (
                SmlColumn::ALL.len(),
                SmfColumn::ALL.len(),
                SmeColumn::ALL.len()
            ),
            (13, 10, 17),
            "FREF counts"
        );
        assert_eq!(
            SmeColumn::from_label("id_confidence_measure[1]"),
            None,
            "not stable"
        );
    }

    #[test]
    fn column_display() {
        let assay = AssayId::new(1).expect("valid fixture");
        let sv = StudyVariableId::new(2).expect("valid fixture");
        let run = MsRunId::new(3).expect("valid fixture");
        assert_eq!(
            AbundanceColumn::Assay(assay).to_string(),
            "abundance_assay[1]",
            "assay"
        );
        assert_eq!(
            AbundanceColumn::StudyVariable(sv).to_string(),
            "abundance_study_variable[2]",
            "sv"
        );
        assert_eq!(
            AbundanceColumn::VariationStudyVariable(sv).to_string(),
            "abundance_variation_study_variable[2]",
            "variation"
        );
        let name = Text::new("cv_MS:1002217_decoy_peptide").expect("valid fixture");
        let targets = [
            (OptTarget::Global, "opt_global_"),
            (OptTarget::Assay(assay), "opt_assay[1]_"),
            (OptTarget::StudyVariable(sv), "opt_study_variable[2]_"),
            (OptTarget::MsRun(run), "opt_ms_run[3]_"),
        ];
        for (target, prefix) in targets {
            let col = OptColumn {
                target,
                name: name.clone(),
            };
            assert_eq!(
                col.to_string(),
                [prefix, name.as_str()].concat(),
                "{prefix}"
            );
        }
        assert_eq!(SmlId(4).to_string(), "4", "row id");
    }
}
