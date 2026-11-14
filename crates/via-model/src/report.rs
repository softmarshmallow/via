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
    /// For a feature whose draped heights differ (`build.md` 11.2): the surface field it is
    /// draped on, whose own causes, found by the graph, are this entry's too (`identity.md` 5).
    pub drape: Option<String>,
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

/// The maker of a produced name (`identity.md` 1): the part before `#`; `None` for a name
/// written in text, which never holds `#`. A name from inside an import keeps its prefix
/// (`a.trees#s7000_4_7` is made by `a.trees`).
pub fn maker_of_name(name: &str) -> Option<String> {
    name.split_once('#').map(|(m, _)| m.to_string())
}

/// A build's surface (`build.md` 11.1): the field whose role is `surface`, as `features.json`
/// drapes points and path vertices on it.
struct Drape(Option<(via_num::Grid, via_raster::Raster<f32>)>);

impl Drape {
    fn of(b: &Build) -> Drape {
        Drape(b.grid.and_then(|g| {
            let f = b
                .fields
                .values()
                .find(|f| f.role.as_deref() == Some("surface"))?;
            let r = via_raster::raster::Raster::from_values(
                via_raster::window::Window::whole(&g),
                f.values.clone()?,
            )?;
            Some((g, r))
        }))
    }

    /// The height at each point in integer millimetres (`identity.md` 5: a position differs
    /// only in integer millimetres), `None` where there is no surface or the point is outside.
    fn heights(&self, points: &[(i64, i64)]) -> Vec<Option<i64>> {
        points
            .iter()
            .map(|p| {
                let (g, r) = self.0.as_ref()?;
                via_raster::raster::read_at(g, r, p.0, p.1).ok().map(mm)
            })
            .collect()
    }
}

/// The name of the build's surface field, whose statements are the causes of a draped height.
fn surface_name(b: &Build) -> Option<&String> {
    b.fields
        .iter()
        .find(|(_, f)| f.role.as_deref() == Some("surface"))
        .map(|(n, _)| n)
}

fn mm(z: f64) -> i64 {
    via_num::round_to_lattice(z * 1000.0).unwrap_or(0)
}

/// The facts of another asset a thing can depend on: a scattered instance gives way to a hand
/// footprint (`rules.md` 7.5), which reads that asset's footprint and its size, never its mounts.
const SHAPE_FACTS: [&str; 3] = ["bounds", "footprint", "size"];

