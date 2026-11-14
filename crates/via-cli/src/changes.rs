//! `changes.json` (`build.md` 11.2): the model's change report (`identity.md` 5) written in the
//! fixed form, with position objects, the keys that changed and their values, a field's region,
//! and each cause located at its line, in this build's text or the baseline's.
//!
//! The model decides what is an entry and why (its `what` and its causes); this module adds only
//! what the format holds beyond that, worked out from the two builds as `features.json` and the
//! manifest write them, so a key that differs here is a key a reader of the two builds sees differ.

use std::collections::{BTreeMap, BTreeSet};

use via_model::report::{maker_of_name, ChangeReport, Entry};
use via_model::Build;

use crate::json::{obj, J};
use crate::layout::Layout;
use crate::out::{fact_json, feature_json, pieces, post_bytes, Surface};
use crate::snap::Snapshot;

/// The format of `changes.json`.
pub const FORMAT: &str = "via-changes-0";

/// One side of the comparison: a build and where its statements and things were written.
pub struct Side<'a> {
    pub build: &'a Build,
    pub lines: &'a crate::snap::Lines,
    pub homes: &'a BTreeMap<String, String>,
}

/// The chains of `graph.md` 6.4 per thing ([`chain_key`]): statement id to node ids, from the
/// node made from the statement to the thing's own node. Supplied by the caller from the graph.
pub type Chains = BTreeMap<String, BTreeMap<String, Vec<String>>>;

/// `changes.json` with no baseline.
pub fn without_baseline() -> J {
    obj([
        ("against", J::Null),
        ("entries", J::Arr(Vec::new())),
        ("format", J::s(FORMAT)),
        ("summary", J::Arr(Vec::new())),
    ])
}

fn box_of(points: &[(i64, i64)]) -> Option<[i64; 4]> {
    let f = points.first()?;
    let mut b = [f.0, f.1, f.0, f.1];
    for p in points {
        b[0] = b[0].min(p.0);
        b[1] = b[1].min(p.1);
        b[2] = b[2].max(p.0);
        b[3] = b[3].max(p.1);
    }
    Some(b)
}

fn box_json(b: [i64; 4]) -> J {
    J::Arr(b.iter().map(|v| J::mm(*v)).collect())
}

fn grow(acc: &mut Option<[i64; 4]>, b: [i64; 4]) {
    *acc = Some(match acc {
        None => b,
        Some(a) => [
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ],
    });
}

/// What a thing is, as `features.json` or the manifest writes it, key by key.
fn described(table: &str, name: &str, side: &Side<'_>) -> BTreeMap<String, J> {
    let b = side.build;
    let mut m = BTreeMap::new();
    let Some(grid) = b.grid else { return m };
    let surface = Surface::new(b, grid);
    match table {
        "feature" => {
            if let Some(f) = b.features.get(name) {
                let layer = side
                    .homes
                    .get(&format!("feature.{name}"))
                    .map(String::as_str);
                if let J::Obj(o) = feature_json(name, f, layer, &surface) {
                    for (k, v) in o {
                        match k.as_str() {
                            "attrs" => {
                                if let J::Obj(a) = v {
                                    for (ak, av) in a {
                                        m.insert(format!("attrs.{ak}"), av);
                                    }
                                }
                            }
                            "name" | "shape" | "at" => {}
                            _ => {
                                m.insert(k, v);
                            }
                        }
                    }
                }
            }
        }
        "footprint" => {
            if let Some(f) = b.footprints.get(name) {
                m.insert("kind".into(), J::s(f.kind.clone()));
                m.insert("pieces".into(), pieces(&f.ring));
                m.insert(
                    "layer".into(),
                    J::opt(
                        side.homes
                            .get(&format!("instance.{name}"))
                            .map(String::as_str),
                    ),
                );
            }
        }
        "instance" => {
            if let Some(i) = b.instances.get(name) {
                m.insert(
                    "scale".into(),
                    J::dbl(f64::from(i.scale as f32)).unwrap_or(J::Null),
                );
                m.insert(
                    "tint".into(),
                    J::dbl(f64::from(i.tint as f32)).unwrap_or(J::Null),
                );
            }
        }
        "field" => {
            if let Some(f) = b.fields.get(name) {
                m.insert("role".into(), J::opt(f.role.as_deref()));
                m.insert("unit".into(), J::opt(f.unit.as_deref()));
                m.insert("tags".into(), J::strs(f.tags.iter().map(String::as_str)));
                m.insert(
                    "type".into(),
                    J::s(if f.values.is_some() {
                        "scalar"
                    } else {
                        "weights"
                    }),
                );
            }
        }
        _ => {}
    }
    m
}

