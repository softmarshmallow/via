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
pub const FORMAT: &str = "via-build-1";

/// The numerics epoch (`graph.md` 3.3): raised by any engine change that may change the bytes of
/// a value. Written in `manifest.json`'s `engine` and in every key. 2 from M3 slice 4: a smoothed
/// course is held to the extent (`derived.md` 3 step 6), which moves stored courses. 3 from K1 of
/// `docs/plan-kits.md` (R64, R65): a step's run no longer gives way to its own step's instances,
/// and a pin removes on its own level only, which changes held instances a store may hold. 4 from
/// K5 of `docs/plan-kits.md` (R69, R70): two instances one step returned that overlap are
/// `place.crowded` where they were `place.overlap`, two kind surfaces at one height are one level,
/// and only the lowest surface with `cut` is one level with the terrain, which changes what gives
/// way, what a pin removes and the diagnostics a store may hold.
pub const EPOCH: u64 = 4;

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
    /// The world's `[budget]` (5.1).
    pub budget: Option<&'a via_model::Budget>,
}

/// The files of the `id` by build path, and the manifest's world keys.
pub struct Written {
    pub files: BTreeMap<String, Vec<u8>>,
    pub head: BTreeMap<String, J>,
    /// The budget's diagnostics (5.1): world diagnostics found while writing.
    pub budget: Vec<Diag>,
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

/// The surface field's samples, for the height beneath a feature (9.1), and the kind surfaces a
/// point feature or a named place stands on (`surfaces.md` 4.1, 3.3).
pub struct Surface {
    grid: Grid,
    raster: Option<Raster<f32>>,
    /// Feature id or place name to the kind surface it stands on, and whether it is a point
    /// feature placed on it (taken on it within the bound's grace, 4.1), not a named owner.
    stands: BTreeMap<String, (via_model::surfaces::SurfaceOut, bool)>,
}

impl Surface {
    pub fn new(build: &Build, grid: Grid) -> Surface {
        let raster = build
            .fields
            .values()
            .find(|f| f.role.as_deref() == Some("surface"))
            .and_then(|f| f.values.clone())
            .and_then(|v| Raster::from_values(Window::whole(&grid), v));
        let stands = build
            .place_surface
            .iter()
            .filter_map(|(n, s)| {
                // An instance or a path's end named as a target is a point too: within the
                // bound's grace it takes the surface's `base` (4.1).
                let point = match build.features.get(n) {
                    Some(f) => f.shape == "point",
                    None => !build.surfaces.contains_key(n),
                };
                Some((n.clone(), (build.surfaces.get(s)?.clone(), point)))
            })
            .collect();
        Surface {
            grid,
            raster,
            stands,
        }
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

    /// [`Surface::xyz`] for the point of `name`: on the kind surface it stands on, if any. A
    /// point feature the build stood on it within the bound's one-unit grace (`surfaces.md` 4.1)
    /// takes the surface's `base` there, as `meets.json` does; a named owner outside its bound,
    /// the terrain.
    pub fn xyz_of(&self, name: &str, p: (i64, i64)) -> J {
        let on = self.stands.get(name).and_then(|(s, point)| match s.z(p) {
            Ok(Some(z)) => Some(z),
            Ok(None) if *point => s.base.eval(&[p]).ok().and_then(|v| v.first().copied()),
            _ => None,
        });
        match on {
            Some(z) => J::Arr(vec![J::mm(p.0), J::mm(p.1), J::dbl(z).unwrap_or(J::Null)]),
            None => self.xyz(p),
        }
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
                m.insert("at".into(), surface.xyz_of(name, *p));
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
            obj([
                ("at", surface.xyz_of(place, *p)),
                ("name", J::s(place.clone())),
            ]),
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
    // The quarter turn of `front` and `up` times the model's `scale` (`assets.md` 2.4; whole numbers
    // where it is 1, so a manifest is as before), then minus the pivot in exact metres.
    let s = a.model_scale();
    let mut to_asset = Vec::new();
    for row in a.to_asset() {
        for (k, v) in row.iter().enumerate() {
            to_asset.push(if k == 3 {
                J::mm(*v)
            } else if s == 1.0 || *v == 0 {
                J::int(*v)
            } else {
                J::dbl(*v as f64 * s).unwrap_or(J::int(*v))
            });
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
        let mut o = obj([("at", xy(*at)), ("face", J::int(*face))]);
        if let (Some(c), J::Obj(m)) = (a.connectors.get(n), &mut o) {
            m.insert("connector".into(), J::s(c.clone()));
        }
        // `rise` only where it is not 0 (`assets.md` 2.7, R75).
        if let (Some(r), J::Obj(m)) = (a.rises.get(n), &mut o) {
            m.insert("rise".into(), J::mm(*r));
        }
        mounts.insert(n.clone(), o);
    }
    m.insert("mounts".into(), J::Obj(mounts));
    m.insert("solidity".into(), J::s(a.solidity()));
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
    let layout = Layout::with_side(&grid, b.chunk);
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
            ("chunk", J::mm(layout.side_mm)),
            (
                "chunks",
                J::Arr(vec![
                    J::Num(layout.ncx.to_string()),
                    J::Num(layout.ncy.to_string()),
                ]),
            ),
            ("posts", J::Arr(vec![J::count(nx), J::count(ny)])),
            ("resolution", J::mm(grid.resolution)),
            ("unit", J::s("m")),
        ]),
    );
    // What each chunk's digest reads of the scalar fields (5.2): the samples as written.
    let mut scalars: Vec<(String, Vec<f32>)> = Vec::new();

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
            scalars.push((name.clone(), clean.clone()));
            let path = format!("fields/{}.npy", segment(name));
            files.insert(path.clone(), npy::f4(&[ny, nx], &clean));
            e.insert("type".into(), J::s("scalar"));
            e.insert("file".into(), J::s(path));
            e.insert("min".into(), J::dbl(f64::from(lo)).unwrap_or(J::Null));
            e.insert("max".into(), J::dbl(f64::from(hi)).unwrap_or(J::Null));
            // The terrain's cut (6.5): one byte per quad, all 0 when nothing cuts.
            if f.role.as_deref() == Some("surface") {
                if let Some(cut) = &b.cut {
                    let path = cut_path(name);
                    let shape = [ny.saturating_sub(1), nx.saturating_sub(1)];
                    files.insert(path.clone(), npy::u1(&shape, cut));
                    e.insert("cut".into(), J::s(path));
                }
            }
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

