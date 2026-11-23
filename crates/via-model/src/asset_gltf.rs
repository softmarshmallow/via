//! Measuring a glTF 2.0 model (`assets.md` 3): its bounds in the asset frame's axes, its
//! triangles, whether it is textured, and the files it references.
//!
//! A reader of our own, kept to what 3.1 reads: the scene, the glTF node transforms, `POSITION`
//! accessors (sparse and `KHR_mesh_quantization` included), index counts and material textures.
//! Nothing else of the model is read. Every product and sum is written out in a fixed order, with
//! `+`, `-` and `*` only (`numbers.md` 8, rule 1), and every position is rounded to the lattice by
//! `via-num` (`numbers.md` 3.1), so a model measures to the same integers on every system.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Code};

use crate::asset_json::{self, Json};

/// What is wrong with a model: a code of `assets.md`, where in the glTF JSON (a JSON Pointer,
/// RFC 6901), a message and a hint.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub code: Code,
    pub pointer: Option<String>,
    pub message: String,
    pub hint: String,
}

fn bad(pointer: Option<&str>, message: impl Into<String>) -> Fault {
    Fault {
        code: codes::pack::BAD_MODEL,
        pointer: pointer.map(str::to_string),
        message: message.into(),
        hint: "re-export the model as glTF 2.0, or fix the pointed part".into(),
    }
}

/// A signed model axis: index 0, 1, 2 for x, y, z, and its sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis {
    pub index: usize,
    pub sign: i64,
}

impl Axis {
    /// One of the six words of `assets.md` 2.3.
    pub fn parse(w: &str) -> Option<Axis> {
        let sign = match w.as_bytes().first()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let index = match w.get(1..)? {
            "x" => 0,
            "y" => 1,
            "z" => 2,
            _ => return None,
        };
        Some(Axis { index, sign })
    }

    fn vector(self) -> [i64; 3] {
        let mut v = [0; 3];
        v[self.index] = self.sign;
        v
    }
}

/// The mapping from a model's axes to the asset frame (2.3): the asset's `y` is the model's
/// `front`, its `z` the model's `up`, its `x` is `y × z`. Always a rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axes {
    pub front: Axis,
    pub up: Axis,
}

impl Axes {
    /// `None` when either word is not an axis, or both name one axis (`pack.bad_axes`).
    pub fn parse(front: &str, up: &str) -> Option<Axes> {
        let (front, up) = (Axis::parse(front)?, Axis::parse(up)?);
        (front.index != up.index).then_some(Axes { front, up })
    }

    /// The three rows of the quarter turn taking a model point to the asset frame: entries -1, 0
    /// or 1 (`build.md` 7, `to_asset`).
    pub fn rows(&self) -> [[i64; 3]; 3] {
        let f = self.front.vector();
        let u = self.up.vector();
        let x = [
            f[1] * u[2] - f[2] * u[1],
            f[2] * u[0] - f[0] * u[2],
            f[0] * u[1] - f[1] * u[0],
        ];
        [x, f, u]
    }

    /// A model point in the asset frame's axes, exactly: each row has one entry ±1.
    pub fn map(&self, p: [f64; 3]) -> [f64; 3] {
        let rows = self.rows();
        let mut out = [0.0; 3];
        for (o, r) in out.iter_mut().zip(rows) {
            for (k, c) in r.iter().enumerate() {
                if *c == 1 {
                    *o = p[k];
                } else if *c == -1 {
                    *o = -p[k];
                }
            }
        }
        out
    }
}

/// A URI the model references outside itself: a buffer's or an image's, percent-decoded.
#[derive(Clone, Debug, PartialEq)]
pub struct Uri {
    /// Where in the glTF JSON it is written, as a JSON Pointer.
    pub pointer: String,
    /// The URI as written.
    pub written: String,
    /// Percent-decoded.
    pub decoded: String,
    /// The buffer it holds, `None` for an image.
    pub buffer: Option<usize>,
}

/// What 3.1 and 3.2 measure.
#[derive(Clone, Debug, PartialEq)]
pub struct Measure {
    /// `[x0, y0, z0, x1, y1, z1]` in the asset frame's axes before the pivot, millimetres.
    pub raw: [i64; 6],
    pub triangles: u64,
    /// No material has a base colour texture.
    pub untextured: bool,
    /// Every triangle in plan, in `raw`'s axes and millimetres: what [`plan_rings`] traces where
    /// an asset asks for its plan (3.4).
    pub plan_tris: Vec<[[i64; 2]; 3]>,
    /// Each `mount_<name>` empty (`assets.md` 2.7): its name, its origin in `raw`'s axes and
    /// millimetres, and the bearing of its +Z in plan, millidegrees clockwise from the front.
    pub mounts: Vec<(String, [i64; 3], i64)>,
}

/// A model read: its JSON, and the binary chunk of a `.glb`.
pub struct Model {
    json: Json,
    bin: Option<Vec<u8>>,
}

const GLB_MAGIC: u32 = 0x4654_6C67;
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The extensions 3.1 tolerates in `extensionsRequired`, besides every `KHR_materials_*` one.
const TOLERATED: [&str; 5] = [
    "KHR_mesh_quantization",
    "KHR_texture_transform",
    "KHR_texture_basisu",
    "EXT_texture_webp",
    "KHR_lights_punctual",
];