/// The position object (11.2) of a thing in one build, with `values` for `keys`.
fn position(
    table: &str,
    name: &str,
    side: &Side<'_>,
    keys: &[String],
    desc: &BTreeMap<String, J>,
) -> (Option<J>, Option<[i64; 4]>) {
    let b = side.build;
    let Some(grid) = b.grid else {
        return (None, None);
    };
    let surface = Surface::new(b, grid);
    let mut m = J::obj();
    let bx;
    match table {
        "instance" => {
            let Some(i) = b.instances.get(name) else {
                return (None, None);
            };
            m.insert("asset".into(), J::s(i.asset.clone()));
            m.insert(
                "at".into(),
                J::Arr(vec![
                    J::mm(i.origin.0),
                    J::mm(i.origin.1),
                    J::dbl(f64::from(i.z as f32)).unwrap_or(J::Null),
                ]),
            );
            m.insert("face".into(), J::int(i.facing.rem_euclid(360_000)));
            bx = Some([i.origin.0, i.origin.1, i.origin.0, i.origin.1]);
        }
        "feature" => {
            let Some(f) = b.features.get(name) else {
                return (None, None);
            };
            if f.shape == "point" {
                if let Some(p) = f.points.first() {
                    m.insert("at".into(), surface.xyz(*p));
                }
            }
            bx = box_of(&f.points);
            if let Some(v) = bx {
                m.insert("box".into(), box_json(v));
            }
        }
        "footprint" => {
            let Some(f) = b.footprints.get(name) else {
                return (None, None);
            };
            bx = box_of(&f.ring);
            if let Some(v) = bx {
                m.insert("box".into(), box_json(v));
            }
        }
        _ => return (None, None),
    }
    let mut values = J::obj();
    for k in keys {
        if k == "facts" || k == "samples" {
            continue;
        }
        if let Some(v) = desc.get(k) {
            values.insert(k.clone(), v.clone());
        }
    }
    m.insert("values".into(), J::Obj(values));
    (Some(J::Obj(m)), bx)
}

