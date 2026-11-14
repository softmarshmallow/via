//! Resolving fields (`fields.md` 5 and 6, `authority.md` 2 to 5): a field's base, then every
//! entry in the one order of R21 and R29, in doubles, narrowed to 32-bit floats once.
//!
//! A field is resolved when something first reads it, once per view, and kept. Formulas are
//! evaluated by `via-expr` over a window of posts (a base, a row) or over the posts within an
//! effect's radius (`fields.md` 5.3), reading other fields, the served feature's `d`, `sd`, `t`
//! and attributes, and `v` through [`Ctx`], this crate's [`via_expr::Reads`]. A field met again
//! while it is being resolved is a cycle between fields, `graph.cycle`.
//!
//! Views (`fields.md` 6.2): [`View::Build`], every entry; [`View::Head`], the base and every row
//! and no effect (what a shape reads, `authority.md` 2.3); [`View::NoFootprint`], the build's
//! view less footprint features' effects (the `keep_out` test of `rules.md` 8.3).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use via_diag::{codes, Diag, Site, Subject, Value as DValue};
use via_expr::{
    bind, evaluate, Binding, Domain, FieldInfo, FormulaId, Measure, ReadError, Reads, Scalar,
    Select, Setup, Shape, ShapeInfo, Stat,
};
use via_geom::P;
use via_num::Grid;
use via_raster::raster::read_at;
use via_raster::{Raster, Window as RWindow};

use crate::decl::{Attr, Decls, Role, Selector, Tier};
use crate::place::{Geo, Placed};
use crate::text::{self, at_path};

/// The views a field is resolved in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    Build,
    Head,
    NoFootprint,
    /// The head view a generate step of layer rank `r` reads (`protocol.md` 1, its stratum): the
    /// base and the rows of rank `r` or below, without the rows a step of rank `r` or above
    /// returned (`<step>#<row>`), and no effect.
    Stratum(i64),
}

/// A resolved field: one raster over the whole grid, or one per painted material.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub scalar: Option<Raster<f32>>,
    pub weights: BTreeMap<String, Raster<f32>>,
    /// Which features' effects it holds, and their tiers: for the change report's causes.
    pub writers: BTreeSet<String>,
    /// The fields its formulas read.
    pub reads: BTreeSet<String>,
}

/// A feature that writes fields: an authored feature with a kind, or a footprint feature.
#[derive(Clone, Debug)]
pub struct Writer {
    pub name: String,
    pub kind: String,
    pub tier: Tier,
    pub rank: i64,
    /// The written-relation links: (other, true when the other comes later).
    pub links: Vec<(String, bool, &'static str)>,
    pub attrs: BTreeMap<String, Attr>,
    /// True for a footprint feature.
    pub footprint: bool,
}

/// One entry on a field (`authority.md` 4).
#[derive(Clone, Debug)]
enum Entry {
    Row(usize),
    Effect { writer: usize, effect: String },
}

/// What one anchored feature left on one field: for `authority.md` 5.
#[derive(Clone, Debug, Default)]
struct Anchor {
    writer: usize,
    /// Per entry: post index to (value, mask).
    entries: Vec<BTreeMap<usize, (f64, f64)>>,
    /// The field's value just after its last entry, at each post it touched.
    after: BTreeMap<usize, f64>,
}

/// The served feature of an effect, and its local measures at the domain's points.
struct Served<'a> {
    name: &'a str,
    geo: &'a Geo,
    attrs: &'a BTreeMap<String, Attr>,
    local: &'a [lineref_cols::Cols],
}

mod lineref_cols {
    #[derive(Clone, Copy, Debug)]
    pub struct Cols {
        pub d: f64,
        pub sd: f64,
        pub t: f64,
    }
}

/// What each applied effect touched, and at each post the last entry that set it with mask 1:
/// for `authority.hidden`.
#[derive(Default)]
struct Hidden {
    /// (entry index, writer, posts with mask above 0).
    entries: Vec<(usize, usize, Vec<usize>)>,
    /// Post to (entry index, writer) of the last entry with mask exactly 1 there.
    last1: BTreeMap<usize, (usize, usize)>,
}

impl Hidden {
    /// `erases`: the entry's value does not read the field before it (`v`, one-argument `under`,
    /// `smoothed_along`), so where its mask is 1 nothing earlier survives. An entry that reads `v`
    /// carries the earlier value on and covers nothing.
    fn track(&mut self, idx: usize, writer: usize, posts: &[usize], mask: &[f64], erases: bool) {
        let mut mine = Vec::new();
        for (k, &p) in posts.iter().enumerate() {
            if mask[k] > 0.0 {
                mine.push(p);
            }
            if erases && mask[k] == 1.0 {
                self.last1.insert(p, (idx, writer));
            }
        }
        self.entries.push((idx, writer, mine));
    }
}

/// Resolved fields by (name, view); `None` for one that failed.
type Cache = BTreeMap<(String, View), Option<Rc<Resolved>>>;

/// The resolver: fields by (name, view), kept once made.
pub struct Fields<'a> {
    pub d: &'a Decls,
    pub placed: &'a Placed,
    pub grid: Grid,
    pub writers: Vec<Writer>,
    cache: RefCell<Cache>,
    stack: RefCell<Vec<(String, View)>>,
    pub diags: RefCell<Vec<Diag>>,
    keep_out: RefCell<BTreeMap<View, Rc<Raster<f64>>>>,
    reported: RefCell<BTreeSet<String>>,
    /// Indices into `diags` of records a later evaluation reached again and added a reader to,
    /// in order: so a caller that gives each field its own records gives them to every reader.
    pub touched: RefCell<Vec<usize>>,
}

fn post_index(grid: &Grid, i: u64, j: u64) -> usize {
    (j * grid.nx + i) as usize
}

fn post_of(grid: &Grid, k: usize) -> (u64, u64) {
    (k as u64 % grid.nx, k as u64 / grid.nx)
}

