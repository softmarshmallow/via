//! The JSON of `via kernel` files (`spec/geometry.md` 7.1): a reader that keeps each number's
//! text (so lengths are read exactly and twice-areas past 2^63 survive), and a canonical writer
//! (`documents.md` 8: members ordered bytewise, no whitespace, one closing line feed).

use std::collections::BTreeMap;

/// A JSON value. Numbers keep their text.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as written.
    Num(String),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object; members ordered bytewise.
    Obj(BTreeMap<String, Json>),
}

impl Json {
    /// A member of an object.
    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(k),
            _ => None,
        }
    }

    /// An array's items.
    pub fn arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(v) => Some(v),
            _ => None,
        }
    }

    /// A string's text.
    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// An integer.
    pub fn int(&self) -> Option<i64> {
        match self {
            Json::Num(t) => t.parse::<i64>().ok(),
            _ => None,
        }
    }

    /// A 128-bit integer.
    pub fn int128(&self) -> Option<i128> {
        match self {
            Json::Num(t) => t.parse::<i128>().ok(),
            _ => None,
        }
    }

    /// A double, read exactly by `via-num`.
    pub fn double(&self) -> Option<f64> {
        match self {
            Json::Num(t) => via_num::parse_double(t).ok(),
            _ => None,
        }
    }

    /// A number's text.
    pub fn num_text(&self) -> Option<&str> {
        match self {
            Json::Num(t) => Some(t),
            _ => None,
        }
    }

    /// An integer value.
    pub fn from_i(v: i64) -> Json {
        Json::Num(v.to_string())
    }

    /// A 128-bit integer value.
    pub fn from_i128(v: i128) -> Json {
        Json::Num(v.to_string())
    }

    /// A double, written as `numbers.md` 3.4 says; a non-finite value is `null` (never reached).
    pub fn from_f(v: f64) -> Json {
        match via_num::write_double(v) {
            Ok(t) => Json::Num(t),
            Err(_) => Json::Null,
        }
    }

    /// A string.
    pub fn s(v: &str) -> Json {
        Json::Str(v.to_string())
    }

    /// An object from pairs.
    pub fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    /// Canonical text, without the closing line feed.
    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(t) => out.push_str(t),
            Json::Str(s) => write_str(s, out),
            Json::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    x.write(out);
                }
                out.push(']');
            }
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, x)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    x.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Canonical text with one closing line feed.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s.push('\n');
        s
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Why a text is not JSON: the byte offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError(pub usize);

/// Parses JSON text.
pub fn parse(text: &str) -> Result<Json, JsonError> {
    let b = text.as_bytes();
    let mut i = 0;
    let v = value(b, &mut i)?;
    ws(b, &mut i);
    if i != b.len() {
        return Err(JsonError(i));
    }
    Ok(v)
}

fn ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\n' | b'\r') {
        *i += 1;
    }
}

fn value(b: &[u8], i: &mut usize) -> Result<Json, JsonError> {
    ws(b, i);
    let Some(&c) = b.get(*i) else {
        return Err(JsonError(*i));
    };
    match c {
        b'{' => {
            *i += 1;
            let mut m = BTreeMap::new();
            ws(b, i);
            if b.get(*i) == Some(&b'}') {
                *i += 1;
                return Ok(Json::Obj(m));
            }
            loop {
                ws(b, i);
                let k = string(b, i)?;
                ws(b, i);
                if b.get(*i) != Some(&b':') {
                    return Err(JsonError(*i));
                }
                *i += 1;
                let v = value(b, i)?;
                if m.insert(k, v).is_some() {
                    return Err(JsonError(*i));
                }
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b'}') => {
                        *i += 1;
                        return Ok(Json::Obj(m));
                    }
                    _ => return Err(JsonError(*i)),
                }
            }
        }
        b'[' => {
            *i += 1;
            let mut v = Vec::new();
            ws(b, i);
            if b.get(*i) == Some(&b']') {
                *i += 1;
                return Ok(Json::Arr(v));
            }
            loop {
                v.push(value(b, i)?);
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b']') => {
                        *i += 1;
                        return Ok(Json::Arr(v));
                    }
                    _ => return Err(JsonError(*i)),
                }
            }
        }
        b'"' => Ok(Json::Str(string(b, i)?)),
        b't' if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(Json::Bool(true))
        }
        b'f' if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(Json::Bool(false))
        }
        b'n' if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(Json::Null)
        }
        b'-' | b'0'..=b'9' => {
            let s = *i;
            *i += 1;
            while *i < b.len() && matches!(b[*i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
                *i += 1;
            }
            let t = std::str::from_utf8(&b[s..*i]).map_err(|_| JsonError(s))?;
            Ok(Json::Num(t.to_string()))
        }
        _ => Err(JsonError(*i)),
    }
}

fn string(b: &[u8], i: &mut usize) -> Result<String, JsonError> {
    if b.get(*i) != Some(&b'"') {
        return Err(JsonError(*i));
    }
    *i += 1;
    let mut out: Vec<u8> = Vec::new();
    loop {
        let Some(&c) = b.get(*i) else {
            return Err(JsonError(*i));
        };
        *i += 1;
        match c {
            b'"' => break,
            b'\\' => {
                let Some(&e) = b.get(*i) else {
                    return Err(JsonError(*i));
                };
                *i += 1;
                match e {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'/' => out.push(b'/'),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'u' => {
                        let h = b.get(*i..*i + 4).ok_or(JsonError(*i))?;
                        let h = std::str::from_utf8(h).map_err(|_| JsonError(*i))?;
                        let cp = u32::from_str_radix(h, 16).map_err(|_| JsonError(*i))?;
                        *i += 4;
                        let ch = char::from_u32(cp).ok_or(JsonError(*i))?;
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    }
                    _ => return Err(JsonError(*i)),
                }
            }
            c => out.push(c),
        }
    }
    String::from_utf8(out).map_err(|_| JsonError(*i))
}
