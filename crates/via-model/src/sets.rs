//! Sets and runs (`rules.md` 4 and 5): their tables read, and placed as plans.
//!
//! A set is a rule that lowers to plain instances: placed by an instance or a run member, each
//! item becomes a plan named `<placing>.<item>` in the placing thing's frame. A run is a producer:
//! its members are plans named `<run>#r<k>`, at whole-millimetre arc lengths along its stretch.
//! Both reach the rest of the build as plans, so grounding, checks and hand edits read them as
//! they read any instance.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Site, Subject, Value as DValue};
use via_doc::{Table, Value, World};
use via_geom::lineref::{self, Line};
use via_geom::offset::{offset, Join};
use via_geom::overlay::{boolean, BoolOp};
use via_geom::{Region, P};
use via_num::Frame;

use crate::decl::{table_of, Decls, Face, On, Place, Ranged};
use crate::place::{Geo, Placer, Plan, Spot};
use crate::text::{self, at_header, at_key, at_path};

/// What an item or a run member places.
#[derive(Clone, Debug, PartialEq)]
pub enum What {
    Asset(String),
    Set(String),
    /// A run's `assets` (`rules.md` 5, R68): one asset id for each member, in station order.
    /// Never an item's.
    Assets(Vec<String>),
}

/// One item of a set, in the set's frame (`rules.md` 4).
#[derive(Clone, Debug)]
pub struct ItemDecl {
    pub what: What,
    pub at: (i64, i64),
    /// Millidegrees, added to the placing thing's bearing.
    pub face: i64,
    pub scale: f64,
    pub tint: f64,
}

/// `[set.<id>]`.
#[derive(Clone, Debug)]
pub struct SetDecl {
    pub items: BTreeMap<String, ItemDecl>,
}

/// `[run.<id>]` (`rules.md` 5).
#[derive(Clone, Debug)]
pub struct RunDecl {
    pub what: What,
    /// The stretch's host: `Place::On` (its `t` unused) or `Place::Of`.
    pub place: Place,
    pub span: (f64, f64),
    pub offset: i64,
    pub count: Option<i64>,
    /// Millimetres.
    pub spacing: Option<i64>,
    /// `ends = "both"`.
    pub both: bool,
    pub face: Option<Face>,
    pub scale: f64,
    pub tint: f64,
    pub ground: Option<String>,
    pub margin: i64,
    pub rank: i64,
}

impl RunDecl {
    /// Every asset id the run names: its `asset`, or each entry of its `assets` (`rules.md` 5).
    /// None for a run of a set, whose items name theirs.
    pub fn asset_ids(&self) -> Vec<&str> {
        match &self.what {
            What::Asset(a) => vec![a.as_str()],
            What::Assets(xs) => xs.iter().map(String::as_str).collect(),
            What::Set(_) => Vec::new(),
        }
    }

    /// The place name the stretch reads.
    pub fn reference(&self) -> &str {
        match &self.place {
            Place::On(o) => &o.reference,
            Place::Of(s) => &s.of,
            Place::At(_) => "",
        }
    }
}

/// Item names `identity.md` reserves for parts of a place name.
const RESERVED: [&str; 3] = ["start", "end", "ground"];

