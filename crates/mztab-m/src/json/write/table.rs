//! Pre-flight table validation and streaming JSON row serialization.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt;

use super::emit::JsonWriter;
use super::{Section, split_key, unwritable};
use crate::json::schema::{Col, TABLES, Table, standard_header};
use crate::write::WriteError;

enum Kind<'t> {
    Stable(&'static str, Col),
    /// An abundance array: its JSON name, its index in `table.arrays`, and id.
    Array(&'static str, usize, u32),
    Opt(&'t str),
}

enum RowMember<'t> {
    Stable(&'static str, usize, Col),
    Array(&'static str, Vec<usize>),
    Opt(Vec<(&'t str, usize)>),
}

pub(super) struct TablePlan<'t> {
    row_prefix: &'static str,
    header_prefix: &'static str,
    members: Vec<RowMember<'t>>,
    rows: &'t [(u32, Vec<&'t str>)],
}

pub(super) fn plan_tables<'t>(
    tables: &'t BTreeMap<&'t str, Section<'t>>,
    ids: &BTreeMap<&'static str, Vec<u32>>,
) -> Result<[Option<TablePlan<'t>>; 3], WriteError> {
    let [t0, t1, t2] = &TABLES;
    Ok([
        tables
            .get(t0.header)
            .map(|s| plan_one(t0, s, ids))
            .transpose()?,
        tables
            .get(t1.header)
            .map(|s| plan_one(t1, s, ids))
            .transpose()?,
        tables
            .get(t2.header)
            .map(|s| plan_one(t2, s, ids))
            .transpose()?,
    ])
}

fn kind<'t>(table: &Table, column: &'t str) -> Option<Kind<'t>> {
    if let Some((_, json_key, col)) = table.columns.iter().find(|(c, ..)| *c == column) {
        return Some(Kind::Stable(json_key, *col));
    }
    if let Some(name) = column.strip_prefix("opt_") {
        return Some(Kind::Opt(name));
    }
    let (name, n, rest) = split_key(column)?;
    let (at, (json, ..)) = table
        .arrays
        .iter()
        .enumerate()
        .find(|(_, (_, _, c))| *c == name)?;
    rest.is_empty().then_some(Kind::Array(json, at, n))
}

fn plan_one<'t>(
    table: &Table,
    section: &'t Section<'t>,
    ids: &BTreeMap<&'static str, Vec<u32>>,
) -> Result<TablePlan<'t>, WriteError> {
    let distinct: BTreeSet<&str> = section.columns.iter().copied().collect();
    if distinct.len() != section.columns.len() {
        return Err(unwritable(section.line, "header repeats a column"));
    }
    let kinds = section
        .columns
        .iter()
        .map(|c| kind(table, c))
        .collect::<Option<Vec<Kind<'t>>>>()
        .ok_or_else(|| unwritable(section.line, "column has no JSON form"))?;
    check_arrays(table, section.line, &kinds, ids)?;
    if section.rows.is_empty() {
        let standard = standard_header(table, |e| ids.get(e).cloned().unwrap_or_default());
        if section
            .columns
            .iter()
            .copied()
            .ne(standard.iter().map(alloc::string::String::as_str))
        {
            return Err(unwritable(
                section.line,
                "table without rows has a header other than the standard one",
            ));
        }
    }
    if let Some((line, _)) = section
        .rows
        .iter()
        .find(|(_, cells)| cells.len() != kinds.len())
    {
        return Err(unwritable(
            *line,
            "row has another number of cells than its header",
        ));
    }
    Ok(TablePlan {
        row_prefix: table.row,
        header_prefix: table.header,
        members: build_members(&kinds),
        rows: &section.rows,
    })
}

fn check_arrays(
    table: &Table,
    line: u32,
    kinds: &[Kind<'_>],
    ids: &BTreeMap<&'static str, Vec<u32>>,
) -> Result<(), WriteError> {
    for (at, (_, element, _)) in table.arrays.iter().enumerate() {
        let found: Vec<u32> = kinds
            .iter()
            .filter_map(|k| match k {
                Kind::Array(_, a, n) if *a == at => Some(*n),
                _ => None,
            })
            .collect();
        let expected = ids.get(element).map_or_default(Vec::as_slice);
        if !found.is_empty() && found.as_slice() != expected {
            return Err(unwritable(
                line,
                "abundance columns do not match the metadata ids one to one, in order",
            ));
        }
    }
    Ok(())
}

/// Groups header columns into JSON members ordered by first appearance.
fn build_members<'t>(kinds: &[Kind<'t>]) -> Vec<RowMember<'t>> {
    let mut members: Vec<RowMember<'t>> = Vec::new();
    for (idx, k) in kinds.iter().enumerate() {
        match k {
            Kind::Stable(json_key, col) => members.push(RowMember::Stable(json_key, idx, *col)),
            Kind::Array(json_key, ..) => {
                if let Some(RowMember::Array(_, indices)) = members
                    .iter_mut()
                    .find(|m| matches!(m, RowMember::Array(k, _) if *k == *json_key))
                {
                    indices.push(idx);
                } else {
                    members.push(RowMember::Array(json_key, Vec::from([idx])));
                }
            }
            Kind::Opt(name) => {
                if let Some(RowMember::Opt(items)) =
                    members.iter_mut().find(|m| matches!(m, RowMember::Opt(_)))
                {
                    items.push((name, idx));
                } else {
                    members.push(RowMember::Opt(Vec::from([(*name, idx)])));
                }
            }
        }
    }
    members
}

pub(super) fn write_table(w: &mut JsonWriter<'_>, plan: &TablePlan<'_>) -> fmt::Result {
    w.char('[')?;
    for (i, (_, cells)) in plan.rows.iter().enumerate() {
        if i > 0 {
            w.char(',')?;
        }
        write_row(w, plan, cells)?;
    }
    w.char(']')
}

fn write_row(w: &mut JsonWriter<'_>, plan: &TablePlan<'_>, cells: &[&str]) -> fmt::Result {
    w.raw("{\"prefix\":")?;
    w.str(plan.row_prefix)?;
    w.raw(",\"header_prefix\":")?;
    w.str(plan.header_prefix)?;
    for member in &plan.members {
        w.char(',')?;
        match member {
            RowMember::Stable(json_key, idx, col) => {
                w.str(json_key)?;
                w.char(':')?;
                w.cell(cells.get(*idx).copied().unwrap_or(""), *col)?;
            }
            RowMember::Array(json_key, indices) => {
                w.str(json_key)?;
                w.raw(":[")?;
                for (i, idx) in indices.iter().enumerate() {
                    if i > 0 {
                        w.char(',')?;
                    }
                    w.cell(cells.get(*idx).copied().unwrap_or(""), Col::Number)?;
                }
                w.char(']')?;
            }
            RowMember::Opt(items) => {
                w.raw("\"opt\":[")?;
                for (i, (name, idx)) in items.iter().enumerate() {
                    if i > 0 {
                        w.char(',')?;
                    }
                    w.raw("{\"identifier\":")?;
                    w.str(name)?;
                    w.raw(",\"param\":null,\"value\":")?;
                    let text = cells.get(*idx).copied().unwrap_or("");
                    if text == "null" {
                        w.raw("null")?;
                    } else {
                        w.str(text)?;
                    }
                    w.char('}')?;
                }
                w.char(']')?;
            }
        }
    }
    w.char('}')
}
