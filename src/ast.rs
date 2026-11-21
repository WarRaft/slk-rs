//! The SYLK document: its records, in order. Records are stateful (`X` and
//! `Y` carry over), so the cells come from [`crate::Table`], which reads
//! them in order.

pub use parser_rs::{ColumnEncoding, ParseError, Span};

/// A parsed file. Parsing never fails as a whole: a line with a problem is
/// still read as far as it goes, and the problem recorded in `errors`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub records: Vec<Record>,
    /// In source order.
    pub errors: Vec<ParseError>,
    /// The line break the file uses: `\r\n` or `\n`.
    pub newline: &'static str,
}

impl Document {
    /// The first record of type `kind`.
    pub fn record(&self, kind: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.kind == kind)
    }
}

/// One line: `C;X2;K"text"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The record type as written: `ID`, `B`, `C`, `F`, `P`, `O`, `E`,
    /// `NN`, ..., or anything else (which readers ignore).
    pub kind: String,
    pub line: usize,
    /// The record type.
    pub span: Span,
    pub fields: Vec<Field>,
}

impl Record {
    /// The first field with letter `name`.
    pub fn field(&self, name: char) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The whole record, from its type to its last field.
    pub fn full_span(&self) -> Span {
        let end = self.fields.last().map_or(self.span, |f| f.span);
        Span::new(self.line, self.span.start_col, self.line, end.end_col)
    }

    /// A row or column field (`Y`, `X`) as a number, when it is one.
    pub fn coordinate(&self, name: char) -> Option<u32> {
        match &self.field(name)?.value {
            Value::Number(n) => n.parse().ok().filter(|&v| v > 0),
            _ => None,
        }
    }
}

/// `K"text"`: a letter and a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: char,
    pub value: Value,
    /// The letter and the value.
    pub span: Span,
    /// The value alone; empty (at the letter's end) for a field without
    /// one, such as `C;S`.
    pub value_span: Span,
}

/// A field's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// No value: a flag field (`ID;N`, `C;S`).
    Empty,
    /// As written: `12`, `-0.5`, `1E+3`.
    Number(String),
    /// The text, with `;;` and `ESC` sequences decoded.
    Str(String),
    Bool(bool),
    /// `#VALUE!`, `#N/A`, ...
    Error(String),
    /// A formula, as written: `CONCATENATE(RC[-1],".mp3")`.
    Formula(String),
    /// A cell's comment (`C;A`), decoded.
    Comment(String),
    /// Anything else, with `;;` as `;`.
    Text(String),
}

impl Value {
    /// The text a table shows for the value.
    pub fn display(&self) -> String {
        match self {
            Value::Empty => String::new(),
            Value::Number(s)
            | Value::Str(s)
            | Value::Error(s)
            | Value::Formula(s)
            | Value::Comment(s)
            | Value::Text(s) => s.clone(),
            Value::Bool(true) => "TRUE".into(),
            Value::Bool(false) => "FALSE".into(),
        }
    }
}
