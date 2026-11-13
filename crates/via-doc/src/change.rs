//! Changes (`spec/documents.md` 6): what a body may hold, its write set, and how it merges onto
//! the base.

use crate::check::not_yet;
use crate::ids;
use crate::report::{At, Fault};
use crate::schema::{self, Rule, PATH_ONLY, PLACEMENT, VALUE_TABLES};
use crate::source::Span;
use crate::tree::{Entry, Form, Link, Table, Value};
use std::collections::BTreeMap;
use via_diag::nearest::{nearest, Nearest};
use via_diag::{codes, KeyPart, KeyPath, Subject, Through};

/// The base and, after changes, the merged world: one root table, and the home of every entity
/// (the layer that declares it; for a kind, its library).
#[derive(Clone, Debug)]
pub(crate) struct Base {
    pub root: Table,
    pub homes: BTreeMap<(String, String), String>,
}

/// One change, inline or from a file.
#[derive(Clone, Debug)]
pub(crate) struct Change {
    pub name: String,
    /// The body: shaped like a world.
    pub body: Table,
    /// Where an entity it adds is declared (6.1): the declaring layer, or `world` for a file.
    pub home: String,
    /// The span that creates it: its `[change.<name>]` header, or the file's start.
    pub header: Span,
}

/// A path a change writes or removes, with the span that writes it.
pub(crate) type Write = (Vec<String>, Span);

/// An entity a change removed, for `doc.dangling`.
#[derive(Clone, Debug)]
pub(crate) struct Removal {
    pub top: String,
    pub id: String,
    pub change: String,
    pub span: Span,
}

/// What applying one change did.
#[derive(Clone, Debug, Default)]
pub(crate) struct Applied {
    pub writes: Vec<Write>,
    pub removed: Vec<Removal>,
    pub faults: Vec<Fault>,
    /// Entities this change added, so its own additions are not each other's near names.
    pub added: Vec<(String, String)>,
}

fn change_subject(name: &str, path: &[&str]) -> Subject {
    let parts: Vec<KeyPart> = path
        .iter()
        .map(|p| KeyPart::Key((*p).to_string()))
        .collect();
    match KeyPath::from_parts(parts) {
        Some(kp) => Subject::entity("change", name).with_keys(&kp),
        None => Subject::entity("change", name),
    }
}

fn entity_subject(top: &str, id: &str, path: &[String]) -> Subject {
    let parts: Vec<KeyPart> = path.iter().map(|p| KeyPart::Key(p.clone())).collect();
    match KeyPath::from_parts(parts) {
        Some(kp) => Subject::entity(top, id).with_keys(&kp),
        None => Subject::entity(top, id),
    }
}

/// The words a change body may hold at its root, for nearest-word hints.
const BODY_WORDS: [&str; 12] = [
    "clear",
    "extent",
    "feature",
    "field",
    "instance",
    "kind",
    "let",
    "pack",
    "pin",
    "resolution",
    "scatter",
    "seed",
];

/// Step 4's checks of one body that need no base: what it may not hold, and ids.
pub(crate) fn check_body(ch: &Change) -> Vec<Fault> {
    let mut out = Vec::new();
    if ch.body.is_empty() {
        out.push(Fault::new(
            codes::doc::CHANGE_EMPTY,
            change_subject(&ch.name, &[]),
            At::Span(ch.header),
            format!("the change `{}` writes nothing", ch.name),
            "write the difference it stands for, or delete the change",
        ));
    }
    for (k, e) in ch.body.iter() {
        let sub = change_subject(&ch.name, &[k]);
        let at = At::Span(e.key);
        match k.as_str() {
            "pack" | "seed" | "extent" | "resolution" => {}
            "include" | "name" => out.push(
                Fault::new(
                    codes::doc::CHANGE_FIXED,
                    sub,
                    at,
                    format!("the change `{}` writes `{k}`, which no change may write", ch.name),
                    if k == "include" {
                        "edit `include` in the world file: the library list is settled before any change applies"
                    } else {
                        "edit `name` in the world file: it names the world the changes differ from"
                    },
                )
                .chain(&e.chain),
            ),
            "chunk" | "budget" => out.push(not_yet(sub, at, k, 6).chain(&e.chain)),
            "layer" => out.push(not_yet(sub, at, "[layer]", 4).chain(&e.chain)),
            "change" => out.push(
                Fault::new(
                    codes::doc::CHANGE_NESTED,
                    sub,
                    at,
                    format!("the change `{}` holds a `[change]` table; a change has no changes", ch.name),
                    "write the inner change as its own `[change.<name>]` in the world or a layer",
                )
                .chain(&e.chain),
            ),
            "asset" | "material" | "root" => out.push(
                Fault::new(
                    codes::doc::UNKNOWN_KEY,
                    sub,
                    at,
                    format!("a change cannot write the pack's `{k}`"),
                    "a change reaches a pack only by swapping it: write `pack`",
                )
                .chain(&e.chain),
            ),
            "remove" | "unset" => out.push(above_entity(&ch.name, k, e)),
            "kind" => out.extend(ids_of(ch, k, e)),
            _ => match schema::top_table(k) {
                Some(Rule::Ok(_)) => out.extend(ids_of(ch, k, e)),
                Some(Rule::NotYet(m)) => out.push(not_yet(sub, at, k, m).chain(&e.chain)),
                Some(Rule::Fixed(h)) => out.push(
                    Fault::new(
                        codes::doc::UNKNOWN_KEY,
                        sub,
                        at,
                        format!("`{k}` is not a word of a change body"),
                        h,
                    )
                    .chain(&e.chain),
                ),
                None => {
                    let mut f = Fault::new(
                        codes::doc::UNKNOWN_KEY,
                        sub,
                        at,
                        format!("`{k}` is not a word of a change body"),
                        String::new(),
                    )
                    .chain(&e.chain);
                    match nearest(k, BODY_WORDS) {
                        Nearest::Found(n) => {
                            f.hint = format!("write `{n}`");
                            f = f.with("nearest", n);
                        }
                        other => f.hint = other.hint("remove it"),
                    }
                    out.push(f);
                }
            },
        }
    }
    out
}

