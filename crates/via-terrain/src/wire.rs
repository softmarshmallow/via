//! The terrain step on the step protocol (`spec/protocol.md`): the request read, the facts read as
//! feature lists, and the answer written as a document, field files and diagnostics.
//!
//! One program serves both transports (R22). With `VIA_STEP` set it is a plain command and reads
//! the request's `data`; otherwise it is a worker (`via-step`) and asks the engine for what it reads
//! (`build.field`, `build.features`), so its trace names exactly that and an edit elsewhere leaves
//! it reused. Both reach the engine by the door a Python step uses (R19b).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use via_num as num;
use via_proto::column::{self, Column};
use via_proto::npy;
use via_proto::store::Store;

use crate::facts::{self, Finding, Found, Limits};
use crate::settle::{self, Hold, Input, Lake, Params, Posts, River, Settled};

/// The kinds this library declares (`lib/terrain/kinds.toml`). The step reads its facts by them.
pub const LAKE: &str = "terrain_lake";
pub const RIVER: &str = "terrain_river";
pub const HOLD: &str = "terrain_hold";
pub const COURSE: &str = "terrain_course";

/// The fewest post spacings a course's drawn channel spans (its `channel` attribute).
pub const CHANNEL_POSTS: f64 = 3.0;

/// The row's id on each field it writes, and the tags of its own fields.
pub const ROW: &str = "erosion";

/// A fault the step reports as its own diagnostic (`step.<id>.<code>`), or a fault of the
/// protocol it reports by failing.
#[derive(Debug)]
pub enum Fault {
    /// Reported as an error diagnostic of the step; the call itself succeeds.
    Said(Box<Finding>),
    /// The call fails (`protocol.raised` from a worker, `protocol.failed` from a plain command).
    Broken(String),
}

impl From<String> for Fault {
    fn from(s: String) -> Self {
        Fault::Broken(s)
    }
}

/// A feature from a feature list (4.3): its name, kind, shape and attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub name: String,
    pub kind: String,
    pub authority: String,
    pub geometry: u8,
    /// Every ring of every piece, millimetres.
    pub rings: Vec<Vec<(i64, i64)>>,
    pub attrs: BTreeMap<String, Attr>,
}

/// One attribute value.
#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Num(f64),
    Text(String),
}

