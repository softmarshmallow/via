//! The files of a build directory (`build.md` 2 to 9) from an in-memory build: every array file,
//! `features.json`, `meets.json`, and the world keys of `manifest.json`. What is written depends
//! on the build only, never on where it is written or how it was asked for (rules 3 and 6); the
//! caller adds `files`, `id`, `run` and `locations` (sections 4 and 12).

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Diag, Site, Subject};
use via_doc::World;
use via_model::pack::Pack;
use via_model::{Build, FeatureOut};
use via_num::Grid;
use via_raster::{Raster, Window};

use crate::json::{obj, J};
use crate::layout::Layout;
use crate::npy;
use crate::txt;

/// The build format (`build.md` 4): the build format version of `graph.md` 3.3.
pub const FORMAT: &str = "via-build-0";

/// The numerics epoch (`graph.md` 3.3): raised by any engine change that may change the bytes of
/// a value. Written in `manifest.json`'s `engine` and in every key.
pub const EPOCH: u64 = 1;

/// The epoch this binary builds with: [`EPOCH`], and in a test build (debug assertions on) plus
/// `VIA_TEST_EPOCH_OFFSET` when it is set to an integer (`graph.md` 7, case `epoch`). It goes
/// into every node key, the build key, the manifest's `engine` and the run report's
/// `engine.epoch`, so a raised epoch is seen to reuse nothing. A release binary ignores the
/// variable.
pub fn epoch() -> u64 {
    if cfg!(debug_assertions) {
        if let Some(off) = std::env::var("VIA_TEST_EPOCH_OFFSET")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            return EPOCH.saturating_add(off);
        }
    }
    EPOCH
}

/// Whether a build path is a file of the `id` rather than a report or `.previous`.
pub fn is_listed_file(p: &str) -> bool {
    !matches!(
        p,
        "manifest.json" | "changes.json" | "diagnostics.json" | ".previous"
    )
}

/// What the writer is handed.
pub struct Input<'a> {
    pub world: &'a World,
    pub build: &'a Build,
    pub pack: Option<&'a Pack>,
    pub name: &'a str,
    pub seed: u64,
}

/// The files of the `id` by build path, and the manifest's world keys.
pub struct Written {
    pub files: BTreeMap<String, Vec<u8>>,
    pub head: BTreeMap<String, J>,
}

fn internal(message: String) -> Diag {
    Diag::new(
        codes::build::INTERNAL,
        Subject::World,
        Site::World,
        txt(message),
        via_diag::text!("a defect of ours: report it with the world that gives it"),
    )
}

/// A path segment with the escape of 3.4: a Windows device name before the first `.` gets `-`.
pub fn segment(name: &str) -> String {
    let (stem, rest) = match name.find('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    let device = matches!(stem, "con" | "prn" | "aux" | "nul")
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if device {
        format!("{stem}-{rest}")
    } else {
        name.to_string()
    }
}

fn finite32(v: f32, what: &str, at: usize) -> Result<f32, Diag> {
    if v.is_finite() {
        Ok(if v == 0.0 { 0.0 } else { v })
    } else {
        Err(internal(format!(
            "a value that is not finite reached the writer: {what}, sample {at}"
        )))
    }
}

/// `min` and `max` over every post, widened to doubles.
fn min_max(values: &[f32]) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for v in values {
        if *v < lo {
            lo = *v;
        }
        if *v > hi {
            hi = *v;
        }
    }
    (lo, hi)
}

/// The bytes of every material at one post (6.2): largest remainder over doubles, ties to the
/// lower material id. `w` is in material id order.
pub fn post_bytes(w: &[f64]) -> Vec<u8> {
    let mut s = 0.0f64;
    for x in w {
        s += *x;
    }
    if s == 0.0 {
        return vec![0; w.len()];
    }
    let mut bytes = Vec::with_capacity(w.len());
    let mut rems: Vec<(f64, usize)> = Vec::with_capacity(w.len());
    let mut total: u32 = 0;
    for (m, x) in w.iter().enumerate() {
        let q = *x * 255.0 / s;
        let f = q.floor();
        let b = f as u32;
        total += b;
        bytes.push(b);
        rems.push((q - f, m));
    }
    // Largest remainder first; equal remainders to the lower id.
    rems.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut i = 0;
    while total < 255 && !rems.is_empty() {
        bytes[rems[i % rems.len()].1] += 1;
        total += 1;
        i += 1;
    }
    bytes
        .into_iter()
        .map(|b| u8::try_from(b).unwrap_or(255))
        .collect()
}