impl Decls {
    /// The run that made a produced name, its members and their items alike. A run a step
    /// returned is itself `<step>#<key>`, so its members are `<step>#<key>#r<k>`: the run is what
    /// stands before the last `#` (split at the first, a laid wall's members stood on the terrain).
    pub fn run_of<'n>(&self, name: &'n str) -> Option<&'n str> {
        let (m, _) = name.rsplit_once('#')?;
        self.runs.contains_key(m).then_some(m)
    }

    /// The declared step whose returned document holds a produced name, `<step>#...`
    /// (`protocol.md` 5, Names): its instances, its runs and their members alike. `None` for a
    /// name the world's own text or another maker gave.
    pub fn step_of<'n>(&self, name: &'n str) -> Option<&'n str> {
        let (step, _) = name.split_once('#')?;
        self.steps.contains_key(step).then_some(step)
    }

    /// The layer rank of a placed name (`authority.md` 3): a hand instance's, or its set item's
    /// (`<instance>.<item>`); a run member's (`<run>#r<k>`, and its items) is its run's. 0 when it
    /// names neither.
    pub fn placed_rank(&self, name: &str) -> i64 {
        self.placed_rank_of(name).unwrap_or(0)
    }

    /// [`Decls::placed_rank`], `None` when the name is neither a hand instance's nor a run's.
    pub fn placed_rank_of(&self, name: &str) -> Option<i64> {
        let host = name.split('.').next().unwrap_or(name);
        if let Some(i) = self.instances.get(host) {
            return Some(i.rank);
        }
        self.run_of(host)
            .and_then(|r| self.runs.get(r))
            .map(|r| r.rank)
    }

    /// The statements a placed set reads: `set.<id>` of it and of every set inside it.
    pub fn set_stmts(&self, set: Option<&str>) -> Vec<String> {
        let mut out = BTreeSet::new();
        let mut stack: Vec<&str> = set.into_iter().collect();
        while let Some(s) = stack.pop() {
            if !out.insert(format!("set.{s}")) {
                continue;
            }
            if let Some(sd) = self.sets.get(s) {
                for it in sd.items.values() {
                    if let What::Set(n) = &it.what {
                        stack.push(n);
                    }
                }
            }
        }
        out.into_iter().collect()
    }

    /// `asset` or `set`: exactly one (`place.asset_or_set`); a set names a set (`set.unknown`).
    /// Gives (asset, set); the asset is empty for a set. `None` after a fault.
    pub(crate) fn read_what(
        &mut self,
        top: &str,
        id: &str,
        t: &Table,
    ) -> Option<(String, Option<String>)> {
        match (t.str("asset"), t.str("set")) {
            (Some(a), None) => Some((a.to_string(), None)),
            (None, Some(s)) => self
                .known_set(top, id, &["set"], s)
                .then(|| (String::new(), Some(s.to_string()))),
            (a, _) => {
                let both = a.is_some();
                self.push(at_header(
                    codes::place::ASSET_OR_SET,
                    top,
                    id,
                    if both {
                        "it writes both `asset` and `set`"
                    } else {
                        "it writes neither `asset` nor `set`"
                    },
                    "keep one of `asset`, `set`",
                ));
                None
            }
        }
    }

    /// Whether `set` names a set; `set.unknown` with the nearest id when not. A set that failed
    /// its own checks names one, and says nothing more here.
    fn known_set(&mut self, top: &str, id: &str, path: &[&str], set: &str) -> bool {
        if self.sets.contains_key(set) {
            return true;
        }
        if self.failed_sets.contains(set) {
            return false;
        }
        let names: Vec<String> = self.sets.keys().cloned().collect();
        self.push(text::with_nearest(
            at_path(
                codes::set::UNKNOWN,
                top,
                id,
                path,
                &format!("`{set}` names no set"),
                &text::near_hint(
                    set,
                    names.iter().map(String::as_str),
                    "declare `[set.<id>]`",
                ),
            ),
            set,
            names.iter().map(String::as_str),
            text::header_of("set"),
        ));
        false
    }

    /// Every `[set.<id>]`: items read, then names and cycles checked.
    pub(crate) fn read_sets(&mut self, root: &Table) {
        let tables = table_of(root, "set");
        let ids: BTreeSet<&str> = tables.iter().map(|(id, _)| *id).collect();
        for (id, t) in &tables {
            let items = table_of(t, "item");
            if items.is_empty() {
                self.push(at_header(
                    codes::set::EMPTY,
                    "set",
                    id,
                    &format!("the set `{id}` has no items"),
                    "add `[set.<id>.item.<name>]`, or remove the set",
                ));
                self.failed_sets.insert(id.to_string());
                continue;
            }
            let mut out = BTreeMap::new();
            let mut ok = true;
            for (name, it) in items {
                if RESERVED.contains(&name) {
                    let free = via_doc::free_name(name, &|c| t.get(c).is_some());
                    self.push(
                        at_path(
                            codes::identity::RESERVED,
                            "set",
                            id,
                            &["item", name],
                            &format!(
                                "an item may not be named `{name}`: it is a part of a place name"
                            ),
                            &format!("choose another name, such as `{free}`"),
                        )
                        .with("nearest", DValue::text(free)),
                    );
                    ok = false;
                    continue;
                }
                match self.read_item(id, name, it, &ids) {
                    Some(i) => {
                        out.insert(name.to_string(), i);
                    }
                    None => ok = false,
                }
            }
            if ok {
                self.sets.insert(id.to_string(), SetDecl { items: out });
            } else {
                self.failed_sets.insert(id.to_string());
            }
        }
        // A set that contains itself at any depth is the graph's cycle (`rules.md` 4).
        let mut cyclic = BTreeSet::new();
        for start in self.sets.keys() {
            let mut path = vec![start.clone()];
            if let Some(c) = self.cycle_from(start, &mut path) {
                let mut members = c.clone();
                members.sort();
                members.dedup();
                if members.first() == Some(start) {
                    self.diags.push(text::diag(
                        codes::graph::CYCLE,
                        Subject::entity("set", start),
                        Site::Header,
                        &format!("a set contains itself: {}", c.join(" -> ")),
                        "break the loop: let one of these sets place assets only",
                    ));
                }
                cyclic.extend(members);
            }
        }
        for s in cyclic {
            self.sets.remove(&s);
            self.failed_sets.insert(s);
        }
        // An item naming a set that failed leaves its set failed too.
        loop {
            let bad: Vec<String> = self
                .sets
                .iter()
                .filter(|(_, sd)| {
                    sd.items.values().any(|i| match &i.what {
                        What::Set(n) => !self.sets.contains_key(n),
                        What::Asset(_) | What::Assets(_) => false,
                    })
                })
                .map(|(k, _)| k.clone())
                .collect();
            if bad.is_empty() {
                break;
            }
            for b in bad {
                self.sets.remove(&b);
                self.failed_sets.insert(b);
            }
        }
    }

    /// The loop through `path`'s last set, if one is reached again.
    fn cycle_from(&self, at: &str, path: &mut Vec<String>) -> Option<Vec<String>> {
        let sd = self.sets.get(at)?;
        for it in sd.items.values() {
            if let What::Set(n) = &it.what {
                if let Some(k) = path.iter().position(|p| p == n) {
                    let mut c = path[k..].to_vec();
                    c.push(n.clone());
                    return Some(c);
                }
                path.push(n.clone());
                if let Some(c) = self.cycle_from(n, path) {
                    return Some(c);
                }
                path.pop();
            }
        }
        None
    }

    fn read_item(
        &mut self,
        set: &str,
        name: &str,
        t: &Table,
        ids: &BTreeSet<&str>,
    ) -> Option<ItemDecl> {
        let what = match (t.str("asset"), t.str("set")) {
            (Some(a), None) => What::Asset(a.to_string()),
            (None, Some(s)) => {
                if !ids.contains(s) {
                    let names: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
                    self.push(text::with_nearest(
                        at_path(
                            codes::set::UNKNOWN,
                            "set",
                            set,
                            &["item", name, "set"],
                            &format!("`{s}` names no set"),
                            &text::near_hint(
                                s,
                                names.iter().map(String::as_str),
                                "declare `[set.<id>]`",
                            ),
                        ),
                        s,
                        names.iter().map(String::as_str),
                        text::header_of("set"),
                    ));
                    return None;
                }
                What::Set(s.to_string())
            }
            (a, _) => {
                self.push(at_path(
                    codes::place::ASSET_OR_SET,
                    "set",
                    set,
                    &["item", name],
                    if a.is_some() {
                        "the item writes both `asset` and `set`"
                    } else {
                        "the item writes neither `asset` nor `set`"
                    },
                    "keep one of `asset`, `set`",
                ));
                return None;
            }
        };
        let at = match t.value("at") {
            None => (0, 0),
            Some(v) => match text::point(v)? {
                Ok(p) => p,
                Err(e) => {
                    self.push(text::num_diag(&e, "set", set, &["item", name, "at"]));
                    return None;
                }
            },
        };
        let face = match t.value("face") {
            None => 0,
            Some(Value::Num(n)) => match via_num::parse_facing(&n.norm) {
                Ok(m) => m,
                Err(e) => {
                    self.push(text::num_diag(&e, "set", set, &["item", name, "face"]));
                    return None;
                }
            },
            Some(_) => {
                self.push(at_path(
                    codes::doc::TYPE,
                    "set",
                    set,
                    &["item", name, "face"],
                    "an item's `face` is a bearing, added to the set's",
                    "write a number of degrees",
                ));
                return None;
            }
        };
        let mut st = [1.0, 1.0];
        for (k, key) in ["scale", "tint"].into_iter().enumerate() {
            let Some(v) = t.value(key) else { continue };
            let x = match v {
                Value::Num(n) => text::num(n).ok(),
                _ => None,
            };
            let ok = x.is_some_and(|x| if k == 0 { x > 0.0 } else { x >= 0.0 });
            if !ok {
                self.push(at_path(
                    codes::place::RANGE,
                    "set",
                    set,
                    &["item", name, key],
                    &format!("`{key}` is out of range, or a pair: only a scatter draws"),
                    if k == 0 {
                        "`scale` is one number above 0"
                    } else {
                        "`tint` is one number, 0 or more"
                    },
                ));
                return None;
            }
            st[k] = x.unwrap_or(1.0);
        }
        Some(ItemDecl {
            what,
            at,
            face,
            scale: st[0],
            tint: st[1],
        })
    }

    /// `set.unused` (info) for each set nothing places.
    pub(crate) fn sets_unused(&mut self) {
        let mut used: BTreeSet<String> = BTreeSet::new();
        for i in self.instances.values() {
            used.extend(i.set.clone());
        }
        for r in self.runs.values() {
            if let What::Set(s) = &r.what {
                used.insert(s.clone());
            }
        }
        for sd in self.sets.values() {
            for it in sd.items.values() {
                if let What::Set(s) = &it.what {
                    used.insert(s.clone());
                }
            }
        }
        let unused: Vec<String> = self
            .sets
            .keys()
            .filter(|s| !used.contains(*s))
            .cloned()
            .collect();
        for s in unused {
            self.push(at_header(
                codes::set::UNUSED,
                "set",
                &s,
                &format!("nothing places the set `{s}`"),
                "place it with an instance or run, or remove it",
            ));
        }
    }

    /// Every `[run.<id>]` (`rules.md` 5).
    pub(crate) fn read_runs(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "run") {
            if let Some(r) = self.read_run(world, id, t) {
                self.runs.insert(id.to_string(), r);
            }
        }
    }

    /// A run's `asset`, `set` or `assets`: exactly one (`place.asset_or_set`, `rules.md` 5).
    /// `assets` is a list of asset ids, one for each member (R68); the schema holds its form.
    fn read_run_what(&mut self, id: &str, t: &Table) -> Option<What> {
        let Some(list) = t.value("assets") else {
            if t.get("asset").is_none() && t.get("set").is_none() {
                self.push(at_header(
                    codes::place::ASSET_OR_SET,
                    "run",
                    id,
                    "the run writes none of `asset`, `set` and `assets`",
                    "write one of `asset`, `set`, `assets`",
                ));
                return None;
            }
            let (asset, set) = self.read_what("run", id, t)?;
            return Some(match set {
                Some(s) => What::Set(s),
                None => What::Asset(asset),
            });
        };
        if let Some(other) = ["asset", "set"].into_iter().find(|k| t.get(k).is_some()) {
            self.push(at_header(
                codes::place::ASSET_OR_SET,
                "run",
                id,
                &format!("the run writes both `{other}` and `assets`"),
                "keep one of `asset`, `set`, `assets`",
            ));
            return None;
        }
        let Value::List(xs) = list else { return None };
        let mut ids = Vec::with_capacity(xs.len());
        for x in xs {
            let Value::Str(a) = &x.value else { return None };
            ids.push(a.text.to_string());
        }
        Some(What::Assets(ids))
    }

    fn read_run(&mut self, world: &World, id: &str, t: &Table) -> Option<RunDecl> {
        let what = self.read_run_what(id, t)?;
        let Some(on) = t.table("on") else {
            self.push(at_header(
                codes::place::NO_POSITION,
                "run",
                id,
                "the run has no `on`",
                "write `on = { ref, span }`",
            ));
            return None;
        };
        if on.get("t").is_some() {
            self.push(at_path(
                codes::place::ON_FORM,
                "run",
                id,
                &["on", "t"],
                "`t` is a placement's; a run takes `span`",
                "write `span = [from, to]`",
            ));
            return None;
        }
        if on.get("of").is_some() && on.get("ref").is_some() {
            self.push(at_key(
                codes::identity::ON_BOTH,
                "run",
                id,
                "on",
                "`on` has both `ref` and `of`",
                "keep `ref` for an authored thing, `of` for a produced one",
            ));
            return None;
        }
        let (_, offset) = self.read_t_offset("run", id, on, "on")?;
        let place = if let Some(of) = on.str("of") {
            Place::Of(self.read_select("run", id, on, of, &["on"])?)
        } else {
            let reference = on.str("ref")?.to_string();
            if reference.contains('#') {
                self.push(at_path(
                    codes::identity::KEY_IN_TEXT,
                    "run",
                    id,
                    &["on", "ref"],
                    &format!("`{reference}` is a produced name, which text never writes"),
                    &format!(
                        "select it by place: `on = {{ of = \"{}\", near = [x, y] }}`",
                        reference.split('#').next().unwrap_or("")
                    ),
                ));
                return None;
            }
            Place::On(On {
                reference,
                t: None,
                offset,
            })
        };
        let span = match on.value("span") {
            None => (0.0, 1.0),
            Some(Value::List(xs)) if xs.len() == 2 => {
                let a = text::number(&xs[0].value)?.ok()?;
                let b = text::number(&xs[1].value)?.ok()?;
                if !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b) {
                    self.push(at_path(
                        codes::geom::T_RANGE,
                        "run",
                        id,
                        &["on", "span"],
                        "`span` lies outside 0 to 1",
                        "write two numbers from 0 to 1",
                    ));
                    return None;
                }
                (a, b)
            }
            Some(_) => return None,
        };
        let count = t.value("count").and_then(|v| match v {
            Value::Num(n) => n.norm.parse::<i64>().ok(),
            _ => None,
        });
        let spacing = match t.value("spacing") {
            None => None,
            Some(v) => match text::length(v)? {
                Ok(mm) if mm > 0 => Some(mm),
                Ok(_) => {
                    self.push(at_key(
                        codes::place::RANGE,
                        "run",
                        id,
                        "spacing",
                        "`spacing` is not above 0",
                        "write a length above 0",
                    ));
                    return None;
                }
                Err(e) => {
                    self.push(text::num_diag(&e, "run", id, &["spacing"]));
                    return None;
                }
            },
        };
        // `assets` is the count (`rules.md` 5, R68): no `spacing` beside it, and a `count` beside
        // it says the list's length again.
        if let What::Assets(ids) = &what {
            let n = ids.len() as i64;
            if t.get("spacing").is_some() {
                self.push(at_key(
                    codes::run::COUNT_OR_SPACING,
                    "run",
                    id,
                    "spacing",
                    &format!(
                        "the run writes `spacing` beside `assets`, whose {n} entries are the count"
                    ),
                    "remove `spacing`: a run with `assets` has one member for each entry",
                ));
                return None;
            }
            if n == 0 {
                self.push(at_key(
                    codes::run::COUNT,
                    "run",
                    id,
                    "assets",
                    "`assets` is empty: the run would have no member",
                    "name one asset for each member, or remove the run",
                ));
                return None;
            }
            if let Some(c) = count.filter(|c| *c != n) {
                self.push(
                    at_key(
                        codes::run::COUNT,
                        "run",
                        id,
                        "count",
                        &format!("`count = {c}` beside `assets`, which names {n}"),
                        &format!("write `count = {n}`, or remove `count`: the list is the count"),
                    )
                    .with("count", DValue::Int(c))
                    .with("assets", DValue::Int(n)),
                );
                return None;
            }
        } else if t.get("count").is_some() == t.get("spacing").is_some() {
            self.push(at_header(
                codes::run::COUNT_OR_SPACING,
                "run",
                id,
                if count.is_some() {
                    "the run writes both `count` and `spacing`"
                } else {
                    "the run writes neither `count` nor `spacing`"
                },
                "keep one of `count`, `spacing`",
            ));
            return None;
        }
        let both = match t.str("ends") {
            None | Some("half") => false,
            Some("both") => true,
            Some(w) => {
                self.push(at_key(
                    codes::doc::TYPE,
                    "run",
                    id,
                    "ends",
                    &format!("`{w}` is not a word `ends` takes"),
                    "write `\"half\"` or `\"both\"`",
                ));
                return None;
            }
        };
        // The number of members written: `count`, or the length of `assets`.
        let count = match &what {
            What::Assets(ids) => Some(ids.len() as i64),
            _ => count,
        };
        let counted = if matches!(what, What::Assets(_)) && t.get("count").is_none() {
            "assets"
        } else {
            "count"
        };
        if let Some(c) = count {
            if c < 1 || (both && c < 2 && !self.closed_ref(&place, span)) {
                let least = if both { 2 } else { 1 };
                self.push(if counted == "assets" {
                    at_key(
                        codes::run::COUNT,
                        "run",
                        id,
                        "assets",
                        &format!("`assets` names {c}, too few with `ends = \"both\"`"),
                        &format!("name {least} assets or more, or write `ends = \"half\"`"),
                    )
                } else {
                    at_key(
                        codes::run::COUNT,
                        "run",
                        id,
                        "count",
                        &format!("`count = {c}` is too few"),
                        &format!("write `count = {least}` or more"),
                    )
                });
                return None;
            }
        }
        let face = self.read_face("run", id, t);
        let Some(Ranged::Fixed(scale)) = self.read_scale_tint("run", id, t, "scale", false) else {
            return None;
        };
        let Some(Ranged::Fixed(tint)) = self.read_scale_tint("run", id, t, "tint", false) else {
            return None;
        };
        let ground = t.str("ground").map(str::to_string);
        let margin = match t.value("margin") {
            None => 0,
            Some(v) => match text::length(v) {
                Some(Ok(mm)) if mm >= 0 => mm,
                Some(Ok(_)) => {
                    self.push(at_key(
                        codes::place::RANGE,
                        "run",
                        id,
                        "margin",
                        "`margin` is below 0",
                        "write a length, 0 or more",
                    ));
                    return None;
                }
                _ => 0,
            },
        };
        if t.get("margin").is_some() && ground.is_none() {
            self.push(at_key(
                codes::place::MARGIN_ALONE,
                "run",
                id,
                "margin",
                "`margin` without `ground`",
                "add `ground = \"<kind>\"`, or remove `margin`",
            ));
            return None;
        }
        if let Some(g) = &ground {
            if !self.kinds.contains_key(g) {
                let names: Vec<String> = self.kinds.keys().cloned().collect();
                self.push(text::with_nearest(
                    at_key(
                        codes::authority::UNKNOWN_KIND,
                        "run",
                        id,
                        "ground",
                        &format!("`{g}` names no kind of this world"),
                        &text::near_hint(
                            g,
                            names.iter().map(String::as_str),
                            "include the library that declares it",
                        ),
                    ),
                    g,
                    names.iter().map(String::as_str),
                    text::header_of("kind"),
                ));
                return None;
            }
        }
        Some(RunDecl {
            what,
            place,
            span,
            offset: offset.unwrap_or(0),
            count,
            spacing,
            both,
            face,
            scale,
            tint,
            ground,
            margin,
            rank: world.rank("run", id).unwrap_or(0),
        })
    }

    /// Whether a run's stretch is a whole outline, known from the text alone: `span = [0, 1]` on
    /// a region feature written as points (`rules.md` 5.1). A `where` region or a produced one is
    /// told at placement.
    fn closed_ref(&self, place: &Place, span: (f64, f64)) -> bool {
        span == (0.0, 1.0)
            && match place {
                Place::On(o) => self.features.get(&o.reference).is_some_and(|f| {
                    !matches!(
                        f.shape,
                        crate::decl::ShapeDecl::Point(_) | crate::decl::ShapeDecl::Path(_)
                    )
                }),
                _ => true,
            }
    }
}