impl Model {
    /// Reads a `.glb` (by its magic) or `.gltf` JSON text, and checks the version and the
    /// extensions (3.1).
    pub fn open(bytes: &[u8]) -> Result<Model, Fault> {
        let (json_bytes, bin) = if u32_at(bytes, 0) == Some(GLB_MAGIC) {
            let version = u32_at(bytes, 4).unwrap_or(0);
            if version != 2 {
                return Err(bad(
                    None,
                    format!("a binary glTF of version {version}, not 2"),
                ));
            }
            let length = u32_at(bytes, 8).unwrap_or(0) as usize;
            if length > bytes.len() || length < 12 {
                return Err(bad(
                    None,
                    "the binary glTF's length does not match the file",
                ));
            }
            let mut at = 12;
            let mut json: Option<&[u8]> = None;
            let mut bin: Option<Vec<u8>> = None;
            while at + 8 <= length {
                let (Some(len), Some(kind)) = (u32_at(bytes, at), u32_at(bytes, at + 4)) else {
                    break;
                };
                let len = len as usize;
                let Some(data) = bytes.get(at + 8..at + 8 + len) else {
                    return Err(bad(None, "a chunk of the binary glTF runs past its end"));
                };
                match kind {
                    CHUNK_JSON if json.is_none() => json = Some(data),
                    CHUNK_BIN if bin.is_none() && json.is_some() => bin = Some(data.to_vec()),
                    _ => {}
                }
                at += 8 + len;
            }
            let Some(json) = json else {
                return Err(bad(None, "the binary glTF has no JSON chunk"));
            };
            (json, bin)
        } else {
            (bytes, None)
        };
        let json = asset_json::parse(json_bytes)
            .map_err(|e| bad(None, format!("the model is not glTF JSON: {e}")))?;
        if let Some(at) = json.non_finite("") {
            return Err(bad(
                Some(&at),
                "the model holds a number that is not finite",
            ));
        }
        let version = json
            .get("asset")
            .and_then(|a| a.get("version"))
            .and_then(Json::as_str);
        if !version.is_some_and(|v| v == "2" || v.starts_with("2.")) {
            return Err(bad(
                Some("/asset/version"),
                format!(
                    "the model is not glTF 2.0 (version {})",
                    version.unwrap_or("absent")
                ),
            ));
        }
        let names = |key: &str| -> Vec<(usize, String)> {
            json.get(key)
                .and_then(Json::as_arr)
                .map(|a| {
                    a.iter()
                        .enumerate()
                        .filter_map(|(i, v)| v.as_str().map(|s| (i, s.to_string())))
                        .collect()
                })
                .unwrap_or_default()
        };
        for (i, e) in names("extensionsRequired") {
            if !(TOLERATED.contains(&e.as_str()) || e.starts_with("KHR_materials_")) {
                return Err(Fault {
                    hint: format!("re-export without `{e}`"),
                    ..bad(
                        Some(&format!("/extensionsRequired/{i}")),
                        format!("the model requires `{e}`, which v0 does not read"),
                    )
                });
            }
        }
        for key in ["extensionsRequired", "extensionsUsed"] {
            for (i, e) in names(key) {
                if e == "EXT_mesh_gpu_instancing" {
                    return Err(Fault {
                        hint: format!("re-export without `{e}`"),
                        ..bad(
                            Some(&format!("/{key}/{i}")),
                            format!(
                                "the model uses `{e}`, which changes geometry and v0 does not read"
                            ),
                        )
                    });
                }
            }
        }
        Ok(Model { json, bin })
    }

    fn list(&self, key: &str) -> &[Json] {
        self.json.get(key).and_then(Json::as_arr).unwrap_or(&[])
    }

    /// Every URI of a buffer or an image that is not a `data:` URI, in the order buffers then
    /// images (3.1, "Bound files").
    pub fn uris(&self) -> Result<Vec<Uri>, Fault> {
        let mut out = Vec::new();
        for (key, is_buffer) in [("buffers", true), ("images", false)] {
            for (i, b) in self.list(key).iter().enumerate() {
                let Some(u) = b.get("uri").and_then(Json::as_str) else {
                    continue;
                };
                if u.starts_with("data:") {
                    continue;
                }
                let pointer = format!("/{key}/{i}/uri");
                let decoded = asset_json::percent_decode(u).ok_or_else(|| {
                    bad(
                        Some(&pointer),
                        format!("the URI `{u}` does not percent-decode"),
                    )
                })?;
                out.push(Uri {
                    pointer,
                    written: u.to_string(),
                    decoded,
                    buffer: is_buffer.then_some(i),
                });
            }
        }
        Ok(out)
    }

    /// The bytes of every buffer: the binary chunk, a `data:` URI, or the file the caller read
    /// for an external URI (`external`, by buffer index).
    fn buffers(&self, external: &BTreeMap<usize, Vec<u8>>) -> Result<Vec<Vec<u8>>, Fault> {
        let mut out = Vec::new();
        for (i, b) in self.list("buffers").iter().enumerate() {
            let here = format!("/buffers/{i}");
            let data = match b.get("uri").and_then(Json::as_str) {
                None if i == 0 => self
                    .bin
                    .clone()
                    .ok_or_else(|| bad(Some(&here), "the buffer has no URI and no binary chunk"))?,
                None => return Err(bad(Some(&here), "the buffer has no URI")),
                Some(u) if u.starts_with("data:") => asset_json::data_uri(u)
                    .ok_or_else(|| bad(Some(&here), "the buffer's data URI does not decode"))?,
                Some(_) => external.get(&i).cloned().unwrap_or_default(),
            };
            let len = b.get("byteLength").and_then(Json::as_index).unwrap_or(0);
            if data.len() < len {
                return Err(bad(
                    Some(&here),
                    format!(
                        "the buffer holds {} bytes, fewer than its byteLength {len}",
                        data.len()
                    ),
                ));
            }
            out.push(data);
        }
        Ok(out)
    }

