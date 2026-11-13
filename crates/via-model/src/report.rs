//! The change report (`identity.md` 5, `assets.md` 7): what changed against a baseline, by name,
//! each entry with its causes.
//!
//! Two builds hold the same thing when they hold the same name in the same table. A cause is a
//! statement the thing depends on whose text differs: an edited key (`feature.ford.on`), a table
//! added or removed (`clear.gap`), a world key (`world.pack`), or an asset fact
//! (`asset.<asset>.<fact>`). Causes are sorted bytewise. The `via` chains of `graph.md` 6.4 are
//! the graph's to add when the build runs through it.

use std::collections::{BTreeMap, BTreeSet};

use crate::build::Build;

/// Added, removed, or any of moved, swapped and changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub added: bool,
    pub removed: bool,
    pub moved: bool,
    pub swapped: bool,
    pub changed: bool,
}

/// One entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `feature`, `instance`, `footprint` or `field`.
    pub table: String,
    pub name: String,
    pub status: Status,
    /// Bounding boxes `[x0, y0, x1, y1]` in millimetres, before and after.
    pub before: Option<[i64; 4]>,
    pub after: Option<[i64; 4]>,
    pub causes: Vec<String>,
    /// Asset facts that differ: fact to (old, new).
    pub facts: BTreeMap<String, (String, String)>,
}

/// One summary line per maker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub added: u64,
    pub removed: u64,
    pub moved: u64,
    pub swapped: u64,
    pub changed: u64,
    pub untouched: u64,
}

impl Summary {
    /// `identity.md` 5's one fixed form.
    pub fn line(&self, maker: &str) -> String {
        format!(
            "{maker}: +{} -{} ~{} swapped {} changed {}, {} untouched",
            self.added, self.removed, self.moved, self.swapped, self.changed, self.untouched
        )
    }
}

/// The report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangeReport {
    /// False when there was no baseline (`identity.no_previous`).
    pub baseline: bool,
    pub entries: Vec<Entry>,
    pub summary: BTreeMap<String, Summary>,
}

fn bbox(points: &[(i64, i64)]) -> Option<[i64; 4]> {
    let first = points.first()?;
    let mut b = [first.0, first.1, first.0, first.1];
    for p in points {
        b[0] = b[0].min(p.0);
        b[1] = b[1].min(p.1);
        b[2] = b[2].max(p.0);
        b[3] = b[3].max(p.1);
    }
    Some(b)
}

fn mm(z: f64) -> i64 {
    via_num::round_to_lattice(z * 1000.0).unwrap_or(0)
}

/// The causes among a thing's statements (`identity.md` 5).
fn causes(
    old: &Build,
    new: &Build,
    deps: &BTreeSet<String>,
) -> (Vec<String>, BTreeMap<String, (String, String)>) {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut facts = BTreeMap::new();
    for s in deps {
        if let Some(asset) = s.strip_prefix("asset:") {
            let (a, b) = (old.assets.get(asset), new.assets.get(asset));
            if let (Some(a), Some(b)) = (a, b) {
                // `bounds` and `footprint` of a primitive or engine-made asset come from its
                // declared `size` (`assets.md` 7): when `size` differs, it is the cause and they
                // are facts only.
                let size_differs = a.get("size") != b.get("size") && b.contains_key("size");
                for (k, v) in b {
                    let o = a.get(k).cloned().unwrap_or_default();
                    if &o != v {
                        if !(size_differs && (k == "bounds" || k == "footprint")) {
                            out.insert(format!("asset.{asset}.{k}"));
                        }
                        facts.insert(k.clone(), (o, v.clone()));
                    }
                }
            }
            continue;
        }
        match (old.statements.get(s), new.statements.get(s)) {
            (Some(a), Some(b)) => {
                let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
                for k in keys {
                    if a.get(k) != b.get(k) {
                        out.insert(if k.is_empty() {
                            s.clone()
                        } else {
                            format!("{s}.{k}")
                        });
                    }
                }
            }
            (None, None) => {}
            _ => {
                out.insert(s.clone());
            }
        }
    }
    let mut v: Vec<String> = out.into_iter().collect();
    v.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    (v, facts)
}

