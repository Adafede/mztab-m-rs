//! The JSCH shape of mzTab-M (`schema/mzTab_2_1-M.json`): which JSON
//! property carries which metadata key or table column, and in what form.
//! Both directions of the transcoding read these tables.

/// The JSON form of a metadata value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    /// A string.
    Text,
    /// A `Parameter` object.
    Param,
    /// An array of `Parameter` objects: a `|`-separated parameter list.
    Params,
    /// An integer: a reference such as `instrument[2]`.
    Ref(&'static str),
    /// An array of integers: a list of references.
    Refs(&'static str),
    /// An array of `Parameter` objects with `id`s: the indexed keys
    /// `name[1]`, `name[2]`, ...
    Indexed,
    /// An array of strings: the indexed keys `name[1]`, ..., by position.
    Strings,
    /// An array of `PublicationItem` objects.
    Publication,
}

/// A property of a metadata element: JSON name, TSV sub-key, form.
pub(super) type Prop = (&'static str, &'static str, Shape);

/// A metadata element `name[n]`: its JSON array, its TSV name, the
/// property that holds the value of the bare `name[n]` key (if any), and
/// its other properties.
#[derive(Debug)]
pub(super) struct Element {
    pub(super) json: &'static str,
    pub(super) tsv: &'static str,
    pub(super) value: Option<(&'static str, Shape)>,
    pub(super) props: &'static [Prop],
}

/// One entry of the metadata object, in JSCH order.
#[derive(Debug)]
pub(super) enum Item {
    /// A single key: JSON name, TSV key, form.
    Field(&'static str, &'static str, Shape),
    /// An array of elements.
    Element(Element),
    /// Top-level indexed keys `name[n]` as an array of `Parameter`s.
    Indexed(&'static str),
    /// A repeated `colunit-*` key as an array of `ColumnParameterMapping`s.
    Colunit(&'static str),
}

use Shape::{Indexed, Param, Params, Publication, Ref, Refs, Strings, Text};

pub(super) const METADATA: &[Item] = &[
    Item::Field("mzTab-version", "mzTab-version", Text),
    Item::Field("mzTab-ID", "mzTab-ID", Text),
    Item::Field("mzTab-profile", "mzTab-profile", Text),
    Item::Field("title", "title", Text),
    Item::Field("description", "description", Text),
    Item::Element(Element {
        json: "contact",
        tsv: "contact",
        value: None,
        props: &[
            ("name", "name", Text),
            ("affiliation", "affiliation", Text),
            ("email", "email", Text),
            ("orcid", "orcid", Text),
        ],
    }),
    Item::Element(Element {
        json: "publication",
        tsv: "publication",
        value: Some(("publication_items", Publication)),
        props: &[],
    }),
    Item::Element(Element {
        json: "uri",
        tsv: "uri",
        value: Some(("value", Text)),
        props: &[],
    }),
    Item::Element(Element {
        json: "external_study_uri",
        tsv: "external_study_uri",
        value: Some(("value", Text)),
        props: &[],
    }),
    Item::Element(Element {
        json: "instrument",
        tsv: "instrument",
        value: None,
        props: &[
            ("name", "name", Param),
            ("source", "source", Param),
            ("analyzer", "analyzer", Indexed),
            ("detector", "detector", Param),
        ],
    }),
    Item::Field("quantification_method", "quantification_method", Param),
    Item::Element(Element {
        json: "sample",
        tsv: "sample",
        value: Some(("name", Text)),
        props: &[
            ("custom", "custom", Indexed),
            ("species", "species", Indexed),
            ("tissue", "tissue", Indexed),
            ("cell_type", "cell_type", Indexed),
            ("disease", "disease", Indexed),
            ("description", "description", Text),
        ],
    }),
    Item::Element(Element {
        json: "protocol",
        tsv: "protocol",
        value: None,
        props: &[
            ("name", "name", Text),
            ("type", "type", Param),
            ("description", "description", Text),
            ("parameters", "parameters", Params),
        ],
    }),
    Item::Element(Element {
        json: "sample_processing",
        tsv: "sample_processing",
        value: Some(("sample_processing", Params)),
        props: &[],
    }),
    Item::Element(Element {
        json: "software",
        tsv: "software",
        value: Some(("parameter", Param)),
        props: &[("setting", "setting", Strings)],
    }),
    Item::Indexed("derivatization_agent"),
    Item::Element(Element {
        json: "ms_run",
        tsv: "ms_run",
        value: None,
        props: &[
            ("location", "location", Text),
            ("instrument_ref", "instrument_ref", Ref("instrument")),
            ("format", "format", Param),
            ("id_format", "id_format", Param),
            ("fragmentation_method", "fragmentation_method", Indexed),
            ("scan_polarity", "scan_polarity", Indexed),
            ("hash", "hash", Text),
            ("hash_method", "hash_method", Param),
            ("parameters", "parameters", Params),
            ("usi_identifier", "usi_identifier", Text),
        ],
    }),
    Item::Element(Element {
        json: "assay",
        tsv: "assay",
        value: Some(("name", Text)),
        props: &[
            ("custom", "custom", Indexed),
            ("external_uri", "external_uri", Text),
            ("sample_ref", "sample_ref", Ref("sample")),
            ("ms_run_ref", "ms_run_ref", Refs("ms_run")),
            ("protocol_refs", "protocol_refs", Refs("protocol")),
            ("parameters", "parameters", Params),
        ],
    }),
    Item::Element(Element {
        json: "study_variable_group",
        tsv: "study_variable_group",
        value: Some(("name", Param)),
        props: &[
            ("description", "description", Text),
            ("type", "type", Param),
            ("datatype", "datatype", Text),
            ("unit", "unit", Param),
            (
                "study_variable_refs",
                "study_variable_refs",
                Refs("study_variable"),
            ),
        ],
    }),
    Item::Element(Element {
        json: "study_variable",
        tsv: "study_variable",
        value: Some(("name", Text)),
        props: &[
            ("assay_refs", "assay_refs", Refs("assay")),
            ("ms_run_refs", "ms_run_refs", Refs("ms_run")),
            ("description", "description", Text),
            ("average_function", "average_function", Param),
            ("variation_function", "variation_function", Param),
            ("factors", "factors", Params),
        ],
    }),
    Item::Indexed("custom"),
    Item::Element(Element {
        json: "cv",
        tsv: "cv",
        value: None,
        props: &[
            ("label", "label", Text),
            ("full_name", "full_name", Text),
            ("version", "version", Text),
            ("uri", "uri", Text),
        ],
    }),
    Item::Field(
        "small_molecule-quantification_unit",
        "small_molecule-quantification_unit",
        Param,
    ),
    Item::Field(
        "small_molecule_feature-quantification_unit",
        "small_molecule_feature-quantification_unit",
        Param,
    ),
    Item::Field(
        "small_molecule-identification_reliability",
        "small_molecule-identification_reliability",
        Param,
    ),
    Item::Element(Element {
        json: "database",
        tsv: "database",
        value: Some(("param", Param)),
        props: &[
            ("prefix", "prefix", Text),
            ("version", "version", Text),
            ("uri", "uri", Text),
        ],
    }),
    Item::Indexed("id_confidence_measure"),
    Item::Colunit("colunit-small_molecule"),
    Item::Colunit("colunit-small_molecule_feature"),
    Item::Colunit("colunit-small_molecule_evidence"),
];

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

/// A table: its JSON array, TSV prefixes, and stable columns with their
/// JSON forms.
#[derive(Debug)]
pub(super) struct Table {
    pub(super) json: &'static str,
    pub(super) header: &'static str,
    pub(super) row: &'static str,
    pub(super) columns: &'static [(&'static str, Col)],
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
            ("SML_ID", Col::Integer),
            ("SMF_ID_REFS", Col::Integers),
            ("database_identifier", Col::Texts),
            ("chemical_formula", Col::Texts),
            ("smiles", Col::Texts),
            ("inchi", Col::Texts),
            ("chemical_name", Col::Texts),
            ("uri", Col::Texts),
            ("theoretical_neutral_mass", Col::Numbers),
            ("adduct_ions", Col::Texts),
            ("reliability", Col::Text),
            ("best_id_confidence_measure", Col::Param),
            ("best_id_confidence_value", Col::Number),
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
            ("SMF_ID", Col::Integer),
            ("SME_ID_REFS", Col::Integers),
            ("SME_ID_REF_ambiguity_code", Col::Integer),
            ("adduct_ion", Col::Text),
            ("isotopomer", Col::Param),
            ("exp_mass_to_charge", Col::Number),
            ("charge", Col::Integer),
            ("retention_time_in_seconds", Col::Number),
            ("retention_time_in_seconds_start", Col::Number),
            ("retention_time_in_seconds_end", Col::Number),
        ],
        arrays: &[("abundance_assay", "assay", "abundance_assay")],
    },
    Table {
        json: "smallMoleculeEvidence",
        header: "SEH",
        row: "SME",
        columns: &[
            ("SME_ID", Col::Integer),
            ("evidence_input_id", Col::Text),
            ("database_identifier", Col::Text),
            ("chemical_formula", Col::Text),
            ("smiles", Col::Text),
            ("inchi", Col::Text),
            ("chemical_name", Col::Text),
            ("uri", Col::Text),
            ("derivatized_form", Col::Param),
            ("adduct_ion", Col::Text),
            ("exp_mass_to_charge", Col::Number),
            ("charge", Col::Integer),
            ("theoretical_mass_to_charge", Col::Number),
            ("spectra_ref", Col::SpectraRefs),
            ("identification_method", Col::Param),
            ("ms_level", Col::Param),
            ("rank", Col::Integer),
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
pub(super) fn standard_header(
    table: &Table,
    ids: impl Fn(&str) -> alloc::vec::Vec<u32>,
) -> alloc::vec::Vec<alloc::string::String> {
    use alloc::string::ToString as _;
    let split = table
        .columns
        .iter()
        .position(|(name, _)| *name == "rank")
        .unwrap_or(table.columns.len());
    let (before, after) = table.columns.split_at(split);
    let mut out: alloc::vec::Vec<alloc::string::String> =
        before.iter().map(|(name, _)| (*name).to_string()).collect();
    for (_, element, prefix) in table.arrays {
        out.extend(
            ids(element)
                .iter()
                .map(|id| alloc::format!("{prefix}[{id}]")),
        );
    }
    out.extend(after.iter().map(|(name, _)| (*name).to_string()));
    out
}

/// The JSON key of a stable column: its name in lower case.
pub(super) fn key(column: &str) -> alloc::string::String {
    column.to_ascii_lowercase()
}

/// The top-level keys of the `snake_case` dialect some tools write
/// (`MetaboScape`), which JSCH does not define (DECISIONS D-004).
pub(super) const SNAKE_DIALECT: &[&str] = &[
    "small_molecule_summary",
    "small_molecule_feature",
    "small_molecule_evidence",
];