impl Feature {
    pub fn num(&self, key: &str) -> Option<f64> {
        match self.attrs.get(key) {
            Some(Attr::Num(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        match self.attrs.get(key) {
            Some(Attr::Text(v)) => Some(v),
            _ => None,
        }
    }

    /// The first ring's vertices in metres: a point's one vertex, a path's line.
    pub fn line_m(&self) -> Vec<(f64, f64)> {
        self.rings
            .first()
            .map(|r| {
                r.iter()
                    .map(|&(x, y)| (num::metres(x), num::metres(y)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn col(cols: &BTreeMap<String, Column>, name: &str) -> Result<Column, String> {
    cols.get(name)
        .cloned()
        .ok_or_else(|| format!("the feature list has no `{name}` column"))
}

fn ints(cols: &BTreeMap<String, Column>, name: &str) -> Result<Vec<i64>, String> {
    col(cols, name)?
        .as_i64()
        .ok_or_else(|| format!("`{name}` is not an integer column"))
}

fn strs(cols: &BTreeMap<String, Column>, name: &str) -> Result<Vec<String>, String> {
    match col(cols, name)? {
        Column::Str(v) => Ok(v),
        _ => Err(format!("`{name}` is not a str column")),
    }
}

const STRUCTURE: [&str; 9] = [
    "coords",
    "geometry",
    "pieces",
    "rings",
    "ring_starts",
    "name",
    "kind",
    "authority",
    "layer",
];

/// Reads a feature list (4.3) from its JSON form, blobs from the store.
pub fn features(list: &Value, store: &Store) -> Result<Vec<Feature>, String> {
    let obj = list
        .as_object()
        .ok_or("a feature list is an object of columns")?;
    let mut cols = BTreeMap::new();
    for (k, v) in obj {
        let c = column::read(k, v, store).map_err(|e| e.message)?;
        cols.insert(k.clone(), c);
    }
    let names = strs(&cols, "name")?;
    let n = names.len();
    let kinds = strs(&cols, "kind")?;
    let auth = strs(&cols, "authority").unwrap_or_else(|_| vec![String::new(); n]);
    let geometry = ints(&cols, "geometry")?;
    let pieces = ints(&cols, "pieces")?;
    let rings = ints(&cols, "rings")?;
    let starts = ints(&cols, "ring_starts")?;
    let coords = ints(&cols, "coords")?;
    if pieces.len() != n + 1 || kinds.len() != n || geometry.len() != n {
        return Err("the feature list's columns differ in length".into());
    }
    let mut out = Vec::with_capacity(n);
    for f in 0..n {
        let mut rs = Vec::new();
        for p in pieces[f]..pieces[f + 1] {
            let p = p as usize;
            for r in rings[p]..rings[p + 1] {
                let r = r as usize;
                let (a, b) = (starts[r] as usize, starts[r + 1] as usize);
                let ring: Vec<(i64, i64)> =
                    (a..b).map(|v| (coords[2 * v], coords[2 * v + 1])).collect();
                rs.push(ring);
            }
        }
        let mut attrs = BTreeMap::new();
        for (k, c) in &cols {
            if STRUCTURE.contains(&k.as_str()) || k.ends_with('?') {
                continue;
            }
            let present = cols
                .get(&format!("{k}?"))
                .and_then(Column::as_i64)
                .is_none_or(|v| v.get(f) == Some(&1));
            if !present {
                continue;
            }
            let value = match c {
                Column::Str(v) => v.get(f).map(|s| Attr::Text(s.clone())),
                // A lattice length travels as i32 millimetres (4.3).
                Column::I32(v) => v.get(f).map(|&x| Attr::Num(num::metres(i64::from(x)))),
                other => other
                    .as_f64()
                    .and_then(|v| v.get(f).copied())
                    .map(Attr::Num),
            };
            if let Some(v) = value {
                attrs.insert(k.clone(), v);
            }
        }
        out.push(Feature {
            name: names[f].clone(),
            kind: kinds[f].clone(),
            authority: auth[f].clone(),
            geometry: geometry[f] as u8,
            rings: rs,
            attrs,
        });
    }
    Ok(out)
}

/// The request's grid.
pub fn posts(request: &Value) -> Result<Posts, String> {
    let g = &request["grid"];
    let e: Vec<i64> = g["extent_mm"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let (res, nx, ny) = (
        g["resolution_mm"].as_i64(),
        g["nx"].as_u64(),
        g["ny"].as_u64(),
    );
    match (e.as_slice(), res, nx, ny) {
        ([x0, y0, _, _], Some(res), Some(nx), Some(ny)) if res > 0 => Ok(Posts {
            x0: *x0,
            y0: *y0,
            res,
            nx: nx as usize,
            ny: ny as usize,
        }),
        _ => Err("the request's grid is malformed".into()),
    }
}

/// The request's seed, 16 hexadecimal digits.
pub fn seed(request: &Value) -> Result<u64, String> {
    request["seed"]
        .as_str()
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .ok_or_else(|| "the request's seed is not 16 hexadecimal digits".into())
}

/// A field file's values as doubles, post order; checks its shape against the grid.
pub fn field_values(path: &Path, posts: Posts) -> Result<Vec<f64>, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("cannot read field {}: {e}", path.display()))?;
    let h = npy::parse(&bytes).map_err(|e| e.0)?;
    if h.shape != [posts.ny, posts.nx] {
        return Err(format!(
            "field {} has shape {:?}, not ({}, {})",
            path.display(),
            h.shape,
            posts.ny,
            posts.nx
        ));
    }
    Ok(bytes[h.data_offset..]
        .chunks_exact(4)
        .map(|c| f64::from(f32::from_le_bytes([c[0], c[1], c[2], c[3]])))
        .collect())
}

fn param_num(p: &Value, key: &str) -> Result<Option<f64>, Fault> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .filter(|x| x.is_finite())
            .map(Some)
            .ok_or_else(|| {
                said(
                    "params",
                    "step",
                    format!("`params.{key}` is not a number"),
                    format!("write `{key}` as a number"),
                )
            }),
    }
}

fn said(code: &'static str, subject: &str, message: String, hint: String) -> Fault {
    Fault::Said(Box::new(Finding {
        severity: "error",
        code,
        subject: subject.into(),
        message,
        hint,
        position: None,
        data: "{}".into(),
    }))
}

/// The generate step's parameters from `params`.
pub fn params(p: &Value) -> Result<Params, Fault> {
    let d = Params::default();
    let count = |key: &str, dflt: usize| -> Result<usize, Fault> {
        match param_num(p, key)? {
            None => Ok(dflt),
            Some(v) if v >= 0.0 && v.fract_part_is_zero() => Ok(v as usize),
            Some(_) => Err(said(
                "params",
                "step",
                format!("`params.{key}` is not a whole number of cells"),
                format!("write `{key}` as a whole number, 0 or more"),
            )),
        }
    };
    let edge_held = match p.get("edge").and_then(Value::as_str) {
        None | Some("held") => true,
        Some("open") => false,
        Some(other) => {
            return Err(said(
                "params",
                "step",
                format!("`params.edge` is `{other}`, not `held` or `open`"),
                "write `edge = \"held\"` (the land at the edge stays as it lies) or `\"open\"`"
                    .into(),
            ))
        }
    };
    Ok(Params {
        cell: param_num(p, "cell")?,
        margin: count("margin", d.margin)?,
        sea: param_num(p, "sea")?.unwrap_or(d.sea),
        edge_held,
        hold_cells: count("hold_cells", d.hold_cells)?,
        steps: param_num(p, "steps")?.map(|v| v as u32),
        erodibility: param_num(p, "erodibility")?,
        diffusivity: param_num(p, "diffusivity")?,
        deposition: param_num(p, "deposition")?,
        timestep: param_num(p, "timestep")?,
    })
}

/// Whole-number test without the denied `fract`.
trait Whole {
    fn fract_part_is_zero(self) -> bool;
}

impl Whole for f64 {
    fn fract_part_is_zero(self) -> bool {
        self.floor() == self
    }
}

/// The lint's limits from `params`.
pub fn limits(p: &Value) -> Result<Limits, Fault> {
    let d = Limits::default();
    Ok(Limits {
        level_within: param_num(p, "level_within")?.unwrap_or(d.level_within),
        follow_within: param_num(p, "follow_within")?.unwrap_or(d.follow_within),
        min_catchment: param_num(p, "min_catchment")?.unwrap_or(d.min_catchment),
        hold_within: param_num(p, "hold_within")?.unwrap_or(d.hold_within),
        sea: param_num(p, "sea")?.unwrap_or(d.sea),
    })
}

fn need(f: &Feature, key: &str) -> Result<f64, Fault> {
    f.num(key).ok_or_else(|| {
        said(
            "attribute",
            &format!("feature.{}", f.name),
            format!("`{}` has no number `{key}`", f.name),
            format!("write `{key}` on it, or include the terrain library that declares it"),
        )
    })
}

/// The stated lakes among features.
pub fn lakes(fs: &[Feature]) -> Result<Vec<Lake>, Fault> {
    fs.iter()
        .filter(|f| f.kind == LAKE && f.geometry == 2)
        .map(|f| {
            Ok(Lake {
                name: f.name.clone(),
                rings: f.rings.clone(),
                level: need(f, "level")?,
                depth: f.num("depth").unwrap_or(4.0),
                shore: f.num("shore").unwrap_or(30.0),
            })
        })
        .collect()
}

/// The stated rivers among features.
pub fn rivers(fs: &[Feature]) -> Result<Vec<River>, Fault> {
    fs.iter()
        .filter(|f| f.kind == RIVER && f.geometry == 1)
        .map(|f| {
            Ok(River {
                name: f.name.clone(),
                line: f.rings.first().cloned().unwrap_or_default(),
                incise: f.num("incise").unwrap_or(2.0),
                fall: f.num("fall").unwrap_or(0.003),
                valley: f.num("valley").unwrap_or(70.0),
                width: f.num("width").filter(|w| *w > 0.0),
            })
        })
        .collect()
}

/// The holds among features.
pub fn holds(fs: &[Feature]) -> Vec<Hold> {
    fs.iter()
        .filter(|f| f.kind == HOLD)
        .map(|f| Hold {
            name: f.name.clone(),
            rings: f.rings.clone(),
            geometry: f.geometry,
            level: f.num("level"),
        })
        .collect()
}

/// The courses a generate step returned, among features.
pub fn found(fs: &[Feature], step: Option<&str>) -> Vec<Found> {
    fs.iter()
        .filter(|f| f.kind == COURSE)
        .filter(|f| step.is_none_or(|s| f.name.starts_with(&format!("{s}#"))))
        .filter_map(|f| {
            Some(Found {
                name: f.name.clone(),
                fact: f.text("fact")?.to_string(),
                points: f.line_m(),
                end: f.text("ends").unwrap_or("").to_string(),
                catchment: f.num("catchment"),
            })
        })
        .collect()
}

/// The field names a returned document writes rows on.
#[derive(Clone, Debug, Default)]
pub struct Targets {
    pub surface: String,
    pub water: Option<String>,
    pub order: Option<i64>,
}

fn decimal(v: f64, places: u32) -> String {
    let p = 10u64.pow(places);
    let m = (v * p as f64).round_ties_even() as i64;
    let sign = if m < 0 { "-" } else { "" };
    let a = m.unsigned_abs();
    let frac = format!("{:0width$}", a % p, width = places as usize);
    format!("{sign}{}.{frac}", a / p)
}

fn lattice_point((x, y): (i64, i64)) -> String {
    format!("[{}, {}]", num::write_length(x), num::write_length(y))
}

/// The document a generate call returns (5), and its field files as `(name, values)`.
pub fn document<'a>(
    s: &'a Settled,
    start: &'a [f64],
    t: &Targets,
    spacing_mm: i64,
) -> (String, Vec<(&'static str, &'a [f64])>) {
    let files: Vec<(&'static str, &'a [f64])> = vec![
        ("before", start),
        ("eroded", &s.eroded),
        ("standing", &s.standing),
        ("level", &s.level),
        ("wet", &s.wet),
        ("damp", &s.damp),
        ("silt", &s.silt),
    ];
    // `before` is the ground the step read: what a hold is checked against (R14, `facts`).
    let meta = [
        ("before", Some("m"), "settled_from"),
        ("eroded", Some("m"), "eroded"),
        ("standing", Some("m"), "standing_water"),
        ("level", Some("m"), "standing_level"),
        ("wet", None, "wet"),
        ("damp", None, "damp"),
        ("silt", Some("m"), "silt"),
    ];
    let mut d = String::new();
    d.push_str("# The land settled by the terrain library's step (lib/terrain).\n");
    for (name, unit, tag) in meta {
        d.push_str(&format!(
            "\n[field.{name}]\nfile = \"{name}.npy\"\noutside = 0\n"
        ));
        if let Some(u) = unit {
            d.push_str(&format!("unit = \"{u}\"\n"));
        }
        d.push_str(&format!("tags = [\"{tag}\"]\n"));
    }
    let order = t
        .order
        .map(|o| format!("order = {o}\n"))
        .unwrap_or_default();
    d.push_str(&format!(
        "\n[field.{}.then.{ROW}]\nvalue = \"= f('eroded')\"\n{order}",
        t.surface
    ));
    if let Some(w) = &t.water {
        d.push_str(&format!(
            "\n[field.{w}.then.{ROW}]\nvalue = \"= max(v, f('level'))\"\n{order}"
        ));
    }
    // A course is a path: water that goes nowhere from its source has none, and the lint says so.
    for c in s.courses.iter().filter(|c| c.points.len() >= 2) {
        d.push_str(&format!(
            "\n[feature.{}_course]\nkind = \"{COURSE}\"\n",
            c.fact
        ));
        let pts: Vec<String> = c.points.iter().map(|&p| lattice_point(p)).collect();
        d.push_str(&format!("path = [{}]\n", pts.join(", ")));
        // The channel drawn: the course's width, held to three post spacings so the water field
        // holds at least three wet posts across it (a narrower channel draws as a thread). The
        // width stays the fact the lint checks.
        let channel = num::max(c.width, CHANNEL_POSTS * num::metres(spacing_mm));
        d.push_str(&format!(
            "fact = \"{}\"\nends = \"{}\"\ncatchment = {}\nwidth = {}\ndepth = {}\nchannel = {}\n",
            c.fact,
            c.end.word(),
            decimal(c.catchment, 4),
            decimal(c.width, 2),
            decimal(c.depth, 2),
            decimal(channel, 2)
        ));
    }
    (d, files)
}

/// Writes the field files of a document into `out`: f32 `(ny, nx)`, no `-0.0`.
pub fn write_fields(out: &Path, posts: Posts, files: &[(&str, &[f64])]) -> Result<(), String> {
    for (name, values) in files {
        let v: Vec<f32> = values
            .iter()
            .map(|&x| num::store_f32(x).map_err(|_| format!("a value of {name} is not finite")))
            .collect::<Result<_, _>>()?;
        std::fs::write(
            out.join(format!("{name}.npy")),
            npy::f32_file(&[posts.ny, posts.nx], &v),
        )
        .map_err(|e| format!("cannot write {name}.npy: {e}"))?;
    }
    Ok(())
}

/// Findings as `diagnostics.json` (5), canonical JSON, `explain` on the first of each code.
pub fn diagnostics_json(findings: &[Finding]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let rows: Vec<Value> = findings
        .iter()
        .map(|f| {
            let mut o = json!({
                "severity": f.severity,
                "code": f.code,
                "subject": f.subject,
                "message": f.message,
                "hint": f.hint,
                "data": serde_json::from_str::<Value>(&f.data).unwrap_or(json!({})),
            });
            if let Some(p) = f.position {
                o["position"] = json!([p.0, p.1]);
            }
            if seen.insert(f.code) {
                o["explain"] = json!(facts::explain(f.code));
            }
            o
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

/// The generate step over what it read.
pub fn generate(
    posts: Posts,
    ground: Vec<f64>,
    facts_in: &[Feature],
    params_v: &Value,
    seed: u64,
    targets: &Targets,
    out: &Path,
) -> Result<(String, Vec<Finding>), Fault> {
    let input = Input {
        posts,
        ground,
        lakes: lakes(facts_in)?,
        rivers: rivers(facts_in)?,
        holds: holds(facts_in),
        params: params(params_v)?,
        seed,
    };
    let settled = settle::settle(&input).map_err(|m| {
        said(
            "params",
            "step",
            m,
            "change the step's `params`, or remove the one named".into(),
        )
    })?;
    let (doc, files) = document(&settled, &input.ground, targets, posts.res);
    write_fields(out, posts, &files)?;
    Ok((doc, grid_locked(&settled)))
}

/// The share of the model's channels above which they read as locked to its grid.
pub const GRID_LOCKED: f64 = 0.25;

/// `grid_locked` (warning): the model's channels run on along one grid direction, a row, a column
/// or a diagonal, for more than `gates::AXIS_RUN_CELLS` cells over more than `GRID_LOCKED` of
/// their length, so the gullies and the wet ground painted from them would read as ruler-straight
/// trenches.
fn grid_locked(s: &Settled) -> Vec<Finding> {
    if s.axis_runs <= GRID_LOCKED {
        return Vec::new();
    }
    vec![Finding {
        severity: "warning",
        code: "grid_locked",
        subject: "world".into(),
        message: format!(
            "{}% of the model's channels run straight along its grid for more than {} cells",
            decimal(s.axis_runs * 100.0, 1),
            crate::gates::AXIS_RUN_CELLS
        ),
        hint: format!(
            "a finer `cell`, or more `steps`, lets the channels follow the land; at most {}% is accepted",
            decimal(GRID_LOCKED * 100.0, 0)
        ),
        position: None,
        data: format!(
            "{{\"axis_runs\": {}, \"run_cells\": {}, \"would_accept\": {}}}",
            decimal(s.axis_runs, 4),
            crate::gates::AXIS_RUN_CELLS,
            decimal(GRID_LOCKED, 2)
        ),
    }]
}

/// What a request names on disk.
pub struct Paths {
    pub out: PathBuf,
    pub objects: PathBuf,
}

pub fn paths(request: &Value) -> Result<Paths, String> {
    let get = |k: &str| {
        request[k]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| format!("the request has no `{k}`"))
    };
    Ok(Paths {
        out: get("out")?,
        objects: get("objects")?,
    })
}

/// The stated facts a lint checks, by name, from the request's `checks` (`derived.md` 5); `None`
/// when the lint names no step, and then it checks every terrain fact in the build.
pub fn checked(request: &Value) -> Option<(Vec<String>, Vec<String>)> {
    let list = request.get("checks")?.as_array()?;
    let mut steps = Vec::new();
    let mut facts = Vec::new();
    for c in list {
        if let Some(s) = c["step"].as_str() {
            steps.push(s.to_string());
        }
        for f in c["facts"].as_array().into_iter().flatten() {
            if let Some(f) = f.as_str() {
                facts.push(f.to_string());
            }
        }
    }
    facts.sort();
    facts.dedup();
    Some((steps, facts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_from_integers() {
        assert_eq!(decimal(0.12346, 4), "0.1235");
        assert_eq!(decimal(9.0, 2), "9.00");
        assert_eq!(decimal(-1.005, 2), "-1.00");
    }

    #[test]
    fn params_refuse_what_they_cannot_read() {
        assert!(params(&json!({})).is_ok());
        assert!(matches!(
            params(&json!({"edge": "shut"})),
            Err(Fault::Said(_))
        ));
        assert!(matches!(
            params(&json!({"margin": 2.5})),
            Err(Fault::Said(_))
        ));
        let p = params(&json!({"cell": 12.5, "edge": "open", "sea": -1})).unwrap();
        assert_eq!((p.cell, p.edge_held, p.sea), (Some(12.5), false, -1.0));
    }

    /// `protocol.md` 4.3's vector: a point, a path, a region with a hole, a region in two pieces.
    #[test]
    fn feature_list_vector() {
        let dir = std::env::temp_dir().join(format!("via-terrain-fl-{}", std::process::id()));
        let store = Store::open(&dir).unwrap();
        let coords: Vec<i64> = vec![
            0, 0, 0, 0, 1000, 0, 0, 0, 4000, 0, 4000, 4000, 0, 4000, 1000, 1000, 1000, 3000, 3000,
            3000, 3000, 1000, 10000, 0, 11000, 0, 10000, 1000, 20000, 0, 21000, 0, 20000, 1000,
        ];
        let list = json!({
            "coords": {"dtype": "i32", "shape": [17, 2], "values": coords},
            "geometry": {"dtype": "u8", "values": [0, 1, 2, 2]},
            "pieces": {"dtype": "i64", "values": [0, 1, 2, 3, 5]},
            "rings": {"dtype": "i64", "values": [0, 1, 2, 4, 5, 6]},
            "ring_starts": {"dtype": "i64", "values": [0, 1, 3, 7, 11, 14, 17]},
            "name": {"dtype": "str", "values": ["a", "b", "c", "d"]},
            "kind": {"dtype": "str", "values": ["", "", "terrain_lake", ""]},
            "authority": {"dtype": "str", "values": ["", "", "anchored", ""]},
            "level": {"dtype": "f64", "values": [0.0, 0.0, 72.5, 0.0]},
            "level?": {"dtype": "u8", "values": [0, 0, 1, 0]},
        });
        let fs = features(&list, &store).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(fs.len(), 4);
        assert_eq!(fs[0].rings, vec![vec![(0, 0)]]);
        assert_eq!(fs[2].rings.len(), 2);
        assert_eq!(fs[3].rings.len(), 2);
        assert_eq!(fs[2].num("level"), Some(72.5));
        assert_eq!(fs[0].num("level"), None);
        let l = lakes(&fs).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].level, 72.5);
    }
}
