//! The build through `via-graph` (`graph.md` 1 and 2): every statement as nodes, every reference
//! as an edge, and the build's own fields cut by its chunks (`build.md` 5).
//!
//! # The nodes
//!
//! [`ModelPlan`] defines, in the build's view: `source` and `merge`; `attr`, `shape`,
//! `instance`, `footprint` and `edit` per statement; `select` per selector read; `field` per
//! field and chunk, and in view `build-nofootprint` per `keep_out` field and chunk under a hand
//! instance; `distance` per writer (or selector read) and chunk its window meets; `measure` per
//! effect that calls `under` or `smoothed_along`; `rule`, `held` and `height` per producer;
//! `height` per hand instance and pin. The command line adds `write`, `step` and `input`.
//!
//! Every definition is worked out once, from the text and the placement pass (shapes and plans,
//! which every build that is not the null build runs, and which are cheap): which chunks an
//! effect's window meets is known from its shape before anything runs. So a node's edges are
//! those `graph.md` 2 gives, and its key is a function of what it reads.
//!
//! # The values
//!
//! A node's value is computed from the model's own passes, made once per process and only when a
//! node has to run: fields over the whole extent (`fields.md` 6.1: field diagnostics are found
//! over the whole extent, once), then producers, hand edits and heights. A node takes its part
//! of them: a chunk is the cut of its field (`build.md` 5: a chunk's samples equal the same cut of
//! the whole field), a `held` node its producer's instances. Each part depends only on what the
//! node's edges carry, so a value is a function of its key, and a reused value equals what a
//! clean build computes (section 7). Two exceptions, both conservative: a `measure` node's value
//! is the digest of its inputs (it is never early cut off), and a `rule` node is the scatter run
//! with no hand edit.
//!
//! # Diagnostics
//!
//! Each diagnostic belongs to one node, which carries it in its value (`graph.md` 3.4): a field's
//! to the chunk that holds its position, or to every chunk of the field when it has none; a
//! statement's to that statement's node (a feature's placement to `shape`, its attributes to
//! `attr`, a scatter's to `held`, an instance's checks to `height`); anything else to `merge`,
//! whose value covers every statement.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use via_diag::{codes, Diag, KeyPart, Site, Subject};
use via_doc::World;
use via_expr::{Class, FormulaId, Op};
use via_geom::P;
use via_graph::{
    Dec, Def, Defined, Digest, Enc, Inputs, Known, NodeId, Output, Plan, Reports, Val,
};
use via_num::Grid;
use via_raster::window::{effect_window, Chunks};
use via_raster::Window;

use crate::build::{finish_from, prepare, Build, Options, Prepared};
use crate::decl::{Attr, Place, Role, Selector, ShapeDecl, Tier};
use crate::place::{Geo, Placer};
use crate::resolve::{Fields, Resolved, View};
use crate::rules::{self, Placement};

/// The build's own view.
const BUILD: &str = "build";
/// The build's view less every footprint feature's entries (`rules.md` 8.3).
const NOFP: &str = "build-nofootprint";
/// The build's head view of a field: base and rows, no effect (what a `where` reads).
const HEAD: &str = "head";
/// The scope `via-expr` gives a world `let`.
const WORLD_LET: &str = "the world's [let]";

/// What a formula reads, for edges.
#[derive(Clone, Debug, Default)]
struct FReads {
    fields: BTreeSet<String>,
    shapes: BTreeSet<String>,
    lets: BTreeSet<String>,
    halo: u64,
    measures: BTreeSet<&'static str>,
}

impl FReads {
    fn add(&mut self, o: &FReads) {
        self.fields.extend(o.fields.iter().cloned());
        self.shapes.extend(o.shapes.iter().cloned());
        self.lets.extend(o.lets.iter().cloned());
        self.halo = self.halo.max(o.halo);
        self.measures.extend(o.measures.iter().copied());
    }
}

/// One effect of one writer on one field.
#[derive(Clone, Debug)]
struct Eff {
    /// The writer's place name: a feature id, or `<instance>.ground`.
    writer: String,
    /// For a footprint feature, its instance.
    instance: Option<String>,
    /// For a made thing (`geometry.md` 8.1), its network or cells table.
    maker: Option<String>,
    kind: String,
    effect: String,
    /// The effect's window (`fields.md` 5.3); the whole extent without a radius.
    window: Option<Window>,
    reads: FReads,
    /// The writer's layer rank: a key of row order (R29), so a chunk's key holds it.
    rank: i64,
}

impl Eff {
    fn label(&self) -> String {
        format!("effect {}.{}", self.writer, self.effect)
    }

    fn measure_id(&self, field: &str, call: &str) -> NodeId {
        NodeId::of("measure", field)
            .view(&format!("{BUILD}/{}.{}/{call}", self.writer, self.effect))
    }

    /// The node that holds the writer's shape.
    fn shape_node(&self) -> NodeId {
        match (&self.instance, &self.maker) {
            (Some(i), _) => view(NodeId::of("footprint", i)),
            (None, Some(_)) => view(NodeId::of("made", &self.writer)),
            (None, None) => view(NodeId::of("shape", &self.writer)),
        }
    }

    /// The node that holds the writer's attributes: a made thing's are its kind's defaults, in
    /// its `made` node beside its shape.
    fn attr_node(&self) -> NodeId {
        match (&self.instance, &self.maker) {
            (Some(i), _) => view(NodeId::of("footprint", i)),
            (None, Some(_)) => view(NodeId::of("made", &self.writer)),
            (None, None) => NodeId::of("attr", &self.writer),
        }
    }

    /// The statement subject of a node about this writer.
    fn subject(&self, d: &crate::decl::Decls) -> Subject {
        match (&self.instance, &self.maker) {
            (Some(i), _) => Subject::entity("instance", i.clone()),
            (None, Some(m)) => Subject::entity(d.maker_top(m).unwrap_or("network"), m.clone()),
            (None, None) => Subject::entity("feature", self.writer.clone()),
        }
    }
}

/// A field's entries as the graph needs them.
#[derive(Clone, Debug, Default)]
struct FieldInfo {
    /// Base and rows.
    own: FReads,
    effs: Vec<Eff>,
}

/// Fields over the whole extent, made once per process (`fields.md` 6.1).
/// Fields in a step's head view, by (field, layer rank).
type StratumMemo = BTreeMap<(String, i64), Option<Vec<f32>>>;

struct FieldsMemo {
    build: BTreeMap<String, Option<Resolved>>,
    head: BTreeMap<String, Option<Resolved>>,
    nofp: BTreeMap<String, Option<Resolved>>,
    routed: BTreeMap<NodeId, Vec<Diag>>,
}

/// The rest of the build, made once per process.
/// Time spent evaluating the world as a whole, outside any node's own inputs: the field memo, the
/// whole build, the pre-edit scatter and the step strata, in nanoseconds, over the process. Not
/// a node's cost (graph.md 6.3 `totals.memo_ms`): the gap between the nodes' times and the wall.
static MEMO_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// [`MEMO_NS`] in whole milliseconds.
pub fn memo_ms() -> u64 {
    MEMO_NS.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000
}

/// Adds the time since `t0` to [`MEMO_NS`].
fn memo_spent(t0: std::time::Instant) {
    let ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
    MEMO_NS.fetch_add(ns, std::sync::atomic::Ordering::Relaxed);
}

struct Whole {
    build: Build,
    routed: BTreeMap<NodeId, Vec<Diag>>,
}

/// Each producer's output before hand edits.
type PreEdit = BTreeMap<String, Vec<Placement>>;

fn view(id: NodeId) -> NodeId {
    id.view(BUILD)
}

fn chunk_id(field: &str, v: &str, w: &Window) -> NodeId {
    NodeId::of("field", field)
        .view(v)
        .window(w.i0, w.j0, w.i1, w.j1)
}

fn dist_id(of: &str, w: &Window) -> NodeId {
    NodeId::of("distance", of)
        .view(BUILD)
        .window(w.i0, w.j0, w.i1, w.j1)
}

fn meets(a: &Window, b: &Window) -> bool {
    a.intersect(b).is_some()
}

fn win_text(w: &Window) -> String {
    format!("{}.{}.{}.{}", w.i0, w.j0, w.i1, w.j1)
}

/// The window of a node id (`/w:<i0>.<j0>.<i1>.<j1>`).
fn window_of(id: &NodeId) -> Option<Window> {
    let s = id.as_str();
    let (_, w) = s.split_once("/w:")?;
    let w = w.split('/').next()?;
    let n: Vec<u64> = w.split('.').filter_map(|x| x.parse().ok()).collect();
    (n.len() == 4)
        .then(|| Window::new(n[0], n[1], n[2], n[3]))
        .flatten()
}

/// The subject of a node id: between `<family>:` and the first suffix.
fn subject_of(id: &NodeId) -> String {
    // The vertex keeps a head view as its own (`graph.md` 2.1); the subject never holds it.
    let v = id.vertex();
    let v = v.strip_suffix("/v:head").unwrap_or(&v);
    v.split_once(':')
        .map_or(String::new(), |(_, s)| s.to_string())
}

fn f32s(e: &mut Enc, v: &[f32]) {
    e.len(v.len());
    for x in v {
        e.u64(u64::from(x.to_bits()));
    }
}

/// Each member's box, for a lower bound on its distance in [`nearest_member`]; `None` for an empty
/// region, which measures 0 everywhere and so has no bound.
fn member_boxes(geos: &[&Geo]) -> Vec<Option<[i64; 4]>> {
    geos.iter()
        .map(|g| match g {
            Geo::Region { region, .. } if region.bbox().is_none() => None,
            g => Some(bbox_of(g)),
        })
        .collect()
}

/// The `(d, sd)` of the first member in order with the least distance to `at`: the values of
/// projecting every member. A member whose box bound passes the distance of the member nearest by
/// box (with a margin far wider than the doubles' error) cannot be the least, nor tie it, so it
/// is not projected.
fn nearest_member(geos: &[&Geo], boxes: &[Option<[i64; 4]>], at: P) -> Option<(f64, f64)> {
    let lower = |b: &Option<[i64; 4]>| -> f64 {
        let Some(b) = b else { return 0.0 };
        let gap = |lo: i64, hi: i64, v: i64| (lo - v).max(v - hi).max(0) as f64;
        let (gx, gy) = (gap(b[0], b[2], at.x), gap(b[1], b[3], at.y));
        (gx * gx + gy * gy).sqrt() / 1000.0
    };
    let bounds: Vec<f64> = boxes.iter().map(lower).collect();
    let reach = bounds
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map(|(k, _)| geos[k].project(at).d)
        .map_or(f64::INFINITY, |d| d * (1.0 + 1e-9) + 1e-6);
    let mut best: Option<(f64, f64)> = None;
    for (g, lb) in geos.iter().zip(&bounds) {
        if *lb > reach {
            continue;
        }
        let m = g.project(at);
        if best.is_none_or(|b| m.d < b.0) {
            best = Some((m.d, m.sd));
        }
    }
    best
}

fn bbox_of(g: &Geo) -> [i64; 4] {
    let (a, b) = g.bbox();
    [a.x, a.y, b.x, b.y]
}

fn region_bbox(r: &via_geom::Region, at: P) -> [i64; 4] {
    match r.bbox() {
        Some((a, b)) => [a.x, a.y, b.x, b.y],
        None => [at.x, at.y, at.x, at.y],
    }
}

/// Removes repeated diagnostics, keeping the first of each: a field's diagnostic with no
/// position is carried by every chunk of the field, and reported once.
pub fn dedupe(diags: Vec<Diag>) -> Vec<Diag> {
    let mut out: Vec<Diag> = Vec::with_capacity(diags.len());
    for d in diags {
        if !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

/// Takes away from `all` one copy of each diagnostic in `less`.
fn minus(all: &[Diag], less: &[Diag]) -> Vec<Diag> {
    let mut used = vec![false; less.len()];
    let mut out = Vec::new();
    'next: for d in all {
        for (k, l) in less.iter().enumerate() {
            if !used[k] && l == d {
                used[k] = true;
                continue 'next;
            }
        }
        out.push(d.clone());
    }
    out
}

/// What a file of the build directory reads, for its `write` node's edges (`graph.md` 2, "write").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileReads {
    /// A field's file: every chunk of the field.
    Field(String),
    /// One chunk's file of a field (a weights chunk, `build.md` 5): that chunk's node.
    Chunk(String, u64, u64),
    /// `features.json`: every feature's shape and attributes, every footprint, and the surface
    /// chunks under their points.
    Features,
    /// The instance columns: every plan, held output and height.
    Instances,
    /// `meets.json`: the paths with ends at a place, the owners of mounts and surfaces, what the
    /// ends name, the instances and their mounts' facts, and the terrain under them.
    Meets,
    /// A file whose reads are not known here: every node of the model.
    Everything,
}

/// The model's node kinds for the graph.
pub struct ModelPlan<'w> {
    world: &'w World,
    options: Options,
    baseline: Option<Build>,
    p: Prepared,
    grid: Option<Grid>,
    chunks: Vec<((u64, u64), Window)>,
    tiling: Option<Chunks>,
    stmts: BTreeMap<String, Digest>,
    fields: BTreeMap<String, FieldInfo>,
    defs: BTreeMap<NodeId, Def>,
    early: BTreeMap<NodeId, Vec<Diag>>,
    field_memo: Mutex<Option<Arc<FieldsMemo>>>,
    /// Fields in a generate step's head view, by (field, layer rank), made once per process.
    stratum: Mutex<StratumMemo>,
    whole: Mutex<Option<Arc<Whole>>>,
    pre: Mutex<Option<Arc<PreEdit>>>,
    /// Members of a loop the reference graph reported (`graph.md` 2.2): the passes' own loop
    /// messages about them are dropped.
    superseded: BTreeSet<String>,
}

/// The key under the reader's statement that makes an edge, from its label (`graph.md` 2):
/// `on.ref`, `starts_at`, `ends_at`, `follows`, `base` for a field's base and rows, `where`, a
/// name attribute. `None` when no one key of the reader's table makes it (an effect).
fn edge_key(label: &str) -> Option<via_diag::KeyPath> {
    let head = label.split(' ').next().unwrap_or(label);
    if head == "on.ref" {
        return Some(via_diag::KeyPath::key("on").then("ref"));
    }
    if label.contains(' ') {
        return match head {
            "base" | "where" => Some(via_diag::KeyPath::key(head)),
            _ => None,
        };
    }
    match head {
        "starts_at" | "ends_at" | "follows" | "within" => Some(via_diag::KeyPath::key(head)),
        // A name attribute's label is the attribute.
        h if !h.is_empty()
            && h.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') =>
        {
            Some(via_diag::KeyPath::key(h))
        }
        _ => None,
    }
}

/// The world's top-level keys that make the frame, the `world` statement (`graph.md` 1): the keys
/// that decide geometry and numerics. `name` and `include` are not among them, and `pack` is its
/// own statement ([`PACK`]).
pub const FRAME_KEYS: [&str; 4] = ["extent", "resolution", "seed", "chunk"];

/// The statement of the world's `pack` key, read by the nodes that resolve asset names.
pub const PACK: &str = "world.pack";

/// The one node of a declared feature, instance or pin whose declaration failed.
fn failed_node(top: &str, id: &str) -> NodeId {
    view(NodeId::of(
        match top {
            "feature" => "shape",
            "instance" => "instance",
            _ => "edit",
        },
        id,
    ))
}

