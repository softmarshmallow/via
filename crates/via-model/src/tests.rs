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
    // A clean build runs every node, the field's chunk among them.
    assert!(
        ran.iter()
            .any(|n| n.as_str().starts_with("field:height/v:build/w:")),
        "{ran:?}"
    );
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

/// No two nodes of one family are keyed on the same parts (`graph.md` 3.1). The members of a
/// run share every statement and edge; without their name as the key's argument the store gave
/// one member's footprint to every other (example `third_cottage`: `diag.internal`).
#[test]
fn no_two_nodes_share_a_key() {
    let w = WORLD.replace(
        "[scatter.growth]",
        "[run.row]\nasset  = \"box_a\"\nground = \"flat\"\non     = { ref = \"way_e\", offset = 8 }\ncount  = 3\n\n[scatter.growth]",
    );
    let d = world_dir(&w);
    let lo = LoadOptions {
        lib_roots: Vec::new(),
        via_lib: None,
        shipped_root: None,
        changes: Vec::new(),
    };
    let path = d.0.join("world.toml");
    let world = load(&path, 2, &lo).world.expect("merged");
    let plan = crate::graph::ModelPlan::new(&world, &Options::from_load(&path, &lo), None);
    let ids: Vec<String> = plan.ids().iter().map(ToString::to_string).collect();
    assert!(
        ids.iter()
            .filter(|n| n.starts_with("footprint:row#r"))
            .count()
            == 3,
        "{ids:?}"
    );
    assert_eq!(plan.key_clashes(), Vec::new());
}

const GRID: &str = r#"name       = "plain"
extent     = [0, 0, 64, 64]
resolution = 1
"#;

/// Three discs, the first of radius squared `r`, as a `where` condition.
fn discs(r: u32) -> String {
    format!(
        "{GRID}\n[field.height]\nrole = \"surface\"\nbase = \"= 10\"\n\n[feature.patch]\nwhere = \"= if((x - 10) * (x - 10) + (y - 10) * (y - 10) < {r}, 1, 0) + if((x - 40) * (x - 40) + (y - 12) * (y - 12) < 9, 1, 0) + if((x - 20) * (x - 20) + (y - 50) * (y - 50) < 9, 1, 0)\"\n"
    )
}

#[test]
fn where_keeps_every_piece_by_place() {
    // geometry.md 3.7: every piece kept; identity.md 2.3 and case 32: a piece that shrinks
    // renames nothing else.
    let d = world_dir(&discs(16));
    let (b, _) = run(&d, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    assert_eq!(b.features["patch"].rings.len(), 3);
    let d2 = world_dir(&discs(9));
    let (b2, _) = run(&d2, &[], None);
    assert_eq!(b2.features["patch"].rings.len(), 3);
    // The selection by place names the piece nearest `near`; the other two keep their names.
    let pick = |w: &str| {
        let d = world_dir(&format!(
            "{w}\n[feature.spot]\non = {{ of = \"patch\", near = [40, 30], t = 0, offset = 0 }}\n"
        ));
        let (b, _) = run(&d, &[], None);
        assert!(errors(&b).is_empty(), "{:?}", errors(&b));
        b.selections["feature.spot"].0.clone()
    };
    let a = pick(&discs(16));
    assert_eq!(a, pick(&discs(9)));
    assert!(a.starts_with("patch#p1_0_"), "{a}");
}

#[test]
fn where_reads_the_head_view_and_a_cycle_is_said_once() {
    // authority.md 2.3: a shape reads the head view; its own kind's effect does not move it.
    let w = format!(
        "{GRID}include = [\"marks\"]\n\n[field.height]\nrole = \"surface\"\nbase = \"= x / 4\"\n\n[field.height.then.lift]\nvalue = \"= v + 1\"\n\n[feature.low]\nkind  = \"strip\"\nwhere = \"= f('height') < 6\"\n"
    );
    let d = world_dir(&w);
    let (b, _) = run(&d, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    let ring = &b.features["low"].rings[0][0];
    assert!(ring.iter().all(|p| p.0 <= 20_000), "{ring:?}");
    assert!(ring.iter().any(|p| p.0 == 20_000), "{ring:?}");
    // authority.md case where_head_cycle: a row that reads the region it shapes.
    let w = format!(
        "{GRID}\n[field.height]\nrole = \"surface\"\nbase = \"= x / 4\"\n\n[field.height.then.lift]\nvalue = \"= v + 1\"\nmask  = \"= if(inside('r'), 1, 0)\"\n\n[feature.r]\nwhere = \"= f('height') < 6\"\n"
    );
    let d = world_dir(&w);
    let (b, _) = run(&d, &[], None);
    let e = errors(&b);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].starts_with("graph.cycle"), "{e:?}");
}

#[test]
fn follows_and_smooth() {
    let w = format!(
        "{GRID}\n[field.height]\nrole = \"surface\"\nbase = \"= 0\"\n\n[feature.line]\npath   = [[5, 5], [30, 20], [55, 10]]\nsmooth = true\n\n[feature.along]\nfollows = {{ ref = \"straight\", span = [0.25, 0.75], offset = 2 }}\n\n[feature.straight]\npath = [[0, 32], [64, 32]]\n\n[feature.bad]\nregion = [[0, 0], [4, 0], [4, 4]]\nsmooth = true\n"
    );
    let d = world_dir(&w);
    let (b, _) = run(&d, &[], None);
    // geometry.md 2.4: written points kept; each span in `ceil(c / 250 mm)` equal steps of the
    // curve parameter: 117 and 108 steps for chords of 29.155 m and 26.926 m.
    let pts = &b.features["line"].points;
    for p in [(5_000, 5_000), (30_000, 20_000), (55_000, 10_000)] {
        assert!(pts.contains(&p), "{p:?}");
    }
    assert_eq!(pts.len(), 117 + 108 + 1);
    // geometry.md 3.5: the substring, 2 m to the left.
    assert_eq!(
        b.features["along"].points,
        vec![(16_000, 34_000), (48_000, 34_000)]
    );
    let e = errors(&b);
    assert!(
        e.iter()
            .any(|x| x.starts_with("geom.smooth_not_path feature.bad")),
        "{e:?}"
    );
}