/// Where a field's samples differ (11.2), and its box in millimetres.
fn region(name: &str, old: &Build, new: &Build) -> Option<(J, Option<[i64; 4]>, bool)> {
    let (a, b) = (old.fields.get(name)?, new.fields.get(name)?);
    let (ga, gb) = (old.grid?, new.grid?);
    let layout = Layout::new(&gb);
    let n = usize::try_from(gb.nx * gb.ny).unwrap_or(0);
    let same_frame = ga.extent == gb.extent && ga.resolution == gb.resolution;
    let mut differs = vec![false; n];
    let mut most: Option<f64> = None;
    if !same_frame {
        differs.iter_mut().for_each(|d| *d = true);
    } else if let (Some(va), Some(vb)) = (&a.values, &b.values) {
        for p in 0..n.min(va.len()).min(vb.len()) {
            if va[p].to_bits() != vb[p].to_bits() {
                differs[p] = true;
                let d = (f64::from(va[p]) - f64::from(vb[p])).abs();
                if most.is_none_or(|m| d > m) {
                    most = Some(d);
                }
            }
        }
    } else {
        let mats: BTreeSet<&String> = a.weights.keys().chain(b.weights.keys()).collect();
        let bytes = |f: &via_model::FieldOut, p: usize| -> BTreeMap<String, u8> {
            let ms: Vec<(&String, f64)> = f
                .weights
                .iter()
                .map(|(m, v)| (m, f64::from(*v.get(p).unwrap_or(&0.0))))
                .collect();
            let w: Vec<f64> = ms.iter().map(|x| x.1).collect();
            ms.iter().map(|x| x.0.clone()).zip(post_bytes(&w)).collect()
        };
        for (p, d) in differs.iter_mut().enumerate() {
            let (x, y) = (bytes(a, p), bytes(b, p));
            for m in &mats {
                let (u, v) = (
                    f64::from(*x.get(*m).unwrap_or(&0)),
                    f64::from(*y.get(*m).unwrap_or(&0)),
                );
                if u != v {
                    *d = true;
                    let diff = (u - v).abs();
                    if most.is_none_or(|q| diff > q) {
                        most = Some(diff);
                    }
                }
            }
        }
    }
    let count = differs.iter().filter(|d| **d).count();
    let keys_differ = a.role != b.role || a.unit != b.unit || a.tags != b.tags;
    let mut bx: Option<[i64; 4]> = None;
    let mut windows = Vec::new();
    for c in &layout.chunks {
        let mut w: Option<[u64; 4]> = None;
        for j in c.j0..=c.j1 {
            for i in c.i0..=c.i1 {
                if differs[usize::try_from(j * gb.nx + i).unwrap_or(0)] {
                    w = Some(match w {
                        None => [i, j, i + 1, j + 1],
                        Some(o) => [o[0].min(i), o[1].min(j), o[2].max(i + 1), o[3].max(j + 1)],
                    });
                    if let Some((x, y)) = gb.post(i, j) {
                        grow(&mut bx, [x, y, x, y]);
                    }
                }
            }
        }
        if let Some(w) = w {
            windows.push(obj([
                (
                    "chunk",
                    J::Arr(vec![J::Num(c.cx.to_string()), J::Num(c.cy.to_string())]),
                ),
                (
                    "window",
                    J::Arr(w.iter().map(|v| J::Num(v.to_string())).collect()),
                ),
            ]));
        }
    }
    let r = obj([
        ("box", bx.map_or(J::Null, box_json)),
        (
            "most",
            if same_frame {
                most.and_then(J::dbl).unwrap_or(J::Null)
            } else {
                J::Null
            },
        ),
        ("posts", J::count(count)),
        ("windows", J::Arr(windows)),
    ]);
    Some((r, bx, count > 0 || keys_differ))
}

/// The key of a thing's chains in [`Chains`]: its table and name.
pub fn chain_key(table: &str, name: &str) -> String {
    format!("{table}\0{name}")
}

/// Whether a cause (`identity.md` 5's form) names a statement: the statement itself, a key below
/// it (`feature.ford.on`, `world.extent`), or an asset's fact (`asset.<id>.<fact>` of the
/// statement `asset:<id>`).
fn names_statement(cause: &str, statement: &str) -> bool {
    if cause == statement || cause.starts_with(&format!("{statement}.")) {
        return true;
    }
    match (
        cause.strip_prefix("asset."),
        statement.strip_prefix("asset:"),
    ) {
        (Some(rest), Some(id)) => rest == id || rest.starts_with(&format!("{id}.")),
        _ => false,
    }
}

