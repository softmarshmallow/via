//! Words of Rust source, as the deny-list reads them (`spec/vocabulary.md` 7).
//!
//! A hand-written lexer rather than `syn` or `proc-macro2`: the test must read every `.rs` file of
//! a core crate, macro bodies and files that do not parse included, and must skip doc comments,
//! which `proc-macro2` turns into `#[doc = "..."]` string literals. It knows just enough of Rust's
//! lexical grammar to tell comments, string and character literals, lifetimes and identifiers
//! apart; everything else is skipped.

use std::fmt;

/// What a unit of source is; the two split into words by different rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// An identifier, or a path of identifiers joined by `::`.
    Ident,
    /// The text of a string, byte string or C string literal.
    Literal,
}

/// One identifier path or string literal, with the line it starts on and its lowercased words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub kind: UnitKind,
    pub line: usize,
    pub text: String,
    pub words: Vec<String>,
}

/// A file the lexer cannot read to its end: an unterminated literal or block comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Splits an identifier into lowercased words.
///
/// The chapter's rule: split at `_`, `::` and each lower-to-upper case change. Two further
/// splits make the test stricter, never looser: an acronym ends before its last capital when a
/// lower-case letter follows (`HTTPRoad` is `http road`), and digits separate words and are
/// dropped (`road2` is `road`). Without them a noun hides behind an acronym or a number.
pub fn split_ident(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    for part in s.split(['_', ':']) {
        let chars: Vec<char> = part.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let (p, c) = (chars[i - 1], chars[i]);
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            let boundary = (p.is_lowercase() && c.is_uppercase())
                || (p.is_uppercase() && c.is_uppercase() && next_lower)
                || (p.is_ascii_digit() != c.is_ascii_digit());
            if boundary {
                push_word(&mut words, &chars[start..i]);
                start = i;
            }
        }
        push_word(&mut words, &chars[start..]);
    }
    words
}

fn push_word(words: &mut Vec<String>, chars: &[char]) {
    if chars.iter().any(|c| c.is_alphabetic()) {
        let w: String = chars.iter().filter(|c| c.is_alphabetic()).collect();
        words.push(w.to_lowercase());
    }
}

