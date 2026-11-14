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
            for v in points {
                let p = [
                    world[0][0] * v[0] + world[0][1] * v[1] + world[0][2] * v[2] + world[0][3],
                    world[1][0] * v[0] + world[1][1] * v[1] + world[1][2] * v[2] + world[1][3],
                    world[2][0] * v[0] + world[2][1] * v[1] + world[2][2] * v[2] + world[2][3],
                ];
                let a = self.axes.map(p);
                for (k, ak) in a.iter().enumerate() {
                    let mm = via_num::round_to_lattice(ak * 1000.0).map_err(|_| {
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
                }
            }
        }
        Ok(())
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
            .measure(&BTreeMap::new(), Axes::parse("+z", "+y").unwrap())
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
