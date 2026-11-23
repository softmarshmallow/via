//! What a step sees of the build, and the engine's answers to a worker's calls (`protocol.md`
//! 1 and 3.3, `derived.md` 4).
//!
//! One function answers a call live and again when a trace is replayed for reuse (`graph.md`
//! 5.1), so a recorded answer and a fresh one are made the same way. A call's request is
//! normalised before it is recorded ([`normalise`]) and its answer is digested without machine
//! paths ([`answer_digest`]), so a trace is the same in every folder and on every machine.
//!
//! **The view.** A lint or export step reads the finished build (view `build`). A generate step
//! reads its stratum: what its layer and the layers below state. Features, instances and fields
//! declared above its layer are left out of every selector and refused by name
//! (`graph.read_outside`). Field values are the finished build's, except where a statement above
//! the step's layer changes a field the step reads: then a scalar field's full view is the
//! model's stratum view (`authority.md` 4.4, the build's order filtered to the layer and below),
//! and a read the model cannot give in the view (a weights field) is refused with
//! `doc.not_yet`, never answered with the wrong values.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};
use via_diag::{codes, Diag, Site, Subject};
use via_doc::World;
use via_graph::Digest;
use via_model::Build;
use via_num::Grid;
use via_proto::column::{self, Column, Shaped};
use via_proto::store::Store as Objects;

use crate::steps::StepDecl;
use crate::txt;

/// A refused call: the wire error (3.4), and the diagnostic the engine reports itself, if any
/// (`graph.read_outside`, `protocol.input_undeclared`, `doc.not_yet`).
#[derive(Debug)]
pub struct Refusal {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
    pub diag: Option<Diag>,
}

fn refuse(code: i64, message: impl Into<String>) -> Refusal {
    Refusal {
        code,
        message: message.into(),
        data: None,
        diag: None,
    }
}

/// The columns a feature list always has (`protocol.md` 3.3, 4.3); `class` joins them when the
/// list holds a maker's output.
const CORE_COLUMNS: [&str; 9] = [
    "name",
    "kind",
    "authority",
    "layer",
    "coords",
    "geometry",
    "pieces",
    "rings",
    "ring_starts",
];

fn invalid(message: impl Into<String>) -> Refusal {
    refuse(-32602, message)
}

/// What failed or was skipped in this build, as a traced lint's reads meet it (`diagnostics.md`
/// 4, M2): a read that reaches any of it cancels the call and skips the lint.
#[derive(Clone, Debug, Default)]
pub struct Bad {
    pub fields: BTreeSet<String>,
    pub features: BTreeSet<String>,
    /// Steps whose returned features (`<step>#...`) are not known.
    pub steps: BTreeSet<String>,
    /// Whether any instance's placement failed or was skipped.
    pub instances: bool,
}

impl Bad {
    /// The entities of the nodes that failed or were skipped (`graph.md` 1, node ids).
    pub fn of(nodes: &BTreeSet<via_graph::NodeId>) -> Bad {
        let mut b = Bad::default();
        for n in nodes {
            let s = n.as_str();
            let subject = s.split_once(':').map_or("", |(_, r)| r);
            let subject = subject
                .split("/v:")
                .next()
                .unwrap_or(subject)
                .split("/w:")
                .next()
                .unwrap_or(subject)
                .split("/i:")
                .next()
                .unwrap_or(subject)
                .to_string();
            match n.family() {
                "field" => {
                    b.fields.insert(subject);
                }
                "shape" | "attr" | "distance" | "measure" | "made" => {
                    b.features.insert(subject);
                }
                "instance" | "held" | "height" | "footprint" | "edit" | "rule" => {
                    b.instances = true;
                }
                "step" => {
                    b.steps.insert(subject);
                }
                _ => {}
            }
        }
        b
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
            && self.features.is_empty()
            && self.steps.is_empty()
            && !self.instances
    }
}

/// The refusal of a read that reaches a failed or skipped node.
pub const BAD_READ: i64 = -32099;

/// What a step reads.
pub struct View<'a> {
    pub world: &'a World,
    pub build: &'a Build,
    pub grid: Grid,
    pub objects: &'a Objects,
    pub pack: Option<&'a via_model::pack::Pack>,
    pub step: &'a StepDecl,
    /// The step's layer rank for a generate step; `None` for the finished build.
    pub rank: Option<i64>,
    /// The world seed (`numbers.md` 7.1).
    pub seed: u64,
    /// The model, which gives a generate step's head view of its stratum.
    pub stratum: Option<&'a via_model::ModelPlan<'a>>,
    /// For a traced lint on a build with a failure: what failed or was skipped.
    pub bad: Option<&'a Bad>,
}

/// The methods whose calls are traced (3.3): reads, and computes, which read.
pub fn traced(method: &str) -> bool {
    matches!(
        method,
        "build.field"
            | "build.features"
            | "build.instances"
            | "input.open"
            | "pack.query"
            | "expr.eval"
            | "derive.where"
            | "derive.route"
            | "derive.descend"
            | "derive.spill"
            | "surface.meetings"
            | "surface.surfaces_at"
            | "surface.sample"
    )
}

fn sorted_strings(v: &Value) -> Value {
    match v {
        Value::Array(a) => {
            let mut s: Vec<Value> = a.clone();
            s.sort_by(|x, y| {
                x.as_str()
                    .unwrap_or("")
                    .as_bytes()
                    .cmp(y.as_str().unwrap_or("").as_bytes())
            });
            s.dedup();
            Value::Array(s)
        }
        x => x.clone(),
    }
}

/// A window as `[i0, j0, i1, j1]`, from an object or a list; `None` when absent.
fn window_of(v: Option<&Value>) -> Result<Option<[i64; 4]>, Refusal> {
    let Some(v) = v else { return Ok(None) };
    if v.is_null() {
        return Ok(None);
    }
    let get = |k: &str, i: usize| -> Option<i64> {
        v.get(k)
            .and_then(Value::as_i64)
            .or_else(|| v.get(i).and_then(Value::as_i64))
    };
    match (get("i0", 0), get("j0", 1), get("i1", 2), get("j1", 3)) {
        (Some(a), Some(b), Some(c), Some(d)) => Ok(Some([a, b, c, d])),
        _ => Err(invalid("window: give i0, j0, i1, j1")),
    }
}

/// The request as a trace records it (4.4): `call` left out, the method in, names and columns
/// sorted, the window clipped to the grid as a list, defaults written.
pub fn normalise(method: &str, params: &Value, grid: &Grid) -> Value {
    let mut m: Map<String, Value> = params.as_object().cloned().unwrap_or_default();
    m.remove("call");
    m.insert("method".into(), json!(method));
    for k in ["fields", "columns"] {
        if let Some(v) = m.get(k) {
            let s = sorted_strings(v);
            m.insert(k.into(), s);
        }
    }
    if method == "build.field" {
        m.entry("view").or_insert(json!("full"));
        // The window as the answer resolves it (`field`): the halo added, then clipped to the
        // grid, written with halo 0, so a replay answers the very posts the live call did. A
        // malformed window is kept as written, so the replay refuses it as the live call did.
        if let Ok(c) = resolved_window(&Value::Object(m.clone()), grid) {
            m.insert("window".into(), json!(c));
            m.insert("halo".into(), json!(0));
        } else {
            m.entry("halo").or_insert(json!(0));
        }
    }
    if method.starts_with("derive.") || method == "expr.eval" {
        m.entry("view").or_insert(json!("full"));
    }
    Value::Object(m)
}

/// The posts a `build.field` request reads: its window (the whole grid when absent) grown by its
/// halo, then clipped to the grid. The one definition the answer and the trace share.
fn resolved_window(p: &Value, grid: &Grid) -> Result<[i64; 4], Refusal> {
    let halo = p.get("halo").and_then(Value::as_i64).unwrap_or(0).max(0);
    let w = window_of(p.get("window"))?.unwrap_or([0, 0, grid.nx as i64, grid.ny as i64]);
    Ok(clip(
        [w[0] - halo, w[1] - halo, w[2] + halo, w[3] + halo],
        grid,
    ))
}

fn clip(w: [i64; 4], grid: &Grid) -> [i64; 4] {
    let (nx, ny) = (grid.nx as i64, grid.ny as i64);
    [
        w[0].clamp(0, nx),
        w[1].clamp(0, ny),
        w[2].clamp(0, nx),
        w[3].clamp(0, ny),
    ]
}

/// Removes every `path` beside a `blob`: where the store lies is no part of an answer.
pub fn without_paths(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let blob = m.contains_key("blob");
            Value::Object(
                m.iter()
                    .filter(|(k, _)| !(blob && k.as_str() == "path"))
                    .map(|(k, x)| (k.clone(), without_paths(x)))
                    .collect(),
            )
        }
        Value::Array(a) => Value::Array(a.iter().map(without_paths).collect()),
        x => x.clone(),
    }
}

/// The digest of a canonical JSON value (members sorted, as serde_json's map keeps them).
pub fn digest_of(v: &Value) -> Digest {
    Digest::of(&serde_json::to_vec(v).unwrap_or_default())
}

/// The digest an answer is recorded by: its canonical JSON without machine paths.
pub fn answer_digest(v: &Value) -> Digest {
    digest_of(&without_paths(v))
}

/// Rings of millimetre points.
type Rings = Vec<Vec<(i64, i64)>>;

/// A feature row of a feature list (4.3).
#[derive(Clone, Debug)]
pub struct Row {
    pub name: String,
    pub kind: String,
    pub authority: String,
    pub layer: String,
    pub rank: Option<i64>,
    pub geometry: u8,
    /// Pieces, each its rings, each its vertices.
    pub pieces: Vec<Vec<Vec<(i64, i64)>>>,
    pub attrs: BTreeMap<String, String>,
    pub types: BTreeMap<String, String>,
}

impl Row {
    fn vertices(&self) -> impl Iterator<Item = &(i64, i64)> {
        self.pieces.iter().flatten().flatten()
    }
}

fn tier_word(t: Option<via_model::Tier>) -> &'static str {
    match t {
        Some(via_model::Tier::Derived) => "derived",
        Some(via_model::Tier::Shaped) => "shaped",
        Some(via_model::Tier::Anchored) => "anchored",
        None => "",
    }
}

impl<'a> View<'a> {
    /// The rank of a name of the world namespace, `None` when the world does not declare it (a
    /// produced thing's rank is its producer's).
    fn rank_of(&self, top: &str, name: &str) -> Option<i64> {
        rank_of(self.world, top, name)
    }

    fn layer_of(&self, top: &str, name: &str) -> String {
        self.world
            .home(top, name)
            .map(str::to_string)
            .or_else(|| {
                let head = name.split(['.', '#', '/']).next().unwrap_or(name);
                self.world
                    .find(head)
                    .and_then(|(t, _)| self.world.home(&t, head).map(str::to_string))
            })
            .unwrap_or_default()
    }

    /// Whether a thing of this rank is in the view.
    fn visible(&self, rank: Option<i64>) -> bool {
        match (self.rank, rank) {
            (Some(own), Some(r)) => r <= own,
            _ => true,
        }
    }

    /// The layer of least rank above `rank`: where a step would need to stand to read a thing of
    /// that rank's layer's returned output.
    fn layer_above(&self, rank: i64) -> Option<String> {
        let mut best: Option<(i64, String)> = None;
        for id in self.world.ids("layer") {
            if let Some(r) = self.world.layer_rank(id) {
                if r > rank && best.as_ref().is_none_or(|(b, _)| r < *b) {
                    best = Some((r, id.to_string()));
                }
            }
        }
        best.map(|(_, l)| l)
    }

    /// `graph.read_outside` for a name beyond the stratum (3.3): error -32003 on the wire, and
    /// the one diagnostic of the fault, which the engine reports itself.
    fn outside(&self, subject: &str, layer: &str) -> Refusal {
        let d = Diag::new(
            codes::graph::READ_OUTSIDE,
            Subject::entity("step", self.step.id.clone()),
            Site::Header,
            txt(format!(
                "step `{}` read `{subject}`, which lies outside its stratum (layer `{layer}`)",
                self.step.id
            )),
            txt(format!(
                "declare the step in layer `{layer}` or above, or read only what lies at or below its own layer"
            )),
        );
        Refusal {
            code: -32003,
            message: format!("`{subject}` lies outside the step's stratum"),
            data: Some(json!({"code": "graph.read_outside", "subject": subject, "layer": layer})),
            diag: Some(d),
        }
    }

