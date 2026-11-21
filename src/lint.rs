//! Lint rules for SYLK: what a reader accepts but almost certainly isn't
//! meant. Malformed records are parse errors, in [`Document::errors`].
//!
//! - `duplicate-cell`: a cell set twice; the later record wins.
//! - `cell-outside-bounds`: a cell past the size `B` gives. Excel reads it
//!   anyway; other readers (Warcraft III among them) may size the table
//!   from `B` and lose it.

use crate::ast::Document;
use crate::table::Table;

use parser_rs::rule;
pub use parser_rs::{Diagnostic, Level};

/// Runs every lint rule over a document and returns what it finds, sorted
/// by line.
pub fn lint(doc: &Document) -> Vec<Diagnostic> {
    let table = Table::new(doc);
    let mut out = Vec::new();
    for &(later, earlier) in &table.duplicates {
        let r = &doc.records[later];
        out.push(
            Diagnostic::warning(
                rule::DUPLICATE_CELL,
                format!(
                    "this cell is already set at line {}; this record wins",
                    doc.records[earlier].line
                ),
                r.line,
            )
            .at(r.full_span()),
        );
    }
    if let Some((rows, cols)) = table.bounds {
        for ((row, col), cell) in table.cells() {
            if row > rows || col > cols {
                let r = &doc.records[cell.record];
                out.push(
                    Diagnostic::warning(
                        rule::CELL_OUTSIDE_BOUNDS,
                        format!("row {row}, column {col} is outside the {rows} rows and {cols} columns the B record gives"),
                        r.line,
                    )
                    .at(r.full_span()),
                );
            }
        }
    }
    out.sort_by_key(|d| d.line);
    out
}

#[cfg(test)]
#[path = "lint.test.rs"]
mod tests;
