//! Value and table-row rendering traits for TSV output.

use alloc::vec::Vec;

use super::error::WriteError;
use super::sink::Out;
use crate::id::{AssayId, InstrumentId, MsRunId, ProtocolId, SampleId, StudyVariableId};
use crate::model::colunit::{ColUnit, SpectraRef};
use crate::model::meta_value::ValueRef;
use crate::model::rows::{SmeRow, SmfRow, SmlRow};
use crate::model::table::{SmeColumn, SmeId, SmfColumn, SmfId, SmlColumn, SmlId};
use crate::param::Param;
use crate::value::NULL;
use crate::value::cell::Cell;
use crate::value::decimal::Decimal;
use crate::value::text::{Raw, Text};
use crate::vocab::field::{AmbiguityCode, Datatype, PublicationItem};
use crate::vocab::profile::Profile;
use crate::vocab::version::Version;

/// A value that can be written as (part of) a cell.
pub(super) trait Render {
    /// Whether a list of these splits like a parameter list.
    const BRACKETS: bool = false;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError>;
}

macro_rules! render_display {
    ($($ty:ty),+ $(,)?) => {$(
        impl Render for $ty {
            fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
                out.display(self)
            }
        }
    )+};
}

render_display! {
    Decimal<'_>, Version, Profile, Datatype, AmbiguityCode, u32, i32,
    SmlId, SmfId, SmeId, AssayId, InstrumentId, MsRunId, ProtocolId, SampleId, StudyVariableId,
}

impl Render for Param<'_> {
    const BRACKETS: bool = true;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.display(self)
    }
}

impl Render for Text<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.str(self.as_str())
    }
}

impl Render for PublicationItem<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.str(self.kind.as_str())?;
        out.str(":")?;
        out.str(self.accession.as_str())
    }
}

impl Render for SpectraRef<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.display(&self.ms_run)?;
        out.str(":")?;
        out.str(self.reference.as_str())
    }
}

impl Render for ColUnit<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        if self.column.as_str().contains('=') {
            return Err(out.fail("a unit column name containing `=`"));
        }
        self.column.render(out)?;
        out.str("=")?;
        out.display(&self.param)
    }
}

impl<T: Render> Render for Cell<'_, T> {
    const BRACKETS: bool = T::BRACKETS;

    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        match self {
            Self::Value(value) => value.render(out),
            Self::Null => out.str(NULL),
            Self::Invalid(raw) if raw.as_str() == NULL => {
                Err(out.fail("invalid text `null`, which reads back as null"))
            }
            Self::Invalid(raw) => raw.render(out),
        }
    }
}

impl Render for Raw<'_> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        out.checked(self.as_str())
    }
}

/// Refuses lists that cannot be written: empty, or a single `null` (which
/// reads back as a null cell, not a list). Not generic, so covered once.
const fn list_shape(out: &Out<'_>, len: usize, first_is_null: bool) -> Result<(), WriteError> {
    match (len, first_is_null) {
        (0, _) => Err(out.fail("an empty list")),
        (1, true) => Err(out.fail("a list of one `null`, which reads back as a null cell")),
        _ => Ok(()),
    }
}

impl<T: Render> Render for Vec<Cell<'_, T>> {
    fn render(&self, out: &mut Out<'_>) -> Result<(), WriteError> {
        list_shape(out, self.len(), self.first().is_some_and(Cell::is_null))?;
        out.with_list(T::BRACKETS, |out| {
            self.iter().enumerate().try_for_each(|(i, item)| {
                if i > 0 {
                    out.separator()?;
                }
                item.render(out)
            })
        })
    }
}

