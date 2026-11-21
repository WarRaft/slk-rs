//! Semantic highlighting for SYLK, straight from the scanner's tokens (the
//! same tokens the parser reads), so it works on any file, broken or not:
//!
//! - record types (`ID`, `C`, `F`, `E`): `keyword`; `;`: `operator`;
//! - field letters: `property`;
//! - numbers: `number`; strings and other text: `string`;
//! - `TRUE` / `FALSE`: `keyword`; error values (`#VALUE!`): `enumMember`;
//! - formulas: `function`; cell comments (`C;A`): `comment`.

use crate::lexer::{Piece, ValueKind, scan};
use parser_rs::semantic;
use parser_rs::{Modifiers, SemanticToken, SemanticType as T};

/// Every semantic token in `src`, unencoded. Read with [`scan`], which
/// copies nothing: the scanner's pieces, not its tokens.
pub fn semantic_tokens(src: &str) -> Vec<SemanticToken> {
    let mut out = Vec::new();
    scan(src, |piece, line, start, end, _| {
        let ty = match piece {
            Piece::RecordType | Piece::Value(ValueKind::Bool) => T::Keyword,
            Piece::Semi => T::Operator,
            Piece::FieldName => T::Property,
            Piece::Value(ValueKind::Number) => T::Number,
            Piece::Value(ValueKind::Str { .. } | ValueKind::Text) => T::String,
            Piece::Value(ValueKind::Error) => T::EnumMember,
            Piece::Value(ValueKind::Formula) => T::Function,
            Piece::Value(ValueKind::Comment) => T::Comment,
            Piece::AfterEnd | Piece::Newline => return,
        };
        out.push(SemanticToken::new(line, start, end, ty, Modifiers::NONE));
    });
    out
}

/// [`semantic_tokens`] in the editor's wire format.
pub fn encoded_semantic_tokens(src: &str) -> Vec<u32> {
    semantic::encode(semantic_tokens(src), src)
}

#[cfg(test)]
#[path = "semantic.test.rs"]
mod tests;
