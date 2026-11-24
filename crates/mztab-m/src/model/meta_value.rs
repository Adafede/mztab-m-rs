//! Typed metadata value variants, borrowed views, and [`InsertError`].

use super::List;
use super::colunit::ColUnit;
use crate::id::{AssayId, InstrumentId, MsRunId, ProtocolId, SampleId, StudyVariableId};
use crate::meta::key::MtdKey;
use crate::param::Param;
use crate::value::cell::Cell;
use crate::value::text::{Raw, Text};
use crate::vocab::field::{Datatype, PublicationItem};
use crate::vocab::profile::Profile;
use crate::vocab::version::Version;

/// A typed metadata value. Each variant matches one [`ValueKind`](crate::meta::key::ValueKind).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MtdValue<'a> {
    /// `ValueKind::Version`.
    Version(Cell<'a, Version>),
    /// `ValueKind::Profile`.
    Profile(Cell<'a, Profile>),
    /// `ValueKind::Datatype`.
    Datatype(Cell<'a, Datatype>),
    /// `ValueKind::Text`.
    Text(Cell<'a, Text<'a>>),
    /// `ValueKind::Param`.
    Param(Cell<'a, Param<'a>>),
    /// `ValueKind::ParamList`.
    ParamList(Cell<'a, List<'a, Param<'a>>>),
    /// `ValueKind::Publication`.
    Publication(Cell<'a, List<'a, PublicationItem<'a>>>),
    /// `ValueKind::InstrumentRef`.
    InstrumentRef(Cell<'a, InstrumentId>),
    /// `ValueKind::SampleRef`.
    SampleRef(Cell<'a, SampleId>),
    /// `ValueKind::MsRunRefs`.
    MsRunRefs(Cell<'a, List<'a, MsRunId>>),
    /// `ValueKind::ProtocolRefs`.
    ProtocolRefs(Cell<'a, List<'a, ProtocolId>>),
    /// `ValueKind::AssayRefs`.
    AssayRefs(Cell<'a, List<'a, AssayId>>),
    /// `ValueKind::StudyVariableRefs`.
    StudyVariableRefs(Cell<'a, List<'a, StudyVariableId>>),
    /// `ValueKind::ColUnit`.
    ColUnit(Cell<'a, ColUnit<'a>>),
}

/// Why [`Metadata::insert`](super::Metadata::insert) did not place a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InsertError {
    /// The key already has a value. Only `colunit-*` keys repeat.
    Occupied,
    /// The value is not of the key's [`ValueKind`](crate::meta::key::ValueKind).
    WrongKind,
}

/// One `MTD` line: the key and value as written, and their typed form.
///
/// `typed` is `None` when the key is unknown; the line then belongs in
/// [`Metadata::unknown`](super::Metadata::unknown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MtdEntry<'a> {
    /// The key cell, as written.
    pub key: Raw<'a>,
    /// The value cell, as written (empty if the line has no value cell).
    pub value: Raw<'a>,
    /// Everything after the value cell, starting with its tab; usually
    /// empty, sometimes a run of empty cells.
    pub trailing: Raw<'a>,
    /// The parsed key and typed value.
    pub typed: Option<(MtdKey, MtdValue<'a>)>,
}

/// A borrowed metadata value, as [`Metadata::entries`](super::Metadata::entries) lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueRef<'r, 'a> {
    /// See [`MtdValue::Version`].
    Version(&'r Cell<'a, Version>),
    /// See [`MtdValue::Profile`].
    Profile(&'r Cell<'a, Profile>),
    /// See [`MtdValue::Datatype`].
    Datatype(&'r Cell<'a, Datatype>),
    /// See [`MtdValue::Text`].
    Text(&'r Cell<'a, Text<'a>>),
    /// See [`MtdValue::Param`].
    Param(&'r Cell<'a, Param<'a>>),
    /// See [`MtdValue::ParamList`].
    ParamList(&'r Cell<'a, List<'a, Param<'a>>>),
    /// See [`MtdValue::Publication`].
    Publication(&'r Cell<'a, List<'a, PublicationItem<'a>>>),
    /// See [`MtdValue::InstrumentRef`].
    InstrumentRef(&'r Cell<'a, InstrumentId>),
    /// See [`MtdValue::SampleRef`].
    SampleRef(&'r Cell<'a, SampleId>),
    /// See [`MtdValue::MsRunRefs`].
    MsRunRefs(&'r Cell<'a, List<'a, MsRunId>>),
    /// See [`MtdValue::ProtocolRefs`].
    ProtocolRefs(&'r Cell<'a, List<'a, ProtocolId>>),
    /// See [`MtdValue::AssayRefs`].
    AssayRefs(&'r Cell<'a, List<'a, AssayId>>),
    /// See [`MtdValue::StudyVariableRefs`].
    StudyVariableRefs(&'r Cell<'a, List<'a, StudyVariableId>>),
    /// See [`MtdValue::ColUnit`].
    ColUnit(&'r Cell<'a, ColUnit<'a>>),
}

impl MtdValue<'_> {
    /// The value, borrowed.
    pub(crate) const fn as_ref(&self) -> ValueRef<'_, '_> {
        match self {
            Self::Version(c) => ValueRef::Version(c),
            Self::Profile(c) => ValueRef::Profile(c),
            Self::Datatype(c) => ValueRef::Datatype(c),
            Self::Text(c) => ValueRef::Text(c),
            Self::Param(c) => ValueRef::Param(c),
            Self::ParamList(c) => ValueRef::ParamList(c),
            Self::Publication(c) => ValueRef::Publication(c),
            Self::InstrumentRef(c) => ValueRef::InstrumentRef(c),
            Self::SampleRef(c) => ValueRef::SampleRef(c),
            Self::MsRunRefs(c) => ValueRef::MsRunRefs(c),
            Self::ProtocolRefs(c) => ValueRef::ProtocolRefs(c),
            Self::AssayRefs(c) => ValueRef::AssayRefs(c),
            Self::StudyVariableRefs(c) => ValueRef::StudyVariableRefs(c),
            Self::ColUnit(c) => ValueRef::ColUnit(c),
        }
    }
}