fn above_entity(change: &str, k: &str, e: &Entry) -> Fault {
    Fault::new(
        codes::doc::REMOVE_ABOVE_ENTITY,
        change_subject(change, &[k]),
        At::Span(e.key),
        format!("`{k}` is written above any entity; removal acts on an entity or inside one"),
        "write it inside the entity's table, such as `[change.<c>.feature.<id>] remove = true`",
    )
    .chain(&e.chain)
}

fn ids_of(ch: &Change, top: &str, e: &Entry) -> Vec<Fault> {
    let mut out = Vec::new();
    let Value::Table(t) = &e.item.value else {
        out.push(
            Fault::new(
                codes::doc::TYPE,
                change_subject(&ch.name, &[top]),
                At::Span(e.key),
                format!(
                    "`{top}` in a change is a table of entities, and {} is written",
                    e.item.value.form_word()
                ),
                format!("write `[change.{}.{top}.<id>]`", ch.name),
            )
            .chain(&e.chain),
        );
        return out;
    };
    for (id, ee) in t.iter() {
        if id == "remove" || id == "unset" {
            if !(top == "let" && id == "unset") {
                out.push(above_entity(&ch.name, id, ee));
            }
            continue;
        }
        if top == "let" {
            continue;
        }
        let sub = Subject::entity(top, id);
        if let Some(f) = ids::check(id, sub.clone(), At::Span(ee.key), "id") {
            out.push(f.chain(&ee.chain));
        } else if id == "world" {
            out.push(reserved(sub, ee));
        }
    }
    out
}

pub(crate) fn reserved(sub: Subject, e: &Entry) -> Fault {
    Fault::new(
        codes::identity::RESERVED,
        sub,
        At::Span(e.key),
        "the id `world` is reserved: it names the world file's own layer",
        "choose another id",
    )
    .chain(&e.chain)
}

fn link(ch: &Change, span: Span) -> Link {
    Link {
        through: Through::Change,
        name: ch.name.clone(),
        span,
        owner: None,
    }
}

fn set_chain_deep(e: &mut Entry, chain: &[Link]) {
    e.chain = chain.to_vec();
    if let Value::Table(t) = &mut e.item.value {
        for sub in t.entries.values_mut() {
            set_chain_deep(sub, chain);
        }
    }
}

fn written_entry(e: &Entry, prior: &[Link], ch: &Change) -> Entry {
    let mut chain = prior.to_vec();
    chain.extend(e.chain.iter().cloned());
    chain.push(link(ch, e.key));
    let mut out = e.clone();
    set_chain_deep(&mut out, &chain);
    out
}

fn path_of(prefix: &[String], k: &str) -> Vec<String> {
    let mut p = prefix.to_vec();
    p.push(k.to_string());
    p
}

fn is_remove(t: &Table) -> bool {
    matches!(t.value("remove"), Some(Value::Bool(true)))
}

