//! The writer: canonical form, edits after reading, and every write error.
#![expect(
    clippy::expect_used,
    reason = "test helpers: a fixture that does not parse is a setup error"
)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

use mztab_m::{
    Cell, ColUnit, MzTab, Param, Raw, SourceLayout, Text, WriteError, parse, parse_bytes, to_string,
};

const FULL: &[u8] = include_bytes!("fixtures/full.mztab");
const FULL_CANONICAL: &str = include_str!("golden/full.canonical.mztab");

fn without_layout(mut mztab: MzTab<'_>) -> MzTab<'_> {
    mztab.layout = SourceLayout::default();
    mztab
}

fn text(s: &str) -> Text<'_> {
    Text::new(s).expect("valid text")
}

/// The canonical form of the full fixture: metadata in field-reference
/// order, one blank line between sections, no comments. Set `BLESS=1` to
/// rewrite the golden file after an intended change.
#[test]
fn canonical_form_matches_golden() {
    let canonical = without_layout(parse_bytes(FULL).mztab);
    let written = to_string(&canonical).expect("writable");
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BLESS").is_some() {
        let golden = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/full.canonical.mztab"
        );
        std::fs::write(golden, &written).expect("write golden");
        return;
    }
    assert_eq!(written, FULL_CANONICAL, "canonical form");
    assert!(
        without_layout(parse(&written).mztab) == canonical,
        "reads back"
    );
    assert_eq!(
        to_string(&parse(&written).mztab).as_deref(),
        Ok(FULL_CANONICAL),
        "fixed point"
    );
}

#[test]
fn canonical_order_groups_elements_in_field_reference_order() {
    let shuffled = "MTD\ttitle\tT\nMTD\tassay[2]\tb\nMTD\tassay[1]-ms_run_ref\tms_run[1]\nMTD\tassay[1]\ta\nMTD\tcolunit-small_molecule\tx=[,,u,]\nMTD\tmzTab-version\t2.1.0-M\nMTD\tcolunit-small_molecule\ty=[,,v,]\nMTD\tms_run[1]-location\tfile:///r\n";
    let canonical = "MTD\tmzTab-version\t2.1.0-M\nMTD\ttitle\tT\nMTD\tms_run[1]-location\tfile:///r\nMTD\tassay[1]\ta\nMTD\tassay[1]-ms_run_ref\tms_run[1]\nMTD\tassay[2]\tb\nMTD\tcolunit-small_molecule\tx=[, , u, ]\nMTD\tcolunit-small_molecule\ty=[, , v, ]\n";
    let mztab = without_layout(parse(shuffled).mztab);
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok(canonical),
        "sorted, colunits in source order"
    );
}

/// A file already in canonical form keeps no source text in its layout.
#[test]
fn canonical_lines_store_no_text() {
    let parsed = parse(FULL_CANONICAL).mztab;
    assert!(
        parsed
            .layout
            .lines
            .iter()
            .all(|line| line.verbatim.is_none()),
        "nothing differs from its canonical form"
    );
}

/// Cells beyond the header are kept and written, in every table.
#[test]
fn extra_cells_are_written() {
    let source = "SMH\tSML_ID\nSML\t1\tx\n\nSFH\tSMF_ID\nSMF\t1\ty\n\nSEH\tSME_ID\nSME\t1\tz\n";
    let canonical = without_layout(parse(source).mztab);
    assert_eq!(to_string(&canonical).as_deref(), Ok(source), "extras kept");
}

#[test]
fn empty_and_partial_models() {
    assert_eq!(
        to_string(&MzTab::default()).as_deref(),
        Ok(""),
        "nothing to write"
    );
    let only_table = without_layout(parse("SFH\tSMF_ID\nSMF\t1\n").mztab);
    assert_eq!(
        to_string(&only_table).as_deref(),
        Ok("SFH\tSMF_ID\nSMF\t1\n"),
        "no leading blank"
    );
}

/// After reading, an edited value is written canonically, untouched lines
/// keep their source text, and added content goes next to its section.
#[test]
fn edits_after_reading() {
    let source = "COM\tkept\r\nMTD\tmzTab-ID\t x \r\nMTD\ttitle\t[,,a,]\r\n\r\nSFH\tSMF_ID\tcharge\r\nSMF\t1\t+1\r\nSMF\t2\t+2";
    let mut mztab = parse(source).mztab;
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok(source),
        "unchanged: byte-exact"
    );

    mztab.metadata.description = Some(Cell::Value(text("added")));
    mztab.metadata.title = Some(Cell::Value(text("[,,b,]")));
    if let Some(table) = &mut mztab.smf {
        table.rows.swap(0, 1);
        let extra = table.rows.first().cloned().expect("a row");
        table.rows.push(extra);
    }
    let expected = "COM\tkept\r\nMTD\tmzTab-ID\t x \r\nMTD\ttitle\t[,,b,]\r\nMTD\tdescription\tadded\n\r\nSFH\tSMF_ID\tcharge\r\nSMF\t2\t2\r\nSMF\t1\t1\nSMF\t2\t2\n";
    assert_eq!(to_string(&mztab).as_deref(), Ok(expected), "edits");
}