/// A cause (11.2), located in this build's text or the baseline's.
fn cause_json(cause: &str, new: &Side<'_>, old: &Side<'_>, via: Vec<String>) -> J {
    let locate = |lines: &crate::snap::Lines| -> Option<(String, u32)> {
        if let Some(rest) = cause.strip_prefix("world.") {
            return lines.get("world")?.get(rest).cloned();
        }
        if let Some(m) = lines.get(cause) {
            return m.get("").cloned();
        }
        // `<table>.<id>.<key>`: the statement is the first two parts.
        let mut parts = cause.splitn(3, '.');
        let (t, id, key) = (parts.next()?, parts.next()?, parts.next()?);
        let m = lines.get(&format!("{t}.{id}"))?;
        m.get(key).or_else(|| m.get("")).cloned()
    };
    // A declared asset fact is located at its pack text like any statement (`assets.md` 7, its
    // lines recorded by `snap::lines_of`); only a fact measured from a model file (M2) has none.
    let (file, line, inn) = if let Some((f, l)) = locate(new.lines) {
        (J::s(f), J::Num(l.to_string()), "this")
    } else if let Some((f, l)) = locate(old.lines) {
        (J::s(f), J::Num(l.to_string()), "baseline")
    } else {
        (J::Null, J::Null, "this")
    };
    obj([
        ("cause", J::s(cause)),
        ("digests", J::Null),
        ("file", file),
        ("in", J::s(inn)),
        ("line", line),
        ("via", J::strs(via.iter().map(String::as_str))),
    ])
}

/// The maker of a thing (`identity.md` 1): a scatter's recorded maker, else the part of a
/// produced name before `#` (a step's returned feature, field or instance); `None` for a thing
/// written in text.
fn maker_of(e: &Entry, old: &Build, new: &Build) -> Option<String> {
    let recorded = match e.table.as_str() {
        "instance" | "footprint" => new
            .instances
            .get(&e.name)
            .or_else(|| old.instances.get(&e.name))
            .and_then(|i| i.maker.clone()),
        _ => None,
    };
    recorded.or_else(|| maker_of_name(&e.name))
}

#[derive(Default)]
struct Row {
    added: u64,
    removed: u64,
    moved: u64,
    swapped: u64,
    changed: u64,
    untouched: u64,
    bx: Option<[i64; 4]>,
}

