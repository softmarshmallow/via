//! The facts file (`assets.md` 4): what `via pack accept` writes, byte for byte (4.1), how it is
//! read back, and what a build or `via pack check` raises when the facts it measured and resolved
//! differ from it (4.2, 8.1).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use via_diag::{codes, Diag, Site, Subject, Value};

use crate::pack::{Asset, Pack};
use crate::text;

/// The first line of a facts file (4.1).
pub const HEADER: &str = "# Written by via pack accept. Do not edit.";

/// The keys of an entry, in the order 4.1 writes them.
pub const KEYS: [&str; 18] = [
    "bounds",
    "origin",
    "footprint",
    "triangles",
    "detail_triangles",
    "front",
    "up",
    "pivot",
    "solidity",
    "collision",
    "sink",
    "max_relief",
    "colour",
    "taxon",
    "tags",
    "mounts",
    "params",
    "files",
];

/// The facts that decide where and how an instance stands: a difference is `asset.revised` (4.2).
pub const PLACEMENT: [&str; 10] = [
    "bounds",
    "origin",
    "footprint",
    "front",
    "up",
    "pivot",
    "solidity",
    "sink",
    "max_relief",
    "mounts",
];

/// Filing: a difference alone is `asset.refiled`.
pub const FILING: [&str; 2] = ["taxon", "tags"];

/// The facts file of a pack: the pack file's path with `.toml` replaced by `.facts.toml` (4).
pub fn path_of(pack: &Path) -> PathBuf {
    let name = pack
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pack.toml");
    let stem = name.strip_suffix(".toml").unwrap_or(name);
    pack.with_file_name(format!("{stem}.facts.toml"))
}

/// The facts file as read.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FactsFile {
    /// No facts file.
    #[default]
    Absent,
    /// Present but not in 4.1's form; `pack.bad_facts` was raised.
    Bad,
    /// Each asset's entry: key to the value's text, as written after `key = `.
    Present(BTreeMap<String, BTreeMap<String, String>>),
}

/// A TOML basic string.
pub fn basic(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(", "))
}

fn len(mm: i64) -> String {
    via_num::write_length(mm)
}

/// A number of metres held on the lattice (2.4: every length is), in exact metre form.
fn metres(x: f64) -> String {
    via_num::round_to_lattice(x * 1000.0).map_or_else(|_| text::dbl(x), len)
}