    /// A column the step sent that cannot be read: its wire error, and for a blob whose size does
    /// not match its `dtype` and `shape`, the one diagnostic of the fault, `protocol.blob_header`
    /// (4.2), which the engine reports itself.
    fn column_fault(&self, e: column::ColumnError) -> Refusal {
        let diag = e.header.then(|| {
            Diag::new(
                codes::protocol::BLOB_HEADER,
                Subject::entity("step", self.step.id.clone()),
                Site::Header,
                txt(format!(
                    "step `{}` sent a column that does not match its header: {}",
                    self.step.id, e.message
                )),
                txt("send the column through the SDK's column writer".into()),
            )
        });
        Refusal {
            diag,
            ..refuse(e.code, e.message)
        }
    }

    fn not_yet(&self, what: String) -> Refusal {
        let d = Diag::new(
            codes::doc::NOT_YET,
            Subject::entity("step", self.step.id.clone()),
            Site::Header,
            txt(format!("step `{}`: {what}", self.step.id)),
            txt("read the field in its full view, or move the step to the top layer, until the model gives a stratum's own view".into()),
        );
        Refusal {
            code: -32602,
            message: what,
            data: Some(json!({"code": "doc.not_yet"})),
            diag: Some(d),
        }
    }

    // ---- features --------------------------------------------------------------------------

