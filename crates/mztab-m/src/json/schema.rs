//! The JSCH shape of mzTab-M (`schema/mzTab_2_1-M.json`): which JSON
//! property carries which metadata key or table column, and in what form.
//! Both directions of the transcoding read these tables.

pub(super) use super::schema_tables::{Col, SNAKE_DIALECT, TABLES, Table, standard_header};

/// The JSON form of a metadata value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    /// A string.
    Text,
    /// A string or a `Parameter` object (`StudyVariable.name`).
    TextOrParam,
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

use Shape::{Indexed, Param, Params, Publication, Ref, Refs, Strings, Text, TextOrParam};

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
        value: Some(("name", TextOrParam)),
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