impl<'a> Fields<'a> {
    pub fn new(d: &'a Decls, placed: &'a Placed, grid: Grid) -> Fields<'a> {
        let mut writers = Vec::new();
        for (id, f) in &d.features {
            let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) else {
                continue;
            };
            if !placed.shapes.contains_key(id) {
                continue;
            }
            writers.push(Writer {
                name: id.clone(),
                kind: k.id.clone(),
                tier: f.tier.or(k.authority).unwrap_or(Tier::Derived),
                rank: f.rank,
                links: Vec::new(),
                attrs: f.attrs.clone(),
                footprint: false,
            });
        }
        for (name, (inst, kind)) in &placed.footprints {
            let Some(k) = d.kinds.get(kind) else { continue };
            writers.push(Writer {
                name: name.clone(),
                kind: kind.clone(),
                tier: k.authority.unwrap_or(Tier::Derived),
                rank: d.instances.get(inst).map_or(0, |i| i.rank),
                links: Vec::new(),
                attrs: k.defaults.clone(),
                footprint: true,
            });
        }
        writers.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        let mut f = Fields {
            d,
            placed,
            grid,
            writers,
            cache: RefCell::new(BTreeMap::new()),
            stack: RefCell::new(Vec::new()),
            diags: RefCell::new(Vec::new()),
            keep_out: RefCell::new(BTreeMap::new()),
            reported: RefCell::new(BTreeSet::new()),
            touched: RefCell::new(Vec::new()),
        };
        f.link();
        f
    }

    /// The written nesting relations (`authority.md` 4.3): stands on, arrives at.
    fn link(&mut self) {
        let d = self.d;
        let names: BTreeSet<String> = self.writers.iter().map(|w| w.name.clone()).collect();
        // The writer a place name refers to: a feature, an end of it, an instance's footprint.
        let target = |r: &str| -> Option<String> {
            let base = r
                .strip_suffix(".start")
                .or_else(|| r.strip_suffix(".end"))
                .unwrap_or(r);
            if names.contains(base) {
                return Some(base.to_string());
            }
            let host = base.split('.').next().unwrap_or(base);
            let fp = format!("{host}.ground");
            names.contains(&fp).then_some(fp)
        };
        let mut links: BTreeMap<String, Vec<(String, bool, &'static str)>> = BTreeMap::new();
        for w in &self.writers {
            let mut v = Vec::new();
            if w.footprint {
                let inst = w.name.trim_end_matches(".ground");
                match d.instances.get(inst).map(|i| &i.place) {
                    Some(crate::decl::Place::On(o)) => {
                        if let Some(t) = target(&o.reference) {
                            v.push((t, false, "on.ref"));
                        }
                    }
                    Some(crate::decl::Place::Of(o)) => {
                        if let Some(t) = target(&o.of) {
                            v.push((t, false, "on.of"));
                        }
                    }
                    _ => {}
                }
            } else if let Some(f) = d.features.get(&w.name) {
                match &f.shape {
                    crate::decl::ShapeDecl::Point(crate::decl::Place::On(o)) => {
                        if let Some(t) = target(&o.reference) {
                            v.push((t, false, "on.ref"));
                        }
                    }
                    crate::decl::ShapeDecl::Point(crate::decl::Place::Of(o)) => {
                        if let Some(t) = target(&o.of) {
                            v.push((t, false, "on.of"));
                        }
                    }
                    crate::decl::ShapeDecl::Follows(fl) => {
                        if let Some(t) = target(&fl.reference) {
                            v.push((t, false, "follows.ref"));
                        }
                    }
                    _ => {}
                }
                for (key, e) in [("starts_at", &f.starts_at), ("ends_at", &f.ends_at)] {
                    if let Some(t) = e.as_deref().and_then(target) {
                        v.push((t, true, key));
                    }
                }
            }
            v.retain(|(t, _, _)| t != &w.name);
            links.insert(w.name.clone(), v);
        }
        for w in &mut self.writers {
            w.links = links.remove(&w.name).unwrap_or_default();
        }
    }

    fn push(&self, d: Diag) {
        self.diags.borrow_mut().push(d);
    }

    fn matches(&self, sel: &Selector, field: &str) -> bool {
        let Some(f) = self.d.fields.get(field) else {
            return false;
        };
        match sel {
            Selector::Role(r) => f.role == Some(*r),
            Selector::Tag(t) => f.tags.iter().any(|x| x == t),
        }
    }

    /// The fields an effect's selector matches.
    pub fn matching(&self, sel: &Selector) -> Vec<String> {
        self.d
            .fields
            .keys()
            .filter(|f| self.matches(sel, f))
            .cloned()
            .collect()
    }

    /// `authority.no_field`, once per (kind, effect) with a feature in the build.
    pub fn report_no_field(&self) {
        let used: BTreeSet<&str> = self.writers.iter().map(|w| w.kind.as_str()).collect();
        // A selector is checked at each feature's rank (`fields.md` 2.3): a field of a higher
        // layer is `doc.upward_ref`.
        for w in &self.writers {
            let k = &self.d.kinds[&w.kind];
            for e in k.effects.values() {
                for f in self.matching(&e.selector) {
                    if self.d.fields[&f].rank > w.rank {
                        let (top, id) = writer_subject(self.d, &w.name);
                        self.push(text::at_header(codes::doc::UPWARD_REF, top, &id, &format!("`{}` (rank {}) writes `{f}`, declared in a higher layer (rank {}) through its kind's effect `{}`", w.name, w.rank, self.d.fields[&f].rank, e.id), "declare the field in this layer or below, or move the feature up"));
                    }
                }
            }
        }
        for k in self.d.kinds.values() {
            if !used.contains(k.id.as_str()) {
                continue;
            }
            for e in k.effects.values() {
                // `fields.md` 5: `value` on a weights entry, `paint` on a scalar one.
                for f in self.matching(&e.selector) {
                    let weights = self.d.fields[&f].weights;
                    let (bad, key) = if weights {
                        (e.value.is_some(), "value")
                    } else {
                        (e.paint.is_some() && e.value.is_none(), "paint")
                    };
                    if bad {
                        self.push(at_path(
                            codes::field::ROW_KEY,
                            "kind",
                            &k.id,
                            &["effect", &e.id, key],
                            &format!(
                                "the effect `{}` of `{}` writes `{key}` into the {} field `{f}`",
                                e.id,
                                k.id,
                                if weights { "weights" } else { "scalar" }
                            ),
                            if weights {
                                "a weights field takes `paint`; select the fields apart by tag"
                            } else {
                                "a scalar field takes `value`; select the fields apart by tag"
                            },
                        ));
                    }
                }
                if self.matching(&e.selector).is_empty() {
                    let tags: Vec<&str> = self
                        .d
                        .fields
                        .values()
                        .flat_map(|f| f.tags.iter().map(String::as_str))
                        .collect();
                    let written = match &e.selector {
                        Selector::Tag(t) => t.clone(),
                        Selector::Role(r) => r.word().to_string(),
                    };
                    self.push(at_path(
                        codes::authority::NO_FIELD,
                        "kind",
                        &k.id,
                        &["effect", &e.id, "field"],
                        &format!(
                            "the effect `{}` of `{}` writes no field: none matches its selector",
                            e.id, k.id
                        ),
                        &text::near_hint(
                            &written,
                            tags,
                            "add the role or tag to a field, or drop the effect",
                        ),
                    ));
                }
            }
        }
    }

    fn ordered(&self, field: &str, view: View) -> (Vec<Entry>, Vec<Diag>) {
        let fd = &self.d.fields[field];
        let mut rows: Vec<usize> = (0..fd.rows.len()).collect();
        rows.sort_by(|a, b| {
            let (ra, rb) = (&fd.rows[*a], &fd.rows[*b]);
            (ra.order, ra.rank, ra.id.as_bytes()).cmp(&(rb.order, rb.rank, rb.id.as_bytes()))
        });
        if let View::Stratum(r) = view {
            rows.retain(|i| {
                let row = &fd.rows[*i];
                row.rank <= r && !(row.id.contains('#') && row.rank >= r)
            });
        }
        let mut out: Vec<Entry> = rows.into_iter().map(Entry::Row).collect();
        let mut diags = Vec::new();
        if matches!(view, View::Head | View::Stratum(_)) {
            return (out, diags);
        }
        // Effects, by tier, then nesting, then (order, rank, place name, effect id).
        for tier in [Tier::Derived, Tier::Shaped, Tier::Anchored] {
            let mut entries: Vec<(usize, String, i64)> = Vec::new();
            for (wi, w) in self.writers.iter().enumerate() {
                if w.tier != tier || (view == View::NoFootprint && w.footprint) {
                    continue;
                }
                let k = &self.d.kinds[&w.kind];
                for e in k.effects.values() {
                    if self.matches(&e.selector, field) {
                        entries.push((wi, e.id.clone(), e.order));
                    }
                }
            }
            if entries.is_empty() {
                continue;
            }
            let present: BTreeSet<usize> = entries.iter().map(|e| e.0).collect();
            // Feature-level edges: before -> after.
            let mut edges: BTreeSet<(usize, usize, &'static str)> = BTreeSet::new();
            let index: BTreeMap<&str, usize> = self
                .writers
                .iter()
                .enumerate()
                .map(|(i, w)| (w.name.as_str(), i))
                .collect();
            let mut linked: BTreeSet<(usize, usize)> = BTreeSet::new();
            for &b in &present {
                for (other, other_later, key) in &self.writers[b].links {
                    let Some(&a) = index.get(other.as_str()) else {
                        continue;
                    };
                    if !present.contains(&a) {
                        continue;
                    }
                    let (first, later) = if *other_later { (b, a) } else { (a, b) };
                    edges.insert((first, later, key));
                    linked.insert((a.min(b), a.max(b)));
                }
            }
            // Containment, only between features whose shapes' boxes meet.
            let pv: Vec<usize> = present.iter().copied().collect();
            for (x, &a) in pv.iter().enumerate() {
                for &b in &pv[x + 1..] {
                    if linked.contains(&(a.min(b), a.max(b))) {
                        continue;
                    }
                    let (ga, gb) = (
                        &self.placed.shapes[&self.writers[a].name],
                        &self.placed.shapes[&self.writers[b].name],
                    );
                    if inside(gb, ga) {
                        edges.insert((a, b, "inside"));
                    } else if inside(ga, gb) {
                        edges.insert((b, a, "inside"));
                    }
                }
            }
            let mut preds: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
            for (a, b, _) in &edges {
                preds.entry(*b).or_default().insert(*a);
            }
            let mut remaining: BTreeMap<usize, usize> = BTreeMap::new();
            for e in &entries {
                *remaining.entry(e.0).or_default() += 1;
            }
            let mut left: Vec<(usize, String, i64)> = entries;
            while !left.is_empty() {
                let ready = |w: usize| {
                    preds.get(&w).is_none_or(|ps| {
                        ps.iter()
                            .all(|p| remaining.get(p).copied().unwrap_or(0) == 0)
                    })
                };
                let pick = left
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| ready(e.0))
                    .min_by(|(_, x), (_, y)| {
                        let (wx, wy) = (&self.writers[x.0], &self.writers[y.0]);
                        (x.2, wx.rank, wx.name.as_bytes(), x.1.as_bytes()).cmp(&(
                            y.2,
                            wy.rank,
                            wy.name.as_bytes(),
                            y.1.as_bytes(),
                        ))
                    })
                    .map(|(i, _)| i);
                match pick {
                    Some(i) => {
                        let e = left.remove(i);
                        *remaining.get_mut(&e.0).unwrap_or(&mut 0) -= 1;
                        out.push(Entry::Effect {
                            writer: e.0,
                            effect: e.1,
                        });
                    }
                    None => {
                        let stuck: BTreeSet<usize> = left.iter().map(|e| e.0).collect();
                        let names: Vec<&str> = stuck
                            .iter()
                            .map(|w| self.writers[*w].name.as_str())
                            .collect();
                        let link_words: Vec<String> = edges
                            .iter()
                            .filter(|(a, b, _)| stuck.contains(a) && stuck.contains(b))
                            .map(|(a, b, k)| {
                                format!(
                                    "`{}` before `{}` by {k}",
                                    self.writers[*a].name, self.writers[*b].name
                                )
                            })
                            .collect();
                        let first = names.first().copied().unwrap_or("");
                        let top = if self.d.features.contains_key(first) {
                            "feature"
                        } else {
                            "instance"
                        };
                        diags.push(
                            text::diag(
                                codes::authority::ORDER_CYCLE,
                                Subject::entity(top, first.trim_end_matches(".ground")),
                                Site::Header,
                                &format!(
                                    "the entries on `{field}` cannot be ordered: {}",
                                    link_words.join("; ")
                                ),
                                "state the meeting once",
                            )
                            .with("field", DValue::text(field))
                            .with("features", DValue::texts(names.iter().copied())),
                        );
                        // Apply the rest by key alone, so the build still shows something.
                        left.sort_by(|x, y| {
                            (x.2, self.writers[x.0].name.as_bytes(), x.1.as_bytes()).cmp(&(
                                y.2,
                                self.writers[y.0].name.as_bytes(),
                                y.1.as_bytes(),
                            ))
                        });
                        for e in left.drain(..) {
                            out.push(Entry::Effect {
                                writer: e.0,
                                effect: e.1,
                            });
                        }
                    }
                }
            }
        }
        (out, diags)
    }

    /// Takes a field resolved elsewhere (a graph node's value) as this view's.
    pub fn seed(&self, field: &str, view: View, r: Option<Resolved>) {
        self.cache
            .borrow_mut()
            .insert((field.to_string(), view), r.map(Rc::new));
    }

    /// A field in a view, resolving it first if needed. `None` after a fault, reported once.
    pub fn get(&self, field: &str, view: View) -> Option<Rc<Resolved>> {
        let key = (field.to_string(), view);
        if let Some(r) = self.cache.borrow().get(&key) {
            return r.clone();
        }
        if self.stack.borrow().contains(&key) {
            let members: Vec<String> = {
                let s = self.stack.borrow();
                let at = s.iter().position(|k| k == &key).unwrap_or(0);
                s[at..].iter().map(|(f, _)| f.clone()).collect()
            };
            let mut sorted = members.clone();
            sorted.sort();
            // One loop, one record (`graph.md` 2): written from its least member, whichever
            // member's node found it.
            let mut members = members;
            if let Some(at) = members.iter().position(|m| *m == sorted[0]) {
                members.rotate_left(at);
            }
            let tag = sorted.join(",");
            if self.reported.borrow_mut().insert(format!("cycle:{tag}")) {
                // At the key that makes the edge (`graph.md` 2): the base when it is a formula.
                let site = match self.d.fields.get(&sorted[0]) {
                    Some(fd) if fd.base.is_some() => Site::Key(via_diag::KeyPath::key("base")),
                    _ => Site::Header,
                };
                self.push(text::diag(
                    codes::graph::CYCLE,
                    Subject::entity("field", sorted[0].clone()),
                    site,
                    &format!(
                        "fields read each other in a loop: {} -> {}",
                        members.join(" -> "),
                        members[0]
                    ),
                    "break the loop: one of these fields must not read the next",
                ));
            }
            return None;
        }
        self.stack.borrow_mut().push(key.clone());
        let r = self.resolve(field, view).map(Rc::new);
        self.stack.borrow_mut().pop();
        self.cache.borrow_mut().insert(key, r.clone());
        r
    }

    fn whole(&self) -> RWindow {
        RWindow::whole(&self.grid)
    }

    fn report(&self, view: View, d: Diag) {
        // A fault is the build's to report; another view reports errors only (graph.md 3.4).
        if view == View::Build || d.severity() == via_diag::Severity::Error {
            // A fault a kind's effect finds is reported once per feature it finds it on
            // (`authority.md` 2.2: once per effect, feature and field), so the feature it names
            // is part of what makes two records the same.
            let per_feature = [codes::authority::MAX_CHANGE, codes::field::EFFECT_CUT]
                .iter()
                .any(|c| *d.code() == via_diag::DiagCode::Core(*c));
            let feature = d.data().get("feature").filter(|_| per_feature);
            let tag = format!("{:?}{:?}{:?}{:?}", d.code(), d.subject(), d.site(), feature);
            if self.reported.borrow_mut().insert(tag) {
                self.push(d);
            } else {
                // The same fault reached again: one location, every reader in `related`
                // (`formulas.md` 9, a fault inside a `let` read by two formulas).
                let mut diags = self.diags.borrow_mut();
                let same = diags.iter_mut().enumerate().find(|(_, o)| {
                    o.code() == d.code()
                        && o.subject() == d.subject()
                        && o.site() == d.site()
                        && o.data().get("feature").filter(|_| per_feature) == feature
                });
                if let Some((i, o)) = same {
                    for r in d.related_list() {
                        if !o.related_list().contains(r) {
                            *o = o.clone().related(r.clone());
                        }
                    }
                    self.touched.borrow_mut().push(i);
                }
            }
        }
    }

    fn binding(&self, f: FormulaId, served: Option<&str>) -> Option<Binding> {
        let p = self.d.formulas.programs.get(&f)?;
        // A feature that was declared but not placed said why (a cycle, a bad shape); a formula
        // reading it says nothing more (`graph.md` 3.4).
        let unplaced = p.reads.selectors.iter().any(|(c, sel)| {
            *c == via_expr::Class::Shape
                && !sel.starts_with("kind:")
                && self.d.features.contains_key(sel.as_str())
                && !self.placed.shapes.contains_key(sel.as_str())
        });
        if unplaced {
            return None;
        }
        let sel = self.selector(f, served);
        match bind(p, &sel) {
            Ok(b) => {
                for w in &b.warnings {
                    self.report(View::Build, self.d.formulas.diag(w));
                }
                Some(b)
            }
            Err(e) => {
                self.report(View::Build, self.d.formulas.diag(&e));
                None
            }
        }
    }

    /// The selection phase for formula `f`, served to `served` (a kind's formula) or not. A read
    /// selector sees the reader's rank and below (`documents.md` 5.1, `formulas.md` 7): a kind's
    /// formula reads at the rank of the feature it serves.
    pub(crate) fn selector<'s>(&'s self, f: FormulaId, served: Option<&'s str>) -> Sel<'s, 'a> {
        let rank = match served {
            Some(s) => self.writers.iter().find(|w| w.name == s).map(|w| w.rank),
            None => self.d.formula_rank.get(&f).copied(),
        };
        Sel {
            fields: self,
            own: served,
            rank,
        }
    }

    /// The rank of a placed thing a shape selector names: a feature's, a footprint's instance's.
    fn shape_rank(&self, id: &str) -> i64 {
        let base = id
            .strip_suffix(".start")
            .or_else(|| id.strip_suffix(".end"))
            .unwrap_or(id);
        if let Some(w) = self.writers.iter().find(|w| w.name == base) {
            return w.rank;
        }
        if let Some(f) = self.d.features.get(base) {
            return f.rank;
        }
        let host = base.split('.').next().unwrap_or(base);
        self.d.instances.get(host).map_or(0, |i| i.rank)
    }

    /// Evaluates a formula over a domain; a failure is reported and gives `None`.
    fn eval(
        &self,
        f: FormulaId,
        ctx: &Ctx<'_>,
        domain: &Domain<'_>,
        view: View,
        served: Option<&str>,
    ) -> Option<Vec<f64>> {
        let p = self.d.formulas.programs.get(&f)?;
        let b = self.binding(f, served)?;
        let setup = Setup {
            world_seed: self.d.seed,
            feature: served,
        };
        match evaluate(p, &b, ctx, &setup, domain) {
            Ok(v) => Some(v),
            Err(fail) if fail.error.code == codes::diag::SKIPPED => None,
            Err(fail) => {
                let mut d = self.d.formulas.diag(&fail.error);
                if let Some(s) = served {
                    d = d.with("feature", DValue::text(s));
                }
                self.report(view, d);
                None
            }
        }
    }

    /// A feature's `where` at every post, in the head view (`formulas.md` 6.6, `authority.md`
    /// 2.3): its base and rows, never an effect. `None` after a fault, reported.
    pub fn eval_where(&self, formula: FormulaId, feature: &str) -> Option<Vec<f64>> {
        let attrs = self.d.features.get(feature).map(|f| &f.attrs);
        let ctx = Ctx {
            f: self,
            view: View::Head,
            current: None,
            served: None,
            reads: RefCell::new(BTreeSet::new()),
            own: attrs,
            own_id: Some(feature),
        };
        let ww = via_expr::Window::whole(&self.grid);
        let wdom = Domain::Window {
            grid: self.grid,
            window: ww,
        };
        let p = self.d.formulas.programs.get(&formula)?;
        let b = self.binding(formula, None)?;
        let setup = Setup {
            world_seed: self.d.seed,
            feature: Some(feature),
        };
        match evaluate(p, &b, &ctx, &setup, &wdom) {
            Ok(v) => Some(v),
            Err(fail) if fail.error.code == codes::diag::SKIPPED => None,
            Err(fail) => {
                let d = self.d.formulas.diag(&fail.error);
                self.report(View::Build, d);
                None
            }
        }
    }

    fn resolve(&self, field: &str, view: View) -> Option<Resolved> {
        let fd = self.d.fields.get(field)?;
        let grid = self.grid;
        let n = grid.len() as usize;
        let mut reads: BTreeSet<String> = BTreeSet::new();
        let mut writers: BTreeSet<String> = BTreeSet::new();
        let (entries, order_diags) = self.ordered(field, view);
        for d in order_diags {
            self.report(view, d);
        }
        let ww = via_expr::Window::whole(&grid);
        let wdom = Domain::Window { grid, window: ww };
        let mut cur: Vec<f64> = vec![0.0; n];
        let mut weights: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        if let Some(b) = fd.base {
            let ctx = Ctx {
                f: self,
                view,
                current: None,
                served: None,
                reads: RefCell::new(BTreeSet::new()),
                own: None,
                own_id: None,
            };
            cur = self.eval(b, &ctx, &wdom, view, None)?;
            reads.extend(ctx.reads.into_inner());
        } else if let Some(fb) = self.d.files.get(field) {
            // A base from a file (`fields.md` 4.3), sampled to the posts.
            match via_raster::file::resample(&grid, self.whole(), &fb.samples, fb.placement) {
                Ok(r) => cur = r.values().to_vec(),
                Err(e) => {
                    let pos = e.position();
                    let mut d = text::at_key(
                        e.code(),
                        "field",
                        field,
                        "file",
                        &e.to_string(),
                        "check the file and its `extent` and `outside`",
                    );
                    if let Some(p) = pos {
                        d = d.at(p);
                    }
                    self.report(view, d);
                    return None;
                }
            }
        } else if fd.file.is_some() {
            // The file could not be read; its fault is said at `file`.
            return None;
        }
        let mut anchors: Vec<Anchor> = Vec::new();
        let mut hid = Hidden::default();
        let mut failed = false;
        for (idx, entry) in entries.iter().enumerate() {
            match entry {
                Entry::Row(r) => {
                    let row = &fd.rows[*r];
                    let ctx = Ctx {
                        f: self,
                        view,
                        current: Some(&cur),
                        served: None,
                        reads: RefCell::new(BTreeSet::new()),
                        own: None,
                        own_id: None,
                    };
                    let mask = match row.mask {
                        Some(m) => self.eval(m, &ctx, &wdom, view, None),
                        None => Some(vec![1.0; n]),
                    };
                    let value = if fd.weights {
                        None
                    } else {
                        row.value
                            .and_then(|v| self.eval(v, &ctx, &wdom, view, None))
                    };
                    reads.extend(ctx.reads.into_inner());
                    if fd.weights {
                        let (Some(mask), Some(paint)) = (mask, &row.paint) else {
                            failed = true;
                            continue;
                        };
                        paint_into(&mut weights, paint, &mask, None, n);
                    } else {
                        let (Some(value), Some(mask)) = (value, mask) else {
                            failed = true;
                            continue;
                        };
                        for k in 0..n {
                            let m = mask[k];
                            cur[k] = (1.0 - m) * cur[k] + m * value[k];
                        }
                    }
                }
                Entry::Effect { writer, effect } => {
                    let w = &self.writers[*writer];
                    let k = &self.d.kinds[&w.kind];
                    let e = &k.effects[effect];
                    let geo = &self.placed.shapes[&w.name];
                    writers.insert(w.name.clone());
                    // The posts within the radius (fields.md 5.3), and the feature's measures there.
                    let (bmin, bmax) = geo.bbox();
                    let window = match e.radius {
                        Some(r) => match via_raster::window::effect_window(
                            &grid,
                            [bmin.x, bmin.y, bmax.x, bmax.y],
                            r,
                        ) {
                            Ok(Some(w)) => w,
                            _ => continue,
                        },
                        None => self.whole(),
                    };
                    let mut posts: Vec<usize> = Vec::new();
                    let mut pts: Vec<(i64, i64)> = Vec::new();
                    let mut cols: Vec<lineref_cols::Cols> = Vec::new();
                    let mut ring: Vec<bool> = Vec::new();
                    for (i, j) in window.posts() {
                        let (x, y) = via_raster::window::post_mm(&grid, i, j);
                        let m = geo.project(P::new(x, y));
                        if let Some(r) = e.radius {
                            if m.d > via_num::metres(r) {
                                continue;
                            }
                            ring.push(via_raster::window::on_ring(&grid, i, j, m.d, r));
                        } else {
                            ring.push(false);
                        }
                        posts.push(post_index(&grid, i, j));
                        pts.push((x, y));
                        cols.push(lineref_cols::Cols {
                            d: m.d,
                            sd: m.sd,
                            t: m.t.unwrap_or(0.0),
                        });
                    }
                    if posts.is_empty() {
                        continue;
                    }
                    let dom = Domain::Points(&pts);
                    let served = Served {
                        name: &w.name,
                        geo,
                        attrs: &w.attrs,
                        local: &cols,
                    };
                    let ctx = Ctx {
                        f: self,
                        view,
                        current: Some(&cur),
                        served: Some(&served),
                        reads: RefCell::new(BTreeSet::new()),
                        own: None,
                        own_id: None,
                    };
                    let mask = match e.mask {
                        Some(m) => self.eval(m, &ctx, &dom, view, Some(&w.name)),
                        None => Some(vec![1.0; posts.len()]),
                    };
                    let Some(mask) = mask else {
                        failed = true;
                        continue;
                    };
                    if view == View::Build {
                        if let Some(k0) = (0..posts.len()).find(|k| ring[*k] && mask[*k] > 0.02) {
                            let (x, y) = pts[k0];
                            self.report(view, at_path(codes::field::EFFECT_CUT, "kind", &w.kind, &["effect", effect], &format!("the effect `{effect}` on `{}` stops in a visible step at its radius", w.name), "raise `radius`, or fade `mask` to 0 by it").at(text::pos(x, y)).with("feature", DValue::text(w.name.clone())));
                        }
                    }
                    if fd.weights {
                        let Some(paint) = &e.paint else {
                            self.report(view, at_path(codes::field::ROW_MISSING, "kind", &w.kind, &["effect", effect], &format!("the effect writes the weights field `{field}` with no `paint`"), "add `paint`"));
                            continue;
                        };
                        if e.value.is_some() {
                            self.report(
                                view,
                                at_path(
                                    codes::field::ROW_KEY,
                                    "kind",
                                    &w.kind,
                                    &["effect", effect, "value"],
                                    &format!(
                                        "`value` on an effect matching the weights field `{field}`"
                                    ),
                                    "use `paint`, or narrow the selector",
                                ),
                            );
                        }
                        paint_into(&mut weights, paint, &mask, Some(&posts), n);
                        if view == View::Build {
                            hid.track(idx, *writer, &posts, &mask, true);
                        }
                        continue;
                    }
                    if e.paint.is_some() {
                        self.report(
                            view,
                            at_path(
                                codes::field::ROW_KEY,
                                "kind",
                                &w.kind,
                                &["effect", effect, "paint"],
                                &format!(
                                    "`paint` on an effect matching the scalar field `{field}`"
                                ),
                                "use `value`, or narrow the selector",
                            ),
                        );
                    }
                    let Some(vf) = e.value else {
                        self.report(
                            view,
                            at_path(
                                codes::field::ROW_MISSING,
                                "kind",
                                &w.kind,
                                &["effect", effect],
                                &format!(
                                    "the effect writes the scalar field `{field}` with no `value`"
                                ),
                                "add `value`",
                            ),
                        );
                        continue;
                    };
                    let value = self.eval(vf, &ctx, &dom, view, Some(&w.name));
                    let limit = match (view, e.max_change, &value) {
                        (View::Build, Some(mc), Some(_)) => {
                            let one = [pts[0]];
                            let dom1 = Domain::Points(&one);
                            self.eval(mc, &ctx, &dom1, view, Some(&w.name))
                                .and_then(|v| v.first().copied())
                        }
                        _ => None,
                    };
                    reads.extend(ctx.reads.into_inner());
                    let Some(value) = value else {
                        failed = true;
                        continue;
                    };
                    if view == View::Build {
                        {
                            if let Some(limit) = limit {
                                let mut worst: Option<(usize, f64)> = None;
                                for k in 0..posts.len() {
                                    if mask[k] > 0.0 {
                                        let moved = ((value[k] - cur[posts[k]]) * mask[k]).abs();
                                        if worst.is_none_or(|(_, m)| moved > m) {
                                            worst = Some((k, moved));
                                        }
                                    }
                                }
                                if let Some((k, moved)) = worst.filter(|(_, m)| *m > limit) {
                                    let (x, y) = pts[k];
                                    let mut d = at_path(codes::authority::MAX_CHANGE, "kind", &w.kind, &["effect", effect, "max_change"], &format!("`{}` moves `{field}` by {} where `max_change` allows {}", w.name, text::dbl(moved), text::dbl(limit)), &format!("at this post `v` is {} and `value` {}; widen `max_change` or change `value`", text::dbl(cur[posts[k]]), text::dbl(value[k]))).at(text::pos(x, y)).with("feature", DValue::text(w.name.clone()));
                                    if let Some(v) = DValue::double(moved) {
                                        d = d.with("amount", v);
                                    }
                                    self.report(view, d);
                                }
                            }
                        }
                    }
                    if view == View::Build && w.tier == Tier::Anchored {
                        let pos = anchors.iter().position(|a| a.writer == *writer);
                        let a = match pos {
                            Some(p) => &mut anchors[p],
                            None => {
                                anchors.push(Anchor {
                                    writer: *writer,
                                    ..Anchor::default()
                                });
                                anchors.last_mut().expect("just pushed")
                            }
                        };
                        let mut rec = BTreeMap::new();
                        for k in 0..posts.len() {
                            rec.insert(posts[k], (value[k], mask[k]));
                        }
                        a.entries.push(rec);
                    }
                    for k in 0..posts.len() {
                        let m = mask[k];
                        let p = posts[k];
                        cur[p] = (1.0 - m) * cur[p] + m * value[k];
                    }
                    if view == View::Build {
                        let erases = !self
                            .d
                            .formulas
                            .programs
                            .get(&vf)
                            .is_some_and(|p| p.reads.entry_field);
                        hid.track(idx, *writer, &posts, &mask, erases);
                    }
                    if view == View::Build && w.tier == Tier::Anchored {
                        // The value just after this feature's last entry so far.
                        let last_of_writer = entries[idx + 1..].iter().all(
                            |x| !matches!(x, Entry::Effect { writer: ww, .. } if ww == writer),
                        );
                        if last_of_writer {
                            if let Some(a) = anchors.iter_mut().find(|a| a.writer == *writer) {
                                let touched: BTreeSet<usize> =
                                    a.entries.iter().flat_map(|r| r.keys().copied()).collect();
                                for p in touched {
                                    a.after.insert(p, cur[p]);
                                }
                            }
                        }
                    }
                }
            }
        }
        if failed {
            return None;
        }
        if view == View::Build && !fd.weights {
            self.anchors_meet(field, &anchors);
        }
        if view == View::Build {
            self.report_hidden(field, &hid);
        }
        // Narrow once (fields.md 1.1).
        let narrow = |v: &[f64]| -> Option<Raster<f32>> {
            let mut out = Vec::with_capacity(v.len());
            for (k, x) in v.iter().enumerate() {
                match via_num::store_f32(*x) {
                    Ok(f) => out.push(f),
                    Err(_) => {
                        let (i, j) = post_of(&grid, k);
                        let (px, py) = via_raster::window::post_mm(&grid, i, j);
                        self.report(
                            view,
                            text::at_header(
                                codes::num::NOT_FINITE_RESULT,
                                "field",
                                field,
                                &format!("`{field}` is not finite as a 32-bit float at this post"),
                                "keep the field's values within the 32-bit float's range",
                            )
                            .at(text::pos(px, py)),
                        );
                        return None;
                    }
                }
            }
            Raster::from_values(self.whole(), out)
        };
        let (scalar, wmap) = if fd.weights {
            let mut m = BTreeMap::new();
            for (mat, plane) in &weights {
                m.insert(mat.clone(), narrow(plane)?);
            }
            (None, m)
        } else {
            (Some(narrow(&cur)?), BTreeMap::new())
        };
        if view == View::Build && fd.role == Some(Role::Material) {
            let mut first: Option<usize> = None;
            let mut count = 0u64;
            for k in 0..n {
                let s: f64 = wmap.values().map(|r| f64::from(r.values()[k])).sum();
                if s < 1.0 - 1e-6 {
                    count += 1;
                    first.get_or_insert(k);
                }
            }
            if let Some(k) = first {
                let (i, j) = post_of(&grid, k);
                let (x, y) = via_raster::window::post_mm(&grid, i, j);
                self.report(
                    view,
                    text::at_header(
                        codes::field::UNPAINTED,
                        "field",
                        field,
                        &format!("{count} posts of `{field}` have weights summing below 1"),
                        "add a first row with no mask",
                    )
                    .at(text::pos(x, y))
                    .with("count", DValue::Int(count as i64)),
                );
            }
        }
        Some(Resolved {
            scalar,
            weights: wmap,
            writers,
            reads,
        })
    }

    /// `authority.md` 5, hidden entries: an entry whose every post with mask above 0 is later set
    /// by an entry with mask 1 left no trace; once per (feature, field), naming the features that
    /// cover it.
    fn report_hidden(&self, field: &str, hid: &Hidden) {
        let mut said: BTreeSet<usize> = BTreeSet::new();
        for (idx, writer, mine) in &hid.entries {
            if mine.is_empty() || said.contains(writer) {
                continue;
            }
            let mut by: BTreeSet<usize> = BTreeSet::new();
            let covered = mine.iter().all(|p| match hid.last1.get(p) {
                Some((at, w)) if at > idx => {
                    by.insert(*w);
                    true
                }
                _ => false,
            });
            by.remove(writer);
            if !covered || by.is_empty() {
                continue;
            }
            said.insert(*writer);
            let w = &self.writers[*writer];
            let names: Vec<String> = by
                .iter()
                .map(|b| format!("`{}`", self.writers[*b].name))
                .collect();
            let (top, id) = writer_subject(self.d, &w.name);
            let mut d = text::at_header(
                codes::authority::HIDDEN,
                top,
                &id,
                &format!(
                    "`{}` leaves no trace on `{field}`: {} later set every post it touches with mask 1",
                    w.name,
                    names.join(", ")
                ),
                "move it out from under them, raise its authority, or drop it if that is meant",
            )
            .with("field", DValue::text(field));
            for b in &by {
                let (bt, bid) = writer_subject(self.d, &self.writers[*b].name);
                d = d.related(via_diag::Related {
                    subject: Subject::entity(bt, bid),
                    site: Site::Header,
                    position: None,
                    note: via_diag::text!("a later entry with mask 1 that covers it"),
                });
            }
            self.report(View::Build, d);
        }
    }

    /// `authority.md` 5: anchored features that meet on one scalar field.
    fn anchors_meet(&self, field: &str, anchors: &[Anchor]) {
        for (bi, b) in anchors.iter().enumerate() {
            for a in &anchors[..bi] {
                let mut both1 = 0u64;
                let mut both0 = 0u64;
                let mut worst1: Option<(usize, f64, f64, f64)> = None;
                let mut worst0: Option<(usize, f64, f64, f64)> = None;
                for eb in &b.entries {
                    for (&p, &(vb, mb)) in eb {
                        if mb <= 0.0 {
                            continue;
                        }
                        let ma = a
                            .entries
                            .iter()
                            .filter_map(|r| r.get(&p))
                            .map(|x| x.1)
                            .fold(0.0, via_num::max);
                        if ma <= 0.0 {
                            continue;
                        }
                        let Some(&va) = a.after.get(&p) else { continue };
                        let diff = (va - vb).abs();
                        if diff <= 0.001 {
                            continue;
                        }
                        both0 += 1;
                        let better = |w: &Option<(usize, f64, f64, f64)>| {
                            w.is_none_or(|(q, d, _, _)| diff > d || (diff == d && p < q))
                        };
                        if better(&worst0) {
                            worst0 = Some((p, diff, va, vb));
                        }
                        if mb == 1.0
                            && a.entries
                                .iter()
                                .any(|r| r.get(&p).is_some_and(|x| x.1 == 1.0))
                        {
                            both1 += 1;
                            if better(&worst1) {
                                worst1 = Some((p, diff, va, vb));
                            }
                        }
                    }
                }
                let (code, worst, count) = match (worst1, worst0) {
                    (Some(w), _) => (codes::authority::CONFLICT, w, both1),
                    (None, Some(w)) => (codes::authority::SQUEEZED, w, both0),
                    _ => continue,
                };
                let (wa, wb) = (&self.writers[a.writer], &self.writers[b.writer]);
                let (i, j) = post_of(&self.grid, worst.0);
                let (x, y) = via_raster::window::post_mm(&self.grid, i, j);
                let (top, id) = writer_subject(self.d, &wb.name);
                let (atop, aid) = writer_subject(self.d, &wa.name);
                let mut d = text::diag(
                    code,
                    Subject::entity(top, id.clone()),
                    authority_site(),
                    &format!("`{}` and `{}` are both anchored and disagree on `{field}` at {count} posts: `{}` leaves {}, `{}` asks {}", wb.name, wa.name, wa.name, text::dbl(worst.2), wb.name, text::dbl(worst.3)),
                    &format!("make one `shaped` so the other holds; or move one off the other; or let `{}` read `{}` with `value = \"= v\"` where they overlap", wb.name, wa.name),
                )
                .at(text::pos(x, y))
                .with("field", DValue::text(field))
                .with("both_full", DValue::Int(both1 as i64))
                .with("both_partial", DValue::Int(both0 as i64));
                d = d.related(via_diag::Related {
                    subject: Subject::entity(atop, aid.clone()),
                    site: authority_site(),
                    position: None,
                    note: via_diag::text!("the earlier anchored feature"),
                });
                self.report(View::Build, d);
            }
        }
    }

    /// The per-post maximum of every `keep_out` field in a view (`fields.md` 2.1).
    pub fn keep_out(&self, view: View) -> Option<Rc<Raster<f64>>> {
        if let Some(k) = self.keep_out.borrow().get(&view) {
            return Some(k.clone());
        }
        let names: Vec<String> = self
            .d
            .fields
            .values()
            .filter(|f| f.role == Some(Role::KeepOut))
            .map(|f| f.name.clone())
            .collect();
        let mut rasters = Vec::new();
        for n in &names {
            let r = self.get(n, view)?;
            rasters.push(r);
        }
        let refs: Vec<&Raster<f32>> = rasters.iter().filter_map(|r| r.scalar.as_ref()).collect();
        let (k, first) = via_raster::rows::keep_out(self.whole(), &refs).ok()?;
        if view == View::Build {
            for (n, f) in names.iter().zip(first) {
                if let Some((i, j)) = f {
                    let (x, y) = via_raster::window::post_mm(&self.grid, i, j);
                    self.report(
                        view,
                        text::at_header(
                            codes::field::ROLE_RANGE,
                            "field",
                            n,
                            &format!("`{n}` holds a `keep_out` value outside 0 to 1; clamped"),
                            "wrap the row or base in `clamp(..., 0, 1)`",
                        )
                        .at(text::pos(x, y)),
                    );
                }
            }
        }
        let k = Rc::new(k);
        self.keep_out.borrow_mut().insert(view, k.clone());
        Some(k)
    }

    /// A scalar field read at a lattice point by `fields.md` 1.2.
    pub fn read(&self, field: &str, view: View, p: P) -> Option<f64> {
        let r = self.get(field, view)?;
        read_at(&self.grid, r.scalar.as_ref()?, p.x, p.y).ok()
    }

    /// The surface field's name.
    pub fn surface(&self) -> Option<&'a str> {
        self.d
            .fields
            .values()
            .find(|f| f.role == Some(Role::Surface))
            .map(|f| f.name.as_str())
    }
}

