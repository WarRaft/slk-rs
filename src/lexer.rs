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

use std::borrow::Cow;

use crate::token::{Tok, Token};

/// What a piece of a line is, without its text: all the highlighter needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    RecordType,
    Semi,
    FieldName,
    Value(ValueKind),
    /// A line after the `E` record.
    AfterEnd,
    Newline,
}

/// What a field's value is, by its record, its letter and its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Number,
    /// `"..."`; `closed` is false without the closing quote.
    Str {
        closed: bool,
    },
    Bool,
    Error,
    Formula,
    Comment,
    Text,
}

/// Walks `src` piece by piece, without copying anything: `f` gets each
/// piece's kind, line, columns (1-based characters, end exclusive) and
/// its text as written (a value with its quotes and `;;`). [`tokenize`]
/// and the highlighter both read it.
pub fn scan(src: &str, mut f: impl FnMut(Piece, usize, usize, usize, &str)) {
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    let mut ended = false;
    let mut lines = src.split('\n').peekable();
    let mut line = 0;
    while let Some(raw) = lines.next() {
        line += 1;
        let text = raw.strip_suffix('\r').unwrap_or(raw);
        let ascii = text.is_ascii();
        // The column of byte offset `b`: the byte itself on an ASCII line.
        let col = |b: usize| {
            if ascii {
                b + 1
            } else {
                text[..b].chars().count() + 1
            }
        };
        if !text.is_empty() {
            if ended {
                f(Piece::AfterEnd, line, 1, col(text.len()), text);
            } else {
                ended = scan_line(text, line, &col, &mut f);
            }
        }
        // The line break, just past the line's last character; the last
        // line has none.
        let end = col(text.len());
        if lines.peek().is_some() {
            f(Piece::Newline, line, end, end + 1, "");
        } else {
            f(Piece::Newline, line, end, end, "");
        }
    }
}

/// The fields of a line: (start, end) byte ranges, end exclusive, the
/// first being the record type. A `;` ends a field unless another follows
/// it.
fn split(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let bytes = text.as_bytes();
    let (mut start, mut i) = (0, 0);
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        while i < bytes.len() {
            if bytes[i] == b';' {
                if bytes.get(i + 1) == Some(&b';') && i > start {
                    i += 2;
                    continue;
                }
                let field = (start, i);
                i += 1;
                start = i;
                return Some(field);
            }
            i += 1;
        }
        done = true;
        Some((start, bytes.len()))
    })
}

/// Scans one record; whether it is the `E` that ends the file.
fn scan_line(
    text: &str,
    line: usize,
    col: &impl Fn(usize) -> usize,
    f: &mut impl FnMut(Piece, usize, usize, usize, &str),
) -> bool {
    let mut fields = split(text);
    let (ts, te) = fields.next().expect("a line has a record type");
    let record = &text[ts..te];
    f(Piece::RecordType, line, col(ts), col(te), record);
    for (s, e) in fields {
        // The `;` before the field.
        f(Piece::Semi, line, col(s - 1), col(s), ";");
        if s == e {
            continue;
        }
        let letter_end = s + text[s..].chars().next().map_or(1, char::len_utf8);
        let letter = &text[s..letter_end];
        f(Piece::FieldName, line, col(s), col(letter_end), letter);
        if e > letter_end {
            let raw = &text[letter_end..e];
            let kind = value_kind(record, letter, raw);
            f(Piece::Value(kind), line, col(letter_end), col(e), raw);
        }
    }
    record == "E"
}

/// What the value `raw` of field `letter` in a `record` record is.
fn value_kind(record: &str, letter: &str, raw: &str) -> ValueKind {
    match (record, letter) {
        ("C", "K") => literal_kind(raw),
        ("C" | "NN", "E") => ValueKind::Formula,
        ("C", "A") => ValueKind::Comment,
        ("C" | "F" | "B", "X" | "Y") if is_number(raw) => ValueKind::Number,
        _ => ValueKind::Text,
    }
}

/// A cell value: `"text"`, a number, `TRUE` / `FALSE`, `#ERROR`.
fn literal_kind(raw: &str) -> ValueKind {
    match raw {
        _ if raw.starts_with('"') => ValueKind::Str {
            closed: raw.len() >= 2 && raw.ends_with('"'),
        },
        "TRUE" | "FALSE" => ValueKind::Bool,
        _ if raw.starts_with('#') => ValueKind::Error,
        _ if is_number(raw) => ValueKind::Number,
        _ => ValueKind::Text,
    }
}

/// Scans `src` into tokens: each line's tokens, then [`Tok::Newline`]
/// (none after the last line), then [`Tok::Eof`]. `\r\n` endings and a
/// leading byte-order mark are accepted. Lines after the `E` record are
/// one [`Tok::AfterEnd`] each.
pub fn tokenize(src: &str) -> Vec<Token<Tok>> {
    let mut out = Vec::new();
    let mut last = (1, 1);
    scan(src, |piece, line, start, end, text| {
        if piece == Piece::Newline && start == end {
            // Past the last line: where Eof goes.
            last = (line, start);
            return;
        }
        let kind = match piece {
            Piece::RecordType => Tok::RecordType(text.to_string()),
            Piece::Semi => Tok::Semi,
            Piece::FieldName => Tok::FieldName(text.chars().next().expect("a letter")),
            Piece::Value(kind) => value(kind, text),
            Piece::AfterEnd => Tok::AfterEnd(text.to_string()),
            Piece::Newline => Tok::Newline,
        };
        out.push(Token::new(kind, line, start, end));
    });
    out.push(Token::new(Tok::Eof, last.0, last.1, last.1));
    out
}

/// A value's token: its text with `;;` read as `;`, and escapes decoded in
/// strings and comments.
pub(crate) fn value(kind: ValueKind, raw: &str) -> Tok {
    let text = unescape(raw);
    match kind {
        ValueKind::Str { closed } => {
            let inner = &text[1..text.len() - usize::from(closed)];
            Tok::Str {
                value: decode(inner),
                closed,
            }
        }
        ValueKind::Number => Tok::Number(text.into_owned()),
        ValueKind::Bool => Tok::Bool(text == "TRUE"),
        ValueKind::Error => Tok::ErrorValue(text.into_owned()),
        ValueKind::Formula => Tok::Formula(text.into_owned()),
        ValueKind::Comment => Tok::Comment(decode(&text)),
        ValueKind::Text => Tok::Text(text.into_owned()),
    }
}

/// `;;` as `;`, copying only when there is one.
fn unescape(raw: &str) -> Cow<'_, str> {
    if raw.contains(";;") {
        Cow::Owned(raw.replace(";;", ";"))
    } else {
        Cow::Borrowed(raw)
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
    if !text.contains('\x1B') {
        return text.to_string();
    }
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
