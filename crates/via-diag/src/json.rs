//! The canonical JSON writer (`spec/documents.md` 8, which `diagnostics.json` follows).
//!
//! Object members bytewise by key (a `BTreeMap<String, _>` orders by bytes), no white space
//! outside strings, only `"`, `\` and U+0000 to U+001F escaped (as `\"`, `\\`, `\u00xx`), lengths as
//! exact metres from integer millimetres and doubles as ECMAScript prints them (`numbers.md` 3.4).
//! It is small and owned here rather than taken from a serialiser, because each of these rules
//! differs from serde_json's defaults.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Json {
    Bool(bool),
    Int(i64),
    /// Finite by construction: only `Finite` values become one.
    Double(f64),
    Metres(i64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub(crate) fn obj() -> BTreeMap<String, Json> {
        BTreeMap::new()
    }

    pub(crate) fn write(&self, out: &mut String) {
        match self {
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Json::Double(d) => out.push_str(&format_double(*d)),
            Json::Metres(mm) => out.push_str(&format_metres(*mm)),
            Json::Str(s) => write_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(members) => {
                out.push('{');
                for (i, (k, v)) in members.iter().enumerate() {
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

    /// The canonical bytes, without the final line feed (a document adds it).
    pub(crate) fn to_canonical(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }
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

/// A finite double as ECMAScript's Number::toString prints it (`3.0` as `3`, `1e21` as `1e+21`),
/// through `via_num::write_double` so the rule has one definition. Every double this crate writes
/// is a `Finite` already, so the non-finite error cannot arise; were it to, `null` (what
/// ECMAScript's JSON.stringify writes for one) keeps the document valid JSON rather than panicking.
pub(crate) fn format_double(d: f64) -> String {
    via_num::write_double(d).unwrap_or_else(|_| "null".to_string())
}

/// Integer millimetres as exact metres, through `via_num::write_length`.
pub(crate) fn format_metres(mm: i64) -> String {
    via_num::write_length(mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metres_are_exact() {
        assert_eq!(format_metres(1005), "1.005");
        assert_eq!(format_metres(3000), "3");
        assert_eq!(format_metres(-250), "-0.25");
        assert_eq!(format_metres(0), "0");
        assert_eq!(format_metres(181_500), "181.5");
        assert_eq!(format_metres(i64::MIN), "-9223372036854775.808");
    }

    #[test]
    fn doubles_follow_ecmascript() {
        assert_eq!(format_double(3.0), "3");
        assert_eq!(format_double(0.1), "0.1");
        assert_eq!(format_double(1e21), "1e+21");
        assert_eq!(format_double(1e-7), "1e-7");
        assert_eq!(format_double(5e-324), "5e-324");
        assert_eq!(format_double(1.23e-18), "1.23e-18");
        assert_eq!(format_double(-0.0), "0");
        assert_eq!(format_double(2.4), "2.4");
    }

    #[test]
    fn strings_escape_only_three_things() {
        let mut s = String::new();
        write_str("a\"b\\c\nd\u{1f}é/", &mut s);
        assert_eq!(s, "\"a\\\"b\\\\c\\u000ad\\u001fé/\"");
    }

    #[test]
    fn members_sort_bytewise() {
        let mut o = Json::obj();
        o.insert("b".into(), Json::Int(1));
        o.insert("Z".into(), Json::Int(2));
        o.insert(
            "a".into(),
            Json::Arr(vec![Json::Bool(true), Json::Metres(1500)]),
        );
        assert_eq!(
            Json::Obj(o).to_canonical(),
            "{\"Z\":2,\"a\":[true,1.5],\"b\":1}"
        );
    }
}
