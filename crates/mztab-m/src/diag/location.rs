//! Finding [`Location`]s, [`Path`]s, and [`Segment`]s.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

/// One step of a [`Path`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    /// A named field or object key.
    Key(Cow<'static, str>),
    /// A position in a list or table. Zero-based in a JSON pointer, as JSON
    /// arrays are; one-based in a model path, as mzTab indices are.
    Index(u32),
}

/// A path into a JSON document or into the typed model.
///
/// # Examples
///
/// ```
/// use mztab_m::{Location, Path};
///
/// let path = Path::new().key("smallMoleculeSummary").index(3).key("chemical_name");
/// assert_eq!(Location::Json(path.clone()).to_string(), "/smallMoleculeSummary/3/chemical_name");
/// assert_eq!(Location::Model(path).to_string(), "smallMoleculeSummary[3].chemical_name");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Path(Vec<Segment>);

impl Path {
    /// The empty path: the document root.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends a key.
    #[must_use]
    pub fn key(mut self, key: impl Into<Cow<'static, str>>) -> Self {
        self.0.push(Segment::Key(key.into()));
        self
    }

    /// Appends an index.
    #[must_use]
    pub fn index(mut self, index: u32) -> Self {
        self.0.push(Segment::Index(index));
        self
    }

    /// The segments, from the root.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// Writes an RFC 6901 JSON pointer, escaping `~` and `/` in keys.
    fn fmt_pointer(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.0 {
            f.write_str("/")?;
            match segment {
                Segment::Key(key) => {
                    for c in key.chars() {
                        match c {
                            '~' => f.write_str("~0")?,
                            '/' => f.write_str("~1")?,
                            c => fmt::Write::write_char(f, c)?,
                        }
                    }
                }
                Segment::Index(i) => write!(f, "{i}")?,
            }
        }
        Ok(())
    }

    /// Writes `a.b[3].c`.
    fn fmt_model(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (n, segment) in self.0.iter().enumerate() {
            match segment {
                Segment::Key(key) if n == 0 => f.write_str(key)?,
                Segment::Key(key) => write!(f, ".{key}")?,
                Segment::Index(i) => write!(f, "[{i}]")?,
            }
        }
        Ok(())
    }
}

/// Where a finding applies.
///
/// # Examples
///
/// ```
/// use mztab_m::Location;
///
/// let at = Location::text(3, 1).with_cell(4);
/// assert_eq!(at.to_string(), "line 3, column 1, cell 4");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Location {
    /// A position in TSV text.
    Text {
        /// 1-based line number.
        line: u32,
        /// 1-based column, counted in Unicode scalar values.
        column: u32,
        /// 1-based cell number on the line (the prefix is cell 1), if the
        /// finding concerns one cell.
        cell: Option<u32>,
    },
    /// A position in a JSON document, as a JSON pointer.
    Json(Path),
    /// A position in a model built in code, which has no source text.
    Model(Path),
    /// The document as a whole, e.g. a missing mandatory field.
    Document,
}

impl Location {
    /// A text position with no cell.
    #[must_use]
    pub const fn text(line: u32, column: u32) -> Self {
        Self::Text {
            line,
            column,
            cell: None,
        }
    }

    /// Attaches a cell number to a text position; other locations are
    /// returned unchanged.
    #[must_use]
    pub fn with_cell(self, cell: u32) -> Self {
        match self {
            Self::Text { line, column, .. } => Self::Text {
                line,
                column,
                cell: Some(cell),
            },
            other => other,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { line, column, cell } => {
                write!(f, "line {line}, column {column}")?;
                cell.map_or(Ok(()), |cell| write!(f, ", cell {cell}"))
            }
            Self::Json(path) => path.fmt_pointer(f),
            Self::Model(path) => path.fmt_model(f),
            Self::Document => f.write_str("document"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{Location, Path};
    use alloc::string::ToString as _;

    #[test]
    fn locations_render() {
        assert_eq!(Location::text(2, 9).to_string(), "line 2, column 9", "text");
        assert_eq!(
            Location::text(2, 9).with_cell(4).to_string(),
            "line 2, column 9, cell 4",
            "cell"
        );
        assert_eq!(
            Location::Document.with_cell(3),
            Location::Document,
            "cell ignored"
        );
        assert_eq!(Location::Document.to_string(), "document", "document");
        let path = Path::new().key("a/b~c").index(0).key("d");
        assert_eq!(path.segments().len(), 3, "segments");
        assert_eq!(
            Location::Json(path.clone()).to_string(),
            "/a~1b~0c/0/d",
            "pointer escapes"
        );
        assert_eq!(Location::Model(path).to_string(), "a/b~c[0].d", "model");
        assert_eq!(Location::Json(Path::new()).to_string(), "", "root pointer");
        assert_eq!(
            Location::Model(Path::new().index(2)).to_string(),
            "[2]",
            "leading index"
        );
    }
}