    /// Every feature of the view, footprint features included, by name.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (n, f) in &self.build.features {
            let mut attrs = f.attrs.clone();
            let mut types = f.attr_types.clone();
            for (a, (place, _)) in &f.names {
                attrs.insert(a.clone(), place.clone());
                types.insert(a.clone(), "text".into());
            }
            let pieces = if f.shape == "region" {
                if f.rings.is_empty() {
                    vec![vec![crate::out::canonical_ring(&f.points, true)]]
                } else {
                    f.rings.clone()
                }
            } else {
                vec![vec![f.points.clone()]]
            };
            rows.push(Row {
                name: n.clone(),
                kind: f.kind.clone().unwrap_or_default(),
                authority: tier_word(f.tier).into(),
                layer: self.layer_of("feature", n),
                rank: view_rank(self.world, "feature", n),
                geometry: match f.shape.as_str() {
                    "point" => 0,
                    "path" => 1,
                    _ => 2,
                },
                pieces,
                attrs,
                types,
            });
        }
        for (inst, f) in &self.build.footprints {
            rows.push(Row {
                name: format!("{inst}.ground"),
                kind: f.kind.clone(),
                authority: String::new(),
                layer: self.layer_of("instance", inst),
                rank: self.rank_of("instance", inst),
                geometry: 2,
                pieces: vec![vec![crate::out::canonical_ring(&f.ring, true)]],
                attrs: BTreeMap::new(),
                types: BTreeMap::new(),
            });
        }
        rows.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        rows
    }

    /// The rows of the view: the build's, less what lies above a generate step's layer.
    pub fn visible_rows(&self) -> Vec<Row> {
        self.rows()
            .into_iter()
            .filter(|r| self.visible(r.rank))
            .collect()
    }

    /// Resolves a feature selection (3.3): `"*"`, `kind:<k>`, a name, or a list of names.
    fn select_rows(&self, select: &Value) -> Result<Vec<Row>, Refusal> {
        let all = self.rows();
        let by_name = |n: &str| -> Result<Row, Refusal> {
            if let Some((producer, _)) = n.split_once('#') {
                // A returned feature: outside the stratum of every step of its layer and below.
                let pr = self.world.rank("step", producer);
                if let (Some(own), Some(pr)) = (self.rank, pr) {
                    if pr >= own {
                        let layer = self.layer_above(pr).unwrap_or_else(|| {
                            self.world.home("step", producer).unwrap_or("").to_string()
                        });
                        return Err(self.outside(&format!("feature.{n}"), &layer));
                    }
                }
            }
            match all.iter().find(|r| r.name == n) {
                Some(r) if self.visible(r.rank) => Ok(r.clone()),
                Some(r) => Err(self.outside(&format!("feature.{n}"), &r.layer)),
                None => Err(invalid(format!("no feature is named `{n}`"))),
            }
        };
        // A network or cells table (`geometry.md` 8.3): what it made, as region features.
        let maker_rows = |n: &str| -> Option<Result<Vec<Row>, Refusal>> {
            let top = ["network", "cells"]
                .into_iter()
                .find(|t| self.world.entity(t, n).is_some())?;
            let r = self.world.rank(top, n);
            if !self.visible(r) {
                let layer = self.world.home(top, n).unwrap_or("").to_string();
                return Some(Err(self.outside(&format!("{top}.{n}"), &layer)));
            }
            let prefix = format!("{n}#");
            Some(Ok(all
                .iter()
                .filter(|row| row.name.starts_with(&prefix))
                .cloned()
                .collect()))
        };
        match select {
            Value::String(s) if !s.contains(':') && s != "*" => {
                if let Some(rows) = maker_rows(s) {
                    return rows;
                }
            }
            Value::Array(a)
                if a.iter()
                    .any(|v| v.as_str().is_some_and(|n| maker_rows(n).is_some())) =>
            {
                let mut out = Vec::new();
                let mut names: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
                if names.len() != a.len() {
                    return Err(invalid("select: a list of names"));
                }
                names.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                names.dedup();
                for n in names {
                    match maker_rows(n) {
                        Some(rows) => out.extend(rows?),
                        None => out.push(by_name(n)?),
                    }
                }
                out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
                out.dedup_by(|a, b| a.name == b.name);
                return Ok(out);
            }
            _ => {}
        }
        match select {
            Value::String(s) if s == "*" => {
                Ok(all.into_iter().filter(|r| self.visible(r.rank)).collect())
            }
            Value::String(s) if s.starts_with("kind:") => {
                let k = &s["kind:".len()..];
                Ok(all
                    .into_iter()
                    .filter(|r| r.kind == k && self.visible(r.rank))
                    .collect())
            }
            Value::String(s) if s.starts_with("given:") => {
                let names = given_names(self.world, self.build, &s["given:".len()..]);
                Ok(all
                    .into_iter()
                    .filter(|r| names.contains(&r.name) && self.visible(r.rank))
                    .collect())
            }
            Value::String(s) if s.contains(':') => Err(invalid(format!(
                "`{s}`: a feature selection is `*`, `kind:<kind>`, `given:<name>`, a name or a list of names"
            ))),
            Value::String(s) => Ok(vec![by_name(s)?]),
            Value::Array(a) => {
                let mut names: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
                if names.len() != a.len() {
                    return Err(invalid("select: a list of names"));
                }
                names.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                names.dedup();
                names.into_iter().map(by_name).collect()
            }
            _ => Err(invalid(
                "select: `*`, `kind:<kind>`, a name or a list of names",
            )),
        }
    }

    /// A region to filter by: a region feature's name, or a ring of `[x, y]` millimetres.
    fn within(&self, v: Option<&Value>) -> Result<Option<Rings>, Refusal> {
        let Some(v) = v else { return Ok(None) };
        match v {
            Value::Null => Ok(None),
            Value::String(n) => {
                let r = self
                    .select_rows(&json!(n))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| invalid(format!("no feature is named `{n}`")))?;
                if r.geometry != 2 {
                    return Err(invalid(format!("`within`: `{n}` is not a region")));
                }
                Ok(Some(r.pieces.iter().flatten().cloned().collect()))
            }
            Value::Array(a) => {
                let ring: Option<Vec<(i64, i64)>> = a
                    .iter()
                    .map(|p| Some((p.get(0)?.as_i64()?, p.get(1)?.as_i64()?)))
                    .collect();
                ring.map(|r| Some(vec![r]))
                    .ok_or_else(|| invalid("`within`: a list of [x, y] in millimetres"))
            }
            _ => Err(invalid("`within`: a region's name or a ring")),
        }
    }

    /// A network's or cells table's row carries its class: the class letter its key starts with
    /// (`identity.md` 2.3). `None` for a plain feature.
    fn made_class(&self, r: &Row) -> Option<String> {
        let (maker, key) = r.name.split_once('#')?;
        if self.world.entity("network", maker).is_none()
            && self.world.entity("cells", maker).is_none()
        {
            return None;
        }
        Some(
            match key.as_bytes().first() {
                Some(b'a') => "area",
                Some(b'c') => "cell",
                Some(b'b') => "rest",
                _ => "",
            }
            .to_string(),
        )
    }

    /// A feature list (4.3) of these rows, with these attribute columns.
    fn feature_list(&self, rows: &[Row], columns: &Value) -> Result<Value, Refusal> {
        // The core's own columns (protocol.md 3.3): an attribute of the same name is never a
        // column, so it cannot overwrite them. Named in `columns`, or under `*`, the name gives
        // the core column, which is always there. `class` is a core column when any row is a
        // maker's output.
        let made_any = rows.iter().any(|r| self.made_class(r).is_some());
        let reserved =
            |a: &str| CORE_COLUMNS.contains(&a) || (made_any && a == "class") || a.ends_with('?');
        let attrs: Vec<String> = match columns {
            Value::String(s) if s == "*" => {
                let set: BTreeSet<&String> = rows
                    .iter()
                    .flat_map(|r| r.attrs.keys())
                    .filter(|a| !reserved(a))
                    .collect();
                set.into_iter().cloned().collect()
            }
            Value::Array(a) => {
                let mut v: Vec<String> = a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                v.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                v.dedup();
                v.retain(|a| !reserved(a));
                v
            }
            Value::Null => Vec::new(),
            _ => return Err(invalid("columns: attribute names, or `*`")),
        };
        let mut coords: Vec<i32> = Vec::new();
        let (mut pieces, mut rings, mut starts) = (vec![0i64], vec![0i64], vec![0i64]);
        for r in rows {
            for pc in &r.pieces {
                for ring in pc {
                    for (x, y) in ring {
                        coords.push(i32::try_from(*x).unwrap_or(0));
                        coords.push(i32::try_from(*y).unwrap_or(0));
                    }
                    starts.push((coords.len() / 2) as i64);
                }
                rings.push((starts.len() - 1) as i64);
            }
            pieces.push((rings.len() - 1) as i64);
        }
        let mut m = Map::new();
        let put = |col: Shaped| -> Result<Value, Refusal> {
            column::write_shaped(&col, self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        let v = coords.len() / 2;
        m.insert(
            "coords".into(),
            put(Shaped {
                shape: vec![v, 2],
                column: Column::I32(coords),
            })?,
        );
        m.insert(
            "geometry".into(),
            put(Shaped::flat(Column::U8(
                rows.iter().map(|r| r.geometry).collect(),
            )))?,
        );
        m.insert("pieces".into(), put(Shaped::flat(Column::I64(pieces)))?);
        m.insert("rings".into(), put(Shaped::flat(Column::I64(rings)))?);
        m.insert(
            "ring_starts".into(),
            put(Shaped::flat(Column::I64(starts)))?,
        );
        // A network's or cells table's output carries its class (`geometry.md` 8.3); in a list
        // that mixes made things with plain features, a plain feature's class is "".
        if made_any {
            m.insert(
                "class".into(),
                put(Shaped::flat(Column::Str(
                    rows.iter()
                        .map(|r| self.made_class(r).unwrap_or_default())
                        .collect(),
                )))?,
            );
        }
        for (k, get) in [
            ("name", (|r: &Row| r.name.clone()) as fn(&Row) -> String),
            ("kind", |r: &Row| r.kind.clone()),
            ("authority", |r: &Row| r.authority.clone()),
            ("layer", |r: &Row| r.layer.clone()),
        ] {
            m.insert(
                k.into(),
                put(Shaped::flat(Column::Str(rows.iter().map(get).collect())))?,
            );
        }
        for a in &attrs {
            let mut ty: Option<(String, String)> = None;
            for r in rows {
                if let Some(v) = r.attrs.get(a) {
                    let t = r.types.get(a).cloned().unwrap_or_else(|| {
                        if v == "true" || v == "false" {
                            "bool".into()
                        } else if v.parse::<f64>().is_ok_and(f64::is_finite) {
                            "number".into()
                        } else {
                            "text".into()
                        }
                    });
                    match &ty {
                        Some((k0, t0)) if *t0 != t => {
                            let d = Diag::new(
                                codes::protocol::COLUMN_MIXED,
                                Subject::entity("step", self.step.id.clone()),
                                Site::Header,
                                txt(format!(
                                    "step `{}`: the attribute `{a}` is {t0} on kind `{k0}` and {t} on kind `{}`, so it cannot be one column",
                                    self.step.id, r.kind
                                )),
                                txt(format!(
                                    "give `{a}` one type in both kinds, or read each kind by its own `kind:` selector"
                                )),
                            )
                            .with("attribute", a.clone())
                            .with(
                                "kinds",
                                via_diag::Value::texts([k0.clone(), r.kind.clone()]),
                            );
                            return Err(Refusal {
                                code: -32602,
                                message: format!("the attribute `{a}` has two types"),
                                data: Some(json!({"code": "protocol.column_mixed"})),
                                diag: Some(d),
                            });
                        }
                        Some(_) => {}
                        None => ty = Some((r.kind.clone(), t)),
                    }
                }
            }
            let ty = ty.map_or_else(|| "text".to_string(), |t| t.1);
            let vals: Vec<Option<&String>> = rows.iter().map(|r| r.attrs.get(a)).collect();
            let col = match ty.as_str() {
                "bool" => Column::Bool(
                    vals.iter()
                        .map(|v| u8::from(v.is_some_and(|v| v == "true")))
                        .collect(),
                ),
                "number" => Column::F64(
                    vals.iter()
                        .map(|v| v.and_then(|v| v.parse().ok()).unwrap_or(0.0))
                        .collect(),
                ),
                _ => Column::Str(
                    vals.iter()
                        .map(|v| v.cloned().unwrap_or_default())
                        .collect(),
                ),
            };
            m.insert(a.clone(), put(Shaped::flat(col))?);
            m.insert(
                format!("{a}?"),
                put(Shaped::flat(Column::U8(
                    vals.iter().map(|v| u8::from(v.is_some())).collect(),
                )))?,
            );
        }
        Ok(Value::Object(m))
    }

    fn features(&self, p: &Value) -> Result<Value, Refusal> {
        let select = p.get("select").cloned().unwrap_or(json!("*"));
        let mut rows = self.select_rows(&select)?;
        if let Some(region) = self.within(p.get("within"))? {
            rows.retain(|r| meets(&region, r));
        }
        self.feature_list(&rows, p.get("columns").unwrap_or(&Value::Null))
    }

    // ---- instances -------------------------------------------------------------------------

    fn instances(&self, p: &Value) -> Result<Value, Refusal> {
        let select = p.get("select").and_then(Value::as_str).unwrap_or("*");
        let region = self.within(p.get("within"))?;
        let taxon = |asset: &str| -> String {
            self.pack
                .and_then(|pk| pk.asset(asset))
                .and_then(|a| a.taxon.clone())
                .unwrap_or_default()
        };
        let mut rows: Vec<(&String, &via_model::InstanceOut, String, String)> = Vec::new();
        for (n, i) in &self.build.instances {
            let by = match &i.maker {
                Some(p) => p.clone(),
                None if self.build.statements.contains_key(&format!("pin.{n}")) => {
                    format!("pin.{n}")
                }
                None => format!("instance.{n}"),
            };
            let rank = match &i.maker {
                Some(p) => maker_rank(self.world, p),
                None => self.rank_of("instance", n),
            };
            if !self.visible(rank) {
                continue;
            }
            let t = taxon(&i.asset);
            let keep = match select {
                "*" => true,
                s if s.starts_with('@') => {
                    let want = &s[1..];
                    t == want || t.starts_with(&format!("{want}/"))
                }
                s => by == s || i.maker.as_deref() == Some(s),
            };
            if !keep {
                continue;
            }
            if let Some(reg) = &region {
                if !inside(reg, i.origin) {
                    continue;
                }
            }
            rows.push((n, i, by, t));
        }
        let all = [
            "asset", "by", "face", "name", "scale", "taxon", "tint", "x", "y", "z",
        ];
        let want: BTreeSet<&str> = match p.get("columns") {
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
            _ => all.iter().copied().collect(),
        };
        let mut m = Map::new();
        let put = |col: Column| -> Result<Value, Refusal> {
            column::write_shaped(&Shaped::flat(col), self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        for c in all {
            if c != "name" && !want.contains(c) {
                continue;
            }
            let col = match c {
                "name" => Column::Str(rows.iter().map(|r| r.0.clone()).collect()),
                "asset" => Column::Str(rows.iter().map(|r| r.1.asset.clone()).collect()),
                "by" => Column::Str(rows.iter().map(|r| r.2.clone()).collect()),
                "taxon" => Column::Str(rows.iter().map(|r| r.3.clone()).collect()),
                "x" => Column::I32(rows.iter().map(|r| r.1.origin.0 as i32).collect()),
                "y" => Column::I32(rows.iter().map(|r| r.1.origin.1 as i32).collect()),
                "z" => Column::F64(rows.iter().map(|r| r.1.z + 0.0).collect()),
                "face" => Column::I32(
                    rows.iter()
                        .map(|r| r.1.facing.rem_euclid(360_000) as i32)
                        .collect(),
                ),
                "scale" => Column::F64(rows.iter().map(|r| r.1.scale + 0.0).collect()),
                _ => Column::F64(rows.iter().map(|r| r.1.tint + 0.0).collect()),
            };
            m.insert(c.into(), put(col)?);
        }
        Ok(Value::Object(m))
    }

    // ---- surfaces --------------------------------------------------------------------------

    /// `surface.meetings` (`surfaces.md` 2.4): every meeting of the finished build, as a table
    /// ordered by `a`, then `b`. Meetings are checked after grounding, so a generate step, whose
    /// view is a stratum, is refused with `surface.too_early` naming the step (2.6), which the
    /// engine reports itself. Faces, turns and connectors arrive with kind mounts (M3, M5): until
    /// then each such column is absent on every row, its `<column>?` 0.
    fn meetings(&self) -> Result<Value, Refusal> {
        if self.rank.is_some() || self.step.when == "generate" {
            let d = Diag::new(
                codes::surface::TOO_EARLY,
                Subject::entity("step", self.step.id.clone()),
                Site::Header,
                txt(format!(
                    "step `{}` asked for `meetings()` at `generate`; meetings are checked after grounding",
                    self.step.id
                )),
                txt("ask for meetings from a `lint` or `export` step, which reads the finished build".into()),
            );
            return Err(Refusal {
                code: -32006,
                message: "`meetings()` is answered at `lint` and `export` only".into(),
                data: Some(
                    json!({"code": "surface.too_early", "subject": format!("step.{}", self.step.id)}),
                ),
                diag: Some(d),
            });
        }
        let rows = via_model::query::meetings(self.build);
        let mut m = Map::new();
        let put = |col: Column| -> Result<Value, Refusal> {
            column::write_shaped(&Shaped::flat(col), self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        m.insert(
            "a".into(),
            put(Column::Str(rows.iter().map(|r| r.a.clone()).collect()))?,
        );
        m.insert(
            "b".into(),
            put(Column::Str(rows.iter().map(|r| r.b.clone()).collect()))?,
        );
        m.insert(
            "x".into(),
            put(Column::I64(rows.iter().map(|r| r.at.0).collect()))?,
        );
        m.insert(
            "y".into(),
            put(Column::I64(rows.iter().map(|r| r.at.1).collect()))?,
        );
        m.insert(
            "z_a".into(),
            put(Column::F64(rows.iter().map(|r| r.z_a + 0.0).collect()))?,
        );
        m.insert(
            "z_b".into(),
            put(Column::F64(rows.iter().map(|r| r.z_b + 0.0).collect()))?,
        );
        m.insert(
            "gap".into(),
            put(Column::I64(rows.iter().map(|r| r.gap).collect()))?,
        );
        // M5: the face, turn and connector columns as `query meetings` writes them (protocol.md
        // 3.3), each with its `<column>?` beside it, 0 where the value is null.
        type Int = fn(&via_model::Meeting) -> Option<i64>;
        let ints: [(&str, Int); 3] = [
            ("face_a", |r| r.face_a),
            ("face_b", |r| r.face_b),
            ("turn", |r| r.turn),
        ];
        for (c, get) in ints {
            let v: Vec<Option<i64>> = rows.iter().map(|r| get(r)).collect();
            m.insert(
                c.into(),
                put(Column::I64(v.iter().map(|x| x.unwrap_or(0)).collect()))?,
            );
            m.insert(
                format!("{c}?"),
                put(Column::U8(
                    v.iter().map(|x| u8::from(x.is_some())).collect(),
                ))?,
            );
        }
        type Txt = fn(&via_model::Meeting) -> Option<String>;
        let txts: [(&str, Txt); 2] = [
            ("connector_a", |r| r.connector_a.clone()),
            ("connector_b", |r| r.connector_b.clone()),
        ];
        for (c, get) in txts {
            let v: Vec<Option<String>> = rows.iter().map(|r| get(r)).collect();
            m.insert(
                c.into(),
                put(Column::Str(
                    v.iter().map(|x| x.clone().unwrap_or_default()).collect(),
                ))?,
            );
            m.insert(
                format!("{c}?"),
                put(Column::U8(
                    v.iter().map(|x| u8::from(x.is_some())).collect(),
                ))?,
            );
        }
        Ok(Value::Object(m))
    }

    /// The terrain's field name: the one `role = "surface"` field (`surfaces.md` 1, rule 1).
    pub fn terrain_field(&self) -> Option<String> {
        self.build
            .fields
            .iter()
            .find(|(_, f)| f.role.as_deref() == Some("surface"))
            .map(|(n, _)| n.clone())
    }

    /// The terrain in the step's view (`surfaces.md` 2.6): the one `role = "surface"` field, its
    /// posts as `build.field` gives them in `view`.
    fn terrain_posts(&self, view: &str) -> Result<Option<(String, Vec<f32>)>, Refusal> {
        let Some(name) = self.terrain_field() else {
            return Ok(None);
        };
        let (name, v) = self.field_values(&name, view)?;
        // The values were f32 posts; back to f32 exactly.
        Ok(Some((name, v.iter().map(|x| *x as f32).collect())))
    }

    /// The kind surfaces in the view: an owner above a generate step's layer is absent (2.6).
    /// The one test `surfaces_at` and `sample` share ([`View::out_of_view`]).
    fn kind_surfaces(&self) -> impl Iterator<Item = &via_model::surfaces::SurfaceOut> {
        self.build
            .surfaces
            .values()
            .filter(|s| self.out_of_view(s).is_none())
    }

    /// Why a kind surface is not in the step's view (2.6), as `graph.read_outside`, or `None`
    /// when it is: as a feature read by name (`select_rows`), a returned owner lies outside the
    /// stratum of every step of its producer's layer and below, and an owner above a generate
    /// step's layer is absent.
    fn out_of_view(&self, s: &via_model::surfaces::SurfaceOut) -> Option<Refusal> {
        if let Some((producer, _)) = s.owner.split_once('#') {
            if let (Some(own), Some(pr)) = (self.rank, self.world.rank("step", producer)) {
                if pr >= own {
                    let layer = self.layer_above(pr).unwrap_or_else(|| {
                        self.world.home("step", producer).unwrap_or("").to_string()
                    });
                    return Some(self.outside(&format!("feature.{}", s.owner), &layer));
                }
            }
        }
        if !self.visible(view_rank(self.world, "feature", &s.owner)) {
            let layer = self.layer_of("feature", &s.owner);
            return Some(self.outside(&format!("feature.{}", s.owner), &layer));
        }
        None
    }

    /// A kind surface's `base` that failed at a point asked: the whole call fails with that
    /// diagnostic, which the engine reports itself (`surfaces.md` 2.7).
    fn surface_fault(
        &self,
        s: &via_model::surfaces::SurfaceOut,
        e: &via_expr::ExprError,
        p: (i64, i64),
    ) -> Refusal {
        Refusal {
            code: -32602,
            message: format!("the surface `{}` fails at ({}, {}): {e}", s.place, p.0, p.1),
            data: Some(json!({"code": e.code.as_str(), "subject": format!("feature.{}", s.owner)})),
            diag: Some(s.fault(e)),
        }
    }

    fn points(&self, p: &Value) -> Result<Vec<(i64, i64)>, Refusal> {
        let xs = self.ints(p, "x")?;
        let ys = self.ints(p, "y")?;
        if xs.len() != ys.len() {
            return Err(invalid("x and y: columns of one length"));
        }
        Ok(xs.into_iter().zip(ys).collect())
    }

    /// `surface.surfaces_at` (`surfaces.md` 2.1): every surface in the view at each point, a
    /// table `point` (i64, the row in), `surface` (str), `z` (f64); a point's rows by `z`, then
    /// surface name bytewise; a point outside the extent has none.
    fn surfaces_at(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let pts = self.points(p)?;
        let mut rows: Vec<(usize, String, f64)> = Vec::new();
        if let Some((name, posts)) = self.terrain_posts(view)? {
            if let Some(t) = via_model::query::Terrain::of_values(&name, self.grid, &posts) {
                rows.extend(
                    t.surfaces_at(&pts)
                        .into_iter()
                        .map(|(k, n, z)| (k, n.to_string(), z)),
                );
            }
        }
        let [x0, y0, x1, y1] = self.grid.extent;
        for (k, &(x, y)) in pts.iter().enumerate() {
            if !(x0..=x1).contains(&x) || !(y0..=y1).contains(&y) {
                continue;
            }
            for s in self.kind_surfaces() {
                match s.z((x, y)) {
                    Ok(Some(z)) => rows.push((k, s.place.clone(), z)),
                    Ok(None) => {}
                    Err(e) => return Err(self.surface_fault(s, &e, (x, y))),
                }
            }
        }
        rows.sort_by(|a, c| {
            a.0.cmp(&c.0)
                .then(a.2.total_cmp(&c.2))
                .then(a.1.as_bytes().cmp(c.1.as_bytes()))
        });
        let put = |col: Column| -> Result<Value, Refusal> {
            column::write_shaped(&Shaped::flat(col), self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        let mut m = Map::new();
        m.insert(
            "point".into(),
            put(Column::I64(rows.iter().map(|r| r.0 as i64).collect()))?,
        );
        m.insert(
            "surface".into(),
            put(Column::Str(rows.iter().map(|r| r.1.clone()).collect()))?,
        );
        m.insert(
            "z".into(),
            put(Column::F64(rows.iter().map(|r| r.2 + 0.0).collect()))?,
        );
        Ok(Value::Object(m))
    }

    /// `surface.sample` (`surfaces.md` 2.2): one surface by place name, the terrain's field name
    /// or `<owner>.<surface>`; per point `inside` (bool), `z`, `gx`, `gy` (f64). A name that is
    /// no surface in the view is `surface.not_surface`; an owner above a generate step's layer is
    /// `graph.read_outside`, as a feature read by name is.
    fn sample(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let name = p
            .get("surface")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("surface: one surface's place name"))?;
        let pts = self.points(p)?;
        let mut out: Vec<via_model::query::Sample> = Vec::with_capacity(pts.len());
        let terrain = self.terrain_posts(view)?;
        if let Some((tn, posts)) = terrain.as_ref().filter(|(tn, _)| tn == name) {
            let t = via_model::query::Terrain::of_values(tn, self.grid, posts)
                .ok_or_else(|| invalid(format!("`{name}` does not cover the grid")))?;
            out.extend(pts.iter().map(|&(x, y)| t.sample(x, y)));
        } else if let Some(s) = self.build.surfaces.get(name) {
            if let Some(r) = self.out_of_view(s) {
                return Err(r);
            }
            for &(x, y) in &pts {
                match via_model::query::sample_kind(s, self.grid.resolution, x, y) {
                    Ok(v) => out.push(v),
                    Err(e) => return Err(self.surface_fault(s, &e, (x, y))),
                }
            }
        } else {
            let mut known: Vec<&str> = self.kind_surfaces().map(|s| s.place.as_str()).collect();
            if let Some((tn, _)) = &terrain {
                known.push(tn);
            }
            let near = via_diag::nearest::nearest(name, known.iter().copied());
            let mut d = Diag::new(
                codes::surface::NOT_SURFACE,
                Subject::entity("step", self.step.id.clone()),
                Site::Header,
                txt(format!(
                    "step `{}` asked `sample` of `{name}`, which is no surface in its view",
                    self.step.id
                )),
                txt(near.hint("name the terrain field or a kind surface as `<feature>.<surface>`")),
            );
            if let Some(n) = near.found() {
                d = d.with("nearest", n);
            }
            return Err(Refusal {
                code: -32602,
                message: format!("`{name}` is no surface in the step's view"),
                data: Some(json!({"code": "surface.not_surface", "subject": name})),
                diag: Some(d),
            });
        }
        let put = |col: Column| -> Result<Value, Refusal> {
            column::write_shaped(&Shaped::flat(col), self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        let mut m = Map::new();
        m.insert(
            "inside".into(),
            put(Column::Bool(
                out.iter().map(|s| u8::from(s.inside)).collect(),
            ))?,
        );
        for (c, get) in [
            (
                "z",
                (|s: &via_model::query::Sample| s.z) as fn(&via_model::query::Sample) -> f64,
            ),
            ("gx", |s| s.gx),
            ("gy", |s| s.gy),
        ] {
            m.insert(
                c.into(),
                put(Column::F64(out.iter().map(|s| get(s) + 0.0).collect()))?,
            );
        }
        Ok(Value::Object(m))
    }

    // ---- fields ----------------------------------------------------------------------------

    fn field_inexact(&self, field: &str) -> bool {
        self.rank.is_some_and(|own| {
            field_inexact(self.world, self.build, field, own, true)
                || self.stratum.is_some_and(|m| {
                    m.reads_returned_shape(field, own) || m.brought_above(field, own)
                })
        })
    }

    /// The fields a formula reads, as checked in the view; none when it does not check.
    pub fn formula_fields(&self, formula: &str) -> Vec<String> {
        match self.program(formula) {
            Ok((_, b)) => b
                .targets
                .into_iter()
                .filter_map(|t| match t {
                    via_expr::Target::Field(n) => Some(n),
                    _ => None,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The kinds whose effect writes this field.
    fn effect_kinds(&self, field: &str) -> BTreeSet<&'a str> {
        effect_kinds(self.world, self.build, field)
    }

    /// Whether some feature or footprint of the build has a kind whose effect writes this field:
    /// then its head view differs from its full view.
    fn has_effects(&self, field: &str) -> bool {
        let kinds = self.effect_kinds(field);
        self.build
            .features
            .values()
            .any(|f| f.kind.as_deref().is_some_and(|k| kinds.contains(k)))
            || self
                .build
                .footprints
                .values()
                .any(|f| kinds.contains(f.kind.as_str()))
    }

    /// The fields a selector names in the view: a name, `role:<r>` or `tag:<t>`.
    fn field_names(&self, sel: &str) -> Result<Vec<String>, Refusal> {
        let vis = |n: &str| self.visible(view_rank(self.world, "field", n));
        if let Some(r) = sel.strip_prefix("role:") {
            return Ok(self
                .build
                .fields
                .iter()
                .filter(|(n, f)| f.role.as_deref() == Some(r) && vis(n))
                .map(|(n, _)| n.clone())
                .collect());
        }
        if let Some(t) = sel.strip_prefix("tag:") {
            return Ok(self
                .build
                .fields
                .iter()
                .filter(|(n, f)| f.tags.iter().any(|x| x == t) && vis(n))
                .map(|(n, _)| n.clone())
                .collect());
        }
        if let Some(r) = self.scoped_rank(sel) {
            // `<import>.<field>` (`composition.md` 6.5): a step at a join stands above the import.
            if self.rank.is_some_and(|own| own <= r) {
                let id = sel.split_once('.').map_or(sel, |(i, _)| i);
                let layer = self.world.home("import", id).unwrap_or("world").to_string();
                let mut refusal = self.outside(sel, &layer);
                if let Some(d) = refusal.diag.take() {
                    let hint = match self.layer_above(r) {
                        Some(l) => format!(
                            "declare the step in layer `{l}`, or another ranked above `{layer}`"
                        ),
                        None => format!("declare the step in a layer ranked above `{layer}`"),
                    };
                    refusal.diag = Some(Diag::new(
                        codes::graph::READ_OUTSIDE,
                        d.subject().clone(),
                        Site::Header,
                        d.message().clone(),
                        txt(hint),
                    ));
                }
                return Err(refusal);
            }
            return Ok(vec![sel.to_string()]);
        }
        if self.build.fields.contains_key(sel) {
            if !vis(sel) {
                let layer = self.world.home("field", sel).unwrap_or("").to_string();
                return Err(self.outside(&format!("field.{sel}"), &layer));
            }
            return Ok(vec![sel.to_string()]);
        }
        Ok(Vec::new())
    }

    /// The declaring layer's rank of the import a scoped field `<import>.<field>` names, when
    /// `sel` is one and not a field of this world.
    fn scoped_rank(&self, sel: &str) -> Option<i64> {
        if self.build.fields.contains_key(sel) {
            return None;
        }
        let (id, _) = sel.split_once('.')?;
        self.stratum?.import_rank(id)
    }

    /// An import's field on this world's posts, for a step at a join.
    fn scoped_values(&self, sel: &str) -> Option<Vec<f32>> {
        self.scoped_rank(sel)?;
        self.stratum?.scoped_values(sel)
    }

    /// One field's values in the view, checked: `view` `full` or `head`.
    pub fn field_values(&self, sel: &str, view: &str) -> Result<(String, Vec<f64>), Refusal> {
        let names = self.field_names(sel)?;
        let [name] = names.as_slice() else {
            return Err(invalid(format!(
                "`{sel}` matches {} fields; give a selector that matches one scalar field",
                names.len()
            )));
        };
        if let Some(v) = self.scoped_values(name) {
            return Ok((name.clone(), v.iter().map(|x| f64::from(*x)).collect()));
        }
        let f = &self.build.fields[name];
        let Some(v) = &f.values else {
            return Err(invalid(format!("`{name}` is a weights field")));
        };
        self.view_ok(name, view)?;
        let v = self.scalar_values(name, view).unwrap_or_else(|| v.clone());
        Ok((name.clone(), v.iter().map(|x| f64::from(*x)).collect()))
    }

    /// A scalar field in the head view of this generate step's stratum (`protocol.md` 1): the
    /// base and the rows of its layer and below, without what steps of its layer and above
    /// returned, and no effect. `None` for a lint, a weights field, or no model.
    fn stratum_view(&self, name: &str) -> Option<Vec<f32>> {
        let rank = self.rank?;
        self.stratum?.stratum_values(name, rank)
    }

    /// A field's scalar values as the view gives them: the stratum's head view for a generate
    /// step reading `head`, else the build's.
    fn scalar_values(&self, name: &str, view: &str) -> Option<Vec<f32>> {
        if view == "head" {
            if let Some(v) = self.stratum_view(name) {
                return Some(v);
            }
        }
        if view == "full" && self.field_inexact(name) {
            if let Some(v) = self.stratum_full(name) {
                return Some(v);
            }
        }
        self.build.fields.get(name)?.values.clone()
    }

    /// A scalar field in the full view of this generate step's stratum (`authority.md` 4.4):
    /// the build's order of entries, keeping those of its layer and below, without what steps
    /// of its layer and above returned. `None` for a lint, a weights field, or no model.
    fn stratum_full(&self, name: &str) -> Option<Vec<f32>> {
        let rank = self.rank?;
        self.stratum?.stratum_full_values(name, rank)
    }

    fn view_ok(&self, name: &str, view: &str) -> Result<(), Refusal> {
        match view {
            "full" | "head" => {}
            v => return Err(invalid(format!("view `{v}`: `full` or `head`"))),
        }
        if view == "head" && self.stratum_view(name).is_some() {
            return Ok(());
        }
        if view == "head" && self.has_effects(name) {
            return Err(self.not_yet(format!(
                "the head view of `{name}`, which a kind's effect writes, is owed by the model"
            )));
        }
        if self.field_inexact(name) && (view != "full" || self.stratum_full(name).is_none()) {
            return Err(self.not_yet(format!(
                "`{name}` in the view of layer `{}`, which a statement of a higher layer changes, is owed by the model",
                self.step.layer
            )));
        }
        Ok(())
    }

    fn field(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let w = resolved_window(p, &self.grid)?;
        if !(w[0] < w[2] && w[1] < w[3]) {
            return Err(invalid("window: empty, or outside the grid"));
        }
        let sels: Vec<String> = match p.get("fields") {
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => return Err(invalid("fields: a list of names or selectors")),
        };
        let nx = self.grid.nx as usize;
        let (i0, j0, i1, j1) = (w[0] as usize, w[1] as usize, w[2] as usize, w[3] as usize);
        let mut out = Map::new();
        for sel in sels {
            let names = self.field_names(&sel)?;
            let [name] = names.as_slice() else {
                return Err(invalid(format!(
                    "`{sel}` matches {} fields; a read names one field per entry",
                    names.len()
                )));
            };
            let scoped = self.scoped_values(name);
            if scoped.is_none() {
                self.view_ok(name, view)?;
            }
            let cut = |v: &[f32]| -> Vec<f32> {
                let mut c = Vec::with_capacity((i1 - i0) * (j1 - j0));
                for j in j0..j1 {
                    c.extend_from_slice(&v[j * nx + i0..j * nx + i1]);
                }
                c
            };
            let mut e = Map::new();
            e.insert("name".into(), json!(name));
            let bytes = match (&scoped, self.build.fields.get(name)) {
                (Some(v), _) => {
                    e.insert("type".into(), json!("scalar"));
                    crate::npy::f4(&[j1 - j0, i1 - i0], &cut(v))
                }
                (None, None) => return Err(invalid(format!("`{name}` is no field"))),
                (None, Some(f)) => match &f.values {
                    Some(v) => {
                        e.insert("type".into(), json!("scalar"));
                        let v = self.scalar_values(name, view).unwrap_or_else(|| v.clone());
                        crate::npy::f4(&[j1 - j0, i1 - i0], &cut(&v))
                    }
                    None => {
                        e.insert("type".into(), json!("weights"));
                        let mut mats = Vec::new();
                        let mut all = Vec::new();
                        for (m, v) in &f.weights {
                            let c = cut(v);
                            if c.iter().any(|x| *x != 0.0) {
                                mats.push(m.clone());
                                all.extend(c);
                            }
                        }
                        e.insert("materials".into(), json!(mats));
                        crate::npy::f4(&[mats.len(), j1 - j0, i1 - i0], &all)
                    }
                },
            };
            let blob = self
                .objects
                .put(&bytes)
                .map_err(|e| refuse(-32603, format!("a field file could not be stored: {e}")))?;
            e.insert("blob".into(), json!(blob));
            e.insert(
                "path".into(),
                json!(via_proto::worker::wire_path(&self.objects.path_of(&blob))),
            );
            for (k, v) in [("i0", i0), ("j0", j0), ("i1", i1), ("j1", j1)] {
                e.insert(k.into(), json!(v));
            }
            out.insert(sel, Value::Object(e));
        }
        Ok(Value::Object(out))
    }

    // ---- inputs and the pack ---------------------------------------------------------------

    fn input(&self, p: &Value) -> Result<Value, Refusal> {
        let path = p
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("path: the input as written in `inputs`"))?;
        if !self.step.inputs.iter().any(|i| i == path) {
            let d = Diag::new(
                codes::protocol::INPUT_UNDECLARED,
                Subject::entity("step", self.step.id.clone()),
                Site::Header,
                txt(format!(
                    "step `{}` opened `{path}`, which its `inputs` do not declare",
                    self.step.id
                )),
                txt(format!("add `{path}` to the step's `inputs`")),
            );
            return Err(Refusal {
                code: -32602,
                message: format!("`{path}` is not a declared input"),
                data: Some(json!({"code": "protocol.input_undeclared", "path": path})),
                diag: Some(d),
            });
        }
        let bytes = std::fs::read(self.step.home.join(path))
            .map_err(|e| refuse(-32603, format!("`{path}` could not be read: {e}")))?;
        let blob = self
            .objects
            .put(&bytes)
            .map_err(|e| refuse(-32603, format!("a blob could not be stored: {e}")))?;
        Ok(json!({
            "blob": blob,
            "bytes": bytes.len(),
            "path": via_proto::worker::wire_path(&self.objects.path_of(&blob)),
        }))
    }

    fn pack_query(&self, p: &Value) -> Result<Value, Refusal> {
        let Some(pack) = self.pack else {
            return Err(invalid("the world names no pack"));
        };
        let all = |pk: &'a via_model::pack::Pack| -> Vec<&'a via_model::pack::Asset> {
            pk.assets
                .values()
                .chain(pk.brought.values().map(|b| &b.asset))
                .collect()
        };
        let mut assets: Vec<&via_model::pack::Asset> = match p.get("select") {
            // Every asset of the build, an import's brought ones included (`composition.md`
            // 10); a taxon selects this world's own, as its scatters do.
            None | Some(Value::Null) => all(pack),
            Some(Value::String(s)) if s == "*" => all(pack),
            Some(Value::String(s)) if s.starts_with('@') => pack.select(&s[1..]),
            Some(Value::String(s)) => pack.asset(s).into_iter().collect(),
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|s| pack.asset(s))
                .collect(),
            _ => return Err(invalid("select: `*`, asset ids, or `@<taxon path>`")),
        };
        assets.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
        assets.dedup_by(|a, b| a.id == b.id);
        let facts: Vec<BTreeMap<String, String>> = assets.iter().map(|a| a.facts()).collect();
        let known: BTreeSet<String> = facts
            .iter()
            .flat_map(|f| f.keys().cloned())
            .chain(
                [
                    "size",
                    "bounds",
                    "origin",
                    "colour",
                    "triangles",
                    "sink",
                    "footprint",
                    "plan",
                    "front",
                    "up",
                    "mounts",
                    "collision",
                    "primitive",
                    "params",
                    "shared",
                ]
                .into_iter()
                .map(str::to_string),
            )
            .collect();
        let cols: Vec<String> = match p.get("columns") {
            Some(Value::String(s)) if s == "*" => known.iter().cloned().collect(),
            Some(Value::Array(a)) => {
                let mut v: Vec<String> = a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                v.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                v.dedup();
                for c in &v {
                    if !known.contains(c) {
                        return Err(invalid(format!("`{c}` is not a fact of the pack")));
                    }
                }
                v
            }
            _ => Vec::new(),
        };
        let put = |col: Shaped| -> Result<Value, Refusal> {
            column::write_shaped(&col, self.objects)
                .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
        };
        let mut m = Map::new();
        m.insert(
            "id".into(),
            put(Shaped::flat(Column::Str(
                assets.iter().map(|a| a.id.clone()).collect(),
            )))?,
        );
        m.insert(
            "source".into(),
            put(Shaped::flat(Column::Str(
                assets.iter().map(|a| a.source.clone()).collect(),
            )))?,
        );
        m.insert(
            "taxon".into(),
            put(Shaped::flat(Column::Str(
                assets
                    .iter()
                    .map(|a| a.taxon.clone().unwrap_or_default())
                    .collect(),
            )))?,
        );
        m.insert(
            "tags".into(),
            put(Shaped::flat(Column::Str(
                assets
                    .iter()
                    .map(|a| serde_json::to_string(&a.tags).unwrap_or_default())
                    .collect(),
            )))?,
        );
        let n = assets.len();
        for c in cols {
            let ints = |f: &dyn Fn(&via_model::pack::Asset) -> Vec<i64>, k: usize| Shaped {
                shape: vec![n, k],
                column: Column::I32(
                    assets
                        .iter()
                        .flat_map(|a| f(a))
                        .map(|v| i32::try_from(v).unwrap_or(i32::MAX))
                        .collect(),
                ),
            };
            let col = match c.as_str() {
                "size" => ints(&|a| a.size.to_vec(), 3),
                "bounds" => ints(&|a| a.bounds.to_vec(), 6),
                "origin" => ints(&|a| a.origin.to_vec(), 3),
                "collision" => Shaped::flat(Column::Str(
                    assets.iter().map(|a| a.collision.clone()).collect(),
                )),
                "primitive" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| a.primitive.clone().unwrap_or_default())
                        .collect(),
                )),
                "params" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            a.params
                                .as_ref()
                                .and_then(|t| serde_json::to_value(t).ok())
                                .map_or_else(String::new, |v| v.to_string())
                        })
                        .collect(),
                )),
                "shared" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            a.shared
                                .as_ref()
                                .and_then(|t| serde_json::to_value(t).ok())
                                .map_or_else(String::new, |v| v.to_string())
                        })
                        .collect(),
                )),
                "colour" => Shaped {
                    shape: vec![n, 3],
                    column: Column::F64(assets.iter().flat_map(|a| a.colour).collect()),
                },
                "triangles" => Shaped::flat(Column::I64(
                    assets.iter().map(|a| a.triangles as i64).collect(),
                )),
                "sink" => Shaped::flat(Column::F64(assets.iter().map(|a| a.sink).collect())),
                "footprint" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            serde_json::to_string(
                                &a.footprint.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>(),
                            )
                            .unwrap_or_default()
                        })
                        .collect(),
                )),
                // `assets.md` 2.3: a model's `front` and `up`, the words its pack declares (or
                // their defaults); `""` for a primitive or engine-made asset, which has no model axes.
                "front" | "up" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            a.model.as_ref().map_or_else(String::new, |m| {
                                if c == "front" {
                                    m.front.clone()
                                } else {
                                    m.up.clone()
                                }
                            })
                        })
                        .collect(),
                )),
                // `assets.md` 3.4: rings of `[x, y]` millimetres, `""` where the model is its
                // box (or no model): what a library reads to know what a piece covers.
                "plan" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            if a.plan.is_empty() {
                                return String::new();
                            }
                            serde_json::to_string(
                                &a.plan
                                    .iter()
                                    .map(|r| r.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>())
                                    .collect::<Vec<_>>(),
                            )
                            .unwrap_or_default()
                        })
                        .collect(),
                )),
                "mounts" => Shaped::flat(Column::Str(
                    assets
                        .iter()
                        .map(|a| {
                            let mut o = Map::new();
                            for (k, (at, face)) in &a.mounts {
                                let mut m = json!({"at": [at.0, at.1], "face": face});
                                if let Some(c) = a.connectors.get(k) {
                                    m["connector"] = json!(c);
                                }
                                if let Some(r) = a.rises.get(k) {
                                    m["rise"] = json!(r);
                                }
                                o.insert(k.clone(), m);
                            }
                            Value::Object(o).to_string()
                        })
                        .collect(),
                )),
                other => Shaped::flat(Column::Str(
                    facts
                        .iter()
                        .map(|f| f.get(other).cloned().unwrap_or_default())
                        .collect(),
                )),
            };
            m.insert(c, put(col)?);
        }
        Ok(Value::Object(m))
    }

    // ---- computes --------------------------------------------------------------------------

    /// A formula checked in the context of a `where` (`formulas.md` 6.6): fields, world `let`s,
    /// features by name, `x` and `y`.
    fn program(&self, formula: &str) -> Result<(via_expr::Program, via_expr::Binding), Refusal> {
        let mut env = via_expr::Env::new();
        for n in self.build.fields.keys() {
            env.field(n);
        }
        if let Some(lets) = self.world.root().table("let") {
            for (name, e) in lets.iter() {
                let id = match &e.item.value {
                    via_doc::Value::Str(s) => env.text(&s.text),
                    via_doc::Value::Num(n) => env.number(n.norm.parse().unwrap_or(0.0)),
                    _ => continue,
                };
                env.world_let(name, id);
            }
        }
        for r in self.visible_rows() {
            env.feature(&r.name, (!r.kind.is_empty()).then_some(r.kind.as_str()));
        }
        let text = if formula.trim_start().starts_with('=') {
            formula.to_string()
        } else {
            format!("= {formula}")
        };
        let id = env.text(&text);
        let program = env
            .check(id, &via_expr::Context::new(via_expr::Key::Where))
            .map_err(|e| expr_refusal(&e))?;
        let binding =
            via_expr::bind(&program, &Sel { view: self }).map_err(|e| expr_refusal(&e))?;
        Ok((program, binding))
    }

    fn evaluate(
        &self,
        formula: &str,
        view: &str,
        domain: &via_expr::Domain,
    ) -> Result<Vec<f64>, Refusal> {
        let (program, binding) = self.program(formula)?;
        for t in &binding.targets {
            if let via_expr::Target::Field(n) = t {
                self.view_ok(n, view)?;
            }
        }
        let setup = via_expr::Setup {
            world_seed: self.seed,
            feature: None,
        };
        via_expr::evaluate(
            &program,
            &binding,
            &Reads {
                view: self,
                which: view,
            },
            &setup,
            domain,
        )
        .map_err(|f| expr_refusal(&f.error))
    }

    fn expr_eval(&self, p: &Value) -> Result<Value, Refusal> {
        let formula = p
            .get("formula")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("formula: a formula string"))?;
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let values = if let Some(w) = window_of(p.get("window"))? {
            let c = clip(w, &self.grid);
            if !(c[0] < c[2] && c[1] < c[3]) || c != w {
                return Err(invalid("window: empty, or outside the grid"));
            }
            let window = via_expr::Window {
                i0: c[0] as u64,
                j0: c[1] as u64,
                i1: c[2] as u64,
                j1: c[3] as u64,
            };
            self.evaluate(
                formula,
                view,
                &via_expr::Domain::Window {
                    grid: self.grid,
                    window,
                },
            )?
        } else {
            let xs = self.ints(p, "x")?;
            let ys = self.ints(p, "y")?;
            if xs.len() != ys.len() {
                return Err(invalid("x and y: columns of one length"));
            }
            let pts: Vec<(i64, i64)> = xs.into_iter().zip(ys).collect();
            self.evaluate(formula, view, &via_expr::Domain::Points(&pts))?
        };
        column::write(&Column::F64(values), self.objects)
            .map_err(|e| refuse(-32603, format!("a column could not be stored: {e}")))
    }

    /// The posts an `expr.eval` call's values stand on (`protocol.md` 3.3): its window, or the
    /// window round its points (each point's four posts). The fields its formula names are
    /// traced over this window. `None` when the call names neither, or no point.
    pub fn eval_window(&self, p: &Value) -> Option<[i64; 4]> {
        if let Ok(Some(w)) = window_of(p.get("window")) {
            return Some(clip(w, &self.grid));
        }
        let xs = self.ints(p, "x").ok()?;
        let ys = self.ints(p, "y").ok()?;
        let (x0, x1) = (*xs.iter().min()?, *xs.iter().max()?);
        let (y0, y1) = (*ys.iter().min()?, *ys.iter().max()?);
        let r = self.grid.resolution;
        let (ex, ey) = (self.grid.extent[0], self.grid.extent[1]);
        Some(clip(
            [
                (x0 - ex).div_euclid(r),
                (y0 - ey).div_euclid(r),
                (x1 - ex).div_euclid(r) + 2,
                (y1 - ey).div_euclid(r) + 2,
            ],
            &self.grid,
        ))
    }

    fn ints(&self, p: &Value, name: &str) -> Result<Vec<i64>, Refusal> {
        let v = p
            .get(name)
            .ok_or_else(|| invalid(format!("{name}: a column")))?;
        column::read(name, v, self.objects)
            .map_err(|e| self.column_fault(e))?
            .as_i64()
            .ok_or_else(|| invalid(format!("{name}: an integer column")))
    }

    fn whole(&self) -> via_expr::Domain<'static> {
        via_expr::Domain::Window {
            grid: self.grid,
            window: via_expr::Window::whole(&self.grid),
        }
    }

    fn derive_where(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let formulas = match p.get("formulas") {
            Some(v) => {
                match column::read("formulas", v, self.objects).map_err(|e| self.column_fault(e))? {
                    Column::Str(s) => s,
                    _ => return Err(invalid("formulas: a str column")),
                }
            }
            None => return Err(invalid("formulas: a str column")),
        };
        let holds: Option<Vec<i64>> = match p.get("holds") {
            None | Some(Value::Null) => None,
            Some(_) => Some(self.ints(p, "holds")?),
        };
        if let Some(h) = &holds {
            if h.len() != 2 * formulas.len() {
                return Err(invalid("holds: (n, 2), one point per formula"));
            }
        }
        let mut rows = Vec::new();
        let mut dry = Vec::new();
        for (k, f) in formulas.iter().enumerate() {
            let values = self.evaluate(f, view, &self.whole())?;
            // The one implementation of `where` and `where` with `holds` (`derived.md` 1, R19b).
            let at = holds
                .as_ref()
                .map(|h| via_geom::P::new(h[2 * k], h[2 * k + 1]));
            let found = via_model::derived::where_region(&self.grid, &values, f, at)
                .map_err(|e| invalid(format!("formula {k}: {}", e.message)))?;
            let pieces: Vec<Vec<Vec<(i64, i64)>>> = found
                .region
                .pieces
                .iter()
                .map(|pc| {
                    std::iter::once(&pc.shell)
                        .chain(pc.holes.iter())
                        .map(|r| r.iter().map(|q| (q.x, q.y)).collect())
                        .collect()
                })
                .collect();
            dry.push(u8::from(found.dry));
            rows.push(Row {
                name: format!("{k}"),
                kind: String::new(),
                authority: String::new(),
                layer: String::new(),
                rank: None,
                geometry: 2,
                pieces,
                attrs: BTreeMap::new(),
                types: BTreeMap::new(),
            });
        }
        let mut list = self.found_list(&rows)?;
        list["dry"] = column::write(&Column::Bool(dry), self.objects)
            .map_err(|e| refuse(-32603, e.to_string()))?;
        Ok(list)
    }

    /// A feature list of found shapes: geometry columns only.
    fn found_list(&self, rows: &[Row]) -> Result<Value, Refusal> {
        let mut v = self.feature_list(rows, &Value::Null)?;
        if let Value::Object(m) = &mut v {
            for k in ["name", "kind", "authority", "layer"] {
                m.remove(k);
            }
        }
        Ok(v)
    }

    fn derive_route(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let cost = p
            .get("cost")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("cost: a formula"))?;
        let wp = p
            .get("waypoints")
            .ok_or_else(|| invalid("waypoints: a feature list of paths"))?;
        let ways = read_paths(wp, self.objects)?;
        let values = self.evaluate(cost, view, &self.whole())?;
        let mut rows = Vec::new();
        let mut costs = Vec::new();
        for (k, w) in ways.iter().enumerate() {
            let pts: Vec<via_geom::P> = w.iter().map(|&(x, y)| via_geom::P::new(x, y)).collect();
            // The one implementation of `route` (`derived.md` 1 and 3, R19b).
            let course = via_model::derived::route(&self.grid, &values, &pts)
                .map_err(|e| self.raster_refusal(&e, &format!("waypoints {k}")))?;
            costs.push(course.cost);
            rows.push(path_row(
                k,
                course.points.iter().map(|q| (q.x, q.y)).collect(),
            ));
        }
        let mut list = self.found_list(&rows)?;
        list["cost"] = column::write(&Column::F64(costs), self.objects)
            .map_err(|e| refuse(-32603, e.to_string()))?;
        Ok(list)
    }

    /// A refusal for a fault of the one implementation (`derived.md` 1): `-32602` with the
    /// located diagnostic the text form would give, at the step.
    fn raster_refusal(&self, e: &via_raster::RasterError, what: &str) -> Refusal {
        let code = match e {
            via_raster::RasterError::Internal { .. } => -32603,
            _ => -32602,
        };
        let d = e.diag(Subject::entity("step", self.step.id.clone()), Site::Header);
        Refusal {
            code,
            message: format!("{what}: {e}"),
            data: Some(json!({"code": e.code().to_string()})),
            diag: Some(d),
        }
    }

    /// A field's values as a raster over the whole grid, for the one implementation's batches.
    fn field_raster(&self, values: &[f64]) -> Result<via_raster::Raster<f32>, Refusal> {
        // The values are a stored field's 32-bit values widened; narrowing back is exact.
        #[allow(clippy::cast_possible_truncation)]
        let narrow: Vec<f32> = values.iter().map(|v| *v as f32).collect();
        via_raster::Raster::from_values(via_raster::Window::whole(&self.grid), narrow)
            .ok_or_else(|| refuse(-32603, "a field's values do not match the grid"))
    }

    fn one_field(&self, p: &Value, view: &str) -> Result<(String, Vec<f64>), Refusal> {
        let sel = p
            .get("field")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("field: a name or selector matching one scalar field"))?;
        self.field_values(sel, view)
    }

    fn derive_descend(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let (_, values) = self.one_field(p, view)?;
        let xs = self.ints(p, "x")?;
        let ys = self.ints(p, "y")?;
        if xs.len() != ys.len() {
            return Err(invalid("x and y: columns of one length"));
        }
        let field = self.field_raster(&values)?;
        let pts: Vec<via_geom::P> = xs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| via_geom::P::new(x, y))
            .collect();
        // Steepest descent over the triangulated surface (`derived.md` 4.2), the one
        // implementation.
        let lines = via_model::derived::descend(&self.grid, &field, &pts)
            .map_err(|e| self.raster_refusal(&e, "x, y"))?;
        let mut rows = Vec::new();
        let mut ends = Vec::new();
        for (k, d) in lines.into_iter().enumerate() {
            ends.push(d.end.as_str().to_string());
            rows.push(path_row(k, d.points.iter().map(|q| (q.x, q.y)).collect()));
        }
        let mut list = self.found_list(&rows)?;
        list["end"] = column::write(&Column::Str(ends), self.objects)
            .map_err(|e| refuse(-32603, e.to_string()))?;
        Ok(list)
    }

    fn derive_spill(&self, p: &Value) -> Result<Value, Refusal> {
        let view = p.get("view").and_then(Value::as_str).unwrap_or("full");
        let (_, values) = self.one_field(p, view)?;
        let xs = self.ints(p, "x")?;
        let ys = self.ints(p, "y")?;
        let rs = match p.get("radius") {
            None | Some(Value::Null) => vec![0; xs.len()],
            Some(_) => self.ints(p, "radius")?,
        };
        if xs.len() != ys.len() || xs.len() != rs.len() {
            return Err(invalid("x, y and radius: columns of one length"));
        }
        let field = self.field_raster(&values)?;
        let pts: Vec<via_geom::P> = xs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| via_geom::P::new(x, y))
            .collect();
        // The one implementation of `spill` (`derived.md` 4.1).
        let found = via_model::derived::spill(&self.grid, &field, &pts, &rs)
            .map_err(|e| self.raster_refusal(&e, "x, y, radius"))?;
        let level: Vec<f64> = found.iter().map(|s| s.level).collect();
        let dry: Vec<u8> = found.iter().map(|s| u8::from(s.dry)).collect();
        Ok(json!({
            "level": column::write(&Column::F64(level), self.objects).map_err(|e| refuse(-32603, e.to_string()))?,
            "dry": column::write(&Column::Bool(dry), self.objects).map_err(|e| refuse(-32603, e.to_string()))?,
        }))
    }

    /// Answers one call (3.3, `derived.md` 4).
    pub fn answer(&self, method: &str, params: &Value) -> Result<Value, Refusal> {
        if self.reaches_bad(method, params) {
            return Err(refuse(
                BAD_READ,
                "the read reaches a part of the build that failed or was skipped",
            ));
        }
        match method {
            "build.field" => self.field(params),
            "build.features" => self.features(params),
            "build.instances" => self.instances(params),
            "input.open" => self.input(params),
            "pack.query" => self.pack_query(params),
            "expr.eval" => self.expr_eval(params),
            "derive.where" => self.derive_where(params),
            "derive.route" => self.derive_route(params),
            "derive.descend" => self.derive_descend(params),
            "derive.spill" => self.derive_spill(params),
            "surface.meetings" => self.meetings(),
            "surface.surfaces_at" => self.surfaces_at(params),
            "surface.sample" => self.sample(params),
            m => Err(Refusal {
                code: -32601,
                message: format!("unknown method {m}"),
                data: Some(json!({"method": m})),
                diag: None,
            }),
        }
    }

    /// Whether a read reaches a failed or skipped node (`diagnostics.md` 4): a field it names,
    /// a feature it may select, any instance, or for a computing call any failed field.
    fn reaches_bad(&self, method: &str, params: &Value) -> bool {
        let Some(bad) = self.bad.filter(|b| !b.is_empty()) else {
            return false;
        };
        let feature_bad = |n: &str| {
            bad.features.contains(n)
                || n.split_once('#')
                    .is_some_and(|(p, _)| bad.steps.contains(p))
        };
        let kind_of = |n: &str| -> Option<String> {
            self.build
                .features
                .get(n)
                .and_then(|f| f.kind.clone())
                .or_else(|| {
                    self.world.entity("feature", n).and_then(|t| {
                        t.get("kind").and_then(|v| match &v.item.value {
                            via_doc::Value::Str(s) => Some(s.text.clone()),
                            _ => None,
                        })
                    })
                })
        };
        match method {
            "build.field" => {
                let sels: Vec<String> = match params.get("fields") {
                    Some(Value::Array(a)) => a
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect(),
                    Some(Value::String(s)) => vec![s.clone()],
                    _ => Vec::new(),
                };
                // A failed field may be missing from the build, so a selector is also matched
                // against the text's statements of the failed fields.
                let text_of = |n: &str, key: &str| -> Vec<String> {
                    let Some(t) = self.world.entity("field", n) else {
                        return Vec::new();
                    };
                    match t.get(key).map(|v| &v.item.value) {
                        Some(via_doc::Value::Str(s)) => vec![s.text.clone()],
                        Some(via_doc::Value::List(l)) => l
                            .iter()
                            .filter_map(|i| match &i.value {
                                via_doc::Value::Str(s) => Some(s.text.clone()),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    }
                };
                sels.iter().any(|s| {
                    bad.fields.contains(s)
                        || bad.fields.iter().any(|n| {
                            s.strip_prefix("role:")
                                .is_some_and(|r| text_of(n, "role").iter().any(|x| x == r))
                                || s.strip_prefix("tag:")
                                    .is_some_and(|t| text_of(n, "tags").iter().any(|x| x == t))
                        })
                        || self
                            .field_names(s)
                            .unwrap_or_default()
                            .iter()
                            .any(|n| bad.fields.contains(n))
                })
            }
            "build.features" => match params.get("select").unwrap_or(&json!("*")) {
                Value::String(s) if s == "*" => !bad.features.is_empty() || !bad.steps.is_empty(),
                Value::String(s) if s.starts_with("kind:") => {
                    let k = &s["kind:".len()..];
                    bad.features
                        .iter()
                        .any(|n| kind_of(n).is_none_or(|x| x == k))
                        || !bad.steps.is_empty()
                }
                Value::String(s) => feature_bad(s),
                Value::Array(a) => a.iter().filter_map(Value::as_str).any(feature_bad),
                _ => false,
            },
            "build.instances" => bad.instances,
            "expr.eval" | "derive.where" | "derive.route" | "derive.descend" | "derive.spill" => {
                !bad.fields.is_empty()
            }
            // The terrain and every kind surface: any failed field or feature may be one.
            "surface.surfaces_at" | "surface.sample" => {
                !bad.fields.is_empty() || !bad.features.is_empty() || !bad.steps.is_empty()
            }
            _ => false,
        }
    }

    /// The stated facts among a feature list's names (`derived.md` 5): features written in text
    /// whose tier is shaped or anchored.
    pub fn stated(&self, names: &[String]) -> Vec<String> {
        names
            .iter()
            .filter(|n| {
                self.world.entity("feature", n).is_some()
                    && self.build.features.get(*n).is_some_and(|f| {
                        matches!(
                            f.tier,
                            Some(via_model::Tier::Shaped | via_model::Tier::Anchored)
                        )
                    })
            })
            .cloned()
            .collect()
    }
}

/// The features a `for_each = "kind:<k>"` selects in the view of layer rank `rank`, by name.
pub fn items(world: &World, build: &Build, rank: Option<i64>, kind: &str) -> Vec<String> {
    // `for_each = "given:<name>"`: one call per feature given under `<name>`, which stand below
    // every layer of the fill (`composition.md` 8, 12).
    if let Some(name) = kind.strip_prefix("given:") {
        return given_names(world, build, name);
    }
    build
        .features
        .iter()
        .filter(|(n, f)| {
            f.kind.as_deref() == Some(kind)
                && match (rank, view_rank(world, "feature", n)) {
                    (Some(own), Some(r)) => r <= own,
                    _ => true,
                }
        })
        .map(|(n, _)| n.clone())
        .collect()
}

/// The features `given:<name>` selects (`composition.md` 8), by name: those a parent gave under
/// `<name>`, named `<name>.<parent id>`; when none was given, those the given's `default`
/// selector (`kind:<k>`) selects among the fill's own. Empty when `<name>` is no given of
/// `accepts = "features"`.
pub fn given_names(world: &World, build: &Build, name: &str) -> Vec<String> {
    let Some(t) = world.entity("given", name) else {
        return Vec::new();
    };
    if t.str("accepts") != Some("features") {
        return Vec::new();
    }
    let prefix = format!("{name}.");
    let given: Vec<String> = build
        .features
        .keys()
        .filter(|n| n.starts_with(&prefix) && world.entity("feature", n).is_none())
        .cloned()
        .collect();
    if !given.is_empty() {
        return given;
    }
    match t.str("default").and_then(|d| d.strip_prefix("kind:")) {
        Some(k) => build
            .features
            .iter()
            .filter(|(_, f)| f.kind.as_deref() == Some(k))
            .map(|(n, _)| n.clone())
            .collect(),
        None => Vec::new(),
    }
}

/// The kinds whose effect writes this field (by name, `role:` or `tag:`).
fn effect_kinds<'w>(world: &'w World, build: &Build, field: &str) -> BTreeSet<&'w str> {
    let mut kinds: BTreeSet<&str> = BTreeSet::new();
    let Some(f) = build.fields.get(field) else {
        return kinds;
    };
    let matches = |sel: &str| -> bool {
        if let Some(r) = sel.strip_prefix("role:") {
            f.role.as_deref() == Some(r)
        } else if let Some(t) = sel.strip_prefix("tag:") {
            f.tags.iter().any(|x| x == t)
        } else {
            sel == field
        }
    };
    for k in world.ids("kind") {
        let Some(t) = world.entity("kind", k) else {
            continue;
        };
        let Some(effects) = t.table("effect") else {
            continue;
        };
        if effects.iter().any(|(_, e)| match &e.item.value {
            via_doc::Value::Table(et) => et.str("field").is_some_and(matches),
            _ => false,
        }) {
            kinds.insert(k);
        }
    }
    kinds
}

