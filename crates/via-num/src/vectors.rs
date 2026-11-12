//! The vector file of `numbers.md` 10 and the null digest.
//!
//! `spec/vectors/numbers.toml` holds one table per vector, `[vector.<id>]` with `fn`, `args` and
//! `expected`, in the chapter's order. This module evaluates each vector through the code the build
//! uses and writes the result in the form `via vectors` prints (`build.md` 14): one canonical JSON
//! object per line, `{"got":<value>,"id":"<id>"}`. It judges nothing; the caller compares `got`
//! with `expected`.
//!
//! The **null digest** is the BLAKE3 of those lines, each ending in a line feed, over every vector
//! this crate evaluates. Two systems that print the same hex line computed every vector to the same
//! bits; it is the one line the cross-system check compares.
//!
//! Vectors of `point_at`, `measure`, `samples` and `digest` are in the same file but need the exact
//! geometric predicates of `geometry.md` 1 and the digest form of `identity.md` 10; they are
//! evaluated by the crates that own those, and this module reports them as not its own.

use crate::error::NumError;
use crate::frame::Frame;
use toml::Value;

/// The functions of `numbers.md` 10 that another crate evaluates.
pub const DELEGATED: &[&str] = &["point_at", "measure", "samples", "digest"];

/// A computed result, before it is written as JSON.
#[derive(Clone, Debug, PartialEq)]
pub enum Got {
    /// A 64-bit word, written `0x` and 16 upper-case hex digits.
    Word(u64),
    /// A double, written as its bits like a word.
    Double(f64),
    /// An integer: millimetres, millidegrees, a key.
    Int(i64),
    /// Text, such as a written number.
    Text(String),
    /// Several results, in the order of the table's columns.
    List(Vec<Got>),
    /// A rejection, `{ code = "<code>" }`.
    Code(&'static str),
}

impl Got {
    /// The canonical JSON of this value.
    pub fn to_json(&self) -> String {
        match self {
            Got::Word(w) => format!("\"0x{w:016X}\""),
            Got::Double(d) => format!("\"0x{:016X}\"", d.to_bits()),
            Got::Int(i) => i.to_string(),
            Got::Text(t) => json_string(t),
            Got::List(items) => {
                let inner: Vec<String> = items.iter().map(Got::to_json).collect();
                format!("[{}]", inner.join(","))
            }
            Got::Code(c) => format!("{{\"code\":{}}}", json_string(c)),
        }
    }
}

/// Why a vector could not be evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VectorError {
    /// The file is not TOML, or a vector is not of the chapter's form.
    Form { id: Option<String>, message: String },
    /// The function is not one of this crate's ([`DELEGATED`] or unknown).
    NotOurs { id: String, func: String },
}

impl std::fmt::Display for VectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorError::Form {
                id: Some(id),
                message,
            } => write!(f, "vector {id}: {message}"),
            VectorError::Form { id: None, message } => write!(f, "vector file: {message}"),
            VectorError::NotOurs { id, func } => {
                write!(f, "vector {id}: `{func}` is not evaluated by via-num")
            }
        }
    }
}

impl std::error::Error for VectorError {}

/// One vector of the file, evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The vector's id, its key under `vector`.
    pub id: String,
    /// Its `fn`.
    pub func: String,
    /// The computed value as canonical JSON, or `None` for a function another crate evaluates.
    pub got: Option<String>,
    /// The file's `expected`, as canonical JSON, for the caller to compare.
    pub expected: String,
}

