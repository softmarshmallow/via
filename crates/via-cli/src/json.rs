//! Canonical JSON (`documents.md` 8, `build.md` 3.1): a value that keeps every number as its
//! written text, a writer in the canonical form, and a reader for the files a build reads back
//! (`manifest.json`, `.previous`, a step's `diagnostics.json`).
//!
//! Numbers are kept as text so a lattice value written as exact metres (`1.005`) and a double
//! written as its shortest decimal pass through unchanged; the writer never formats a float
//! itself. Members are ordered bytewise by the `BTreeMap` of [`J::Obj`].

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum J {
    Null,
    Bool(bool),
    /// A number token, exactly as written.
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}

impl J {
    /// An integer.
    pub fn int(n: i64) -> J {
        J::Num(n.to_string())
    }

    /// A count.
    pub fn count(n: usize) -> J {
        J::Num(n.to_string())
    }

    /// A string.
    pub fn s(t: impl Into<String>) -> J {
        J::Str(t.into())
    }

    /// Exact metres from integer millimetres (`numbers.md` 3.4).
    pub fn mm(v: i64) -> J {
        J::Num(via_num::write_length(v))
    }

    /// A double as its shortest decimal; `None` when not finite.
    pub fn dbl(v: f64) -> Option<J> {
        via_num::write_double(v).ok().map(J::Num)
    }

    /// An optional string.
    pub fn opt(t: Option<&str>) -> J {
        t.map_or(J::Null, |t| J::Str(t.to_string()))
    }

    /// A list of strings.
    pub fn strs<'a>(items: impl IntoIterator<Item = &'a str>) -> J {
        J::Arr(items.into_iter().map(J::s).collect())
    }

    /// An empty object.
    pub fn obj() -> BTreeMap<String, J> {
        BTreeMap::new()
    }

    /// The member `k` of an object.
    pub fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.get(k),
            _ => None,
        }
    }

    /// The text of a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The value of an integer token.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            J::Num(n) => n.parse().ok(),
            _ => None,
        }
    }

    /// The items of a list.
    pub fn as_arr(&self) -> Option<&[J]> {
        match self {
            J::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// The canonical text, without a final line feed.
    pub fn canonical(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    /// The canonical text with one line feed: a file's bytes.
    pub fn file(&self) -> Vec<u8> {
        let mut s = self.canonical();
        s.push('\n');
        s.into_bytes()
    }

    fn write(&self, out: &mut String) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::Num(n) => out.push_str(n),
            J::Str(s) => write_str(s, out),
            J::Arr(items) => {
                out.push('[');
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    it.write(out);
                }
                out.push(']');
            }
            J::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Builds an object from pairs.
pub fn obj<const N: usize>(members: [(&str, J); N]) -> J {
    J::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Why a text is not JSON: the byte offset and a reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub reason: String,
}

/// Reads one JSON value; white space around it is allowed, nothing else.
pub fn parse(text: &str) -> Result<J, ParseError> {
    let mut p = Parser {
        b: text.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(p.err("text after the value"));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, reason: &str) -> ParseError {
        ParseError {
            offset: self.i,
            reason: reason.to_string(),
        }
    }

    fn ws(&mut self) {
        while self
            .b
            .get(self.i)
            .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.i += 1;
        }
    }

    fn lit(&mut self, word: &str, v: J) -> Result<J, ParseError> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(self.err("not a JSON value"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<J, ParseError> {
        if depth > 256 {
            return Err(self.err("nested too deeply"));
        }
        match self.b.get(self.i) {
            None => Err(self.err("unexpected end")),
            Some(b'n') => self.lit("null", J::Null),
            Some(b't') => self.lit("true", J::Bool(true)),
            Some(b'f') => self.lit("false", J::Bool(false)),
            Some(b'"') => Ok(J::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(J::Arr(items));
                }
                loop {
                    self.ws();
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(J::Arr(items));
                        }
                        _ => return Err(self.err("expected `,` or `]`")),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(J::Obj(m));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return Err(self.err("expected a member name"));
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return Err(self.err("expected `:`"));
                    }
                    self.i += 1;
                    self.ws();
                    let v = self.value(depth + 1)?;
                    if m.insert(k, v).is_some() {
                        return Err(self.err("a member name is repeated"));
                    }
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(J::Obj(m));
                        }
                        _ => return Err(self.err("expected `,` or `}`")),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                if self.b[self.i] == b'-' {
                    self.i += 1;
                }
                let digits = |p: &mut Self| {
                    let s = p.i;
                    while p.b.get(p.i).is_some_and(u8::is_ascii_digit) {
                        p.i += 1;
                    }
                    p.i > s
                };
                if !digits(self) {
                    return Err(self.err("a number needs digits"));
                }
                if self.b.get(self.i) == Some(&b'.') {
                    self.i += 1;
                    if !digits(self) {
                        return Err(self.err("a fraction needs digits"));
                    }
                }
                if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
                    self.i += 1;
                    if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                        self.i += 1;
                    }
                    if !digits(self) {
                        return Err(self.err("an exponent needs digits"));
                    }
                }
                let t = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("0");
                Ok(J::Num(t.to_string()))
            }
            Some(_) => Err(self.err("not a JSON value")),
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.b.get(self.i) {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(out).map_err(|_| self.err("not UTF-8"));
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = *self.b.get(self.i).ok_or_else(|| self.err("bad escape"))?;
                    self.i += 1;
                    let ch = match c {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return Err(self.err("lone surrogate"));
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            char::from_u32(code).ok_or_else(|| self.err("bad escape"))?
                        }
                        _ => return Err(self.err("bad escape")),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                Some(c) if *c < 0x20 => return Err(self.err("control character in a string")),
                Some(c) => {
                    out.push(*c);
                    self.i += 1;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let s = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .ok_or_else(|| self.err("bad \\u escape"))?;
        self.i += 4;
        Ok(s)
    }
}

/// The byte offset of the first difference between a file and its canonical form, or `None`
/// when it is canonical. A text that is not JSON differs at its parse error.
pub fn first_non_canonical(bytes: &[u8]) -> Option<usize> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Some(0);
    };
    let v = match parse(text) {
        Ok(v) => v,
        Err(e) => return Some(e.offset),
    };
    let canon = v.file();
    if canon == bytes {
        return None;
    }
    Some(
        canon
            .iter()
            .zip(bytes)
            .position(|(a, b)| a != b)
            .unwrap_or(canon.len().min(bytes.len())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_canonical_text() {
        let t = r#"{"a":[1,2.5,-0.125,1e3],"b":{"c":"x\"y\u0001"},"d":null,"e":true}"#;
        let v = parse(t).unwrap();
        assert_eq!(v.canonical(), t);
        assert_eq!(first_non_canonical(format!("{t}\n").as_bytes()), None);
    }

    #[test]
    fn finds_reordered_keys() {
        assert_eq!(first_non_canonical(b"{\"b\":1,\"a\":2}\n"), Some(2));
        assert!(first_non_canonical(b"{\"a\":1}").is_some());
    }
}
