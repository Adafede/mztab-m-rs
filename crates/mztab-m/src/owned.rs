//! [`IntoOwned`]: detaching a borrowed model or event from its input.

mod model;

use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::id::IndexMap;
use crate::model::{Column, ExtraLine, Header, LineContent, MtdValue, Table};
use crate::param::{Accession, Param, ParamText};
use crate::read::Event;
use crate::value::{Cell, Decimal, Raw, Text};

/// Converts a value that borrows its input into one that owns its data.
///
/// The reader's model borrows the input text, which is what makes reading
/// zero-copy. Call `into_owned` to keep a model, a row or an event after the
/// input is gone.
///
/// # Examples
///
/// ```
/// use mztab_m::{IntoOwned, MzTab};
///
/// fn read(input: String) -> MzTab<'static> {
///     mztab_m::parse(&input).mztab.into_owned()
/// }
/// let mztab = read("MTD\tmzTab-ID\tEX-1\n".to_owned());
/// assert!(mztab.metadata.id.is_some());
/// ```
pub trait IntoOwned {
    /// The owned form.
    type Owned: 'static;

    /// Copies every borrowed string.
    fn into_owned(self) -> Self::Owned;
}

macro_rules! owned_copy {
    ($($ty:ty),+ $(,)?) => {$(
        impl IntoOwned for $ty {
            type Owned = Self;

            fn into_owned(self) -> Self {
                self
            }
        }
    )+};
}

// Only the types that sit inside converted values; index keys and ids that
// are only ever map keys are copied as they are.
owned_copy! {
    u32, i32, bool,
    crate::id::AssayId, crate::id::InstrumentId, crate::id::MsRunId, crate::id::ProtocolId,
    crate::id::SampleId, crate::id::StudyVariableId,
    crate::model::SmlId, crate::model::SmfId, crate::model::SmeId,
    crate::model::OptTarget, crate::model::LineInfo,
    crate::vocab::Version, crate::vocab::Profile, crate::vocab::AmbiguityCode,
    crate::vocab::PublicationKind, crate::vocab::Datatype,
    crate::meta::MtdKey, crate::diag::Findings, crate::model::LineEnding, u64,
}

fn cow(text: Cow<'_, str>) -> Cow<'static, str> {
    Cow::Owned(text.into_owned())
}

impl IntoOwned for Raw<'_> {
    type Owned = Raw<'static>;

    fn into_owned(self) -> Raw<'static> {
        Raw::owned(self.into_cow().into_owned())
    }
}

impl IntoOwned for Text<'_> {
    type Owned = Text<'static>;

    fn into_owned(self) -> Text<'static> {
        Text::from_cow_unchecked(cow(self.into_cow()))
    }
}

impl IntoOwned for ParamText<'_> {
    type Owned = ParamText<'static>;

    fn into_owned(self) -> ParamText<'static> {
        ParamText::from_cow_unchecked(cow(self.into_cow()))
    }
}

impl IntoOwned for Accession<'_> {
    type Owned = Accession<'static>;

    fn into_owned(self) -> Accession<'static> {
        Accession::new(self.into_text().into_owned())
    }
}

impl IntoOwned for Decimal<'_> {
    type Owned = Decimal<'static>;

    fn into_owned(self) -> Decimal<'static> {
        self.into_static()
    }
}

impl<T: IntoOwned> IntoOwned for Cell<'_, T> {
    type Owned = Cell<'static, T::Owned>;

    fn into_owned(self) -> Self::Owned {
        match self {
            Cell::Value(value) => Cell::Value(value.into_owned()),
            Cell::Null => Cell::Null,
            Cell::Invalid(raw) => Cell::Invalid(raw.into_owned()),
        }
    }
}

impl<T: IntoOwned> IntoOwned for Vec<T> {
    type Owned = Vec<T::Owned>;

    fn into_owned(self) -> Self::Owned {
        self.into_iter().map(IntoOwned::into_owned).collect()
    }
}