#[test]
fn removed_content_is_skipped_and_new_tables_are_appended() {
    let mut mztab = parse("MTD\tmzTab-ID\tx\nMTD\ttitle\tt\n\nSMH\tSML_ID\nSML\t1\nSML\t2\n").mztab;
    mztab.metadata.title = None;
    if let Some(table) = &mut mztab.sml {
        table.rows.truncate(1);
    }
    mztab.smf = parse("SFH\tSMF_ID\n").mztab.smf;
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok("MTD\tmzTab-ID\tx\n\nSMH\tSML_ID\nSML\t1\n\nSFH\tSMF_ID\n"),
        "title and second row gone, SMF appended"
    );
    mztab.sml = None;
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok("MTD\tmzTab-ID\tx\n\n\nSFH\tSMF_ID\n"),
        "a removed table leaves its blank line"
    );
}

#[test]
fn content_after_an_unterminated_last_line_starts_a_new_line() {
    let mut mztab = parse("MTD\tmzTab-ID\tx").mztab;
    mztab.metadata.title = Some(Cell::Value(text("t")));
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok("MTD\tmzTab-ID\tx\nMTD\ttitle\tt\n"),
        "line closed"
    );
    let mut layout_only_comment = parse("COM\tc").mztab;
    layout_only_comment.metadata.id = Some(Cell::Value(text("x")));
    assert_eq!(
        to_string(&layout_only_comment).as_deref(),
        Ok("MTD\tmzTab-ID\tx\nCOM\tc"),
        "metadata goes first when the source had none"
    );
}

/// Any table type can hold any column kind; all are written.
#[test]
fn every_column_kind_is_written() {
    use mztab_m::{AbundanceColumn, AssayId, Column, IdConfidenceMeasureId, OptColumn, OptTarget};
    let mut mztab = parse("SMH\tSML_ID\n").mztab;
    if let Some(table) = &mut mztab.sml {
        table.header.columns.extend([
            Column::Abundance(AbundanceColumn::Assay(AssayId::new(1).expect("non-zero"))),
            Column::IdConfidence(IdConfidenceMeasureId::new(2).expect("non-zero")),
            Column::Opt(OptColumn {
                target: OptTarget::Global,
                name: text("x"),
            }),
            Column::Unknown(Raw::new("odd")),
        ]);
    }
    assert_eq!(
        to_string(&mztab).as_deref(),
        Ok("SMH\tSML_ID\tabundance_assay[1]\tid_confidence_measure[2]\topt_global_x\todd\n"),
        "all kinds"
    );
}

/// A layout that names the same header twice (only a model edited in code
/// can) writes it once.
#[test]
fn a_header_is_written_once() {
    let mut mztab = parse("SFH\tSMF_ID\n").mztab;
    let line = mztab.layout.lines.first().cloned().expect("one line");
    mztab.layout.lines.push(line);
    assert_eq!(to_string(&mztab).as_deref(), Ok("SFH\tSMF_ID\n"), "once");
}