fn writer_subject(d: &Decls, name: &str) -> (&'static str, String) {
    if d.features.contains_key(name) {
        ("feature", name.to_string())
    } else {
        ("instance", name.trim_end_matches(".ground").to_string())
    }
}

fn authority_site() -> Site {
    // `authority.md` 5: the line of each one's `authority`, the feature's, or its kind's when
    // inherited (as a footprint's always is). An unwritten key resolves to the kind's, with the
    // `defaults` entry in the chain (`diagnostics.md` 1.2).
    Site::Key(via_diag::KeyPath::key("authority"))
}

/// Whether `b` lies inside region `a` and is smaller (`authority.md` 4.3), exactly.
fn inside(b: &Geo, a: &Geo) -> bool {
    let Geo::Region { region: ra, .. } = a else {
        return false;
    };
    let (amin, amax) = a.bbox();
    let (bmin, bmax) = b.bbox();
    if bmin.x < amin.x || bmin.y < amin.y || bmax.x > amax.x || bmax.y > amax.y {
        return false;
    }
    match b {
        Geo::Point { p, .. } => a.contains(*p),
        Geo::Path { line } => {
            line.pts.iter().all(|p| a.contains(*p))
                && (0..line.segments()).all(|k| {
                    let (p, q) = line.seg(k);
                    a.contains(P::new((p.x + q.x).div_euclid(2), (p.y + q.y).div_euclid(2)))
                })
        }
        Geo::Region { region: rb, .. } => {
            rb.twice_area() < ra.twice_area()
                && via_geom::overlay::boolean(
                    via_geom::overlay::BoolOp::Difference,
                    &[rb.clone(), ra.clone()],
                )
                .is_ok_and(|r| r.is_empty())
        }
    }
}