impl<T: IntoOwned> IntoOwned for Option<T> {
    type Owned = Option<T::Owned>;

    fn into_owned(self) -> Self::Owned {
        self.map(IntoOwned::into_owned)
    }
}

impl<A: IntoOwned, B: IntoOwned> IntoOwned for (A, B) {
    type Owned = (A::Owned, B::Owned);

    fn into_owned(self) -> Self::Owned {
        (self.0.into_owned(), self.1.into_owned())
    }
}

impl<K: Ord + Copy + 'static, V: IntoOwned> IntoOwned for IndexMap<K, V> {
    type Owned = IndexMap<K, V::Owned>;

    fn into_owned(self) -> Self::Owned {
        self.into_iter().map(|(k, v)| (k, v.into_owned())).collect()
    }
}

impl IntoOwned for Param<'_> {
    type Owned = Param<'static>;

    fn into_owned(self) -> Param<'static> {
        match self {
            Param::Cv {
                label,
                accession,
                name,
                value,
            } => Param::Cv {
                label: label.into_owned(),
                accession: accession.into_owned(),
                name: name.into_owned(),
                value: value.into_owned(),
            },
            Param::User { name, value } => Param::User {
                name: name.into_owned(),
                value: value.into_owned(),
            },
        }
    }
}

impl<S: Copy + 'static> IntoOwned for Column<'_, S> {
    type Owned = Column<'static, S>;

    fn into_owned(self) -> Self::Owned {
        match self {
            Column::Stable(s) => Column::Stable(s),
            Column::Abundance(a) => Column::Abundance(a),
            Column::IdConfidence(i) => Column::IdConfidence(i),
            Column::Opt(o) => Column::Opt(o.into_owned()),
            Column::Unknown(raw) => Column::Unknown(raw.into_owned()),
        }
    }
}

impl<S: Copy + 'static> IntoOwned for Header<'_, S> {
    type Owned = Header<'static, S>;

    fn into_owned(self) -> Self::Owned {
        Header {
            columns: self.columns.into_owned(),
        }
    }
}

impl<S: Copy + 'static, R: IntoOwned> IntoOwned for Table<'_, S, R> {
    type Owned = Table<'static, S, R::Owned>;

    fn into_owned(self) -> Self::Owned {
        Table {
            header: self.header.into_owned(),
            rows: self.rows.into_owned(),
        }
    }
}

impl IntoOwned for ExtraLine<'_> {
    type Owned = ExtraLine<'static>;

    fn into_owned(self) -> ExtraLine<'static> {
        match self {
            ExtraLine::Blank(raw) => ExtraLine::Blank(raw.into_owned()),
            ExtraLine::Comment(raw) => ExtraLine::Comment(raw.into_owned()),
            ExtraLine::Unrecognized(raw) => ExtraLine::Unrecognized(raw.into_owned()),
        }
    }
}

impl IntoOwned for LineContent<'_> {
    type Owned = LineContent<'static>;

    fn into_owned(self) -> LineContent<'static> {
        match self {
            LineContent::Metadata(key) => LineContent::Metadata(key),
            LineContent::UnknownMetadata => LineContent::UnknownMetadata,
            LineContent::Header(section) => LineContent::Header(section),
            LineContent::Row(section) => LineContent::Row(section),
            LineContent::Extra(line) => LineContent::Extra(line.into_owned()),
        }
    }
}

