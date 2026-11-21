//! A parser, table model, editor and semantic highlighter for SYLK
//! (`.slk`), the spreadsheet format of Warcraft III's data tables
//! (`Units\UnitBalance.slk`, `Units\AbilityMetaData.slk`, ...). Structured
//! like the other language crates; everything not SYLK-specific comes from
//! `parser-rs` (see its README for the shared design).
//!
//! ```
//! let doc = slk_rs::parse("ID;PWXL;N;E\nC;Y1;X1;K\"name\"\nC;X2;K5\nE\n");
//! let table = slk_rs::Table::new(&doc);
//! assert_eq!(table.text(1, 2), "5");
//! ```

pub mod ast;
pub mod edit;
pub mod html;
pub mod lexer;
pub mod lint;
pub mod parser;
pub mod printer;
pub mod semantic;
pub mod table;
pub mod token;

pub use ast::{ColumnEncoding, Document, Field, Record, Span, Value};
pub use edit::{apply, parse_input, set_cell};
pub use html::render_html;
pub use lint::{Diagnostic, Level, lint};
pub use parser::Parser;
pub use parser_rs::ParseError;
pub use printer::{render_dot, render_tree};
pub use semantic::{encoded_semantic_tokens, semantic_tokens};
pub use table::{Cell, Table};

/// Parses SYLK source. This never fails as a whole: problems are listed in
/// [`Document::errors`] and the rest still parses.
pub fn parse(src: &str) -> Document {
    Parser::parse_str(src)
}
