//! Step 6 of reading (`spec/documents.md` 8): closed keys against the final kinds, required
//! keys, what each key takes, and placement, all on the merged world.

use crate::report::{At, Fault};
use crate::schema::{self, Rule, Shape, Takes, FEATURE_RESERVED, PLACEMENT};
use crate::tree::{Entry, Item, Table, Value};
use std::collections::BTreeSet;
use via_diag::nearest::nearest;
use via_diag::{codes, registry, KeyPath, Subject};
use via_num::NumError;

/// A subject for a key path below an entity, or below the world.
pub(crate) fn subject(top: &str, id: Option<&str>, path: &[String]) -> Subject {
    let base = match id {
        Some(id) => Subject::entity(top, id),
        None => {
            // A top-level key of the world is written `world` (`diagnostics.md` 1.1).
            return Subject::World;
        }
    };
    match KeyPath::from_parts(
        path.iter()
            .map(|k| via_diag::KeyPart::Key(k.clone()))
            .collect(),
    ) {
        Some(p) => base.with_keys(&p),
        None => base,
    }
}

/// The checker for one merged world.
pub(crate) struct Checker<'w> {
    pub root: &'w Table,
    pub faults: Vec<Fault>,
}

impl<'w> Checker<'w> {
    pub(crate) fn new(root: &'w Table) -> Checker<'w> {
        Checker {
            root,
            faults: Vec::new(),
        }
    }

    /// Every check of step 6.
    pub(crate) fn run(mut self) -> Vec<Fault> {
        let root = self.root;
        for (top, entry) in root.iter() {
            if schema::WORLD_KEYS.contains(&top.as_str()) {
                if let Some(Rule::Ok(t)) = schema::world_key(top) {
                    self.value(t, entry, &Ctx::world(top));
                }
                continue;
            }
            let Value::Table(table) = &entry.item.value else {
                continue;
            };
            match top.as_str() {
                "layer" | "change" => {}
                "let" => {
                    for (name, e) in table.iter() {
                        let ctx = Ctx::entity("let", name, Vec::new());
                        self.value(Takes::Formula, e, &ctx);
                    }
                }
                "kind" => {
                    for (id, e) in table.iter() {
                        if let Value::Table(t) = &e.item.value {
                            let ctx = Ctx::entity("kind", id, Vec::new());
                            self.table(Shape::Kind, t, &ctx);
                            self.reserved_attributes(t, &ctx);
                        }
                    }
                }
                _ => {
                    let Some(shape) = schema::entity_shape(top) else {
                        continue;
                    };
                    for (id, e) in table.iter() {
                        if let Value::Table(t) = &e.item.value {
                            let ctx = Ctx::entity(top, id, Vec::new());
                            if shape == Shape::Feature {
                                self.feature(t, &ctx);
                            } else {
                                self.table(shape, t, &ctx);
                            }
                        }
                    }
                }
            }
        }
        self.faults
    }

    fn push(&mut self, f: Fault) {
        self.faults.push(f);
    }

    fn unknown(&mut self, e: &Entry, ctx: &Ctx, key: &str, listed: &[&str], fixed: Option<&str>) {
        let sub = ctx.subject_key(key);
        let mut f = Fault::new(
            codes::doc::UNKNOWN_KEY,
            sub,
            At::Span(e.key),
            format!("`{key}` is not a key of {}", ctx.place()),
            String::new(),
        )
        .chain(&e.chain);
        match fixed {
            Some(h) => f.hint = h.to_string(),
            None => match nearest(key, listed.iter().copied()) {
                via_diag::nearest::Nearest::Found(n) => {
                    f.hint = format!("write `{n}`");
                    f = f.with("nearest", n);
                }
                other => f.hint = other.hint("remove the key"),
            },
        }
        self.push(f);
    }

    fn not_yet(&mut self, e: &Entry, ctx: &Ctx, word: &str, milestone: u32) {
        self.push(not_yet(ctx.subject_key(word), At::Span(e.key), word, milestone).chain(&e.chain));
    }

    /// A table of a known shape.
    fn table(&mut self, shape: Shape, t: &Table, ctx: &Ctx) {
        let listed: Vec<&str> = schema::listed(shape).collect();
        for (k, e) in t.iter() {
            match schema::rule(shape, k) {
                None if shape == Shape::Let => {
                    self.value(Takes::Formula, e, &ctx.below(k));
                }
                None => self.unknown(e, ctx, k, &listed, None),
                Some(Rule::NotYet(m)) => self.not_yet(e, ctx, k, m),
                Some(Rule::Fixed(h)) => self.unknown(e, ctx, k, &listed, Some(h)),
                Some(Rule::Ok(takes)) => {
                    if let (Takes::Str | Takes::Bool, Some(m)) = (takes, word_not_yet(shape, k, e))
                    {
                        let word = describe(k, &e.item.value);
                        self.push(
                            not_yet(ctx.subject_key(k), At::Span(e.key), &word, m).chain(&e.chain),
                        );
                        continue;
                    }
                    self.value(takes, e, &ctx.below(k));
                }
            }
        }
        self.placement(t, ctx);
        self.required(shape, t, ctx);
    }

    fn required(&mut self, shape: Shape, t: &Table, ctx: &Ctx) {
        let need: &[(&str, &str)] = match shape {
            // `on` names an authored thing by `ref` or a produced one by `of`.
            Shape::On if t.get("ref").is_none() && t.get("of").is_none() => &[("ref", "a name")],
            Shape::Follows if t.get("ref").is_none() => &[("ref", "a name")],
            Shape::Choice if t.get("id").is_none() => &[("id", "an asset id")],
            _ => &[],
        };
        for (key, takes) in need {
            let mut f = Fault::new(
                codes::doc::MISSING_KEY,
                ctx.subject(),
                At::Span(t.header()),
                format!("{} has no `{key}`", ctx.place()),
                format!("write `{key}`, {takes}"),
            );
            if ctx
                .path
                .last()
                .is_some_and(|k| schema::VALUE_TABLES.contains(&k.as_str()))
            {
                f.hint = format!(
                    "write `{key}`, {takes}: a change replaces `{}` whole, so write the whole table",
                    ctx.path.last().map_or("", String::as_str)
                );
            }
            self.push(f);
        }
    }

    fn placement(&mut self, t: &Table, ctx: &Ctx) {
        let written: Vec<&str> = PLACEMENT
            .iter()
            .copied()
            .filter(|k| t.get(k).is_some())
            .collect();
        if written.len() > 1 {
            let f = Fault::new(
                codes::doc::PLACEMENT_TWICE,
                ctx.subject(),
                At::Span(t.header()),
                format!(
                    "{} holds the placement keys `{}` and `{}`, which exclude each other",
                    ctx.place(),
                    written[0],
                    written[1]
                ),
                format!("keep one of `{}`", written.join("`, `")),
            );
            self.push(f);
        }
    }

    fn reserved_attributes(&mut self, kind: &Table, ctx: &Ctx) {
        if let Some(Value::Table(d)) = kind.value("defaults") {
            for (k, e) in d.iter() {
                if FEATURE_RESERVED.contains(&k.as_str()) {
                    self.push(reserved(ctx, &["defaults", k], e, k));
                }
            }
        }
        if let Some(Value::List(items)) = kind.value("requires") {
            for it in items {
                if let Value::Str(s) = &it.value {
                    if FEATURE_RESERVED.contains(&s.text.as_str()) {
                        let e = Entry {
                            key: it.span,
                            item: it.clone(),
                            chain: Vec::new(),
                        };
                        self.push(reserved(ctx, &["requires"], &e, &s.text));
                    }
                }
            }
        }
    }

    /// A feature: its reserved keys, then the attributes its kind declares as the changes left it.
    fn feature(&mut self, t: &Table, ctx: &Ctx) {
        let kind = t
            .str("kind")
            .and_then(|k| self.root.table("kind").and_then(|ks| ks.table(k)));
        let unknown_kind = t.str("kind").is_some() && kind.is_none();
        let mut attrs: BTreeSet<String> = BTreeSet::new();
        let mut formula_attrs: BTreeSet<String> = BTreeSet::new();
        if let Some(kind) = kind {
            if let Some(Value::Table(d)) = kind.value("defaults") {
                for (k, e) in d.iter() {
                    attrs.insert(k.clone());
                    let takes_formula = match &e.item.value {
                        Value::Num(_) => true,
                        Value::Str(s) => s.text.starts_with('='),
                        _ => false,
                    };
                    if takes_formula {
                        formula_attrs.insert(k.clone());
                    }
                }
            }
            if let Some(Value::List(items)) = kind.value("requires") {
                for it in items {
                    if let Value::Str(s) = &it.value {
                        attrs.insert(s.text.clone());
                    }
                }
            }
        }
        let mut listed: Vec<&str> = schema::listed(Shape::Feature).collect();
        listed.extend(attrs.iter().map(String::as_str));
        let mut reserved_only = Table::new(t.header(), t.form());
        for (k, e) in t.iter() {
            if FEATURE_RESERVED.contains(&k.as_str()) || schema::rule(Shape::Feature, k).is_some() {
                reserved_only.insert(k.clone(), e.clone());
            } else if attrs.contains(k) {
                if formula_attrs.contains(k) {
                    self.value(Takes::Formula, e, &ctx.below(k));
                } else if let Value::DateTime = e.item.value {
                    self.value(Takes::Str, e, &ctx.below(k));
                }
            } else if !unknown_kind {
                self.unknown(e, ctx, k, &listed, None);
            }
        }
        self.table(Shape::Feature, &reserved_only, ctx);
    }

    /// One value against what its key takes.
    fn value(&mut self, takes: Takes, e: &Entry, ctx: &Ctx) {
        self.item(takes, &e.item, e, ctx);
    }

    fn item(&mut self, takes: Takes, item: &Item, e: &Entry, ctx: &Ctx) {
        let v = &item.value;
        let ok = match (takes, v) {
            (_, Value::DateTime) => false,
            (Takes::Str | Takes::Name | Takes::RuleSeed, Value::Str(_)) => true,
            (Takes::Integer, Value::Num(n)) => n.integer,
            (Takes::Number, Value::Num(n)) => {
                self.num(via_num::parse_double(&n.norm).map(|_| ()), e, ctx)
            }
            (Takes::Length, Value::Num(n)) => {
                self.num(via_num::parse_length(&n.norm).map(|_| ()), e, ctx)
            }
            (Takes::Facing, Value::Num(n)) => {
                self.num(via_num::parse_facing(&n.norm).map(|_| ()), e, ctx)
            }
            (Takes::Facing, Value::Str(_)) => true,
            (Takes::Bool, Value::Bool(_)) => true,
            (Takes::Formula, Value::Num(n)) => {
                self.num(via_num::parse_double(&n.norm).map(|_| ()), e, ctx)
            }
            (Takes::Formula, Value::Str(s)) => {
                if !s.text.starts_with('=') {
                    self.push(
                        Fault::new(
                            codes::formula::UNMARKED,
                            ctx.subject(),
                            At::Span(e.key),
                            format!(
                                "{} takes a formula, and the string `{}` has no leading `=`",
                                ctx.place(),
                                s.text
                            ),
                            format!("write `\"= {}\"`, or a number", s.text),
                        )
                        .chain(&e.chain),
                    );
                }
                true
            }
            (Takes::Point, Value::List(xs)) => {
                xs.len() == 2 && xs.iter().all(|x| self.item_ok(Takes::Length, x, e, ctx))
            }
            (Takes::Points, Value::List(xs)) => {
                xs.iter().all(|x| self.item_ok(Takes::Point, x, e, ctx))
            }
            (Takes::Lengths(n), Value::List(xs)) => {
                xs.len() == n && xs.iter().all(|x| self.item_ok(Takes::Length, x, e, ctx))
            }
            (Takes::Numbers(n), Value::List(xs)) => {
                xs.len() == n && xs.iter().all(|x| self.item_ok(Takes::Number, x, e, ctx))
            }
            (Takes::Strings | Takes::Names, Value::List(xs)) => {
                xs.iter().all(|x| matches!(x.value, Value::Str(_)))
            }
            (Takes::NumberOrPair, Value::Num(_)) => self.item_ok(Takes::Number, item, e, ctx),
            (Takes::NumberOrPair, Value::List(_)) => self.item_ok(Takes::Numbers(2), item, e, ctx),
            (Takes::NameOrSelect, Value::Str(_)) => true,
            (Takes::NameOrSelect, Value::Table(t)) => {
                self.table(Shape::Select, t, ctx);
                true
            }
            (Takes::Choices, Value::List(xs)) => {
                let mut all = true;
                for (i, x) in xs.iter().enumerate() {
                    match &x.value {
                        Value::Str(_) => {}
                        Value::Table(t) => self.table(Shape::Choice, t, &ctx.index(i)),
                        _ => all = false,
                    }
                }
                all
            }
            (Takes::WorldSeed, Value::Num(n)) if n.integer => {
                let r = n
                    .norm
                    .parse::<i64>()
                    .map_err(|_| NumError::OutOfRange {
                        text: n.norm.clone(),
                    })
                    .and_then(via_num::world_seed);
                self.num(r.map(|_| ()), e, ctx)
            }
            (Takes::Table(shape), Value::Table(t)) => {
                self.table(shape, t, ctx);
                true
            }
            (Takes::Ids(shape), Value::Table(t)) => {
                for (id, sub) in t.iter() {
                    let sctx = ctx.below(id);
                    if let Some(f) = crate::ids::check(id, sctx.subject(), At::Span(sub.key), "id")
                    {
                        self.push(f.chain(&sub.chain));
                    }
                    match &sub.item.value {
                        Value::Table(st) => self.table(shape, st, &sctx),
                        _ => self.type_fault(Takes::Table(shape), sub, &sctx),
                    }
                }
                true
            }
            (Takes::Free | Takes::Defaults, Value::Table(_)) => true,
            _ => false,
        };
        if !ok {
            self.type_fault(takes, e, ctx);
        }
    }

    /// A list element: true when it fits, without its own report (the whole value is reported).
    fn item_ok(&mut self, takes: Takes, item: &Item, e: &Entry, ctx: &Ctx) -> bool {
        let before = self.faults.len();
        let mut probe = Checker::new(self.root);
        probe.item(takes, item, e, ctx);
        // A number fault inside (too precise, out of range) is kept; a type fault is the caller's.
        let mut fits = true;
        for f in probe.faults {
            if f.code == codes::doc::TYPE {
                fits = false;
            } else {
                self.faults.push(f);
            }
        }
        let _ = before;
        fits
    }

    fn num(&mut self, r: Result<(), NumError>, e: &Entry, ctx: &Ctx) -> bool {
        match r {
            Ok(()) => true,
            Err(NumError::Malformed { .. }) => false,
            Err(err) => {
                let code = registry::lookup(err.code()).unwrap_or(codes::doc::TYPE);
                let hint = match &err {
                    NumError::TooPrecise { nearest, .. } => {
                        format!("write `{}` or `{}`", nearest[0], nearest[1])
                    }
                    NumError::SeedRange { .. } => "write a seed from 0 to 2^63 - 1".to_string(),
                    _ => "write a value in range".to_string(),
                };
                self.push(
                    Fault::new(
                        code,
                        ctx.subject(),
                        At::Span(e.key),
                        format!("{}: {err}", ctx.place()),
                        hint,
                    )
                    .chain(&e.chain),
                );
                true
            }
        }
    }

    fn type_fault(&mut self, takes: Takes, e: &Entry, ctx: &Ctx) {
        let written = e.item.value.form_word();
        let mut hint = format!("write {}", takes.words());
        if let (Value::Str(s), Takes::Number | Takes::Integer | Takes::Point | Takes::Length) =
            (&e.item.value, takes)
        {
            if s.text.starts_with('=') {
                hint = format!("this key takes no formula; write {}", takes.words());
            }
        }
        if let Value::DateTime = e.item.value {
            hint = format!(
                "TOML dates and times have no meaning here; write {}",
                takes.words()
            );
        }
        self.push(
            Fault::new(
                codes::doc::TYPE,
                ctx.subject(),
                At::Span(e.key),
                format!(
                    "{} takes {}, and {written} is written",
                    ctx.place(),
                    takes.words()
                ),
                hint,
            )
            .chain(&e.chain),
        );
    }
}

fn reserved(ctx: &Ctx, path: &[&str], e: &Entry, name: &str) -> Fault {
    let mut sub = ctx.clone();
    for p in path {
        sub = sub.below(p);
    }
    Fault::new(
        codes::doc::RESERVED_ATTRIBUTE,
        sub.subject(),
        At::Span(e.key),
        format!("the kind declares the attribute `{name}`, which is a reserved feature key"),
        format!("name the attribute other than `{name}`"),
    )
    .chain(&e.chain)
}

/// `doc.not_yet` for a word of a later milestone.
pub(crate) fn not_yet(subject: Subject, at: At, word: &str, milestone: u32) -> Fault {
    Fault::new(
        codes::doc::NOT_YET,
        subject,
        at,
        format!("`{word}` arrives in milestone M{milestone}; this engine reads M1"),
        format!("remove `{word}` until M{milestone}"),
    )
}

fn word_not_yet(shape: Shape, key: &str, e: &Entry) -> Option<u32> {
    match &e.item.value {
        Value::Str(s) => schema::value_not_yet(shape, key, &s.text),
        Value::Bool(true) if shape == Shape::Step && key == "worker" => Some(2),
        _ => None,
    }
}

fn describe(key: &str, v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{key} = \"{}\"", s.text),
        Value::Bool(b) => format!("{key} = {b}"),
        _ => key.to_string(),
    }
}