    /// Measures the model (3.1, 3.2) with its axes mapped by `axes`.
    pub fn measure(
        &self,
        external: &BTreeMap<usize, Vec<u8>>,
        axes: Axes,
        want_plan: bool,
        scale: f64,
    ) -> Result<Measure, Fault> {
        let buffers = self.buffers(external)?;
        let scenes = self.list("scenes");
        let scene_index = match self.json.get("scene") {
            Some(s) => s
                .as_index()
                .ok_or_else(|| bad(Some("/scene"), "`scene` is not an index"))?,
            None => 0,
        };
        let empty = |message: &str| Fault {
            code: codes::asset::EMPTY,
            pointer: None,
            message: message.to_string(),
            hint: "export the mesh into the default scene".into(),
        };
        let Some(scene) = scenes.get(scene_index) else {
            return Err(empty(if scenes.is_empty() {
                "the model has no scene"
            } else {
                "the model's `scene` names no scene"
            }));
        };
        let mut walk = Walk {
            m: self,
            buffers: &buffers,
            axes,
            lo: [i64::MAX; 3],
            hi: [i64::MIN; 3],
            triangles: 0,
            want_plan,
            scale,
            mounts: Vec::new(),
            plan_tris: Vec::new(),
            on_path: BTreeSet::new(),
        };
        let roots = scene.get("nodes").and_then(Json::as_arr).unwrap_or(&[]);
        for (k, r) in roots.iter().enumerate() {
            let n = r.as_index().ok_or_else(|| {
                bad(
                    Some(&format!("/scenes/{scene_index}/nodes/{k}")),
                    "a scene's node is not an index",
                )
            })?;
            walk.node(n, &IDENTITY, 0)?;
        }
        if walk.triangles == 0 || walk.lo[0] > walk.hi[0] {
            return Err(empty("the model's scene reaches no triangles"));
        }
        let untextured = !self.list("materials").iter().any(|m| {
            m.get("pbrMetallicRoughness")
                .and_then(|p| p.get("baseColorTexture"))
                .is_some()
        });
        Ok(Measure {
            raw: [
                walk.lo[0], walk.lo[1], walk.lo[2], walk.hi[0], walk.hi[1], walk.hi[2],
            ],
            triangles: walk.triangles,
            untextured,
            plan_tris: walk.plan_tris,
            mounts: walk.mounts,
        })
    }
}

type M4 = [[f64; 4]; 4];

const IDENTITY: M4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// `a * b`, each entry summed left to right.
fn mul(a: &M4, b: &M4) -> M4 {
    let mut out = [[0.0; 4]; 4];
    for (r, row) in out.iter_mut().enumerate() {
        for (c, o) in row.iter_mut().enumerate() {
            *o = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
        }
    }
    out
}

fn numbers(v: Option<&Json>, n: usize) -> Option<Vec<f64>> {
    let a = v?.as_arr()?;
    if a.len() != n {
        return None;
    }
    a.iter().map(Json::as_f64).collect()
}

/// A glTF node's own matrix: `matrix` (column-major), or `T * R * S`.
fn local(node: &Json, here: &str) -> Result<M4, Fault> {
    if let Some(m) = node.get("matrix") {
        let m = numbers(Some(m), 16).ok_or_else(|| {
            bad(
                Some(&format!("{here}/matrix")),
                "`matrix` is not 16 numbers",
            )
        })?;
        let mut out = [[0.0; 4]; 4];
        for (c, col) in m.chunks(4).enumerate() {
            for (r, v) in col.iter().enumerate() {
                out[r][c] = *v;
            }
        }
        return Ok(out);
    }
    let get = |k: &str, n: usize, default: &[f64]| -> Result<Vec<f64>, Fault> {
        match node.get(k) {
            None => Ok(default.to_vec()),
            Some(v) => numbers(Some(v), n).ok_or_else(|| {
                bad(
                    Some(&format!("{here}/{k}")),
                    format!("`{k}` is not {n} numbers"),
                )
            }),
        }
    };
    let t = get("translation", 3, &[0.0, 0.0, 0.0])?;
    let q = get("rotation", 4, &[0.0, 0.0, 0.0, 1.0])?;
    let s = get("scale", 3, &[1.0, 1.0, 1.0])?;
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let mut out = IDENTITY;
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = r[i][j] * s[j];
        }
        out[i][3] = t[i];
    }
    Ok(out)
}

struct Walk<'a> {
    m: &'a Model,
    buffers: &'a [Vec<u8>],
    axes: Axes,
    lo: [i64; 3],
    hi: [i64; 3],
    triangles: u64,
    /// Each triangle in plan, millimetres in the asset frame's axes (3.4); kept only where the
    /// asset asks for its plan, so a model that does not reads no index and keeps no triangle.
    want_plan: bool,
    /// The model's units to metres (`assets.md` 2.4, `scale`).
    scale: f64,
    mounts: Vec<(String, [i64; 3], i64)>,
    plan_tris: Vec<[[i64; 2]; 3]>,
    on_path: BTreeSet<usize>,
}

/// How deep glTF nodes may nest before the model is refused.
const MAX_NODE_DEPTH: usize = 512;

