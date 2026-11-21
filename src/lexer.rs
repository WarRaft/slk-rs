//! The SYLK line scanner. A line is a record, `TYPE;Fvalue;Fvalue...`, and
//! what a value is depends on its record and letter, so SYLK has its own
//! scanner instead of a `LexSpec`. It is still the **only** lexer: the
//! parser, the semantic highlighter and the HTML viewer all read its tokens.
//!
//! - `;` separates fields; `;;` is a `;` inside a value.
//! - A field's first character is its letter; the rest is the value.
//! - `C;K` holds a cell value: a `"string"`, a number, `TRUE` / `FALSE` or
//!   an error such as `#VALUE!`. `C;E` and `NN;E` hold formulas, `C;A` a
//!   comment, `X` / `Y` a column / row.
//! - Inside strings and comments, `ESC` and two characters stand for one
//!   byte (`ESC :` is a line break): see [`decode`].
//!
//! The scanner never fails: a value that doesn't fit its field becomes a
//! [`Tok::Text`] and the parser reports it.

use crate::token::{Tok, Token};

/// Scans `src` into tokens: each line's tokens, then [`Tok::Newline`]
/// (none after the last line), then [`Tok::Eof`]. `\r\n` endings and a
/// leading byte-order mark are accepted. Lines after the `E` record are
/// one [`Tok::AfterEnd`] each.
pub fn tokenize(src: &str) -> Vec<Token<Tok>> {
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    let mut out = Vec::new();
    let (mut line, mut end) = (1, 1);
    let mut ended = false;
    for (i, raw) in src.split('\n').enumerate() {
        if i > 0 {
            out.push(Token::new(Tok::Newline, line, end, end + 1));
            line += 1;
        }
        let chars: Vec<char> = raw.strip_suffix('\r').unwrap_or(raw).chars().collect();
        end = chars.len() + 1;
        if chars.is_empty() {
            continue;
        }
        if ended {
            out.push(Token::new(
                Tok::AfterEnd(chars.iter().collect()),
                line,
                1,
                end,
            ));
            continue;
        }
        ended = scan_line(&chars, line, &mut out);
    }
    out.push(Token::new(Tok::Eof, line, end, end));
    out
}

/// The fields of a line: (start, end) char ranges, end exclusive, the first
/// being the record type. A `;` ends a field unless another follows it.
pub(crate) fn split(chars: &[char]) -> Vec<(usize, usize)> {
    let mut fields = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < chars.len() {
        if chars[i] == ';' {
            if chars.get(i + 1) == Some(&';') && i > start {
                i += 2;
                continue;
            }
            fields.push((start, i));
            start = i + 1;
        }
        i += 1;
    }
    fields.push((start, chars.len()));
    fields
}

/// Scans one record; whether it is the `E` that ends the file.
fn scan_line(chars: &[char], line: usize, out: &mut Vec<Token<Tok>>) -> bool {
    let fields = split(chars);
    let (ts, te) = fields[0];
    let record: String = chars[ts..te].iter().collect();
    out.push(Token::new(
        Tok::RecordType(record.clone()),
        line,
        ts + 1,
        te + 1,
    ));
    for &(s, e) in &fields[1..] {
        out.push(Token::new(Tok::Semi, line, s, s + 1));
        if s == e {
            continue;
        }
        let letter = chars[s];
        out.push(Token::new(Tok::FieldName(letter), line, s + 1, s + 2));
        if e > s + 1 {
            let raw: String = chars[s + 1..e].iter().collect();
            out.push(Token::new(value(&record, letter, &raw), line, s + 2, e + 1));
        }
    }
    record == "E"
}

/// What the value `raw` of field `letter` in a `record` record is.
fn value(record: &str, letter: char, raw: &str) -> Tok {
    let text = raw.replace(";;", ";");
    match (record, letter) {
        ("C", 'K') => literal(&text),
        ("C" | "NN", 'E') => Tok::Formula(text),
        ("C", 'A') => Tok::Comment(decode(&text)),
        ("C" | "F" | "B", 'X' | 'Y') if is_number(&text) => Tok::Number(text),
        _ => Tok::Text(text),
    }
}

/// A cell value: `"text"`, a number, `TRUE` / `FALSE`, `#ERROR`.
fn literal(text: &str) -> Tok {
    if let Some(rest) = text.strip_prefix('"') {
        let (value, closed) = match rest.strip_suffix('"') {
            Some(inner) => (inner, true),
            None => (rest, false),
        };
        return Tok::Str {
            value: decode(value),
            closed,
        };
    }
    match text {
        "TRUE" => Tok::Bool(true),
        "FALSE" => Tok::Bool(false),
        _ if text.starts_with('#') => Tok::ErrorValue(text.to_string()),
        _ if is_number(text) => Tok::Number(text.to_string()),
        _ => Tok::Text(text.to_string()),
    }
}

/// `12`, `-0.5`, `.5`, `1E+3`: what SYLK writes numbers as.
pub fn is_number(text: &str) -> bool {
    let t = text.strip_prefix(['-', '+']).unwrap_or(text);
    let (mantissa, exponent) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let mantissa_ok = !(int.is_empty() && frac.is_empty()) && digits(int) && digits(frac);
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['-', '+']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    mantissa_ok && exponent_ok
}

/// Decodes SYLK's escapes in text: `ESC` and two characters
/// `[\x20-\x2F][\x30-\x3F]` are one byte, high bits from the first
/// (`- 0x20`), low bits from the second (`- 0x30`): `ESC :` is `\n`. Other
/// `ESC` sequences are kept as written.
pub fn decode(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1B'
            && let (Some(&hi), Some(&lo)) = (chars.get(i + 1), chars.get(i + 2))
            && ('\x20'..='\x2F').contains(&hi)
            && ('\x30'..='\x3F').contains(&lo)
        {
            let byte = ((hi as u32 - 0x20) << 4) | (lo as u32 - 0x30);
            out.push(char::from_u32(byte).unwrap_or('\u{FFFD}'));
            i += 3;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Encodes text for a SYLK string or comment: `;` as `;;`, and control
/// characters (a line break, a tab) as `ESC` sequences. The inverse of
/// [`decode`] after `;;` is read as `;`.
pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            ';' => out += ";;",
            c if (c as u32) < 0x20 => {
                let b = c as u32;
                out.push('\x1B');
                out.push(char::from_u32(0x20 + (b >> 4)).expect("ASCII"));
                out.push(char::from_u32(0x30 + (b & 0xF)).expect("ASCII"));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[path = "lexer.test.rs"]
mod tests;