/// `round_half_even` of a double, through `via-num` (`numbers.md` 3.1).
fn half_even(x: f64) -> i64 {
    via_num::round_to_lattice(x).unwrap_or(0)
}

impl Placer<'_> {
    /// A set placed at a frame: the placing thing's plan (no asset, no footprint) and every
    /// item's plan, named `<name>.<item>`, held in `out.plans`. `on` is the outermost placing
    /// thing, so the items of one placement are known as one unit (`rules.md` 4).
    #[allow(clippy::too_many_arguments)] // one frame's facts, as `rules.md` 4 lists them
    pub(crate) fn place_set(
        &mut self,
        top: &str,
        id: &str,
        name: &str,
        set: &str,
        origin: P,
        facing: i64,
        scale: f64,
        tint: f64,
        unit: &str,
        rank: i64,
    ) -> Option<Plan> {
        let host = Plan {
            name: name.to_string(),
            asset: String::new(),
            origin,
            facing,
            scale,
            tint,
            footprint: Region::default(),
            on: Some(unit.to_string()).filter(|u| u != name),
            rank,
        };
        let sd = self.d.sets.get(set)?.clone();
        let f = Frame::new((origin.x, origin.y), facing, scale).ok()?;
        for (item, it) in &sd.items {
            let full = format!("{name}.{item}");
            let Ok(w) = f.to_world(it.at) else { continue };
            let p = P::new(w.0, w.1);
            let fa = via_num::normalise_facing(facing + it.face);
            let (sc, ti) = (scale * it.scale, tint * it.tint);
            match &it.what {
                What::Asset(a) => {
                    if let Some(mut pl) = self.plan_at(top, id, a, p, fa, sc, ti, None, rank) {
                        pl.name = full.clone();
                        pl.on = Some(unit.to_string());
                        self.out.plans.insert(full, pl);
                    }
                }
                What::Set(s) => {
                    let s = s.clone();
                    self.place_set(top, id, &full, &s, p, fa, sc, ti, unit, rank);
                }
                // A run's list (`rules.md` 5): an item names one asset or one set.
                What::Assets(_) => {}
            }
        }
        self.out.plans.insert(name.to_string(), host.clone());
        Some(host)
    }

    /// The footprints of a placed set's items, unioned (`rules.md` 2.1: every piece kept).
    pub(crate) fn set_footprint(&self, name: &str) -> Region {
        let prefix = format!("{name}.");
        let parts: Vec<Region> = self
            .out
            .plans
            .range(prefix.clone()..)
            .take_while(|(k, _)| k.starts_with(&prefix))
            .filter(|(_, p)| !p.footprint.is_empty())
            .map(|(_, p)| p.footprint.clone())
            .collect();
        boolean(BoolOp::Union, &parts).unwrap_or_default()
    }

    /// A footprint feature `<name>.ground` of kind `g`, grown by `margin` (`rules.md` 2.1).
    pub(crate) fn ground_feature(
        &mut self,
        top: &str,
        id: &str,
        name: &str,
        fp: &Region,
        g: &str,
        margin: i64,
    ) {
        match offset(id, fp, margin, Join::Mitre(2.0)) {
            Ok((r, _)) => {
                let fname = format!("{name}.ground");
                let line =
                    (r.pieces.len() == 1).then(|| Line::new(r.pieces[0].shell.clone(), true));
                self.out
                    .shapes
                    .insert(fname.clone(), Geo::Region { region: r, line });
                self.out
                    .footprints
                    .insert(fname, (name.to_string(), g.to_string()));
            }
            Err(e) => self
                .diags
                .push(e.to_diag(Subject::entity(top, id), Site::Key(text::path(&["margin"])))),
        }
    }

    /// A run's members (`rules.md` 5.1), as plans `<run>#r<k>`.
    pub(crate) fn run_members(&mut self, id: &str) {
        let Some(r) = self.d.runs.get(id).cloned() else {
            return;
        };
        let spot = match &r.place {
            Place::On(o) => self.spot("run", id, "on", &o.reference),
            Place::Of(s) => self.select("run", id, s),
            Place::At(_) => None,
        };
        let Some(spot) = spot else { return };
        let no_length = |p: &mut Self, why: &str| {
            p.diags.push(at_key(
                codes::run::NO_LENGTH,
                "run",
                id,
                "on",
                &format!("`{}` {why}", r.reference()),
                "name a path or a region, or widen `span`",
            ));
        };
        let (line, closed) = match spot {
            Spot::Long(Geo::Path { line }) => (line, false),
            Spot::Long(Geo::Region { line: Some(l), .. }) => (l, true),
            Spot::Long(_) => {
                no_length(
                    self,
                    "is a region of no single outline, so it has no length",
                );
                return;
            }
            Spot::At { .. } => {
                no_length(self, "has no length");
                return;
            }
        };
        // 5.1 steps 1 and 2, in integer millimetres.
        let lmm = half_even(line.length());
        let a = half_even(r.span.0 * lmm as f64);
        let b = half_even(r.span.1 * lmm as f64);
        let len = b - a;
        if len <= 0 {
            no_length(self, "gives a stretch of zero millimetres");
            return;
        }
        let whole = closed && r.span == (0.0, 1.0);
        if r.both && !whole && r.count.is_some_and(|c| c < 2) {
            let listed = matches!(r.what, What::Assets(_));
            self.diags.push(at_key(
                codes::run::COUNT,
                "run",
                id,
                if listed { "assets" } else { "count" },
                "`ends = \"both\"` on an open stretch needs 2 members",
                if listed {
                    "name 2 assets or more"
                } else {
                    "write `count = 2` or more"
                },
            ));
            return;
        }
        // Each entry of `assets` is an asset the pack binds (`rules.md` 5, R68): one it does not
        // is said at that entry, by its index, whether or not its member would be made.
        if let (What::Assets(ids), false) = (&r.what, self.pack.failed) {
            let mut unknown = false;
            for (k, a) in ids.iter().enumerate() {
                if self.pack.assets.contains_key(a) {
                    continue;
                }
                unknown = true;
                let site = Site::Key(text::path(&["assets"]).index(k as u32));
                self.diags.push(
                    text::with_nearest(
                        text::diag(
                            codes::asset::UNKNOWN,
                            Subject::entity("run", id),
                            site,
                            &format!("the pack binds no asset `{a}`, entry {k} of `assets`"),
                            &text::near_hint(
                                a,
                                self.pack.assets.keys().map(String::as_str),
                                "name an asset the pack declares",
                            ),
                        ),
                        a,
                        self.pack.assets.keys().map(String::as_str),
                        |n: &str| self.pack.declaration("asset", n),
                    )
                    .with("index", DValue::Int(k as i64)),
                );
            }
            if unknown {
                return;
            }
        }
        let rounded = |sp: i64| (2 * len + sp).div_euclid(2 * sp);
        let n = match (r.count, r.spacing) {
            (Some(c), _) => c,
            (None, Some(sp)) if r.both && !whole => 2.max(rounded(sp) + 1),
            (None, Some(sp)) => 1.max(rounded(sp)),
            (None, None) => return,
        };
        let s_of = |k: i64| -> i64 {
            let num = if !r.both {
                i128::from(2 * k + 1) * i128::from(len) / i128::from(2 * n)
            } else if whole {
                i128::from(k) * i128::from(len) / i128::from(n)
            } else {
                i128::from(k) * i128::from(len) / i128::from(n - 1)
            };
            a + num as i64
        };
        let [x0, y0, x1, y1] = self
            .d
            .grid
            .map_or([i64::MIN, i64::MIN, i64::MAX, i64::MAX], |g| g.extent);
        let mut outside = 0;
        for k in 0..n {
            let s = s_of(k);
            let Some((p, bearing, on_line)) = point_at(&line, s, r.offset) else {
                continue;
            };
            if p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1 {
                outside += 1;
                continue;
            }
            let facing = match r.face {
                None | Some(Face::Along) => bearing,
                Some(Face::Bearing(b)) => b,
                Some(w) => {
                    let Some(b) = via_num::bearing(on_line.x - p.x, on_line.y - p.y) else {
                        self.diags.push(at_key(
                            codes::geom::FACE_SIDE,
                            "run",
                            id,
                            "face",
                            "`toward` and `away` need an offset to face across",
                            "write a bearing, or give `offset`",
                        ));
                        return;
                    };
                    if w == Face::Away {
                        via_num::normalise_facing(b + 180_000)
                    } else {
                        b
                    }
                }
            };
            let name = format!("{id}#r{k}");
            // What stands at station `k`: the run's one asset, or the entry of `assets` at
            // index `k` (`rules.md` 5, R68), whatever became of the stations before it.
            let one = match &r.what {
                What::Asset(asset) => Some(asset.as_str()),
                What::Assets(ids) => ids.get(k as usize).map(String::as_str),
                What::Set(_) => None,
            };
            let plan = match (&r.what, one) {
                (_, Some(asset)) => self
                    .plan_at(
                        "run",
                        id,
                        asset,
                        p,
                        facing,
                        r.scale,
                        r.tint,
                        Some(r.reference().to_string()),
                        r.rank,
                    )
                    .map(|mut pl| {
                        pl.name = name.clone();
                        self.out.plans.insert(name.clone(), pl.clone());
                        pl
                    }),
                (What::Set(s), _) => {
                    let s = s.clone();
                    self.place_set(
                        "run", id, &name, &s, p, facing, r.scale, r.tint, &name, r.rank,
                    )
                }
                _ => None,
            };
            let Some(plan) = plan else { continue };
            if let Some(g) = &r.ground {
                let fp = if plan.asset.is_empty() {
                    self.set_footprint(&name)
                } else {
                    plan.footprint.clone()
                };
                if !fp.is_empty() {
                    self.ground_feature("run", id, &name, &fp, g, r.margin);
                }
            }
        }
        if outside > 0 {
            self.diags.push(
                at_header(
                    codes::run::OUTSIDE,
                    "run",
                    id,
                    &format!("{outside} members fell outside the extent and were not made"),
                    "narrow `span` to the part inside the extent",
                )
                .with("count", DValue::Int(outside)),
            );
        }
    }
}