/// Writes a metadata value.
pub(super) fn render_value(value: ValueRef<'_, '_>, out: &mut Out<'_>) -> Result<(), WriteError> {
    match value {
        ValueRef::Version(c) => c.render(out),
        ValueRef::Profile(c) => c.render(out),
        ValueRef::Datatype(c) => c.render(out),
        ValueRef::Text(c) => c.render(out),
        ValueRef::Param(c) => c.render(out),
        ValueRef::ParamList(c) => c.render(out),
        ValueRef::Publication(c) => c.render(out),
        ValueRef::InstrumentRef(c) => c.render(out),
        ValueRef::SampleRef(c) => c.render(out),
        ValueRef::MsRunRefs(c) => c.render(out),
        ValueRef::ProtocolRefs(c) => c.render(out),
        ValueRef::AssayRefs(c) => c.render(out),
        ValueRef::StudyVariableRefs(c) => c.render(out),
        ValueRef::ColUnit(c) => c.render(out),
    }
}

/// A table row whose cells can be streamed to [`Out`].
pub(crate) trait RenderRow<'a> {
    type Column: Copy;

    fn render_stable(&self, column: Self::Column, out: &mut Out<'_>) -> Result<(), WriteError>;
    fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>];
    fn opt_ref(&self) -> &[Cell<'a, Text<'a>>];
    fn unknown_ref(&self) -> &[Raw<'a>];
    fn extra_ref(&self) -> &[Raw<'a>];
}

macro_rules! render_row {
    ($row:ident, $col:ident, $numbers:ident, { $($variant:ident => $field:ident),+ $(,)? }) => {
        impl<'a> RenderRow<'a> for $row<'a> {
            type Column = $col;

            fn render_stable(&self, column: $col, out: &mut Out<'_>) -> Result<(), WriteError> {
                match column {
                    $($col::$variant => self.$field.render(out),)+
                }
            }

            fn numbers_ref(&self) -> &[Cell<'a, Decimal<'a>>] {
                &self.$numbers
            }

            fn opt_ref(&self) -> &[Cell<'a, Text<'a>>] {
                &self.opt
            }

            fn unknown_ref(&self) -> &[Raw<'a>] {
                &self.unknown
            }

            fn extra_ref(&self) -> &[Raw<'a>] {
                &self.extra
            }
        }
    };
}

render_row!(SmlRow, SmlColumn, abundance, {
    SmlId => sml_id,
    SmfIdRefs => smf_id_refs,
    DatabaseIdentifier => database_identifier,
    ChemicalFormula => chemical_formula,
    Smiles => smiles,
    Inchi => inchi,
    ChemicalName => chemical_name,
    Uri => uri,
    TheoreticalNeutralMass => theoretical_neutral_mass,
    AdductIons => adduct_ions,
    Reliability => reliability,
    BestIdConfidenceMeasure => best_id_confidence_measure,
    BestIdConfidenceValue => best_id_confidence_value,
});

render_row!(SmfRow, SmfColumn, abundance, {
    SmfId => smf_id,
    SmeIdRefs => sme_id_refs,
    SmeIdRefAmbiguityCode => sme_id_ref_ambiguity_code,
    AdductIon => adduct_ion,
    Isotopomer => isotopomer,
    ExpMassToCharge => exp_mass_to_charge,
    Charge => charge,
    RetentionTimeInSeconds => retention_time_in_seconds,
    RetentionTimeInSecondsStart => retention_time_in_seconds_start,
    RetentionTimeInSecondsEnd => retention_time_in_seconds_end,
});

render_row!(SmeRow, SmeColumn, id_confidence, {
    SmeId => sme_id,
    EvidenceInputId => evidence_input_id,
    DatabaseIdentifier => database_identifier,
    ChemicalFormula => chemical_formula,
    Smiles => smiles,
    Inchi => inchi,
    ChemicalName => chemical_name,
    Uri => uri,
    DerivatizedForm => derivatized_form,
    AdductIon => adduct_ion,
    ExpMassToCharge => exp_mass_to_charge,
    Charge => charge,
    TheoreticalMassToCharge => theoretical_mass_to_charge,
    SpectraRef => spectra_ref,
    IdentificationMethod => identification_method,
    MsLevel => ms_level,
    Rank => rank,
});
