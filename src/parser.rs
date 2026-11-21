//! Reads the scanner's tokens into a [`Document`], line by line. A line
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
use crate::lexer::tokenize;
use crate::token::{Tok, Token};

pub struct Parser;

impl Parser {
    pub fn parse_str(src: &str) -> Document {
        let tokens = tokenize(src);
        let mut doc = Document {
            newline: if src.contains("\r\n") { "\r\n" } else { "\n" },
            ..Document::default()
        };
        let mut ended = None;
        let mut last_line = 1;
        for line in tokens.split(|t| t.kind == Tok::Newline) {
            let Some(first) = line.first() else { continue };
            last_line = first.line;
            match &first.kind {
                Tok::RecordType(_) => {
                    let record = record(line, &mut doc.errors);
                    if doc.records.is_empty() && record.kind != "ID" {
                        doc.errors.push(error(
                            "a SYLK file starts with an ID record, like ID;PWXL;N;E",
                            record.span,
                        ));
                    }
                    if record.kind == "E" {
                        ended = Some(record.line);
                    }
                    doc.records.push(record);
                }
                Tok::AfterEnd(_) => {
                    let line_no = ended.unwrap_or(0);
                    doc.errors.push(error(
                        format!("text after the E record (line {line_no}), which ends the file"),
                        Span::new(first.line, first.col, first.line, first.end_col),
                    ));
                }
                _ => {}
            }
        }
        if ended.is_none() {
            let line = doc.records.last().map_or(last_line, |r| r.line);
            doc.errors.push(ParseError::at(
                "missing the E record that ends the file",
                line,
                1,
            ));
        }
        doc
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

fn span(t: &Token<Tok>) -> Span {
    Span::new(t.line, t.col, t.end_line, t.end_col)
}

/// One line's tokens, from its record type: the record.
fn record(tokens: &[Token<Tok>], errors: &mut Vec<ParseError>) -> Record {
    let head = &tokens[0];
    let Tok::RecordType(kind) = &head.kind else {
        unreachable!("a line starts with its record type")
    };
    let mut record = Record {
        kind: kind.clone(),
        line: head.line,
        span: span(head),
        fields: Vec::new(),
    };
    if kind.is_empty() {
        errors.push(error("missing the record type before ';'", span(head)));
    }
    let mut i = 1;
    while i < tokens.len() {
        // `;`, then the letter and its value, if any.
        let Some(name_tok) = tokens
            .get(i + 1)
            .filter(|t| matches!(t.kind, Tok::FieldName(_)))
        else {
            i += 1;
            continue;
        };
        let Tok::FieldName(name) = name_tok.kind else {
            unreachable!()
        };
        let value_tok = tokens
            .get(i + 2)
            .filter(|t| !matches!(t.kind, Tok::Semi | Tok::FieldName(_) | Tok::Eof));
        let value_span = value_tok.map_or(
            Span::new(
                name_tok.line,
                name_tok.end_col,
                name_tok.line,
                name_tok.end_col,
            ),
            span,
        );
        let value = value_tok.map_or(Value::Empty, |t| value(&t.kind));
        record.fields.push(Field {
            name,
            value,
            span: Span::new(
                name_tok.line,
                name_tok.col,
                name_tok.line,
                value_span.end_col,
            ),
            value_span,
        });
        check(
            &record,
            record.fields.last().expect("just pushed"),
            value_tok,
            errors,
        );
        i += if value_tok.is_some() { 3 } else { 2 };
    }
    record
}

fn value(tok: &Tok) -> Value {
    match tok {
        Tok::Number(n) => Value::Number(n.clone()),
        Tok::Str { value, .. } => Value::Str(value.clone()),
        Tok::Bool(b) => Value::Bool(*b),
        Tok::ErrorValue(e) => Value::Error(e.clone()),
        Tok::Formula(f) => Value::Formula(f.clone()),
        Tok::Comment(c) => Value::Comment(c.clone()),
        Tok::Text(t) => Value::Text(t.clone()),
        _ => Value::Empty,
    }
}

/// The checks of the module docs, for one field.
fn check(record: &Record, field: &Field, tok: Option<&Token<Tok>>, errors: &mut Vec<ParseError>) {
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
        ("C", 'K') => match tok.map(|t| &t.kind) {
            Some(Tok::Str { closed: false, .. }) => errors.push(error("string isn't closed: '\"' missing", at)),
            Some(Tok::Text(t)) => errors.push(error(
                format!("expected a cell value: a number, a \"string\", TRUE, FALSE or an error like #VALUE!, found '{t}'"),
                at,
            )),
            None => errors.push(error("expected the cell value after K", field.span)),
            _ => {}
        },
        _ => {}
    }
}

#[cfg(test)]
#[path = "parser.test.rs"]
mod tests;