/// The canonical rings of 2.5: counter-clockwise, starting at the least vertex by `x` then `y`.
pub fn canonical_ring(ring: &[(i64, i64)], ccw: bool) -> Vec<(i64, i64)> {
    if ring.is_empty() {
        return Vec::new();
    }
    let mut r: Vec<(i64, i64)> = ring.to_vec();
    if r.len() > 1 && r.first() == r.last() {
        r.pop();
    }
    let mut area: i128 = 0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        area += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    if (area < 0) == ccw {
        r.reverse();
    }
    let start = (0..r.len()).min_by_key(|i| r[*i]).unwrap_or(0);
    r.rotate_left(start);
    r
}

/// An attribute's text as JSON: a number or a bool as itself, any other text a string.
pub fn attr_json(text: &str) -> J {
    match text {
        "true" => return J::Bool(true),
        "false" => return J::Bool(false),
        _ => {}
    }
    if text.starts_with(|c: char| c == '-' || c.is_ascii_digit()) {
        if let Ok(J::Num(n)) = crate::json::parse(text) {
            return J::Num(n);
        }
    }
    J::s(text)
}

/// A fact's text (`assets.md` 7) as JSON: a list or a number as itself, else a string.
pub fn fact_json(text: &str) -> J {
    // Facts are recorded as JSON text in the manifest's form (`build.md` 11.2), never a string
    // standing for a value of another form.
    match crate::json::parse(text) {
        Ok(v) => normalise_numbers(v),
        Err(_) => {
            debug_assert!(false, "a fact recorded as `{text}` is not JSON");
            J::Null
        }
    }
}

fn normalise_numbers(v: J) -> J {
    match v {
        J::Num(n) => n.parse::<f64>().ok().and_then(J::dbl).unwrap_or(J::Num(n)),
        J::Arr(a) => J::Arr(a.into_iter().map(normalise_numbers).collect()),
        other => other,
    }
}

/// The surface field's samples, for the height beneath a feature (9.1).
pub struct Surface {
    grid: Grid,
    raster: Option<Raster<f32>>,
}

impl Surface {
    pub fn new(build: &Build, grid: Grid) -> Surface {
        let raster = build
            .fields
            .values()
            .find(|f| f.role.as_deref() == Some("surface"))
            .and_then(|f| f.values.clone())
            .and_then(|v| Raster::from_values(Window::whole(&grid), v));
        Surface { grid, raster }
    }

    fn z(&self, p: (i64, i64)) -> J {
        self.raster
            .as_ref()
            .and_then(|r| via_raster::raster::read_at(&self.grid, r, p.0, p.1).ok())
            .and_then(J::dbl)
            .unwrap_or(J::Null)
    }

    pub fn xyz(&self, p: (i64, i64)) -> J {
        J::Arr(vec![J::mm(p.0), J::mm(p.1), self.z(p)])
    }
}

fn xy(p: (i64, i64)) -> J {
    J::Arr(vec![J::mm(p.0), J::mm(p.1)])
}

pub fn pieces(ring: &[(i64, i64)]) -> J {
    J::Arr(vec![obj([
        ("holes", J::Arr(Vec::new())),
        (
            "outer",
            J::Arr(canonical_ring(ring, true).into_iter().map(xy).collect()),
        ),
    ])])
}