/// Where a check is: the entity and the key path below it.
#[derive(Clone, Debug)]
pub(crate) struct Ctx {
    pub top: String,
    pub id: Option<String>,
    pub path: Vec<String>,
}

impl Ctx {
    pub(crate) fn world(key: &str) -> Ctx {
        Ctx {
            top: key.to_string(),
            id: None,
            path: Vec::new(),
        }
    }

    pub(crate) fn entity(top: &str, id: &str, path: Vec<String>) -> Ctx {
        Ctx {
            top: top.to_string(),
            id: Some(id.to_string()),
            path,
        }
    }

    pub(crate) fn below(&self, k: &str) -> Ctx {
        let mut c = self.clone();
        if c.id.is_some() {
            c.path.push(k.to_string());
        }
        c
    }

    fn index(&self, i: usize) -> Ctx {
        self.below(&format!("[{i}]"))
    }

    pub(crate) fn subject(&self) -> Subject {
        match &self.id {
            None => Subject::World,
            Some(id) => {
                let parts = self
                    .path
                    .iter()
                    .map(
                        |p| match p.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                            Some(n) => n.parse::<u32>().map_or(
                                via_diag::KeyPart::Key(p.clone()),
                                via_diag::KeyPart::Index,
                            ),
                            None => via_diag::KeyPart::Key(p.clone()),
                        },
                    )
                    .collect();
                match KeyPath::from_parts(parts) {
                    Some(kp) => Subject::entity(&self.top, id).with_keys(&kp),
                    None => Subject::entity(&self.top, id),
                }
            }
        }
    }

    pub(crate) fn subject_key(&self, k: &str) -> Subject {
        self.below(k).subject()
    }

    /// The table in words.
    pub(crate) fn place(&self) -> String {
        match &self.id {
            None => format!("the world's `{}`", self.top),
            Some(_) => format!("`{}`", self.subject()),
        }
    }
}

