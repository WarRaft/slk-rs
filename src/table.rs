//! The sheet a document describes. Records are read in order with a
//! cursor: `Y` and `X` of `C` and `F` records move it (a field left out
//! keeps the row or column), and a `C` record sets the cell under it. The
//! cursor starts at row 1, column 1. `B;Y..;X..` gives the size, which
//! readers needn't trust: cells may lie outside it.

use std::collections::BTreeMap;

use crate::ast::{Document, Record, Value};

/// One cell: what its `C` record says.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// `K`: the value; [`Value::Empty`] for a cell with only a formula or
    /// a comment.
    pub value: Value,
    /// `E`: the formula that computed it, if any.
    pub formula: Option<String>,
    /// `A`: a comment.
    pub comment: Option<String>,
    /// The index of the `C` record that sets it, in [`Document::records`]:
    /// the one with the value when a comment has a record of its own.
    pub record: usize,
}

/// Rows and columns count from 1.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    cells: BTreeMap<(u32, u32), Cell>,
    /// `B`'s rows and columns, when it has them.
    pub bounds: Option<(u32, u32)>,
    /// Cells whose value or formula is set twice: (the later record, the
    /// earlier one). A comment in a record of its own isn't one.
    pub duplicates: Vec<(usize, usize)>,
}

/// Does a record of this type move the cursor?
pub fn moves_cursor(record: &Record) -> bool {
    record.kind == "C" || record.kind == "F"
}

/// The cursor after each record, (row, column), in record order.
pub fn cursors(doc: &Document) -> Vec<(u32, u32)> {
    let mut at = (1, 1);
    doc.records
        .iter()
        .map(|r| {
            if moves_cursor(r) {
                at = (
                    r.coordinate('Y').unwrap_or(at.0),
                    r.coordinate('X').unwrap_or(at.1),
                );
            }
            at
        })
        .collect()
}

impl Table {
    pub fn new(doc: &Document) -> Table {
        let mut table = Table::default();
        if let Some(b) = doc.record("B")
            && let (Some(Value::Number(y)), Some(Value::Number(x))) = (
                b.field('Y').map(|f| &f.value),
                b.field('X').map(|f| &f.value),
            )
            && let (Ok(y), Ok(x)) = (y.parse(), x.parse())
        {
            table.bounds = Some((y, x));
        }
        for (i, (record, at)) in doc.records.iter().zip(cursors(doc)).enumerate() {
            if record.kind != "C" {
                continue;
            }
            let text = |name| match record.field(name).map(|f| &f.value) {
                Some(Value::Formula(s) | Value::Comment(s) | Value::Text(s)) => Some(s.clone()),
                _ => None,
            };
            let value = record.field('K').map_or(Value::Empty, |f| f.value.clone());
            let cell = Cell {
                value,
                formula: text('E'),
                comment: text('A'),
                record: i,
            };
            let holds = |c: &Cell| c.value != Value::Empty || c.formula.is_some();
            match table.cells.get_mut(&at) {
                // Excel writes a comment as a record of its own, before or
                // after the value's.
                Some(old) if !holds(&cell) || !holds(old) => {
                    if holds(&cell) {
                        let comment = old.comment.take();
                        *old = cell;
                        old.comment = old.comment.take().or(comment);
                    } else {
                        old.comment = cell.comment.or(old.comment.take());
                    }
                }
                Some(old) => {
                    table.duplicates.push((i, old.record));
                    let comment = old.comment.take();
                    *old = cell;
                    old.comment = old.comment.take().or(comment);
                }
                None => {
                    table.cells.insert(at, cell);
                }
            }
        }
        table
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&(row, col))
    }

    /// The text a table shows at a cell: its value, empty for none.
    pub fn text(&self, row: u32, col: u32) -> String {
        self.get(row, col)
            .map_or(String::new(), |c| c.value.display())
    }

    /// Every cell, by row, then column.
    pub fn cells(&self) -> impl Iterator<Item = ((u32, u32), &Cell)> {
        self.cells.iter().map(|(&at, c)| (at, c))
    }

    /// The rows: the last row with a cell, or `B`'s, whichever is more.
    pub fn rows(&self) -> u32 {
        let used = self.cells.keys().map(|&(r, _)| r).max().unwrap_or(0);
        used.max(self.bounds.map_or(0, |b| b.0))
    }

    /// The columns, likewise.
    pub fn cols(&self) -> u32 {
        let used = self.cells.keys().map(|&(_, c)| c).max().unwrap_or(0);
        used.max(self.bounds.map_or(0, |b| b.1))
    }
}

#[cfg(test)]
#[path = "table.test.rs"]
mod tests;
