//! The model's cases, through `via_doc::load` and [`crate::build`], on worlds written in neutral
//! words (the deny-list reads string literals, `vocabulary.md` 7). The command-line cases of the
//! chapters are the conformance runner's; these hold the mechanisms at the crate.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use via_doc::{load, ChangeRequest, LoadOptions};

use crate::{build, Build, Options};

static NEXT: AtomicU32 = AtomicU32::new(0);

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dir(files: &[(&str, &str)]) -> Dir {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("via-model-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (p, text) in files {
        let path = root.join(p);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
    Dir(root)
}

const KINDS: &str = r#"
[kind.strip]
authority = "shaped"
radius    = 8

[kind.strip.defaults]
width = 3

[kind.strip.effect.bed]
field = "role:surface"
value = "= v - 1"
mask  = "= 1 - smoothstep(0, width, d)"

[kind.knot]
authority = "anchored"
radius    = 6

[kind.knot.effect.approach]
field = "role:surface"
value = "= v"
mask  = "= 1 - smoothstep(0, 4, d)"

[kind.flat]
authority = "shaped"
radius    = 3

[kind.flat.effect.level]
field = "role:surface"
value = "= under('mean')"

[kind.flat.effect.bare]
field = "role:keep_out"
value = 1
mask  = "= if(d <= 0, 1, 0)"

[kind.marker]
output   = "mark"
requires = ["toward"]
names    = ["toward"]

[kind.digit]
radius = 40

[kind.digit.defaults]
k = 1

[kind.digit.effect.put]
field = "tag:digits"
value = "= v * 10 + k"

[kind.digit_s]
authority = "shaped"
radius    = 40

[kind.digit_s.defaults]
k = 2

[kind.digit_s.effect.put]
field = "tag:digits"
value = "= v * 10 + k"

[kind.digit_a]
authority = "anchored"
radius    = 40

[kind.digit_a.defaults]
k = 3

[kind.digit_a.effect.put]
field = "tag:digits"
value = "= v * 10 + k"
"#;

const PACK: &str = r#"
[taxon."veg/tall"]
[taxon."prop"]

[asset.box_a]
primitive = "box"
size      = [8, 6, 4]
taxon     = "prop"

[asset.box_a.mount.front]
at = [0, 3]

[asset.stem]
primitive = "sphere"
size      = [5, 5, 7]
taxon     = "veg/tall"
footprint = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]
"#;

const WORLD: &str = r#"name    = "slice"
extent  = [0, 0, 64, 64]
resolution = 1
include = ["marks"]
pack    = "p"

[field.height]
role = "surface"
base = "= 20 + 0.05 * y"

[field.clearing]
role = "keep_out"
base = "= 0"

[feature.course]
kind      = "strip"
authority = "anchored"
path      = [[30, 64], [32, 0]]

[feature.knot]
kind = "knot"
on   = { ref = "course", t = 0.5 }

[feature.way_w]
kind    = "strip"
path    = [[0, 30]]
ends_at = "knot"

[feature.way_e]
kind      = "strip"
starts_at = "knot"
path      = [[64, 34]]

[feature.look]
kind   = "marker"
on     = { ref = "way_w", t = 0.9 }
toward = "knot"

[instance.hut]
asset  = "box_a"
ground = "flat"
margin = 1
on     = { ref = "way_w", t = 0.4, offset = 8 }
face   = "toward"

[scatter.growth]
assets  = ["@veg/tall"]
spacing = 5
density = "= 0.8"

[pin.kept]
asset = "stem"
at    = [50, 50]
by    = ["growth"]

[clear.gap]
at     = [10, 50]
radius = 4
by     = ["growth"]

[change.moved.feature.knot]
on = { ref = "course", t = 0.7 }

[change.denser.scatter.growth]
spacing = 4
"#;

fn world_dir(world: &str) -> Dir {
    dir(&[
        ("world.toml", world),
        ("lib/marks/kinds.toml", KINDS),
        ("lib/packs/p.toml", PACK),
    ])
}

fn run(d: &Dir, changes: &[&str], baseline: Option<&Build>) -> (Build, Vec<String>) {
    let lo = LoadOptions {
        lib_roots: Vec::new(),
        via_lib: None,
        shipped_root: None,
        changes: changes
            .iter()
            .enumerate()
            .map(|(i, c)| ChangeRequest::new(*c, i as u32 + 3))
            .collect(),
    };
    let path = d.0.join("world.toml");
    let loaded = load(&path, 2, &lo);
    let Some(world) = loaded.world else {
        let codes: Vec<String> = loaded
            .records
            .iter()
            .map(|r| r.code().to_string())
            .collect();
        panic!("the world did not merge: {codes:?}");
    };
    let b = build(&world, &Options::from_load(&path, &lo), baseline);
    let codes = b.diagnostics.iter().map(|x| x.code().to_string()).collect();
    (b, codes)
}

fn errors(b: &Build) -> Vec<String> {
    b.diagnostics
        .iter()
        .filter(|d| d.severity() == via_diag::Severity::Error)
        .map(|d| format!("{} {} {}", d.code(), d.subject(), d.message().as_str()))
        .collect()
}

