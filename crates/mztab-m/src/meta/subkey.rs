//! Sub-key enums for indexed metadata elements (`instrument[n]-*`, `assay[n]-*`, etc.).

use core::fmt;

use crate::id::{Ix, parse_index};

/// One `name` or `name[n]` part of a key, split at `-`.
pub(super) type Segment<'k> = (&'k str, Option<u32>);

/// Splits `name[n]` into its name and index. `None` if the brackets or the
/// index are malformed.
pub(super) fn segment(text: &str) -> Option<Segment<'_>> {
    match text.split_once('[') {
        None => Some((text, None)),
        Some((name, rest)) => {
            let index = rest.strip_suffix(']').and_then(parse_index)?;
            Some((name, Some(index)))
        }
    }
}

pub(super) trait SubKey: Copy {
    /// Writes `-label` or `-label[m]`, or nothing for the bare element.
    fn fmt_suffix(self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub(super) fn fmt_elem(
    f: &mut fmt::Formatter<'_>,
    id: impl fmt::Display,
    k: impl SubKey,
) -> fmt::Result {
    id.fmt(f)?;
    k.fmt_suffix(f)
}

/// How a sub-key's variant is written: a label, and whether it takes `[m]`.
macro_rules! sub_keys {
    ($(
        $(#[$doc:meta])*
        $name:ident {
            $( $(#[$vdoc:meta])* $variant:ident $(($ix:ident))? = $label:tt, )+
        }
    )+) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$vdoc])* $variant $(($ix))?, )+
        }

        impl $name {
            /// Parses the part after the element, e.g. `custom[2]` in
            /// `assay[1]-custom[2]`. `None` is the bare element.
            pub(super) fn parse(rest: Option<Segment<'_>>) -> Option<Self> {
                $( sub_keys!(@parse rest, $variant $(($ix))?, $label); )+
                None
            }
        }

        impl SubKey for $name {
            fn fmt_suffix(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $( sub_keys!(@pat m, $variant $(($ix))?) => sub_keys!(@fmt f, m, $label $(, $ix)?), )+
                }
            }
        }
    )+};

    (@parse $rest:ident, $variant:ident, "") => {
        if $rest.is_none() {
            return Some(Self::$variant);
        }
    };
    (@parse $rest:ident, $variant:ident, $label:literal) => {
        if $rest == Some(($label, None)) {
            return Some(Self::$variant);
        }
    };
    (@parse $rest:ident, $variant:ident($ix:ident), $label:literal) => {
        if let Some(($label, Some(m))) = $rest {
            return $ix::new(m).map(Self::$variant);
        }
    };
    (@pat $m:ident, $variant:ident) => { Self::$variant };
    (@pat $m:ident, $variant:ident($ix:ident)) => { Self::$variant($m) };
    (@fmt $f:ident, $m:ident, "") => { Ok(()) };
    (@fmt $f:ident, $m:ident, $label:literal) => { $f.write_str(concat!("-", $label)) };
    (@fmt $f:ident, $m:ident, $label:literal, $ix:ident) => {
        write!($f, concat!("-", $label, "[{}]"), $m.get())
    };
}

sub_keys! {
    /// The fields of `instrument[n]`.
    InstrumentKey {
        /// `-name`.
        Name = "name",
        /// `-source`.
        Source = "source",
        /// `-analyzer[m]`.
        Analyzer(Ix) = "analyzer",
        /// `-detector`.
        Detector = "detector",
    }

    /// `software[n]` and its settings.
    SoftwareKey {
        /// `software[n]` itself.
        Software = "",
        /// `-setting[m]`.
        Setting(Ix) = "setting",
    }

    /// The fields of `contact[n]`.
    ContactKey {
        /// `-name`.
        Name = "name",
        /// `-affiliation`.
        Affiliation = "affiliation",
        /// `-email`.
        Email = "email",
        /// `-orcid`.
        Orcid = "orcid",
    }

    /// `sample[n]` and its fields.
    SampleKey {
        /// `sample[n]` itself: the name.
        Name = "",
        /// `-species[m]`.
        Species(Ix) = "species",
        /// `-tissue[m]`.
        Tissue(Ix) = "tissue",
        /// `-cell_type[m]`.
        CellType(Ix) = "cell_type",
        /// `-disease[m]`.
        Disease(Ix) = "disease",
        /// `-description`.
        Description = "description",
        /// `-custom[m]`.
        Custom(Ix) = "custom",
    }

    /// The fields of `ms_run[n]`.
    MsRunKey {
        /// `-location`.
        Location = "location",
        /// `-instrument_ref`.
        InstrumentRef = "instrument_ref",
        /// `-format`.
        Format = "format",
        /// `-id_format`.
        IdFormat = "id_format",
        /// `-fragmentation_method[m]`.
        FragmentationMethod(Ix) = "fragmentation_method",
        /// `-scan_polarity[m]`.
        ScanPolarity(Ix) = "scan_polarity",
        /// `-hash`.
        Hash = "hash",
        /// `-hash_method`.
        HashMethod = "hash_method",
        /// `-parameters`.
        Parameters = "parameters",
        /// `-usi_identifier` (2.0 only).
        UsiIdentifier = "usi_identifier",
    }

    /// `assay[n]` and its fields.
    AssayKey {
        /// `assay[n]` itself: the name.
        Name = "",
        /// `-custom[m]`.
        Custom(Ix) = "custom",
        /// `-external_uri`.
        ExternalUri = "external_uri",
        /// `-sample_ref`.
        SampleRef = "sample_ref",
        /// `-ms_run_ref`.
        MsRunRef = "ms_run_ref",
        /// `-protocol_refs`.
        ProtocolRefs = "protocol_refs",
        /// `-parameters`.
        Parameters = "parameters",
    }

    /// `study_variable[n]` and its fields.
    StudyVariableKey {
        /// `study_variable[n]` itself: the name.
        Name = "",
        /// `-assay_refs`.
        AssayRefs = "assay_refs",
        /// `-ms_run_refs`.
        MsRunRefs = "ms_run_refs",
        /// `-description`.
        Description = "description",
        /// `-average_function`.
        AverageFunction = "average_function",
        /// `-variation_function`.
        VariationFunction = "variation_function",
        /// `-factors` (2.0 only).
        Factors = "factors",
    }

    /// `study_variable_group[n]` and its fields.
    StudyVariableGroupKey {
        /// `study_variable_group[n]` itself.
        Group = "",
        /// `-description`.
        Description = "description",
        /// `-type`.
        Type = "type",
        /// `-datatype`.
        Datatype = "datatype",
        /// `-unit`.
        Unit = "unit",
        /// `-study_variable_refs`.
        StudyVariableRefs = "study_variable_refs",
    }

    /// The fields of `protocol[n]`.
    ProtocolKey {
        /// `-name`.
        Name = "name",
        /// `-type`.
        Type = "type",
        /// `-description`.
        Description = "description",
        /// `-parameters`.
        Parameters = "parameters",
    }

    /// The fields of `cv[n]`.
    CvKey {
        /// `-label`.
        Label = "label",
        /// `-full_name`.
        FullName = "full_name",
        /// `-version`.
        Version = "version",
        /// `-uri`.
        Uri = "uri",
    }

    /// `database[n]` and its fields.
    DatabaseKey {
        /// `database[n]` itself.
        Database = "",
        /// `-prefix`.
        Prefix = "prefix",
        /// `-version`.
        Version = "version",
        /// `-uri`.
        Uri = "uri",
    }
}