impl IntoOwned for MtdValue<'_> {
    type Owned = MtdValue<'static>;

    fn into_owned(self) -> MtdValue<'static> {
        match self {
            MtdValue::Version(c) => MtdValue::Version(c.into_owned()),
            MtdValue::Profile(c) => MtdValue::Profile(c.into_owned()),
            MtdValue::Datatype(c) => MtdValue::Datatype(c.into_owned()),
            MtdValue::Text(c) => MtdValue::Text(c.into_owned()),
            MtdValue::Param(c) => MtdValue::Param(c.into_owned()),
            MtdValue::ParamList(c) => MtdValue::ParamList(c.into_owned()),
            MtdValue::Publication(c) => MtdValue::Publication(c.into_owned()),
            MtdValue::InstrumentRef(c) => MtdValue::InstrumentRef(c.into_owned()),
            MtdValue::SampleRef(c) => MtdValue::SampleRef(c.into_owned()),
            MtdValue::MsRunRefs(c) => MtdValue::MsRunRefs(c.into_owned()),
            MtdValue::ProtocolRefs(c) => MtdValue::ProtocolRefs(c.into_owned()),
            MtdValue::AssayRefs(c) => MtdValue::AssayRefs(c.into_owned()),
            MtdValue::StudyVariableRefs(c) => MtdValue::StudyVariableRefs(c.into_owned()),
            MtdValue::ColUnit(c) => MtdValue::ColUnit(c.into_owned()),
        }
    }
}

impl IntoOwned for Event<'_> {
    type Owned = Event<'static>;

    fn into_owned(self) -> Event<'static> {
        match self {
            Event::Bom => Event::Bom,
            Event::Blank(i, raw) => Event::Blank(i, raw.into_owned()),
            Event::Comment(i, raw) => Event::Comment(i, raw.into_owned()),
            Event::Unrecognized(i, raw) => Event::Unrecognized(i, raw.into_owned()),
            Event::Metadata(i, entry) => Event::Metadata(i, entry.into_owned()),
            Event::SmlHeader(i, h) => Event::SmlHeader(i, h.into_owned()),
            Event::SmfHeader(i, h) => Event::SmfHeader(i, h.into_owned()),
            Event::SmeHeader(i, h) => Event::SmeHeader(i, h.into_owned()),
            Event::Sml(i, row) => Event::Sml(i, row.into_owned()),
            Event::Smf(i, row) => Event::Smf(i, row.into_owned()),
            Event::Sme(i, row) => Event::Sme(i, row.into_owned()),
            Event::Verbatim(v) => Event::Verbatim(v.into_owned()),
            Event::Finding(f) => Event::Finding(f),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::IntoOwned as _;
    use crate::id::{AssayId, IdConfidenceMeasureId};
    use crate::model::{AbundanceColumn, Column, Header, OptColumn, OptTarget, SmlColumn};
    use crate::value::{Raw, Text};
    use alloc::string::String;
    use alloc::vec;

    /// Every column kind, in one table type, survives `into_owned` (the
    /// reader never puts an SME column in an SML header, but the type allows
    /// it).
    #[test]
    fn leaf_texts_keep_their_text() {
        use crate::param::{Accession, ParamText};
        let text = ParamText::new("MS:1").expect("valid parameter text");
        assert_eq!(text.clone().into_owned(), text, "parameter text");
        let accession = Accession::new(text);
        assert_eq!(accession.clone().into_owned(), accession, "accession");
        let plain = Text::new("x").expect("valid text");
        assert_eq!(plain.clone().into_owned(), plain, "text");
        assert_eq!(Raw::new("r").into_owned(), Raw::new("r"), "raw");
    }

    #[test]
    fn every_column_kind_converts() {
        let text = String::from("x");
        let header: Header<'_, SmlColumn> = Header {
            columns: vec![
                Column::Stable(SmlColumn::SmlId),
                Column::Abundance(AbundanceColumn::Assay(AssayId::new(1).expect("non-zero"))),
                Column::IdConfidence(IdConfidenceMeasureId::new(1).expect("non-zero")),
                Column::Opt(OptColumn {
                    target: OptTarget::Global,
                    name: Text::new(&text).expect("valid text"),
                }),
                Column::Unknown(Raw::new(&text)),
            ],
        };
        let owned = header.clone().into_owned();
        assert_eq!(owned, header, "every kind kept");
    }
}
