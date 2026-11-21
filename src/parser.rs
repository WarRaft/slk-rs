//! Reads the scanner's pieces into a [`Document`], line by line. A line
//! with a problem is read as far as it goes and the problem recorded; the
//! rest of the file still parses. What's checked:
//!
//! - the file starts with an `ID` record and ends with an `E` record, with
//!   nothing after it;
//! - a record has a type;
//! - `X` and `Y` of `C`, `F` and `B` are whole numbers, and of `C` and `F`
//!   at least 1;
//! - a cell value (`C;K`) is a number, a closed `"string"`, `TRUE`,
//!   `FALSE` or an error like `#VALUE!`.
//!
//! Records and fields of other kinds are kept but not checked: readers
//! skip what they don't know.

use crate::ast::*;
use crate::lexer::{Piece, ValueKind, scan};
use crate::token::Tok;

pub struct Parser;

impl Parser {
    /// Reads `src` piece by piece from the scanner ([`scan`]): no token
    /// list in between, and only the values the document keeps are copied.
    pub fn parse_str(src: &str) -> Document {
        let mut p = Builder {
            doc: Document {
                newline: if src.contains("\r\n") { "\r\n" } else { "\n" },
                ..Document::default()
            },
            record: None,
            field: None,
            ended: None,
            last_line: 1,
        };
        scan(src, |piece, line, start, end, text| {
            p.piece(piece, Span::new(line, start, line, end), text)
        });
        p.end_record();
        let mut doc = p.doc;
        if p.ended.is_none() {
            let line = doc.records.last().map_or(p.last_line, |r| r.line);
            doc.errors.push(ParseError::at(
                "missing the E record that ends the file",
                line,
                1,
            ));
        }
        doc
    }
}

/// The document so far, and the record and field being read.
struct Builder {
    doc: Document,
    record: Option<Record>,
    /// The letter of a field whose value may follow, and its span.
    field: Option<(char, Span)>,
    /// The line of the `E` record.
    ended: Option<usize>,
    last_line: usize,
}

impl Builder {
    fn piece(&mut self, piece: Piece, at: Span, text: &str) {
        match piece {
            Piece::RecordType => {
                if text.is_empty() {
                    self.doc
                        .errors
                        .push(error("missing the record type before ';'", at));
                }
                self.record = Some(Record {
                    kind: text.to_string(),
                    line: at.start_line,
                    span: at,
                    fields: Vec::new(),
                });
            }
            Piece::Semi => self.end_field(),
            Piece::FieldName => {
                self.end_field();
                self.field = text.chars().next().map(|c| (c, at));
            }
            Piece::Value(kind) => {
                if let Some((name, name_span)) = self.field.take() {
                    let unclosed = kind == (ValueKind::Str { closed: false });
                    let value = value(crate::lexer::value(kind, text));
                    self.push_field(name, name_span, value, at, unclosed);
                }
            }
            Piece::AfterEnd => {
                let line = self.ended.unwrap_or(0);
                self.doc.errors.push(error(
                    format!("text after the E record (line {line}), which ends the file"),
                    at,
                ));
            }
            Piece::Newline => {
                self.last_line = at.start_line;
                self.end_record();
            }
        }
    }

    /// A field without a value: its letter alone.
    fn end_field(&mut self) {
        if let Some((name, name_span)) = self.field.take() {
            let at = Span::new(
                name_span.start_line,
                name_span.end_col,
                name_span.start_line,
                name_span.end_col,
            );
            self.push_field(name, name_span, Value::Empty, at, false);
        }
    }

    fn push_field(
        &mut self,
        name: char,
        name_span: Span,
        value: Value,
        value_span: Span,
        unclosed: bool,
    ) {
        let Some(record) = self.record.as_mut() else {
            return;
        };
        let has_value = value != Value::Empty;
        record.fields.push(Field {
            name,
            value,
            span: Span::new(
                name_span.start_line,
                name_span.start_col,
                name_span.start_line,
                value_span.end_col,
            ),
            value_span,
        });
        check(
            record,
            record.fields.last().expect("just pushed"),
            has_value,
            unclosed,
            &mut self.doc.errors,
        );
    }

    fn end_record(&mut self) {
        self.end_field();
        let Some(record) = self.record.take() else {
            return;
        };
        if self.doc.records.is_empty() && record.kind != "ID" {
            self.doc.errors.push(error(
                "a SYLK file starts with an ID record, like ID;PWXL;N;E",
                record.span,
            ));
        }
        if record.kind == "E" {
            self.ended = Some(record.line);
        }
        self.doc.records.push(record);
    }
}

fn error(message: impl Into<String>, span: Span) -> ParseError {
    ParseError {
        message: message.into(),
        line: span.start_line,
        col: span.start_col,
        end_col: span.end_col.max(span.start_col + 1),
    }
}

fn value(tok: Tok) -> Value {
    match tok {
        Tok::Number(n) => Value::Number(n),
        Tok::Str { value, .. } => Value::Str(value),
        Tok::Bool(b) => Value::Bool(b),
        Tok::ErrorValue(e) => Value::Error(e),
        Tok::Formula(f) => Value::Formula(f),
        Tok::Comment(c) => Value::Comment(c),
        Tok::Text(t) => Value::Text(t),
        _ => Value::Empty,
    }
}

/// The checks of the module docs, for one field.
fn check(
    record: &Record,
    field: &Field,
    has_value: bool,
    unclosed: bool,
    errors: &mut Vec<ParseError>,
) {
    let kind = record.kind.as_str();
    let at = field.value_span;
    match (kind, field.name) {
        ("C" | "F" | "B", 'X' | 'Y') => {
            let what = if field.name == 'X' { "column" } else { "row" };
            match &field.value {
                Value::Number(n) => {
                    let whole = n.parse::<u32>().ok();
                    if whole.is_none() || (kind != "B" && whole == Some(0)) {
                        errors.push(error(format!("expected the {what} number, 1 or more: {n}"), at));
                    }
                }
                _ => errors.push(error(format!("expected the {what} number after {}", field.name), field.span)),
            }
        }
        ("C", 'K') => match &field.value {
            _ if !has_value => errors.push(error("expected the cell value after K", field.span)),
            Value::Str(_) if unclosed => errors.push(error("string isn't closed: '\"' missing", at)),
            Value::Text(t) => errors.push(error(
                format!("expected a cell value: a number, a \"string\", TRUE, FALSE or an error like #VALUE!, found '{t}'"),
                at,
            )),
            _ => {}
        },
        _ => {}
    }
}

#[cfg(test)]
#[path = "parser.test.rs"]
mod tests;