impl Walk<'_> {
    fn node(&mut self, n: usize, parent: &M4, depth: usize) -> Result<(), Fault> {
        let here = format!("/nodes/{n}");
        let Some(node) = self.m.list("nodes").get(n) else {
            return Err(bad(Some(&here), "a glTF node index names no node"));
        };
        if depth > MAX_NODE_DEPTH || !self.on_path.insert(n) {
            return Err(bad(Some(&here), "the glTF nodes form a cycle"));
        }
        let world = mul(parent, &local(node, &here)?);
        // 2.7: an empty named `mount_<name>` is a mount, measured as a vertex is.
        if node.get("mesh").is_none() {
            if let Some(name) = node
                .get("name")
                .and_then(Json::as_str)
                .and_then(|n| n.strip_prefix("mount_"))
                .filter(|n| !n.is_empty())
            {
                let o = self.axes.map([world[0][3], world[1][3], world[2][3]]);
                let mut at = [0i64; 3];
                for (k, v) in o.iter().enumerate() {
                    at[k] = via_num::round_to_lattice(v * 1000.0 * self.scale).map_err(|_| {
                        bad(
                            Some(&here),
                            "a mount node lies outside the lattice, or is not finite",
                        )
                    })?;
                }
                // The node faces the way its own axis named by the pack's `front` points, so a node
                // at rest faces the asset's front whatever `up` is (F7's review: +Z lost every turn
                // of a Z-up model).
                let (fi, fs) = (self.axes.front.index, self.axes.front.sign as f64);
                let d = self
                    .axes
                    .map([world[0][fi] * fs, world[1][fi] * fs, world[2][fi] * fs]);
                let face = if d[0].abs() < 1e-9 && d[1].abs() < 1e-9 {
                    0
                } else {
                    let deg = via_num::atan2(d[0], d[1]) * (180.0 / via_num::PI);
                    via_num::normalise_facing(via_num::round_to_lattice(deg * 1000.0).unwrap_or(0))
                };
                self.mounts.push((name.to_string(), at, face));
            }
        }
        if let Some(mesh) = node.get("mesh") {
            let mi = mesh
                .as_index()
                .ok_or_else(|| bad(Some(&format!("{here}/mesh")), "`mesh` is not an index"))?;
            self.mesh(mi, &world)?;
        }
        if let Some(children) = node.get("children").and_then(Json::as_arr) {
            for (k, c) in children.iter().enumerate() {
                let ci = c.as_index().ok_or_else(|| {
                    bad(
                        Some(&format!("{here}/children/{k}")),
                        "a child is not an index",
                    )
                })?;
                self.node(ci, &world, depth + 1)?;
            }
        }
        self.on_path.remove(&n);
        Ok(())
    }

    fn mesh(&mut self, mi: usize, world: &M4) -> Result<(), Fault> {
        let here = format!("/meshes/{mi}");
        let Some(mesh) = self.m.list("meshes").get(mi) else {
            return Err(bad(Some(&here), "`mesh` names no mesh"));
        };
        let prims = mesh.get("primitives").and_then(Json::as_arr).unwrap_or(&[]);
        for (pi, p) in prims.iter().enumerate() {
            let at = format!("{here}/primitives/{pi}");
            let Some(pos) = p
                .get("attributes")
                .and_then(|a| a.get("POSITION"))
                .and_then(Json::as_index)
            else {
                continue;
            };
            let points = self.vec3(pos)?;
            let n = match p.get("indices") {
                Some(i) => {
                    let ii = i
                        .as_index()
                        .ok_or_else(|| bad(Some(&format!("{at}/indices")), "not an index"))?;
                    let acc =
                        self.m.list("accessors").get(ii).ok_or_else(|| {
                            bad(Some(&format!("{at}/indices")), "names no accessor")
                        })?;
                    acc.get("count").and_then(Json::as_index).unwrap_or(0)
                }
                None => points.len(),
            };
            let mode = match p.get("mode") {
                None => 4,
                Some(m) => m
                    .as_index()
                    .ok_or_else(|| bad(Some(&format!("{at}/mode")), "`mode` is not an index"))?,
            };
            let t = match mode {
                4 if n % 3 == 0 => n / 3,
                4 => {
                    return Err(bad(
                        Some(&at),
                        format!("a triangle list of {n} indices, not a multiple of 3"),
                    ))
                }
                5 | 6 if n >= 3 => n - 2,
                5 | 6 => {
                    return Err(bad(
                        Some(&at),
                        format!("a triangle strip or fan of {n} indices, fewer than 3"),
                    ))
                }
                0..=3 => 0,
                _ => {
                    return Err(bad(
                        Some(&format!("{at}/mode")),
                        format!("mode {mode} is not a glTF mode"),
                    ))
                }
            };
            self.triangles = self.triangles.saturating_add(t as u64);
            let order: Vec<usize> = match p.get("indices") {
                Some(i) if self.want_plan => {
                    self.indices(i.as_index().unwrap_or(usize::MAX), &format!("{at}/indices"))?
                }
                _ if self.want_plan => (0..points.len()).collect(),
                _ => Vec::new(),
            };
            let mut mapped: Vec<[i64; 3]> = Vec::with_capacity(points.len());
            for v in points {
                let p = [
                    world[0][0] * v[0] + world[0][1] * v[1] + world[0][2] * v[2] + world[0][3],
                    world[1][0] * v[0] + world[1][1] * v[1] + world[1][2] * v[2] + world[1][3],
                    world[2][0] * v[0] + world[2][1] * v[1] + world[2][2] * v[2] + world[2][3],
                ];
                let a = self.axes.map(p);
                let mut mm_point = [0i64; 3];
                for (k, ak) in a.iter().enumerate() {
                    let mm = via_num::round_to_lattice(ak * 1000.0 * self.scale).map_err(|_| {
                        bad(
                            Some(&format!("/accessors/{pos}")),
                            "a position lies outside the lattice, or is not finite",
                        )
                    })?;
                    if mm < self.lo[k] {
                        self.lo[k] = mm;
                    }
                    if mm > self.hi[k] {
                        self.hi[k] = mm;
                    }
                    mm_point[k] = mm;
                }
                if self.want_plan {
                    mapped.push(mm_point);
                }
            }
            // The triangles in plan (3.4): what the mode makes of the index list.
            let tri = |a: usize, b: usize, c: usize| -> Option<[[i64; 2]; 3]> {
                let (pa, pb, pc) = (
                    mapped.get(*order.get(a)?)?,
                    mapped.get(*order.get(b)?)?,
                    mapped.get(*order.get(c)?)?,
                );
                Some([[pa[0], pa[1]], [pb[0], pb[1]], [pc[0], pc[1]]])
            };
            for k in 0..if self.want_plan { t } else { 0 } {
                let got = match mode {
                    4 => tri(3 * k, 3 * k + 1, 3 * k + 2),
                    5 => tri(k, k + 1, k + 2),
                    _ => tri(0, k + 1, k + 2),
                };
                if let Some(g) = got {
                    self.plan_tris.push(g);
                }
            }
        }
        Ok(())
    }

    /// A `SCALAR` index accessor's values (glTF 2.0 3.7.2.1: unsigned byte, short or int).
    fn indices(&self, ai: usize, here: &str) -> Result<Vec<usize>, Fault> {
        let Some(acc) = self.m.list("accessors").get(ai) else {
            return Err(bad(Some(here), "names no accessor"));
        };
        let ct = acc
            .get("componentType")
            .and_then(Json::as_index)
            .unwrap_or(0);
        let size = component_size(ct)
            .filter(|_| matches!(ct, 5121 | 5123 | 5125))
            .ok_or_else(|| bad(Some(here), "an index accessor of an unknown type"))?;
        let count = acc.get("count").and_then(Json::as_index).unwrap_or(0);
        let Some(bv) = acc.get("bufferView").and_then(Json::as_index) else {
            return Ok(vec![0; count]);
        };
        let (data, start, stride) = self.view(bv, here)?;
        let start = start + acc.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
        let stride = stride.unwrap_or(size);
        (0..count)
            .map(|i| {
                read(data, start + i * stride, ct, false)
                    .map(|v| v as usize)
                    .ok_or_else(|| bad(Some(here), "the index accessor reads past its buffer"))
            })
            .collect()
    }

    /// A `VEC3` accessor's values as doubles, sparse values applied (glTF 2.0, 3.6.2).
    fn vec3(&self, ai: usize) -> Result<Vec<[f64; 3]>, Fault> {
        let here = format!("/accessors/{ai}");
        let Some(acc) = self.m.list("accessors").get(ai) else {
            return Err(bad(Some(&here), "`POSITION` names no accessor"));
        };
        if acc.get("type").and_then(Json::as_str) != Some("VEC3") {
            return Err(bad(
                Some(&format!("{here}/type")),
                "`POSITION` is not a VEC3 accessor",
            ));
        }
        let ct = acc
            .get("componentType")
            .and_then(Json::as_index)
            .unwrap_or(0);
        let size = component_size(ct).ok_or_else(|| {
            bad(
                Some(&format!("{here}/componentType")),
                "an unknown component type",
            )
        })?;
        let normalized = acc
            .get("normalized")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let count = acc
            .get("count")
            .and_then(Json::as_index)
            .ok_or_else(|| bad(Some(&format!("{here}/count")), "`count` is not an index"))?;
        let mut out = vec![[0.0; 3]; count];
        if let Some(bv) = acc.get("bufferView") {
            let bv = bv
                .as_index()
                .ok_or_else(|| bad(Some(&format!("{here}/bufferView")), "not an index"))?;
            let (data, start, stride) = self.view(bv, &here)?;
            let start = start + acc.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
            let stride = stride.unwrap_or(3 * size);
            for (i, o) in out.iter_mut().enumerate() {
                for (c, v) in o.iter_mut().enumerate() {
                    *v = read(data, start + i * stride + c * size, ct, normalized)
                        .ok_or_else(|| bad(Some(&here), "the accessor reads past its buffer"))?;
                }
            }
        }
        if let Some(sp) = acc.get("sparse") {
            let sh = format!("{here}/sparse");
            let n = sp.get("count").and_then(Json::as_index).unwrap_or(0);
            let ix = sp
                .get("indices")
                .ok_or_else(|| bad(Some(&sh), "no `indices`"))?;
            let vs = sp
                .get("values")
                .ok_or_else(|| bad(Some(&sh), "no `values`"))?;
            let ibv = ix
                .get("bufferView")
                .and_then(Json::as_index)
                .ok_or_else(|| bad(Some(&format!("{sh}/indices")), "no `bufferView`"))?;
            let ict = ix
                .get("componentType")
                .and_then(Json::as_index)
                .unwrap_or(0);
            let isize = component_size(ict)
                .filter(|_| matches!(ict, 5121 | 5123 | 5125))
                .ok_or_else(|| bad(Some(&format!("{sh}/indices")), "an unknown index type"))?;
            let (idata, istart, _) = self.view(ibv, &sh)?;
            let istart = istart + ix.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
            let vbv = vs
                .get("bufferView")
                .and_then(Json::as_index)
                .ok_or_else(|| bad(Some(&format!("{sh}/values")), "no `bufferView`"))?;
            let (vdata, vstart, _) = self.view(vbv, &sh)?;
            let vstart = vstart + vs.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
            for k in 0..n {
                let target = read(idata, istart + k * isize, ict, false)
                    .ok_or_else(|| bad(Some(&sh), "sparse indices read past their buffer"))?;
                let target = target as usize;
                let Some(o) = out.get_mut(target) else {
                    return Err(bad(Some(&sh), "a sparse index lies past `count`"));
                };
                for (c, v) in o.iter_mut().enumerate() {
                    *v = read(vdata, vstart + (k * 3 + c) * size, ct, normalized)
                        .ok_or_else(|| bad(Some(&sh), "sparse values read past their buffer"))?;
                }
            }
        }
        Ok(out)
    }

    /// A buffer view's bytes, its start and its stride.
    fn view(&self, bv: usize, from: &str) -> Result<(&[u8], usize, Option<usize>), Fault> {
        let here = format!("/bufferViews/{bv}");
        let Some(v) = self.m.list("bufferViews").get(bv) else {
            return Err(bad(Some(from), "names no buffer view"));
        };
        let b = v
            .get("buffer")
            .and_then(Json::as_index)
            .ok_or_else(|| bad(Some(&format!("{here}/buffer")), "not an index"))?;
        let data = self
            .buffers
            .get(b)
            .ok_or_else(|| bad(Some(&format!("{here}/buffer")), "names no buffer"))?;
        let off = v.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
        let len = v.get("byteLength").and_then(Json::as_index).unwrap_or(0);
        let Some(slice) = data.get(off..off.saturating_add(len)) else {
            return Err(bad(Some(&here), "the buffer view runs past its buffer"));
        };
        let stride = v.get("byteStride").and_then(Json::as_index);
        Ok((slice, 0, stride))
    }
}

