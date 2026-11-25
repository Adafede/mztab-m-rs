//! Document root [`MzTab`] and [`Stripped`] counts.

use super::layout::SourceLayout;
use super::metadata::Metadata;
use super::rows::{SmeRow, SmfRow, SmlRow};
use super::table::{Column, SmeColumn, SmfColumn, SmlColumn, Table};
use crate::vocab::profile::{Profile, ProhibitedProfile};

/// What [`MzTab::strip_unknown`] dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stripped {
    /// Unknown metadata lines.
    pub metadata: usize,
    /// Unknown or repeated header columns.
    pub columns: usize,
    /// Row cells: those of the dropped columns, and those beyond a header.
    pub cells: usize,
}

/// A whole mzTab-M document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MzTab<'a> {
    /// The metadata section (`MTD`), always present.
    pub metadata: Metadata<'a>,
    /// The small molecule summary (`SMH` / `SML`), if present.
    pub sml: Option<Table<'a, SmlColumn, SmlRow<'a>>>,
    /// The small molecule features (`SFH` / `SMF`), if present.
    pub smf: Option<Table<'a, SmfColumn, SmfRow<'a>>>,
    /// The small molecule evidence (`SEH` / `SME`), if present.
    pub sme: Option<Table<'a, SmeColumn, SmeRow<'a>>>,
    /// What the typed model does not capture about the source text.
    pub layout: SourceLayout<'a>,
}

impl MzTab<'_> {
    /// The profile implied by the tables present (SPEC §5.18), or the
    /// prohibited combination they form.
    ///
    /// A header-only table counts as present.
    ///
    /// # Errors
    ///
    /// Returns the prohibited combination for evidence without features.
    ///
    /// # Examples
    ///
    /// ```
    /// use mztab_m::{MzTab, Profile};
    ///
    /// assert_eq!(MzTab::default().inferred_profile(), Ok(Profile::M));
    /// ```
    pub const fn inferred_profile(&self) -> Result<Profile, ProhibitedProfile> {
        Profile::from_tables(self.sml.is_some(), self.smf.is_some(), self.sme.is_some())
    }

    /// Drops what the reader kept but could not place: unknown metadata
    /// lines, unknown (or repeated) columns with their cells, and cells
    /// beyond a row's header. What remains is the document as the
    /// specification defines it; JSON can carry it (D-071, D-075).
    ///
    /// # Examples
    ///
    /// ```
    /// let mut model = mztab_m::parse("MTD\tnot-a-key\tx\nSFH\tSMF_ID\tbogus\nSMF\t1\ty\tz\n").mztab;
    /// let dropped = model.strip_unknown();
    /// assert_eq!((dropped.metadata, dropped.columns, dropped.cells), (1, 1, 2));
    /// ```
    pub fn strip_unknown(&mut self) -> Stripped {
        let mut dropped = Stripped {
            metadata: self.metadata.unknown.len(),
            ..Stripped::default()
        };
        self.metadata.unknown.clear();
        macro_rules! strip {
            ($table:expr) => {
                if let Some(t) = $table {
                    let before = t.header.columns.len();
                    t.header
                        .columns
                        .retain(|c| !matches!(c, Column::Unknown(_)));
                    dropped.columns += before - t.header.columns.len();
                    for row in &mut t.rows {
                        dropped.cells += row.unknown.len() + row.extra.len();
                        row.unknown.clear();
                        row.extra.clear();
                    }
                }
            };
        }
        strip!(&mut self.sml);
        strip!(&mut self.smf);
        strip!(&mut self.sme);
        dropped
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::MzTab;
    use crate::model::table::{Header, Table};
    use crate::vocab::profile::{Profile, ProhibitedProfile};
    use alloc::vec::Vec;

    #[test]
    fn inferred_profile_counts_header_only_tables() {
        fn empty<S, R>() -> Table<'static, S, R> {
            Table {
                header: Header {
                    columns: Vec::new(),
                },
                rows: Vec::new(),
            }
        }
        let mut m = MzTab {
            sme: Some(empty()),
            ..MzTab::default()
        };
        assert_eq!(m.inferred_profile(), Err(ProhibitedProfile::ME), "M+E");
        m.smf = Some(empty());
        assert_eq!(m.inferred_profile(), Ok(Profile::MFE), "M+F+E");
    }
}