fn paint_into(
    weights: &mut BTreeMap<String, Vec<f64>>,
    paint: &str,
    mask: &[f64],
    posts: Option<&[usize]>,
    n: usize,
) {
    weights
        .entry(paint.to_string())
        .or_insert_with(|| vec![0.0; n]);
    let mats: Vec<String> = weights.keys().cloned().collect();
    let count = posts.map_or(n, <[usize]>::len);
    for k in 0..count {
        let p = posts.map_or(k, |ps| ps[k]);
        let m = mask[k];
        for mat in &mats {
            let w = weights.get_mut(mat).expect("listed");
            if mat == paint {
                w[p] = (1.0 - m) * w[p] + m;
            } else {
                w[p] *= 1.0 - m;
            }
        }
    }
}

/// The selection phase over this world, at the reader's rank (`None`: every rank).
pub(crate) struct Sel<'f, 'a> {
    fields: &'f Fields<'a>,
    own: Option<&'f str>,
    rank: Option<i64>,
}

impl Sel<'_, '_> {
    fn sees(&self, rank: i64) -> bool {
        self.rank.is_none_or(|r| rank <= r)
    }
}

fn shape_of(g: &Geo) -> Shape {
    match g {
        Geo::Point { .. } => Shape::Point,
        Geo::Path { .. } => Shape::Path,
        Geo::Region { region, .. } => Shape::Region {
            rings: region.ring_count() as u32,
        },
    }
}

