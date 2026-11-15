//! The canonical JSON of the merged world, and its origins (`spec/documents.md` 8).

use crate::report::chain_entries;
use crate::schema::{self, Rule, Shape, Takes};
use crate::source::Sources;
use crate::tree::{Entry, Item, Table, Value};
use crate::world::World;
use std::collections::BTreeMap;

/// The layout of the printed JSON.
pub const FORMAT: &str = "via-merged-1";

enum J {
    Bool(bool),
    /// A number already written as JSON text.
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}

#[derive(Clone, Copy)]
enum Mode<'s> {
    Values,
    Origins(&'s Sources),
}

/// Strings: raw UTF-8; only `"`, `\` and U+0000 to U+001F are escaped.
fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(j: &J, out: &mut String) {
    match j {
        J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        J::Num(n) => out.push_str(n),
        J::Str(s) => string(s, out),
        J::Arr(xs) => {
            out.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, out);
            }
            out.push(']');
        }
        J::Obj(m) => {
            out.push('{');
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(k, out);
                out.push(':');
                write(v, out);
            }
            out.push('}');
        }
    }
}

/// A number by what its key takes (8, Numbers). Text that fails to convert, which the checks of
/// step 6 have already reported, is printed as its normalised text.
fn number(norm: &str, integer: bool, takes: Option<Takes>) -> String {
    let fallback = || norm.to_string();
    match takes {
        Some(Takes::Length | Takes::Point | Takes::Points | Takes::Ways | Takes::Lengths(_)) => {
            via_num::parse_length(norm).map_or_else(|_| fallback(), via_num::write_length)
        }
        Some(Takes::Integer) => fallback(),
        Some(Takes::Facing) => {
            via_num::parse_facing(norm).map_or_else(|_| fallback(), via_num::write_facing)
        }
        Some(Takes::WorldSeed) => norm
            .parse::<i64>()
            .ok()
            .and_then(|s| via_num::world_seed(s).ok())
            .map_or_else(fallback, |s| format!("\"{}\"", via_num::write_seed(s))),
        Some(Takes::Free) => fallback(),
        _ => {
            let _ = integer;
            via_num::parse_double(norm)
                .ok()
                .and_then(|d| via_num::write_double(d).ok())
                .unwrap_or_else(fallback)
        }
    }
}

fn element(takes: Option<Takes>) -> Option<Takes> {
    match takes {
        Some(Takes::Points | Takes::Ways) => Some(Takes::Point),
        Some(Takes::Point | Takes::Lengths(_)) => Some(Takes::Length),
        Some(Takes::Numbers(_) | Takes::NumberOrPair) => Some(Takes::Number),
        Some(Takes::Choices) => Some(Takes::Table(Shape::Choice)),
        Some(Takes::Free) => Some(Takes::Free),
        Some(Takes::Strings | Takes::Names) => Some(Takes::Str),
        _ => None,
    }
}

fn child(takes: Option<Takes>, key: &str) -> Option<Takes> {
    match takes {
        Some(Takes::Table(Shape::Let)) => Some(Takes::Formula),
        Some(Takes::Table(Shape::Feature)) => match schema::rule(Shape::Feature, key) {
            Some(Rule::Ok(t)) => Some(t),
            _ => None,
        },
        Some(Takes::Table(s)) => match schema::rule(s, key) {
            Some(Rule::Ok(t)) => Some(t),
            _ => None,
        },
        Some(Takes::NameOrSelect) => match schema::rule(Shape::Select, key) {
            Some(Rule::Ok(t)) => Some(t),
            _ => None,
        },
        Some(Takes::Ids(s)) => Some(Takes::Table(s)),
        Some(Takes::Free) => Some(Takes::Free),
        _ => None,
    }
}

fn origin(e: &Entry, sources: &Sources) -> J {
    let mut o = BTreeMap::new();
    let (line, column) = sources
        .get(e.key.src)
        .map_or((1, 1), |s| s.line_column(e.key.start));
    o.insert("file".into(), J::Str(sources.name(e.key.src).to_string()));
    o.insert("line".into(), J::Num(line.to_string()));
    o.insert("column".into(), J::Num(column.to_string()));
    let chain = chain_entries(sources, &e.chain)
        .into_iter()
        .map(|c| {
            let mut m = BTreeMap::new();
            m.insert("through".into(), J::Str(c.through.as_str().to_string()));
            m.insert("name".into(), J::Str(c.name));
            m.insert("file".into(), J::Str(c.file));
            m.insert("line".into(), J::Num(c.line.to_string()));
            if let Some(o) = c.owner {
                m.insert("owner".into(), J::Str(o));
            }
            J::Obj(m)
        })
        .collect();
    o.insert("chain".into(), J::Arr(chain));
    J::Obj(o)
}