/// Each statement's digest (`graph.md` 1): the canonical text of each key with its table path,
/// never a source position. `world` is the frame ([`FRAME_KEYS`]); `world.pack` the pack key;
/// `asset:<id>` holds a pack asset's facts.
fn statement_digests(world: &World, p: &Prepared) -> BTreeMap<String, Digest> {
    let mut out = BTreeMap::new();
    let mut top_keys: Vec<(String, String)> = Vec::new();
    for (top, e) in world.root().iter() {
        if top.starts_with('@') {
            continue;
        }
        match &e.item.value {
            via_doc::Value::Table(t) => {
                for (id, ent) in t.iter() {
                    let mut enc = Enc::new();
                    enc.str("via-statement-1").str(top).str(id);
                    // A layer statement's rank is part of its digest (`graph.md` 1): rank is a
                    // key of row order (R29), so moving it to a layer of another rank re-runs
                    // what reads it. Kinds and the pack have no rank.
                    match world.rank(top, id) {
                        Some(r) => enc.str("rank").str(&r.to_string()),
                        None => enc.str("no rank"),
                    };
                    match &ent.item.value {
                        via_doc::Value::Table(et) => {
                            let keys: Vec<(&String, String)> = et
                                .iter()
                                .map(|(k, v)| (k, crate::text::canonical(&v.item.value)))
                                .collect();
                            enc.len(keys.len());
                            for (k, v) in keys {
                                enc.str(k).str(&v);
                            }
                        }
                        v => {
                            enc.len(1);
                            enc.str("").str(&crate::text::canonical(v));
                        }
                    }
                    out.insert(format!("{top}.{id}"), Digest::of(&enc.finish()));
                }
            }
            // `pack` is its own statement, read by the nodes that resolve asset names; `name`
            // and `include` enter no node key: lookups and sources are in the build key.
            v if top == "pack" => {
                let mut enc = Enc::new();
                enc.str("via-statement-1").str("world").str("pack");
                enc.str(&crate::text::canonical(v));
                out.insert(PACK.into(), Digest::of(&enc.finish()));
            }
            v if FRAME_KEYS.contains(&top.as_str()) => {
                top_keys.push((top.clone(), crate::text::canonical(v)));
            }
            _ => {}
        }
    }
    let mut enc = Enc::new();
    enc.str("via-statement-1").str("world").len(top_keys.len());
    for (k, v) in &top_keys {
        enc.str(k).str(v);
    }
    out.insert("world".into(), Digest::of(&enc.finish()));
    for (id, a) in &p.pack.assets {
        out.insert(
            format!("asset:{id}"),
            Digest::of(format!("{:?}", a.facts()).as_bytes()),
        );
    }
    // A field's `file` is a declared outside input (`fields.md` 4): its bytes, by digest, are
    // part of the field's statement, so editing the file re-runs the field.
    for (name, f) in &p.decls.files {
        if let Some(d) = out.get_mut(&format!("field.{name}")) {
            *d = Digest::of(format!("{d}{}", f.digest).as_bytes());
        }
    }
    out
}

/// The files a build reads besides its sources (`graph.md` 1, `source` and `input`): the pack,
/// as `pack:<file name>`, and each field's `file`, by its path from the world's folder. Their
/// digests enter the build key (6.2), so a pack or a field file edited in place is never taken
/// for an unchanged build.
pub fn leaves(world: &World, options: &Options) -> Vec<(String, Digest)> {
    let p = prepare(world, options);
    let mut out: Vec<(String, Digest)> = leaves_of(&p, options)
        .into_iter()
        .map(|(path, _, d)| (path, d))
        .collect();
    // Each import's files, by its digest (`composition.md` 12: `file` is a source of the build
    // key, never of a node's).
    for imp in p.decls.imported.values() {
        out.push((
            format!("import {}", imp.shown),
            Digest::of(imp.alone.digest.as_bytes()),
        ));
    }
    out
}