/// The rank of a name of the world namespace; a produced thing's is its producer's.
fn rank_of(world: &World, top: &str, name: &str) -> Option<i64> {
    world.rank(top, name).or_else(|| {
        let head = name.split(['.', '#', '/']).next().unwrap_or(name);
        world.find(head).map(|(_, r)| r)
    })
}

/// The rank a thing has in a generate step's view (`protocol.md` 1, 3.3): its layer's, except that
/// what a generate step returned is outside the stratum of every step of that layer, so it counts
/// as one above it: a step of a higher layer sees it, a step of its own layer does not. What a
/// network or cells table made stands where a step's return stands (`geometry.md` 8.1). The rule
/// is the resolver's (`Decls::stands_returned`): a step's or a maker table's `#` names count, not
/// a scatter's.
pub fn view_rank(world: &World, top: &str, name: &str) -> Option<i64> {
    let r = rank_of(world, top, name)?;
    let returned = |m: &str| {
        ["step", "network", "cells"]
            .iter()
            .any(|t| world.entity(t, m).is_some())
    };
    Some(match name.split_once('#') {
        Some((maker, _)) if returned(maker) => r + 1,
        _ => r,
    })
}

/// The view rank of an instance a producer made: a step's returned instance counts one above its
/// layer (as [`view_rank`]).
pub fn maker_rank(world: &World, maker: &str) -> Option<i64> {
    let (_, r) = world.find(maker)?;
    Some(if world.entity("step", maker).is_some() {
        r + 1
    } else {
        r
    })
}

