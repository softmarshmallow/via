//! The lexical grammar (`formulas.md` 2).
//!
//! Spans are byte offsets in the formula's string value, its leading `=` included, so a span
//! goes to the document reader's span map unchanged.

use std::ops::Range;

use via_diag::codes::formula;

use crate::error::{At, ExprError, FormulaId};

/// The most bytes a formula's text may hold, after its `=` (`formula.too_long`).
pub const MAX_LEN: usize = 4096;

/// An operator of the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Plus,
    Minus,
    Star,
    Slash,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Ne,
    LParen,
    RParen,
    Comma,
    Dot,
    /// `=`, legal only after a parameter name in a call.
    Assign,
}

/// A token.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    Ident(String),
    Sel(String),
    Op(Op),
    And,
    Or,
    Not,
}

/// A token and its bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Range<u32>,
}

fn at(formula: FormulaId, start: usize, end: usize) -> At {
    At::Expr {
        formula,
        span: start as u32..end as u32,
    }
}

fn name_of(c: char) -> String {
    if c.is_ascii_graphic() {
        format!("`{c}`")
    } else {
        format!("U+{:04X}", c as u32)
    }
}

/// Splits the text after `=` into tokens. `value` is the whole string value; `start` the byte
/// after its `=`.
pub fn lex(formula: FormulaId, value: &str, start: usize) -> Result<Vec<Token>, ExprError> {
    let b = value.as_bytes();
    let mut out = Vec::new();
    let mut i = start;
    let err = |code, s: usize, e: usize, m: String, h: &str| {
        Err(ExprError::new(code, at(formula, s, e), m, h))
    };
    while i < b.len() {
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'0'..=b'9' => {
                let (tok, end) = number(formula, value, i)?;
                out.push(Token {
                    tok,
                    span: i as u32..end as u32,
                });
                i = end;
            }
            b'.' if b.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                let mut e = i + 1;
                while e < b.len() && b[e].is_ascii_digit() {
                    e += 1;
                }
                let lit = &value[i..e];
                return err(
                    formula::SYNTAX,
                    i,
                    e,
                    format!("`{lit}` is not a number: a number starts with a digit"),
                    &format!("write `0{lit}`"),
                );
            }
            b'a'..=b'z' => {
                let mut e = i + 1;
                while e < b.len()
                    && (b[e].is_ascii_lowercase() || b[e].is_ascii_digit() || b[e] == b'_')
                {
                    e += 1;
                }
                if e < b.len() && b[e].is_ascii_uppercase() {
                    return err(
                        formula::CHARACTER,
                        e,
                        e + 1,
                        format!(
                            "{} is not a character of the language",
                            name_of(b[e] as char)
                        ),
                        "names are lower case",
                    );
                }
                let word = &value[i..e];
                let tok = match word {
                    "and" => Tok::And,
                    "or" => Tok::Or,
                    "not" => Tok::Not,
                    _ => Tok::Ident(word.to_string()),
                };
                out.push(Token {
                    tok,
                    span: i as u32..e as u32,
                });
                i = e;
            }
            b'\'' => {
                let mut e = i + 1;
                loop {
                    let Some(ch) = value[e..].chars().next() else {
                        return err(
                            formula::SYNTAX,
                            i,
                            b.len(),
                            "a selector is not closed".to_string(),
                            "end the selector with `'`",
                        );
                    };
                    if ch == '\'' {
                        break;
                    }
                    if ch.is_control() {
                        return err(
                            formula::CHARACTER,
                            e,
                            e + ch.len_utf8(),
                            format!("{} is a control character inside a selector", name_of(ch)),
                            "a selector holds no control characters; close it with `'`",
                        );
                    }
                    e += ch.len_utf8();
                }
                out.push(Token {
                    tok: Tok::Sel(value[i + 1..e].to_string()),
                    span: i as u32..(e + 1) as u32,
                });
                i = e + 1;
            }
            _ => {
                let two = b.get(i + 1).copied();
                let (op, len) = match (c, two) {
                    (b'<', Some(b'=')) => (Some(Op::Le), 2),
                    (b'>', Some(b'=')) => (Some(Op::Ge), 2),
                    (b'=', Some(b'=')) => (Some(Op::EqEq), 2),
                    (b'!', Some(b'=')) => (Some(Op::Ne), 2),
                    (b'*', Some(b'*')) => (None, 2),
                    (b'&', Some(b'&')) | (b'|', Some(b'|')) => (None, 2),
                    (b'+', _) => (Some(Op::Plus), 1),
                    (b'-', _) => (Some(Op::Minus), 1),
                    (b'*', _) => (Some(Op::Star), 1),
                    (b'/', _) => (Some(Op::Slash), 1),
                    (b'<', _) => (Some(Op::Lt), 1),
                    (b'>', _) => (Some(Op::Gt), 1),
                    (b'(', _) => (Some(Op::LParen), 1),
                    (b')', _) => (Some(Op::RParen), 1),
                    (b',', _) => (Some(Op::Comma), 1),
                    (b'.', _) => (Some(Op::Dot), 1),
                    (b'=', _) => (Some(Op::Assign), 1),
                    _ => (None, 1),
                };
                if let Some(op) = op {
                    out.push(Token {
                        tok: Tok::Op(op),
                        span: i as u32..(i + len) as u32,
                    });
                    i += len;
                    continue;
                }
                let foreign = match c {
                    b'&' => Some("write `and`"),
                    b'|' => Some("write `or`"),
                    b'~' | b'!' => Some("write `not`"),
                    b'*' | b'^' => Some("write `pow(a, b)`"),
                    b'%' => Some("write `a - b * floor(a / b)`"),
                    b'"' => Some("selectors take single quotes"),
                    b'?' | b':' => Some("write `if(c, a, b)`"),
                    _ => None,
                };
                if let Some(hint) = foreign {
                    let text = &value[i..i + len];
                    return err(
                        formula::OPERATOR,
                        i,
                        i + len,
                        format!("`{text}` is not an operator of the language"),
                        hint,
                    );
                }
                let ch = value[i..].chars().next().unwrap_or('\u{FFFD}');
                let hint = if ch.is_ascii_uppercase() {
                    "names are lower case"
                } else {
                    "remove it; the language has letters a-z, digits, `_`, quotes and + - * / < > = ! ( ) , ."
                };
                return err(
                    formula::CHARACTER,
                    i,
                    i + ch.len_utf8(),
                    format!("{} is not a character of the language", name_of(ch)),
                    hint,
                );
            }
        }
    }
    Ok(out)
}

