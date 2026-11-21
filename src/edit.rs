//! Changing a sheet through its text: [`set_cell`] gives the text edits
//! that write or clear one cell, so an editor keeps the file as text
//! (undo, save, and the text view all just work) and changes only what it
//! must.
//!
//! - A cell that has a `C` record gets its `K` replaced, or added.
//! - A new cell gets a `C` record in reading order: before the first cell
//!   after it, or before `E`. It names its row only when the cursor is on
//!   another one (`C;Y5;X2;K1`, or `C;X2;K1` in the same row).
//! - Clearing a cell drops its `K`, or its whole record when that's all it
//!   had.
//! - Records rely on the cursor (`X`, `Y` carry over), so when a record is
//!   added or dropped, the next record that moves the cursor gets the `X`
//!   or `Y` it now needs to stay where it was.
//! - A cell past `B`'s size grows it.

use parser_rs::{Edit, Span};

use crate::ast::{Document, Record, Value};
use crate::lexer::encode;
use crate::table::{Table, cursors, moves_cursor};

/// How a value is written in a `C;K` field.
pub fn literal(value: &Value) -> String {
    match value {
        Value::Str(s) => format!("\"{}\"", encode(s)),
        Value::Bool(true) => "TRUE".into(),
        Value::Bool(false) => "FALSE".into(),
        Value::Number(s)
        | Value::Error(s)
        | Value::Text(s)
        | Value::Formula(s)
        | Value::Comment(s) => s.replace(';', ";;"),
        Value::Empty => String::new(),
    }
}

/// What a person typed into a cell, as a value: `TRUE` / `FALSE`, an
/// error like `#VALUE!`, a number, or else text. A cell that holds text
/// keeps holding text (`123` stays the string `"123"`), and so does
/// anything in quotes (`"123"`). Empty input is no value.
pub fn parse_input(input: &str, previous: Option<&Value>) -> Option<Value> {
    if input.is_empty() {
        return None;
    }
    if let Some(inner) = input
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .filter(|_| input.len() >= 2)
    {
        return Some(Value::Str(inner.to_string()));
    }
    if matches!(previous, Some(Value::Str(_))) {
        return Some(Value::Str(input.to_string()));
    }
    Some(match input {
        "TRUE" => Value::Bool(true),
        "FALSE" => Value::Bool(false),
        _ if input.starts_with('#') && !input.contains(char::is_whitespace) => {
            Value::Error(input.to_string())
        }
        _ if crate::lexer::is_number(input) => Value::Number(input.to_string()),
        _ => Value::Str(input.to_string()),
    })
}

fn insert(line: usize, col: usize, text: impl Into<String>) -> Edit {
    Edit {
        span: Span::new(line, col, line, col),
        text: text.into(),
    }
}

/// The text edits that set the cell at `row`, `col` (from 1) to `value`,
/// or clear it for `None`. Applied together, to the text `doc` was parsed
/// from. Empty when nothing changes.
pub fn set_cell(doc: &Document, row: u32, col: u32, value: Option<&Value>) -> Vec<Edit> {
    assert!(row > 0 && col > 0, "rows and columns count from 1");
    let table = Table::new(doc);
    let at = cursors(doc);
    let mut edits = Vec::new();
    match (table.get(row, col), value) {
        (Some(cell), Some(value)) => {
            let record = &doc.records[cell.record];
            let text = literal(value);
            match record.field('K') {
                Some(k) if k.value == *value => {}
                Some(k) => edits.push(Edit {
                    span: k.value_span,
                    text,
                }),
                None => {
                    let end = record.full_span().end_col;
                    edits.push(insert(record.line, end, format!(";K{text}")));
                }
            }
        }
        (Some(cell), None) => {
            let record = &doc.records[cell.record];
            let only_place = record
                .fields
                .iter()
                .all(|f| matches!(f.name, 'X' | 'Y' | 'K'));
            if only_place {
                edits.push(Edit {
                    span: Span::new(record.line, 1, record.line + 1, 1),
                    text: String::new(),
                });
                let before = before(&at, cell.record);
                edits.extend(keep_next(doc, cell.record + 1, before, at[cell.record]));
            } else if let Some(k) = record.field('K') {
                // `;K...`: from the `;` before the letter.
                edits.push(Edit {
                    span: Span::new(
                        record.line,
                        k.span.start_col - 1,
                        record.line,
                        k.span.end_col,
                    ),
                    text: String::new(),
                });
            }
        }
        (None, Some(value)) => edits.extend(add_cell(doc, &at, row, col, value)),
        (None, None) => {}
    }
    if value.is_some() {
        edits.extend(grow_bounds(doc, &table, row, col));
    }
    edits
}