/// An array file of `<f4` (`build.md` 3.2), rows from the south.
fn npy(rows: usize, cols: usize, v: impl Fn(usize, usize) -> f32) -> Vec<u8> {
    let head = format!("{{'descr': '<f4', 'fortran_order': False, 'shape': ({rows}, {cols}), }}");
    let total = 10 + head.len() + 1;
    let pad = (64 - total % 64) % 64;
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend(((head.len() + pad + 1) as u16).to_le_bytes());
    out.extend(head.as_bytes());
    out.extend(std::iter::repeat_n(b' ', pad));
    out.push(b'\n');
    for r in 0..rows {
        for c in 0..cols {
            out.extend(v(r, c).to_le_bytes());
        }
    }
    out
}

#[test]
fn field_from_an_array_file() {
    // fields.md cases "array file base", "orientation", "file extent", "outside missing".
    let root = dir(&[(
        "world.toml",
        "name = \"f\"\nextent = [0, 0, 8, 8]\nresolution = 1\n\n[field.height]\nrole = \"surface\"\nfile = \"g.npy\"\noutside = 0\n\n[field.o]\nfile = \"o.npy\"\noutside = -1\nfilter = \"nearest\"\nextent = [0, 0, 4, 4]\n",
    )]);
    std::fs::write(root.0.join("g.npy"), npy(9, 9, |r, c| (r * 9 + c) as f32)).expect("w");
    std::fs::write(
        root.0.join("o.npy"),
        npy(2, 2, |r, c| if r == 0 && c == 0 { 1.0 } else { 0.0 }),
    )
    .expect("w");
    let (b, _) = run(&root, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    let h = b.fields["height"].values.as_ref().expect("scalar");
    assert!(h.iter().enumerate().all(|(k, v)| *v == k as f32));
    let o = b.fields["o"].values.as_ref().expect("scalar");
    assert_eq!(o[0], 1.0);
    assert_eq!(o[8], -1.0);
    assert_eq!(o[4 * 9 + 4], 0.0);
    let bad = dir(&[(
        "world.toml",
        "name = \"f\"\nextent = [0, 0, 8, 8]\nresolution = 1\n\n[field.a]\nfile = \"G.npy\"\noutside = 0\n\n[field.b]\nfile = \"g.npy\"\n\n[field.c]\nbase = \"= 0\"\nfilter = \"nearest\"\n",
    )]);
    std::fs::write(bad.0.join("g.npy"), npy(2, 2, |_, _| 0.0)).expect("w");
    let (b, _) = run(&bad, &[], None);
    let e = errors(&b);
    assert!(
        e.iter()
            .any(|x| x.starts_with("doc.path_case field.a")
                || x.starts_with("doc.file_missing field.a")),
        "{e:?}"
    );
    assert!(
        e.iter()
            .any(|x| x.starts_with("field.file_outside field.b")),
        "{e:?}"
    );
    assert!(
        e.iter().any(|x| x.starts_with("field.file_key field.c")),
        "{e:?}"
    );
}

#[test]
fn hand_edits_select_a_scatters_instance() {
    // identity.md cases 7, 10 and 15: a clear and a pin select by place from the run stage.
    let extra = "\n[clear.one]\non = { of = \"growth\", near = [30, 20] }\n\n[pin.bench]\nasset = \"stem\"\non = { of = \"growth\", near = [12, 40] }\nby = [\"growth\"]\n";
    let d = world_dir(&format!("{WORLD}{extra}"));
    let (b, _) = run(&d, &[], None);
    assert!(errors(&b).is_empty(), "{:?}", errors(&b));
    let (gone, at) = b.selections["clear.one"].clone();
    assert!(!b.instances.contains_key(&gone), "{gone}");
    let (under, at2) = b.selections["pin.bench"].clone();
    assert!(gone.starts_with("growth#") && under.starts_with("growth#"));
    assert_eq!(b.instances["bench"].origin, at2);
    assert_ne!(at, at2);
    let missing = "\n[scatter.none]\nassets = [\"stem\"]\nspacing = 9\ndensity = \"= 0\"\n\n[pin.p1]\nasset = \"stem\"\non = { of = \"none\", near = [10, 10] }\n\n[pin.p2]\nasset = \"stem\"\non = { of = \"none\", near = [20, 10] }\n\n[pin.p3]\nasset = \"stem\"\non = { of = \"growht\", near = [20, 10] }\n";
    let d = world_dir(&format!("{WORLD}{missing}"));
    let (b, _) = run(&d, &[], None);
    let e = errors(&b);
    assert_eq!(
        e.iter()
            .filter(|x| x.starts_with("identity.near_none scatter.none"))
            .count(),
        1,
        "{e:?}"
    );
    assert!(
        e.iter()
            .any(|x| x.starts_with("identity.unknown_maker pin.p3")),
        "{e:?}"
    );
}
