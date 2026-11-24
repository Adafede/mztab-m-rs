//! Typing metadata cell values according to their key's [`ValueKind`].

use super::typing::{Cx, RawCell, Spaces, list, param, reference, text, typed};
use crate::diag::{Code, Detail};
use crate::meta::ValueKind;
use crate::model::{ColUnit, ColUnitError, MtdValue};
use crate::vocab::{Datatype, Profile, PublicationItem, PublicationItemError, Version};

/// Types a metadata value according to its key's [`ValueKind`].
pub(crate) fn mtd_value<'a>(cx: &mut Cx<'_>, kind: ValueKind, raw: RawCell<'a>) -> MtdValue<'a> {
    match kind {
        ValueKind::Version => {
            MtdValue::Version(typed(cx, raw, Spaces::Trim, Version::parse, |e| {
                (Code::VersionSyntax, e.offset, Detail::Version(e))
            }))
        }
        ValueKind::Profile => MtdValue::Profile(typed(
            cx,
            raw,
            Spaces::Trim,
            |s| Profile::parse(s).ok_or(()),
            |()| {
                (
                    Code::ProfileValue,
                    0,
                    Detail::Expected("`M`, `M+S`, `M+F`, `M+F+E` or `M+S+F+E`"),
                )
            },
        )),
        ValueKind::Datatype => MtdValue::Datatype(typed(
            cx,
            raw,
            Spaces::Trim,
            |s| Datatype::parse(s).ok_or(()),
            |()| {
                (
                    Code::ValueSyntax,
                    0,
                    Detail::Expected("an `xsd:` datatype or `Parameter`"),
                )
            },
        )),
        ValueKind::Text => MtdValue::Text(text(cx, raw)),
        ValueKind::Param => MtdValue::Param(param(cx, raw)),
        ValueKind::ParamList => MtdValue::ParamList(list(cx, raw, true, param)),
        ValueKind::Publication => MtdValue::Publication(list(cx, raw, false, |cx, item| {
            typed(
                cx,
                item,
                Spaces::Trim,
                PublicationItem::parse,
                |e| match e {
                    PublicationItemError::Qualifier => (
                        Code::ValueSyntax,
                        0,
                        Detail::Expected("`pubmed:`, `doi:` or `uri:` followed by an accession"),
                    ),
                    PublicationItemError::Accession(t) => (Code::TextCell, 0, Detail::Text(t)),
                },
            )
        })),
        ValueKind::InstrumentRef => MtdValue::InstrumentRef(reference(cx, raw, "`instrument[n]`")),
        ValueKind::SampleRef => MtdValue::SampleRef(reference(cx, raw, "`sample[n]`")),
        ValueKind::MsRunRefs => MtdValue::MsRunRefs(list(cx, raw, false, |cx, item| {
            reference(cx, item, "`ms_run[n]`")
        })),
        ValueKind::ProtocolRefs => MtdValue::ProtocolRefs(list(cx, raw, false, |cx, item| {
            reference(cx, item, "`protocol[n]`")
        })),
        ValueKind::AssayRefs => MtdValue::AssayRefs(list(cx, raw, false, |cx, item| {
            reference(cx, item, "`assay[n]`")
        })),
        ValueKind::StudyVariableRefs => {
            MtdValue::StudyVariableRefs(list(cx, raw, false, |cx, item| {
                reference(cx, item, "`study_variable[n]`")
            }))
        }
        ValueKind::ColUnit => {
            MtdValue::ColUnit(typed(cx, raw, Spaces::Trim, ColUnit::parse, |e| match e {
                ColUnitError::MissingEquals => (
                    Code::ValueSyntax,
                    0,
                    Detail::Expected("`column=[unit parameter]`"),
                ),
                ColUnitError::Column(t) => (Code::TextCell, 0, Detail::Text(t)),
                ColUnitError::Param(p) => {
                    let at = raw
                        .text
                        .trim_start_matches(' ')
                        .find('=')
                        .map_or(0, |eq| eq + 1);
                    (Code::ParamSyntax, at + p.offset, Detail::Param(p))
                }
            }))
        }
    }
}