/// `changes.json` against a baseline. `against` is `(id, key)` of the baseline.
pub fn write(
    report: &ChangeReport,
    new: &Side<'_>,
    old: &Snapshot,
    against: (String, String),
    chains: &Chains,
    causes_known: bool,
) -> J {
    let old_side = Side {
        build: &old.build,
        lines: &old.lines,
        homes: &old.homes,
    };
    // Statements written in another layer than in the baseline.
    let moved_home: BTreeSet<String> = new
        .homes
        .iter()
        .filter(|(k, v)| old.homes.get(*k).is_some_and(|o| o != *v))
        .map(|(k, _)| k.clone())
        .collect();
    let mut entries: Vec<(String, String, J)> = Vec::new();
    let mut rows: BTreeMap<(Option<String>, String), Row> = BTreeMap::new();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for e in &report.entries {
        seen.insert((e.table.clone(), e.name.clone()));
        let s = &e.status;
        let mut what = Vec::new();
        if s.added {
            what.push("added");
        } else if s.removed {
            what.push("removed");
        } else {
            if s.moved {
                what.push("moved");
            }
            if s.swapped {
                what.push("swapped");
            }
            if s.changed {
                what.push("changed");
            }
        }
        let (da, db) = (
            described(&e.table, &e.name, &old_side),
            described(&e.table, &e.name, new),
        );
        let mut keys: Vec<String> = Vec::new();
        let mut region_j = J::Null;
        let mut region_box = None;
        if s.changed {
            let all: BTreeSet<&String> = da.keys().chain(db.keys()).collect();
            for k in all {
                if da.get(k) != db.get(k) {
                    keys.push(k.clone());
                }
            }
            // A field's keys are `samples` and its manifest keys (11.2), never `facts`.
            if !e.facts.is_empty() && (e.table == "instance" || e.table == "footprint") {
                keys.push("facts".into());
            }
        }
        if e.table == "field" {
            if let Some((r, bx, _)) = region(&e.name, &old.build, new.build) {
                let samples = r.get("posts").and_then(J::as_i64).unwrap_or(0) > 0;
                if samples && s.changed {
                    keys.push("samples".into());
                }
                region_j = r;
                region_box = bx;
            }
        }
        keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        keys.dedup();
        let (before, bb) = if s.added {
            (J::Null, None)
        } else {
            let (p, b) = position(&e.table, &e.name, &old_side, &keys, &da);
            (p.unwrap_or(J::Null), b)
        };
        let (after, ab) = if s.removed {
            (J::Null, None)
        } else {
            let (p, b) = position(&e.table, &e.name, new, &keys, &db);
            (p.unwrap_or(J::Null), b)
        };
        let facts: Vec<J> = e
            .facts
            .iter()
            .map(|(f, (o, n))| {
                obj([
                    ("after", fact_json(n)),
                    ("before", fact_json(o)),
                    ("fact", J::s(f.clone())),
                ])
            })
            .collect();
        // The causes are the statements the graph finds the thing depends on, with a chain each
        // (`graph.md` 6.4), each expanded to the keys that differ (`identity.md` 5), beside
        // those the model names.
        let own: BTreeSet<String> = match e.table.as_str() {
            "instance" | "footprint" => [
                old.build.instances.get(&e.name),
                new.build.instances.get(&e.name),
            ]
            .into_iter()
            .flatten()
            .map(|i| i.asset.clone())
            .collect(),
            _ => BTreeSet::new(),
        };
        let found: BTreeSet<String> = chains
            .get(&chain_key(&e.table, &e.name))
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let (from_graph, _) = via_model::report::causes_of(&old.build, new.build, &found, &own);
        let mut all: BTreeSet<String> = e.causes.iter().cloned().collect();
        all.extend(from_graph);
        // A feature whose draped heights differ (`build.md` 11.2) has the surface field's
        // causes, as the graph finds them for that field, each chain extended to the feature.
        let mut drape_via: BTreeMap<String, Vec<String>> = BTreeMap::new();
        if let Some(sn) = &e.drape {
            let field_chains = chains.get(&chain_key("field", sn));
            let field_found: BTreeSet<String> = field_chains
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            let (mut fc, _) =
                via_model::report::causes_of(&old.build, new.build, &field_found, &BTreeSet::new());
            if fc.is_empty() {
                // No chain at hand for the field: its own entry's causes.
                fc = report
                    .entries
                    .iter()
                    .find(|f| f.table == "field" && &f.name == sn)
                    .map(|f| f.causes.clone())
                    .unwrap_or_default();
            }
            all.extend(fc);
            for (st, v) in field_chains.into_iter().flatten() {
                let mut v = v.clone();
                v.push(format!("attr:{}", e.name));
                drape_via.insert(st.clone(), v);
            }
        }
        if all.is_empty() {
            // Nothing it depends on differs in text: a statement moved to another layer, whose
            // rank orders its rows (`authority.md` 4). The moved statement is the cause.
            all.extend(moved_home.iter().cloned());
        }
        let mut causes: Vec<&String> = all.iter().collect();
        causes.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        debug_assert!(
            !causes_known || !causes.is_empty(),
            "change entry {} {} has no cause (build.md 11.2)",
            e.table,
            e.name
        );
        let causes_j: Vec<J> = causes
            .into_iter()
            .map(|c| {
                // One chain over the build's node edges (`build.md` 11.2), found by the graph.
                let via = chains
                    .get(&chain_key(&e.table, &e.name))
                    .and_then(|m| {
                        // The statement the cause is itself (`world.pack`) before one it is a
                        // key below (`world.extent` of the frame `world`).
                        m.get(c.as_str())
                            .or_else(|| {
                                m.iter()
                                    .find(|(st, _)| names_statement(c, st))
                                    .map(|(_, v)| v)
                            })
                            .cloned()
                    })
                    .or_else(|| {
                        drape_via.get(c.as_str()).cloned().or_else(|| {
                            drape_via
                                .iter()
                                .find(|(st, _)| names_statement(c, st))
                                .map(|(_, v)| v.clone())
                        })
                    })
                    .unwrap_or_default();
                cause_json(c, new, &old_side, via)
            })
            .collect();
        let mut m = J::obj();
        m.insert("after".into(), after);
        m.insert("before".into(), before);
        // `null` when the baseline's build record is not at hand (11.2, `graph.md` 6.4).
        m.insert(
            "causes".into(),
            if causes_known {
                J::Arr(causes_j)
            } else {
                J::Null
            },
        );
        m.insert("facts".into(), J::Arr(facts));
        m.insert("keys".into(), J::strs(keys.iter().map(String::as_str)));
        m.insert("name".into(), J::s(e.name.clone()));
        m.insert("region".into(), region_j);
        m.insert("table".into(), J::s(e.table.clone()));
        m.insert("what".into(), J::strs(what.iter().copied()));
        let row = rows
            .entry((maker_of(e, &old.build, new.build), e.table.clone()))
            .or_default();
        row.added += u64::from(s.added);
        row.removed += u64::from(s.removed);
        row.moved += u64::from(s.moved);
        row.swapped += u64::from(s.swapped);
        row.changed += u64::from(s.changed);
        for bx in [bb, ab, region_box].into_iter().flatten() {
            grow(&mut row.bx, bx);
        }
        entries.push((e.table.clone(), e.name.clone(), J::Obj(m)));
    }
    entries.sort_by(|a, b| {
        a.0.as_bytes()
            .cmp(b.0.as_bytes())
            .then(a.1.as_bytes().cmp(b.1.as_bytes()))
    });
    // Untouched things, and a row for every maker and table with a thing in either build.
    let things = |b: &Build| -> Vec<(Option<String>, String, String)> {
        let mut v = Vec::new();
        for n in b.features.keys() {
            v.push((maker_of_name(n), "feature".to_string(), n.clone()));
        }
        for n in b.fields.keys() {
            v.push((maker_of_name(n), "field".to_string(), n.clone()));
        }
        for (n, i) in &b.instances {
            let maker = i.maker.clone().or_else(|| maker_of_name(n));
            v.push((maker, "instance".to_string(), n.clone()));
        }
        for n in b.footprints.keys() {
            let maker = b
                .instances
                .get(n)
                .and_then(|i| i.maker.clone())
                .or_else(|| maker_of_name(n));
            v.push((maker, "footprint".to_string(), n.clone()));
        }
        v
    };
    let olds: BTreeSet<(Option<String>, String, String)> = things(&old.build).into_iter().collect();
    for t in things(new.build).into_iter().chain(olds.iter().cloned()) {
        rows.entry((t.0.clone(), t.1.clone())).or_default();
    }
    for t in things(new.build) {
        if olds.contains(&t) && !seen.contains(&(t.1.clone(), t.2.clone())) {
            if let Some(r) = rows.get_mut(&(t.0.clone(), t.1.clone())) {
                r.untouched += 1;
            }
        }
    }
    let summary: Vec<J> = rows
        .into_iter()
        .map(|((maker, table), r)| {
            let label = maker
                .clone()
                .unwrap_or_else(|| format!("{table} (written)"));
            let line = format!(
                "{label}: +{} -{} ~{} swapped {} changed {}, {} untouched",
                r.added, r.removed, r.moved, r.swapped, r.changed, r.untouched
            );
            let n = |v: u64| J::Num(v.to_string());
            obj([
                ("added", n(r.added)),
                ("box", r.bx.map_or(J::Null, box_json)),
                ("changed", n(r.changed)),
                ("line", J::s(line)),
                ("maker", J::opt(maker.as_deref())),
                ("moved", n(r.moved)),
                ("removed", n(r.removed)),
                ("swapped", n(r.swapped)),
                ("table", J::s(table)),
                ("untouched", n(r.untouched)),
            ])
        })
        .collect();
    obj([
        (
            "against",
            obj([("id", J::s(against.0)), ("key", J::s(against.1))]),
        ),
        (
            "entries",
            J::Arr(entries.into_iter().map(|(_, _, j)| j).collect()),
        ),
        ("format", J::s(FORMAT)),
        ("summary", J::Arr(summary)),
    ])
}