impl Select for Sel<'_, '_> {
    fn fields(&self, selector: &str) -> Vec<FieldInfo> {
        let d = self.fields.d;
        d.fields
            .values()
            .filter(|f| match selector.strip_prefix("tag:") {
                // A read selector leaves out what lies above the reader (`documents.md` 5.1); a
                // field named outright is a named reference, checked as `doc.upward_ref`.
                Some(t) => f.tags.iter().any(|x| x == t) && self.sees(f.rank),
                None => f.name == selector,
            })
            .map(|f| FieldInfo {
                name: f.name.clone(),
                weights: f.weights,
            })
            .collect()
    }

    fn shapes(&self, selector: &str) -> Vec<ShapeInfo> {
        let p = self.fields.placed;
        match selector.strip_prefix("kind:") {
            Some(k) => self
                .fields
                .writers
                .iter()
                .filter(|w| w.kind == k && self.sees(w.rank))
                .filter_map(|w| {
                    p.shapes.get(&w.name).map(|g| ShapeInfo {
                        id: w.name.clone(),
                        shape: shape_of(g),
                    })
                })
                .collect(),
            None => p
                .shapes
                .get(selector)
                .filter(|_| self.sees(self.fields.shape_rank(selector)))
                .map(|g| {
                    vec![ShapeInfo {
                        id: selector.to_string(),
                        shape: shape_of(g),
                    }]
                })
                .unwrap_or_default(),
        }
    }

