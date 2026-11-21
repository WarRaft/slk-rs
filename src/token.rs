//! SYLK's token kinds. A line is a record: its type, then `;`-separated
//! fields, each a letter and a value. What a value is depends on the record
//! and the letter (`C;K` is a cell value, `C;E` a formula, `X` a column),
//! so the tokens are line roles, not the shared `TokenKind` vocabulary.
//! They still flow through the shared [`Token`] plumbing.

use std::fmt;

pub use parser_rs::{ParseError, Token, TokenKindExt};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// The record type at the start of a line: `ID`, `C`, `F`, `E`, ...
    RecordType(String),
    /// `;` between fields.
    Semi,
    /// A field's letter: the `K` of `K"text"`.
    FieldName(char),
    /// A number: `12`, `-0.5`, `1E+3`, as written.
    Number(String),
    /// `"..."`, with `;;` already one `;`. `closed` is false when the
    /// field ends before the closing quote.
    Str {
        value: String,
        closed: bool,
    },
    /// `TRUE` or `FALSE`.
    Bool(bool),
    /// An error value: `#VALUE!`, `#N/A`, ...
    ErrorValue(String),
    /// A formula: `C;E...`, `NN;E...` (`CONCATENATE(RC[-1],".mp3")`).
    Formula(String),
    /// A cell's comment: `C;A...`.
    Comment(String),
    /// Any other value: a format, a style, a program name, or something a
    /// field shouldn't hold (the parser says so).
    Text(String),
    /// Anything after the `E` record, which ends the file.
    AfterEnd(String),
    Newline,
    Eof,
}

impl TokenKindExt for Tok {
    fn is_eof(&self) -> bool {
        matches!(self, Tok::Eof)
    }
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::RecordType(t) => write!(f, "record type '{t}'"),
            Tok::Semi => write!(f, "';'"),
            Tok::FieldName(c) => write!(f, "field '{c}'"),
            Tok::Number(n) => write!(f, "number {n}"),
            Tok::Str { value, .. } => write!(f, "\"{value}\""),
            Tok::Bool(true) => write!(f, "TRUE"),
            Tok::Bool(false) => write!(f, "FALSE"),
            Tok::ErrorValue(e) => write!(f, "{e}"),
            Tok::Formula(e) => write!(f, "formula {e}"),
            Tok::Comment(_) => write!(f, "comment"),
            Tok::Text(t) => write!(f, "'{t}'"),
            Tok::AfterEnd(t) => write!(f, "'{t}'"),
            Tok::Newline => write!(f, "end of line"),
            Tok::Eof => write!(f, "end of file"),
        }
    }
}