/// The causes among a thing's statements (`identity.md` 5), and the differing facts of its own
/// assets (`own`, `build.md` 11.2): an asset's every fact is a cause for a thing of that asset; for
/// a thing of another asset, only the facts of its shape are.
pub fn causes_of(
    old: &Build,
    new: &Build,
    deps: &BTreeSet<String>,
    own: &BTreeSet<String>,
) -> (Vec<String>, BTreeMap<String, (String, String)>) {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut facts = BTreeMap::new();
    for s in deps {
        if let Some(asset) = s.strip_prefix("asset:") {
            let (a, b) = (old.assets.get(asset), new.assets.get(asset));
            let mine = own.contains(asset);
            if let (Some(a), Some(b)) = (a, b) {
                // `bounds` and `footprint` of a primitive or engine-made asset come from its
                // declared `size` (`assets.md` 7): when `size` differs, it is the cause and they
                // are facts only.
                let size_differs = a.get("size") != b.get("size") && b.contains_key("size");
                for (k, v) in b {
                    if !mine && !SHAPE_FACTS.contains(&k.as_str()) {
                        continue;
                    }
                    let o = a.get(k).cloned().unwrap_or_default();
                    if &o != v {
                        if !(size_differs && (k == "bounds" || k == "footprint")) {
                            out.insert(format!("asset.{asset}.{k}"));
                        }
                        if mine {
                            facts.insert(k.clone(), (o, v.clone()));
                        }
                    }
                }
            } else if a.is_some() != b.is_some() {
                // An asset added to the pack or taken from it (a scatter now draws it, or no
                // longer can): the asset itself is the cause.
                out.insert(format!("asset.{asset}"));
            }
            continue;
        }
        // A world key (`world.<key>`): the frame's keys and `pack` are each their own cause.
        if let Some(k) = s.strip_prefix("world.") {
            let get = |b: &Build| b.statements.get("world").and_then(|m| m.get(k)).cloned();
            if get(old) != get(new) {
                out.insert(s.clone());
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
                    deps: &BTreeSet<String>,
                    drape: Option<String>| {
        // The thing's own asset, before and after (`build.md` 11.2).
        let own: BTreeSet<String> = match table {
            "instance" | "footprint" => [old.instances.get(name), new.instances.get(name)]
                .into_iter()
                .flatten()
                .map(|i| i.asset.clone())
                .collect(),
            _ => BTreeSet::new(),
        };
        let (causes, facts) = causes_of(old, new, deps, &own);
        // `facts` belong to an instance or footprint whose asset's facts differ (`build.md`
        // 11.2); a field depends on an asset's facts, which may be its cause, but has none.
        let facts =
            if (status.changed || status.moved) && (table == "instance" || table == "footprint") {
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
            drape,
        });
    };
    // Features.
    let (drape_old, drape_new) = (Drape::of(old), Drape::of(new));
    let names: BTreeSet<&String> = old.features.keys().chain(new.features.keys()).collect();
    for n in names {
        let (a, b) = (old.features.get(n), new.features.get(n));
        // Heights as `features.json` writes them (`build.md` 11.2): a point's `at` and a path's
        // vertices are draped on the surface, so a surface edit under them moves a point (its
        // `at`) and changes a path (its `points`) though its plan is the same.
        let mut lifted = false;
        let status = match (a, b) {
            (Some(a), Some(b)) => {
                let same_plan = a.points == b.points && a.shape == b.shape;
                let z_differs = |pts_a: &[(i64, i64)], pts_b: &[(i64, i64)]| {
                    drape_old.heights(pts_a) != drape_new.heights(pts_b)
                };
                let point_z = same_plan
                    && a.shape == "point"
                    && z_differs(
                        &a.points[..a.points.len().min(1)],
                        &b.points[..b.points.len().min(1)],
                    );
                let path_z = same_plan && a.shape == "path" && z_differs(&a.points, &b.points);
                let names_z = a.names == b.names && {
                    let pa: Vec<(i64, i64)> = a.names.values().map(|v| v.1).collect();
                    let pb: Vec<(i64, i64)> = b.names.values().map(|v| v.1).collect();
                    z_differs(&pa, &pb)
                };
                lifted = point_z || path_z || names_z;
                Status {
                    moved: a.points != b.points
                        || a.rings != b.rings
                        || a.facing != b.facing
                        || point_z,
                    changed: a.kind != b.kind
                        || a.attrs != b.attrs
                        || a.names != b.names
                        || a.tier != b.tier
                        || path_z
                        || names_z,
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
        if status != Status::default() {
            // The surface it is draped on is what it depends on for its heights: its causes
            // are the surface field's, which the graph finds (the caller joins them).
            let drape = if lifted {
                surface_name(new).or(surface_name(old)).cloned()
            } else {
                None
            };
            push(
                "feature",
                n,
                status,
                a.and_then(|x| bbox(&x.all_points())),
                b.and_then(|x| bbox(&x.all_points())),
                &deps_of("feature", n),
                drape,
            );
        }
    }
    // Instances, with the summary per maker.
    let names: BTreeSet<&String> = old.instances.keys().chain(new.instances.keys()).collect();
    for n in names {
        let (a, b) = (old.instances.get(n), new.instances.get(n));
        let maker = a
            .or(b)
            .and_then(|x| x.maker.clone())
            .or_else(|| maker_of_name(n));
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
            // A hand edit is a cause like any key (`identity.md` 5): the clear or pin that
            // removed a produced instance, in this build or the baseline (its return).
            deps.extend(new.removed_by.get(n.as_str()).cloned());
            deps.extend(old.removed_by.get(n.as_str()).cloned());
            push(
                "instance",
                n,
                status,
                a.and_then(|x| bbox(&[x.origin])),
                b.and_then(|x| bbox(&[x.origin])),
                &deps,
                None,
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
                None,
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
            push(
                "field",
                n,
                status,
                region,
                region,
                &deps_of("field", n),
                None,
            );
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
