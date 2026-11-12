//! Values in a record's `data`, `occurrences` and file locators.

use crate::json::Json;

/// A finite double. A non-finite value is never stored or written (R26), so the only way in is
/// [`Finite::new`], which refuses one.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Finite(f64);

impl Finite {
    /// `None` for NaN or an infinity.
    pub fn new(v: f64) -> Option<Finite> {
        v.is_finite().then_some(Finite(v))
    }

    /// The double.
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A fact in a record's `data` (`spec/diagnostics.md` 1): what agents act on, rather than the
/// message. Lengths are integer millimetres and are written as exact metres (`numbers.md` 3.4).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `true` or `false`.
    Bool(bool),
    /// An integer, written as digits.
    Int(i64),
    /// A double, written in the shortest form that reads back (ECMAScript Number::toString).
    Double(Finite),
    /// A length in integer millimetres, written as exact metres (`1005` as `1.005`).
    Length(i64),
    /// A string.
    Text(String),
    /// A list of values.
    List(Vec<Value>),
}

impl Value {
    /// A text value.
    pub fn text(s: impl Into<String>) -> Value {
        Value::Text(s.into())
    }

    /// A list of text values.
    pub fn texts<I, S>(items: I) -> Value
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Value::List(items.into_iter().map(|s| Value::Text(s.into())).collect())
    }

    /// A double; `None` when it is not finite.
    pub fn double(v: f64) -> Option<Value> {
        Finite::new(v).map(Value::Double)
    }

    pub(crate) fn to_json(&self) -> Json {
        match self {
            Value::Bool(b) => Json::Bool(*b),
            Value::Int(i) => Json::Int(*i),
            Value::Double(d) => Json::Double(d.get()),
            Value::Length(mm) => Json::Metres(*mm),
            Value::Text(s) => Json::Str(s.clone()),
            Value::List(l) => Json::Arr(l.iter().map(Value::to_json).collect()),
        }
    }

    /// The value as a number for ranking occurrences by a `worst` field (section 5): lengths in
    /// metres, so a length and a double compare in one unit. `None` for a non-number.
    pub(crate) fn rank(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Double(d) => Some(d.get()),
            Value::Length(mm) => Some(*mm as f64 / 1000.0),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::Text(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Text(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Value {
        Value::Int(i)
    }
}

impl From<Finite> for Value {
    fn from(d: Finite) -> Value {
        Value::Double(d)
    }
}