/// A number literal at `i`: `digits [ "." digits ] [ ("e" / "E") ["+" / "-"] digits ]`, not
/// followed at once by a letter, digit, `.` or `_`.
fn number(formula: FormulaId, value: &str, i: usize) -> Result<(Tok, usize), ExprError> {
    let b = value.as_bytes();
    let digits = |mut e: usize| {
        while e < b.len() && b[e].is_ascii_digit() {
            e += 1;
        }
        e
    };
    let tail = |e: usize| {
        let mut t = e;
        while t < b.len() && (b[t].is_ascii_alphanumeric() || b[t] == b'.' || b[t] == b'_') {
            t += 1;
        }
        t
    };
    let bad = |e: usize| {
        let t = tail(e).max(e);
        let lit = &value[i..t];
        Err(ExprError::new(
            formula::SYNTAX,
            at(formula, i, t),
            format!("`{lit}` is not a number"),
            "write the number in full, `1e3`, `5.0`",
        ))
    };
    let mut e = digits(i);
    if e < b.len() && b[e] == b'.' {
        if !b.get(e + 1).is_some_and(u8::is_ascii_digit) {
            return bad(e + 1);
        }
        e = digits(e + 1);
    }
    if e < b.len() && (b[e] == b'e' || b[e] == b'E') {
        let mut k = e + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        if !b.get(k).is_some_and(u8::is_ascii_digit) {
            return bad(k);
        }
        e = digits(k);
    }
    if e < b.len() && (b[e].is_ascii_alphanumeric() || b[e] == b'.' || b[e] == b'_') {
        return bad(e);
    }
    let lit = &value[i..e];
    let mantissa_end = lit.find(['e', 'E']).unwrap_or(lit.len());
    let nonzero = lit[..mantissa_end]
        .bytes()
        .any(|d| (b'1'..=b'9').contains(&d));
    match via_num::parse_double(lit) {
        Ok(v) if v == 0.0 && nonzero => Err(ExprError::new(
            formula::SYNTAX,
            at(formula, i, e),
            format!("`{lit}` rounds to 0 as a double"),
            "the smallest magnitude a double holds is about 4.9e-324",
        )),
        Ok(v) => Ok((Tok::Num(v), e)),
        Err(_) => Err(ExprError::new(
            formula::SYNTAX,
            at(formula, i, e),
            format!("`{lit}` rounds to infinity as a double"),
            "the largest magnitude a double holds is about 1.8e308",
        )),
    }
}