/// Applies one change to the base (6.1, 6.2). Faults are those of step 4 that need the base.
pub(crate) fn apply(base: &mut Base, ch: &Change) -> Applied {
    let mut a = Applied::default();
    for (k, e) in ch.body.iter() {
        match k.as_str() {
            "pack" | "seed" | "extent" | "resolution" => {
                let prior = base
                    .root
                    .get(k)
                    .map(|old| old.chain.clone())
                    .unwrap_or_default();
                base.root.insert(k.clone(), written_entry(e, &prior, ch));
                a.writes.push((vec![k.clone()], e.key));
            }
            "let" => {
                if let Value::Table(t) = &e.item.value {
                    apply_let(base, ch, t, &mut a);
                }
            }
            _ => {
                let top_ok = k == "kind" || matches!(schema::top_table(k), Some(Rule::Ok(_)));
                if !top_ok {
                    continue;
                }
                if let Value::Table(t) = &e.item.value {
                    for (id, ee) in t.iter() {
                        if id == "remove" || id == "unset" {
                            continue;
                        }
                        entity(base, ch, k, id, ee, &mut a);
                    }
                }
            }
        }
    }
    a
}

fn top_table_mut<'b>(base: &'b mut Base, top: &str, span: Span) -> Option<&'b mut Table> {
    if base.root.get(top).is_none() {
        base.root.insert(
            top.to_string(),
            Entry {
                key: span,
                item: crate::tree::Item {
                    span,
                    value: Value::Table(Table::new(span, Form::Header)),
                },
                chain: Vec::new(),
            },
        );
    }
    base.root.table_mut(top)
}

fn apply_let(base: &mut Base, ch: &Change, t: &Table, a: &mut Applied) {
    if let Some(u) = t.get("unset") {
        for (name, span) in strings(u) {
            let present = base
                .root
                .table("let")
                .is_some_and(|l| l.get(&name).is_some());
            if t.get(&name).is_some() {
                a.faults.push(with_keys(&ch.name, "let", &name, span, u));
            } else if !present {
                a.faults.push(missing(&ch.name, "let", &name, &[], span, u));
            } else if let Some(l) = base.root.table_mut("let") {
                l.remove(&name);
                base.homes.remove(&("let".to_string(), name.clone()));
                a.writes.push((vec!["let".into(), name.clone()], span));
            }
        }
    }
    for (name, e) in t.iter() {
        if name == "unset" {
            continue;
        }
        let Some(l) = top_table_mut(base, "let", e.key) else {
            continue;
        };
        let prior = l.get(name).map(|old| old.chain.clone());
        let added = prior.is_none();
        l.insert(
            name.clone(),
            written_entry(e, &prior.unwrap_or_default(), ch),
        );
        if added {
            base.homes
                .insert(("let".to_string(), name.clone()), ch.home.clone());
        }
        a.writes.push((vec!["let".into(), name.clone()], e.key));
    }
}