/// What a model covers in plan (`assets.md` 3.4): the cells of a lattice laid from `[lo0, lo1]`
/// that a triangle of positive area overlaps, traced into rings. `None` where every cell is
/// covered, or none is.
pub fn plan_rings(
    tris: &[[[i64; 2]; 3]],
    lo: [i64; 3],
    hi: [i64; 3],
) -> Option<Vec<Vec<[i64; 2]>>> {
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    if w <= 0 || h <= 0 {
        return None;
    }
    let c = 10.max((w.max(h) + 511) / 512);
    let (nx, ny) = (((w + c - 1) / c) as usize, ((h + c - 1) / c) as usize);
    let mut cov = vec![false; nx * ny];
    for t in tris {
        let a2 =
            (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]);
        if a2 == 0 {
            continue;
        }
        let xs = [t[0][0], t[1][0], t[2][0]];
        let ys = [t[0][1], t[1][1], t[2][1]];
        let (tx0, tx1) = (*xs.iter().min()?, *xs.iter().max()?);
        let (ty0, ty1) = (*ys.iter().min()?, *ys.iter().max()?);
        let i0 = ((tx0 - lo[0]) / c).max(0) as usize;
        let i1 = (((tx1 - lo[0]) + c - 1) / c).min(nx as i64) as usize;
        let j0 = ((ty0 - lo[1]) / c).max(0) as usize;
        let j1 = (((ty1 - lo[1]) + c - 1) / c).min(ny as i64) as usize;
        for j in j0..j1 {
            for i in i0..i1 {
                let x0 = lo[0] + i as i64 * c;
                let y0 = lo[1] + j as i64 * c;
                if !cov[j * nx + i] && tri_overlaps_cell(t, a2 > 0, [x0, y0, x0 + c, y0 + c]) {
                    cov[j * nx + i] = true;
                }
            }
        }
    }
    let n = cov.iter().filter(|v| **v).count();
    if n == 0 || n == cov.len() {
        return None;
    }
    Some(trace(&cov, nx, ny, c, lo, hi))
}