#[test]
fn slice_builds() {
    let d = world_dir(WORLD);
    let (b, codes) = run(&d, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    assert!(codes.contains(&"identity.no_previous".to_string()));
    // Every feature, the footprint, the instances.
    for f in ["course", "knot", "way_w", "way_e", "look"] {
        assert!(b.features.contains_key(f), "{f}");
    }
    assert!(b.footprints.contains_key("hut"));
    let hut = &b.instances["hut"];
    assert_eq!(hut.maker, None);
    // The ways end at the knot's point.
    let knot = b.features["knot"].points[0];
    assert_eq!(*b.features["way_w"].points.last().expect("pts"), knot);
    assert_eq!(b.features["way_e"].points[0], knot);
    // The name attribute resolves to the knot's point.
    assert_eq!(
        b.features["look"].names["toward"],
        ("knot".to_string(), knot)
    );
    // Two meetings, ordered.
    assert_eq!(b.meetings.len(), 2);
    assert_eq!(b.meetings[0].gap, 0);
    // The pin stands, nothing of the scatter inside the clear.
    assert!(b.instances.contains_key("kept"));
    for (n, i) in &b.instances {
        if i.maker.as_deref() == Some("growth") {
            assert!(n.starts_with("growth#s5000_"), "{n}");
            let (dx, dy) = (i.origin.0 - 10_000, i.origin.1 - 50_000);
            assert!(dx * dx + dy * dy > 4_000 * 4_000, "{n} inside the clear");
        }
    }
    assert!(b.instances.values().filter(|i| i.maker.is_some()).count() > 10);
}

#[test]
fn same_text_same_build() {
    let d = world_dir(WORLD);
    let (a, _) = run(&d, &[], None);
    let (b, _) = run(&d, &[], None);
    assert_eq!(a.fields, b.fields);
    assert_eq!(a.instances, b.instances);
    assert_eq!(a.features, b.features);
}

#[test]
fn footprint_levels_and_keeps_out() {
    let d = world_dir(WORLD);
    let (b, codes) = run(&d, &[], None);
    // The footprint keeps the scatter off it, and no `place.keep_out` for its own instance.
    assert!(!codes.contains(&"place.keep_out".to_string()), "{codes:?}");
    let ring = &b.footprints["hut"].ring;
    let xs: Vec<i64> = ring.iter().map(|p| p.0).collect();
    let ys: Vec<i64> = ring.iter().map(|p| p.1).collect();
    let (x0, x1) = (*xs.iter().min().expect("x"), *xs.iter().max().expect("x"));
    let (y0, y1) = (*ys.iter().min().expect("y"), *ys.iter().max().expect("y"));
    for (n, i) in &b.instances {
        if i.maker.is_some() {
            let (x, y) = i.origin;
            assert!(
                !(x > x0 && x < x1 && y > y0 && y < y1),
                "{n} on the footprint"
            );
        }
    }
    // The ground is level under the footprint's interior posts.
    let g = b.grid.expect("grid");
    let h = b.fields["height"].values.as_ref().expect("values");
    let mut seen = Vec::new();
    for j in 0..g.ny {
        for i in 0..g.nx {
            let (x, y) = via_raster::window::post_mm(&g, i, j);
            if x > x0 + 1000 && x < x1 - 1000 && y > y0 + 1000 && y < y1 - 1000 {
                seen.push(h[(j * g.nx + i) as usize]);
            }
        }
    }
    assert!(!seen.is_empty());
    assert!(seen.iter().all(|v| *v == seen[0]), "not level: {seen:?}");
}

#[test]
fn tiers_order_entries() {
    let w = r#"name = "o"
extent = [0, 0, 8, 8]
resolution = 1
include = ["marks"]

[field.n]
tags = ["digits"]
base = "= 0"

[feature.c]
kind = "digit_a"
at = [4, 4]

[feature.b]
kind = "digit_s"
at = [4, 4]

[feature.a]
kind = "digit"
at = [4, 4]
"#;
    let d = world_dir(w);
    let (b, _) = run(&d, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    let v = b.fields["n"].values.as_ref().expect("values");
    assert!(v.iter().all(|x| *x == 123.0), "{:?}", &v[..4]);
}

#[test]
fn anchored_regions_conflict() {
    let w = r#"name = "c"
extent = [0, 0, 16, 16]
resolution = 1
include = ["marks"]

[field.n]
tags = ["digits"]
base = "= 0"

[feature.left]
kind = "digit_a"
region = [[0, 0], [10, 0], [10, 10], [0, 10]]

[feature.right]
kind = "digit_a"
k    = 5
region = [[6, 0], [16, 0], [16, 10], [6, 10]]
"#;
    let d = world_dir(w);
    let (b, codes) = run(&d, &[], None);
    assert!(
        codes.contains(&"authority.conflict".to_string()),
        "{codes:?}"
    );
    let c = b
        .diagnostics
        .iter()
        .find(|x| x.code().to_string() == "authority.conflict")
        .expect("conflict");
    assert_eq!(c.subject().to_string(), "feature.right");
}

#[test]
fn change_report_names_causes() {
    let d = world_dir(WORLD);
    let (a, _) = run(&d, &[], None);
    let (b, _) = run(&d, &["moved"], Some(&a));
    let r = b.changes.as_ref().expect("report");
    assert!(r.baseline);
    let find = |t: &str, n: &str| r.entries.iter().find(|e| e.table == t && e.name == n);
    let knot = find("feature", "knot").expect("knot");
    assert!(knot.status.moved);
    assert!(
        knot.causes.contains(&"feature.knot.on".to_string()),
        "{:?}",
        knot.causes
    );
    let way = find("feature", "way_w").expect("way_w");
    assert!(way.status.moved);
    assert!(way.causes.contains(&"feature.knot.on".to_string()));
    let look = find("feature", "look").expect("look");
    assert!(look.status.changed || look.status.moved);
    assert!(find("field", "height").is_some_and(|e| e.status.changed));
    assert!(find("feature", "course").is_none());
    // Null: the same text against itself changes nothing.
    let (c, _) = run(&d, &[], Some(&a));
    let rc = c.changes.as_ref().expect("report");
    assert!(
        rc.entries.is_empty(),
        "{:?}",
        rc.entries
            .iter()
            .map(|e| (&e.table, &e.name))
            .collect::<Vec<_>>()
    );
}

#[test]
fn spacing_renames_under_hand_edits() {
    let d = world_dir(WORLD);
    let (a, _) = run(&d, &[], None);
    let (b, _) = run(&d, &["denser"], Some(&a));
    let r = b.changes.as_ref().expect("report");
    for e in &r.entries {
        if e.table == "instance" && e.name.starts_with("growth#") {
            if e.name.starts_with("growth#s5000_") {
                assert!(e.status.removed, "{}", e.name);
            } else {
                assert!(
                    e.status.added && e.name.starts_with("growth#s4000_"),
                    "{}",
                    e.name
                );
                assert!(e.causes.contains(&"scatter.growth.spacing".to_string()));
            }
        }
    }
    assert!(b.instances.contains_key("kept"));
    assert!(r.entries.iter().all(|e| e.name != "kept"));
}

#[test]
fn faults_are_located_values() {
    let bad = WORLD
        .replace("asset  = \"box_a\"", "asset  = \"box_b\"")
        .replace("pack    = \"p\"", "pack    = \"q\"");
    let d = world_dir(&bad);
    let (_, codes) = run(&d, &[], None);
    assert!(codes.contains(&"pack.not_found".to_string()), "{codes:?}");
    let bad = WORLD.replace("asset  = \"box_a\"", "asset  = \"box_b\"");
    let d = world_dir(&bad);
    let (b, codes) = run(&d, &[], None);
    assert!(codes.contains(&"asset.unknown".to_string()), "{codes:?}");
    let u = b
        .diagnostics
        .iter()
        .find(|x| x.code().to_string() == "asset.unknown")
        .expect("unknown");
    assert!(u.hint().as_str().contains("box_a"), "{}", u.hint().as_str());
}

#[test]
fn field_cycle() {
    let w = r#"name = "y"
extent = [0, 0, 8, 8]
resolution = 1

[field.a]
base = "= f('b')"

[field.b]
base = "= f('a')"
"#;
    let d = world_dir(w);
    let (_, codes) = run(&d, &[], None);
    assert!(codes.contains(&"graph.cycle".to_string()), "{codes:?}");
}

fn graph_run(
    d: &Dir,
    store: &via_graph::Store,
    changes: &[&str],
) -> (Build, std::collections::BTreeSet<via_graph::NodeId>) {
    let lo = LoadOptions {
        lib_roots: Vec::new(),
        via_lib: None,
        shipped_root: None,
        changes: changes
            .iter()
            .enumerate()
            .map(|(i, c)| ChangeRequest::new(*c, i as u32 + 3))
            .collect(),
    };
    let path = d.0.join("world.toml");
    let world = load(&path, 2, &lo).world.expect("merged");
    let opts = via_graph::Options::new(via_graph::Versions {
        document: via_doc::LANGUAGE.into(),
        protocol: "via-step-1".into(),
        build: "test".into(),
        epoch: 0,
    });
    let (b, run) = crate::build_graph(&world, &Options::from_load(&path, &lo), None, store, &opts);
    (b, run.expect("ran").ran())
}

#[test]
fn graph_reuses_unchanged_fields() {
    let d = world_dir(WORLD);
    let store = via_graph::Store::open(d.0.join("store"));
    let (a, ran) = graph_run(&d, &store, &[]);
    assert_eq!(ran.len(), 2, "{ran:?}");
    // The same build as in memory.
    let (m, _) = run(&d, &[], None);
    assert_eq!(a.fields, m.fields);
    assert_eq!(a.instances, m.instances);
    // Nothing changed: no field runs.
    let (_, ran) = graph_run(&d, &store, &[]);
    assert!(ran.is_empty(), "{ran:?}");
    // A moved knot re-runs the field it shapes.
    let (_, ran) = graph_run(&d, &store, &["moved"]);
    assert!(
        ran.iter()
            .any(|n| n.to_string().starts_with("field:height/")),
        "{ran:?}"
    );
}