/// The cursor before record `i`.
fn before(at: &[(u32, u32)], i: usize) -> (u32, u32) {
    if i == 0 { (1, 1) } else { at[i - 1] }
}

/// A new `C` record for a cell no record sets.
fn add_cell(doc: &Document, at: &[(u32, u32)], row: u32, col: u32, value: &Value) -> Vec<Edit> {
    // Before the first cell after it in reading order, else before `E`,
    // else at the end.
    let next = doc
        .records
        .iter()
        .enumerate()
        .find(|(i, r)| r.kind == "C" && at[*i] > (row, col))
        .or_else(|| doc.records.iter().enumerate().find(|(_, r)| r.kind == "E"))
        .map(|(i, _)| i);
    let index = next.unwrap_or(doc.records.len());
    let cursor = before(at, index);
    let mut record = String::from("C");
    if cursor.0 != row {
        record += &format!(";Y{row}");
    }
    record += &format!(";X{col};K{}", literal(value));

    let mut edits = Vec::new();
    match next {
        Some(i) => {
            edits.push(insert(
                doc.records[i].line,
                1,
                format!("{record}{}", doc.newline),
            ));
            edits.extend(keep_next(doc, i, (row, col), cursor));
        }
        None => match doc.records.last() {
            Some(last) => {
                let end = last.full_span().end_col;
                edits.push(insert(last.line, end, format!("{}{record}", doc.newline)));
            }
            None => edits.push(insert(1, 1, record)),
        },
    }
    edits
}

/// After a change before record `from`, the cursor there is `now` instead
/// of `was`: the next record that moves the cursor names the row and
/// column it relied on.
fn keep_next(doc: &Document, from: usize, now: (u32, u32), was: (u32, u32)) -> Vec<Edit> {
    let Some((_, record)) = doc
        .records
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, r)| moves_cursor(r))
    else {
        return Vec::new();
    };
    let mut add = String::new();
    if record.field('Y').is_none() && now.0 != was.0 {
        add += &format!(";Y{}", was.0);
    }
    if record.field('X').is_none() && now.1 != was.1 {
        add += &format!(";X{}", was.1);
    }
    if add.is_empty() {
        return Vec::new();
    }
    vec![insert(record.line, record.span.end_col, add)]
}

/// `B` grows to take in the cell.
fn grow_bounds(doc: &Document, table: &Table, row: u32, col: u32) -> Vec<Edit> {
    let (Some(b), Some((rows, cols))) = (doc.record("B"), table.bounds) else {
        return Vec::new();
    };
    let mut edits = Vec::new();
    let mut grow = |b: &Record, name: char, to: u32| {
        if let Some(f) = b.field(name) {
            edits.push(Edit {
                span: f.value_span,
                text: to.to_string(),
            });
        }
    };
    if row > rows {
        grow(b, 'Y', row);
    }
    if col > cols {
        grow(b, 'X', col);
    }
    edits
}

/// `src` with `edits` applied (spans as [`set_cell`] gives them: 1-based
/// lines and character columns, end exclusive, all on the original text).
pub fn apply(src: &str, edits: &[Edit]) -> String {
    // Each edit's span as character offsets into `src`.
    let mut line_starts = vec![0];
    let chars: Vec<char> = src.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            line_starts.push(i + 1);
        }
    }
    let offset = |line: usize, col: usize| {
        line_starts
            .get(line - 1)
            .map_or(chars.len(), |s| (s + col - 1).min(chars.len()))
    };
    let mut ranges: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|e| {
            (
                offset(e.span.start_line, e.span.start_col),
                offset(e.span.end_line, e.span.end_col),
                e.text.as_str(),
            )
        })
        .collect();
    ranges.sort_by_key(|r| (r.0, r.1));
    let mut out = String::new();
    let mut pos = 0;
    for (start, end, text) in ranges {
        out.extend(&chars[pos..start]);
        out += text;
        pos = end;
    }
    out.extend(&chars[pos..]);
    out
}

#[cfg(test)]
#[path = "edit.test.rs"]
mod tests;
