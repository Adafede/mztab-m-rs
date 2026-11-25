//! Shared row-checking helpers and header column-position caches.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::diag::code::Code;
use crate::diag::finding::Detail;
use crate::diag::severity::Severity;
use crate::model::List;
use crate::model::table::{Column, Header};
use crate::validate::label;
use crate::validate::patterns::is_adduct;
use crate::validate::report::{Report, Site};
use crate::value::cell::Cell;
use crate::value::decimal::Decimal;
use crate::value::text::Text;
use crate::vocab::section::TableSection;

/// The header position of each non-stable column kind, to locate row cells.
pub(super) struct Positions {
    pub(super) numbers: Vec<usize>,
    pub(super) opt: Vec<usize>,
}

impl Positions {
    pub(super) fn of<S>(header: &Header<'_, S>) -> Self {
        let mut numbers = Vec::new();
        let mut opt = Vec::new();
        for (i, column) in header.columns.iter().enumerate() {
            match column {
                Column::Abundance(_) | Column::IdConfidence(_) => numbers.push(i),
                Column::Opt(_) => opt.push(i),
                Column::Stable(_) | Column::Unknown(_) => {}
            }
        }
        Self { numbers, opt }
    }
}

/// The header position of a stable column (or the end of the header, for
/// a value of a missing column).
pub(crate) fn position<S: PartialEq>(header: &Header<'_, S>, column: &S) -> usize {
    header
        .columns
        .iter()
        .position(|c| matches!(c, Column::Stable(x) if x == column))
        .unwrap_or(header.columns.len())
}

/// Per-row reporting context using precomputed column indices.
pub(super) struct Rows<'r, 'm, 'a> {
    pub(super) report: &'r mut Report<'m, 'a>,
    pub(super) section: TableSection,
    pub(super) header_len: usize,
    pub(super) row: usize,
}

impl Rows<'_, '_, '_> {
    pub(super) fn add(&mut self, code: Code, col: usize, detail: Detail) {
        self.report
            .add(code, Site::Cell(self.section, self.row, col), detail);
    }

    /// `null` in a column that is not nullable (and present in the header).
    pub(super) fn not_null<T>(&mut self, cell: &Cell<'_, T>, col: usize, severity: Severity) {
        if cell.is_null() && col < self.header_len {
            let site = Site::Cell(self.section, self.row, col);
            self.report
                .add_at(Code::NullValue, severity, site, Detail::None);
        }
    }

    /// A value that failed to type.
    pub(super) fn valid<T>(&mut self, cell: &Cell<'_, T>, col: usize) {
        if cell.invalid().is_some() {
            self.add(Code::InvalidValue, col, Detail::None);
        }
    }

    pub(super) fn valid_list<T>(&mut self, cell: &Cell<'_, List<'_, T>>, col: usize) {
        let items_invalid = cell
            .value()
            .is_some_and(|items| items.iter().any(|i| i.invalid().is_some()));
        if cell.invalid().is_some() || items_invalid {
            self.add(Code::InvalidValue, col, Detail::None);
        }
    }

    pub(super) fn number(&mut self, cell: &Cell<'_, Decimal<'_>>, col: usize) {
        if bad_decimal(cell) {
            self.add(Code::InvalidValue, col, Detail::None);
        }
    }

    pub(super) fn numbers(&mut self, cells: &[Cell<'_, Decimal<'_>>], positions: &[usize]) {
        for (cell, &at) in cells.iter().zip(positions) {
            if bad_decimal(cell) {
                self.add(Code::InvalidValue, at, Detail::None);
            }
        }
    }

    pub(super) fn opts(&mut self, cells: &[Cell<'_, Text<'_>>], positions: &[usize]) {
        for (cell, &at) in cells.iter().zip(positions) {
            if cell.invalid().is_some() {
                self.add(Code::InvalidValue, at, Detail::None);
            }
        }
    }

    pub(super) fn adduct(&mut self, cell: &Cell<'_, Text<'_>>, col: usize) {
        if cell.value().is_some_and(|t| !is_adduct(t.as_str())) {
            self.add(Code::AdductIon, col, Detail::None);
        }
    }
}

/// Duplicate row ids, keyed by the id's text.
pub(super) fn duplicates<'c, I: Ord + Copy + core::fmt::Display + 'c>(
    report: &mut Report<'_, '_>,
    section: TableSection,
    column: usize,
    ids: impl Iterator<Item = &'c Cell<'c, I>>,
) {
    let mut seen = BTreeSet::new();
    for (row, cell) in ids.enumerate() {
        if let Some(&id) = cell.value()
            && !seen.insert(id)
        {
            report.add(
                Code::DuplicateRowId,
                Site::Cell(section, row, column),
                label(id),
            );
        }
    }
}

/// The ids a list cell names, without intermediate allocation.
pub(super) fn refs<'a, I: Copy + 'a>(
    cell: &'a Cell<'a, List<'a, I>>,
) -> impl Iterator<Item = I> + 'a {
    cell.value()
        .into_iter()
        .flatten()
        .filter_map(|c| c.value().copied())
}

fn bad_decimal(cell: &Cell<'_, Decimal<'_>>) -> bool {
    cell.invalid().is_some() || cell.value().is_some_and(|d| !d.kind().is_conforming())
}