    // Materials and assets (7): every pack material a weights field or a generated mesh names
    // (a loader reads a mesh's colour from `materials[material]`, 15.1).
    for m in &b.meshes {
        if let Some(n) = &m.material {
            used_materials.insert(n.clone());
        }
    }
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
    // Each material's and asset's entry digest, for the chunk digest (5.2): a pack revision that
    // changes what an instance row or a weights layer points to changes the chunks that hold it.
    let entry_digest = |m: &BTreeMap<String, J>| -> BTreeMap<String, String> {
        m.iter()
            .map(|(k, v)| (k.clone(), digest_hex(v.canonical().as_bytes())))
            .collect()
    };
    let material_digest = entry_digest(&materials);
    head.insert("materials".into(), J::Obj(materials));
    let mut assets = J::obj();
    let mut triangles: BTreeMap<String, u64> = BTreeMap::new();
    for i in b.instances.values() {
        if let Some(a) = input.pack.and_then(|p| p.asset(&i.asset)) {
            triangles.insert(a.id.clone(), a.triangles);
            let stand_ins = input.pack.is_some_and(|p| p.stand_ins);
            let mut j = asset_json(a, stand_ins);
            // An asset an import's pack binds names that import (`build.md` 7): its `files`
            // lie under that pack's root, `locations.pack_roots`.
            if let (Some(b), J::Obj(m)) = (input.pack.and_then(|p| p.brought.get(&a.id)), &mut j) {
                m.insert("pack".into(), J::s(b.from.clone()));
            }
            assets.insert(a.id.clone(), j);
        }
    }
    let asset_digest = entry_digest(&assets);
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
    let mut row_json: Vec<J> = Vec::with_capacity(rows.len());
    let mut per_chunk: Vec<BTreeMap<String, (usize, usize)>> =
        vec![BTreeMap::new(); layout.chunks.len()];
    let mut chunk_tri: Vec<u64> = vec![0; layout.chunks.len()];
    // Per chunk and asset, its instances per import they come from ("" for this world's own).
    let mut chunk_from: Vec<BTreeMap<(String, String), u64>> =
        vec![BTreeMap::new(); layout.chunks.len()];
    let import_ids: BTreeSet<&str> = input.world.ids("import").into_iter().collect();
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
        row_json.push(J::Arr(vec![
            J::s((*name).clone()),
            J::s((*asset).clone()),
            J::int(i.origin.0),
            J::int(i.origin.1),
            J::int(i.facing.rem_euclid(360_000)),
            J::int(i64::from(zs[row].to_bits())),
            J::int(i64::from(scales[row].to_bits())),
            J::int(i64::from(tints[row].to_bits())),
        ]));
        let e = per_chunk[*ci].entry((*asset).clone()).or_insert((row, 0));
        e.1 += 1;
        *chunk_from[*ci]
            .entry(((*asset).clone(), import_of(name, &import_ids).to_string()))
            .or_insert(0) += 1;
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