/// Whether a triangle (`ccw`: its winding) overlaps the inside of a cell `[x0, y0, x1, y1]`:
/// separating axes, the cell's two and the triangle's three; touching is no overlap.
fn tri_overlaps_cell(t: &[[i64; 2]; 3], ccw: bool, cell: [i64; 4]) -> bool {
    let xs = [t[0][0], t[1][0], t[2][0]];
    let ys = [t[0][1], t[1][1], t[2][1]];
    if xs.iter().max().copied().unwrap_or(i64::MIN) <= cell[0]
        || xs.iter().min().copied().unwrap_or(i64::MAX) >= cell[2]
        || ys.iter().max().copied().unwrap_or(i64::MIN) <= cell[1]
        || ys.iter().min().copied().unwrap_or(i64::MAX) >= cell[3]
    {
        return false;
    }
    let corners = [
        [cell[0], cell[1]],
        [cell[2], cell[1]],
        [cell[2], cell[3]],
        [cell[0], cell[3]],
    ];
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        // Every corner on the outer side of this edge (or on it): separated.
        let outside = corners.iter().all(|p| {
            let cr = i128::from(b[0] - a[0]) * i128::from(p[1] - a[1])
                - i128::from(b[1] - a[1]) * i128::from(p[0] - a[0]);
            if ccw {
                cr <= 0
            } else {
                cr >= 0
            }
        });
        if outside {
            return false;
        }
    }
    true
}