impl Outcome {
    /// The line `via vectors` prints for this vector, if it was evaluated here.
    pub fn line(&self) -> Option<String> {
        self.got
            .as_ref()
            .map(|got| format!("{{\"got\":{got},\"id\":{}}}", json_string(&self.id)))
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A TOML value as canonical JSON: table keys sorted, no spaces. A float is refused: the file
/// writes doubles as bit strings and decimals as text, never as TOML floats.
fn toml_json(v: &Value) -> Result<String, String> {
    Ok(match v {
        Value::String(s) => json_string(s),
        Value::Integer(i) => i.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Array(items) => {
            let inner: Result<Vec<String>, String> = items.iter().map(toml_json).collect();
            format!("[{}]", inner?.join(","))
        }
        Value::Table(t) => {
            let mut entries: Vec<(&String, &Value)> = t.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let inner: Result<Vec<String>, String> = entries
                .into_iter()
                .map(|(k, v)| Ok(format!("{}:{}", json_string(k), toml_json(v)?)))
                .collect();
            format!("{{{}}}", inner?.join(","))
        }
        Value::Float(_) => return Err("a TOML float; write the bits or the text".to_string()),
        Value::Datetime(_) => return Err("a TOML datetime".to_string()),
    })
}

fn word(v: &Value) -> Result<u64, String> {
    match v {
        Value::String(s) => {
            let hex = s
                .strip_prefix("0x")
                .filter(|h| {
                    h.len() == 16
                        && h.bytes()
                            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
                })
                .ok_or_else(|| format!("`{s}` is not 0x and 16 upper-case hex digits"))?;
            u64::from_str_radix(hex, 16).map_err(|e| e.to_string())
        }
        Value::Integer(i) => u64::try_from(*i).map_err(|_| format!("{i} is not a word")),
        other => Err(format!("{other} is not a word")),
    }
}

fn double(v: &Value) -> Result<f64, String> {
    match v {
        Value::String(s) if s.starts_with("0x") => word(v).map(f64::from_bits),
        Value::String(s) => crate::parse_double(s).map_err(|e| e.to_string()),
        other => Err(format!("{other} is not a double's bits or text")),
    }
}

fn int(v: &Value) -> Result<i64, String> {
    v.as_integer()
        .ok_or_else(|| format!("{v} is not an integer"))
}

fn text(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| format!("{v} is not a string"))
}

fn array(v: &Value) -> Result<&[Value], String> {
    v.as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{v} is not an array"))
}

fn point(v: &Value) -> Result<(i64, i64), String> {
    match array(v)? {
        [x, y] => Ok((int(x)?, int(y)?)),
        _ => Err(format!("{v} is not a point [x, y]")),
    }
}

fn arg(args: &[Value], i: usize) -> Result<&Value, String> {
    args.get(i).ok_or_else(|| format!("missing argument {i}"))
}

fn code(e: NumError) -> Got {
    Got::Code(e.code())
}

/// Evaluates one vector's function. `Ok(None)` is a function this crate does not evaluate.
pub fn evaluate(func: &str, args: &[Value]) -> Result<Option<Got>, String> {
    let a = |i| arg(args, i);
    let got = match func {
        "splitmix64" => Got::Word(crate::splitmix64(word(a(0)?)?)),
        "stream" => {
            let parts: Result<Vec<&str>, String> = array(a(1)?)?.iter().map(text).collect();
            Got::Word(crate::stream(word(a(0)?)?, &parts?))
        }
        "draw" => {
            let keys: Result<Vec<i64>, String> = array(a(1)?)?.iter().map(int).collect();
            Got::Word(crate::draw(word(a(0)?)?, &keys?))
        }
        "unit" => Got::Double(crate::unit(word(a(0)?)?)),
        "facing" => {
            let (s, c) = crate::facing_sin_cos(int(a(0)?)?);
            Got::List(vec![Got::Double(s), Got::Double(c)])
        }
        "bearing" => match crate::bearing(int(a(0)?)?, int(a(1)?)?) {
            Some(m) => Got::Int(m),
            None => Got::Code("geom.no_length"),
        },
        "frame" => {
            let mut frame = Frame::WORLD;
            let mut result = Ok(());
            for f in array(a(0)?)? {
                let parts = array(f)?;
                let [origin, facing, scale] = parts else {
                    return Err(format!("{f} is not [origin, facing, scale]"));
                };
                match frame.nest(point(origin)?, int(facing)?, double(scale)?) {
                    Ok(next) => frame = next,
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                }
            }
            let local = point(a(1)?)?;
            match result.and_then(|()| frame.to_world(local)) {
                Ok((x, y)) => Got::List(vec![Got::Int(x), Got::Int(y)]),
                Err(e) => code(e),
            }
        }
        "length" => match crate::parse_length(text(a(0)?)?) {
            Ok(mm) => Got::Int(mm),
            Err(e) => code(e),
        },
        "metres" => Got::Double(crate::metres(int(a(0)?)?)),
        "sum" => {
            let values: Result<Vec<f64>, String> = array(a(0)?)?.iter().map(double).collect();
            Got::Double(crate::sum(&values?))
        }
        "min" => Got::Double(crate::min(double(a(0)?)?, double(a(1)?)?)),
        "max" => Got::Double(crate::max(double(a(0)?)?, double(a(1)?)?)),
        "write" => match crate::write_double(double(a(0)?)?) {
            Ok(t) => Got::Text(t),
            Err(e) => code(e),
        },
        "write_length" => Got::Text(crate::write_length(int(a(0)?)?)),
        "noise" => {
            match crate::gradient_noise(word(a(0)?)?, int(a(1)?)?, double(a(2)?)?, double(a(3)?)?) {
                Ok(n) => Got::Double(n),
                Err(e) => code(e),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(got))
}

/// Reads a vector file and evaluates every vector, in the file's order.
///
/// A function this crate does not evaluate gives an outcome with `got = None`; the caller decides
/// whether that is allowed ([`DELEGATED`]).
pub fn evaluate_file(text: &str) -> Result<Vec<Outcome>, VectorError> {
    let form = |id: Option<&str>, message: String| VectorError::Form {
        id: id.map(str::to_string),
        message,
    };
    let doc: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| form(None, e.to_string()))?;
    let vectors = doc
        .get("vector")
        .and_then(Value::as_table)
        .ok_or_else(|| form(None, "no [vector] table".to_string()))?;
    let mut out = Vec::with_capacity(vectors.len());
    for (id, v) in vectors {
        let at = |m: String| form(Some(id), m);
        let t = v.as_table().ok_or_else(|| at("not a table".to_string()))?;
        let func = t
            .get("fn")
            .and_then(Value::as_str)
            .ok_or_else(|| at("no `fn`".to_string()))?;
        let args = t
            .get("args")
            .and_then(Value::as_array)
            .ok_or_else(|| at("no `args` array".to_string()))?;
        let expected = t
            .get("expected")
            .ok_or_else(|| at("no `expected`".to_string()))?;
        let got = evaluate(func, args).map_err(at)?;
        out.push(Outcome {
            id: id.clone(),
            func: func.to_string(),
            got: got.map(|g| g.to_json()),
            expected: toml_json(expected).map_err(at)?,
        });
    }
    Ok(out)
}

/// The null digest: BLAKE3 of the `via vectors` lines of every vector evaluated here, each ending
/// in a line feed, as 64 lower-case hex digits.
pub fn null_digest(outcomes: &[Outcome]) -> String {
    let mut hasher = blake3::Hasher::new();
    for line in outcomes.iter().filter_map(Outcome::line) {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}
