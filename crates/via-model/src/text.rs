//! Reading values of the merged world, and making located diagnostics.
//!
//! Every value the model reads comes from `via-doc`'s tree, which holds numbers as their
//! normalised text, so a length is read exactly to millimetres (`numbers.md` 3.1) and any other
//! number to the nearest double (3.3). A value of the wrong form was already refused by the
//! document checks; a reader here that finds one anyway gives `None`, and the caller says what
//! it needed.

use via_diag::nearest::nearest;
use via_diag::{codes, registry, Code, Diag, KeyPath, Position, Site, Subject, Text};
use via_doc::{Num, Value};
use via_num::NumError;

/// A diagnostic from its parts; message and hint are never blank.
pub fn diag(code: Code, subject: Subject, site: Site, message: &str, hint: &str) -> Diag {
    let message = Text::new(message).unwrap_or_else(|_| via_diag::text!("a model fault"));
    let hint = Text::new(hint).unwrap_or_else(|_| via_diag::text!("see the chapter for the code"));
    Diag::new(code, subject, site, message, hint)
}

/// A diagnostic about an entity, located at one of its keys: the entity is at fault, the key is
/// the text to edit. A fault in the key's own value takes [`at_own_key`].
pub fn at_key(code: Code, top: &str, id: &str, key: &str, message: &str, hint: &str) -> Diag {
    diag(
        code,
        Subject::entity(top, id),
        Site::Key(KeyPath::key(key)),
        message,
        hint,
    )
}

/// A diagnostic about one key's value: the subject is the address and the key path
/// (`diagnostics.md` 1.1, "one key of it").
pub fn at_own_key(code: Code, top: &str, id: &str, key: &str, message: &str, hint: &str) -> Diag {
    let p = KeyPath::key(key);
    diag(
        code,
        Subject::entity(top, id).with_keys(&p),
        Site::Key(p),
        message,
        hint,
    )
}

/// A diagnostic at a key path of an entity.
pub fn at_path(code: Code, top: &str, id: &str, path: &[&str], message: &str, hint: &str) -> Diag {
    let mut p = KeyPath::key(path[0]);
    for k in &path[1..] {
        p = p.then(*k);
    }
    diag(code, Subject::entity(top, id), Site::Key(p), message, hint)
}

/// A diagnostic at an entity's header.
pub fn at_header(code: Code, top: &str, id: &str, message: &str, hint: &str) -> Diag {
    diag(code, Subject::entity(top, id), Site::Header, message, hint)
}

/// A key path from its parts.
pub fn path(parts: &[&str]) -> KeyPath {
    let mut p = KeyPath::key(parts[0]);
    for k in &parts[1..] {
        p = p.then(*k);
    }
    p
}

/// A position in plan.
pub fn pos(x: i64, y: i64) -> Position {
    Position::plan(x, y)
}

/// The code of a number fault, by its registry name.
pub fn num_code(e: &NumError) -> Code {
    registry::lookup(e.code()).unwrap_or(codes::diag::INTERNAL)
}

/// A number fault at a key.
pub fn num_diag(e: &NumError, top: &str, id: &str, key: &[&str]) -> Diag {
    at_path(
        num_code(e),
        top,
        id,
        key,
        &e.to_string(),
        "write the number on the lattice, within range",
    )
}

/// The `write `x`` hint of the nearest visible name, or `otherwise`.
pub fn near_hint<'a>(
    written: &str,
    names: impl IntoIterator<Item = &'a str>,
    otherwise: &str,
) -> String {
    nearest(written, names).hint(otherwise)
}

/// `d` with `data.nearest` when a name close to `written` is visible (`diagnostics.md` 1.3).
pub fn with_nearest<'a>(d: Diag, written: &str, names: impl IntoIterator<Item = &'a str>) -> Diag {
    match nearest(written, names) {
        via_diag::nearest::Nearest::Found(n) => {
            d.with("nearest", via_diag::Value::text(n.to_string()))
        }
        via_diag::nearest::Nearest::None(_) => d,
    }
}

/// A number's double (`numbers.md` 3.3).
pub fn num(n: &Num) -> Result<f64, NumError> {
    via_num::parse_double(&n.norm)
}

/// A value as a double, if it is a number.
pub fn number(v: &Value) -> Option<Result<f64, NumError>> {
    match v {
        Value::Num(n) => Some(num(n)),
        _ => None,
    }
}

/// A value as a length in millimetres, if it is a number.
pub fn length(v: &Value) -> Option<Result<i64, NumError>> {
    match v {
        Value::Num(n) => Some(via_num::parse_length(&n.norm)),
        _ => None,
    }
}

/// A point `[x, y]` in millimetres.
pub fn point(v: &Value) -> Option<Result<(i64, i64), NumError>> {
    let Value::List(xs) = v else { return None };
    if xs.len() != 2 {
        return None;
    }
    let x = length(&xs[0].value)?;
    let y = length(&xs[1].value)?;
    Some(x.and_then(|x| y.map(|y| (x, y))))
}

/// A list of points in millimetres.
pub fn points(v: &Value) -> Option<Result<Vec<(i64, i64)>, NumError>> {
    let Value::List(xs) = v else { return None };
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match point(&x.value)? {
            Ok(p) => out.push(p),
            Err(e) => return Some(Err(e)),
        }
    }
    Some(Ok(out))
}

/// A list of strings.
pub fn strings(v: &Value) -> Option<Vec<String>> {
    let Value::List(xs) = v else { return None };
    xs.iter()
        .map(|i| match &i.value {
            Value::Str(s) => Some(s.text.clone()),
            _ => None,
        })
        .collect()
}

/// A value's canonical text, for statement digests and the change report: tables by key, lists
/// in order, numbers by their normalised text. Spans are not part of it.
pub fn canonical(v: &Value) -> String {
    let mut s = String::new();
    write_canonical(v, &mut s);
    s
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Table(t) => {
            out.push('{');
            for (i, (k, e)) in t.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!("{k:?}:"));
                write_canonical(&e.item.value, out);
            }
            out.push('}');
        }
        Value::List(xs) => {
            out.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(&x.value, out);
            }
            out.push(']');
        }
        Value::Str(s) => out.push_str(&format!("{:?}", s.text)),
        Value::Num(n) => out.push_str(&n.norm),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::DateTime => out.push_str("date"),
    }
}

/// A formula key's text: a string formula as written, or a number's text as a constant.
#[derive(Clone, Debug, PartialEq)]
pub enum FormulaText {
    Text(String),
    Number(f64),
}

/// Reads a key that takes a formula.
pub fn formula_text(v: &Value) -> Option<Result<FormulaText, NumError>> {
    match v {
        Value::Str(s) => Some(Ok(FormulaText::Text(s.text.clone()))),
        Value::Num(n) => Some(num(n).map(FormulaText::Number)),
        Value::Bool(b) => Some(Ok(FormulaText::Number(if *b { 1.0 } else { 0.0 }))),
        _ => None,
    }
}

/// A finite double in its shortest form; a non-finite one as words (it is never stored).
pub fn dbl(x: f64) -> String {
    via_num::write_double(x).unwrap_or_else(|_| "a non-finite number".to_string())
}