/// The rings round the covered cells of a grid, along cell edges: outer rings counter-clockwise,
/// holes clockwise, a vertex only where the outline turns, coordinates held to `hi`.
fn trace(
    cov: &[bool],
    nx: usize,
    ny: usize,
    c: i64,
    lo: [i64; 3],
    hi: [i64; 3],
) -> Vec<Vec<[i64; 2]>> {
    let on = |i: i64, j: i64| {
        i >= 0
            && j >= 0
            && (i as usize) < nx
            && (j as usize) < ny
            && cov[j as usize * nx + i as usize]
    };
    // Directed boundary edges between lattice corners, the covered cell on their left.
    let mut next: BTreeMap<(i64, i64), Vec<(i64, i64)>> = BTreeMap::new();
    for j in 0..ny as i64 {
        for i in 0..nx as i64 {
            if !on(i, j) {
                continue;
            }
            if !on(i, j - 1) {
                next.entry((i, j)).or_default().push((i + 1, j));
            }
            if !on(i + 1, j) {
                next.entry((i + 1, j)).or_default().push((i + 1, j + 1));
            }
            if !on(i, j + 1) {
                next.entry((i + 1, j + 1)).or_default().push((i, j + 1));
            }
            if !on(i - 1, j) {
                next.entry((i, j + 1)).or_default().push((i, j));
            }
        }
    }
    let mut rings = Vec::new();
    // Each vertex once, in order, as a start: the work is the edges', not islands times edges (F5's
    // review: 65 536 islands took 25 s when each ring searched the map from its start).
    let starts: Vec<(i64, i64)> = next.keys().copied().collect();
    for start in starts {
        while next.get(&start).is_some_and(|v| !v.is_empty()) {
            let mut ring = vec![start];
            let mut at = start;
            let mut came: Option<(i64, i64)> = None;
            loop {
                let outs = next
                    .get_mut(&at)
                    .expect("an edge leaves every vertex on a ring");
                // At a corner two cells touch by: turn left (keep the covered cell on the left).
                let k = match (came, outs.len()) {
                    (Some(d), n) if n > 1 => {
                        let left = (-d.1, d.0);
                        outs.iter()
                            .position(|o| (o.0 - at.0, o.1 - at.1) == left)
                            .unwrap_or(0)
                    }
                    _ => 0,
                };
                let to = outs.remove(k);
                came = Some((to.0 - at.0, to.1 - at.1));
                at = to;
                if at == start {
                    break;
                }
                ring.push(at);
            }
            // Keep only the vertices where the outline turns.
            let m = ring.len();
            let turned: Vec<(i64, i64)> = (0..m)
                .filter(|&k| {
                    let (p, q, r) = (ring[(k + m - 1) % m], ring[k], ring[(k + 1) % m]);
                    (q.0 - p.0, q.1 - p.1).0.signum() != (r.0 - q.0, r.1 - q.1).0.signum()
                        || (q.0 - p.0, q.1 - p.1).1.signum() != (r.0 - q.0, r.1 - q.1).1.signum()
                })
                .map(|k| ring[k])
                .collect();
            rings.push(
                turned
                    .into_iter()
                    .map(|(i, j)| [(lo[0] + i * c).min(hi[0]), (lo[1] + j * c).min(hi[1])])
                    .collect(),
            );
        }
    }
    rings.sort();
    rings
}

fn component_size(ct: usize) -> Option<usize> {
    match ct {
        5120 | 5121 => Some(1),
        5122 | 5123 => Some(2),
        5125 | 5126 => Some(4),
        _ => None,
    }
}