    fn own_shape(&self) -> Option<Shape> {
        self.own
            .and_then(|o| self.fields.placed.shapes.get(o))
            .map(shape_of)
    }

    fn names(&self, class: via_expr::Class) -> Vec<String> {
        match class {
            via_expr::Class::Field => self.fields.d.fields.keys().cloned().collect(),
            _ => self.fields.placed.shapes.keys().cloned().collect(),
        }
    }
}

/// What one evaluation reads.
struct Ctx<'c> {
    f: &'c Fields<'c>,
    view: View,
    current: Option<&'c [f64]>,
    served: Option<&'c Served<'c>>,
    reads: RefCell<BTreeSet<String>>,
    /// The attributes of the feature whose `where` is evaluated (`formulas.md` 6.6).
    own: Option<&'c BTreeMap<String, Attr>>,
    /// The id of that feature.
    own_id: Option<&'c str>,
}

fn internal(what: &str) -> ReadError {
    ReadError::new(
        codes::diag::INTERNAL,
        format!("the model cannot give {what}"),
        "report it: this is a defect of the engine",
    )
}

impl Ctx<'_> {
    fn read_view(&self) -> View {
        match self.view {
            View::Head => View::Head,
            View::Stratum(r) => View::Stratum(r),
            _ => View::Build,
        }
    }

    fn samples(&self, geo: &Geo) -> Vec<P> {
        let g = &self.f.grid;
        match geo {
            Geo::Point { p, .. } => vec![*p],
            Geo::Path { line } => {
                let l = line.length();
                let r = g.resolution as f64;
                let mut s = 0.0;
                let mut out = Vec::new();
                while s < l {
                    if let Ok(a) = via_geom::lineref::at_length(line, "under", s, false, 0) {
                        out.push(a.point);
                    }
                    s += r;
                }
                if let Ok(a) = via_geom::lineref::at_length(line, "under", l, true, 0) {
                    out.push(a.point);
                }
                out
            }
            Geo::Region { .. } => {
                let (bmin, bmax) = geo.bbox();
                let Ok(Some(w)) =
                    via_raster::window::effect_window(g, [bmin.x, bmin.y, bmax.x, bmax.y], 0)
                else {
                    return Vec::new();
                };
                w.posts()
                    .map(|(i, j)| via_raster::window::post_mm(g, i, j))
                    .map(|(x, y)| P::new(x, y))
                    .filter(|p| geo.contains(*p))
                    .collect()
            }
        }
    }
}

