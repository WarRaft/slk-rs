//! Semantic highlighting for SYLK, straight from the scanner's tokens (the
//! same tokens the parser reads), so it works on any file, broken or not:
//!
//! - record types (`ID`, `C`, `F`, `E`): `keyword`; `;`: `operator`;
//! - field letters: `property`;
//! - numbers: `number`; strings and other text: `string`;
//! - `TRUE` / `FALSE`: `keyword`; error values (`#VALUE!`): `enumMember`;
//! - formulas: `function`; cell comments (`C;A`): `comment`.

use crate::lexer::tokenize;
use crate::token::Tok;
use parser_rs::semantic;
use parser_rs::{Modifiers, SemanticToken, SemanticType as T};

/// Every semantic token in `src`, unencoded.
pub fn semantic_tokens(src: &str) -> Vec<SemanticToken> {
    tokenize(src)
        .iter()
        .filter_map(|t| {
            let ty = match &t.kind {
                Tok::RecordType(_) | Tok::Bool(_) => T::Keyword,
                Tok::Semi => T::Operator,
                Tok::FieldName(_) => T::Property,
                Tok::Number(_) => T::Number,
                Tok::Str { .. } | Tok::Text(_) => T::String,
                Tok::ErrorValue(_) => T::EnumMember,
                Tok::Formula(_) => T::Function,
                Tok::Comment(_) => T::Comment,
                _ => return None,
            };
            Some(SemanticToken::at(t, ty, Modifiers::NONE))
        })
        .collect()
}

/// [`semantic_tokens`] in the editor's wire format.
pub fn encoded_semantic_tokens(src: &str) -> Vec<u32> {
    semantic::encode(semantic_tokens(src), src)
}

#[cfg(test)]
#[path = "semantic.test.rs"]
mod tests;