/// The point at arc length `s` mm (`rules.md` 5.1): the segment with `c_k <= s < c_(k+1)`, or the
/// last; the fraction capped at 1; moved `offset` along the segment's left normal; rounded once.
/// Gives the point, the segment's bearing and the point on the line itself.
fn point_at(line: &Line, s: i64, off: i64) -> Option<(P, i64, P)> {
    let s = s as f64;
    let k = lineref::holding(line, s, false);
    let (a, b) = line.seg(k);
    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let lk = line.lens[k];
    let f = via_num::min((s - line.cum[k]) / lk, 1.0);
    let (qx, qy) = (a.x as f64 + f * dx, a.y as f64 + f * dy);
    let (nx, ny) = (-dy / lk, dx / lk);
    let o = off as f64;
    let p = P::new(
        lineref::round("run", qx + o * nx).ok()?,
        lineref::round("run", qy + o * ny).ok()?,
    );
    let q = P::new(
        lineref::round("run", qx).ok()?,
        lineref::round("run", qy).ok()?,
    );
    Some((p, via_num::bearing(b.x - a.x, b.y - a.y).unwrap_or(0), q))
}

/// A run's members give way (`rules.md` 3.4, 5): a member, or an item of it, whose solid
/// footprint overlaps a solid hand instance of the run's layer or below is not placed, with its
/// items; counted in one `place.gave_way` per run. Neighbours of one run are never tested.
///
/// A run in a step's returned document does not give way to an instance of that same step's
/// document (3.4, R64): the step wrote both at once, so neither is the firmer. Their overlap is
/// `place.crowded`, said with the checks ([`crate::rules::ground_and_check`]).
pub(crate) fn runs_give_way(
    d: &Decls,
    pack: &crate::pack::Pack,
    shapes: &BTreeMap<String, crate::place::Geo>,
    all: &mut Vec<crate::rules::Placement>,
    diags: &mut Vec<via_diag::Diag>,
) {
    if d.runs.is_empty() {
        return;
    }
    let levels = crate::surfaces::Levels::new(d, shapes);
    let solid = |a: &str| pack.assets.get(a).is_some_and(|x| x.solid);
    let hands: Vec<&crate::rules::Placement> = all
        .iter()
        .filter(|p| p.maker.is_none() && !p.pin && solid(&p.asset))
        .collect();
    // Per run: the members that gave way, and the box of what gave way.
    let mut gone: BTreeMap<String, (BTreeSet<String>, P, P)> = BTreeMap::new();
    for p in all.iter() {
        let Some(run) = p.maker.as_deref().filter(|m| d.runs.contains_key(*m)) else {
            continue;
        };
        if p.pin || !solid(&p.asset) {
            continue;
        }
        let rank = d.runs[run].rank;
        let own = d.step_of(run);
        let hit = hands.iter().any(|h| {
            let hrank = d
                .instances
                .get(h.name.split('.').next().unwrap_or(&h.name))
                .map_or(0, |i| i.rank);
            hrank <= rank
                && (own.is_none() || d.step_of(&h.name) != own)
                && levels.clash(
                    &crate::surfaces::level(d, pack, p),
                    &p.footprint,
                    &crate::surfaces::level(d, pack, h),
                    &h.footprint,
                )
                && crate::rules::overlaps(&p.footprint, &h.footprint)
        });
        if !hit {
            continue;
        }
        let member = match p.name.split_once('.') {
            Some((m, _)) => m.to_string(),
            None => p.name.clone(),
        };
        let (lo, hi) = p.footprint.bbox().unwrap_or((p.origin, p.origin));
        let e = gone
            .entry(run.to_string())
            .or_insert((BTreeSet::new(), lo, hi));
        e.0.insert(member);
        e.1 = P::new(e.1.x.min(lo.x), e.1.y.min(lo.y));
        e.2 = P::new(e.2.x.max(hi.x), e.2.y.max(hi.y));
    }
    if gone.is_empty() {
        return;
    }
    all.retain(|p| {
        let Some(run) = p.maker.as_deref() else {
            return true;
        };
        let member = p.name.split_once('.').map_or(p.name.as_str(), |(m, _)| m);
        !gone.get(run).is_some_and(|g| g.0.contains(member))
    });
    for (run, (members, a, b)) in gone {
        let n = members.len() as i64;
        diags.push(
            at_header(
                codes::place::GAVE_WAY,
                "run",
                &run,
                &format!("{n} members gave way: {n} by overlap, 0 by relief"),
                "widen the space or change the hand instances it gives way to; nothing if intended",
            )
            .at(text::pos(a.x, a.y))
            .with("overlap", DValue::Int(n))
            .with("relief", DValue::Int(0))
            .with(
                "box",
                DValue::List(vec![
                    DValue::Length(a.x),
                    DValue::Length(a.y),
                    DValue::Length(b.x),
                    DValue::Length(b.y),
                ]),
            ),
        );
    }
}