impl Reads for Ctx<'_> {
    fn attribute(&self, feature: Option<&str>, name: &str) -> Result<Scalar, ReadError> {
        let attrs = match feature {
            None => self.served.map(|s| s.attrs).or(self.own),
            Some(id) => self.f.d.features.get(id).map(|f| &f.attrs),
        }
        .ok_or_else(|| internal("an attribute"))?;
        let owner = feature.or(self.served.map(|s| s.name)).or(self.own_id);
        match attrs.get(name) {
            Some(Attr::Number(x)) => Ok(Scalar::Number(*x)),
            Some(Attr::YesNo(b)) => Ok(Scalar::YesNo(*b)),
            // An attribute formula that failed said so at its own text (`graph.md` 3.4).
            None if owner.is_some_and(|o| {
                self.f
                    .d
                    .failed_attrs
                    .contains(&(o.to_string(), name.to_string()))
            }) =>
            {
                Err(ReadError::new(
                    codes::diag::SKIPPED,
                    format!("`{name}` could not be worked out"),
                    "see the attribute's own diagnostic",
                ))
            }
            _ => Err(ReadError::new(
                codes::formula::TYPE,
                format!("`{name}` is not a number here"),
                "write a number for the attribute",
            )),
        }
    }

    fn field(&self, field: &str, domain: &Domain) -> Result<Vec<Option<f64>>, ReadError> {
        self.reads.borrow_mut().insert(field.to_string());
        let r = self.f.get(field, self.read_view()).ok_or_else(|| {
            ReadError::new(
                // The field failed and said so; its readers say nothing (`graph.md` 3.4).
                codes::diag::SKIPPED,
                format!("`{field}` could not be resolved"),
                "see the field's own diagnostic",
            )
        })?;
        let s = r
            .scalar
            .as_ref()
            .ok_or_else(|| internal("a weights field as a number"))?;
        let g = &self.f.grid;
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| read_at(g, s, x, y).ok())
            .collect())
    }

    fn slope(&self, field: &str, points: &[(i64, i64)]) -> Result<Vec<Option<f64>>, ReadError> {
        self.reads.borrow_mut().insert(field.to_string());
        let g = &self.f.grid;
        let r = self
            .f
            .get(field, self.read_view())
            .ok_or_else(|| internal("a field"))?;
        let s = r
            .scalar
            .as_ref()
            .ok_or_else(|| internal("a weights field as a number"))?;
        let sl =
            via_raster::slope::slope(g, s, RWindow::whole(g)).map_err(|_| internal("a slope"))?;
        Ok(points
            .iter()
            .map(|&(x, y)| via_raster::slope::slope_at(g, &sl, x, y).ok())
            .collect())
    }

    fn v(&self, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let cur = self.current.ok_or_else(|| internal("`v`"))?;
        let g = &self.f.grid;
        domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let i = (x - g.extent[0]) / g.resolution;
                let j = (y - g.extent[1]) / g.resolution;
                cur.get(post_index(g, i as u64, j as u64))
                    .copied()
                    .ok_or_else(|| internal("`v` off the posts"))
            })
            .collect()
    }

    fn local(&self, measure: Measure, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let s = self
            .served
            .ok_or_else(|| internal("the feature's own measures"))?;
        if s.local.len() == domain.len() {
            return Ok(s
                .local
                .iter()
                .map(|c| match measure {
                    Measure::D => c.d,
                    Measure::Sd => c.sd,
                    Measure::T => c.t,
                    Measure::Inside => {
                        if c.d == 0.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                })
                .collect());
        }
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let m = s.geo.project(P::new(x, y));
                match measure {
                    Measure::D => m.d,
                    Measure::Sd => m.sd,
                    Measure::T => m.t.unwrap_or(0.0),
                    Measure::Inside => f64::from(u8::from(s.geo.contains(P::new(x, y)))),
                }
            })
            .collect())
    }

    fn measure(
        &self,
        feature: &str,
        measure: Measure,
        domain: &Domain,
        _cap: Option<f64>,
    ) -> Result<Vec<f64>, ReadError> {
        let g = self
            .f
            .placed
            .shapes
            .get(feature)
            .ok_or_else(|| internal("a feature's shape"))?;
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let p = P::new(x, y);
                match measure {
                    Measure::Inside => f64::from(u8::from(g.contains(p))),
                    m => {
                        let pr = g.project(p);
                        match m {
                            Measure::D => pr.d,
                            Measure::Sd => pr.sd,
                            _ => pr.t.unwrap_or(0.0),
                        }
                    }
                }
            })
            .collect())
    }

    fn under(&self, field: Option<&str>, stat: Stat) -> Result<f64, ReadError> {
        let s = self.served.ok_or_else(|| internal("`under`"))?;
        let g = &self.f.grid;
        let [x0, y0, x1, y1] = g.extent;
        let pts: Vec<P> = self.samples(s.geo);
        // `formulas.md` 8.5: a sample beyond the extent has no field value.
        if let Some(p) = pts
            .iter()
            .find(|p| !(p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1))
        {
            return Err(ReadError::new(
                codes::formula::OUTSIDE_FIELD,
                format!(
                    "`{}` has a sample at ({}, {}), beyond the extent: `under` reads the field there",
                    s.name,
                    via_num::write_length(p.x),
                    via_num::write_length(p.y)
                ),
                "keep the feature inside the world's extent, or widen the extent",
            ));
        }
        let mut vals = Vec::with_capacity(pts.len());
        match field {
            None => {
                let cur = self.current.ok_or_else(|| internal("`under`"))?;
                let r = Raster::from_values(RWindow::whole(g), cur.to_vec())
                    .ok_or_else(|| internal("`under`"))?;
                for p in &pts {
                    vals.push(read_at(g, &r, p.x, p.y).map_err(|_| internal("`under`"))?);
                }
            }
            Some(name) => {
                self.reads.borrow_mut().insert(name.to_string());
                let r = self
                    .f
                    .get(name, self.read_view())
                    .ok_or_else(|| internal("a field"))?;
                let sc = r
                    .scalar
                    .as_ref()
                    .ok_or_else(|| internal("a weights field as a number"))?;
                for p in &pts {
                    vals.push(read_at(g, sc, p.x, p.y).map_err(|_| internal("`under`"))?);
                }
            }
        }
        let res = match stat {
            Stat::Mean => via_num::mean(&vals),
            Stat::Min => via_num::minimum(&vals),
            Stat::Max => via_num::maximum(&vals),
        };
        res.map_err(|_| {
            ReadError::new(
                codes::num::EMPTY_REDUCTION,
                format!("`{}` has no samples inside the extent", s.name),
                "move the feature inside the extent",
            )
        })
    }

    /// `formulas.md` 8.5: the entry's field before this entry at the path's samples, two passes
    /// of a moving mean of half-width `floor(length / 2r)`, read at each point's `t`.
    fn smoothed_along(&self, length: f64, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let s = self.served.ok_or_else(|| internal("`smoothed_along`"))?;
        let Geo::Path { line } = s.geo else {
            return Err(ReadError::new(
                codes::formula::SHAPE,
                "`smoothed_along` takes a path; this feature is a region or a point",
                "use `under` for a region or a point",
            ));
        };
        let g = &self.f.grid;
        let cur = self.current.ok_or_else(|| internal("`smoothed_along`"))?;
        let field = Raster::from_values(RWindow::whole(g), cur.to_vec())
            .ok_or_else(|| internal("`smoothed_along`"))?;
        let l = line.length();
        let r = g.resolution as f64;
        // Arc lengths 0, r, 2r, ... while below L, then L (millimetres).
        let mut arcs = Vec::new();
        let mut a = 0.0;
        while a < l {
            arcs.push(a);
            a += r;
        }
        arcs.push(l);
        let mut vals = Vec::with_capacity(arcs.len());
        for (m, &sm) in arcs.iter().enumerate() {
            let last = m + 1 == arcs.len();
            let at = via_geom::lineref::at_length(line, "smoothed_along", sm, last, 0)
                .map_err(|_| internal("a sample along the path"))?;
            let v = read_at(g, &field, at.point.x, at.point.y).map_err(|_| {
                ReadError::new(
                    codes::formula::OUTSIDE_FIELD,
                    format!("`{}` has a sample beyond the extent", s.name),
                    "keep the path inside the world's extent",
                )
            })?;
            vals.push(v);
        }
        // Half-width in samples; `length` is in metres, `r` in millimetres.
        let k = (length * 1000.0 / (2.0 * r)).floor();
        let k = if k.is_finite() && k > 0.0 {
            k as usize
        } else {
            0
        };
        let pass = |v: &[f64]| -> Vec<f64> {
            let n = v.len();
            (0..n)
                .map(|m| {
                    let lo = m.saturating_sub(k);
                    let hi = m.saturating_add(k).min(n - 1);
                    let mut sum = 0.0;
                    for x in &v[lo..=hi] {
                        sum += *x;
                    }
                    sum / (hi - lo + 1) as f64
                })
                .collect()
        };
        let u = if k == 0 { vals } else { pass(&pass(&vals)) };
        let ts = self.local(Measure::T, domain)?;
        let n = u.len();
        Ok(ts
            .into_iter()
            .map(|t| {
                let at = t * l;
                if at >= l {
                    return u[n - 1];
                }
                let mut m = 0;
                while m + 1 < n && arcs[m + 1] <= at {
                    m += 1;
                }
                if m + 1 >= n {
                    return u[n - 1];
                }
                let span = arcs[m + 1] - arcs[m];
                if span <= 0.0 {
                    return u[m];
                }
                via_num::mix(u[m], u[m + 1], (at - arcs[m]) / span)
            })
            .collect())
    }
}