/// A layer's rank and owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rank {
    pub rank: i64,
    pub owner: Option<String>,
}

/// One named reference: the key that writes it and the name.
struct Named<'a> {
    path: Vec<String>,
    entry: &'a Entry,
    name: String,
    span: crate::source::Span,
}

fn names_in<'a>(path: Vec<String>, e: &'a Entry, out: &mut Vec<Named<'a>>) {
    match &e.item.value {
        Value::Str(s) => out.push(Named {
            path,
            entry: e,
            name: s.text.clone(),
            span: e.key,
        }),
        Value::List(items) => {
            for it in items {
                if let Value::Str(s) = &it.value {
                    out.push(Named {
                        path: path.clone(),
                        entry: e,
                        name: s.text.clone(),
                        span: it.span,
                    });
                }
            }
        }
        _ => {}
    }
}

/// Where an id of the world namespace is declared: its top table and home layer.
pub(crate) fn find_id<'b>(
    root: &Table,
    homes: &'b std::collections::BTreeMap<(String, String), String>,
    id: &str,
) -> Option<(String, &'b String)> {
    for top in [
        "clear", "feature", "field", "import", "instance", "layer", "lock", "pin", "run",
        "scatter", "set", "step",
    ] {
        if root.table(top).is_some_and(|t| t.get(id).is_some()) {
            if let Some(h) = homes.get(&(top.to_string(), id.to_string())) {
                return Some((top.to_string(), h));
            }
        }
    }
    None
}