fn unwritable(mztab: &MzTab<'_>) -> Option<&'static str> {
    match to_string(mztab) {
        Err(WriteError::Unwritable { reason, .. }) => Some(reason),
        Ok(_) | Err(_) => None,
    }
}

#[test]
fn unwritable_values_are_refused() {
    let mut m = MzTab::default();
    m.metadata.title = Some(Cell::Invalid(Raw::new("null")));
    assert_eq!(
        unwritable(&m),
        Some("invalid text `null`, which reads back as null"),
        "null raw"
    );
    m.metadata.title = Some(Cell::Invalid(Raw::new("a\nb")));
    assert_eq!(
        unwritable(&m),
        Some("raw text with a tab or line break"),
        "newline raw"
    );
    m.metadata.title = Some(Cell::Invalid(Raw::new("ends\r")));
    assert_eq!(
        unwritable(&m),
        Some("a line ending in `\\r`, which reads back as a CRLF ending"),
        "trailing CR"
    );
    m.metadata.title = None;
    let mut sml = parse("SMH\tSML_ID\tchemical_name\nSML\t1\tnull\n")
        .mztab
        .sml
        .expect("table");
    if let Some(row) = sml.rows.first_mut() {
        row.chemical_name = Cell::Value(vec![Cell::Value(text("a|b"))]);
    }
    m.sml = Some(sml.clone());
    assert_eq!(
        unwritable(&m),
        Some("a list item containing `|`"),
        "pipe in item"
    );
    if let Some(row) = sml.rows.first_mut() {
        row.chemical_name = Cell::Value(Vec::new());
    }
    m.sml = Some(sml);
    assert_eq!(unwritable(&m), Some("an empty list"), "empty list");
    m.sml = None;
    let param = Param::parse("[UO, UO:0000010, second, ]").expect("valid param");
    m.metadata.colunit_small_molecule = vec![Cell::Value(ColUnit {
        column: text("a=b"),
        param,
    })];
    assert_eq!(
        unwritable(&m),
        Some("a unit column name containing `=`"),
        "= in column"
    );
    m.metadata.colunit_small_molecule.clear();
    m.metadata.unknown = vec![(Raw::new("k\tk"), Raw::new("v"))];
    assert_eq!(
        unwritable(&m),
        Some("raw text with a tab or line break"),
        "tab in unknown key"
    );
}

/// The writer splits lists exactly as the reader does: a `|` inside an open
/// `[` or `"` of a parameter list belongs to its item.
#[test]
fn list_items_split_as_the_reader_splits_them() {
    // Found by the `roundtrip` fuzz target: one invalid item, `[|`.
    let source = "MTD\tsample[1]-cell_type[1]\t[|";
    assert_eq!(
        to_string(&parse(source).mztab).as_deref(),
        Ok(source),
        "fuzz regression"
    );
    let canonical = without_layout(parse(source).mztab);
    assert_eq!(
        to_string(&canonical).as_deref(),
        Ok("MTD\tsample[1]-cell_type[1]\t[|\n"),
        "canonical keeps the one item"
    );

    let opens = Param::parse("[, , [x, ]").expect("a name may hold `[`");
    let closes = Param::parse("[, , y, ]").expect("valid param");
    let mut m = MzTab::default();
    m.metadata.custom.insert(
        mztab_m::Ix::new(1).expect("non-zero"),
        Cell::Value(vec![Cell::Value(opens), Cell::Value(closes)]),
    );
    assert_eq!(
        unwritable(&m),
        Some("a list item that leaves `[` or `\"` open"),
        "open bracket"
    );

    m.metadata.custom = mztab_m::IndexMap::new();
    m.metadata.custom.insert(
        mztab_m::Ix::new(1).expect("non-zero"),
        Cell::Value(vec![Cell::Null]),
    );
    assert_eq!(
        unwritable(&m),
        Some("a list of one `null`, which reads back as a null cell"),
        "single null"
    );
}

#[test]
fn layout_lines_must_read_back_as_themselves() {
    let mut blank = parse("\nMTD\tmzTab-ID\tx\n").mztab;
    blank.layout.lines.iter_mut().for_each(|line| {
        if let mztab_m::LineContent::Extra(mztab_m::ExtraLine::Blank(raw)) = &mut line.content {
            *raw = Raw::new("text");
        }
    });
    assert_eq!(
        unwritable(&blank),
        Some("a blank line with text, or a comment without `COM`"),
        "blank with text"
    );
    let mut comment = parse("COM\tx\n").mztab;
    comment.layout.lines.iter_mut().for_each(|line| {
        line.content =
            mztab_m::LineContent::Extra(mztab_m::ExtraLine::Unrecognized(Raw::new("a\nb")));
    });
    assert_eq!(
        unwritable(&comment),
        Some("raw text with a tab or line break"),
        "newline"
    );
}

/// A `fmt::Write` that always fails.
struct Broken;

impl core::fmt::Write for Broken {
    fn write_str(&mut self, _: &str) -> core::fmt::Result {
        Err(core::fmt::Error)
    }
}

/// A `fmt::Write` that fails once `limit` bytes are written.
struct FailAfter {
    limit: usize,
}

impl core::fmt::Write for FailAfter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.limit = self.limit.checked_sub(s.len()).ok_or(core::fmt::Error)?;
        Ok(())
    }
}

#[test]
fn output_errors_are_reported() {
    let param = without_layout(parse("MTD\tquantification_method\t[,,x,]\n").mztab);
    let midway = &mut FailAfter { limit: 30 };
    assert_eq!(
        mztab_m::write(&param, midway),
        Err(WriteError::Fmt),
        "fails inside a parameter"
    );
    let mztab = parse("MTD\tmzTab-ID\tx\n").mztab;
    assert_eq!(
        mztab_m::write(&mztab, &mut Broken),
        Err(WriteError::Fmt),
        "fmt error"
    );
    assert_eq!(WriteError::Fmt.to_string(), "the output failed", "message");
    assert_eq!(
        WriteError::Unwritable {
            line: 3,
            reason: "x"
        }
        .to_string(),
        "cannot write output line 3: x",
        "message"
    );
}