/// Each leaf as (path, node id, digest).
pub(crate) fn leaves_of(p: &Prepared, options: &Options) -> Vec<(String, NodeId, Digest)> {
    let mut out = Vec::new();
    if let Some(written) = &p.decls.pack {
        let roots = crate::pack::Roots {
            world_dir: options.world_dir.clone(),
            lib_roots: options.lib_roots.clone(),
            via_lib: options.via_lib.clone(),
            shipped: options.shipped_root.clone(),
        };
        if let Some(path) = crate::pack::find(written, &roots, &mut Vec::new()) {
            if let Ok(bytes) = std::fs::read(&path) {
                let name = format!(
                    "pack:{}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or(written)
                );
                out.push((
                    name.clone(),
                    NodeId::of("source", &name),
                    Digest::of(&bytes),
                ));
                // `assets.md` 2.2 and 4: the files the pack binds, and its facts file.
                for (leaf, d) in p.pack.leaves() {
                    let family = if leaf == p.pack.facts_display {
                        "source"
                    } else {
                        "input"
                    };
                    out.push((leaf.clone(), NodeId::of(family, &leaf), d));
                }
            }
        }
    }
    for (name, fd) in &p.decls.fields {
        let Some(f) = &fd.file else { continue };
        let path = input_path(&f.source, &f.written);
        let digest = p
            .decls
            .files
            .get(name)
            .map_or(Digest::of(b"\0missing"), |b| {
                Digest::from_hex(&b.digest).unwrap_or_else(|| Digest::of(b.digest.as_bytes()))
            });
        out.push((path.clone(), NodeId::of("input", &path), digest));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out.dedup_by(|a, b| a.1 == b.1);
    out
}

/// A declared file's path as an `input` id names it: the folder of the document that names it,
/// then the path as written, with `/` (`graph.md` 1, "Paths").
pub(crate) fn input_path(source: &str, written: &str) -> String {
    let dir = match source.rsplit_once('/') {
        Some((d, _)) => d,
        None => "",
    };
    let mut parts: Vec<&str> = dir
        .split('/')
        .filter(|x| !x.is_empty() && *x != ".")
        .collect();
    for seg in written.split(['/', '\\']) {
        match seg {
            "" | "." => {}
            // A path above the world's folder (`documents.md` 7.2 allows `..`) keeps its leading
            // `..`, so it is never the id of a file inside the world; nor is a library's or the
            // pack's root (`lib:<library>`, `pack:`) ever popped.
            ".." => match parts.last() {
                Some(&last) if last != ".." && !last.contains(':') => {
                    parts.pop();
                }
                _ => parts.push(".."),
            },
            s => parts.push(s),
        }
    }
    parts.join("/")
}

impl<'w> ModelPlan<'w> {
    /// The plan of a world: every node defined from the text and the placement pass.
    pub fn new(world: &'w World, options: &Options, baseline: Option<&Build>) -> ModelPlan<'w> {
        let p = prepare(world, options);
        let mut stmts = statement_digests(world, &p);
        // The pack statement is what every node that resolves an asset reads: its files and
        // facts enter it, so a revised model re-runs them (`assets.md` 4, `asset.revised`).
        if let Some(d) = stmts.get(PACK).copied() {
            let mut enc = Enc::new();
            enc.str("via-pack-statement-1").str(&d.hex());
            // The facts file is not among them: it decides only diagnostics (`assets.md` 4.2),
            // which the `facts` nodes raise. With `--stand-ins` it decides every placement fact
            // of a model asset (9), so it is.
            let leaves: Vec<_> = p
                .pack
                .leaves()
                .into_iter()
                .filter(|(path, _)| p.pack.stand_ins || *path != p.pack.facts_display)
                .collect();
            enc.len(leaves.len());
            for (path, digest) in &leaves {
                enc.str(path).str(&digest.hex());
            }
            stmts.insert(PACK.into(), Digest::of(&enc.finish()));
        }
        let grid = p.decls.grid;
        let tiling = grid.as_ref().map(Chunks::new);
        let chunks = tiling.map(|t| t.list()).unwrap_or_default();
        let mut plan = ModelPlan {
            world,
            options: options.clone(),
            baseline: baseline.cloned(),
            p,
            grid,
            chunks,
            tiling,
            stmts,
            fields: BTreeMap::new(),
            defs: BTreeMap::new(),
            early: BTreeMap::new(),
            field_memo: Mutex::new(None),
            stratum: Mutex::new(BTreeMap::new()),
            whole: Mutex::new(None),
            pre: Mutex::new(None),
            superseded: BTreeSet::new(),
        };
        plan.fields = plan.field_infos();
        plan.define_all();
        plan.report_cycles();
        plan.break_cycles();
        let early = plan.route_early();
        plan.early = early;
        plan
    }

    /// Pairs of nodes of one family whose keys are made from the same parts (`graph.md` 3.1):
    /// statements, parameters, edges by label and input, and direct digests. Such a pair shares
    /// one key while it may compute two values, so the store hands one's value to the other.
    #[cfg(test)]
    pub(crate) fn key_clashes(&self) -> Vec<(NodeId, NodeId)> {
        let mut seen: BTreeMap<String, NodeId> = BTreeMap::new();
        let mut out = Vec::new();
        for (id, def) in &self.defs {
            let mut st: Vec<String> = def
                .statements
                .iter()
                .map(|s| format!("{:?}", s.1))
                .collect();
            st.sort();
            let mut params = def.params.clone();
            params.sort();
            let mut edges = def.edges.clone();
            edges.sort();
            let parts = format!(
                "{} {st:?} {params:?} {edges:?} {:?}",
                id.family(),
                def.direct
            );
            match seen.get(&parts) {
                Some(first) => out.push((first.clone(), id.clone())),
                None => {
                    seen.insert(parts, id.clone());
                }
            }
        }
        out
    }

    /// Every node the model defines, in id order.
    pub fn ids(&self) -> Vec<NodeId> {
        self.defs.keys().cloned().collect()
    }

    /// Each field by name, and whether it holds weights (one file per chunk) rather than a
    /// scalar (one file), as `build.md` 6 writes them.
    pub fn field_kinds(&self) -> Vec<(String, bool)> {
        self.p
            .decls
            .fields
            .values()
            .map(|f| (f.name.clone(), f.weights))
            .collect()
    }

    /// The build's chunks, in chunk order (`build.md` 5).
    pub fn chunk_list(&self) -> Vec<(u64, u64)> {
        self.chunks.iter().map(|(c, _)| *c).collect()
    }

    /// The statement digests, by statement id.
    pub fn statements(&self) -> &BTreeMap<String, Digest> {
        &self.stmts
    }

    /// Each node's definition, for the command line's own nodes and the change report's causes.
    pub fn def(&self, id: &NodeId) -> Option<&Def> {
        self.defs.get(id)
    }

    /// What a build file reads, as labelled edges.
    pub fn file_reads(&self, what: &FileReads) -> Vec<(String, NodeId)> {
        let mut out: Vec<(String, NodeId)> = Vec::new();
        match what {
            FileReads::Field(name) => {
                for (_, w) in &self.chunks {
                    out.push((format!("chunk {}", win_text(w)), chunk_id(name, BUILD, w)));
                }
            }
            FileReads::Chunk(name, cx, cy) => {
                for ((x, y), w) in &self.chunks {
                    if (x, y) == (cx, cy) {
                        out.push((format!("chunk {}", win_text(w)), chunk_id(name, BUILD, w)));
                    }
                }
            }
            FileReads::Features => {
                let mut pts: Vec<P> = Vec::new();
                for (id, g) in &self.p.placed.shapes {
                    if let Some((inst, _)) = self.p.placed.footprints.get(id) {
                        out.push((
                            format!("footprint {inst}"),
                            view(NodeId::of("footprint", inst)),
                        ));
                        continue;
                    }
                    out.push((format!("shape {id}"), view(NodeId::of("shape", id))));
                    out.push((format!("attr {id}"), NodeId::of("attr", id)));
                    match g {
                        Geo::Point { p, .. } => pts.push(*p),
                        Geo::Path { line } => pts.extend(line.pts.iter().copied()),
                        Geo::Region { .. } => {}
                    }
                }
                for id in self.p.placed.made.keys() {
                    out.push((format!("op {id}"), NodeId::of("op", id)));
                }
                for ((f, a), (_, p)) in &self.p.placed.names {
                    let _ = (f, a);
                    pts.push(*p);
                }
                out.extend(self.surface_at(&pts));
                out.extend(self.brought_reads());
            }
            FileReads::Instances => {
                out.extend(self.brought_reads());
                for id in self.defs.keys() {
                    // A clear reaches the instances a step returned, which no `held` node
                    // holds (`identity.md` 4.1), so the file reads every edit.
                    if matches!(id.family(), "instance" | "held" | "height")
                        || (id.family() == "edit"
                            && (self.p.decls.pins.contains_key(&subject_of(id))
                                || self.p.decls.clears.contains_key(&subject_of(id))))
                    {
                        out.push((format!("{} {}", id.family(), subject_of(id)), id.clone()));
                    }
                }
            }
            // `meets.json` (`surfaces.md` 3.3, 3.4): every path with an end at a place and every
            // feature whose kind has mounts or surfaces (shape, attributes), what each end names,
            // every instance's plan and height and the facts of assets with mounts (asset mounts,
            // 3.2), and the terrain under every end, mount and instance. Its statements are in
            // [`ModelPlan::file_statements`].
            FileReads::Meets => {
                let d = &self.p.decls;
                let mut pts: Vec<P> = Vec::new();
                for id in self.meets_features() {
                    out.push((format!("shape {id}"), view(NodeId::of("shape", &id))));
                    out.push((format!("attr {id}"), NodeId::of("attr", &id)));
                    match self.p.placed.shapes.get(&id) {
                        Some(Geo::Path { line }) => pts.extend(line.pts.iter().copied()),
                        Some(Geo::Region {
                            line: Some(line), ..
                        }) => pts.extend(line.pts.iter().copied()),
                        Some(Geo::Point { p, .. }) => pts.push(*p),
                        _ => {}
                    }
                    if let Some(f) = d.features.get(&id) {
                        for place in [&f.starts_at, &f.ends_at].into_iter().flatten() {
                            for t in self.targets(place) {
                                out.push((format!("meets {place}"), t));
                            }
                        }
                    }
                }
                for id in self.defs.keys() {
                    if matches!(id.family(), "instance" | "height") {
                        out.push((format!("{} {}", id.family(), subject_of(id)), id.clone()));
                    }
                }
                for (a, x) in &self.p.pack.assets {
                    let n = NodeId::of("facts", a);
                    if !x.mounts.is_empty() && self.defs.contains_key(&n) {
                        out.push((format!("facts {a}"), n));
                    }
                }
                // An asset mount lies off its footprint by up to a few metres: 5 m round it.
                const MOUNT_REACH: i64 = 5_000;
                for pl in self.p.placed.plans.values() {
                    let b = region_bbox(&pl.footprint, pl.origin);
                    let b = [
                        b[0] - MOUNT_REACH,
                        b[1] - MOUNT_REACH,
                        b[2] + MOUNT_REACH,
                        b[3] + MOUNT_REACH,
                    ];
                    for f in self.surface_fields() {
                        for w in self.chunks_under(b) {
                            out.push((
                                format!("surface {f} {}", win_text(&w)),
                                chunk_id(&f, BUILD, &w),
                            ));
                        }
                    }
                }
                for (_, p) in self.p.placed.names.values() {
                    pts.push(*p);
                }
                out.extend(self.surface_at(&pts));
            }
            FileReads::Everything => {
                for id in self.defs.keys() {
                    if id.family() != "source" && id.family() != "merge" {
                        out.push((format!("reads {id}"), id.clone()));
                    }
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// The statements a build file's `write` node holds in its key, beside its edges. A meeting
    /// is made by `starts_at`, `ends_at`, `meets` and the kinds' mounts (`surfaces.md` 3.3),
    /// which no node's value carries: unsetting `ends_at` leaves every shape as it was, so
    /// `meets.json` holds every statement (example `station`, `close_passage`: a warm build kept
    /// the meeting a clean one dropped).
    pub fn file_statements(&self, what: &FileReads) -> Vec<(String, Digest)> {
        match what {
            FileReads::Meets => {
                let d = &self.p.decls;
                let mut ids: Vec<String> = Vec::new();
                for id in self.meets_features() {
                    if let Some(k) = d.features.get(&id).and_then(|f| f.kind.as_ref()) {
                        ids.push(format!("kind.{k}"));
                    }
                    ids.push(format!("feature.{id}"));
                }
                ids.extend(d.instances.keys().map(|i| format!("instance.{i}")));
                self.stmts_of(ids.iter().map(String::as_str))
            }
            _ => Vec::new(),
        }
    }

    /// The features `meets.json` is made from (`surfaces.md` 3.3): every one with an end at a
    /// place, every one whose kind has mounts or surfaces, and every feature an end names.
    fn meets_features(&self) -> BTreeSet<String> {
        let d = &self.p.decls;
        let mut out = BTreeSet::new();
        for (id, f) in &d.features {
            let mounted = f
                .kind
                .as_ref()
                .and_then(|k| d.kinds.get(k))
                .is_some_and(|k| !k.mounts.is_empty() || !k.surfaces.is_empty());
            let ends = [&f.starts_at, &f.ends_at];
            if !mounted && ends.iter().all(|e| e.is_none()) {
                continue;
            }
            out.insert(id.clone());
            for place in ends.into_iter().flatten() {
                let base = place
                    .strip_suffix(".start")
                    .or_else(|| place.strip_suffix(".end"))
                    .unwrap_or(place);
                for h in [base, base.split('.').next().unwrap_or(base)] {
                    if d.features.contains_key(h) {
                        out.insert(h.to_string());
                    }
                }
            }
        }
        out
    }

    /// What brought instances and features read (`composition.md` 12: `import:<id>` is read by
    /// the write nodes of `features.json` and the instance columns): each import, its frame, and
    /// every surface chunk, since their heights are found again on the parent's surface (3).
    fn brought_reads(&self) -> Vec<(String, NodeId)> {
        let mut out = Vec::new();
        if self.p.decls.imported.is_empty() {
            return out;
        }
        for id in self.p.decls.imported.keys() {
            out.push((format!("built {id}"), NodeId::of("import", id)));
            out.push((format!("import {id}"), NodeId::of("placed", id)));
        }
        for f in self.surface_fields() {
            for (_, w) in &self.chunks {
                out.push((
                    format!("surface {f} {}", win_text(w)),
                    chunk_id(&f, BUILD, w),
                ));
            }
        }
        out
    }

    /// The surface fields' chunks that hold these points, by `build.md` 5's rule.
    fn surface_at(&self, pts: &[P]) -> Vec<(String, NodeId)> {
        let mut out = Vec::new();
        for f in self.surface_fields() {
            for p in pts {
                if let Some(w) = self.chunk_at(p.x, p.y) {
                    out.push((
                        format!("surface {f} {}", win_text(&w)),
                        chunk_id(&f, BUILD, &w),
                    ));
                }
            }
        }
        out
    }

    /// The chunk that holds a plan point (`build.md` 5), each index clamped.
    fn chunk_at(&self, x: i64, y: i64) -> Option<Window> {
        let (g, t) = (self.grid?, self.tiling?);
        let side = t.side_mm();
        let cx = (x - g.extent[0]).div_euclid(side).clamp(0, t.cx as i64 - 1) as u64;
        let cy = (y - g.extent[1]).div_euclid(side).clamp(0, t.cy as i64 - 1) as u64;
        t.window(cx, cy)
    }

    /// The chunks a rectangle of plan points meets.
    fn chunks_under(&self, b: [i64; 4]) -> Vec<Window> {
        let Some(g) = self.grid else {
            return Vec::new();
        };
        match effect_window(&g, b, 0) {
            Ok(Some(w)) => self
                .chunks
                .iter()
                .filter(|(_, c)| meets(c, &w))
                .map(|(_, c)| *c)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn surface_fields(&self) -> Vec<String> {
        self.p
            .decls
            .fields
            .values()
            .filter(|f| f.role == Some(Role::Surface))
            .map(|f| f.name.clone())
            .collect()
    }

    fn keep_out_fields(&self) -> Vec<String> {
        self.p
            .decls
            .fields
            .values()
            .filter(|f| f.role == Some(Role::KeepOut))
            .map(|f| f.name.clone())
            .collect()
    }

    fn field_sel(&self, sel: &str) -> Vec<String> {
        self.p
            .decls
            .fields
            .values()
            .filter(|f| match sel.strip_prefix("tag:") {
                Some(t) => f.tags.iter().any(|x| x == t),
                None => f.name == sel,
            })
            .map(|f| f.name.clone())
            .collect()
    }

    /// What one formula reads.
    fn freads(&self, f: Option<FormulaId>, out: &mut FReads) {
        let Some(prog) = f.and_then(|f| self.p.decls.formulas.programs.get(&f)) else {
            return;
        };
        for (class, sel) in &prog.reads.selectors {
            match class {
                Class::Field => out.fields.extend(self.field_sel(sel)),
                Class::Shape => {
                    out.shapes.insert(sel.clone());
                }
                _ => {}
            }
        }
        for (scope, name) in &prog.reads.lets {
            if scope == WORLD_LET {
                out.lets.insert(format!("let.{name}"));
            }
        }
        out.halo = out.halo.max(u64::from(prog.halo_posts));
        for n in &prog.nodes {
            match n.op {
                Op::Under { .. } => {
                    out.measures.insert("under");
                }
                Op::SmoothedAlong(_) => {
                    out.measures.insert("smoothed_along");
                }
                _ => {}
            }
        }
    }

    /// The features a shape selector names: `kind:<k>` (the made things a table gives that kind
    /// included, `geometry.md` 8.1) or a feature id.
    fn shape_members(&self, sel: &str) -> Vec<String> {
        match sel.strip_prefix("kind:") {
            Some(k) => {
                let mut out: Vec<String> = self
                    .p
                    .decls
                    .features
                    .iter()
                    .filter(|(id, f)| {
                        f.kind.as_deref() == Some(k) && self.p.placed.shapes.contains_key(*id)
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                out.extend(
                    self.p
                        .placed
                        .made_writers
                        .iter()
                        .filter(|(_, (_, mk, _))| mk == k)
                        .map(|(n, _)| n.clone()),
                );
                out.sort();
                out
            }
            None => vec![sel.to_string()],
        }
    }

    /// Each field's base and rows, and every effect on it with its window.
    fn field_infos(&self) -> BTreeMap<String, FieldInfo> {
        let d = &self.p.decls;
        // Writers: features with a kind and a shape, footprint features, and what a network or
        // cells table made with a kind (`geometry.md` 8.1).
        type W = (String, Option<String>, Option<String>, String);
        let mut writers: Vec<W> = Vec::new();
        for (id, f) in &d.features {
            if let Some(k) = &f.kind {
                if d.kinds.contains_key(k) && self.p.placed.shapes.contains_key(id) {
                    writers.push((id.clone(), None, None, k.clone()));
                }
            }
        }
        for (name, (inst, kind)) in &self.p.placed.footprints {
            if d.kinds.contains_key(kind) && self.p.placed.shapes.contains_key(name) {
                writers.push((name.clone(), Some(inst.clone()), None, kind.clone()));
            }
        }
        for (name, (maker, kind, _)) in &self.p.placed.made_writers {
            writers.push((name.clone(), None, Some(maker.clone()), kind.clone()));
        }
        let mut out = BTreeMap::new();
        for (name, fd) in &d.fields {
            let mut info = FieldInfo::default();
            self.freads(fd.base, &mut info.own);
            for r in &fd.rows {
                self.freads(r.value, &mut info.own);
                self.freads(r.mask, &mut info.own);
            }
            for (wname, inst, maker, kind) in &writers {
                let k = &d.kinds[kind];
                for e in k.effects.values() {
                    let hit = match &e.selector {
                        Selector::Role(r) => fd.role == Some(*r),
                        Selector::Tag(t) => fd.tags.iter().any(|x| x == t),
                    };
                    if !hit {
                        continue;
                    }
                    let geo = self.p.placed.shape(wname).expect("a writer's shape");
                    let window = match (self.grid, e.radius.or(k.radius)) {
                        (Some(g), Some(r)) => effect_window(&g, bbox_of(geo), r).ok().flatten(),
                        (Some(g), None) => Some(Window::whole(&g)),
                        _ => None,
                    };
                    let mut reads = FReads::default();
                    self.freads(e.value, &mut reads);
                    self.freads(e.mask, &mut reads);
                    self.freads(e.max_change, &mut reads);
                    // As rows are ordered (`authority.md` 3): a feature's own rank, a
                    // footprint's its instance's.
                    let rank = match (inst, maker) {
                        (Some(i), _) => d.placed_rank(i),
                        (None, Some(m)) => d.maker_table_rank(m).unwrap_or(0),
                        (None, None) => d.features.get(wname).map_or(0, |f| f.rank),
                    };
                    info.effs.push(Eff {
                        writer: wname.clone(),
                        instance: inst.clone(),
                        maker: maker.clone(),
                        kind: kind.clone(),
                        effect: e.id.clone(),
                        window,
                        reads,
                        rank,
                    });
                }
            }
            out.insert(name.clone(), info);
        }
        out
    }

    fn stmt(&self, s: &str) -> Option<(String, Digest)> {
        self.stmts.get(s).map(|d| (s.to_string(), *d))
    }

    fn stmts_of<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> Vec<(String, Digest)> {
        let set: BTreeSet<&str> = ids.into_iter().collect();
        set.into_iter().filter_map(|s| self.stmt(s)).collect()
    }

    /// The nodes a place name refers to (`graph.md` 2, "placement" and "mount").
    fn targets(&self, name: &str) -> Vec<NodeId> {
        let d = &self.p.decls;
        if let Some((h, rest)) = name.split_once('.') {
            if d.imports.contains_key(h) {
                return if d.imported.contains_key(h) {
                    vec![NodeId::of("placed", &format!("{h}/{rest}"))]
                } else {
                    Vec::new()
                };
            }
        }
        if let Some((h, top)) = crate::place::failed_owner(d, name) {
            return vec![failed_node(top, &h)];
        }
        let base = name
            .strip_suffix(".start")
            .or_else(|| name.strip_suffix(".end"))
            .unwrap_or(name);
        let head = base.split('.').next().unwrap_or(base);
        if let Some((producer, _)) = name.split_once('#') {
            if d.scatters.contains_key(producer) {
                return vec![view(NodeId::of("held", producer))];
            }
            if d.is_maker_table(producer) {
                return vec![NodeId::of("op", producer)];
            }
        }
        if d.is_maker_table(name) {
            return vec![NodeId::of("op", name)];
        }
        if d.features.contains_key(base) {
            return vec![view(NodeId::of("shape", base))];
        }
        if d.features.contains_key(head) {
            return vec![view(NodeId::of("shape", head))];
        }
        if d.instances.contains_key(head) {
            let mut v = vec![view(NodeId::of("instance", head))];
            if head != name {
                v.push(view(NodeId::of("height", head)));
            }
            return v;
        }
        if d.pins.contains_key(head) {
            return vec![view(NodeId::of("edit", head))];
        }
        Vec::new()
    }

    fn place_edges(&self, place: &Place, out: &mut Vec<(String, NodeId)>) {
        match place {
            Place::On(o) => {
                for t in self.targets(&o.reference) {
                    out.push(("on.ref".into(), t));
                }
            }
            // A selection by place (`identity.md` 4.1) reads what the maker made: a scatter's
            // rule run, which hand edits never change, so the edit cannot read itself.
            Place::Of(sel) => {
                let prefix = format!("{}#", sel.of);
                if self.p.decls.scatters.contains_key(&sel.of) {
                    out.push(("of".into(), view(NodeId::of("rule", &sel.of))));
                } else if self.p.decls.steps.contains_key(&sel.of) {
                    // A step's returned features (`identity.md` 2.4, class `feature`): each one
                    // the selection may land on.
                    let ids: Vec<String> = self
                        .p
                        .decls
                        .features
                        .keys()
                        .filter(|k| k.starts_with(&prefix))
                        .cloned()
                        .collect();
                    for f in ids {
                        for t in self.targets(&f) {
                            out.push(("of".into(), t));
                        }
                    }
                } else {
                    for t in self.targets(&sel.of) {
                        out.push(("of".into(), t));
                    }
                }
            }
            Place::At(_) => {}
        }
    }

    fn put(&mut self, id: NodeId, def: Def) {
        self.defs.insert(id, def);
    }

    /// Every node's definition.
    #[allow(clippy::too_many_lines)]
    fn define_all(&mut self) {
        let d = &self.p.decls;
        // Leaves and the merge.
        let mut merge_edges = Vec::new();
        let mut seen_sources = BTreeSet::new();
        for s in self.world.sources().iter() {
            if !seen_sources.insert(s.name.clone()) {
                continue;
            }
            let id = NodeId::of("source", &s.name);
            merge_edges.push((format!("source {}", s.name), id.clone()));
            let def = Def {
                params: vec![("path".into(), s.name.clone())],
                direct: vec![("bytes".into(), Digest::of(s.text().as_bytes()))],
                ..Def::default()
            };
            self.defs.insert(id, def);
        }
        let leaves = leaves_of(&self.p, &self.options);
        for (path, id, digest) in &leaves {
            // The facts file decides no instance, field or mesh (`assets.md` 4.2): only the
            // `facts` nodes read it.
            if id.family() == "source" && *path != self.p.pack.facts_display {
                merge_edges.push((format!("source {path}"), id.clone()));
            }
            self.defs.insert(
                id.clone(),
                Def {
                    params: vec![("path".into(), path.clone())],
                    direct: vec![("bytes".into(), *digest)],
                    ..Def::default()
                },
            );
        }
        // An import's own diagnostics are the merge's (`composition.md` 4): its build enters the
        // merge's key, never where it is placed.
        let merge = Def {
            params: vec![("changes".into(), self.world.changes().join("\n"))],
            edges: merge_edges,
            direct: self
                .p
                .decls
                .imported
                .iter()
                .map(|(id, imp)| {
                    (
                        format!("import {id}"),
                        Digest::of(imp.alone.digest.as_bytes()),
                    )
                })
                .collect(),
            ..Def::default()
        };
        let mut defs: Vec<(NodeId, Def)> = vec![(NodeId::parse("merge").expect("an id"), merge)];
        // Features: attributes and shapes.
        let mut head_fields: BTreeSet<String> = BTreeSet::new();
        for (id, f) in &d.features {
            let kind = f.kind.clone().map(|k| format!("kind.{k}"));
            let mut statements = self.stmts_of(
                [format!("feature.{id}"), "world".into()]
                    .iter()
                    .map(String::as_str)
                    .chain(kind.as_deref()),
            );
            statements.sort();
            let subject = Some(Subject::entity("feature", id.clone()));
            let mut shape_edges = Vec::new();
            if let ShapeDecl::Point(p) = &f.shape {
                self.place_edges(p, &mut shape_edges);
            }
            let shape_formula = match &f.shape {
                ShapeDecl::Where(w) => Some(*w),
                _ => f.route,
            };
            if let Some(w) = shape_formula {
                // A `where` region, or a `route`'s cost, reads fields in their head view over the
                // whole extent (`graph.md` 1, "Views"; `authority.md` 2.3), and the shapes it
                // names (`derived.md` 1, "Head, not effects").
                let mut r = FReads::default();
                self.freads(Some(w), &mut r);
                for fname in &r.fields {
                    for (_, c) in &self.chunks {
                        shape_edges
                            .push((format!("where reads {fname}"), chunk_id(fname, HEAD, c)));
                    }
                }
                for sel in &r.shapes {
                    for m in self.shape_members(sel) {
                        if m != *id {
                            shape_edges.push((format!("where reads {sel}"), self.member_node(&m)));
                        }
                    }
                }
                head_fields.extend(r.fields.iter().cloned());
            }
            for (key, at) in [("starts_at", &f.starts_at), ("ends_at", &f.ends_at)] {
                if let Some(at) = at {
                    for t in self.targets(at) {
                        shape_edges.push((key.to_string(), t));
                    }
                }
            }
            // A named way point (`derived.md` 3): the shape reads the place it passes.
            for at in f.way_names() {
                for t in self.targets(at) {
                    shape_edges.push(("path".to_string(), t));
                }
            }
            defs.push((
                view(NodeId::of("shape", id)),
                Def {
                    subject: subject.clone(),
                    statements: statements.clone(),
                    edges: shape_edges,
                    ..Def::default()
                },
            ));
            let mut attr_edges = Vec::new();
            if let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) {
                for n in &k.names {
                    if let Some(Attr::Text(t)) = f.attrs.get(n) {
                        for t in self.targets(t) {
                            attr_edges.push((n.clone(), t));
                        }
                    }
                }
            }
            defs.push((
                NodeId::of("attr", id),
                Def {
                    subject,
                    statements,
                    edges: attr_edges,
                    ..Def::default()
                },
            ));
        }
        // Networks and cells tables (`geometry.md` 8.3): one `op` node each, reading what its
        // keys resolve to; a selection from it reads it.
        let makers: Vec<(&'static str, String)> = d
            .networks
            .keys()
            .map(|k| ("network", k.clone()))
            .chain(d.cells.keys().map(|k| ("cells", k.clone())))
            .collect();
        for (top, id) in makers {
            let mut edges = Vec::new();
            let mut names: BTreeSet<String> = self
                .p
                .placed
                .made
                .get(&id)
                .map(|m| m.reads.clone())
                .unwrap_or_default();
            let mut kinds: Vec<String> = Vec::new();
            let mut sel_place = None;
            if let Some(n) = d.networks.get(&id) {
                names.extend(n.from.iter().filter(|s| !s.starts_with("kind:")).cloned());
                kinds.extend(
                    n.from
                        .iter()
                        .filter_map(|s| s.strip_prefix("kind:"))
                        .map(str::to_string),
                );
                names.extend(n.within.clone());
                kinds.extend(n.kind.clone());
            }
            if let Some(c) = d.cells.get(&id) {
                match &c.within {
                    crate::makers::CellsWithin::Name(n) => {
                        names.insert(n.clone());
                    }
                    crate::makers::CellsWithin::Kind(k) => {
                        kinds.push(k.clone());
                        names.extend(d.makers_of_kind(k).into_iter().filter(|m| *m != id));
                    }
                    crate::makers::CellsWithin::Sel(sel) => {
                        sel_place = Some(Place::Of(sel.clone()))
                    }
                }
                for f in c.frontage.iter().flatten() {
                    match f.strip_prefix("kind:") {
                        Some(k) => kinds.push(k.to_string()),
                        None => {
                            names.insert(f.clone());
                        }
                    }
                }
                kinds.extend(c.kind.clone());
            }
            kinds.sort();
            kinds.dedup();
            // A `kind:` selector reads every feature of that kind.
            for (fid, f) in &d.features {
                if f.kind.as_ref().is_some_and(|k| kinds.contains(k)) {
                    names.insert(fid.clone());
                }
            }
            for n in &names {
                if *n == id {
                    continue;
                }
                for t in self.targets(n) {
                    edges.push((format!("reads {n}"), t));
                }
            }
            if let Some(p) = &sel_place {
                self.place_edges(p, &mut edges);
            }
            let kind_stmts: Vec<String> = kinds.iter().map(|k| format!("kind.{k}")).collect();
            let mut statements = self.stmts_of(
                [format!("{top}.{id}"), "world".into()]
                    .iter()
                    .chain(kind_stmts.iter())
                    .map(String::as_str),
            );
            statements.sort();
            defs.push((
                NodeId::of("op", &id),
                Def {
                    subject: Some(Subject::entity(top, id.clone())),
                    statements,
                    edges,
                    ..Def::default()
                },
            ));
        }
        // Each made thing that writes fields (`geometry.md` 8.1): one `made` node reading its
        // table's `op` node, so a field chunk reads only the things whose windows meet it.
        for (name, (maker, kind, _)) in &self.p.placed.made_writers {
            let top = d.maker_top(maker).unwrap_or("network");
            defs.push((
                view(NodeId::of("made", name)),
                Def {
                    subject: Some(Subject::entity(top, maker.clone())),
                    statements: self.stmts_of(
                        [format!("{top}.{maker}"), format!("kind.{kind}")]
                            .iter()
                            .map(String::as_str),
                    ),
                    // Its name is its key's own argument: two things of one maker share every
                    // edge and statement.
                    params: vec![("name".into(), name.clone())],
                    edges: vec![("made by".into(), NodeId::of("op", maker))],
                    ..Def::default()
                },
            ));
        }
        // Hand instances, their footprints and heights.
        let hands: Vec<String> = d.instances.keys().cloned().collect();
        for (id, i) in &d.instances {
            let asset = format!("asset:{}", i.asset);
            let subject = Some(Subject::entity("instance", id.clone()));
            let mut edges = Vec::new();
            self.place_edges(&i.place, &mut edges);
            let mut stmts = vec![
                format!("instance.{id}"),
                "world".into(),
                PACK.into(),
                asset.clone(),
            ];
            stmts.extend(d.set_stmts(i.set.as_deref()));
            let statements = self.stmts_of(stmts.iter().map(String::as_str));
            defs.push((
                view(NodeId::of("instance", id)),
                Def {
                    subject: subject.clone(),
                    statements: statements.clone(),
                    edges,
                    ..Def::default()
                },
            ));
            if let Some(g) = &i.ground {
                defs.push((
                    view(NodeId::of("footprint", id)),
                    Def {
                        subject: subject.clone(),
                        statements: self.stmts_of(
                            [
                                format!("instance.{id}"),
                                "world".into(),
                                asset.clone(),
                                format!("kind.{g}"),
                            ]
                            .iter()
                            .map(String::as_str),
                        ),
                        edges: vec![("ground".into(), view(NodeId::of("instance", id)))],
                        ..Def::default()
                    },
                ));
            }
            let at = self
                .p
                .placed
                .plans
                .get(id)
                .map(|pl| region_bbox(&pl.footprint, pl.origin));
            let mut edges = vec![("plan".to_string(), view(NodeId::of("instance", id)))];
            edges.extend(self.standing_edges(at, true));
            // On a kind surface (`surfaces.md` 4.1): the thing depends on the surface's owner as
            // on an `on` reference, its shape, its attributes and its kind's statements (the
            // surface's `base`, `bound`), so a changed floor re-grounds it, and the change report
            // lists it as grounded on that owner (rule 8).
            let mut statements = statements.clone();
            if let Some(owner) = self.surface_owner(i) {
                let label = format!("grounded on {owner}");
                edges.push((label.clone(), view(NodeId::of("shape", &owner))));
                edges.push((label, NodeId::of("attr", &owner)));
                if let Some(k) = d.features.get(&owner).and_then(|f| f.kind.as_ref()) {
                    statements.extend(self.stmts_of([format!("kind.{k}").as_str()]));
                    statements.sort();
                    statements.dedup();
                }
            }
            // A hand instance and the instance it stands on are not tested for overlap
            // (`rules.md` 8.1), so neither reads the other for it: the one on a mount already
            // reads its host's `height`, and an edge back would close a loop.
            let stands_on = |x: &str, y: &str| {
                d.instances.get(x).is_some_and(|i| match &i.place {
                    Place::On(o) => o.reference.split('.').next() == Some(y),
                    _ => false,
                })
            };
            for other in &hands {
                if other != id && !stands_on(id, other) && !stands_on(other, id) {
                    edges.push((
                        format!("overlap {other}"),
                        view(NodeId::of("instance", other)),
                    ));
                }
            }
            // An instance a step returned is held by every hand edit (`identity.md` 4.1).
            if id
                .split_once('#')
                .is_some_and(|(m, _)| d.steps.contains_key(m))
            {
                for e in d.pins.keys().chain(d.clears.keys()) {
                    edges.push((format!("edit {e}"), view(NodeId::of("edit", e))));
                }
            }
            defs.push((
                view(NodeId::of("height", id)),
                Def {
                    subject,
                    statements,
                    edges,
                    ..Def::default()
                },
            ));
        }
        // A run's members' footprint features (`rules.md` 5): each reads the run and its stretch.
        for (inst, kind) in self.p.placed.footprints.values() {
            let Some(run) = d.run_of(inst).and_then(|r| d.runs.get(r).map(|x| (r, x))) else {
                continue;
            };
            let mut stmts = vec![
                format!("run.{}", run.0),
                "world".into(),
                PACK.into(),
                format!("kind.{kind}"),
            ];
            if let crate::sets::What::Set(s) = &run.1.what {
                stmts.extend(d.set_stmts(Some(s)));
            }
            let mut edges = Vec::new();
            self.place_edges(&run.1.place, &mut edges);
            defs.push((
                view(NodeId::of("footprint", inst)),
                Def {
                    subject: Some(Subject::entity("run", run.0)),
                    statements: self.stmts_of(stmts.iter().map(String::as_str)),
                    // Its member is its key's own argument: the members of one run share every
                    // statement and edge, and a key without it gave one member's footprint to
                    // all four (example `third_cottage`, `move_tarn`: `diag.internal`).
                    params: vec![("member".into(), inst.clone())],
                    edges,
                    ..Def::default()
                },
            ));
        }
        // Hand edits.
        for (id, pin) in &d.pins {
            let subject = Some(Subject::entity("pin", id.clone()));
            let statements = self.stmts_of(
                [
                    format!("pin.{id}"),
                    "world".into(),
                    PACK.into(),
                    format!("asset:{}", pin.asset),
                ]
                .iter()
                .map(String::as_str),
            );
            let mut edges = Vec::new();
            self.place_edges(&pin.place, &mut edges);
            defs.push((
                view(NodeId::of("edit", id)),
                Def {
                    subject: subject.clone(),
                    statements: statements.clone(),
                    edges,
                    ..Def::default()
                },
            ));
            let at = self
                .p
                .placed
                .pins
                .get(id)
                .map(|pl| region_bbox(&pl.footprint, pl.origin));
            let mut edges = vec![("plan".to_string(), view(NodeId::of("edit", id)))];
            edges.extend(self.standing_edges(at, false));
            defs.push((
                view(NodeId::of("height", id)),
                Def {
                    subject,
                    statements,
                    edges,
                    ..Def::default()
                },
            ));
        }
        for (id, c) in &d.clears {
            let mut edges = Vec::new();
            if let Some(p) = &c.place {
                self.place_edges(p, &mut edges);
            }
            defs.push((
                view(NodeId::of("edit", id)),
                Def {
                    subject: Some(Subject::entity("clear", id.clone())),
                    statements: self.stmts_of(
                        [format!("clear.{id}"), "world".into()]
                            .iter()
                            .map(String::as_str),
                    ),
                    edges,
                    ..Def::default()
                },
            ));
        }
        // A declared place name whose declaration failed its own check (`diagnostics.md` 4): one
        // node, failed by that check's record, so a reader of the name has an edge to it and is
        // `skipped` under the one cause, never told the name names nothing.
        for (id, top) in &d.declared {
            if crate::place::failed_owner(d, id).is_none_or(|(h, _)| h != *id) {
                continue;
            }
            defs.push((
                failed_node(top, id),
                Def {
                    subject: Some(Subject::entity(*top, id.clone())),
                    statements: self.stmts_of(
                        [format!("{top}.{id}"), "world".into()]
                            .iter()
                            .map(String::as_str),
                    ),
                    ..Def::default()
                },
            ));
        }
        // Producers.
        let assets: Vec<String> = self
            .p
            .pack
            .assets
            .keys()
            .map(|a| format!("asset:{a}"))
            .collect();
        let surface = self.surface_fields();
        let keep_out = self.keep_out_fields();
        for (id, s) in &d.scatters {
            let subject = Some(Subject::entity("scatter", id.clone()));
            let mut reads = FReads::default();
            self.freads(s.density, &mut reads);
            let mut st: Vec<String> = vec![format!("scatter.{id}"), "world".into(), PACK.into()];
            st.extend(assets.iter().cloned());
            st.extend(reads.lets.iter().cloned());
            let mut edges = vec![
                (
                    "surface".to_string(),
                    NodeId::of("select", "role:surface").view(BUILD),
                ),
                (
                    "keep_out".to_string(),
                    NodeId::of("select", "role:keep_out").view(BUILD),
                ),
            ];
            for (label, fs) in [("surface", &surface), ("keep_out", &keep_out)] {
                for f in fs {
                    for (_, w) in &self.chunks {
                        edges.push((format!("{label} {f}"), chunk_id(f, BUILD, w)));
                    }
                }
            }
            for f in &reads.fields {
                for (_, w) in &self.chunks {
                    edges.push((format!("density reads {f}"), chunk_id(f, BUILD, w)));
                }
            }
            for sel in &reads.shapes {
                for m in self.shape_members(sel) {
                    edges.push((format!("density reads {sel}"), self.member_node(&m)));
                }
            }
            for (h, i) in &d.instances {
                if self.p.pack.assets.get(&i.asset).is_none_or(|a| a.solid) {
                    edges.push((format!("gives way {h}"), view(NodeId::of("instance", h))));
                }
            }
            // `clump.around` (`rules.md` 7.3) reads the named scatter's output.
            if let Some(crate::decl::Clump::Around { of, .. }) = &s.clump {
                if d.scatters.contains_key(of) && of != id {
                    edges.push(("around".into(), view(NodeId::of("rule", of))));
                }
            }
            if let Some(w) = &s.within {
                for t in self.targets(w) {
                    edges.push(("within".into(), t));
                }
                if let Some((inst, _)) = self.p.placed.footprints.get(w) {
                    edges.push(("within".into(), view(NodeId::of("footprint", inst))));
                }
            }
            defs.push((
                view(NodeId::of("rule", id)),
                Def {
                    subject: subject.clone(),
                    statements: self.stmts_of(st.iter().map(String::as_str)),
                    edges,
                    ..Def::default()
                },
            ));
            let mut st: Vec<String> = vec![format!("scatter.{id}"), "world".into()];
            let mut edges = vec![("rule".to_string(), view(NodeId::of("rule", id)))];
            for (p, pin) in &d.pins {
                if pin.by.as_ref().is_none_or(|b| b.contains(id)) {
                    st.push(format!("pin.{p}"));
                    edges.push((format!("edit {p}"), view(NodeId::of("edit", p))));
                }
            }
            for (c, clear) in &d.clears {
                if clear.by.as_ref().is_none_or(|b| b.contains(id)) {
                    st.push(format!("clear.{c}"));
                    edges.push((format!("edit {c}"), view(NodeId::of("edit", c))));
                }
            }
            defs.push((
                view(NodeId::of("held", id)),
                Def {
                    subject: subject.clone(),
                    statements: self.stmts_of(st.iter().map(String::as_str)),
                    edges,
                    ..Def::default()
                },
            ));
            let mut st: Vec<String> = vec![format!("scatter.{id}"), "world".into()];
            st.extend(assets.iter().cloned());
            let mut edges = vec![
                ("plan".to_string(), view(NodeId::of("held", id))),
                (
                    "surface".to_string(),
                    NodeId::of("select", "role:surface").view(BUILD),
                ),
            ];
            for f in &surface {
                for (_, w) in &self.chunks {
                    edges.push((format!("surface {f}"), chunk_id(f, BUILD, w)));
                }
            }
            defs.push((
                view(NodeId::of("height", id)),
                Def {
                    subject,
                    statements: self.stmts_of(st.iter().map(String::as_str)),
                    edges,
                    ..Def::default()
                },
            ));
        }
        // Selections.
        let mut selectors: BTreeSet<String> = BTreeSet::new();
        if !d.instances.is_empty() || !d.scatters.is_empty() || !d.pins.is_empty() {
            selectors.insert("role:surface".into());
            selectors.insert("role:keep_out".into());
        }
        for info in self.fields.values() {
            for s in info
                .own
                .shapes
                .iter()
                .chain(info.effs.iter().flat_map(|e| e.reads.shapes.iter()))
            {
                if s.starts_with("kind:") {
                    selectors.insert(s.clone());
                }
            }
        }
        for sel in &selectors {
            let st: Vec<String> = if sel.starts_with("kind:") {
                d.features.keys().map(|f| format!("feature.{f}")).collect()
            } else {
                d.fields.keys().map(|f| format!("field.{f}")).collect()
            };
            defs.push((
                NodeId::of("select", sel).view(BUILD),
                Def {
                    subject: Some(Subject::Selector(sel.clone())),
                    statements: self.stmts_of(st.iter().map(String::as_str).chain(["world"])),
                    params: vec![("selector".into(), sel.clone())],
                    ..Def::default()
                },
            ));
        }
        // Fields by chunk, their distances and measures.
        let ko_under: Vec<Window> = d
            .instances
            .keys()
            .filter_map(|i| self.p.placed.plans.get(i))
            .flat_map(|pl| self.chunks_under(region_bbox(&pl.footprint, pl.origin)))
            .collect();
        // Head views: the fields a `where` reads, and those their bases and rows read.
        let mut todo: Vec<String> = head_fields.iter().cloned().collect();
        while let Some(f) = todo.pop() {
            if let Some(info) = self.fields.get(&f) {
                for r in &info.own.fields {
                    if head_fields.insert(r.clone()) {
                        todo.push(r.clone());
                    }
                }
            }
        }
        for name in &head_fields {
            if let Some(info) = self.fields.get(name) {
                for (_, w) in &self.chunks {
                    defs.push((chunk_id(name, HEAD, w), self.head_def(name, info, w)));
                }
            }
        }
        for (name, info) in &self.fields {
            let fd = &d.fields[name];
            for (_, w) in &self.chunks {
                defs.push((
                    chunk_id(name, BUILD, w),
                    self.chunk_def(name, info, w, false),
                ));
                if fd.role == Some(Role::KeepOut) && ko_under.contains(w) {
                    defs.push((chunk_id(name, NOFP, w), self.chunk_def(name, info, w, true)));
                }
            }
            for e in &info.effs {
                let Some(ew) = e.window else { continue };
                for (_, w) in &self.chunks {
                    if meets(&ew, w) {
                        defs.push((
                            dist_id(&e.writer, w),
                            Def {
                                subject: Some(e.subject(d)),
                                statements: self.stmts_of(["world"]),
                                params: vec![("window".into(), win_text(w))],
                                edges: vec![("shape".into(), e.shape_node())],
                                ..Def::default()
                            },
                        ));
                    }
                }
                for call in &e.reads.measures {
                    let mut def = self.measure_def(name, info, e);
                    // Its own arguments (3.1, item 4): which effect's call it is.
                    def.params = vec![
                        ("call".into(), (*call).to_string()),
                        ("effect".into(), format!("{}.{}", e.writer, e.effect)),
                    ];
                    defs.push((e.measure_id(name, call), def));
                }
            }
            // A selector's distance, over the chunks where something reads it: every chunk for
            // the base and rows, an effect's window for an effect.
            let mut sel_chunks: BTreeMap<&String, BTreeSet<Window>> = BTreeMap::new();
            for s in &info.own.shapes {
                sel_chunks
                    .entry(s)
                    .or_default()
                    .extend(self.chunks.iter().map(|(_, w)| *w));
            }
            for e in &info.effs {
                let Some(ew) = e.window else { continue };
                for s in &e.reads.shapes {
                    sel_chunks.entry(s).or_default().extend(
                        self.chunks
                            .iter()
                            .map(|(_, w)| *w)
                            .filter(|w| meets(w, &ew)),
                    );
                }
            }
            for (s, ws) in sel_chunks {
                for w in &ws {
                    let mut edges = Vec::new();
                    if s.starts_with("kind:") {
                        edges.push(("select".to_string(), NodeId::of("select", s).view(BUILD)));
                    }
                    for m in self.shape_members(s) {
                        edges.push((format!("member {m}"), self.member_node(&m)));
                    }
                    defs.push((
                        dist_id(s, w),
                        Def {
                            subject: Some(if s.starts_with("kind:") {
                                Subject::Selector(s.clone())
                            } else {
                                Subject::entity("feature", s.clone())
                            }),
                            statements: self.stmts_of(["world"]),
                            params: vec![("window".into(), win_text(w))],
                            edges,
                            ..Def::default()
                        },
                    ));
                }
            }
        }
        for (id, def) in defs {
            self.put(id, def);
        }
        // `graph.md` 1, `facts`: each model asset's comparison with the facts file. It reads the
        // asset's files, the facts file, the pack, and the placements that can hold that asset,
        // whose names `asset.revised` lists (`assets.md` 4.2): the hand instances and pins of the
        // asset, and the scatters that choose it (by id, or any taxon). So a re-exported model
        // re-runs it though no text changed, and a placement of another asset does not.
        let d = &self.p.decls;
        let placed_of = |aid: &str| -> Vec<NodeId> {
            self.defs
                .keys()
                .filter(|n| n.family() == "height")
                .filter(|n| {
                    let s = n.as_str();
                    let subject = s[s.find(':').map_or(0, |k| k + 1)..]
                        .split('/')
                        .next()
                        .unwrap_or("");
                    if let Some(i) = d.instances.get(subject) {
                        return i.asset == aid;
                    }
                    if let Some(p) = d.pins.get(subject) {
                        return p.asset == aid;
                    }
                    if let Some(sc) = d.scatters.get(subject) {
                        return sc
                            .choices
                            .iter()
                            .any(|c| c.what == aid || c.what.starts_with('@'));
                    }
                    true
                })
                .cloned()
                .collect()
        };
        let facts_src = NodeId::of("source", &self.p.pack.facts_display);
        // The pack file itself: a declared fact edited in its text is a revision too (4.2).
        let pack_srcs: Vec<NodeId> = self
            .defs
            .keys()
            .filter(|n| {
                n.family() == "source" && **n != facts_src && n.as_str().starts_with("source:pack:")
            })
            .cloned()
            .collect();
        let mut facts_defs = Vec::new();
        for (aid, a) in &self.p.pack.assets {
            let Some(m) = &a.model else { continue };
            let mut edges = Vec::new();
            for rel in m.files.keys() {
                let path = self.p.pack.root.pack_path(rel);
                let n = NodeId::of("input", &path);
                if self.defs.contains_key(&n) {
                    edges.push((format!("input {path}"), n));
                }
            }
            if self.defs.contains_key(&facts_src) {
                edges.push(("facts file".to_string(), facts_src.clone()));
            }
            for s in &pack_srcs {
                edges.push((format!("pack {s}"), s.clone()));
            }
            for h in placed_of(aid) {
                edges.push((format!("placed {h}"), h));
            }
            facts_defs.push((
                NodeId::of("facts", aid),
                Def {
                    subject: Some(Subject::entity("asset", aid.clone())),
                    statements: self.stmts_of([format!("asset:{aid}").as_str(), PACK]),
                    edges,
                    ..Def::default()
                },
            ));
        }
        for (id, def) in facts_defs {
            self.put(id, def);
        }
        self.define_imports();
    }

    /// The import families (`composition.md` 12): `import:<id>` (the import built alone, keyed on
    /// its own statements and `seed` and `changes`, never on where it is placed), `placed:<id>`
    /// (the frame and footprint as placed) and `placed:<id>/<name>` per scoped name the parent
    /// reads. A field chunk that brings an import's field, or reads `<import>.<field>`, reads
    /// `placed:<id>`.
    fn define_imports(&mut self) {
        let d = &self.p.decls;
        if d.imported.is_empty() {
            return;
        }
        let mut defs: Vec<(NodeId, Def)> = Vec::new();
        let ids: BTreeSet<String> = d.imported.keys().cloned().collect();
        for (id, imp) in &d.imported {
            let mut enc = Enc::new();
            enc.str("via-import-build-1")
                .str(&format!("{:?}", imp.decl.seed))
                .len(imp.decl.changes.len());
            for c in &imp.decl.changes {
                enc.str(c);
            }
            defs.push((
                NodeId::of("import", id),
                Def {
                    subject: Some(Subject::entity("import", id.clone())),
                    statements: vec![(format!("import.{id}.build"), Digest::of(&enc.finish()))],
                    direct: vec![("import".into(), Digest::of(imp.alone.digest.as_bytes()))],
                    ..Def::default()
                },
            ));
            // The place `on` names, and the parent's region `footprint` names (a name of the
            // import itself is in its digest).
            let mut edges = vec![("import".into(), NodeId::of("import", id))];
            if let Some(Place::On(o)) = &imp.decl.place {
                for t in self.targets(&o.reference) {
                    edges.push(("on.ref".into(), t));
                }
            }
            if let Some(crate::import::Foot::Name(n)) = &imp.decl.footprint {
                if !n.starts_with(&format!("{id}.")) {
                    for t in self.targets(n) {
                        edges.push(("footprint".into(), t));
                    }
                }
            }
            defs.push((
                NodeId::of("placed", id),
                Def {
                    subject: Some(Subject::entity("import", id.clone())),
                    statements: self.stmts_of([format!("import.{id}").as_str(), "world"]),
                    edges,
                    ..Def::default()
                },
            ));
        }
        for name in crate::import::scoped_refs(self.world.root(), &ids) {
            let Some((id, rest)) = name.split_once('.') else {
                continue;
            };
            defs.push((
                NodeId::of("placed", &format!("{id}/{rest}")),
                Def {
                    subject: Some(Subject::entity("import", id.to_string())),
                    params: vec![("name".into(), rest.to_string())],
                    edges: vec![
                        ("frame".into(), NodeId::of("placed", id)),
                        ("import".into(), NodeId::of("import", id)),
                    ],
                    ..Def::default()
                },
            ));
        }
        // Field chunks that bring or read an import.
        let mut reads: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (fname, fd) in &d.fields {
            let mut set = BTreeSet::new();
            for (id, imp) in &d.imported {
                if imp.alone.decls.fields.contains_key(fname) {
                    set.insert(id.clone());
                }
            }
            let formulas = fd.base.iter().chain(
                fd.rows
                    .iter()
                    .flat_map(|r| r.value.iter().chain(r.mask.iter())),
            );
            for f in formulas {
                if let Some(p) = d.formulas.programs.get(f) {
                    for (class, sel) in &p.reads.selectors {
                        if *class == Class::Field {
                            if let Some((h, _)) = sel.split_once('.') {
                                if ids.contains(h) {
                                    set.insert(h.to_string());
                                }
                            }
                        }
                    }
                }
            }
            // A blend reads what it names; every brought field reads the blend.
            if !set.is_empty() {
                reads.insert(fname.clone(), set);
            }
        }
        let join: BTreeMap<String, (String, Digest)> = ids
            .iter()
            .filter_map(|id| self.stmt(&format!("import.{id}")).map(|x| (id.clone(), x)))
            .collect();
        // What each blend reads (6.3): parent fields and `let` names, as a base reads them.
        let blends: BTreeMap<String, FReads> = d
            .imported
            .iter()
            .filter_map(|(id, imp)| {
                let b = imp.decl.blend?;
                let mut r = FReads::default();
                self.freads(Some(b), &mut r);
                Some((id.clone(), r))
            })
            .collect();
        let lets: BTreeMap<String, Vec<(String, Digest)>> = blends
            .iter()
            .map(|(id, r)| (id.clone(), self.stmts_of(r.lets.iter().map(String::as_str))))
            .collect();
        let grid = self.grid;
        // Each field's chunks, with view and window, for the blend's reads.
        let mut chunks_of: BTreeMap<String, Vec<(NodeId, Window)>> = BTreeMap::new();
        for n in self.defs.keys().filter(|n| n.family() == "field") {
            let rest = n.as_str().strip_prefix("field:").unwrap_or_default();
            let f = rest.split('/').next().unwrap_or(rest);
            if let Some(w) = window_of(n) {
                chunks_of
                    .entry(f.to_string())
                    .or_default()
                    .push((n.clone(), w));
            }
        }
        for (nid, def) in self.defs.iter_mut() {
            let Some(rest) = nid.as_str().strip_prefix("field:") else {
                continue;
            };
            let fname = rest.split('/').next().unwrap_or(rest);
            if let Some(set) = reads.get(fname) {
                for id in set {
                    // The blend's reads: each read field's chunks over this chunk's window (grown
                    // by the blend's halo), in this chunk's view; its `let` names as statements.
                    if let (Some(r), Some(w), Some(g)) = (blends.get(id), window_of(nid), grid) {
                        let grown = w.grow(r.halo, &g);
                        let v = nid.view_name().unwrap_or(BUILD);
                        for f in r.fields.iter().filter(|f| *f != fname) {
                            for (c, cw) in chunks_of.get(f).into_iter().flatten() {
                                if c.view_name() == Some(v) && meets(cw, &grown) {
                                    def.edges.push((format!("blend {id} reads {f}"), c.clone()));
                                }
                            }
                        }
                        def.statements
                            .extend(lets.get(id).into_iter().flatten().cloned());
                    }
                    // The frame, what the import built, and the join's keys (`blend`, `lift`,
                    // `authority`): the import's whole table is a statement of the chunk.
                    def.edges
                        .push((format!("import {id}"), NodeId::of("placed", id)));
                    def.edges
                        .push((format!("built {id}"), NodeId::of("import", id)));
                    if let Some(x) = join.get(id) {
                        def.statements.push(x.clone());
                    }
                }
            }
        }
        let jc_direct: Vec<(String, Digest)> = d
            .imported
            .iter()
            .map(|(id, imp)| {
                (
                    format!("import {id}"),
                    Digest::of(imp.alone.digest.as_bytes()),
                )
            })
            .collect();
        for (id, def) in defs {
            self.put(id, def);
        }
        // The join's checks (`import.seam`, `import.place_blended`, the exits) read the finished
        // surface, the inputs and the features' attributes. Until section 12's `joincheck` nodes
        // per import and window exist, one node holds them all and reads every node the build
        // makes, so a warm build reports what a fresh one does; it is skipped, as any reader
        // is, when one of them fails (`owners` routes them here).
        let all: Vec<(String, NodeId)> = self
            .defs
            .keys()
            .filter(|n| !matches!(n.family(), "source" | "merge" | "write"))
            .map(|n| (format!("join reads {n}"), n.clone()))
            .collect();
        // Its statements: each import's table (`max_grade`, `blend`, `authority`) and the world.
        let jc_stmts = self.stmts_of(
            ids.iter()
                .map(|id| format!("import.{id}"))
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str)
                .chain(["world"]),
        );
        self.put(
            Self::joincheck_id(),
            Def {
                statements: jc_stmts,
                edges: all,
                direct: jc_direct,
                ..Def::default()
            },
        );
    }

    fn joincheck_id() -> NodeId {
        NodeId::of("joincheck", "imports")
    }

    /// The feature that owns the kind surface a hand instance stands on, if any (`surfaces.md`
    /// 4.1): the head of its `surface`, the owner of the kind mount its `on` names when that mount
    /// lies on a surface, or the feature its `on` names when that feature's kind has surfaces.
    fn surface_owner(&self, i: &crate::decl::InstanceDecl) -> Option<String> {
        self.surface_owner_at(i, 0)
    }

    /// [`ModelPlan::surface_owner`], following `on` an asset mount to its instance (3.2).
    fn surface_owner_at(&self, i: &crate::decl::InstanceDecl, depth: u8) -> Option<String> {
        let d = &self.p.decls;
        let has_surfaces = |f: &str| {
            d.features
                .get(f)
                .and_then(|x| x.kind.as_ref())
                .and_then(|k| d.kinds.get(k))
                .is_some_and(|k| !k.surfaces.is_empty())
        };
        if let Some(sn) = &i.surface {
            let head = sn.split('.').next().unwrap_or(sn);
            return d.features.contains_key(head).then(|| head.to_string());
        }
        let Place::On(o) = &i.place else { return None };
        if let Some((owner, m)) = crate::surfaces::kind_mount(d, &o.reference) {
            return m.surface.as_ref().map(|_| owner.to_string());
        }
        if has_surfaces(&o.reference) {
            return Some(o.reference.clone());
        }
        let host = o.reference.split_once('.').map(|(h, _)| h)?;
        if depth >= 16 {
            return None;
        }
        self.surface_owner_at(d.instances.get(host)?, depth + 1)
    }

    /// The edges of a height: the surface beneath, and for a hand instance the `keep_out`
    /// fields beneath in view `build-nofootprint` (R25, `rules.md` 8, `graph.md` 2 "standing").
    fn standing_edges(&self, at: Option<[i64; 4]>, keep_out: bool) -> Vec<(String, NodeId)> {
        let mut edges = vec![(
            "surface".to_string(),
            NodeId::of("select", "role:surface").view(BUILD),
        )];
        let under = at.map(|b| self.chunks_under(b)).unwrap_or_default();
        for f in self.surface_fields() {
            for w in &under {
                edges.push((format!("surface {f}"), chunk_id(&f, BUILD, w)));
            }
        }
        if keep_out {
            edges.push((
                "keep_out".to_string(),
                NodeId::of("select", "role:keep_out").view(BUILD),
            ));
            for f in self.keep_out_fields() {
                for w in &under {
                    edges.push((format!("keep_out {f}"), chunk_id(&f, NOFP, w)));
                }
            }
        }
        edges
    }

    /// A field's node over one chunk (`graph.md` 2: formula reads, effects, measures).
    fn chunk_def(&self, name: &str, info: &FieldInfo, w: &Window, nofp: bool) -> Def {
        let mut st: Vec<String> = vec![format!("field.{name}"), "world".into()];
        st.extend(info.own.lets.iter().cloned());
        let mut edges: Vec<(String, NodeId)> = Vec::new();
        let grid = self.grid.expect("chunks only with a grid");
        // Reads of other fields, in the build's view, over the chunks the window grown by the
        // halo meets; with no halo, the chunk of the same window, which holds every post read
        // (a neighbour shares only posts this chunk holds too, `build.md` 5).
        let reads = |r: &FReads, label: &str, edges: &mut Vec<(String, NodeId)>| {
            let grown = w.grow(r.halo, &grid);
            for f in &r.fields {
                if f == name {
                    continue;
                }
                for (_, c) in &self.chunks {
                    let hit = if r.halo == 0 {
                        c == w
                    } else {
                        meets(c, &grown)
                    };
                    if hit {
                        edges.push((format!("{label} reads {f}"), chunk_id(f, BUILD, c)));
                    }
                }
            }
            for s in &r.shapes {
                edges.push((format!("{label} reads {s}"), dist_id(s, w)));
                if s.starts_with("kind:") {
                    edges.push((
                        format!("{label} reads {s}"),
                        NodeId::of("select", s).view(BUILD),
                    ));
                }
            }
        };
        reads(&info.own, "base", &mut edges);
        if let Some(f) = self
            .p
            .decls
            .fields
            .get(name)
            .and_then(|fd| fd.file.as_ref())
        {
            edges.push((
                "file".into(),
                NodeId::of("input", &input_path(&f.source, &f.written)),
            ));
        }
        let mut ranks: Vec<(String, String)> = Vec::new();
        for e in &info.effs {
            if nofp && e.instance.is_some() {
                continue;
            }
            let Some(ew) = e.window else { continue };
            if !meets(&ew, w) {
                continue;
            }
            st.push(format!("kind.{}", e.kind));
            st.extend(e.reads.lets.iter().cloned());
            let label = e.label();
            // Layer rank orders the rows (R29): moving a writer to a layer of another rank
            // re-runs the chunk though its shape and attributes are unchanged (`graph.md` 1).
            ranks.push((
                format!("rank {}.{}", e.writer, e.effect),
                e.rank.to_string(),
            ));
            edges.push((label.clone(), dist_id(&e.writer, w)));
            edges.push((label.clone(), e.attr_node()));
            reads(&e.reads, &label, &mut edges);
            for call in &e.reads.measures {
                edges.push((
                    format!("measure {}.{}", e.writer, e.effect),
                    e.measure_id(name, call),
                ));
            }
        }
        let mut params = vec![("window".into(), win_text(w))];
        params.extend(ranks);
        Def {
            subject: Some(Subject::entity("field", name.to_string())),
            statements: self.stmts_of(st.iter().map(String::as_str)),
            params,
            edges,
            reports: if nofp {
                Reports::ErrorsOnly
            } else {
                Reports::All
            },
            ..Def::default()
        }
    }

    /// A field's node over one chunk in the head view: its base and rows, reading other fields
    /// in the head view too (`authority.md` 2.3).
    fn head_def(&self, name: &str, info: &FieldInfo, w: &Window) -> Def {
        let grid = self.grid.expect("chunks only with a grid");
        let mut st: Vec<String> = vec![format!("field.{name}"), "world".into()];
        st.extend(info.own.lets.iter().cloned());
        let mut edges: Vec<(String, NodeId)> = Vec::new();
        let grown = w.grow(info.own.halo, &grid);
        for f in &info.own.fields {
            if f == name {
                continue;
            }
            for (_, c) in &self.chunks {
                let hit = if info.own.halo == 0 {
                    c == w
                } else {
                    meets(c, &grown)
                };
                if hit {
                    edges.push((format!("base reads {f}"), chunk_id(f, HEAD, c)));
                }
            }
        }
        for s in &info.own.shapes {
            for m in self.shape_members(s) {
                edges.push((format!("base reads {s}"), self.member_node(&m)));
            }
        }
        if let Some(f) = self
            .p
            .decls
            .fields
            .get(name)
            .and_then(|fd| fd.file.as_ref())
        {
            edges.push((
                "file".into(),
                NodeId::of("input", &input_path(&f.source, &f.written)),
            ));
        }
        Def {
            subject: Some(Subject::entity("field", name.to_string())),
            statements: self.stmts_of(st.iter().map(String::as_str)),
            params: vec![("window".into(), win_text(w))],
            edges,
            reports: Reports::ErrorsOnly,
            ..Def::default()
        }
    }

    /// A measure (`under`, `smoothed_along`) reads the entries before its own over its
    /// writer's window (`graph.md` 2, "measure"). The order of entries is not known before the
    /// field resolves, so it reads every other entry that can reach that window: each writer
    /// whose window meets it, and, through those that measure too, theirs.
    fn measure_def(&self, name: &str, info: &FieldInfo, e: &Eff) -> Def {
        let grid = self.grid.expect("measures only with a grid");
        let mut members: Vec<&Eff> = vec![e];
        let mut frontier: Vec<Window> = e.window.into_iter().collect();
        let mut done = false;
        while !done {
            done = true;
            for o in &info.effs {
                if members
                    .iter()
                    .any(|m| m.writer == o.writer && m.effect == o.effect)
                {
                    continue;
                }
                let Some(ow) = o.window else { continue };
                if frontier.iter().any(|f| meets(f, &ow)) {
                    members.push(o);
                    if !o.reads.measures.is_empty() {
                        frontier.push(ow);
                    }
                    done = false;
                }
            }
        }
        let mut st: Vec<String> = vec![format!("field.{name}"), "world".into()];
        st.extend(info.own.lets.iter().cloned());
        let mut edges: Vec<(String, NodeId)> = Vec::new();
        // The entries before this one are found by row order, of which layer rank is a key
        // (R29, `graph.md` 1).
        let mut params: Vec<(String, String)> = Vec::new();
        for m in &members {
            st.push(format!("kind.{}", m.kind));
            st.extend(m.reads.lets.iter().cloned());
            params.push((
                format!("rank {}.{}", m.writer, m.effect),
                m.rank.to_string(),
            ));
            edges.push((format!("entry {}.{}", m.writer, m.effect), m.shape_node()));
            edges.push((format!("entry {}.{}", m.writer, m.effect), m.attr_node()));
            for s in &m.reads.shapes {
                for f in self.shape_members(s) {
                    edges.push((
                        format!("entry {}.{} reads {s}", m.writer, m.effect),
                        self.member_node(&f),
                    ));
                }
            }
        }
        let mut all = info.own.clone();
        for m in &members {
            all.add(&m.reads);
        }
        for f in &all.fields {
            if f == name {
                continue;
            }
            for fw in &frontier {
                let grown = fw.grow(all.halo, &grid);
                for (_, c) in &self.chunks {
                    if meets(c, &grown) {
                        edges.push((format!("reads {f}"), chunk_id(f, BUILD, c)));
                    }
                }
            }
        }
        edges.sort();
        edges.dedup();
        Def {
            subject: Some(e.subject(&self.p.decls)),
            statements: self.stmts_of(st.iter().map(String::as_str)),
            params,
            edges,
            ..Def::default()
        }
    }

    /// The node that makes a selector member's shape: an instance's footprint (`<id>.ground`,
    /// placed or not), else the feature's shape.
    fn member_node(&self, m: &str) -> NodeId {
        // A scoped place reads its own placed node (`composition.md` 12).
        if let Some((h, rest)) = m.split_once('.') {
            if self.p.decls.imported.contains_key(h) {
                return NodeId::of("placed", &format!("{h}/{rest}"));
            }
        }
        if self.p.placed.made_writers.contains_key(m) {
            return view(NodeId::of("made", m));
        }
        if let Some((inst, _)) = self.p.placed.footprints.get(m) {
            return view(NodeId::of("footprint", inst));
        }
        if let Some(inst) = m.strip_suffix(".ground") {
            if self.p.decls.instances.contains_key(inst) {
                return view(NodeId::of("footprint", inst));
            }
        }
        view(NodeId::of("shape", m))
    }

    /// The one cycle report (`graph.md` 2.2): the reference graph is the node graph with view,
    /// window and item removed, each vertex belonging to its node's statement. Each loop is one
    /// `graph.cycle`, named from its first member in source order and located at the key that
    /// makes that member's edge; it fails every node of every member of its component, so their
    /// readers are skipped. The per-mechanism loop messages found while placing or resolving
    /// (`cycles`) give way to it for the members it covers.
    fn report_cycles(&mut self) {
        let mut g = via_graph::RefGraph::new();
        let mut members: BTreeSet<String> = BTreeSet::new();
        for (id, def) in &self.defs {
            let vertex = id.vertex();
            let member = match &def.subject {
                Some(
                    s @ Subject::Entity {
                        collection,
                        id: sid,
                        ..
                    },
                ) => {
                    let m = format!("{collection}.{sid}");
                    if members.insert(m.clone()) {
                        let order = match self.world.key_location(s) {
                            Some(via_diag::Location::Span { file, line, .. }) => {
                                via_graph::SourceOrder {
                                    rank: self.world.rank(collection, sid),
                                    file,
                                    line,
                                }
                            }
                            _ => via_graph::SourceOrder {
                                rank: None,
                                file: String::new(),
                                line: u32::MAX,
                            },
                        };
                        g.member(
                            m.clone(),
                            via_graph::Member {
                                subject: Subject::entity(collection.clone(), sid.clone()),
                                order,
                            },
                        );
                    }
                    m
                }
                _ => {
                    // A node of no statement (a selector, `merge`) is its own member, after
                    // every statement in source order.
                    if members.insert(vertex.clone()) {
                        g.member(
                            vertex.clone(),
                            via_graph::Member {
                                subject: Subject::Selector(vertex.clone()),
                                order: via_graph::SourceOrder {
                                    rank: None,
                                    file: "\u{10ffff}".into(),
                                    line: u32::MAX,
                                },
                            },
                        );
                    }
                    vertex.clone()
                }
            };
            g.vertex(vertex.clone(), member);
            for (label, t) in &def.edges {
                g.edge(via_graph::RefEdge {
                    from: vertex.clone(),
                    to: t.vertex(),
                    label: label.clone(),
                    key: edge_key(label),
                });
            }
        }
        let cycles = g.cycles();
        if cycles.is_empty() {
            return;
        }
        let mut covered: BTreeSet<String> = BTreeSet::new();
        for c in &cycles {
            covered.extend(c.members.iter().cloned());
            for (id, def) in &mut self.defs {
                if c.vertices.contains(&id.vertex()) {
                    def.found.push(c.diag.clone());
                }
            }
        }
        // The loop messages of the passes, for members the report covers, are dropped.
        let superseded = |d: &Diag| {
            *d.code() == via_diag::DiagCode::Core(codes::graph::CYCLE)
                && matches!(d.subject(), Subject::Entity { collection, id, .. }
                    if covered.contains(&format!("{collection}.{id}")))
                && !cycles.iter().any(|c| c.diag == *d)
        };
        self.p.diags.retain(|d| !superseded(d));
        self.superseded = covered;
    }

    /// A pass's own loop message about a member the reference graph's report covers.
    fn is_superseded(&self, d: &Diag) -> bool {
        *d.code() == via_diag::DiagCode::Core(codes::graph::CYCLE)
            && d.related_list().is_empty()
            && matches!(d.subject(), Subject::Entity { collection, id, .. }
                if self.superseded.contains(&format!("{collection}.{id}")))
    }

    /// Edges that would close a loop are dropped: the model reports every such loop itself
    /// (`graph.cycle`), and the nodes in it compute alone.
    fn break_cycles(&mut self) {
        let ids: Vec<NodeId> = self.defs.keys().cloned().collect();
        let index: BTreeMap<&NodeId, usize> = ids.iter().enumerate().map(|(i, n)| (n, i)).collect();
        let succ: Vec<Vec<usize>> = ids
            .iter()
            .map(|n| {
                self.defs[n]
                    .edges
                    .iter()
                    .filter_map(|(_, t)| index.get(t).copied())
                    .collect()
            })
            .collect();
        let comp = tarjan(&succ);
        for (i, n) in ids.iter().enumerate() {
            let c = comp[i];
            if let Some(def) = self.defs.get_mut(n) {
                def.edges
                    .retain(|(_, t)| index.get(t).is_none_or(|&j| comp[j] != c));
            }
        }
    }

    /// The read and placement stage's diagnostics, each given to its node. Errors of the static
    /// checks (declarations, formulas, the pack: `diagnostics.md` 4, stage 2) fail their node
    /// before anything runs (`Def::found`), so they are reported whatever else failed; the
    /// placement pass's go into the value of the node that places.
    fn route_early(&mut self) -> BTreeMap<NodeId, Vec<Diag>> {
        let mut placing = match &self.p.placing {
            Some(d) => d.clone(),
            None => Placer::new(&self.p.decls, &self.p.pack).run().1,
        };
        placing.retain(|d| !self.is_superseded(d));
        let fixed = minus(&self.p.diags, &placing);
        let mut out: BTreeMap<NodeId, Vec<Diag>> = BTreeMap::new();
        let mut found: Vec<(NodeId, Diag)> = Vec::new();
        for d in &fixed {
            for o in self.owners(d, false) {
                if d.severity() == via_diag::Severity::Error && o.as_str() != "merge" {
                    found.push((o, d.clone()));
                } else {
                    out.entry(o).or_default().push(d.clone());
                }
            }
        }
        for d in &placing {
            for o in self.owners(d, false) {
                out.entry(o).or_default().push(d.clone());
            }
        }
        for (o, d) in found {
            if let Some(def) = self.defs.get_mut(&o) {
                def.found.push(d);
            }
        }
        out
    }

    fn merge_id() -> NodeId {
        NodeId::parse("merge").expect("an id")
    }

    /// The nodes a statement's diagnostic belongs to; `late` for the producers' and checks'
    /// stage.
    fn owners(&self, d: &Diag, late: bool) -> Vec<NodeId> {
        // What a pin removed is found by the scatter's held run, so it belongs to that node,
        // whose key holds the run; the pin's own nodes do not (their value is the same either way).
        if *d.code() == via_diag::DiagCode::Core(codes::identity::PIN_REMOVED) {
            if let Subject::Entity { collection, id, .. } = d.subject() {
                if let Some(pin) = self.p.decls.pins.get(id).filter(|_| collection == "pin") {
                    let held: Vec<NodeId> = self
                        .p
                        .decls
                        .scatters
                        .keys()
                        .filter(|s| pin.by.as_ref().is_none_or(|b| b.contains(*s)))
                        .map(|s| view(NodeId::of("held", s)))
                        .filter(|n| self.defs.contains_key(n))
                        .collect();
                    if !held.is_empty() {
                        return held;
                    }
                }
            }
        }
        // What the facts file decides alone (`assets.md` 4.2, 8.1) belongs to the comparison,
        // which reads that file; the merge does not (`graph.md` 1, `facts`).
        if [
            codes::pack::UNACCEPTED,
            codes::pack::STALE_FACTS,
            codes::pack::BAD_FACTS,
        ]
        .iter()
        .any(|c| *d.code() == via_diag::DiagCode::Core(*c))
        {
            if let Some(f) = self.defs.keys().find(|n| n.family() == "facts") {
                return vec![f.clone()];
            }
        }
        // The join's checks of the finished build (`composition.md` 6.6, 6.7, 7).
        if late
            && [
                codes::import::SEAM,
                codes::import::PLACE_BLENDED,
                codes::import::EXIT_UNANSWERED,
                codes::import::EXIT_MISMATCH,
                codes::import::EXIT_OUTSIDE,
            ]
            .iter()
            .any(|c| *d.code() == via_diag::DiagCode::Core(*c))
            && self.defs.contains_key(&Self::joincheck_id())
        {
            return vec![Self::joincheck_id()];
        }
        let mut v: Vec<NodeId> = match d.subject() {
            Subject::Entity {
                collection,
                id,
                keys,
            } => {
                let key = keys.first().cloned().or_else(|| match d.site() {
                    Site::Key(p) | Site::Expr(p, _) | Site::Selector(p) => {
                        p.parts().first().cloned()
                    }
                    _ => None,
                });
                let key = match key {
                    Some(KeyPart::Key(k)) => Some(k),
                    _ => None,
                };
                match collection.as_str() {
                    "feature" => {
                        let placing = key.as_deref().is_none_or(|k| {
                            matches!(
                                k,
                                "on" | "at"
                                    | "path"
                                    | "region"
                                    | "starts_at"
                                    | "ends_at"
                                    | "follows"
                                    | "where"
                                    | "smooth"
                                    | "face"
                                    | "kind"
                                    | "authority"
                            )
                        });
                        if late {
                            vec![Self::merge_id()]
                        } else if placing || !self.p.decls.features.contains_key(id) {
                            vec![view(NodeId::of("shape", id))]
                        } else {
                            vec![NodeId::of("attr", id)]
                        }
                    }
                    // A meeting's check reads every path that ends at the mount (`surfaces.md`
                    // 3.4), which the instance's height does not: the merge holds it.
                    "instance"
                        if late
                            && [
                                codes::surface::UNMET,
                                codes::surface::CONNECTOR,
                                codes::surface::GAP,
                                codes::surface::FACING,
                                codes::surface::APART,
                            ]
                            .iter()
                            .any(|c| *d.code() == via_diag::DiagCode::Core(*c)) =>
                    {
                        vec![Self::merge_id()]
                    }
                    "instance" if late => vec![view(NodeId::of("height", id))],
                    "instance" => vec![view(NodeId::of("instance", id))],
                    "pin" if late => vec![view(NodeId::of("height", id))],
                    "pin" | "clear" if !late => vec![view(NodeId::of("edit", id))],
                    "scatter" => vec![view(NodeId::of("held", id))],
                    "network" | "cells" => vec![NodeId::of("op", id)],
                    "asset" if late => vec![NodeId::of("facts", id)],
                    "field" if !late => self
                        .chunks
                        .iter()
                        .map(|(_, w)| chunk_id(id, BUILD, w))
                        .collect(),
                    _ => vec![Self::merge_id()],
                }
            }
            _ => vec![Self::merge_id()],
        };
        v.retain(|n| self.defs.contains_key(n));
        if v.is_empty() {
            v.push(Self::merge_id());
        }
        v
    }

    /// Fields over the whole extent, made once; each field's diagnostics given to its chunks.
    fn fields_memo(&self) -> Arc<FieldsMemo> {
        let mut slot = self
            .field_memo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(m) = slot.as_ref() {
            return m.clone();
        }
        let t0 = std::time::Instant::now();
        let mut memo = FieldsMemo {
            build: BTreeMap::new(),
            head: BTreeMap::new(),
            nofp: BTreeMap::new(),
            routed: BTreeMap::new(),
        };
        if let Some(grid) = self.grid {
            let fields = Fields::new(&self.p.decls, &self.p.placed, grid);
            // Each field's records by index, so one a later field reached again (a `let` both
            // read) goes, in its final form, to every reader.
            let mut routed: BTreeMap<NodeId, BTreeSet<usize>> = BTreeMap::new();
            for name in self.field_order() {
                let before = fields.diags.borrow().len();
                let touched_before = fields.touched.borrow().len();
                let r = fields.get(&name, View::Build);
                let after = fields.diags.borrow().len();
                let mut found: BTreeSet<usize> = (before..after).collect();
                found.extend(fields.touched.borrow()[touched_before..].iter().copied());
                memo.build.insert(name.clone(), r.map(|r| (*r).clone()));
                for i in found {
                    let d = fields.diags.borrow()[i].clone();
                    if self.is_superseded(&d) {
                        continue;
                    }
                    for o in self.field_owners(&name, &d) {
                        routed.entry(o).or_default().insert(i);
                    }
                }
            }
            {
                let all = fields.diags.borrow();
                for (o, idx) in routed {
                    memo.routed
                        .insert(o, idx.into_iter().map(|i| all[i].clone()).collect());
                }
            }
            for name in self.fields.keys() {
                if self
                    .defs
                    .contains_key(&chunk_id(name, HEAD, &self.chunks[0].1))
                {
                    let r = fields.get(name, View::Head);
                    memo.head.insert(name.clone(), r.map(|r| (*r).clone()));
                }
            }
            for name in self.keep_out_fields() {
                let r = fields.get(&name, View::NoFootprint);
                memo.nofp.insert(name, r.map(|r| (*r).clone()));
            }
        }
        let m = Arc::new(memo);
        *slot = Some(m.clone());
        memo_spent(t0);
        m
    }

    /// A field's diagnostic: to the chunk that holds its position, else to every chunk.
    fn field_owners(&self, name: &str, d: &Diag) -> Vec<NodeId> {
        if let Some(p) = d.position() {
            if let Some(w) = self.chunk_at(p.x_mm(), p.y_mm()) {
                return vec![chunk_id(name, BUILD, &w)];
            }
        }
        self.chunks
            .iter()
            .map(|(_, w)| chunk_id(name, BUILD, w))
            .collect()
    }

    /// Fields in an order where each comes after the fields it reads, ties by name.
    fn field_order(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut placed: BTreeSet<String> = BTreeSet::new();
        let reads = |n: &str| -> BTreeSet<String> {
            let mut r = BTreeSet::new();
            if let Some(i) = self.fields.get(n) {
                r.extend(i.own.fields.iter().cloned());
                for e in &i.effs {
                    r.extend(e.reads.fields.iter().cloned());
                }
            }
            r.remove(n);
            r
        };
        loop {
            let ready: Option<String> = self
                .fields
                .keys()
                .find(|n| {
                    !placed.contains(*n)
                        && reads(n)
                            .iter()
                            .all(|r| placed.contains(r) || !self.fields.contains_key(r))
                })
                .cloned()
                .or_else(|| self.fields.keys().find(|n| !placed.contains(*n)).cloned());
            match ready {
                Some(n) => {
                    placed.insert(n.clone());
                    out.push(n);
                }
                None => break,
            }
        }
        out
    }

    /// The whole build, made once from the fields above.
    fn whole(&self) -> Arc<Whole> {
        let mut slot = self
            .whole
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(w) = slot.as_ref() {
            return w.clone();
        }
        let fm = self.fields_memo();
        let t0 = std::time::Instant::now();
        let seeds: BTreeMap<String, Option<Resolved>> = fm.build.clone();
        // The plan's own read and placement pass: the same text and options, so the same result,
        // taken rather than made again.
        let early = self.p.diags.clone();
        let build = finish_from(
            self.world,
            &self.p,
            seeds,
            Vec::new(),
            self.baseline.as_ref(),
        );
        let field_diags: Vec<Diag> = fm.routed.values().flatten().cloned().collect();
        let mut late = minus(&build.diagnostics, &early);
        late = minus(&late, &dedupe(field_diags));
        late.retain(|d| *d.code() != via_diag::DiagCode::Core(codes::identity::NO_PREVIOUS));
        let mut routed: BTreeMap<NodeId, Vec<Diag>> = BTreeMap::new();
        for d in late {
            for o in self.owners(&d, true) {
                routed.entry(o).or_default().push(d.clone());
            }
        }
        let w = Arc::new(Whole { build, routed });
        *slot = Some(w.clone());
        memo_spent(t0);
        w
    }

    /// Whether `field`'s base or rows read, directly or through the fields they read, a shape
    /// selector one of whose members stands outside the view of a generate step of layer rank
    /// `rank`: a step's return or a maker table's made thing at `rank` or above
    /// (`Decls::stands_returned`, `geometry.md` 8.1). Then the build's values are not the step's.
    pub fn reads_returned_shape(&self, field: &str, rank: i64) -> bool {
        let d = &self.p.decls;
        let placed = &self.p.placed;
        let Some(fd) = d.fields.get(field) else {
            return false;
        };
        let mut shapes = BTreeSet::new();
        let formulas = fd.base.into_iter().chain(
            fd.rows
                .iter()
                .flat_map(|r| r.value.into_iter().chain(r.mask)),
        );
        for f in formulas {
            shapes.extend(d.where_reads(f).1);
        }
        let out = |name: &str, r: i64| r >= rank && d.stands_returned(name);
        shapes.iter().any(|sel| match sel.strip_prefix("kind:") {
            Some(k) => {
                placed.made_writers.iter().any(|(n, (m, kind, _))| {
                    kind == k && out(n, d.maker_table_rank(m).unwrap_or(0))
                }) || d
                    .features
                    .iter()
                    .any(|(n, f)| f.kind.as_deref() == Some(k) && out(n, f.rank))
                    || placed
                        .footprints
                        .values()
                        .any(|(inst, kind)| kind == k && out(inst, d.placed_rank(inst)))
            }
            None => {
                let base = sel.split('.').next().unwrap_or(sel);
                d.stands_returned(base)
            }
        })
    }

    /// A scalar field's values in the head view of a generate step of layer rank `rank`
    /// ([`View::Stratum`]), post order; `None` for a weights field or one that does not resolve.
    pub fn stratum_values(&self, field: &str, rank: i64) -> Option<Vec<f32>> {
        let key = (field.to_string(), rank);
        if let Some(v) = self
            .stratum
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return v.clone();
        }
        let grid = self.grid?;
        let t0 = std::time::Instant::now();
        let fields = Fields::new(&self.p.decls, &self.p.placed, grid);
        let v = fields
            .get(field, View::Stratum(rank))
            .and_then(|r| r.scalar.as_ref().map(|s| s.values().to_vec()));
        memo_spent(t0);
        self.stratum
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, v.clone());
        v
    }

    /// A scalar field's values in the full view of a generate step of layer rank `rank`
    /// ([`View::StratumFull`], `authority.md` 4.4): every entry of rank `rank` or below in the
    /// build's order, without what steps of that rank or above returned. Post order; `None` for a
    /// weights field or one that does not resolve.
    pub fn stratum_full_values(&self, field: &str, rank: i64) -> Option<Vec<f32>> {
        let key = (format!("full {field}"), rank);
        if let Some(v) = self
            .stratum
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return v.clone();
        }
        let grid = self.grid?;
        let t0 = std::time::Instant::now();
        let fields = Fields::new(&self.p.decls, &self.p.placed, grid);
        let v = fields
            .get(field, View::StratumFull(rank))
            .and_then(|r| r.scalar.as_ref().map(|s| s.values().to_vec()));
        memo_spent(t0);
        self.stratum
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, v.clone());
        v
    }

    /// The layer rank of the layer that declares import `id` (`composition.md` 6.5); `None` when
    /// there is no such import.
    pub fn import_rank(&self, id: &str) -> Option<i64> {
        self.p.decls.imports.get(id).map(|i| i.rank)
    }

    /// Whether an import declared in a layer ranked above `rank` brings `field` (`composition.md`
    /// 6.4, 6.5): then a generate step of that rank reads the field without it, in its stratum.
    /// True whenever the import's own document declares the field and does not take it as a
    /// given, brought or not: an over-approximation that only forces the stratum to be recomputed
    /// (time, not correctness).
    pub fn brought_above(&self, field: &str, rank: i64) -> bool {
        self.p.decls.imported.iter().any(|(id, imp)| {
            self.p.decls.imports.get(id).is_some_and(|d| d.rank > rank)
                && imp.alone.decls.fields.contains_key(field)
                && !imp.alone.decls.givens.contains_key(field)
        })
    }

    /// An import's field on this world's posts, `<import>.<field>` (`composition.md` 6.2, 6.5):
    /// its finished value inside the footprint, 0 outside; post order. `None` when the import has
    /// no such scalar field.
    pub fn scoped_values(&self, name: &str) -> Option<Vec<f32>> {
        let grid = self.grid?;
        let fields = Fields::new(&self.p.decls, &self.p.placed, grid);
        fields
            .get(name, View::Build)
            .and_then(|r| r.scalar.as_ref().map(|s| s.values().to_vec()))
    }

    /// The build in memory, for the command line's files and steps.
    pub fn build(&self) -> Build {
        self.whole().build.clone()
    }

    /// Each producer's output with no hand edit: the scatter run with every pin and clear left
    /// out (`graph.md` 1, `rule`).
    fn pre_edit(&self) -> Arc<PreEdit> {
        let mut slot = self
            .pre
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(p) = slot.as_ref() {
            return p.clone();
        }
        let mut out: BTreeMap<String, Vec<Placement>> = BTreeMap::new();
        if let Some(grid) = self.grid {
            let fm = self.fields_memo();
            let t0 = std::time::Instant::now();
            // The plan's own declarations (files read, imports brought, steps' returns), less
            // the hand edits: a density that reads a step's returned field or an import's place
            // must read here what the build reads, or the rule's value misses an input and its
            // `held` reader is unstable.
            let mut d = self.p.decls.clone();
            d.pins.clear();
            d.clears.clear();
            let (placed, _) = Placer::new(&d, &self.p.pack).run();
            let fields = Fields::new(&d, &placed, grid);
            for (name, r) in &fm.build {
                fields.seed(name, View::Build, r.clone());
            }
            let mut dd = Vec::new();
            let mut hand = rules::hand(&d, &placed, &grid, &mut dd);
            crate::sets::runs_give_way(&d, &self.p.pack, &mut hand, &mut dd);
            let produced = rules::scatter_all(&d, &self.p.pack, &placed, &fields, &hand);
            for p in produced.instances {
                if let Some(m) = &p.maker {
                    out.entry(m.clone()).or_default().push(p);
                }
            }
            memo_spent(t0);
        }
        let a = Arc::new(out);
        *slot = Some(a.clone());
        a
    }

    fn early_diags(&self, id: &NodeId) -> Vec<Diag> {
        self.early.get(id).cloned().unwrap_or_default()
    }

    fn val(bytes: Vec<u8>, diags: Vec<Diag>) -> Output {
        Output::of(Val {
            bytes: Arc::new(bytes),
            diags: Arc::new(diags),
        })
    }

    fn encode_geo(e: &mut Enc, g: Option<&Geo>) {
        match g {
            None => {
                e.byte(0);
            }
            Some(g) => {
                e.byte(1);
                for v in bbox_of(g) {
                    e.u64(v as u64);
                }
                e.str(&format!("{g:?}"));
            }
        }
    }

    fn encode_instances(
        e: &mut Enc,
        list: &[&crate::build::InstanceOut],
        names: &[&String],
        z: bool,
    ) {
        e.len(list.len());
        for (i, n) in list.iter().zip(names) {
            e.str(n).str(&i.asset);
            e.u64(i.origin.0 as u64)
                .u64(i.origin.1 as u64)
                .u64(i.facing as u64);
            e.u64(i.scale.to_bits()).u64(i.tint.to_bits());
            if z {
                e.u64(i.z.to_bits());
            } else {
                e.str(&format!("{:?}", i.footprint));
            }
        }
    }

    fn compute_chunk(&self, id: &NodeId) -> Output {
        let fm = self.fields_memo();
        let name = subject_of(id);
        let nofp = id.view_name() == Some(NOFP);
        let head = id.view_name() == Some(HEAD);
        let mut e = Enc::new();
        e.str("via-chunk-1");
        let w = window_of(id);
        let r = if nofp {
            fm.nofp.get(&name)
        } else if head {
            fm.head.get(&name)
        } else {
            fm.build.get(&name)
        };
        match (r.and_then(Option::as_ref), w) {
            (Some(r), Some(w)) => match &r.scalar {
                Some(s) => {
                    e.byte(0);
                    f32s(
                        &mut e,
                        s.cut(w)
                            .map(|c| c.values().to_vec())
                            .unwrap_or_default()
                            .as_slice(),
                    );
                }
                None => {
                    e.byte(1).len(r.weights.len());
                    for (m, s) in &r.weights {
                        e.str(m);
                        f32s(
                            &mut e,
                            s.cut(w)
                                .map(|c| c.values().to_vec())
                                .unwrap_or_default()
                                .as_slice(),
                        );
                    }
                }
            },
            _ => {
                e.byte(2);
            }
        }
        let diags = if nofp || head {
            Vec::new()
        } else {
            let mut d = self.early_diags(id);
            d.extend(fm.routed.get(id).cloned().unwrap_or_default());
            d
        };
        Self::val(e.finish(), diags)
    }

    fn compute_distance(&self, id: &NodeId) -> Output {
        let mut e = Enc::new();
        e.str("via-distance-1");
        let of = subject_of(id);
        let geos: Vec<&Geo> = match self.p.placed.shape(&of) {
            Some(g) => vec![g],
            None => self
                .shape_members(&of)
                .iter()
                .filter_map(|m| self.p.placed.shape(m))
                .collect(),
        };
        let boxes = member_boxes(&geos);
        if let (Some(grid), Some(w)) = (self.grid, window_of(id)) {
            e.len(w.len());
            for (i, j) in w.posts() {
                let (x, y) = via_raster::window::post_mm(&grid, i, j);
                let best = nearest_member(&geos, &boxes, P::new(x, y));
                let (d, sd) = best.unwrap_or((f64::INFINITY, f64::INFINITY));
                e.u64((d as f32).to_bits().into())
                    .u64((sd as f32).to_bits().into());
            }
        }
        Self::val(e.finish(), Vec::new())
    }

    fn compute_attr(&self, id: &NodeId) -> Output {
        let fid = subject_of(id);
        let d = &self.p.decls;
        let mut e = Enc::new();
        e.str("via-attr-1");
        if let Some(f) = d.features.get(&fid) {
            let kind = f.kind.as_ref().and_then(|k| d.kinds.get(k));
            e.str(f.kind.as_deref().unwrap_or(""));
            let tier = f.kind.as_ref().map(|_| {
                f.tier
                    .or(kind.and_then(|k| k.authority))
                    .unwrap_or(Tier::Derived)
            });
            e.str(&format!("{tier:?}"));
            e.str(kind.and_then(|k| k.output.as_deref()).unwrap_or(""));
            let mut attrs: BTreeMap<String, String> = kind
                .map(|k| {
                    k.defaults
                        .iter()
                        .map(|(a, v)| (a.clone(), format!("{v:?}")))
                        .collect()
                })
                .unwrap_or_default();
            for (a, v) in &f.attrs {
                attrs.insert(a.clone(), format!("{v:?}"));
            }
            e.len(attrs.len());
            for (a, v) in &attrs {
                e.str(a).str(v);
            }
            let names: Vec<_> = self
                .p
                .placed
                .names
                .iter()
                .filter(|((f2, _), _)| *f2 == fid)
                .collect();
            e.len(names.len());
            for ((_, a), (n, p)) in names {
                e.str(a).str(n).u64(p.x as u64).u64(p.y as u64);
            }
        }
        Self::val(e.finish(), self.early_diags(id))
    }

    fn compute_held(&self, id: &NodeId, z: bool) -> Output {
        let w = self.whole();
        let s = subject_of(id);
        let mine: Vec<(&String, &crate::build::InstanceOut)> = w
            .build
            .instances
            .iter()
            .filter(|(_, i)| i.maker.as_deref() == Some(s.as_str()))
            .collect();
        let mut e = Enc::new();
        e.str(if z { "via-heights-1" } else { "via-held-1" });
        let names: Vec<&String> = mine.iter().map(|(n, _)| *n).collect();
        let list: Vec<&crate::build::InstanceOut> = mine.iter().map(|(_, i)| *i).collect();
        Self::encode_instances(&mut e, &list, &names, z);
        let mut diags = self.early_diags(id);
        diags.extend(w.routed.get(id).cloned().unwrap_or_default());
        Self::val(e.finish(), diags)
    }

    fn compute_height(&self, id: &NodeId) -> Output {
        let s = subject_of(id);
        if self.p.decls.scatters.contains_key(&s) {
            return self.compute_held(id, true);
        }
        let w = self.whole();
        let mut e = Enc::new();
        e.str("via-height-1");
        match w.build.instances.get(&s) {
            Some(i) => {
                e.byte(1).u64(i.z.to_bits());
            }
            None => {
                e.byte(0);
            }
        }
        let mut diags = self.early_diags(id);
        diags.extend(w.routed.get(id).cloned().unwrap_or_default());
        Self::val(e.finish(), diags)
    }

    fn compute_merge(&self) -> Output {
        let mut e = Enc::new();
        e.str("via-merge-1").len(self.stmts.len());
        for (s, d) in &self.stmts {
            e.str(s).digest(d);
        }
        let mut seen = BTreeSet::new();
        let srcs: Vec<_> = self
            .world
            .sources()
            .iter()
            .filter(|s| seen.insert(s.name.clone()))
            .collect();
        e.len(srcs.len());
        for s in srcs {
            e.str(&s.name).digest(&Digest::of(s.text().as_bytes()));
        }
        e.len(self.world.changes().len());
        for c in self.world.changes() {
            e.str(c);
        }
        let id = Self::merge_id();
        let mut diags = self.early_diags(&id);
        diags.extend(
            self.fields_memo()
                .routed
                .get(&id)
                .cloned()
                .unwrap_or_default(),
        );
        diags.extend(self.whole().routed.get(&id).cloned().unwrap_or_default());
        Self::val(e.finish(), diags)
    }
}

/// Strongly connected components, iteratively (Tarjan, 1972): the component of each vertex.
fn tarjan(succ: &[Vec<usize>]) -> Vec<usize> {
    let n = succ.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on = vec![false; n];
    let mut comp = vec![usize::MAX; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut ncomp = 0usize;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        while let Some(&mut (v, ref mut k)) = work.last_mut() {
            if *k < succ[v].len() {
                let w = succ[v][*k];
                *k += 1;
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on[w] = true;
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(u, _)) = work.last() {
                    low[u] = low[u].min(low[v]);
                }
                if low[v] == index[v] {
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp[w] = ncomp;
                        if w == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
            }
        }
    }
    comp
}

impl Plan for ModelPlan<'_> {
    fn define(&self, id: &NodeId, _known: &Known<'_>) -> Defined {
        Defined::Def(self.defs.get(id).cloned().unwrap_or_default())
    }

    #[allow(clippy::too_many_lines)]
    fn compute(&self, id: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output {
        let s = subject_of(id);
        match id.family() {
            "source" | "input" => Self::val(
                def.direct
                    .first()
                    .map(|(_, d)| d.bytes().to_vec())
                    .unwrap_or_default(),
                Vec::new(),
            ),
            "merge" => self.compute_merge(),
            "select" => {
                let mut e = Enc::new();
                e.str("via-select-1");
                let ids: Vec<String> = if let Some(r) = s.strip_prefix("role:") {
                    self.p
                        .decls
                        .fields
                        .values()
                        .filter(|f| f.role.is_some_and(|x| x.word() == r))
                        .map(|f| f.name.clone())
                        .collect()
                } else if s.starts_with("tag:") {
                    self.field_sel(&s)
                } else {
                    self.shape_members(&s)
                };
                e.len(ids.len());
                for i in &ids {
                    e.str(i);
                }
                Self::val(e.finish(), Vec::new())
            }
            "attr" => self.compute_attr(id),
            "shape" => {
                let mut e = Enc::new();
                e.str("via-shape-1");
                Self::encode_geo(&mut e, self.p.placed.shapes.get(&s));
                Self::val(e.finish(), self.early_diags(id))
            }
            "op" => {
                let mut e = Enc::new();
                e.str("via-op-1");
                match self.p.placed.made.get(&s) {
                    Some(m) => {
                        e.byte(1).len(m.things.len());
                        for t in &m.things {
                            e.str(&t.name).str(t.class);
                            Self::encode_geo(&mut e, Some(&t.geo));
                        }
                    }
                    None => {
                        e.byte(0);
                    }
                }
                Self::val(e.finish(), self.early_diags(id))
            }
            "made" => {
                // One made thing that writes fields (`geometry.md` 8.1): its kind and region, so
                // a chunk it does not reach keeps its key when a distant part of its maker moves.
                let mut e = Enc::new();
                e.str("via-made-1");
                match self.p.placed.made_writers.get(&s) {
                    Some((_, kind, g)) => {
                        e.byte(1).str(kind);
                        Self::encode_geo(&mut e, Some(g));
                    }
                    None => {
                        e.byte(0);
                    }
                }
                Self::val(e.finish(), Vec::new())
            }
            "footprint" => {
                let mut e = Enc::new();
                e.str("via-footprint-1");
                let g = self.p.placed.shapes.get(&format!("{s}.ground"));
                Self::encode_geo(&mut e, g);
                Self::val(e.finish(), self.early_diags(id))
            }
            "instance" => {
                let mut e = Enc::new();
                e.str("via-plan-1");
                match self.p.placed.plans.get(&s) {
                    Some(p) => e.byte(1).str(&format!("{p:?}")),
                    None => e.byte(0),
                };
                Self::val(e.finish(), self.early_diags(id))
            }
            "edit" => {
                let mut e = Enc::new();
                e.str("via-edit-1");
                if let Some(p) = self.p.placed.pins.get(&s) {
                    // `by` is part of what the pin does: it says which producers it holds.
                    let by = self.p.decls.pins.get(&s).map(|x| &x.by);
                    e.byte(1).str(&format!("{p:?} {by:?}"));
                } else if let Some(c) = self.p.decls.clears.get(&s) {
                    e.byte(2).str(&format!(
                        "{:?} {:?} {:?} {:?}",
                        c.place, c.region, c.radius, c.by
                    ));
                } else {
                    e.byte(0);
                }
                // A selection by place resolves against the maker's output, which this node
                // reads by its `of` edge (`identity.md` 4.1): that output is part of the value,
                // so a reader keyed on this node sees the place move when the maker's does.
                let place = match (self.p.decls.pins.get(&s), self.p.decls.clears.get(&s)) {
                    (Some(p), _) => Some(&p.place),
                    (None, Some(c)) => c.place.as_ref(),
                    _ => None,
                };
                match place {
                    Some(Place::Of(sel)) if self.p.decls.scatters.contains_key(&sel.of) => {
                        let pre = self.pre_edit();
                        let list = pre.get(&sel.of).map(Vec::as_slice).unwrap_or(&[]);
                        e.byte(1).len(list.len());
                        for p in list {
                            e.str(&p.name)
                                .u64(p.origin.x as u64)
                                .u64(p.origin.y as u64)
                                .u64(p.facing as u64);
                        }
                    }
                    // A selection from a network or cells table: the place it resolved to.
                    Some(Place::Of(sel)) if self.p.decls.is_maker_table(&sel.of) => {
                        e.byte(2).str(&format!(
                            "{:?} {:?}",
                            self.p.placed.clears.get(&s),
                            self.p.placed.selections.get(&format!("clear.{s}"))
                        ));
                    }
                    _ => {
                        e.byte(0);
                    }
                }
                Self::val(e.finish(), self.early_diags(id))
            }
            "rule" => {
                let pre = self.pre_edit();
                let mut e = Enc::new();
                e.str("via-rule-1");
                let list = pre.get(&s).map(Vec::as_slice).unwrap_or(&[]);
                e.len(list.len());
                for p in list {
                    e.str(&p.name).str(&p.asset);
                    e.u64(p.origin.x as u64)
                        .u64(p.origin.y as u64)
                        .u64(p.facing as u64);
                    e.u64(p.scale.to_bits()).u64(p.tint.to_bits());
                    e.str(&format!("{:?}", p.footprint));
                }
                Self::val(e.finish(), Vec::new())
            }
            "held" => self.compute_held(id, false),
            "height" => self.compute_height(id),
            "facts" => {
                let w = self.whole();
                let mut diags = self.early_diags(id);
                diags.extend(w.routed.get(id).cloned().unwrap_or_default());
                let mut e = Enc::new();
                e.str("via-facts-1").len(diags.len());
                for d in &diags {
                    e.str(&format!("{d:?}"));
                }
                Self::val(e.finish(), diags)
            }
            "joincheck" => {
                let w = self.whole();
                let diags = w.routed.get(id).cloned().unwrap_or_default();
                let mut e = Enc::new();
                e.str("via-joincheck-1").len(diags.len());
                for d in &diags {
                    e.str(&format!("{d:?}"));
                }
                Self::val(e.finish(), diags)
            }
            "import" => Self::val(
                self.p
                    .decls
                    .imported
                    .get(&s)
                    .map(|i| i.alone.digest.clone().into_bytes())
                    .unwrap_or_default(),
                Vec::new(),
            ),
            "placed" => {
                let mut e = Enc::new();
                e.str("via-placed-1");
                match s.split_once('/') {
                    Some((h, rest)) => match self.p.decls.imported.get(h) {
                        Some(imp) => match imp.spot(rest) {
                            Some(crate::place::Spot::At { p, facing, end }) => {
                                e.str("at").str(&format!("{p:?} {facing:?} {end}"));
                            }
                            Some(crate::place::Spot::Long(g)) => {
                                e.str("long");
                                Self::encode_geo(&mut e, Some(&g));
                            }
                            None => {
                                e.str("none");
                            }
                        },
                        None => {
                            e.str("none");
                        }
                    },
                    None => match self.p.decls.imported.get(&s) {
                        Some(imp) => {
                            e.str(&format!("{:?} {:?}", imp.frame, imp.footprint));
                        }
                        None => {
                            e.str("none");
                        }
                    },
                }
                Self::val(e.finish(), Vec::new())
            }
            "field" => self.compute_chunk(id),
            "distance" => self.compute_distance(id),
            "measure" => {
                // The digest of what it read: conservative, never cut off early.
                let mut e = Enc::new();
                e.str("via-measure-1");
                let mut all: Vec<(String, String, Digest)> = inputs
                    .all()
                    .iter()
                    .map(|(l, n, v)| {
                        let mut x = Enc::new();
                        x.bytes(&v.bytes);
                        (l.clone(), n.to_string(), Digest::of(&x.finish()))
                    })
                    .collect();
                all.sort();
                e.len(all.len());
                for (l, _, d) in &all {
                    e.str(l).digest(d);
                }
                Self::val(e.finish(), Vec::new())
            }
            _ => Self::val(Vec::new(), Vec::new()),
        }
    }
}

/// A build through the graph with the model's nodes only: no `write` or `step` node. The run
/// says which nodes ran and why (`graph.md` 6.3); the build is the model's, in memory.
pub fn build_graph(
    world: &World,
    options: &Options,
    baseline: Option<&Build>,
    store: &via_graph::Store,
    graph: &via_graph::Options,
) -> (Build, Option<via_graph::Run>) {
    let plan = ModelPlan::new(world, options, baseline);
    if plan.grid.is_none() {
        return (crate::build::build(world, options, baseline), None);
    }
    let roots = plan.ids();
    let run = via_graph::run(&plan, store, graph, &roots);
    let mut b = plan.build();
    let mut diags = dedupe(run.world_diags.clone());
    if baseline.is_none() {
        diags.extend(
            b.diagnostics
                .iter()
                .filter(|d| *d.code() == via_diag::DiagCode::Core(codes::identity::NO_PREVIOUS))
                .cloned(),
        );
    }
    b.diagnostics = diags;
    (b, Some(run))
}

#[allow(dead_code)]
fn decode_bbox(bytes: &[u8]) -> Option<[i64; 4]> {
    let mut d = Dec::new(bytes);
    let _ = d.str()?;
    if d.byte()? != 1 {
        return None;
    }
    Some([
        d.u64()? as i64,
        d.u64()? as i64,
        d.u64()? as i64,
        d.u64()? as i64,
    ])
}

#[cfg(test)]
mod input_path_tests {
    use super::input_path;

    #[test]
    fn above_the_world_keeps_its_dots() {
        assert_eq!(
            input_path("world.toml", "../shared/h.npy"),
            "../shared/h.npy"
        );
        assert_eq!(input_path("world.toml", "shared/h.npy"), "shared/h.npy");
        assert_eq!(input_path("layers/a.toml", "../h.npy"), "h.npy");
        assert_eq!(input_path("layers/a.toml", "../../x/h.npy"), "../x/h.npy");
        assert_eq!(input_path("world.toml", "./a/../b.npy"), "b.npy");
        assert_ne!(
            input_path("world.toml", "../shared/h.npy"),
            input_path("world.toml", "shared/h.npy")
        );
    }
}

#[cfg(test)]
mod distance_tests {
    use super::{member_boxes, nearest_member};
    use crate::place::Geo;
    use via_geom::lineref::Line;
    use via_geom::shape::{Piece, Region};
    use via_geom::P;

    fn path(pts: &[(i64, i64)]) -> Geo {
        Geo::Path {
            line: Line::new(pts.iter().map(|&(x, y)| P::new(x, y)).collect(), false),
        }
    }

    /// The plain scan the pruning must equal: every member projected, the first least kept.
    fn scan(geos: &[&Geo], at: P) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for g in geos {
            let m = g.project(at);
            if best.is_none_or(|b| m.d < b.0) {
                best = Some((m.d, m.sd));
            }
        }
        best
    }

    fn bits(v: Option<(f64, f64)>) -> Option<(u32, u32)> {
        v.map(|(d, sd)| ((d as f32).to_bits(), (sd as f32).to_bits()))
    }

    fn check(geos: &[&Geo]) {
        let boxes = member_boxes(geos);
        for i in -10..=50 {
            for j in -10..=50 {
                let at = P::new(i * 1_000, j * 1_000);
                let (got, want) = (nearest_member(geos, &boxes, at), scan(geos, at));
                assert_eq!(got, want, "at {at:?}");
                assert_eq!(bits(got), bits(want), "at {at:?}");
            }
        }
    }

    #[test]
    fn pruning_keeps_every_value_of_a_plain_scan() {
        // Two paths at equal distance either side of y = 20, one doubled (a tie of equal
        // members), a far path, and a square with a square hole.
        let a = path(&[(0, 10_000), (40_000, 10_000)]);
        let b = path(&[(40_000, 30_000), (0, 30_000)]);
        let b2 = path(&[(40_000, 30_000), (0, 30_000)]);
        let far = path(&[(0, 45_000), (5_000, 45_000)]);
        let ring = |x0: i64, y0: i64, x1: i64, y1: i64| {
            vec![
                P::new(x0, y0),
                P::new(x1, y0),
                P::new(x1, y1),
                P::new(x0, y1),
            ]
        };
        let mut hole = ring(25_000, 15_000, 35_000, 25_000);
        hole.reverse();
        let holed = Geo::Region {
            region: Region {
                pieces: vec![Piece {
                    shell: ring(20_000, 12_000, 40_000, 28_000),
                    holes: vec![hole],
                }],
            },
            line: None,
        };
        check(&[&a, &b, &b2, &far, &holed]);
        check(&[&far, &holed, &b2, &b, &a]);
        // An empty region measures 0 everywhere: it is never pruned, and wins every post.
        let empty = Geo::Region {
            region: Region { pieces: vec![] },
            line: None,
        };
        check(&[&a, &far, &empty]);
        let boxes = member_boxes(&[&a, &far, &empty]);
        assert_eq!(boxes[2], None);
        assert_eq!(
            nearest_member(&[&a, &far, &empty], &boxes, P::new(-9_000, 49_000)),
            Some((0.0, 0.0))
        );
    }
}