fn table(t: &Table, takes: Option<Takes>, mode: Mode) -> BTreeMap<String, J> {
    t.iter()
        .map(|(k, e)| (k.clone(), entry(e, child(takes, k), mode)))
        .collect()
}

fn entry(e: &Entry, takes: Option<Takes>, mode: Mode) -> J {
    match (&e.item.value, mode) {
        (Value::Table(t), _) => J::Obj(table(t, takes, mode)),
        (_, Mode::Origins(sources)) => origin(e, sources),
        (_, Mode::Values) => value(&e.item, takes),
    }
}

fn value(item: &Item, takes: Option<Takes>) -> J {
    match &item.value {
        Value::Str(s) => J::Str(s.text.clone()),
        Value::Bool(b) => J::Bool(*b),
        Value::Num(n) => J::Num(number(&n.norm, n.integer, takes)),
        Value::DateTime => J::Str(String::new()),
        Value::List(xs) => {
            let el = element(takes);
            J::Arr(
                xs.iter()
                    .map(|x| match &x.value {
                        Value::Table(t) => J::Obj(table(t, el, Mode::Values)),
                        _ => value(x, el),
                    })
                    .collect(),
            )
        }
        Value::Table(t) => J::Obj(table(t, takes, Mode::Values)),
    }
}

fn render(world: &World, mode: Mode) -> String {
    let mut top: BTreeMap<String, J> = BTreeMap::new();
    top.insert("@format".into(), J::Str(FORMAT.into()));
    top.insert(
        "@changes".into(),
        J::Arr(world.changes.iter().map(|c| J::Str(c.clone())).collect()),
    );
    let root = &world.base.root;
    for (k, e) in root.iter() {
        if schema::WORLD_KEYS.contains(&k.as_str()) {
            let takes = match schema::world_key(k) {
                Some(Rule::Ok(t)) => Some(t),
                _ => None,
            };
            let mut j = entry(e, takes, mode);
            if k == "include" {
                if let J::Arr(xs) = &mut j {
                    xs.sort_by(|a, b| match (a, b) {
                        (J::Str(a), J::Str(b)) => a.as_bytes().cmp(b.as_bytes()),
                        _ => std::cmp::Ordering::Equal,
                    });
                }
            }
            top.insert(k.clone(), j);
            continue;
        }
        let Value::Table(t) = &e.item.value else {
            continue;
        };
        if k == "change" {
            continue;
        }
        let mut members = BTreeMap::new();
        for (id, ent) in t.iter() {
            let home = world
                .base
                .homes
                .get(&(k.clone(), id.clone()))
                .cloned()
                .unwrap_or_default();
            let j = match k.as_str() {
                "let" => {
                    let mut o = BTreeMap::new();
                    o.insert("@layer".into(), J::Str(home));
                    o.insert("value".into(), entry(ent, Some(Takes::Formula), mode));
                    J::Obj(o)
                }
                "kind" => {
                    let mut o = match entry(ent, Some(Takes::Table(Shape::Kind)), mode) {
                        J::Obj(o) => o,
                        _ => BTreeMap::new(),
                    };
                    o.insert("@library".into(), J::Str(home));
                    J::Obj(o)
                }
                "layer" => entry(ent, Some(Takes::Table(Shape::Layer)), mode),
                _ => {
                    let shape = schema::entity_shape(k);
                    let mut o = match entry(ent, shape.map(Takes::Table), mode) {
                        J::Obj(o) => o,
                        _ => BTreeMap::new(),
                    };
                    o.insert("@layer".into(), J::Str(home));
                    J::Obj(o)
                }
            };
            members.insert(id.clone(), j);
        }
        top.insert(k.clone(), J::Obj(members));
    }
    // `world` always shows its rank (8): written or the default.
    if let Mode::Values = mode {
        let layers = top
            .entry("layer".into())
            .or_insert_with(|| J::Obj(BTreeMap::new()));
        if let J::Obj(m) = layers {
            let w = m
                .entry("world".into())
                .or_insert_with(|| J::Obj(BTreeMap::new()));
            if let J::Obj(w) = w {
                w.entry("rank".into()).or_insert_with(|| J::Num("0".into()));
            }
        }
    }
    let mut out = String::new();
    write(&J::Obj(top), &mut out);
    out.push('\n');
    out
}

/// `via merge`'s output.
pub(crate) fn canonical(world: &World) -> String {
    render(world, Mode::Values)
}

/// `via merge --origins`' output.
pub(crate) fn origins(world: &World) -> String {
    render(world, Mode::Origins(&world.sources))
}