fn strings(e: &Entry) -> Vec<(String, Span)> {
    match &e.item.value {
        Value::List(items) => items
            .iter()
            .filter_map(|it| match &it.value {
                Value::Str(s) => Some((s.text.clone(), it.span)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn with_keys(change: &str, top: &str, key: &str, span: Span, e: &Entry) -> Fault {
    Fault::new(
        codes::doc::REMOVE_WITH_KEYS,
        Subject::entity(top, key),
        At::Span(span),
        format!("the change `{change}` both writes and removes `{key}`"),
        "keep one: write the key, or remove it",
    )
    .chain(&e.chain)
}

fn missing(change: &str, top: &str, id: &str, path: &[String], span: Span, e: &Entry) -> Fault {
    let sub = entity_subject(top, id, path);
    Fault::new(
        codes::doc::REMOVE_MISSING,
        sub.clone(),
        At::Span(span),
        format!("the change `{change}` removes `{sub}`, which the base does not hold"),
        "delete the stale `remove` or `unset`, or correct the name",
    )
    .chain(&e.chain)
}

fn entity(base: &mut Base, ch: &Change, top: &str, id: &str, ee: &Entry, a: &mut Applied) {
    let path = vec![top.to_string(), id.to_string()];
    let Value::Table(body) = &ee.item.value else {
        a.faults.push(
            Fault::new(
                codes::doc::TYPE,
                Subject::entity(top, id),
                At::Span(ee.key),
                format!(
                    "`{top}.{id}` is an entity, a table, and {} is written",
                    ee.item.value.form_word()
                ),
                format!("write `[change.{}.{top}.{id}]` with its keys", ch.name),
            )
            .chain(&ee.chain),
        );
        return;
    };
    let exists = base.root.table(top).is_some_and(|t| t.get(id).is_some());
    if exists {
        if let Some(r) = body.get("remove") {
            if body.len() > 1 {
                a.faults.push(
                    Fault::new(
                        codes::doc::REMOVE_WITH_KEYS,
                        Subject::entity(top, id),
                        At::Span(r.key),
                        format!("`remove = true` on `{top}.{id}` sits beside other keys"),
                        "keep `remove = true` alone, or drop it and write the keys",
                    )
                    .chain(&r.chain),
                );
                return;
            }
            if is_remove(body) {
                if let Some(t) = base.root.table_mut(top) {
                    t.remove(id);
                }
                base.homes.remove(&(top.to_string(), id.to_string()));
                a.removed.push(Removal {
                    top: top.to_string(),
                    id: id.to_string(),
                    change: ch.name.clone(),
                    span: r.key,
                });
                a.writes.push((path, r.key));
            }
            return;
        }
        let Some(target) = base.root.table_mut(top).and_then(|t| t.get_mut(id)) else {
            return;
        };
        let prior = target.chain.clone();
        if let Value::Table(tt) = &mut target.item.value {
            merge(tt, body, &path, &prior, ch, a, (top, id));
        }
        return;
    }
    if let Some(r) = body.get("remove") {
        a.faults.push(missing(&ch.name, top, id, &[], r.key, r));
        return;
    }
    if top == "kind" {
        a.faults.push(
            Fault::new(
                codes::doc::KIND_OUTSIDE_LIBRARY,
                Subject::entity(top, id),
                At::Span(ee.key),
                format!(
                    "the change `{}` adds the kind `{id}`; a kind is declared in a library",
                    ch.name
                ),
                "declare the kind in a library's `kinds.toml` and include it",
            )
            .chain(&ee.chain),
        );
        return;
    }
    // A new entity: anything it would remove names nothing in the base.
    collect_missing(body, &path[2..], ch, top, id, a);
    if let Some(existing) = base.root.table(top) {
        let before = existing
            .keys()
            .filter(|k| !a.added.iter().any(|(t, i)| t == top && i == k));
        if let Nearest::Found(n) = nearest(id, before) {
            a.faults.push(
                Fault::new(
                    codes::doc::CHANGE_ADDS_NEAR,
                    Subject::entity(top, id),
                    At::Span(ee.key),
                    format!(
                        "the change `{}` adds `{top}.{id}`, and `{top}.{n}` exists",
                        ch.name
                    ),
                    format!(
                        "write `{n}` to change the existing one, or keep `{id}` to add a new one"
                    ),
                )
                .with("nearest", n)
                .chain(&ee.chain),
            );
        }
    }
    if let Some(t) = top_table_mut(base, top, ee.key) {
        t.insert(id.to_string(), written_entry(ee, &[], ch));
    }
    base.homes
        .insert((top.to_string(), id.to_string()), ch.home.clone());
    a.added.push((top.to_string(), id.to_string()));
    a.writes.push((path, ee.key));
}

fn collect_missing(
    body: &Table,
    path: &[String],
    ch: &Change,
    top: &str,
    id: &str,
    a: &mut Applied,
) {
    for (k, e) in body.iter() {
        if k == "remove" || k == "unset" {
            a.faults.push(missing(&ch.name, top, id, path, e.key, e));
        } else if let Value::Table(t) = &e.item.value {
            if !VALUE_TABLES.contains(&k.as_str()) {
                collect_missing(t, &path_of(path, k), ch, top, id, a);
            }
        }
    }
}

/// Merges a body onto a table key by key (6.2), with `unset` and the placement rule (6.1).
fn merge(
    target: &mut Table,
    body: &Table,
    path: &[String],
    prior: &[Link],
    ch: &Change,
    a: &mut Applied,
    ent: (&str, &str),
) {
    let (top, id) = ent;
    let below = |p: &[String]| p[2..].to_vec();
    if let Some(u) = body.get("unset") {
        for (k, span) in strings(u) {
            let kp = path_of(path, &k);
            if body.get(&k).is_some() {
                a.faults.push(
                    Fault::new(
                        codes::doc::REMOVE_WITH_KEYS,
                        entity_subject(top, id, &below(&kp)),
                        At::Span(span),
                        format!("the change `{}` both writes and unsets `{k}`", ch.name),
                        "keep one: write the key, or unset it",
                    )
                    .chain(&u.chain),
                );
            } else if target.get(&k).is_none() {
                a.faults
                    .push(missing(&ch.name, top, id, &below(&kp), span, u));
            } else {
                target.remove(&k);
                a.writes.push((kp, span));
            }
        }
    }
    let placed: Vec<&str> = PLACEMENT
        .iter()
        .copied()
        .filter(|k| body.get(k).is_some())
        .collect();
    if let Some(first) = placed.first().and_then(|k| body.get(k)) {
        for p in PLACEMENT {
            if !placed.contains(&p) {
                target.remove(p);
            }
            a.writes.push((path_of(path, p), first.key));
        }
        if placed.iter().any(|k| *k != "path") {
            for q in PATH_ONLY {
                if body.get(q).is_none() {
                    target.remove(q);
                }
                a.writes.push((path_of(path, q), first.key));
            }
        }
    }
    for (k, e) in body.iter() {
        if k == "unset" || k == "remove" {
            continue;
        }
        let kp = path_of(path, k);
        let mergeable = !VALUE_TABLES.contains(&k.as_str());
        match (&e.item.value, target.get(k).map(|x| &x.item.value)) {
            (Value::Table(bt), Some(Value::Table(_))) if mergeable => {
                if let Some(r) = bt.get("remove") {
                    if bt.len() > 1 {
                        a.faults.push(
                            Fault::new(
                                codes::doc::REMOVE_WITH_KEYS,
                                entity_subject(top, id, &below(&kp)),
                                At::Span(r.key),
                                format!(
                                    "`remove = true` on `{}` sits beside other keys",
                                    kp.join(".")
                                ),
                                "keep `remove = true` alone, or drop it and write the keys",
                            )
                            .chain(&r.chain),
                        );
                    } else if is_remove(bt) {
                        target.remove(k);
                        a.writes.push((kp, r.key));
                    }
                    continue;
                }
                let sub_prior = target.get(k).map(|x| x.chain.clone()).unwrap_or_default();
                if let Some(sub) = target.table_mut(k) {
                    merge(sub, bt, &kp, &sub_prior, ch, a, ent);
                }
            }
            (Value::Table(bt), None) if mergeable => {
                if let Some(r) = bt.get("remove") {
                    a.faults
                        .push(missing(&ch.name, top, id, &below(&kp), r.key, r));
                    continue;
                }
                let mut chain = prior.to_vec();
                chain.push(link(ch, e.key));
                target.insert(
                    k.clone(),
                    Entry {
                        key: e.key,
                        item: crate::tree::Item {
                            span: e.item.span,
                            value: Value::Table(Table::new(bt.header(), bt.form())),
                        },
                        chain: chain.clone(),
                    },
                );
                if let Some(sub) = target.table_mut(k) {
                    merge(sub, bt, &kp, &chain, ch, a, ent);
                }
            }
            _ => {
                let old = target.get(k).map(|x| x.chain.clone());
                let base_chain = old.unwrap_or_else(|| prior.to_vec());
                target.insert(k.clone(), written_entry(e, &base_chain, ch));
                a.writes.push((kp, e.key));
            }
        }
    }
}

/// `doc.change_conflict` (6.3): two requested changes whose write sets share a path, or where one
/// path is a prefix of the other. One record per pair of changes per entity.
pub(crate) fn conflicts(applied: &[(String, Applied)]) -> Vec<Fault> {
    let mut out = Vec::new();
    for (i, (na, a)) in applied.iter().enumerate() {
        for (nb, b) in &applied[i + 1..] {
            let mut seen: Vec<Vec<String>> = Vec::new();
            for (pa, sa) in &a.writes {
                for (pb, sb) in &b.writes {
                    let n = pa.len().min(pb.len());
                    if pa[..n] != pb[..n] {
                        continue;
                    }
                    let shorter = if pa.len() <= pb.len() { pa } else { pb };
                    let group: Vec<String> = shorter.iter().take(2).cloned().collect();
                    if seen.contains(&group) {
                        continue;
                    }
                    seen.push(group);
                    let (top, rest) = shorter
                        .split_first()
                        .map_or(("", &[][..]), |(t, r)| (t.as_str(), r));
                    let sub = match rest.split_first() {
                        Some((id, keys)) => entity_subject(top, id, keys),
                        None => Subject::World,
                    };
                    out.push(
                        Fault::new(
                            codes::doc::CHANGE_CONFLICT,
                            sub.clone(),
                            At::Span(*sa),
                            format!(
                                "the changes `{na}` and `{nb}` both write `{}`",
                                shorter.join(".")
                            ),
                            format!(
                                "request one of `{na}` and `{nb}`, or merge them into one change"
                            ),
                        )
                        .related(
                            sub,
                            At::Span(*sb),
                            format!("written by `{nb}`"),
                        ),
                    );
                }
            }
        }
    }
    out
}