/// The canonical JSON (`documents.md` 8) of a TOML value: members in bytewise order, numbers in
/// `numbers.md`'s form, strings escaping only `"`, `\` and control characters.
pub fn canonical_json(v: &toml::Value) -> String {
    fn s(t: &str) -> String {
        let mut out = String::from("\"");
        for c in t.chars() {
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
    match v {
        toml::Value::String(t) => s(t),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => text::dbl(*f),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => s(&d.to_string()),
        toml::Value::Array(a) => format!(
            "[{}]",
            a.iter().map(canonical_json).collect::<Vec<_>>().join(",")
        ),
        toml::Value::Table(t) => {
            let mut keys: Vec<&String> = t.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            format!(
                "{{{}}}",
                keys.iter()
                    .map(|k| format!("{}:{}", s(k), canonical_json(&t[k.as_str()])))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

/// An asset's entry: each fact's text, in 4.1's order, absent facts left out.
pub fn entry(a: &Asset) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    out.push(("bounds", list(a.bounds.iter().map(|v| len(*v)))));
    out.push(("origin", list(a.origin.iter().map(|v| len(*v)))));
    out.push((
        "footprint",
        list(
            a.footprint
                .iter()
                .map(|(x, y)| format!("[{}, {}]", len(*x), len(*y))),
        ),
    ));
    out.push(("triangles", a.triangles.to_string()));
    if let Some(m) = &a.model {
        if !m.detail.is_empty() {
            out.push((
                "detail_triangles",
                list(m.detail_triangles.iter().map(u64::to_string)),
            ));
        }
        out.push(("front", basic(&m.front)));
        out.push(("up", basic(&m.up)));
        out.push(("pivot", basic(&m.pivot)));
    }
    out.push(("solidity", basic(if a.solid { "solid" } else { "none" })));
    out.push(("collision", basic(&a.collision)));
    out.push(("sink", metres(a.sink)));
    if let Some(r) = a.max_relief {
        out.push(("max_relief", metres(r)));
    }
    out.push(("colour", list(a.colour.iter().map(|c| text::dbl(*c)))));
    if let Some(t) = &a.taxon {
        out.push(("taxon", basic(t)));
    }
    out.push(("tags", list(a.tags.iter().map(|t| basic(t)))));
    if !a.mounts.is_empty() {
        let items: Vec<String> = a
            .mounts
            .iter()
            .map(|(n, ((x, y), face))| {
                format!(
                    "{n} = {{ at = [{}, {}], face = {} }}",
                    len(*x),
                    len(*y),
                    via_num::write_facing(*face)
                )
            })
            .collect();
        out.push(("mounts", format!("{{ {} }}", items.join(", "))));
    }
    if let Some(p) = &a.params {
        let json = canonical_json(&toml::Value::Table(p.clone()));
        out.push((
            "params",
            basic(&via_graph::Digest::of(json.as_bytes()).hex()),
        ));
    }
    if let Some(m) = &a.model {
        let items: Vec<String> = m
            .files
            .iter()
            .map(|(p, d)| format!("{} = {}", basic(p), basic(d)))
            .collect();
        out.push(("files", format!("{{ {} }}", items.join(", "))));
    }
    out
}

/// The facts file of a pack, byte for byte (4.1): every model asset, sorted by id bytewise.
pub fn write(pack: &Pack) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    let mut ids: Vec<&String> = pack
        .assets
        .iter()
        .filter(|(_, a)| a.model.is_some())
        .map(|(id, _)| id)
        .collect();
    ids.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for id in ids {
        out.push('\n');
        out.push_str(&format!("[asset.{id}]\n"));
        for (k, v) in entry(&pack.assets[id.as_str()]) {
            out.push_str(&format!("{k} = {v}\n"));
        }
    }
    out
}

/// Reads a facts file: its entries, or why it is not in 4.1's form (with a 1-based line).
pub fn parse(text: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, (String, u32)> {
    if let Err(e) = text.parse::<toml::Table>() {
        let line = e.span().map_or(1, |s| {
            text[..s.start.min(text.len())].matches('\n').count() + 1
        });
        let first = e.message().lines().next().unwrap_or("").trim().to_string();
        return Err((
            format!("the facts file is not TOML: {first}"),
            u32::try_from(line).unwrap_or(u32::MAX),
        ));
    }
    if text.contains('\r') {
        return Err(("the facts file has a carriage return".into(), 1));
    }
    if !text.ends_with('\n') {
        return Err(("the facts file does not end with a newline".into(), 1));
    }
    let lines: Vec<&str> = text[..text.len() - 1].split('\n').collect();
    if lines.first() != Some(&HEADER) {
        return Err(("the facts file's first line is not its header".into(), 1));
    }
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current: Option<(String, usize)> = None;
    let mut expect_header = true;
    for (i, l) in lines.iter().enumerate().skip(1) {
        let at = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if l.is_empty() {
            if !expect_header && current.is_none() {
                return Err(("an unexpected blank line".into(), at));
            }
            expect_header = true;
            continue;
        }
        if let Some(id) = l.strip_prefix("[asset.").and_then(|r| r.strip_suffix(']')) {
            if !expect_header || !via_doc::is_id(id) || out.contains_key(id) {
                return Err((format!("the table `{l}` is not where 4.1 puts one"), at));
            }
            if out
                .keys()
                .next_back()
                .is_some_and(|last| last.as_str() >= id)
            {
                return Err(("the tables are not sorted by id".into(), at));
            }
            out.insert(id.to_string(), BTreeMap::new());
            current = Some((id.to_string(), 0));
            expect_header = false;
            continue;
        }
        let Some((id, next)) = current.as_mut() else {
            return Err(("a line outside any `[asset.<id>]` table".into(), at));
        };
        if expect_header {
            return Err(("a key after a blank line".into(), at));
        }
        let Some((k, v)) = l.split_once(" = ") else {
            return Err((format!("the line `{l}` is not `key = value`"), at));
        };
        let Some(pos) = KEYS.iter().position(|x| *x == k) else {
            return Err((format!("`{k}` is not a key of the facts file"), at));
        };
        if pos < *next {
            return Err((format!("`{k}` is out of 4.1's order"), at));
        }
        *next = pos + 1;
        if let Some(e) = out.get_mut(id.as_str()) {
            e.insert(k.to_string(), v.to_string());
        }
    }
    Ok(out)
}

/// How an asset's facts differ from its accepted entry.
#[derive(Debug, Default)]
pub struct Diff {
    /// (fact, old, new) for each fact that differs, in 4.1's order.
    pub facts: Vec<(String, String, String)>,
}

impl Diff {
    pub fn of(old: &BTreeMap<String, String>, now: &[(&'static str, String)]) -> Diff {
        let now: BTreeMap<&str, &String> = now.iter().map(|(k, v)| (*k, v)).collect();
        let mut facts = Vec::new();
        for k in KEYS {
            let (a, b) = (old.get(k), now.get(k).copied());
            if a != b {
                facts.push((
                    k.to_string(),
                    a.cloned().unwrap_or_else(|| "absent".into()),
                    b.cloned().unwrap_or_else(|| "absent".into()),
                ));
            }
        }
        Diff { facts }
    }

    fn of_kind(&self, keys: &[&str]) -> Vec<&(String, String, String)> {
        self.facts
            .iter()
            .filter(|(k, _, _)| keys.contains(&k.as_str()))
            .collect()
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The asset's header in the pack text, or the world when the text holds none.
fn header(pack: &Pack, id: &str) -> (Subject, Site) {
    pack.declaration("asset", id).unwrap_or_else(|| {
        (
            Subject::parse(&format!("asset.{id}")).unwrap_or(Subject::World),
            Site::World,
        )
    })
}

fn pack_site(pack: &Pack) -> (Subject, Site) {
    (
        Subject::parse("pack").unwrap_or(Subject::World),
        Site::Text {
            file: pack.display.clone(),
            line: 1,
            column: 1,
            bytes: [0, 0],
        },
    )
}

/// The comparison of 4.2 for model assets `ids`, with the instances of each in this build (empty
/// for `via pack check`, 8.1). Raises nothing when the facts file is absent or bad: the caller
/// raised `pack.unaccepted` or `pack.bad_facts` once.
pub fn compare<'a>(
    pack: &Pack,
    ids: impl IntoIterator<Item = &'a str>,
    instances: &BTreeMap<String, Vec<String>>,
) -> Vec<Diag> {
    let FactsFile::Present(accepted) = &pack.facts else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for id in ids {
        let Some(a) = pack.assets.get(id) else {
            continue;
        };
        if a.model.as_ref().is_none_or(|m| !m.measured) {
            continue;
        }
        let (subject, site) = header(pack, id);
        let Some(old) = accepted.get(id) else {
            out.push(text::diag(
                codes::asset::UNACCEPTED,
                subject,
                site,
                &format!("the asset `{id}` is not in {}", pack.facts_display),
                &format!("run `via pack accept {}`", pack.command_path()),
            ));
            continue;
        };
        let diff = Diff::of(old, &entry(a));
        if diff.facts.is_empty() {
            continue;
        }
        let placement = diff.of_kind(&PLACEMENT);
        let filing = diff.of_kind(&FILING);
        let rest: Vec<&(String, String, String)> = diff
            .facts
            .iter()
            .filter(|(k, _, _)| !PLACEMENT.contains(&k.as_str()) && !FILING.contains(&k.as_str()))
            .collect();
        let names = |v: &[&(String, String, String)]| -> Vec<String> {
            v.iter().map(|(k, _, _)| k.clone()).collect()
        };
        let with_facts = |d: Diag, v: &[&(String, String, String)]| {
            d.with("facts", Value::texts(names(v)))
                .with("old", Value::texts(v.iter().map(|(_, o, _)| o.clone())))
                .with("new", Value::texts(v.iter().map(|(_, _, n)| n.clone())))
        };
        if !placement.is_empty() {
            let inst = instances.get(id).cloned().unwrap_or_default();
            let shown: Vec<String> = placement
                .iter()
                .map(|(k, o, n)| format!("{k} {o} -> {n}"))
                .collect();
            let d = text::diag(
                codes::asset::REVISED,
                subject,
                site,
                &format!(
                    "{} of `{id}` differ from {}: {}; {}{}",
                    plural(placement.len(), "fact", "facts"),
                    pack.facts_display,
                    shown.join("; "),
                    plural(inst.len(), "instance", "instances"),
                    if inst.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", inst.join(", "))
                    }
                ),
                &format!(
                    "review the change report, then run `via pack accept {}`",
                    pack.command_path()
                ),
            );
            out.push(
                with_facts(d, &placement)
                    .with("count", inst.len() as i64)
                    .with("instances", Value::texts(inst)),
            );
            continue;
        }
        if !filing.is_empty() {
            let d = text::diag(
                codes::asset::REFILED,
                subject.clone(),
                site.clone(),
                &format!(
                    "`{id}` is filed differently from {}: {}",
                    pack.facts_display,
                    names(&filing).join(", ")
                ),
                "review what selections now pick, then accept",
            );
            out.push(with_facts(d, &filing));
        }
        if !rest.is_empty() {
            let tri = |v: Option<&String>| v.and_then(|t| t.parse::<i64>().ok()).unwrap_or(0);
            let (old_t, new_t) = (
                tri(old.get("triangles")),
                i64::try_from(a.triangles).unwrap_or(i64::MAX),
            );
            let d = text::diag(
                codes::asset::REMODELLED,
                subject,
                site,
                &format!(
                    "`{id}` was remodelled ({}); triangles {old_t} -> {new_t}; nothing moves",
                    names(&rest).join(", ")
                ),
                "none needed; accept to record it",
            )
            .with("old_triangles", old_t)
            .with("new_triangles", new_t);
            out.push(with_facts(d, &rest));
        }
    }
    out
}

/// `pack.unaccepted`, when the pack has a model asset and no facts file (4.2).
pub fn unaccepted(pack: &Pack) -> Option<Diag> {
    if pack.facts != FactsFile::Absent || !pack.assets.values().any(|a| a.model.is_some()) {
        return None;
    }
    let (subject, site) = pack_site(pack);
    Some(text::diag(
        codes::pack::UNACCEPTED,
        subject,
        site,
        &format!(
            "the pack has model assets and no facts file ({}); revisions cannot be reported",
            pack.facts_display
        ),
        &format!("run `via pack accept {}`", pack.command_path()),
    ))
}

/// `pack.stale_facts`: entries for ids the pack no longer has as model assets (8.1).
pub fn stale(pack: &Pack) -> Option<Diag> {
    let FactsFile::Present(accepted) = &pack.facts else {
        return None;
    };
    let gone: Vec<String> = accepted
        .keys()
        .filter(|id| pack.assets.get(*id).is_none_or(|a| a.model.is_none()))
        .cloned()
        .collect();
    if gone.is_empty() {
        return None;
    }
    let (subject, site) = pack_site(pack);
    Some(
        text::diag(
            codes::pack::STALE_FACTS,
            subject,
            site,
            &format!(
                "{} holds {} the pack no longer has: {}",
                pack.facts_display,
                plural(gone.len(), "id", "ids"),
                gone.join(", ")
            ),
            &format!("run `via pack accept {}` to drop them", pack.command_path()),
        )
        .with("ids", Value::texts(gone)),
    )
}

/// What `via pack accept` did.
pub struct Accepted {
    /// The ids whose entries changed, added or dropped, sorted.
    pub changed: Vec<String>,
}

/// `via pack accept` (4.3): measures and resolves every model asset, and writes the facts file
/// atomically, or refuses with `pack.accept_refused` and writes nothing.
///
/// `errors` holds the code of every error the pack has: its pack-wide faults and every asset's own.
pub fn accept(pack: &Pack, errors: &BTreeSet<String>) -> Result<Accepted, Diag> {
    if !errors.is_empty() {
        let (subject, site) = pack_site(pack);
        return Err(text::diag(
            codes::pack::ACCEPT_REFUSED,
            subject,
            site,
            &format!(
                "the pack has errors ({}); nothing was written",
                errors.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
            "fix the listed errors, then accept again",
        )
        .with("codes", Value::texts(errors.iter().cloned())));
    }
    let bytes = write(pack);
    let new = parse(&bytes).unwrap_or_default();
    let old = match &pack.facts {
        FactsFile::Present(m) => m.clone(),
        _ => BTreeMap::new(),
    };
    let ids: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    let changed = ids
        .into_iter()
        .filter(|id| old.get(*id) != new.get(*id))
        .cloned()
        .collect();
    let (subject, site) = pack_site(pack);
    let fail = |e: std::io::Error| {
        text::diag(
            codes::pack::ACCEPT_REFUSED,
            subject.clone(),
            site.clone(),
            &format!("the facts file cannot be written: {e}"),
            "check that the pack's folder is writable",
        )
    };
    let target = &pack.facts_path;
    let tmp = target.with_file_name(format!(
        ".{}.tmp",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("facts")
    ));
    std::fs::write(&tmp, bytes.as_bytes()).map_err(fail)?;
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })?;
    Ok(Accepted { changed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_it_writes() {
        let t = format!(
            "{HEADER}\n\n[asset.a]\nbounds = [0, 0, 0, 1, 1, 1]\ntriangles = 12\nfiles = {{ \"a.glb\" = \"00\" }}\n\n[asset.b]\ntriangles = 2\n"
        );
        let m = parse(&t).unwrap();
        assert_eq!(m["a"]["triangles"], "12");
        assert_eq!(m["b"]["triangles"], "2");
        // Out of order, unsorted, unknown key.
        assert!(parse(&format!(
            "{HEADER}\n\n[asset.a]\ntriangles = 1\nbounds = []\n"
        ))
        .is_err());
        assert!(parse(&format!("{HEADER}\n\n[asset.b]\n\n[asset.a]\n")).is_err());
        assert!(parse(&format!("{HEADER}\n\n[asset.a]\nsize = 1\n")).is_err());
        assert!(parse("[asset.a\n").is_err());
    }

    #[test]
    fn basic_strings_escape() {
        assert_eq!(basic("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000A\"");
    }
}
