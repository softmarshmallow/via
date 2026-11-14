//! The JSON a glTF model holds (RFC 8259), read for measuring (`assets.md` 3), and the two
//! decodings its URIs need: percent-decoding and `data:` URIs (RFC 2397, base64 by RFC 4648).
//!
//! A small reader of our own: a model is read for positions, indices and a few names, so a
//! general JSON crate is not needed. Numbers are read by the standard library's correctly rounded
//! decimal reader, so the same text gives the same double on every system (R26). A number that
//! does not fit a double is kept as infinity, and the measurer refuses it (`pack.bad_model`).

/// A JSON value. Objects keep their members in text order; a repeated name keeps its last value
/// on lookup, as most readers do.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// A member of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// A whole number, 0 or more, that fits `usize`.
    pub fn as_index(&self) -> Option<usize> {
        let n = self.as_f64()?;
        if n.is_nan() || n < 0.0 || n.trunc() != n || n > 9.0e15 {
            return None;
        }
        usize::try_from(n as u64).ok()
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Whether any number in the value is not finite.
    pub fn non_finite(&self, here: &str) -> Option<String> {
        match self {
            Json::Num(n) if !n.is_finite() => Some(here.to_string()),
            Json::Arr(a) => a
                .iter()
                .enumerate()
                .find_map(|(i, v)| v.non_finite(&format!("{here}/{i}"))),
            Json::Obj(m) => m
                .iter()
                .find_map(|(k, v)| v.non_finite(&format!("{here}/{}", pointer_part(k)))),
            _ => None,
        }
    }
}

/// One reference token of a JSON Pointer (RFC 6901): `~` as `~0`, `/` as `~1`.
pub fn pointer_part(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

/// How deep arrays and objects may nest: deeper text is refused, never a stack overflow.
const MAX_DEPTH: usize = 256;

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

/// Reads JSON text. The error says what was wrong and at which byte.
pub fn parse(bytes: &[u8]) -> Result<Json, String> {
    let mut p = P { b: bytes, i: 0 };
    // A byte order mark is allowed before glTF JSON in neither form; RFC 8259 lets a reader
    // ignore one, so it is skipped.
    if p.b.starts_with(&[0xEF, 0xBB, 0xBF]) {
        p.i = 3;
    }
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("text after the value at byte {}", p.i));
    }
    Ok(v)
}

impl P<'_> {
    fn ws(&mut self) {
        while let Some(c) = self.b.get(self.i) {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("{what} at byte {}", self.i))
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return self.err("nesting too deep");
        }
        match self.b.get(self.i) {
            None => self.err("the text ends where a value was expected"),
            Some(b'{') => {
                self.i += 1;
                let mut m = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.err("a member name was expected");
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.err("`:` was expected");
                    }
                    self.i += 1;
                    self.ws();
                    let v = self.value(depth + 1)?;
                    m.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return self.err("`,` or `}` was expected"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    self.ws();
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return self.err("`,` or `]` was expected"),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'n') => self.word("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => self.err("not a JSON value"),
        }
    }

    fn word(&mut self, w: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            self.err("not a JSON value")
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let s = p.i;
            while p.b.get(p.i).is_some_and(u8::is_ascii_digit) {
                p.i += 1;
            }
            p.i - s
        };
        let int_start = self.i;
        if digits(self) == 0 {
            return self.err("a digit was expected");
        }
        if self.b[int_start] == b'0' && self.i - int_start > 1 {
            return self.err("a number may not start with `0`");
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            if digits(self) == 0 {
                return self.err("a digit was expected after `.`");
            }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if digits(self) == 0 {
                return self.err("a digit was expected in the exponent");
            }
        }
        let text = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("0");
        // Rust's reader rounds correctly to nearest, ties to even; an overflow is infinity.
        text.parse::<f64>()
            .map(Json::Num)
            .or_else(|_| self.err("not a number"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let Some(h) = self.b.get(self.i..self.i + 4) else {
            return self.err("four hex digits were expected");
        };
        let s = std::str::from_utf8(h).unwrap_or("");
        let v =
            u32::from_str_radix(s, 16).or_else(|_| self.err("four hex digits were expected"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return self.err("the text ends inside a string");
            };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return self.err("the text ends inside a string");
                    };
                    self.i += 1;
                    let ch = match e {
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
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return self.err("a lone surrogate");
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return self.err("a lone surrogate");
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            match char::from_u32(cp) {
                                Some(c) => c,
                                None => return self.err("a lone surrogate"),
                            }
                        }
                        _ => return self.err("an unknown escape"),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c if c < 0x20 => return self.err("a control character inside a string"),
                c => out.push(c),
            }
        }
        String::from_utf8(out).or_else(|_| self.err("a string that is not UTF-8"))
    }
}

/// Percent-decoding of a URI (RFC 3986 2.1). `None` for a `%` not followed by two hex digits,
/// or a result that is not UTF-8.
pub fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The bytes of a `data:` URI (RFC 2397): base64 when the media part ends in `;base64`, else
/// percent-decoded text. `None` when it is not a `data:` URI or does not decode.
pub fn data_uri(uri: &str) -> Option<Vec<u8>> {
    let rest = uri.strip_prefix("data:")?;
    let (media, data) = rest.split_once(',')?;
    if media.ends_with(";base64") {
        base64(data)
    } else {
        let b = data.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
                out.push(u8::from_str_radix(h, 16).ok()?);
                i += 3;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        Some(out)
    }
}

/// Base64 (RFC 4648 4), padding optional.
fn base64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    };
    let b: Vec<u8> = s.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::with_capacity(b.len() * 3 / 4);
    for chunk in b.chunks(4) {
        let mut acc: u32 = 0;
        for (k, c) in chunk.iter().enumerate() {
            acc |= val(*c)? << (18 - 6 * k);
        }
        let bytes = acc.to_be_bytes();
        match chunk.len() {
            4 => out.extend_from_slice(&bytes[1..4]),
            3 => out.extend_from_slice(&bytes[1..3]),
            2 => out.push(bytes[1]),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json() {
        let j = parse(br#"{"a": [1, -2.5e1, true, null], "b": "x\u00e9\n"}"#).unwrap();
        assert_eq!(
            j.get("a").unwrap().as_arr().unwrap()[1].as_f64(),
            Some(-25.0)
        );
        assert_eq!(j.get("b").unwrap().as_str(), Some("x\u{e9}\n"));
        assert!(parse(b"[1,]").is_err());
        assert!(parse(b"01").is_err());
        assert_eq!(parse(b"1e400").unwrap().non_finite(""), Some(String::new()));
    }

    #[test]
    fn decodes_uris() {
        assert_eq!(
            percent_decode("my%20mesh.bin").as_deref(),
            Some("my mesh.bin")
        );
        assert_eq!(
            data_uri("data:application/octet-stream;base64,AAECAw==").unwrap(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(data_uri("data:,a%20b").unwrap(), b"a b".to_vec());
    }
}