    // Features, footprints, outputs (9).
    let surface = Surface::new(b, grid);
    let feats: Vec<J> = b
        .features
        .iter()
        .map(|(n, f)| feature_json(n, f, input.world.home("feature", n), &surface))
        .collect();
    // Each feature's and footprint's box and digest, for the chunks that list it (5, 5.2).
    let mut feat_box: Vec<(String, [i64; 4], String)> = Vec::new();
    for ((n, f), j) in b.features.iter().zip(&feats) {
        if let Some(bx) = box_of(f.all_points()) {
            feat_box.push((n.clone(), bx, digest_hex(j.canonical().as_bytes())));
        }
    }
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
    let mut foot_box: Vec<(String, [i64; 4], String)> = Vec::new();
    for ((inst, f), j) in b.footprints.iter().zip(&foots) {
        if let Some(bx) = box_of(f.ring.iter().copied()) {
            foot_box.push((inst.clone(), bx, digest_hex(j.canonical().as_bytes())));
        }
    }
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
                ("connector_a", J::opt(m.connector_a.as_deref())),
                ("connector_b", J::opt(m.connector_b.as_deref())),
                ("face_a", m.face_a.map_or(J::Null, J::int)),
                ("face_b", m.face_b.map_or(J::Null, J::int)),
                ("gap", J::mm(m.gap)),
                ("turn", m.turn.map_or(J::Null, J::int)),
                ("x", J::mm(m.at.0)),
                ("y", J::mm(m.at.1)),
                ("z_a", J::dbl(m.z_a).unwrap_or(J::Null)),
                ("z_b", J::dbl(m.z_b).unwrap_or(J::Null)),
            ])
        })
        .collect();
    files.insert("meets.json".into(), obj([("meets", J::Arr(meets))]).file());
    head.insert("meets".into(), J::s("meets.json"));

    // Generated meshes (10): one per kind surface.
    let mut meshes = Vec::new();
    let mut mesh_at: Vec<(String, Option<[i64; 4]>, String)> = Vec::new();
    for m in &b.meshes {
        let path = mesh_path(&m.place);
        let mat = m.material.as_deref().and_then(|n| {
            let colour = input.pack?.materials.get(n)?.colour;
            Some(crate::glb::Material { name: n, colour })
        });
        let bytes = crate::glb::write(m, mat.as_ref());
        // Listed by its box, as features are (5): a large triangle can span a chunk that holds
        // none of its vertices.
        let bx = box_of(m.vertices.iter().map(|v| (v[0], v[1])));
        mesh_at.push((m.place.clone(), bx, digest_hex(&bytes)));
        files.insert(path.clone(), bytes);
        meshes.push(obj([
            ("collides", J::Bool(true)),
            ("feature", J::s(m.owner.clone())),
            ("file", J::s(path)),
            ("material", J::opt(m.material.as_deref())),
            ("name", J::s(m.place.clone())),
            ("surface", J::s(m.surface.clone())),
            ("triangles", J::count(m.triangles.len())),
        ]));
    }
    head.insert("meshes".into(), J::Arr(meshes));

    // Chunks (5, 5.2), and the budget (5.1).
    let [ex0, ey0, _, _] = grid.extent;
    let r = grid.resolution;
    let at_of =
        |c: &crate::layout::Chunk| J::Arr(vec![J::Num(c.cx.to_string()), J::Num(c.cy.to_string())]);
    let mut chunks = Vec::new();
    let mut loads: Vec<Load> = Vec::new();
    for (ci, c) in layout.chunks.iter().enumerate() {
        let bounds = chunk_bounds(c, ex0, ey0, r);
        let meets = |bx: &[i64; 4]| {
            bx[0] <= bounds[2] && bounds[0] <= bx[2] && bx[1] <= bounds[3] && bounds[1] <= bx[3]
        };
        let mut im = J::obj();
        let mut count = 0usize;
        let mut rows_here = Vec::new();
        for (a, (first, k)) in &per_chunk[ci] {
            im.insert(a.clone(), J::Arr(vec![J::count(*first), J::count(*k)]));
            count += k;
            rows_here.extend(row_json[*first..*first + *k].iter().cloned());
        }
        let mut neighbours = Vec::new();
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (nx_, ny_) = (c.cx as i64 + dx, c.cy as i64 + dy);
                if (dx, dy) != (0, 0)
                    && (0..layout.ncx as i64).contains(&nx_)
                    && (0..layout.ncy as i64).contains(&ny_)
                {
                    neighbours.push(J::Arr(vec![J::int(nx_), J::int(ny_)]));
                }
            }
        }
        let mesh_names: BTreeMap<String, J> = mesh_at
            .iter()
            .filter(|(_, bx, _)| bx.as_ref().is_some_and(meets))
            .map(|(n, _, d)| (n.clone(), J::s(d.clone())))
            .collect();
        let feat_names: BTreeMap<String, J> = feat_box
            .iter()
            .filter(|(_, bx, _)| meets(bx))
            .map(|(n, _, d)| (n.clone(), J::s(d.clone())))
            .collect();
        let foot_names: BTreeMap<String, J> = foot_box
            .iter()
            .filter(|(_, bx, _)| meets(bx))
            .map(|(n, _, d)| (n.clone(), J::s(d.clone())))
            .collect();
        // The digest (5.2).
        let mut fd = J::obj();
        for (name, vals) in &scalars {
            let mut bytes = Vec::new();
            for j in c.j0..=c.j1 {
                for i in c.i0..=c.i1 {
                    let v = vals[usize::try_from(j * grid.nx + i).unwrap_or(0)];
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            fd.insert(name.clone(), J::s(digest_hex(&bytes)));
            if b.fields.get(name).and_then(|f| f.role.as_deref()) == Some("surface") {
                if let Some(cut) = &b.cut {
                    let mut q = Vec::new();
                    for j in c.j0..c.j1 {
                        for i in c.i0..c.i1 {
                            let at = usize::try_from(j * (grid.nx - 1) + i).unwrap_or(0);
                            q.push(cut.get(at).copied().unwrap_or(0));
                        }
                    }
                    fd.insert(format!("{name}.cut"), J::s(digest_hex(&q)));
                }
            }
        }
        let mut wd = J::obj();
        for (field, e) in &chunk_weights[ci] {
            let d = e
                .get("file")
                .and_then(J::as_str)
                .and_then(|p| files.get(p))
                .map_or(J::Null, |bytes| J::s(digest_hex(bytes)));
            wd.insert(field.clone(), d);
        }
        // The pack entries this chunk's rows and weights layers point to (5.2).
        let mut ad = J::obj();
        for a in per_chunk[ci].keys() {
            if let Some(d) = asset_digest.get(a) {
                ad.insert(a.clone(), J::s(d.clone()));
            }
        }
        let mut md = J::obj();
        for e in chunk_weights[ci].values() {
            if let Some(J::Arr(ms)) = e.get("materials") {
                for m in ms.iter().filter_map(J::as_str) {
                    if let Some(d) = material_digest.get(m) {
                        md.insert(m.to_string(), J::s(d.clone()));
                    }
                }
            }
        }
        let digest = digest_hex(
            obj([
                ("assets", J::Obj(ad)),
                ("features", J::Obj(feat_names.clone())),
                ("fields", J::Obj(fd)),
                ("footprints", J::Obj(foot_names.clone())),
                (
                    "instances",
                    J::s(digest_hex(J::Arr(rows_here).canonical().as_bytes())),
                ),
                ("materials", J::Obj(md)),
                ("meshes", J::Obj(mesh_names.clone())),
                ("weights", J::Obj(wd)),
            ])
            .canonical()
            .as_bytes(),
        );
        let groups = per_chunk[ci].len();
        chunks.push(obj([
            ("at", at_of(c)),
            ("bounds", J::Arr(bounds.iter().map(|v| J::mm(*v)).collect())),
            ("count", J::count(count)),
            ("digest", J::s(digest)),
            ("features", J::strs(feat_names.keys().map(String::as_str))),
            ("footprints", J::strs(foot_names.keys().map(String::as_str))),
            ("groups", J::count(groups)),
            ("instances", J::Obj(im)),
            ("meshes", J::strs(mesh_names.keys().map(String::as_str))),
            ("neighbours", J::Arr(neighbours)),
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
        // Each asset's share of each measure, for the record that names the dominant one.
        let shares: Vec<(String, u64, u64)> = per_chunk[ci]
            .iter()
            .map(|(a, (_, k))| {
                let k = *k as u64;
                (a.clone(), k, k * triangles.get(a).copied().unwrap_or(0))
            })
            .collect();
        loads.push(Load {
            at: (c.cx, c.cy),
            bounds,
            triangles: chunk_tri[ci],
            instances: count as u64,
            groups: groups as u64,
            shares,
            from: std::mem::take(&mut chunk_from[ci]),
        });
    }
    head.insert("chunks".into(), J::Arr(chunks));
    let budget = input
        .budget
        .map(|bg| budget_diags(bg, &loads))
        .unwrap_or_default();
    Ok(Written {
        files,
        head,
        budget,
    })
}

/// What a chunk holds, as the budget counts it (5.1).
struct Load {
    at: (u64, u64),
    bounds: [i64; 4],
    triangles: u64,
    instances: u64,
    groups: u64,
    /// Per asset: its instances and its triangles in the chunk.
    shares: Vec<(String, u64, u64)>,
    /// Per asset and the import its instances come from ("" for the world's own): how many.
    from: BTreeMap<(String, String), u64>,
}

/// The import an instance comes from, by its scoped name (`composition.md` 4: `a.lamp`,
/// `a.scatter#k`), or "" for the world's own: the part before the first `.`, when the name has
/// one before any `#` and that part is one of the world's imports. A world's own accessory
/// (`table.candle`) is dotted too, and is the world's own (`build.md` 5.1).
fn import_of<'n>(name: &'n str, imports: &BTreeSet<&str>) -> &'n str {
    let head = name.split('#').next().unwrap_or(name);
    match head.split_once('.') {
        Some((a, _)) if imports.contains(a) => a,
        _ => "",
    }
}

impl Load {
    fn value(&self, measure: &str) -> u64 {
        match measure {
            "triangles" => self.triangles,
            "instances" => self.instances,
            _ => self.groups,
        }
    }
}

/// A chunk's first and last posts in millimetres (5).
fn chunk_bounds(c: &crate::layout::Chunk, x0: i64, y0: i64, r: i64) -> [i64; 4] {
    let p = |i: u64| i64::try_from(i).unwrap_or(0) * r;
    [x0 + p(c.i0), y0 + p(c.j0), x0 + p(c.i1), y0 + p(c.j1)]
}

/// The least box holding the points, in millimetres.
fn box_of(points: impl IntoIterator<Item = (i64, i64)>) -> Option<[i64; 4]> {
    let mut b: Option<[i64; 4]> = None;
    for (x, y) in points {
        b = Some(match b {
            None => [x, y, x, y],
            Some(o) => [o[0].min(x), o[1].min(y), o[2].max(x), o[3].max(y)],
        });
    }
    b
}

fn digest_hex(bytes: &[u8]) -> String {
    via_graph::Digest::of(bytes).hex()
}

/// The budget's records (5.1): `build.over_budget` per chunk and measure over its limit, the
/// dominant asset named; `build.waiver_unused` per waiver not needed.
fn budget_diags(bg: &via_model::Budget, loads: &[Load]) -> Vec<Diag> {
    use via_diag::{KeyPart, KeyPath, Value};
    let key = |parts: &[&str]| {
        KeyPath::from_parts(
            parts
                .iter()
                .map(|p| KeyPart::Key((*p).to_string()))
                .collect(),
        )
        .unwrap_or_else(|| KeyPath::key("budget"))
    };
    let m = via_num::write_length;
    let mut out = Vec::new();
    for l in loads {
        for (measure, world_limit) in bg.limits.each() {
            // A waiver raises the world's limit and never adds one (5.1).
            if world_limit.is_none() {
                continue;
            }
            // The first waiver of this chunk, by id, that writes this measure.
            let waiver = bg.waivers.iter().find(|w| {
                w.at == Some(l.at)
                    && w.limits
                        .each()
                        .iter()
                        .any(|(k, v)| *k == measure && v.is_some())
            });
            let limit = waiver
                .and_then(|w| {
                    w.limits
                        .each()
                        .iter()
                        .find(|(k, _)| *k == measure)
                        .and_then(|(_, v)| *v)
                })
                .or(world_limit);
            let Some(limit) = limit else { continue };
            let value = l.value(measure);
            if value <= limit {
                continue;
            }
            // The dominant asset: the largest share of the measure, ties to the lower id.
            let part = |s: &(String, u64, u64)| if measure == "triangles" { s.2 } else { s.1 };
            let top = l.shares.iter().max_by(|a, b| {
                part(a)
                    .cmp(&part(b))
                    .then(b.0.as_bytes().cmp(a.0.as_bytes()))
            });
            let (asset, inst, share) = match top {
                Some(t) => {
                    let whole = if measure == "triangles" {
                        l.triangles
                    } else {
                        l.instances
                    };
                    (t.0.clone(), t.1, part(t) * 100 / whole.max(1))
                }
                None => (String::new(), 0, 0),
            };
            let (cx, cy) = l.at;
            let site = match waiver {
                Some(w) => key(&["budget", "waive", &w.id, measure]),
                None => key(&["budget", measure]),
            };
            let what = match measure {
                "triangles" => "triangles",
                "instances" => "instances",
                _ => "instance groups",
            };
            // A group is one asset, so its share is of the chunk's instances (5.1).
            let of = if measure == "groups" {
                "of its instances"
            } else {
                "of them"
            };
            let msg = format!(
                "chunk [{cx}, {cy}] ({}..{} m east, {}..{} m north) holds {value} {what}, over the {} of {limit}; `{asset}` gives {share}% {of} ({inst} instances)",
                m(l.bounds[0]),
                m(l.bounds[2]),
                m(l.bounds[1]),
                m(l.bounds[3]),
                if waiver.is_some() { "waived limit" } else { "budget" },
            );
            // Where the asset's instances in the chunk come from: an import's are thinned by a
            // change of that import, which this world asks for (`composition.md` 11).
            let from = l
                .from
                .iter()
                .filter(|((a, _), _)| *a == asset)
                .max_by(|(a, x), (b, y)| x.cmp(y).then(b.1.cmp(&a.1)))
                .map(|((_, f), _)| f.clone())
                .unwrap_or_default();
            let thin = if from.is_empty() {
                format!("thin `{asset}` there")
            } else {
                format!("`{asset}` there comes from the import `{from}`: thin it by a change of that import (`changes = [\"<change>\"]` on `[import.{from}]`)")
            };
            let hint = format!(
                "{thin}, or waive the chunk: [budget.waive.c{cx}_{cy}] at = [{cx}, {cy}], {measure} = {value}, reason = \"<why the target can draw it>\""
            );
            let as_i = |v: u64| Value::from(i64::try_from(v).unwrap_or(i64::MAX));
            out.push(
                Diag::new(
                    codes::build::OVER_BUDGET,
                    Subject::World,
                    Site::Key(site),
                    txt(msg),
                    txt(hint),
                )
                .with("chunk", Value::List(vec![as_i(cx), as_i(cy)]))
                .with("measure", measure)
                .with("value", as_i(value))
                .with("limit", as_i(limit))
                .with("asset", asset.as_str())
                .with("share", as_i(share))
                .with("instances", as_i(inst))
                .with("from", from.as_str()),
            );
        }
    }
    for w in &bg.waivers {
        let load = w.at.and_then(|at| loads.iter().find(|l| l.at == at));
        let needed = load.is_some_and(|l| {
            w.limits.each().iter().any(|(measure, v)| {
                v.is_some()
                    && bg
                        .limits
                        .each()
                        .iter()
                        .find(|(k, _)| k == measure)
                        .and_then(|(_, lim)| *lim)
                        .is_some_and(|lim| l.value(measure) > lim)
            })
        });
        if needed {
            continue;
        }
        let msg = match (w.at, load) {
            (Some((cx, cy)), None) => format!(
                "the waiver `{}` names chunk [{cx}, {cy}], which this build does not have",
                w.id
            ),
            (None, _) => format!("the waiver `{}` names no chunk", w.id),
            (Some((cx, cy)), Some(_)) => {
                // A measure the world does not limit cannot be waived (5.1).
                let unlimited: Vec<&str> = w
                    .limits
                    .each()
                    .iter()
                    .filter(|(k, v)| {
                        v.is_some()
                            && bg
                                .limits
                                .each()
                                .iter()
                                .any(|(m, lim)| m == k && lim.is_none())
                    })
                    .map(|(k, _)| *k)
                    .collect();
                if unlimited.is_empty() {
                    format!(
                        "the waiver `{}` is not needed: chunk [{cx}, {cy}] is within the budget for every measure it writes",
                        w.id
                    )
                } else {
                    format!(
                        "the waiver `{}` is not needed: chunk [{cx}, {cy}] is within the budget, and `[budget]` sets no limit for {} to raise",
                        w.id,
                        unlimited.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ")
                    )
                }
            }
        };
        out.push(
            Diag::new(
                codes::build::WAIVER_UNUSED,
                Subject::World,
                Site::Key(key(&["budget", "waive", &w.id])),
                txt(msg),
                via_diag::text!("delete the waiver"),
            )
            .with("waiver", w.id.as_str()),
        );
    }
    out
}

/// The cut file of the terrain `name` (`build.md` 6.5).
pub fn cut_path(name: &str) -> String {
    format!("fields/{}.cut.npy", segment(name))
}

/// The mesh file of the kind surface `place` (`build.md` 10).
pub fn mesh_path(place: &str) -> String {
    format!("meshes/{}.glb", segment(place))
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