/// Whether a statement above layer rank `own` writes this field, so the finished build's values
/// of it are not that layer's stratum's: a feature or footprint above whose kind has an effect
/// on it, or a `then` row a higher layer added.
///
/// `returns_above` says whether what a step returned counts one layer above its own (as
/// [`view_rank`]): true for a worker's view, which leaves a step's returns out. A plain command is
/// handed the build's fields whole, and its own returns are part of its layer there, so for it
/// they are not a higher layer (`authority.md` 4.4).
pub fn field_inexact(
    world: &World,
    build: &Build,
    field: &str,
    own: i64,
    returns_above: bool,
) -> bool {
    let rank = |top: &str, n: &str| {
        if returns_above {
            view_rank(world, top, n)
        } else {
            rank_of(world, top, n)
        }
    };
    let above = |r: Option<i64>| r.is_some_and(|r| r > own);
    let kinds = effect_kinds(world, build, field);
    if build.features.iter().any(|(n, f)| {
        f.kind.as_deref().is_some_and(|k| kinds.contains(k)) && above(rank("feature", n))
    }) || build
        .footprints
        .iter()
        .any(|(n, f)| kinds.contains(f.kind.as_str()) && above(rank("instance", n)))
    {
        return true;
    }
    world
        .entity("field", field)
        .and_then(|t| t.table("then"))
        .is_some_and(|rows| {
            rows.iter().any(|(_, e)| {
                e.chain.iter().any(|l| {
                    l.through == via_diag::Through::Layer && above(world.layer_rank(&l.name))
                })
            })
        })
}