/// Splits a string literal's text into lowercased words at every character that is not a letter.
pub fn split_literal(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

struct Lexer {
    c: Vec<char>,
    i: usize,
    line: usize,
}

impl Lexer {
    fn peek(&self, k: usize) -> Option<char> {
        self.c.get(self.i + k).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.c.get(self.i).copied();
        if let Some(ch) = ch {
            if ch == '\n' {
                self.line += 1;
            }
            self.i += 1;
        }
        ch
    }

    fn err(&self, line: usize, message: &str) -> LexError {
        LexError {
            line,
            message: message.to_string(),
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek(0) {
            if ch == '\n' {
                break;
            }
            self.bump();
        }
    }

    /// Block comments nest in Rust, so depth is counted.
    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let start = self.line;
        let mut depth = 0usize;
        loop {
            match (self.peek(0), self.peek(1)) {
                (Some('/'), Some('*')) => {
                    self.bump();
                    self.bump();
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.bump();
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                (Some(_), _) => {
                    self.bump();
                }
                (None, _) => return Err(self.err(start, "unterminated block comment")),
            }
        }
    }

    /// Reads a quoted literal after its opening `"`; escapes become a space, so `"\nroad"` reads
    /// as `road`, not `nroad`.
    fn read_string(&mut self) -> Result<String, LexError> {
        let start = self.line;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err(start, "unterminated string literal")),
                Some('"') => return Ok(out),
                Some('\\') => {
                    out.push(' ');
                    match self.bump() {
                        Some('u') if self.peek(0) == Some('{') => {
                            while let Some(ch) = self.bump() {
                                if ch == '}' {
                                    break;
                                }
                            }
                        }
                        Some('x') => {
                            self.bump();
                            self.bump();
                        }
                        Some('\n') => {
                            while self.peek(0).is_some_and(char::is_whitespace) {
                                self.bump();
                            }
                        }
                        Some(_) => {}
                        None => return Err(self.err(start, "unterminated string literal")),
                    }
                }
                Some(ch) => out.push(ch),
            }
        }
    }

    /// Reads a raw literal starting at its `#`s or `"`: `r#"..."#` ends at a quote followed by
    /// as many `#` as it opened with.
    fn read_raw_string(&mut self) -> Result<String, LexError> {
        let start = self.line;
        let mut hashes = 0;
        while self.peek(0) == Some('#') {
            self.bump();
            hashes += 1;
        }
        if self.bump() != Some('"') {
            return Err(self.err(start, "malformed raw string literal"));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err(start, "unterminated raw string literal")),
                Some('"') if (0..hashes).all(|k| self.peek(k) == Some('#')) => {
                    for _ in 0..hashes {
                        self.bump();
                    }
                    return Ok(out);
                }
                Some(ch) => out.push(ch),
            }
        }
    }

    /// After a `'`: a character literal is skipped (one character is no word); otherwise it is a
    /// lifetime or label, returned as an identifier so `'road:` is still read.
    fn read_quote(&mut self) -> Option<String> {
        if self.peek(0) == Some('\\') {
            while let Some(ch) = self.bump() {
                if ch == '\'' {
                    break;
                }
                if ch == '\\' {
                    self.bump();
                }
            }
            return None;
        }
        if self.peek(1) == Some('\'') {
            self.bump();
            self.bump();
            return None;
        }
        if self.peek(0).is_some_and(is_ident_start) {
            return Some(self.read_ident_chars());
        }
        None
    }

    fn read_ident_chars(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek(0) {
            if !is_ident_continue(ch) {
                break;
            }
            s.push(ch);
            self.bump();
        }
        s
    }

    fn skip_number(&mut self) {
        while let Some(ch) = self.peek(0) {
            let fraction_dot = ch == '.' && self.peek(1).is_some_and(|n| n.is_ascii_digit());
            if !(is_ident_continue(ch) || fraction_dot) {
                break;
            }
            self.bump();
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Reads identifiers and string literals from Rust source, skipping comments of every kind.
pub fn units(src: &str) -> Result<Vec<Unit>, LexError> {
    let mut lx = Lexer {
        c: src.chars().collect(),
        i: 0,
        line: 1,
    };
    let mut out: Vec<Unit> = Vec::new();
    // Set when `::` follows an identifier, so the next identifier extends the same path unit.
    let mut path_open = false;
    while let Some(ch) = lx.peek(0) {
        let line = lx.line;
        match ch {
            '/' if lx.peek(1) == Some('/') => lx.skip_line_comment(),
            '/' if lx.peek(1) == Some('*') => lx.skip_block_comment()?,
            '"' => {
                lx.bump();
                let s = lx.read_string()?;
                push_literal(&mut out, line, s);
                path_open = false;
            }
            '\'' => {
                lx.bump();
                if let Some(id) = lx.read_quote() {
                    push_ident(&mut out, line, id, false);
                }
                path_open = false;
            }
            ':' if lx.peek(1) == Some(':') => {
                lx.bump();
                lx.bump();
                path_open = out.last().is_some_and(|u| u.kind == UnitKind::Ident);
            }
            c if c.is_ascii_digit() => {
                lx.skip_number();
                path_open = false;
            }
            c if is_ident_start(c) => {
                let id = lx.read_ident_chars();
                let next = lx.peek(0);
                let is_raw_prefix = matches!(id.as_str(), "r" | "br" | "cr");
                if is_raw_prefix && (next == Some('"') || next == Some('#')) {
                    if id == "r" && next == Some('#') && lx.peek(1).is_some_and(is_ident_start) {
                        // A raw identifier, `r#type`.
                        lx.bump();
                        let raw = lx.read_ident_chars();
                        push_ident(&mut out, line, raw, path_open);
                    } else {
                        let s = lx.read_raw_string()?;
                        push_literal(&mut out, line, s);
                        path_open = false;
                    }
                    continue;
                }
                if matches!(id.as_str(), "b" | "c") && next == Some('"') {
                    lx.bump();
                    let s = lx.read_string()?;
                    push_literal(&mut out, line, s);
                    path_open = false;
                    continue;
                }
                if id == "b" && next == Some('\'') {
                    lx.bump();
                    lx.read_quote();
                    path_open = false;
                    continue;
                }
                push_ident(&mut out, line, id, path_open);
                path_open = false;
            }
            c if c.is_whitespace() => {
                lx.bump();
            }
            _ => {
                lx.bump();
                path_open = false;
            }
        }
    }
    Ok(out)
}

fn push_literal(out: &mut Vec<Unit>, line: usize, s: String) {
    let words = split_literal(&s);
    out.push(Unit {
        kind: UnitKind::Literal,
        line,
        text: s,
        words,
    });
}

fn push_ident(out: &mut Vec<Unit>, line: usize, id: String, extend: bool) {
    if extend {
        if let Some(last) = out.last_mut() {
            last.text.push_str("::");
            last.text.push_str(&id);
            last.words.extend(split_ident(&id));
            return;
        }
    }
    let words = split_ident(&id);
    out.push(Unit {
        kind: UnitKind::Ident,
        line,
        text: id,
        words,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn splits_identifiers() {
        assert_eq!(split_ident("road_width"), w(&["road", "width"]));
        assert_eq!(split_ident("MainWay"), w(&["main", "way"]));
        // The acronym rule is why `tree` needs its allow-list sense: `BTreeMap` is `b tree map`.
        assert_eq!(split_ident("BTreeMap"), w(&["b", "tree", "map"]));
        assert_eq!(split_ident("HTTPRoad"), w(&["http", "road"]));
        assert_eq!(split_ident("ROAD_2X"), w(&["road", "x"]));
        assert_eq!(split_ident("road2"), w(&["road"]));
    }

    #[test]
    fn splits_literals() {
        assert_eq!(
            split_literal("two-lakes, BTreeMap"),
            w(&["two", "lakes", "btreemap"])
        );
    }

    #[test]
    fn skips_comments_and_reads_literals() {
        let src = "// road\n/* a /* lake */ river */\n/// bridge\n//! deck\nfn x() { let y = \"wa\\nter\"; let z = r#\"sho\"re\"#; let c = '\"'; let l: &'beck str; std::mem::take }";
        let u = units(src).expect("lexes");
        let words: Vec<String> = u.iter().flat_map(|u| u.words.clone()).collect();
        for gone in ["road", "lake", "river", "bridge", "deck"] {
            assert!(!words.iter().any(|x| x == gone), "{gone} came through");
        }
        assert!(words.iter().any(|x| x == "beck"));
        let path = u
            .iter()
            .find(|u| u.text == "std::mem::take")
            .expect("path merged");
        assert_eq!(path.words, w(&["std", "mem", "take"]));
        assert_eq!(path.line, 5);
        let lit = u
            .iter()
            .find(|u| u.kind == UnitKind::Literal)
            .expect("literal");
        assert_eq!(lit.words, w(&["wa", "ter"]));
    }

    #[test]
    fn unterminated_is_an_error() {
        let e = units("fn a() {}\nlet s = \"open").expect_err("unterminated");
        assert_eq!(e.line, 2);
    }
}