/// Compares a build with its baseline.
pub fn compare(baseline: Option<&Build>, new: &Build) -> ChangeReport {
    let Some(old) = baseline else {
        return ChangeReport::default();
    };
    let mut entries = Vec::new();
    let mut summary: BTreeMap<String, Summary> = BTreeMap::new();
    let empty = BTreeSet::new();
    let deps_of = |table: &str, name: &str| -> BTreeSet<String> {
        let mut s: BTreeSet<String> = old
            .deps
            .get(&(table.to_string(), name.to_string()))
            .unwrap_or(&empty)
            .clone();
        s.extend(
            new.deps
                .get(&(table.to_string(), name.to_string()))
                .unwrap_or(&empty)
                .iter()
                .cloned(),
        );
        s
    };
    let mut push = |table: &str,
                    name: &str,
                    status: Status,
                    before: Option<[i64; 4]>,
                    after: Option<[i64; 4]>,
                    deps: &BTreeSet<String>| {
        let (causes, facts) = causes(old, new, deps);
        let facts = if status.changed || status.moved {
            facts
        } else {
            BTreeMap::new()
        };
        entries.push(Entry {
            table: table.into(),
            name: name.into(),
            status,
            before,
            after,
            causes,
            facts,
        });
    };
    // Features.
    let names: BTreeSet<&String> = old.features.keys().chain(new.features.keys()).collect();
    for n in names {
        let (a, b) = (old.features.get(n), new.features.get(n));
        let status = match (a, b) {
            (Some(a), Some(b)) => Status {
                moved: a.points != b.points || a.facing != b.facing,
                changed: a.kind != b.kind
                    || a.attrs != b.attrs
                    || a.names != b.names
                    || a.tier != b.tier,
                ..Status::default()
            },
            (None, Some(_)) => Status {
                added: true,
                ..Status::default()
            },
            (Some(_), None) => Status {
                removed: true,
                ..Status::default()
            },
            (None, None) => continue,
        };
        if status != Status::default() {
            push(
                "feature",
                n,
                status,
                a.and_then(|x| bbox(&x.points)),
                b.and_then(|x| bbox(&x.points)),
                &deps_of("feature", n),
            );
        }
    }
    // Instances, with the summary per maker.
    let names: BTreeSet<&String> = old.instances.keys().chain(new.instances.keys()).collect();
    for n in names {
        let (a, b) = (old.instances.get(n), new.instances.get(n));
        let maker = a.or(b).and_then(|x| x.maker.clone());
        let mut deps = deps_of("instance", n);
        let status = match (a, b) {
            (Some(a), Some(b)) => {
                let facts_differ =
                    old.assets.get(&a.asset) != new.assets.get(&b.asset) && a.asset == b.asset;
                Status {
                    moved: a.origin != b.origin || a.facing != b.facing || mm(a.z) != mm(b.z),
                    swapped: a.asset != b.asset,
                    changed: a.scale != b.scale || a.tint != b.tint || facts_differ,
                    ..Status::default()
                }
            }
            (None, Some(_)) => Status {
                added: true,
                ..Status::default()
            },
            (Some(_), None) => Status {
                removed: true,
                ..Status::default()
            },
            (None, None) => continue,
        };
        if let Some(m) = &maker {
            let s = summary.entry(m.clone()).or_default();
            if status == Status::default() {
                s.untouched += 1;
            }
            s.added += u64::from(status.added);
            s.removed += u64::from(status.removed);
            s.moved += u64::from(status.moved);
            s.swapped += u64::from(status.swapped);
            s.changed += u64::from(status.changed);
        }
        if status != Status::default() {
            if let Some(i) = b.or(a) {
                deps.insert(format!("asset:{}", i.asset));
            }
            push(
                "instance",
                n,
                status,
                a.and_then(|x| bbox(&[x.origin])),
                b.and_then(|x| bbox(&[x.origin])),
                &deps,
            );
        }
    }
    // Footprint features, under their instance's name.
    let names: BTreeSet<&String> = old.footprints.keys().chain(new.footprints.keys()).collect();
    for n in names {
        let (a, b) = (old.footprints.get(n), new.footprints.get(n));
        let status = match (a, b) {
            (Some(a), Some(b)) => Status {
                moved: bbox(&a.ring) != bbox(&b.ring) || a.ring != b.ring,
                changed: a.kind != b.kind,
                ..Status::default()
            },
            (None, Some(_)) => Status {
                added: true,
                ..Status::default()
            },
            (Some(_), None) => Status {
                removed: true,
                ..Status::default()
            },
            (None, None) => continue,
        };
        if status != Status::default() {
            let mut deps = deps_of("footprint", n);
            if let Some(i) = new.instances.get(n).or(old.instances.get(n)) {
                deps.insert(format!("asset:{}", i.asset));
            }
            push(
                "footprint",
                n,
                status,
                a.and_then(|x| bbox(&x.ring)),
                b.and_then(|x| bbox(&x.ring)),
                &deps,
            );
        }
    }
    // Fields: the box of the samples that changed.
    let names: BTreeSet<&String> = old.fields.keys().chain(new.fields.keys()).collect();
    for n in names {
        let (a, b) = (old.fields.get(n), new.fields.get(n));
        let (status, region) = match (a, b) {
            (Some(a), Some(b)) => {
                let changed = changed_box(old, new, a.values.as_deref(), b.values.as_deref());
                (
                    Status {
                        changed: changed.is_some() || a.weights != b.weights,
                        ..Status::default()
                    },
                    changed,
                )
            }
            (None, Some(_)) => (
                Status {
                    added: true,
                    ..Status::default()
                },
                None,
            ),
            (Some(_), None) => (
                Status {
                    removed: true,
                    ..Status::default()
                },
                None,
            ),
            (None, None) => continue,
        };
        if status != Status::default() {
            push("field", n, status, region, region, &deps_of("field", n));
        }
    }
    entries.sort_by(|x, y| {
        (x.table.as_bytes(), x.name.as_bytes()).cmp(&(y.table.as_bytes(), y.name.as_bytes()))
    });
    ChangeReport {
        baseline: true,
        entries,
        summary,
    }
}

fn changed_box(old: &Build, new: &Build, a: Option<&[f32]>, b: Option<&[f32]>) -> Option<[i64; 4]> {
    let (a, b) = (a?, b?);
    let (ga, gb) = (old.grid?, new.grid?);
    let mut pts = Vec::new();
    if ga != gb {
        let [x0, y0, x1, y1] = gb.extent;
        return Some([x0, y0, x1, y1]);
    }
    for k in 0..a.len().min(b.len()) {
        if a[k].to_bits() != b[k].to_bits() {
            let (i, j) = (k as u64 % gb.nx, k as u64 / gb.nx);
            pts.push(via_raster::window::post_mm(&gb, i, j));
        }
    }
    bbox(&pts)
}