/// Step 7's direction check on named references (5.1), and `doc.dangling` for a name whose
/// target a requested change removed.
pub(crate) fn direction(
    root: &Table,
    homes: &std::collections::BTreeMap<(String, String), String>,
    ranks: &std::collections::BTreeMap<String, Rank>,
    removed: &[crate::change::Removal],
) -> Vec<Fault> {
    let mut out = Vec::new();
    for top in ["feature", "instance", "pin", "clear", "scatter"] {
        let Some(t) = root.table(top) else { continue };
        for (id, e) in t.iter() {
            let Value::Table(ent) = &e.item.value else {
                continue;
            };
            let mut refs: Vec<Named> = Vec::new();
            let simple: &[&str] = match top {
                "feature" => &["starts_at", "ends_at"],
                "pin" | "clear" => &["by"],
                "scatter" => &["within"],
                _ => &[],
            };
            for k in simple {
                if let Some(x) = ent.get(k) {
                    names_in(vec![(*k).to_string()], x, &mut refs);
                }
            }
            for (outer, inner) in [
                ("on", "ref"),
                ("on", "of"),
                ("follows", "ref"),
                ("within", "of"),
            ] {
                if let Some(x) = ent.table(outer).and_then(|o| o.get(inner)) {
                    names_in(vec![outer.to_string(), inner.to_string()], x, &mut refs);
                }
            }
            if top == "feature" {
                let names = ent
                    .str("kind")
                    .and_then(|k| root.table("kind").and_then(|ks| ks.table(k)))
                    .and_then(|k| match k.value("names") {
                        Some(Value::List(items)) => Some(items.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                for n in names {
                    if let Value::Str(s) = &n.value {
                        if let Some(x) = ent.get(&s.text) {
                            names_in(vec![s.text.clone()], x, &mut refs);
                        }
                    }
                }
            }
            let Some(home) = homes.get(&(top.to_string(), id.clone())) else {
                continue;
            };
            let Some(mine) = ranks.get(home) else {
                continue;
            };
            for r in refs {
                let first = r.name.split('.').next().unwrap_or("");
                let sub = subject(top, Some(id), &r.path);
                match find_id(root, homes, first) {
                    Some((ttop, thome)) => {
                        let Some(theirs) = ranks.get(thome) else {
                            continue;
                        };
                        if theirs.rank > mine.rank {
                            let owner = |o: &Option<String>| {
                                o.as_ref()
                                    .map_or("no owner".to_string(), |o| format!("owner `{o}`"))
                            };
                            out.push(
                                Fault::new(
                                    codes::doc::UPWARD_REF,
                                    sub,
                                    At::Span(r.span),
                                    format!(
                                        "`{top}.{id}` in layer `{home}` (rank {}, {}) names `{ttop}.{first}` in layer `{thome}` (rank {}, {}); a layer refers downward only",
                                        mine.rank,
                                        owner(&mine.owner),
                                        theirs.rank,
                                        owner(&theirs.owner)
                                    ),
                                    format!(
                                        "move `{top}.{id}` to layer `{thome}` or above, or move `{ttop}.{first}` to rank {} or below",
                                        mine.rank
                                    ),
                                )
                                .chain(&r.entry.chain)
                                .related(
                                    Subject::entity(&ttop, first),
                                    match root.table(&ttop).and_then(|t| t.table(first)) {
                                        Some(tt) => At::Span(tt.header()),
                                        None => At::Span(r.span),
                                    },
                                    format!("declared in layer `{thome}`"),
                                ),
                            );
                        }
                    }
                    None => {
                        if let Some(rm) = removed.iter().find(|rm| rm.id == first) {
                            out.push(
                                Fault::new(
                                    codes::doc::DANGLING,
                                    sub,
                                    At::Span(r.span),
                                    format!(
                                        "`{top}.{id}` names `{first}`, which the change `{}` removes",
                                        rm.change
                                    ),
                                    format!(
                                        "remove or re-point `{top}.{id}` in the change `{}` too, or keep `{}.{first}`",
                                        rm.change, rm.top
                                    ),
                                )
                                .chain(&r.entry.chain)
                                .related(
                                    Subject::entity(&rm.top, &rm.id),
                                    At::Span(rm.span),
                                    format!("removed by `{}`", rm.change),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
    out
}