/// Whether any field a plain command at layer rank `own` is handed is written from above it.
pub fn stratum_inexact(world: &World, build: &Build, own: i64) -> bool {
    build.fields.keys().any(|f| {
        world.rank("field", f).is_none_or(|r| r <= own)
            && field_inexact(world, build, f, own, false)
    })
}

fn path_row(k: usize, line: Vec<(i64, i64)>) -> Row {
    Row {
        name: format!("{k}"),
        kind: String::new(),
        authority: String::new(),
        layer: String::new(),
        rank: None,
        geometry: if line.len() < 2 { 0 } else { 1 },
        pieces: vec![vec![line]],
        attrs: BTreeMap::new(),
        types: BTreeMap::new(),
    }
}

/// The paths of a feature list in a message: each feature's vertices, every ring in order.
fn read_paths(v: &Value, objects: &Objects) -> Result<Vec<Vec<(i64, i64)>>, Refusal> {
    let col = |k: &str| -> Result<Vec<i64>, Refusal> {
        let c = v
            .get(k)
            .ok_or_else(|| invalid(format!("waypoints: no `{k}` column")))?;
        column::read(k, c, objects)
            .map_err(|e| refuse(e.code, e.message))?
            .as_i64()
            .ok_or_else(|| invalid(format!("waypoints: `{k}` is not integers")))
    };
    let coords = col("coords")?;
    let pieces = col("pieces")?;
    let rings = col("rings")?;
    let starts = col("ring_starts")?;
    let mut out = Vec::new();
    let get = |a: &[i64], i: usize| -> Result<usize, Refusal> {
        a.get(i)
            .and_then(|x| usize::try_from(*x).ok())
            .ok_or_else(|| invalid("waypoints: offsets out of range"))
    };
    for f in 0..pieces.len().saturating_sub(1) {
        let mut pts = Vec::new();
        for pc in get(&pieces, f)?..get(&pieces, f + 1)? {
            for r in get(&rings, pc)?..get(&rings, pc + 1)? {
                for c in get(&starts, r)?..get(&starts, r + 1)? {
                    let x = *coords
                        .get(2 * c)
                        .ok_or_else(|| invalid("waypoints: coords"))?;
                    let y = *coords
                        .get(2 * c + 1)
                        .ok_or_else(|| invalid("waypoints: coords"))?;
                    pts.push((x, y));
                }
            }
        }
        out.push(pts);
    }
    Ok(out)
}

