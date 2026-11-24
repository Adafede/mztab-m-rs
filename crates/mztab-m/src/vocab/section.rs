//! Document [`Section`]s and table-only [`TableSection`]s.

/// A section of an mzTab-M file, identified by its line prefixes.
///
/// # Examples
///
/// ```
/// use mztab_m::Section;
///
/// assert_eq!(Section::from_prefix("SFH"), Some((Section::Smf, true)));
/// assert_eq!(Section::from_prefix("SME"), Some((Section::Sme, false)));
/// assert_eq!(Section::from_prefix("COM"), None);
/// assert_eq!(Section::Sml.header_prefix(), Some("SMH"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    /// Metadata, `MTD`.
    Mtd,
    /// Small molecule summary, `SMH` / `SML`.
    Sml,
    /// Small molecule features, `SFH` / `SMF`.
    Smf,
    /// Small molecule evidence, `SEH` / `SME`.
    Sme,
}

impl Section {
    /// The sections in their mandatory file order.
    pub const ALL: [Self; 4] = [Self::Mtd, Self::Sml, Self::Smf, Self::Sme];

    /// The prefix of the section's data lines.
    #[must_use]
    pub const fn row_prefix(self) -> &'static str {
        match self {
            Self::Mtd => "MTD",
            Self::Sml => "SML",
            Self::Smf => "SMF",
            Self::Sme => "SME",
        }
    }

    /// The prefix of the section's header line; `None` for metadata.
    #[must_use]
    pub const fn header_prefix(self) -> Option<&'static str> {
        match self {
            Self::Mtd => None,
            Self::Sml => Some("SMH"),
            Self::Smf => Some("SFH"),
            Self::Sme => Some("SEH"),
        }
    }

    /// The section a line prefix belongs to, and whether it is the header.
    /// Case-sensitive (SPEC §6). `COM` belongs to no section.
    #[must_use]
    pub fn from_prefix(prefix: &str) -> Option<(Self, bool)> {
        Self::ALL.into_iter().find_map(|s| {
            if s.row_prefix() == prefix {
                Some((s, false))
            } else {
                (s.header_prefix() == Some(prefix)).then_some((s, true))
            }
        })
    }
}

/// One of the three data tables (`SML`, `SMF`, `SME`), excluding `MTD`.
///
/// Using `TableSection` in table-only writers and validators makes passing
/// `Section::Mtd` unrepresentable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TableSection {
    Sml,
    Smf,
    Sme,
}

impl TableSection {
    /// All three tables in specification order.
    pub(crate) const ALL: [Self; 3] = [Self::Sml, Self::Smf, Self::Sme];

    /// Converts a [`Section`] if it names a table, returning `None` for `Mtd`.
    pub(crate) const fn from_section(section: Section) -> Option<Self> {
        match section {
            Section::Mtd => None,
            Section::Sml => Some(Self::Sml),
            Section::Smf => Some(Self::Smf),
            Section::Sme => Some(Self::Sme),
        }
    }

    /// The corresponding [`Section`].
    pub(crate) const fn section(self) -> Section {
        match self {
            Self::Sml => Section::Sml,
            Self::Smf => Section::Smf,
            Self::Sme => Section::Sme,
        }
    }

    /// Lowercase model-path name of this table.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Sml => "sml",
            Self::Smf => "smf",
            Self::Sme => "sme",
        }
    }

    /// Zero-based index into a `[sml, smf, sme]` array.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Sml => 0,
            Self::Smf => 1,
            Self::Sme => 2,
        }
    }

    /// The entry of a per-table array for `self`.
    pub(crate) const fn pick<T>(self, items: &[T; 3]) -> &T {
        let [sml, smf, sme] = items;
        match self {
            Self::Sml => sml,
            Self::Smf => smf,
            Self::Sme => sme,
        }
    }

    /// The mutable entry of a per-table array for `self`.
    pub(crate) const fn pick_mut<T>(self, items: &mut [T; 3]) -> &mut T {
        let [sml, smf, sme] = items;
        match self {
            Self::Sml => sml,
            Self::Smf => smf,
            Self::Sme => sme,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Section, TableSection};

    #[test]
    fn sections() {
        for s in Section::ALL {
            assert_eq!(
                Section::from_prefix(s.row_prefix()),
                Some((s, false)),
                "{s:?}"
            );
            if let Some(h) = s.header_prefix() {
                assert_eq!(Section::from_prefix(h), Some((s, true)), "{s:?}");
            }
        }
        assert_eq!(Section::from_prefix("mtd"), None, "case-sensitive");
        assert_eq!(Section::Mtd.header_prefix(), None, "no MTD header");
        assert_eq!(Section::Sml.header_prefix(), Some("SMH"), "SML header");
    }

    #[test]
    fn table_sections_exclude_metadata() {
        assert_eq!(TableSection::from_section(Section::Mtd), None, "mtd");
        let mut items = ["a", "b", "c"];
        for (idx, ts) in TableSection::ALL.into_iter().enumerate() {
            assert_eq!(TableSection::from_section(ts.section()), Some(ts));
            assert_eq!(ts.index(), idx);
            assert_eq!(Some(*ts.pick(&items)), items.get(idx).copied());
            *ts.pick_mut(&mut items) = ts.name();
        }
        assert_eq!(items, ["sml", "smf", "sme"]);
    }
}