/// One component, as glTF 2.0 3.11 ("Animations", normalised values) and
/// `KHR_mesh_quantization` read it.
fn read(data: &[u8], at: usize, ct: usize, normalized: bool) -> Option<f64> {
    let b = data.get(at..at + component_size(ct)?)?;
    Some(match ct {
        5126 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        5125 => f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        5120 => {
            let c = f64::from(i8::from_le_bytes([b[0]]));
            if normalized {
                via_num::max(c / 127.0, -1.0)
            } else {
                c
            }
        }
        5121 => {
            let c = f64::from(b[0]);
            if normalized {
                c / 255.0
            } else {
                c
            }
        }
        5122 => {
            let c = f64::from(i16::from_le_bytes([b[0], b[1]]));
            if normalized {
                via_num::max(c / 32767.0, -1.0)
            } else {
                c
            }
        }
        5123 => {
            let c = f64::from(u16::from_le_bytes([b[0], b[1]]));
            if normalized {
                c / 65535.0
            } else {
                c
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles of a rectangle in plan, millimetres.
    fn quad(x0: i64, y0: i64, x1: i64, y1: i64) -> [[[i64; 2]; 3]; 2] {
        [
            [[x0, y0], [x1, y0], [x1, y1]],
            [[x0, y0], [x1, y1], [x0, y1]],
        ]
    }

    #[test]
    fn a_box_covers_its_rectangle_and_writes_no_plan() {
        let t = quad(0, 0, 1000, 500);
        assert_eq!(plan_rings(&t, [0, 0, 0], [1000, 500, 300]), None);
    }

    #[test]
    fn an_l_is_one_ring_counter_clockwise_by_its_turns() {
        let mut t: Vec<[[i64; 2]; 3]> = quad(0, 0, 2000, 1000).to_vec();
        t.extend(quad(0, 1000, 1000, 2000));
        let rings = plan_rings(&t, [0, 0, 0], [2000, 2000, 300]).expect("an L is no rectangle");
        assert_eq!(rings.len(), 1);
        let r = &rings[0];
        assert_eq!(r.len(), 6);
        for p in [
            [0, 0],
            [2000, 0],
            [2000, 1000],
            [1000, 1000],
            [1000, 2000],
            [0, 2000],
        ] {
            assert!(r.contains(&p), "{p:?} in {r:?}");
        }
        let area2: i64 = (0..r.len())
            .map(|k| r[k][0] * r[(k + 1) % r.len()][1] - r[(k + 1) % r.len()][0] * r[k][1])
            .sum();
        assert_eq!(area2, 2 * 3_000_000, "counter-clockwise, 3 m2");
    }

    #[test]
    fn a_frame_has_a_hole_and_vertical_faces_have_no_plan() {
        let mut t: Vec<[[i64; 2]; 3]> = Vec::new();
        for q in [
            quad(0, 0, 3000, 1000),
            quad(0, 2000, 3000, 3000),
            quad(0, 1000, 1000, 2000),
            quad(2000, 1000, 3000, 2000),
        ] {
            t.extend(q);
        }
        let rings = plan_rings(&t, [0, 0, 0], [3000, 3000, 300]).expect("a frame");
        assert_eq!(rings.len(), 2);
        let area2 = |r: &Vec<[i64; 2]>| -> i64 {
            (0..r.len())
                .map(|k| r[k][0] * r[(k + 1) % r.len()][1] - r[(k + 1) % r.len()][0] * r[k][1])
                .sum()
        };
        let mut areas: Vec<i64> = rings.iter().map(area2).collect();
        areas.sort();
        assert_eq!(
            areas,
            vec![-2_000_000, 18_000_000],
            "the hole clockwise, the outer ring counter-clockwise"
        );
        // A wall's faces stand on edge: no area in plan, no plan.
        let wall = [[[0, 0], [1000, 0], [1000, 0]], [[0, 0], [0, 0], [1000, 0]]];
        assert_eq!(plan_rings(&wall, [0, 0, 0], [1000, 200, 3000]), None);
    }

    #[test]
    fn many_islands_trace_in_time_and_touching_corners_are_apart() {
        // F5's review: each ring searched the map from its start, 65 536 islands took 25 s.
        let mut t = Vec::new();
        for j in 0..128 {
            for i in 0..128 {
                if (i + j) % 2 == 0 {
                    t.extend(quad(i * 20, j * 20, i * 20 + 20, j * 20 + 20));
                }
            }
        }
        let started = std::time::Instant::now();
        let rings = plan_rings(&t, [0, 0, 0], [2560, 2560, 100]).expect("a chequer");
        assert_eq!(
            rings.len(),
            128 * 64,
            "cells that touch at a corner are apart"
        );
        assert!(started.elapsed().as_secs() < 5);
    }

    #[test]
    fn a_bevel_of_a_few_centimetres_is_seen() {
        // Kenney's corner block (K10): a metre square less a 139 mm bevel at one corner.
        let t = vec![
            [[0, 0], [1000, 0], [1000, 861]],
            [[0, 0], [1000, 861], [861, 1000]],
            [[0, 0], [861, 1000], [0, 1000]],
        ];
        let rings =
            plan_rings(&t, [0, 0, 0], [1000, 1000, 625]).expect("the bevel leaves cells open");
        assert_eq!(rings.len(), 1);
        assert!(
            !rings[0].contains(&[1000, 1000]),
            "the bevelled corner is not covered"
        );
    }

    /// A `.gltf` of one triangle list over `positions`, its buffer a data URI.
    fn gltf(positions: &[[f32; 3]], node: &str, extra: &str) -> Vec<u8> {
        let mut bin = Vec::new();
        for p in positions {
            for c in p {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let b64 = {
            const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut s = String::new();
            for ch in bin.chunks(3) {
                let mut w = [0u8; 3];
                w[..ch.len()].copy_from_slice(ch);
                let n = (u32::from(w[0]) << 16) | (u32::from(w[1]) << 8) | u32::from(w[2]);
                for k in 0..4 {
                    if k <= ch.len() {
                        s.push(A[((n >> (18 - 6 * k)) & 63) as usize] as char);
                    } else {
                        s.push('=');
                    }
                }
            }
            s
        };
        format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"mesh":0{node}}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}}}]}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":{n},"type":"VEC3"}}],
            "bufferViews":[{{"buffer":0,"byteLength":{len}}}],
            "buffers":[{{"byteLength":{len},"uri":"data:application/octet-stream;base64,{b64}"}}]{extra}}}"#,
            n = positions.len(),
            len = bin.len(),
        )
        .into_bytes()
    }

    #[test]
    fn default_axes_map_like_the_chapter() {
        let a = Axes::parse("+z", "+y").unwrap();
        assert_eq!(a.map([1.0, 2.0, 3.0]), [-1.0, 3.0, 2.0]);
        assert!(Axes::parse("+y", "-y").is_none());
        let z_up = Axes::parse("-y", "+z").unwrap();
        assert_eq!(z_up.map([1.0, 2.0, 3.0]), [-1.0, -2.0, 3.0]);
    }

    #[test]
    fn measures_a_translated_triangle_list() {
        let bytes = gltf(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 1.5000001]],
            r#","translation":[1,0,0]"#,
            "",
        );
        let m = Model::open(&bytes).unwrap();
        let r = m
            .measure(
                &BTreeMap::new(),
                Axes::parse("+z", "+y").unwrap(),
                false,
                1.0,
            )
            .unwrap();
        assert_eq!(r.triangles, 1);
        assert_eq!(r.raw, [-2000, 0, 0, -1000, 1500, 1000]);
        assert!(r.untextured);
    }

    #[test]
    fn refuses_compression_and_instancing() {
        let bytes = gltf(
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            "",
            r#","extensionsUsed":["EXT_mesh_gpu_instancing"]"#,
        );
        let f = Model::open(&bytes).err().unwrap();
        assert_eq!(f.pointer.as_deref(), Some("/extensionsUsed/0"));
    }
}