fn expr_refusal(e: &via_expr::ExprError) -> Refusal {
    Refusal {
        code: -32602,
        message: e.to_string(),
        data: Some(json!({"code": e.code.as_str()})),
        diag: None,
    }
}

/// Whether a point lies in a ring, its boundary included, exactly (`geometry.md`'s test).
fn in_ring(ring: &[(i64, i64)], p: (i64, i64)) -> Option<bool> {
    let n = ring.len();
    if n == 0 {
        return Some(false);
    }
    let mut inside = false;
    for k in 0..n {
        let a = ring[k];
        let b = ring[(k + 1) % n];
        // On the segment: collinear and within its box.
        let cross = i128::from(b.0 - a.0) * i128::from(p.1 - a.1)
            - i128::from(b.1 - a.1) * i128::from(p.0 - a.0);
        if cross == 0
            && p.0 >= a.0.min(b.0)
            && p.0 <= a.0.max(b.0)
            && p.1 >= a.1.min(b.1)
            && p.1 <= a.1.max(b.1)
        {
            return None;
        }
        if (a.1 > p.1) != (b.1 > p.1) {
            // x of the crossing compared with p.0, without division.
            let lhs = i128::from(p.0 - a.0) * i128::from(b.1 - a.1);
            let rhs = i128::from(b.0 - a.0) * i128::from(p.1 - a.1);
            let right = if b.1 > a.1 { lhs < rhs } else { lhs > rhs };
            if right {
                inside = !inside;
            }
        }
    }
    Some(inside)
}