/// Every piece of a region, shell then holes, in canonical form (9.1).
fn all_pieces(rings: &[Vec<Vec<(i64, i64)>>]) -> J {
    J::Arr(
        rings
            .iter()
            .filter(|pc| !pc.is_empty())
            .map(|pc| {
                obj([
                    (
                        "holes",
                        J::Arr(
                            pc[1..]
                                .iter()
                                .map(|h| {
                                    J::Arr(canonical_ring(h, false).into_iter().map(xy).collect())
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "outer",
                        J::Arr(canonical_ring(&pc[0], true).into_iter().map(xy).collect()),
                    ),
                ])
            })
            .collect(),
    )
}

/// One feature of `features.json` (9.1).
pub fn feature_json(name: &str, f: &FeatureOut, layer: Option<&str>, surface: &Surface) -> J {
    let mut m = J::obj();
    m.insert("name".into(), J::s(name));
    m.insert("kind".into(), J::opt(f.kind.as_deref()));
    m.insert(
        "authority".into(),
        J::opt(f.tier.map(|t| match t {
            via_model::Tier::Derived => "derived",
            via_model::Tier::Shaped => "shaped",
            via_model::Tier::Anchored => "anchored",
        })),
    );
    m.insert("layer".into(), J::opt(layer));
    m.insert("shape".into(), J::s(f.shape.clone()));
    match f.shape.as_str() {
        "point" => {
            if let Some(p) = f.points.first() {
                m.insert("at".into(), surface.xyz(*p));
            }
        }
        "path" => {
            m.insert(
                "points".into(),
                J::Arr(f.points.iter().map(|p| surface.xyz(*p)).collect()),
            );
        }
        _ if !f.rings.is_empty() => {
            m.insert("pieces".into(), all_pieces(&f.rings));
        }
        // An empty region (a `holds` that holds nothing, `derive.dry`) has no piece.
        _ if f.points.is_empty() => {
            m.insert("pieces".into(), J::Arr(Vec::new()));
        }
        _ => {
            m.insert("pieces".into(), pieces(&f.points));
        }
    }
    let mut attrs = J::obj();
    for (k, v) in &f.attrs {
        attrs.insert(k.clone(), attr_json(v));
    }
    for (k, (place, p)) in &f.names {
        attrs.insert(
            k.clone(),
            obj([("at", surface.xyz(*p)), ("name", J::s(place.clone()))]),
        );
    }
    m.insert("attrs".into(), J::Obj(attrs));
    J::Obj(m)
}

fn kind_authority(world: &World, kind: &str) -> J {
    world
        .entity("kind", kind)
        .and_then(|t| t.get("authority"))
        .and_then(|e| match &e.item.value {
            via_doc::Value::Str(s) => Some(J::s(s.text.clone())),
            _ => None,
        })
        .unwrap_or_else(|| J::s("derived"))
}

fn asset_json(a: &via_model::pack::Asset, stand_ins: bool) -> J {
    let mut m = J::obj();
    // `build.md` 7, `assets.md` 9: a model asset built with `--stand-ins` is drawn as a box of
    // its accepted `bounds` in its `colour`.
    m.insert("stand_in".into(), J::Bool(stand_ins && a.model.is_some()));
    m.insert("source".into(), J::s(a.source.clone()));
    // `build.md` 7: the model, then each `detail` copy, by pack path and digest (M2).
    let files = a.model.as_ref().map_or_else(Vec::new, |md| {
        md.copies
            .iter()
            .map(|(path, digest)| {
                obj([
                    ("digest", J::s(digest.clone())),
                    ("path", J::s(path.clone())),
                ])
            })
            .collect()
    });
    m.insert("files".into(), J::Arr(files));
    m.insert("primitive".into(), J::opt(a.primitive.as_deref()));
    // `size` is a primitive's or an engine-made asset's statement; a model's is measured.
    m.insert(
        "size".into(),
        if a.model.is_some() {
            J::Null
        } else {
            J::Arr(a.size.iter().map(|v| J::mm(*v)).collect())
        },
    );
    // The quarter turn of `front` and `up`, then minus the pivot in exact metres.
    let mut to_asset = Vec::new();
    for row in a.to_asset() {
        for (k, v) in row.iter().enumerate() {
            to_asset.push(if k == 3 { J::mm(*v) } else { J::int(*v) });
        }
    }
    m.insert("to_asset".into(), J::Arr(to_asset));
    m.insert(
        "bounds".into(),
        J::Arr(a.bounds.iter().map(|v| J::mm(*v)).collect()),
    );
    m.insert(
        "footprint".into(),
        J::Arr(a.footprint.iter().map(|p| xy(*p)).collect()),
    );
    m.insert(
        "origin".into(),
        J::Arr(a.origin.iter().map(|v| J::mm(*v)).collect()),
    );
    let mut triangles = vec![J::Num(a.triangles.to_string())];
    if let Some(md) = &a.model {
        triangles.extend(md.detail_triangles.iter().map(|t| J::Num(t.to_string())));
    }
    m.insert("triangles".into(), J::Arr(triangles));
    m.insert("collision".into(), J::s(a.collision.clone()));
    let mut mounts = J::obj();
    for (n, (at, face)) in &a.mounts {
        mounts.insert(n.clone(), obj([("at", xy(*at)), ("face", J::int(*face))]));
    }
    m.insert("mounts".into(), J::Obj(mounts));
    m.insert(
        "solidity".into(),
        J::s(if a.solid { "solid" } else { "none" }),
    );
    m.insert(
        "colour".into(),
        J::Arr(
            a.colour
                .iter()
                .map(|c| J::dbl(*c).unwrap_or(J::Null))
                .collect(),
        ),
    );
    m.insert("taxon".into(), J::opt(a.taxon.as_deref()));
    m.insert("tags".into(), J::strs(a.tags.iter().map(String::as_str)));
    m.insert(
        "params".into(),
        a.params.as_ref().map_or_else(
            || J::Obj(J::obj()),
            |p| {
                let text = via_model::asset_facts::canonical_json(&toml::Value::Table(p.clone()));
                crate::json::parse(&text).unwrap_or_else(|_| J::Obj(J::obj()))
            },
        ),
    );
    J::Obj(m)
}

/// Writes every file of the `id` and the manifest's world keys.
pub fn write(input: &Input<'_>) -> Result<Written, Diag> {
    let b = input.build;
    let grid = b
        .grid
        .ok_or_else(|| internal("a build with no grid reached the writer".into()))?;
    let layout = Layout::new(&grid);
    let (nx, ny) = (
        usize::try_from(grid.nx).unwrap_or(0),
        usize::try_from(grid.ny).unwrap_or(0),
    );
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut head = J::obj();
    head.insert("format".into(), J::s(FORMAT));
    head.insert(
        "engine".into(),
        obj([
            ("epoch", J::Num(epoch().to_string())),
            ("name", J::s("via")),
        ]),
    );
    head.insert("world".into(), J::s(input.name));
    head.insert("seed".into(), J::s(via_num::write_seed(input.seed)));
    let [x0, y0, x1, y1] = grid.extent;
    head.insert(
        "frame".into(),
        obj([
            ("axes", J::s("x east, y north, z up")),
            ("bearing", J::s("degrees clockwise from north")),
            (
                "extent",
                J::Arr(vec![J::mm(x0), J::mm(y0), J::mm(x1), J::mm(y1)]),
            ),
            ("posts", J::Arr(vec![J::count(nx), J::count(ny)])),
            ("resolution", J::mm(grid.resolution)),
            ("unit", J::s("m")),
        ]),
    );

    // Fields (6).
    let mut fields = J::obj();
    let mut chunk_weights: Vec<BTreeMap<String, J>> = vec![J::obj(); layout.chunks.len()];
    let mut used_materials: BTreeSet<String> = BTreeSet::new();
    for (name, f) in &b.fields {
        let mut e = J::obj();
        e.insert("role".into(), J::opt(f.role.as_deref()));
        e.insert("unit".into(), J::opt(f.unit.as_deref()));
        e.insert("taxon".into(), J::Null);
        e.insert("tags".into(), J::strs(f.tags.iter().map(String::as_str)));
        if let Some(values) = &f.values {
            if values.len() != nx * ny {
                return Err(internal(format!(
                    "field `{name}` holds {} samples for {} posts",
                    values.len(),
                    nx * ny
                )));
            }
            let mut clean = Vec::with_capacity(values.len());
            for (i, v) in values.iter().enumerate() {
                clean.push(finite32(*v, &format!("fields/{name}"), i)?);
            }
            let (lo, hi) = min_max(&clean);
            let path = format!("fields/{}.npy", segment(name));
            files.insert(path.clone(), npy::f4(&[ny, nx], &clean));
            e.insert("type".into(), J::s("scalar"));
            e.insert("file".into(), J::s(path));
            e.insert("min".into(), J::dbl(f64::from(lo)).unwrap_or(J::Null));
            e.insert("max".into(), J::dbl(f64::from(hi)).unwrap_or(J::Null));
        } else {
            e.insert("type".into(), J::s("weights"));
            let mats: Vec<(&String, &Vec<f32>)> = f.weights.iter().collect();
            // Bytes per post, before planes are chosen (6.2).
            let mut post_b: Vec<Vec<u8>> = vec![Vec::new(); nx * ny];
            for (p, slot) in post_b.iter_mut().enumerate() {
                let mut w = Vec::with_capacity(mats.len());
                for (m, vals) in &mats {
                    let v = f64::from(*vals.get(p).unwrap_or(&0.0));
                    if !v.is_finite() || v < 0.0 {
                        return Err(internal(format!(
                            "weight of `{m}` in field `{name}` at post {p} is below 0 or not finite"
                        )));
                    }
                    w.push(v);
                }
                *slot = post_bytes(&w);
            }
            for (ci, c) in layout.chunks.iter().enumerate() {
                let mut planes: Vec<usize> = Vec::new();
                for (m, _) in mats.iter().enumerate() {
                    let any = (c.j0..=c.j1).any(|j| {
                        (c.i0..=c.i1)
                            .any(|i| post_b[usize::try_from(j * grid.nx + i).unwrap_or(0)][m] > 0)
                    });
                    if any {
                        planes.push(m);
                    }
                }
                let entry = if planes.is_empty() {
                    obj([("file", J::Null), ("materials", J::Arr(Vec::new()))])
                } else {
                    let (rows, cols) = (
                        usize::try_from(c.rows()).unwrap_or(0),
                        usize::try_from(c.columns()).unwrap_or(0),
                    );
                    let mut data = Vec::with_capacity(planes.len() * rows * cols);
                    for m in &planes {
                        for j in c.j0..=c.j1 {
                            for i in c.i0..=c.i1 {
                                data.push(
                                    post_b[usize::try_from(j * grid.nx + i).unwrap_or(0)][*m],
                                );
                            }
                        }
                    }
                    let path = format!("weights/{}/{}_{}.npy", segment(name), c.cx, c.cy);
                    files.insert(path.clone(), npy::u1(&[planes.len(), rows, cols], &data));
                    for m in &planes {
                        used_materials.insert(mats[*m].0.clone());
                    }
                    obj([
                        ("file", J::s(path)),
                        (
                            "materials",
                            J::strs(planes.iter().map(|m| mats[*m].0.as_str())),
                        ),
                    ])
                };
                chunk_weights[ci].insert(name.clone(), entry);
            }
            for (m, _) in &mats {
                used_materials.insert((*m).clone());
            }
        }
        fields.insert(name.clone(), J::Obj(e));
    }
    head.insert("fields".into(), J::Obj(fields));

    // Materials and assets (7).
    let mut materials = J::obj();
    for m in &used_materials {
        let colour = input
            .pack
            .and_then(|p| p.materials.get(m))
            .map_or(J::Null, |mat| {
                J::Arr(
                    mat.colour
                        .iter()
                        .map(|c| J::dbl(*c).unwrap_or(J::Null))
                        .collect(),
                )
            });
        materials.insert(
            m.clone(),
            obj([("colour", colour), ("params", J::Obj(J::obj()))]),
        );
    }
    head.insert("materials".into(), J::Obj(materials));
    let mut assets = J::obj();
    let mut triangles: BTreeMap<String, u64> = BTreeMap::new();
    for i in b.instances.values() {
        if let Some(a) = input.pack.and_then(|p| p.assets.get(&i.asset)) {
            triangles.insert(a.id.clone(), a.triangles);
            let stand_ins = input.pack.is_some_and(|p| p.stand_ins);
            assets.insert(a.id.clone(), asset_json(a, stand_ins));
        }
    }
    head.insert("assets".into(), J::Obj(assets));

    // Instances (8): sorted by chunk, asset, name.
    let mut rows: Vec<(usize, &String, &String)> = b
        .instances
        .iter()
        .map(|(n, i)| (layout.chunk_of(i.origin.0, i.origin.1), &i.asset, n))
        .collect();
    rows.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.as_bytes().cmp(b.1.as_bytes()))
            .then(a.2.as_bytes().cmp(b.2.as_bytes()))
    });
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut zs = Vec::new();
    let mut faces = Vec::new();
    let mut scales = Vec::new();
    let mut tints = Vec::new();
    let mut names = String::new();
    let mut per_chunk: Vec<BTreeMap<String, (usize, usize)>> =
        vec![BTreeMap::new(); layout.chunks.len()];
    let mut chunk_tri: Vec<u64> = vec![0; layout.chunks.len()];
    for (row, (ci, asset, name)) in rows.iter().enumerate() {
        let i = &b.instances[*name];
        let to32 = |v: i64, what: &str| {
            i32::try_from(v).map_err(|_| {
                internal(format!(
                    "instance `{name}`: {what} {v} does not fit the column"
                ))
            })
        };
        xs.push(to32(i.origin.0, "x")?);
        ys.push(to32(i.origin.1, "y")?);
        faces.push(to32(i.facing.rem_euclid(360_000), "face")?);
        zs.push(finite32(i.z as f32, "instances/z", row)?);
        scales.push(finite32(i.scale as f32, "instances/scale", row)?);
        tints.push(finite32(i.tint as f32, "instances/tint", row)?);
        names.push_str(name);
        names.push('\n');
        let e = per_chunk[*ci].entry((*asset).clone()).or_insert((row, 0));
        e.1 += 1;
        chunk_tri[*ci] += triangles.get(*asset).copied().unwrap_or(0);
    }
    let n = rows.len();
    files.insert("instances/x.npy".into(), npy::i4(&xs));
    files.insert("instances/y.npy".into(), npy::i4(&ys));
    files.insert("instances/z.npy".into(), npy::f4(&[n], &zs));
    files.insert("instances/face.npy".into(), npy::i4(&faces));
    files.insert("instances/scale.npy".into(), npy::f4(&[n], &scales));
    files.insert("instances/tint.npy".into(), npy::f4(&[n], &tints));
    files.insert("instances/names.txt".into(), names.into_bytes());
    let mut inst = J::obj();
    inst.insert("count".into(), J::count(n));
    for c in ["face", "scale", "tint", "x", "y", "z"] {
        inst.insert(c.into(), J::s(format!("instances/{c}.npy")));
    }
    inst.insert("names".into(), J::s("instances/names.txt"));
    head.insert("instances".into(), J::Obj(inst));

    // Chunks (5).
    let mut chunks = Vec::new();
    for (ci, c) in layout.chunks.iter().enumerate() {
        let mut im = J::obj();
        let mut count = 0usize;
        for (a, (first, k)) in &per_chunk[ci] {
            im.insert(a.clone(), J::Arr(vec![J::count(*first), J::count(*k)]));
            count += k;
        }
        chunks.push(obj([
            (
                "at",
                J::Arr(vec![J::Num(c.cx.to_string()), J::Num(c.cy.to_string())]),
            ),
            ("count", J::count(count)),
            ("instances", J::Obj(im)),
            (
                "posts",
                J::Arr(
                    [c.i0, c.j0, c.i1, c.j1]
                        .iter()
                        .map(|v| J::Num(v.to_string()))
                        .collect(),
                ),
            ),
            ("triangles", J::Num(chunk_tri[ci].to_string())),
            ("weights", J::Obj(chunk_weights[ci].clone())),
        ]));
    }
    head.insert("chunks".into(), J::Arr(chunks));

    // Features, footprints, outputs (9).
    let surface = Surface::new(b, grid);
    let feats: Vec<J> = b
        .features
        .iter()
        .map(|(n, f)| feature_json(n, f, input.world.home("feature", n), &surface))
        .collect();
    let foots: Vec<J> = b
        .footprints
        .iter()
        .map(|(inst, f)| {
            obj([
                ("attrs", J::Obj(J::obj())),
                ("authority", kind_authority(input.world, &f.kind)),
                ("instance", J::s(inst.clone())),
                ("kind", J::s(f.kind.clone())),
                (
                    "layer",
                    J::opt(
                        input
                            .world
                            .home("instance", inst)
                            .or_else(|| input.world.home("pin", inst)),
                    ),
                ),
                ("pieces", pieces(&f.ring)),
            ])
        })
        .collect();
    files.insert(
        "features.json".into(),
        obj([("features", J::Arr(feats)), ("footprints", J::Arr(foots))]).file(),
    );
    let mut outputs: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (n, f) in &b.features {
        if let Some(o) = &f.output {
            outputs.entry(o.clone()).or_default().push(n);
        }
    }
    head.insert(
        "outputs".into(),
        J::Obj(outputs.into_iter().map(|(k, v)| (k, J::strs(v))).collect()),
    );
    head.insert("features".into(), J::s("features.json"));

    // Meetings (9.3), the columns of `surfaces.md` 2.4.
    let meets: Vec<J> = b
        .meetings
        .iter()
        .map(|m| {
            obj([
                ("a", J::s(m.a.clone())),
                ("b", J::s(m.b.clone())),
                ("connector_a", J::Null),
                ("connector_b", J::Null),
                ("face_a", J::Null),
                ("face_b", J::Null),
                ("gap", J::mm(m.gap)),
                ("turn", J::Null),
                ("x", J::mm(m.at.0)),
                ("y", J::mm(m.at.1)),
                ("z_a", J::dbl(m.z_a).unwrap_or(J::Null)),
                ("z_b", J::dbl(m.z_b).unwrap_or(J::Null)),
            ])
        })
        .collect();
    files.insert("meets.json".into(), obj([("meets", J::Arr(meets))]).file());
    head.insert("meets".into(), J::s("meets.json"));
    Ok(Written { files, head })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_remainder_rows() {
        assert_eq!(post_bytes(&[1.0, 1.0, 1.0]), vec![85, 85, 85]);
        assert_eq!(post_bytes(&[1.0, 1.0]), vec![128, 127]);
        assert_eq!(post_bytes(&[1e-4, 1.0]), vec![0, 255]);
        assert_eq!(post_bytes(&[0.0, 0.0]), vec![0, 0]);
        let twelve = post_bytes(&[1.0; 12]);
        assert_eq!(twelve.iter().map(|b| u32::from(*b)).sum::<u32>(), 255);
    }

    #[test]
    fn device_names_escape() {
        assert_eq!(segment("nul"), "nul-");
        assert_eq!(segment("com1.npy"), "com1-.npy");
        assert_eq!(segment("console"), "console");
        assert_eq!(segment("com0"), "com0");
    }

    #[test]
    fn rings_start_low_and_turn_left() {
        let r = canonical_ring(&[(1, 1), (0, 1), (0, 0), (1, 0)], true);
        assert_eq!(r, vec![(0, 0), (1, 0), (1, 1), (0, 1)]);
    }

    #[test]
    fn attribute_text_types() {
        assert_eq!(attr_json("1.7"), J::Num("1.7".into()));
        assert_eq!(attr_json("true"), J::Bool(true));
        assert_eq!(attr_json("north gate"), J::s("north gate"));
    }
}