/// Whether a point lies in any of these rings, read as one region by even-odd.
/// Whether a feature meets a region (`protocol.md` 3.3, `within`): a vertex inside it or on its
/// outline, an edge crossing or touching its outline, or, for a region feature, the region inside
/// it. A path crossing the region between two vertices outside it meets it.
fn meets(region: &[Vec<(i64, i64)>], r: &Row) -> bool {
    let bbox = |pts: &mut dyn Iterator<Item = &(i64, i64)>| {
        pts.fold(None, |b: Option<(i64, i64, i64, i64)>, p| {
            Some(b.map_or((p.0, p.1, p.0, p.1), |b| {
                (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
            }))
        })
    };
    let (Some(rb), Some(fb)) = (bbox(&mut region.iter().flatten()), bbox(&mut r.vertices())) else {
        return false;
    };
    if fb.0 > rb.2 || rb.0 > fb.2 || fb.1 > rb.3 || rb.1 > fb.3 {
        return false;
    }
    if r.vertices().any(|v| inside(region, *v)) {
        return true;
    }
    let closed = r.geometry == 2;
    let edges = |ring: &[(i64, i64)], closed: bool| -> Vec<((i64, i64), (i64, i64))> {
        let n = ring.len();
        let m = if closed { n } else { n.saturating_sub(1) };
        (0..m).map(|k| (ring[k], ring[(k + 1) % n])).collect()
    };
    let outline: Vec<_> = region.iter().flat_map(|g| edges(g, true)).collect();
    for ring in r.pieces.iter().flatten() {
        for (a, b) in edges(ring, closed) {
            if outline.iter().any(|&(c, d)| segments_meet(a, b, c, d)) {
                return true;
            }
        }
    }
    closed
        && r.pieces
            .iter()
            .any(|piece| region.iter().flatten().any(|v| inside(piece, *v)))
}

/// Whether two closed segments share a point, exactly, on integer millimetres.
fn segments_meet(a: (i64, i64), b: (i64, i64), c: (i64, i64), d: (i64, i64)) -> bool {
    let orient = |p: (i64, i64), q: (i64, i64), r: (i64, i64)| -> i128 {
        let v = i128::from(q.0 - p.0) * i128::from(r.1 - p.1)
            - i128::from(q.1 - p.1) * i128::from(r.0 - p.0);
        v.signum()
    };
    let on = |p: (i64, i64), q: (i64, i64), r: (i64, i64)| {
        r.0 >= p.0.min(q.0) && r.0 <= p.0.max(q.0) && r.1 >= p.1.min(q.1) && r.1 <= p.1.max(q.1)
    };
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    (o1 != o2 && o3 != o4)
        || (o1 == 0 && on(a, b, c))
        || (o2 == 0 && on(a, b, d))
        || (o3 == 0 && on(c, d, a))
        || (o4 == 0 && on(c, d, b))
}

fn inside(rings: &[Vec<(i64, i64)>], p: (i64, i64)) -> bool {
    let mut n = 0;
    for r in rings {
        match in_ring(r, p) {
            None => return true,
            Some(true) => n += 1,
            Some(false) => {}
        }
    }
    n % 2 == 1
}

/// Selectors resolved in the view (`formulas.md` 7).
struct Sel<'v, 'a> {
    view: &'v View<'a>,
}

impl via_expr::Select for Sel<'_, '_> {
    fn fields(&self, selector: &str) -> Vec<via_expr::FieldInfo> {
        self.view
            .field_names(selector)
            .unwrap_or_default()
            .into_iter()
            .map(|n| via_expr::FieldInfo {
                weights: self.view.build.fields[&n].values.is_none(),
                name: n,
            })
            .collect()
    }

    fn shapes(&self, selector: &str) -> Vec<via_expr::ShapeInfo> {
        let rows = self.view.visible_rows();
        let pick = |r: &Row| via_expr::ShapeInfo {
            id: r.name.clone(),
            shape: match r.geometry {
                0 => via_expr::Shape::Point,
                1 => via_expr::Shape::Path,
                _ => via_expr::Shape::Region {
                    rings: r.pieces.iter().map(|p| p.len() as u32).sum(),
                },
            },
        };
        if let Some(k) = selector.strip_prefix("kind:") {
            rows.iter().filter(|r| r.kind == k).map(pick).collect()
        } else {
            rows.iter()
                .filter(|r| r.name == selector)
                .map(pick)
                .collect()
        }
    }

    fn names(&self, class: via_expr::Class) -> Vec<String> {
        match class {
            via_expr::Class::Field => self.view.build.fields.keys().cloned().collect(),
            _ => self
                .view
                .visible_rows()
                .into_iter()
                .map(|r| r.name)
                .collect(),
        }
    }
}

/// Field values for the evaluator: at posts by index, between posts by `fields.md` 1.2.
struct Reads<'v, 'a> {
    view: &'v View<'a>,
    /// The view the call names (`full` or `head`): each field is read in it, as `build.field`
    /// reads it.
    which: &'v str,
}

impl via_expr::Reads for Reads<'_, '_> {
    fn field(
        &self,
        field: &str,
        domain: &via_expr::Domain,
    ) -> Result<Vec<Option<f64>>, via_expr::ReadError> {
        let g = self.view.grid;
        let Some(v) = self
            .view
            .build
            .fields
            .get(field)
            .and_then(|f| f.values.as_ref())
            .and_then(|_| self.view.scalar_values(field, self.which))
        else {
            return Err(via_expr::ReadError::new(
                codes::formula::SELECTOR,
                format!("`{field}` is not a scalar field here"),
                "read a scalar field",
            ));
        };
        let nx = g.nx as usize;
        let r = g.resolution;
        let at = |i: usize, j: usize| f64::from(v[j * nx + i]);
        Ok(domain
            .points()
            .into_iter()
            .map(|(x, y)| {
                let (a, b) = (x - g.extent[0], y - g.extent[1]);
                if a < 0 || b < 0 || x > g.extent[2] || y > g.extent[3] {
                    return None;
                }
                let i = ((a / r) as usize).min(nx - 2);
                let j = ((b / r) as usize).min(g.ny as usize - 2);
                let fx = (a - i as i64 * r) as f64 / r as f64;
                let fy = (b - j as i64 * r) as f64 / r as f64;
                let s = (1.0 - fx) * at(i, j) + fx * at(i + 1, j);
                let n = (1.0 - fx) * at(i, j + 1) + fx * at(i + 1, j + 1);
                Some((1.0 - fy) * s + fy * n)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_sorts_clips_and_drops_call() {
        let g = Grid {
            extent: [0, 0, 2000, 1000],
            resolution: 1000,
            nx: 3,
            ny: 2,
        };
        let n = normalise(
            "build.field",
            &json!({"call": 2, "fields": ["b", "a"], "window": {"i0": -1, "j0": 0, "i1": 9, "j1": 2}}),
            &g,
        );
        assert_eq!(
            n,
            json!({"method": "build.field", "fields": ["a", "b"], "halo": 0, "view": "full", "window": [0, 0, 3, 2]})
        );
    }

    /// A recorded `build.field` request replays to the posts the live call read: an off-grid
    /// window with a halo, a window whose upper bound is below 0, and a malformed window (refused
    /// live, so refused again on replay).
    #[test]
    fn traced_window_replays_as_answered() {
        let g = Grid {
            extent: [0, 0, 64_000, 64_000],
            resolution: 1000,
            nx: 64,
            ny: 64,
        };
        for raw in [
            json!({"fields": ["h"], "window": [70, 0, 80, 10], "halo": 10}),
            json!({"fields": ["h"], "window": [-20, -20, -5, -5], "halo": 10}),
            json!({"fields": ["h"], "window": [3, 4, 9, 9], "halo": 2}),
            json!({"fields": ["h"], "halo": 5}),
        ] {
            let live = resolved_window(&raw, &g).unwrap();
            let traced = normalise("build.field", &raw, &g);
            assert_eq!(resolved_window(&traced, &g).unwrap(), live, "{raw}");
            assert_eq!(traced["halo"], json!(0));
        }
        assert_eq!(
            resolved_window(&json!({"window": [70, 0, 80, 10], "halo": 10}), &g).unwrap(),
            [60, 0, 64, 20]
        );
        let bad = json!({"fields": ["h"], "window": [1, 2], "halo": 3});
        let traced = normalise("build.field", &bad, &g);
        assert_eq!(traced["window"], json!([1, 2]));
        let (a, b) = (
            resolved_window(&bad, &g).unwrap_err(),
            resolved_window(&traced, &g).unwrap_err(),
        );
        assert_eq!((a.code, a.message), (b.code, b.message));
    }

    #[test]
    fn answers_digest_without_paths() {
        let a = json!({"h": {"blob": "ab", "path": "/x/ab"}});
        let b = json!({"h": {"blob": "ab", "path": "/y/ab"}});
        assert_eq!(answer_digest(&a), answer_digest(&b));
    }

    #[test]
    fn within_takes_what_meets_the_region() {
        // `protocol.md` 3.3 `within`: a region 10 m square.
        let region = vec![vec![(0, 0), (10_000, 0), (10_000, 10_000), (0, 10_000)]];
        // A path passing the region between two far vertices, none inside it: it meets it.
        assert!(meets(
            &region,
            &path_row(0, vec![(-50_000, 5_000), (60_000, 5_000)])
        ));
        // One along its outline (a line on its edge) meets it too.
        assert!(meets(
            &region,
            &path_row(1, vec![(-50_000, 0), (60_000, 0)])
        ));
        // One 1 mm outside, does not.
        assert!(!meets(
            &region,
            &path_row(2, vec![(-50_000, -1), (60_000, -1)])
        ));
        // A region holding the whole of it meets it, with no vertex inside it.
        let mut around = path_row(
            3,
            vec![(-1, -1), (20_000, -1), (20_000, 20_000), (-1, 20_000)],
        );
        around.geometry = 2;
        assert!(meets(&region, &around));
        // A path that only passes near the corner, outside, does not.
        assert!(!meets(
            &region,
            &path_row(4, vec![(10_001, -5_000), (20_000, 5_000)])
        ));
    }
}
