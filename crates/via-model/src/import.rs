//! Composition (`spec/composition.md`): a world built apart, placed in another by name.
//!
//! An import is read from `[import.<id>]` ([`ImportDecl`]), built alone in its own frame with its
//! own seed, pack and layers (section 3), and placed by a frame ([`Imported::frame`]): origin `at`
//! or the point `on` resolves to, facing `face`. The parent then reads it four ways:
//!
//! - a scoped place name, `<import>.<place>` or `<import>.<nested>.<place>`, resolved in the
//!   import's own placement and taken through the frame ([`Imported::spot`]);
//! - a brought field: the import's finished field read on the parent's posts by `sample`'s rule,
//!   `lift` added on the surface, under the weight `w = blend` inside the footprint
//!   ([`Imported::sample`], [`Imported::inside`]); the resolver applies it as two entries (6.4);
//! - its instances and features, under scoped names ([`Imported::bring_out`]);
//! - its diagnostics, prefixed and located in the import's files ([`Imported::diags`]).
//!
//! An import's build depends on nothing the parent says but `seed` and `changes`, so it is built
//! once per process per (file, seed, changes) and shared ([`built`]); moving it re-runs no value
//! inside it (section 12).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use via_diag::{codes, Diag, DiagCode, Site, Subject, Value as DValue};
use via_doc::{Table, Value, World};
use via_expr::FormulaId;
use via_geom::{Region, P};
use via_num::{Frame, Grid};

use crate::build::{Build, Options};
use crate::decl::{Decls, Place, Role, Tier};
use crate::place::{Geo, Placed, Spot};
use crate::text;

/// What `footprint` names (`composition.md` 2).
#[derive(Clone, Debug, PartialEq)]
pub enum Foot {
    /// Points in the import's frame.
    Points(Vec<(i64, i64)>),
    /// A place: of the import (moves with it), or of the parent (stays).
    Name(String),
}

/// One `[import.<id>]` as written.
#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub id: String,
    /// The rank of the layer that declares it (2).
    pub rank: i64,
    pub file: String,
    pub place: Option<Place>,
    /// `face`, millidegrees.
    pub face: i64,
    pub seed: Option<u64>,
    pub changes: Vec<String>,
    pub footprint: Option<Foot>,
    pub blend: Option<FormulaId>,
    pub authority: Tier,
    /// `lift`, metres.
    pub lift: Option<f64>,
    pub max_grade: f64,
    /// `give` (section 8): each given name to what the parent hands it.
    pub give: BTreeMap<String, Give>,
}

/// One value of `give` (section 8).
#[derive(Clone, Debug, PartialEq)]
pub enum Give {
    /// A field, a region, a place inside an import, or a `kind:` selector.
    Name(String),
    /// A list of feature names.
    Names(Vec<String>),
    Number(f64),
}

/// One `[given.<name>]` of a fill (section 8).
#[derive(Clone, Debug)]
pub struct GivenDecl {
    /// `region`, `field`, `features` or `number`.
    pub accepts: String,
    /// The fill's own kind that given features take.
    pub kind: Option<String>,
}

/// A given feature, in the fill's frame (section 8).
#[derive(Clone, Debug)]
pub struct GivenFeature {
    /// The parent's id, whole.
    pub id: String,
    pub shape: crate::decl::ShapeDecl,
    /// The attributes written on it that the fill's kind declares.
    pub attrs: BTreeMap<String, crate::decl::Attr>,
}

/// What a parent gives a fill, in the fill's frame (section 8).
#[derive(Clone, Debug, Default)]
pub struct GivenIn {
    /// The digest of all of it: part of the fill's build key.
    pub digest: String,
    /// Each given field's values on the fill's posts, south row first.
    pub fields: BTreeMap<String, Vec<f32>>,
    pub regions: BTreeMap<String, Vec<(i64, i64)>>,
    pub features: BTreeMap<String, Vec<GivenFeature>>,
    pub numbers: BTreeMap<String, f64>,
}

/// The lowest rank of a fill, below every layer: where given things stand (section 8).
const GIVEN_RANK: i64 = i64::MIN / 4;

/// Reads `[given.<name>]` (section 8), each as what the fill reads: a field of no role, a `let`
/// number, a region, or features of the given's kind; from `given` when a parent gave it, else
/// from its default.
pub(crate) fn read_givens(d: &mut Decls, root: &Table, given: Option<&GivenIn>) {
    use crate::decl::{FeatureDecl, FieldDecl, ShapeDecl};
    for (name, t) in crate::decl::table_of(root, "given") {
        let accepts = t.str("accepts").unwrap_or_default().to_string();
        let kind = t.str("kind").map(str::to_string);
        let feature = |id: String, shape: ShapeDecl, kind: Option<String>, attrs| FeatureDecl {
            id,
            kind,
            tier: None,
            rank: GIVEN_RANK,
            shape,
            starts_at: None,
            ends_at: None,
            face: None,
            attrs,
            smooth: false,
            holds: None,
            route: None,
        };
        match accepts.as_str() {
            "field" => {
                let mut base = None;
                match (given.and_then(|g| g.fields.get(name)), d.grid) {
                    (Some(values), Some(g)) => {
                        d.given_fields.insert(
                            name.to_string(),
                            crate::decl::FileBase {
                                samples: via_raster::npy::Samples {
                                    rows: g.ny,
                                    columns: g.nx,
                                    values: values.clone(),
                                },
                                placement: via_raster::file::Placement {
                                    extent: g.extent,
                                    outside: 0.0,
                                    filter: via_raster::file::Filter::Linear,
                                },
                                digest: given.map(|g| g.digest.clone()).unwrap_or_default(),
                            },
                        );
                    }
                    _ => {
                        if let Some(Ok(ft)) = t.value("default").and_then(text::formula_text) {
                            base = Some(d.add_formula(
                                &ft,
                                Subject::entity("given", name),
                                via_diag::KeyPath::key("default"),
                            ));
                        }
                    }
                }
                d.fields.insert(
                    name.to_string(),
                    FieldDecl {
                        name: name.to_string(),
                        role: None,
                        weights: false,
                        unit: None,
                        tags: Vec::new(),
                        rank: GIVEN_RANK,
                        base,
                        file: None,
                        rows: Vec::new(),
                    },
                );
            }
            "number" => {
                let x = given
                    .and_then(|g| g.numbers.get(name).copied())
                    .or_else(|| {
                        t.value("default")
                            .and_then(text::number)
                            .and_then(Result::ok)
                    });
                if let Some(x) = x {
                    let id = d.formulas.env.number(x);
                    d.formulas.env.world_let(name, id);
                }
            }
            "region" => {
                let ring = given
                    .and_then(|g| g.regions.get(name).cloned())
                    .or_else(|| d.grid.map(|g| extent_ring(&g)));
                if let Some(ring) = ring {
                    d.features.insert(
                        name.to_string(),
                        feature(
                            name.to_string(),
                            ShapeDecl::Region(ring),
                            None,
                            BTreeMap::new(),
                        ),
                    );
                    d.declared.insert(name.to_string(), "feature");
                }
            }
            "features" => {
                for gf in given
                    .and_then(|g| g.features.get(name))
                    .into_iter()
                    .flatten()
                {
                    let id = format!("{name}.{}", gf.id);
                    d.features.insert(
                        id.clone(),
                        feature(id.clone(), gf.shape.clone(), kind.clone(), gf.attrs.clone()),
                    );
                    d.declared.insert(id, "feature");
                }
            }
            _ => {}
        }
        d.givens
            .insert(name.to_string(), GivenDecl { accepts, kind });
    }
}

/// An import built alone: its merged world, declarations, placement and build.
#[derive(Debug)]
pub struct Alone {
    pub world: Option<World>,
    pub pack: crate::pack::Pack,
    pub decls: Decls,
    pub placed: Placed,
    pub build: Build,
    /// The digest of its statements and sources: `import:<id>`'s value (section 12).
    pub digest: String,
    /// The reading records when it did not merge.
    pub failed: Vec<via_diag::Record>,
}

/// An import as placed in its parent.
#[derive(Debug)]
pub struct Imported {
    pub id: String,
    pub decl: ImportDecl,
    pub alone: Arc<Alone>,
    /// The import's file as a path from the parent's world folder.
    pub shown: String,
    pub frame: Frame,
    /// The footprint as placed, in the parent's frame (6.1).
    pub footprint: Region,
}

/// What an import's steps are run over (`composition.md` 12): its id, prefix, file, merged world
/// and the options it is built with (seed, folder, given).
pub struct StepsCall<'a> {
    pub prefix: &'a str,
    pub path: &'a Path,
    pub world: &'a World,
    pub options: &'a Options,
}

/// What running an import's generate steps gave: the world joined with what they returned, the
/// folder of each step's `out`, and the steps' own diagnostics (subjects in the import's terms).
pub struct StepsDone {
    pub world: World,
    pub step_out: BTreeMap<String, PathBuf>,
    pub diags: Vec<Diag>,
}

/// The hook that runs an import's generate steps; the command line hands it in.
type StepsFn = dyn Fn(&StepsCall<'_>) -> Option<StepsDone> + Send + Sync;

#[derive(Clone)]
pub struct ImportSteps(pub Arc<StepsFn>);

impl std::fmt::Debug for ImportSteps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImportSteps")
    }
}

/// Every import's build in this process, by (file, seed, changes): built once, shared.
type Memo = Mutex<BTreeMap<String, Arc<Alone>>>;

fn memo() -> &'static Memo {
    static M: OnceLock<Memo> = OnceLock::new();
    M.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Reads every `[import.<id>]` of the merged world (`composition.md` 2).
pub(crate) fn read_imports(d: &mut Decls, world: &World, root: &Table) {
    let Some(t) = root.table("import") else {
        return;
    };
    for (id, e) in t.iter() {
        let Value::Table(it) = &e.item.value else {
            continue;
        };
        let place = if it.get("at").is_some() || it.get("on").is_some() {
            d.read_import_place(id, it)
        } else {
            None
        };
        let face = match it.value("face") {
            Some(Value::Num(n)) => via_num::parse_facing(&n.norm).unwrap_or(0),
            _ => 0,
        };
        let seed = match it.value("seed") {
            Some(Value::Num(n)) => n
                .norm
                .parse::<i64>()
                .ok()
                .and_then(|s| via_num::world_seed(s).ok()),
            _ => None,
        };
        let footprint = match it.value("footprint") {
            Some(Value::Str(s)) => Some(Foot::Name(s.text.clone())),
            Some(v) => text::points(v).and_then(Result::ok).map(Foot::Points),
            None => None,
        };
        let blend = it.value("blend").and_then(|v| match text::formula_text(v) {
            Some(Ok(ft)) => Some(d.add_formula(
                &ft,
                Subject::entity("import", id),
                via_diag::KeyPath::key("blend"),
            )),
            _ => None,
        });
        let authority = it.str("authority").and_then(Tier::parse);
        if let Some(a) = it.str("authority").filter(|_| authority.is_none()) {
            d.diags.push(text::at_key(
                codes::authority::UNKNOWN,
                "import",
                id,
                "authority",
                &format!("`{a}` is no tier"),
                "write `derived`, `shaped` or `anchored`",
            ));
        }
        let num = |k: &str| match it.value(k) {
            Some(v) => text::number(v).and_then(Result::ok),
            None => None,
        };
        let lift = it
            .value("lift")
            .and_then(text::length)
            .and_then(Result::ok)
            .map(via_num::metres);
        d.imports.insert(
            id.clone(),
            ImportDecl {
                id: id.clone(),
                rank: world.rank("import", id).unwrap_or(0),
                file: it.str("file").unwrap_or_default().to_string(),
                place,
                face,
                seed,
                changes: it
                    .value("changes")
                    .and_then(text::strings)
                    .unwrap_or_default(),
                footprint,
                blend,
                authority: authority.unwrap_or(Tier::Shaped),
                lift,
                max_grade: num("max_grade").unwrap_or(0.5),
                give: it
                    .table("give")
                    .map(|g| {
                        g.iter()
                            .filter_map(|(k, e)| {
                                let v = match &e.item.value {
                                    Value::Str(s) => Give::Name(s.text.clone()),
                                    Value::Num(_) => {
                                        Give::Number(text::number(&e.item.value)?.ok()?)
                                    }
                                    v @ Value::List(_) => Give::Names(text::strings(v)?),
                                    _ => return None,
                                };
                                Some((k.clone(), v))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        );
        d.declared.insert(id.clone(), "import");
    }
}

/// The scoped names formulas read (`<import>.<field>`, section 4): every name in a formula text
/// of `root` whose first part is an import id. They are declared as fields, so the checker binds
/// them; a field the import lacks is said when the field is read.
pub(crate) fn scoped_field_names(root: &Table, ids: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if ids.is_empty() {
        return out;
    }
    fn walk(v: &Value, ids: &BTreeSet<String>, out: &mut BTreeSet<String>) {
        match v {
            Value::Str(s) if s.text.starts_with('=') => {
                let b = s.text.as_bytes();
                let mut i = 0;
                while i < b.len() {
                    if b[i].is_ascii_alphabetic() || b[i] == b'_' {
                        let start = i;
                        while i < b.len()
                            && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.')
                        {
                            i += 1;
                        }
                        let word = &s.text[start..i];
                        // A name inside quotes is a place argument, not a field.
                        let quoted = start > 0 && matches!(b[start - 1], b'\'' | b'"');
                        if let Some((head, rest)) = word.split_once('.') {
                            if ids.contains(head) && !rest.is_empty() && !quoted {
                                out.insert(word.to_string());
                            }
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            Value::Table(t) => {
                for (_, e) in t.iter() {
                    walk(&e.item.value, ids, out);
                }
            }
            Value::List(xs) => {
                for x in xs {
                    walk(&x.value, ids, out);
                }
            }
            _ => {}
        }
    }
    for (_, e) in root.iter() {
        walk(&e.item.value, ids, &mut out);
    }
    out
}

/// Every scoped place name the world's text writes (`<import>.<place>`, section 4): each
/// string value, not a formula, whose first part is an import id.
pub(crate) fn scoped_refs(root: &Table, ids: &BTreeSet<String>) -> BTreeSet<String> {
    fn walk(v: &Value, top: bool, ids: &BTreeSet<String>, out: &mut BTreeSet<String>) {
        match v {
            // A formula's place argument: a quoted name (`dist('a.quay')`).
            Value::Str(s) if s.text.starts_with('=') => {
                for (k, part) in s.text.split(['\'', '"']).enumerate() {
                    if k % 2 == 1 {
                        if let Some((h, rest)) = part.split_once('.') {
                            if ids.contains(h) && !rest.is_empty() {
                                out.insert(part.to_string());
                            }
                        }
                    }
                }
            }
            Value::Str(s) => {
                if let Some((h, rest)) = s.text.split_once('.') {
                    if ids.contains(h) && !rest.is_empty() {
                        out.insert(s.text.clone());
                    }
                }
            }
            Value::Table(t) => {
                for (_, e) in t.iter() {
                    walk(&e.item.value, false, ids, out);
                }
            }
            Value::List(xs) => {
                for x in xs {
                    walk(&x.value, false, ids, out);
                }
            }
            _ => {
                let _ = top;
            }
        }
    }
    let mut out = BTreeSet::new();
    for (top, e) in root.iter() {
        // An import's own `file` is a path, not a place.
        if top == "import" || top == "kind" {
            continue;
        }
        walk(&e.item.value, true, ids, &mut out);
    }
    out
}

/// The canonical text of a path, `.` and `..` taken out by text.
fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Builds the import alone, once per process per (file, seed, changes).
fn built(
    path: &Path,
    decl: &ImportDecl,
    options: &Options,
    given: Option<Arc<GivenIn>>,
) -> Arc<Alone> {
    let key = format!(
        "{}\0{:?}\0{}\0{}",
        path.display(),
        decl.seed,
        decl.changes.join("\0"),
        given.as_ref().map_or("", |g| g.digest.as_str())
    );
    // A fill's first build, before the parent gives, places it only: its steps run once, with
    // what it is given (`composition.md` 12).
    let run_steps = given.is_some() || decl.give.is_empty();
    let key = format!("{key}\0{run_steps}");
    if let Some(a) = memo().lock().ok().and_then(|m| m.get(&key).cloned()) {
        return a;
    }
    let changes: Vec<via_doc::ChangeRequest> = decl
        .changes
        .iter()
        .map(|c| {
            // A path is relative to the import's world folder, not the process's (section 11).
            let r = via_doc::ChangeRequest::new(c.clone(), 0);
            match path.parent() {
                Some(dir) if r.is_path() => {
                    via_doc::ChangeRequest::new(dir.join(c).to_string_lossy().into_owned(), 0)
                }
                _ => r,
            }
        })
        .collect();
    let mut load = via_doc::LoadOptions::from_env(options.lib_roots.clone(), changes);
    load.shipped_root.clone_from(&options.shipped_root);
    let loaded = via_doc::load(path, 0, &load);
    let alone = match loaded.world {
        Some(world) => {
            let mut opts = options.clone();
            opts.seed = decl.seed;
            opts.world_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
            opts.step_out = BTreeMap::new();
            opts.given.clone_from(&given);
            opts.prefix = format!("{}{}.", options.prefix, decl.id);
            // The import's own generate steps, run before it is built (`composition.md` 12).
            let mut step_diags: Vec<Diag> = Vec::new();
            let world = match options.import_steps.as_ref().filter(|_| run_steps) {
                Some(h) => match (h.0)(&StepsCall {
                    prefix: &opts.prefix,
                    path,
                    world: &world,
                    options: &opts,
                }) {
                    Some(done) => {
                        opts.step_out = done.step_out;
                        step_diags = done.diags;
                        done.world
                    }
                    None => world,
                },
                None => world,
            };
            let p = crate::build::prepare(&world, &opts);
            // Its pack's bindings are its text too (section 10), and its input leaves (a field's
            // `file`, the files the pack binds) are what it is built from: an edit to one is a
            // new import (section 12, `import:<id>`).
            let leaves: Vec<(String, String)> = crate::graph::leaves_of(&p, &opts)
                .into_iter()
                .map(|(path, _, d)| (path, d.hex()))
                .collect();
            let mut digest = via_graph::Digest::of(
                format!(
                    "{} leaves {leaves:?} pack {:?}",
                    digest_of(&world, &p.decls),
                    p.pack
                        .assets
                        .iter()
                        .map(|(k, a)| (k, a.facts()))
                        .collect::<Vec<_>>()
                )
                .as_bytes(),
            )
            .hex();
            if let Some(g) = &given {
                digest =
                    via_graph::Digest::of(format!("{digest} given {}", g.digest).as_bytes()).hex();
            }
            let mut build =
                crate::build::finish_from(&world, &p, BTreeMap::new(), Vec::new(), None);
            build.diagnostics.extend(step_diags);
            Alone {
                pack: p.pack.clone(),
                decls: p.decls,
                placed: p.placed,
                build,
                digest,
                world: Some(world),
                failed: Vec::new(),
            }
        }
        None => Alone {
            pack: crate::pack::Pack::default(),
            world: None,
            decls: Decls::default(),
            placed: Placed::default(),
            build: Build::default(),
            digest: String::from("failed"),
            failed: loaded.records,
        },
    };
    let a = Arc::new(alone);
    if let Ok(mut m) = memo().lock() {
        m.insert(key, a.clone());
    }
    a
}

/// The digest of a world's statements, its sources and its imports' in turn: what
/// `import:<id>` is made from (section 12). No path enters it.
fn digest_of(world: &World, d: &Decls) -> String {
    let mut enc = via_graph::Enc::new();
    enc.str("via-import-1");
    for s in world.sources().iter() {
        enc.str(&via_graph::Digest::of(s.text().as_bytes()).hex());
    }
    enc.str(&format!("seed {}", d.seed));
    for c in world.changes() {
        enc.str(c);
    }
    for (id, i) in &d.imported {
        enc.str(id).str(&i.alone.digest);
        enc.str(&format!("{:?} {:?}", i.frame, i.decl.footprint));
    }
    via_graph::Digest::of(&enc.finish()).hex()
}

/// The subject `<import>.<collection>.<id>` of a thing inside an import, in the parts a stored
/// diagnostic reads back (`a.feature.east`: collection `a`, id `feature`, key `east`).
fn scoped_subject(import: &str, collection: &str, id: impl Into<String>) -> Subject {
    Subject::Entity {
        collection: import.to_string(),
        id: collection.to_string(),
        keys: vec![via_diag::KeyPart::Key(id.into())],
    }
}

/// The point of the import's frame at a parent point, in millimetres, unrounded: the inverse of
/// `at + R(face) q` (section 3).
pub fn to_import(frame: &Frame, p: (i64, i64)) -> (f64, f64) {
    let (s, c) = via_num::facing_sin_cos(frame.facing);
    let dx = (p.0 - frame.origin.0) as f64;
    let dy = (p.1 - frame.origin.1) as f64;
    ((dx * c) - (dy * s), (dx * s) + (dy * c))
}

/// The import's ring of its extent, in its own frame.
fn extent_ring(g: &Grid) -> Vec<(i64, i64)> {
    let [x0, y0, x1, y1] = g.extent;
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

/// The twice-area of the intersection of two convex rings (Sutherland-Hodgman, exact on the
/// lattice for the integer corners of rectangles turned by quarter turns; otherwise in doubles).
pub fn overlap_area(a: &[P], b: &[P]) -> f64 {
    let mut poly: Vec<(f64, f64)> = a.iter().map(|p| (p.x as f64, p.y as f64)).collect();
    let n = b.len();
    let ccw = via_geom::pred::twice_area(b) >= 0;
    for k in 0..n {
        let (p, q) = (b[k], b[(k + 1) % n]);
        let (px, py, qx, qy) = (p.x as f64, p.y as f64, q.x as f64, q.y as f64);
        let side = |x: f64, y: f64| {
            let c = (qx - px) * (y - py) - (qy - py) * (x - px);
            if ccw {
                c
            } else {
                -c
            }
        };
        let input = std::mem::take(&mut poly);
        let m = input.len();
        for i in 0..m {
            let cur = input[i];
            let prev = input[(i + m - 1) % m];
            let (sc, sp) = (side(cur.0, cur.1), side(prev.0, prev.1));
            if sc >= 0.0 {
                if sp < 0.0 {
                    let t = sp / (sp - sc);
                    poly.push((prev.0 + t * (cur.0 - prev.0), prev.1 + t * (cur.1 - prev.1)));
                }
                poly.push(cur);
            } else if sp >= 0.0 {
                let t = sp / (sp - sc);
                poly.push((prev.0 + t * (cur.0 - prev.0), prev.1 + t * (cur.1 - prev.1)));
            }
        }
        if poly.is_empty() {
            return 0.0;
        }
    }
    let m = poly.len();
    let mut s = 0.0;
    for i in 0..m {
        let (x0, y0) = poly[i];
        let (x1, y1) = poly[(i + 1) % m];
        s += x0 * y1 - x1 * y0;
    }
    (s / 2.0).abs() / 1e6
}

impl Imported {
    /// A point of the import's frame in the parent (section 3).
    pub fn to_parent(&self, q: (i64, i64)) -> Option<P> {
        self.frame.to_world(q).ok().map(|(x, y)| P::new(x, y))
    }

    /// A facing of the import's frame in the parent (section 3): one written gains `face`, and
    /// so does an unfaced point, which faces 0 at home; unfaced stays unfaced only when the
    /// import is not turned, so a world placed unturned looks as it does alone.
    pub(crate) fn face(&self, facing: Option<i64>) -> Option<i64> {
        if facing.is_none() && self.frame.facing == 0 {
            return None;
        }
        Some(self.frame.facing_to_world(facing.unwrap_or(0)))
    }

    fn geo_to_parent(&self, g: &Geo) -> Option<Geo> {
        Some(match g {
            Geo::Point { p, facing } => Geo::Point {
                p: self.to_parent((p.x, p.y))?,
                facing: self.face(*facing),
            },
            Geo::Path { line } => {
                let pts: Option<Vec<P>> = line
                    .pts
                    .iter()
                    .map(|q| self.to_parent((q.x, q.y)))
                    .collect();
                Geo::Path {
                    line: via_geom::lineref::Line::new(pts?, false),
                }
            }
            Geo::Region { region, line } => {
                let ring: Option<Vec<P>> = region
                    .pieces
                    .first()?
                    .shell
                    .iter()
                    .map(|q| self.to_parent((q.x, q.y)))
                    .collect();
                // A written ring keeps its outline, so `on` with `t` reads it as at home.
                let line = match line {
                    Some(l) => {
                        let pts: Option<Vec<P>> =
                            l.pts.iter().map(|q| self.to_parent((q.x, q.y))).collect();
                        Some(via_geom::lineref::Line::new(pts?, l.closed))
                    }
                    None => None,
                };
                Geo::Region {
                    region: Region::from_ring(ring?),
                    line,
                }
            }
        })
    }

    /// The import's place names, for nearest-name hints.
    pub fn names(&self) -> Vec<String> {
        let a = &self.alone;
        let mut v: Vec<String> = a.decls.declared.keys().cloned().collect();
        for (fid, g) in &a.placed.shapes {
            if matches!(g, Geo::Path { .. }) {
                v.push(format!("{fid}.start"));
                v.push(format!("{fid}.end"));
            }
        }
        for (nid, n) in &a.decls.imported {
            v.extend(n.names().into_iter().map(|x| format!("{nid}.{x}")));
        }
        v.push("footprint".into());
        v
    }

    /// A place inside the import by its own place-name grammar, taken through the frame
    /// (section 4). `None` when it names nothing there.
    pub fn spot(&self, rest: &str) -> Option<Spot> {
        let a = &self.alone;
        if rest == "footprint" {
            return Some(Spot::Long(Geo::Region {
                region: self.footprint.clone(),
                line: None,
            }));
        }
        if let Some((head, tail)) = rest.split_once('.') {
            if let Some(n) = a.decls.imported.get(head) {
                // A nested import: its spot in this import's frame, then through ours.
                return match n.spot(tail)? {
                    Spot::At { p, facing, end } => Some(Spot::At {
                        p: self.to_parent((p.x, p.y))?,
                        facing: self.face(facing),
                        end,
                    }),
                    Spot::Long(g) => Some(Spot::Long(self.geo_to_parent(&g)?)),
                };
            }
        }
        if let Some(stripped) = rest
            .strip_suffix(".start")
            .or_else(|| rest.strip_suffix(".end"))
        {
            if let Some(Geo::Path { line }) = a.placed.shapes.get(stripped) {
                let line = match self.geo_to_parent(&Geo::Path { line: line.clone() })? {
                    Geo::Path { line } => line,
                    _ => return None,
                };
                let end = rest.ends_with(".end");
                let n = line.pts.len();
                let (p, q) = if end {
                    (line.pts[n - 1], line.pts[n - 2])
                } else {
                    (line.pts[0], line.pts[1])
                };
                let (dx, dy) = if end {
                    (p.x - q.x, p.y - q.y)
                } else {
                    (q.x - p.x, q.y - p.y)
                };
                return Some(Spot::At {
                    p,
                    facing: via_num::bearing(dx, dy),
                    end: true,
                });
            }
        }
        if let Some(g) = a.placed.shapes.get(rest) {
            return Some(match self.geo_to_parent(g)? {
                Geo::Point { p, facing } => Spot::At {
                    p,
                    facing: Some(facing.unwrap_or(0)),
                    end: false,
                },
                g => Spot::Long(g),
            });
        }
        let plan = a.placed.plans.get(rest).or_else(|| a.placed.pins.get(rest));
        if let Some(pl) = plan {
            return Some(Spot::At {
                p: self.to_parent((pl.origin.x, pl.origin.y))?,
                facing: Some(self.frame.facing_to_world(pl.facing)),
                end: false,
            });
        }
        None
    }

    /// Whether the parent point lies in the footprint and the import's extent (6.1); on an
    /// outline is inside.
    pub fn inside(&self, p: (i64, i64)) -> bool {
        if self.footprint.side(P::new(p.0, p.1)) == via_geom::pred::Side::Out {
            return false;
        }
        let Some(g) = self.alone.decls.grid else {
            return false;
        };
        let (x, y) = to_import(&self.frame, p);
        let [x0, y0, x1, y1] = g.extent;
        // A frame turned off the quarter turns lands between millimetres: half a millimetre is
        // the lattice's own rounding (`numbers.md` 5.1).
        let e = 0.5;
        x >= x0 as f64 - e && x <= x1 as f64 + e && y >= y0 as f64 - e && y <= y1 as f64 + e
    }

    /// The import's finished value of a field that is not a length at a parent point: bilinear
    /// at that point in the import's frame (6.2). `None` outside its extent.
    pub fn sample(&self, values: &[f64], p: (i64, i64)) -> Option<f64> {
        self.sample_by(values, values, p, false)
    }

    /// The import's finished value of a length field (the `surface`, or unit `m`) at a parent
    /// point, before `lift`: bilinear, or the nearest post in a wall cell (6.2).
    pub fn sample_length(&self, values: &[f64], p: (i64, i64)) -> Option<f64> {
        self.sample_by(values, values, p, true)
    }

    /// A derived field's value and the value of the field it is derived from, at one parent
    /// point: when `from` is a length (`walls`), the wall cell is decided once, by `from`, and
    /// both are read at the same post (6.2), so their difference does not jump inside a wall cell.
    pub fn sample_pair(
        &self,
        values: &[f64],
        from: &[f64],
        p: (i64, i64),
        walls: bool,
    ) -> Option<(f64, f64)> {
        Some((
            self.sample_by(values, from, p, walls)?,
            self.sample_by(from, from, p, walls)?,
        ))
    }

    /// `sample`'s rule on `values`; with `walls`, a cell where `by`'s four posts span more than
    /// twice the spacing reads `values` at the nearest post.
    fn sample_by(&self, values: &[f64], by: &[f64], p: (i64, i64), walls: bool) -> Option<f64> {
        let g = self.alone.decls.grid?;
        let (x, y) = to_import(&self.frame, p);
        let [x0, y0, x1, y1] = g.extent;
        let e = 0.5;
        if x < x0 as f64 - e || x > x1 as f64 + e || y < y0 as f64 - e || y > y1 as f64 + e {
            return None;
        }
        let r = g.resolution as f64;
        let fxr = via_num::clamp((x - x0 as f64) / r, 0.0, (g.nx - 1) as f64);
        let fyr = via_num::clamp((y - y0 as f64) / r, 0.0, (g.ny - 1) as f64);
        // Snap to a post within the half millimetre the frame rounds by.
        let snap = |v: f64| {
            let k = (v + 0.5).floor();
            if (v - k).abs() * r <= e {
                k
            } else {
                v
            }
        };
        let (fxr, fyr) = (snap(fxr), snap(fyr));
        let i = (fxr.floor() as u64).min(g.nx - 1);
        let j = (fyr.floor() as u64).min(g.ny - 1);
        let i1 = (i + 1).min(g.nx - 1);
        let j1 = (j + 1).min(g.ny - 1);
        let at = |i: u64, j: u64| values.get((j * g.nx + i) as usize).copied();
        let (a, b, c, d) = (at(i, j)?, at(i1, j)?, at(i, j1)?, at(i1, j1)?);
        let (tx, ty) = (fxr - i as f64, fyr - j as f64);
        // A wall: the four posts span more than twice the spacing. Read at the nearest post, so a
        // turned wall stays a wall on the parent's posts (6.2); ties to the lower post.
        let wall = walls && {
            let (a, b, c, d) = (
                by.get((j * g.nx + i) as usize).copied()?,
                by.get((j * g.nx + i1) as usize).copied()?,
                by.get((j1 * g.nx + i) as usize).copied()?,
                by.get((j1 * g.nx + i1) as usize).copied()?,
            );
            let span = via_num::max(via_num::max(a, b), via_num::max(c, d))
                - via_num::min(via_num::min(a, b), via_num::min(c, d));
            span > 2.0 * r / 1000.0
        };
        if wall {
            return Some(match (tx > 0.5, ty > 0.5) {
                (false, false) => a,
                (true, false) => b,
                (false, true) => c,
                (true, true) => d,
            });
        }
        Some(via_raster::raster::bilinear(a, b, c, d, tx, ty))
    }

    /// The import's finished values of a field as doubles, `None` when it has no such scalar
    /// field.
    pub fn field_values(&self, field: &str) -> Option<Vec<f64>> {
        let f = self.alone.build.fields.get(field)?;
        Some(f.values.as_ref()?.iter().map(|v| f64::from(*v)).collect())
    }

    /// The import's finished weights of a weights field, per material as doubles (6.2), `None`
    /// when it has no such weights field.
    pub fn weight_values(&self, field: &str) -> Option<BTreeMap<String, Vec<f64>>> {
        let f = self.alone.build.fields.get(field)?;
        if f.values.is_some() {
            return None;
        }
        Some(
            f.weights
                .iter()
                .map(|(m, w)| (m.clone(), w.iter().map(|v| f64::from(*v)).collect()))
                .collect(),
        )
    }

    /// A file as a record of the import names it, as a path from the parent's world folder.
    pub fn shown_file(&self, f: &str) -> String {
        let shown_dir = self.shown.rsplit_once('/').map_or("", |(d, _)| d);
        if f.starts_with("lib:") || shown_dir.is_empty() {
            f.to_string()
        } else {
            format!("{shown_dir}/{f}")
        }
    }

    /// The import's diagnostics, each prefixed with the import's id and located in the import's
    /// file as a path from the parent's world folder (section 4).
    pub fn diags(&self) -> Vec<Diag> {
        let a = &self.alone;
        let mut out = Vec::new();
        let shown_dir = self.shown.rsplit_once('/').map_or("", |(d, _)| d);
        let file_of = |f: &str| -> String {
            if f.starts_with("lib:") || shown_dir.is_empty() {
                f.to_string()
            } else {
                format!("{shown_dir}/{f}")
            }
        };
        let prefixed = |s: &Subject| -> Subject {
            match s {
                Subject::Entity {
                    collection,
                    id,
                    keys,
                } => Subject::Entity {
                    // Read back as written (`a.feature.quay_road`): the import's id is the first
                    // part, so the stored diagnostic parses to the same subject (graph.md 6.2). A
                    // collection already scoped (`n.feature`, raised for a nested import) is split
                    // into its parts the same way.
                    collection: self.id.clone(),
                    id: collection
                        .split('.')
                        .next()
                        .unwrap_or(collection)
                        .to_string(),
                    keys: collection
                        .split('.')
                        .skip(1)
                        .map(|p| via_diag::KeyPart::Key(p.to_string()))
                        .chain(std::iter::once(via_diag::KeyPart::Key(id.clone())))
                        .chain(keys.iter().cloned())
                        .collect(),
                },
                other => other.clone(),
            }
        };
        let located = |code: via_diag::Code,
                       subject: Subject,
                       loc: &via_diag::Location,
                       message: &str,
                       hint: &str|
         -> Option<Diag> {
            let site = match loc {
                via_diag::Location::Span {
                    file,
                    line,
                    column,
                    bytes,
                } => Site::Text {
                    file: file_of(file),
                    line: *line,
                    column: *column,
                    bytes: *bytes,
                },
                _ => Site::Text {
                    file: self.shown.clone(),
                    line: 1,
                    column: 1,
                    bytes: [0, 0],
                },
            };
            Some(Diag::new(
                code,
                subject,
                site,
                via_diag::Text::new(message).ok()?,
                via_diag::Text::new(hint).ok()?,
            ))
        };
        for r in &a.failed {
            let Some(code) = via_diag::registry::lookup(r.code()) else {
                continue;
            };
            let subject = Subject::parse(r.subject()).unwrap_or(Subject::World);
            if let Some(mut d) = located(
                code,
                prefixed(&subject),
                r.location(),
                r.message(),
                r.hint(),
            ) {
                for (k, v) in r.data() {
                    d = d.with(k.clone(), v.clone());
                }
                out.push(d);
            }
        }
        for d in &a.build.diagnostics {
            let DiagCode::Core(code) = d.code() else {
                continue;
            };
            // About the import's own run, not the parent's build.
            if *code == codes::identity::NO_PREVIOUS {
                continue;
            }
            let Some(w) = &a.world else { continue };
            let r = d.emit(w);
            let Some(mut nd) = located(
                *code,
                prefixed(d.subject()),
                r.location(),
                d.message().as_str(),
                d.hint().as_str(),
            ) else {
                continue;
            };
            for (k, v) in d.data() {
                nd = nd.with(k.clone(), v.clone());
            }
            if let Some(p) = d.position() {
                if let Some(q) = self.to_parent((p.x_mm(), p.y_mm())) {
                    nd = nd.at(text::pos(q.x, q.y));
                }
            }
            out.push(nd);
        }
        out
    }

    /// The import's anchored feature at the parent's point `p` (section 6.4, `authority.md` 5),
    /// scoped `<id>.feature.<name>` and located at its `authority` line in the import's file: the
    /// one whose outline's box holds `p`, else the first anchored one. `None` when it holds none.
    pub(crate) fn anchored_at(&self, p: (i64, i64)) -> Option<(Subject, Site)> {
        let (x, y) = to_import(&self.frame, p);
        let anchored: Vec<&String> = self
            .alone
            .build
            .features
            .iter()
            .filter(|(_, f)| f.tier == Some(Tier::Anchored))
            .map(|(n, _)| n)
            .collect();
        let holds = |n: &&String| {
            let pts = self.alone.build.features[*n].all_points();
            let xs = pts.iter().map(|q| q.0);
            let ys = pts.iter().map(|q| q.1);
            let (Some(x0), Some(x1), Some(y0), Some(y1)) =
                (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
            else {
                return false;
            };
            x0 as f64 <= x && x <= x1 as f64 && y0 as f64 <= y && y <= y1 as f64
        };
        let name = anchored
            .iter()
            .find(|n| holds(n))
            .or(anchored.first())?
            .to_string();
        let world = self.alone.world.as_ref()?;
        let site = match text_site(
            world,
            Subject::entity("feature", name.clone()),
            Site::Key(via_diag::KeyPath::key("authority")),
        ) {
            Site::Text {
                file,
                line,
                column,
                bytes,
            } => {
                let dir = self.shown.rsplit_once('/').map_or("", |(d, _)| d);
                let file = if file.starts_with("lib:") || dir.is_empty() {
                    file
                } else {
                    format!("{dir}/{file}")
                };
                Site::Text {
                    file,
                    line,
                    column,
                    bytes,
                }
            }
            other => other,
        };
        Some((scoped_subject(&self.id, "feature", name), site))
    }
}

/// Builds and places every import of `d` (sections 2, 3, 5, 6.1, 6.2, 6.8, 10): fills
/// `d.imported` and says what the join finds before any field is resolved.
pub(crate) fn bring(
    d: &mut Decls,
    pack: &crate::pack::Pack,
    options: &Options,
    diags: &mut Vec<Diag>,
) {
    if d.imports.is_empty() {
        return;
    }
    let Some(grid) = d.grid else { return };
    let decls: Vec<ImportDecl> = d.imports.values().cloned().collect();
    // `at` first; `on` reads a place of the parent, which may be inside an import placed `at`.
    let mut order: Vec<&ImportDecl> = decls.iter().collect();
    order.sort_by_key(|i| matches!(i.place, Some(Place::On(_))));
    for decl in order {
        let path = lexical(&options.world_dir.join(&decl.file));
        let alone = built(&path, decl, options, None);
        let shown = shown(&options.world_dir, &path);
        let origin = match &decl.place {
            Some(Place::At(p)) => Some((*p, 0)),
            Some(Place::On(o)) => {
                let mut placer = crate::place::Placer::new(d, pack);
                match placer.spot("import", &decl.id, "on", &o.reference) {
                    Some(Spot::At { p, facing, .. }) => Some(((p.x, p.y), facing.unwrap_or(0))),
                    Some(Spot::Long(g)) => {
                        let p = g.name_point();
                        Some(((p.x, p.y), 0))
                    }
                    None => {
                        diags.extend(placer.diags);
                        None
                    }
                }
            }
            _ => None,
        };
        let Some((origin, base_face)) = origin else {
            continue;
        };
        let Ok(frame) = Frame::new(origin, base_face + decl.face, 1.0) else {
            continue;
        };
        let mut imp = Imported {
            id: decl.id.clone(),
            decl: decl.clone(),
            alone: alone.clone(),
            shown,
            frame,
            footprint: Region::default(),
        };
        // A ring in the import's frame, or (`stays`) one already in the parent's.
        let mut stays: Option<Vec<P>> = None;
        let ring: Vec<(i64, i64)> = match (&decl.footprint, alone.decls.grid) {
            (Some(Foot::Points(pts)), _) => pts.clone(),
            (Some(Foot::Name(n)), _) => {
                // A place of this import moves with it; a parent's region, or a place inside
                // another import, stays (6.1).
                let own = n.strip_prefix(&format!("{}.", decl.id));
                let geo = match own {
                    Some(rest) => alone.placed.shapes.get(rest).cloned(),
                    None => {
                        let mut placer = crate::place::Placer::new(d, pack);
                        let g = match placer.spot("import", &decl.id, "footprint", n) {
                            Some(Spot::Long(g)) => Some(g),
                            Some(Spot::At { .. }) => Some(Geo::Point {
                                p: P::new(0, 0),
                                facing: None,
                            }),
                            None => None,
                        };
                        diags.extend(placer.diags);
                        g
                    }
                };
                let shell = match &geo {
                    // Its first piece's outline (holes and further pieces are owed).
                    Some(Geo::Region { region, .. }) => {
                        region.pieces.first().map(|pc| pc.shell.clone())
                    }
                    _ => None,
                };
                // Said, never an empty footprint in silence (principle 3); a name the placer
                // could not find it has said already.
                if shell.is_none() && (own.is_some() || geo.is_some()) {
                    diags.push(text::at_key(
                        codes::identity::UNKNOWN_PLACE,
                        "import",
                        &decl.id,
                        "footprint",
                        &format!("`{n}` names no region, so `{}` brings nothing", decl.id),
                        "name a region, or list the footprint's points",
                    ));
                }
                match (own, shell) {
                    (Some(_), Some(sh)) => sh.iter().map(|q| (q.x, q.y)).collect(),
                    (None, Some(sh)) => {
                        stays = Some(sh);
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            (None, Some(g)) => extent_ring(&g),
            (None, None) => Vec::new(),
        };
        let placed_ring: Vec<P> = match stays {
            Some(sh) => sh,
            None => ring.iter().filter_map(|q| imp.to_parent(*q)).collect(),
        };
        if placed_ring.len() >= 3 {
            imp.footprint = Region::from_ring(placed_ring);
        }
        // A fill given things is checked once it is built with them (`give`).
        if decl.give.is_empty() {
            imp.check(d, grid, diags);
        }
        d.imported.insert(decl.id.clone(), Arc::new(imp));
    }
    // A blend that reads, by its own name, a field the import brings reads itself (6.3).
    for (id, imp) in &d.imported {
        let Some(b) = imp.decl.blend else { continue };
        let Some(prog) = d.formulas.programs.get(&b) else {
            continue;
        };
        for (class, sel) in &prog.reads.selectors {
            let brought = imp.alone.decls.fields.contains_key(sel)
                && !imp.alone.decls.givens.contains_key(sel)
                && d.fields.contains_key(sel);
            if *class == via_expr::Class::Field && brought {
                diags.push(
                    text::at_key(
                        codes::graph::CYCLE,
                        "import",
                        id,
                        "blend",
                        &format!("`blend` reads `{sel}`, which `{id}` brings through this blend: it reads itself"),
                        &format!("read `{id}.{sel}` for the import's own, or a field `{id}` does not bring"),
                    )
                    .with("loop", DValue::texts([format!("field.{sel}"), format!("import.{id}.blend")])),
                );
            }
        }
    }
    // Two imports whose footprints meet (6.8), once per pair.
    let ids: Vec<String> = d.imported.keys().cloned().collect();
    for (k, a) in ids.iter().enumerate() {
        for b in &ids[k + 1..] {
            let (ia, ib) = (&d.imported[a], &d.imported[b]);
            let (Some(pa), Some(pb)) = (ia.footprint.pieces.first(), ib.footprint.pieces.first())
            else {
                continue;
            };
            let area = overlap_area(&pa.shell, &pb.shell);
            if area > 0.0 {
                diags.push(
                    text::at_header(
                        codes::import::OVERLAP,
                        "import",
                        b,
                        &format!(
                            "the footprints of `{a}` and `{b}` overlap by {} square metres; their ground takes R29's order",
                            text::dbl(area)
                        ),
                        "move one import, or narrow a `footprint`",
                    )
                    .with("imports", DValue::texts([a.as_str(), b.as_str()]))
                    .with("area", DValue::double(area).unwrap_or(DValue::Int(0))),
                );
            }
        }
    }
    // One binding per asset id or material in a build (section 10): any two worlds of it, the
    // parent, each import and the imports inside them, compared pair by pair.
    let mut packs: Vec<(String, String, &crate::pack::Pack)> = Vec::new();
    if !pack.failed {
        packs.push(("this world".into(), String::new(), pack));
    }
    fn walk<'a>(
        imported: &'a BTreeMap<String, Arc<Imported>>,
        prefix: &str,
        top: Option<&str>,
        out: &mut Vec<(String, String, &'a crate::pack::Pack)>,
    ) {
        for (id, imp) in imported {
            let scoped = format!("{prefix}{id}");
            let top = top.unwrap_or(id);
            if !imp.alone.pack.failed {
                out.push((format!("`{scoped}`"), top.to_string(), pack_of(&imp.alone)));
            }
            walk(
                &imp.alone.decls.imported,
                &format!("{scoped}."),
                Some(top),
                out,
            );
        }
    }
    walk(&d.imported, "", None, &mut packs);
    let mut first: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for (k, (who, top, p)) in packs.iter().enumerate() {
        // Asset ids, then material names (a brought paint is drawn by the build's one binding).
        let bound = p
            .assets
            .iter()
            .map(|(id, a)| (id.clone(), "asset id", format!("{:?}", a.facts())))
            .chain(p.materials.iter().map(|(m, b)| {
                (
                    format!("material:{m}"),
                    "material",
                    format!("{:?}", b.colour),
                )
            }));
        for (key, what, facts) in bound {
            let id = key.strip_prefix("material:").unwrap_or(&key).to_string();
            match first.get(&key) {
                None => {
                    first.insert(key, (k, facts));
                }
                Some((j, f)) if *f != facts => {
                    // The later world of the pair carries it: an import, the parent never.
                    let (other, _, _) = &packs[*j];
                    let subject = if top.is_empty() { &packs[*j].1 } else { top };
                    diags.push(
                        text::at_key(
                            codes::import::BINDING_DIFFERS,
                            "import",
                            subject,
                            "file",
                            &format!(
                                "the {what} `{id}` is bound differently by {other}'s pack and {who}'s"
                            ),
                            "bind it the same, or rename it in one pack",
                        )
                        .with("id", DValue::text(id.clone())),
                    );
                }
                Some(_) => {}
            }
        }
    }
    // A brought paint on the `material` field is drawn by this world's binding (6.2, section 10):
    // one this world's pack lacks is said, never drawn in no colour.
    if pack.failed || d.pack.is_none() {
        return;
    }
    let material = Some(Role::Material);
    for (id, imp) in &d.imported {
        for (name, f) in d.fields.iter().filter(|(_, f)| f.role == material) {
            let Some(theirs) = imp.alone.build.fields.get(name) else {
                continue;
            };
            if imp.alone.decls.fields.get(name).map(|g| g.role) != Some(f.role) {
                continue;
            }
            for (m, w) in &theirs.weights {
                if pack.materials.contains_key(m) || w.iter().all(|v| *v <= 0.0) {
                    continue;
                }
                let names = || pack.materials.keys().map(String::as_str);
                diags.push(text::with_nearest(
                    text::at_header(
                        codes::pack::UNKNOWN_MATERIAL,
                        "import",
                        id,
                        &format!("`{id}` paints `{m}` on `{name}`, and this world's pack binds no material `{m}`"),
                        &text::near_hint(m, names(), "bind it in this world's pack, or share the import's pack"),
                    ),
                    m,
                    names(),
                    |n: &str| pack.declaration("material", n),
                ));
            }
        }
    }
}

/// Every name-like word in a value's strings: ids, dotted names, and the parts of selectors.
fn words(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Str(s) => {
            for w in s
                .text
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .filter(|w| !w.is_empty())
            {
                out.insert(w.trim_end_matches('.').to_string());
            }
        }
        Value::Table(t) => {
            for (_, e) in t.iter() {
                words(&e.item.value, out);
            }
        }
        Value::List(xs) => {
            for x in xs {
                words(&x.value, out);
            }
        }
        _ => {}
    }
}

/// The tables of a world that may read a name, by (top, id).
const READERS: [&str; 11] = [
    "feature", "field", "instance", "scatter", "pin", "clear", "step", "network", "cells", "set",
    "run",
];

/// The fill's names that reach the given `g` (section 8, "What a give may read"): `g`, and every
/// entity whose text names one of them, to a fixed point. Ours: read from the text, as named
/// references and formula names are (`documents.md` 5.1).
fn reach(root: &Table, g: &str) -> BTreeSet<String> {
    let mut tables: Vec<(String, BTreeSet<String>)> = Vec::new();
    for top in READERS {
        for (id, t) in crate::decl::table_of(root, top) {
            let mut w = BTreeSet::new();
            words(&Value::Table(t.clone()), &mut w);
            tables.push((id.to_string(), w));
        }
    }
    let mut r: BTreeSet<String> = BTreeSet::from([g.to_string()]);
    loop {
        let before = r.len();
        for (id, w) in &tables {
            if !r.contains(id)
                && w.iter()
                    .any(|x| r.contains(x.split('.').next().unwrap_or(x)))
            {
                r.insert(id.clone());
            }
        }
        if r.len() == before {
            return r;
        }
    }
}

/// The scoped names of import `id` a parent give reads (section 8): what it names, and what the
/// parent's entities it names read in turn.
fn give_reads(root: &Table, id: &str, v: &Give) -> BTreeSet<String> {
    let mut queue: Vec<String> = match v {
        Give::Name(s) => vec![s.strip_prefix("kind:").unwrap_or(s).to_string()],
        Give::Names(xs) => xs.clone(),
        Give::Number(_) => Vec::new(),
    };
    let kinds: Option<&str> = match v {
        Give::Name(s) => s.strip_prefix("kind:"),
        _ => None,
    };
    if let Some(k) = kinds {
        for (fid, t) in crate::decl::table_of(root, "feature") {
            if t.str("kind") == Some(k) {
                queue.push(fid.to_string());
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut out = BTreeSet::new();
    while let Some(n) = queue.pop() {
        if !seen.insert(n.clone()) {
            continue;
        }
        if let Some(rest) = n.strip_prefix(&format!("{id}.")) {
            out.insert(rest.to_string());
            continue;
        }
        for top in READERS {
            if let Some(t) = root.table(top).and_then(|x| x.table(&n)) {
                let mut w = BTreeSet::new();
                words(&Value::Table(t.clone()), &mut w);
                queue.extend(w);
            }
        }
    }
    out
}

/// The form of what a parent gives (section 8's table), as `accepts` words it.
fn give_form(d: &Decls, placed: &Placed, v: &Give) -> &'static str {
    match v {
        Give::Number(_) => "number",
        Give::Names(_) => "features",
        Give::Name(s) if s.starts_with("kind:") => "features",
        Give::Name(s) if d.fields.contains_key(s) => "field",
        Give::Name(s) => match placed.shapes.get(s) {
            Some(Geo::Region { .. }) => "region",
            Some(Geo::Path { .. }) => "path",
            Some(Geo::Point { .. }) => "point",
            None => match s
                .split_once('.')
                .and_then(|(h, r)| d.imported.get(h)?.spot(r))
            {
                Some(Spot::Long(Geo::Region { .. })) => "region",
                Some(Spot::Long(_)) => "path",
                Some(Spot::At { .. }) => "point",
                None => "nothing",
            },
        },
    }
}

/// The parent's value of `field` at a point, bilinear between its posts; `None` outside.
fn sample_parent(grid: &Grid, values: &[f32], p: P) -> Option<f64> {
    let [x0, y0, x1, y1] = grid.extent;
    if p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1 {
        return None;
    }
    let r = grid.resolution as f64;
    let fx = via_num::clamp((p.x - x0) as f64 / r, 0.0, (grid.nx - 1) as f64);
    let fy = via_num::clamp((p.y - y0) as f64 / r, 0.0, (grid.ny - 1) as f64);
    let i = (fx.floor() as u64).min(grid.nx - 1);
    let j = (fy.floor() as u64).min(grid.ny - 1);
    let i1 = (i + 1).min(grid.nx - 1);
    let j1 = (j + 1).min(grid.ny - 1);
    let at = |i: u64, j: u64| {
        values
            .get((j * grid.nx + i) as usize)
            .map(|v| f64::from(*v))
    };
    Some(via_raster::raster::bilinear(
        at(i, j)?,
        at(i1, j)?,
        at(i, j1)?,
        at(i1, j1)?,
        fx - i as f64,
        fy - j as f64,
    ))
}

/// A shape of the parent taken into the fill's frame (section 8, "Frames").
fn to_fill_shape(imp: &Imported, g: &Geo) -> Option<crate::decl::ShapeDecl> {
    use crate::decl::{ShapeDecl, WayDecl};
    // Rounded once to the lattice (`numbers.md` 5.1).
    let back = |p: &P| {
        let (x, y) = to_import(&imp.frame, (p.x, p.y));
        (
            via_num::round_to_lattice(x).unwrap_or(0),
            via_num::round_to_lattice(y).unwrap_or(0),
        )
    };
    Some(match g {
        Geo::Point { p, .. } => ShapeDecl::Point(Place::At(back(p))),
        Geo::Path { line } => {
            ShapeDecl::Path(line.pts.iter().map(|p| WayDecl::At(back(p))).collect())
        }
        Geo::Region { region, .. } => {
            ShapeDecl::Region(region.pieces.first()?.shell.iter().map(back).collect())
        }
    })
}

/// Hands each fill what its parent gives (section 8) and builds it again with it: checks the
/// names and forms, refuses a give that reads what reads the given (`graph.cycle`), and replaces
/// the import's build. True when a fill was built again, so the parent places again.
#[allow(clippy::too_many_arguments)]
pub(crate) fn give(
    world: &World,
    d: &mut Decls,
    pack: &crate::pack::Pack,
    placed: &Placed,
    grid: Grid,
    options: &Options,
    diags: &mut Vec<Diag>,
) -> bool {
    let _ = pack;
    let ids: Vec<String> = d
        .imports
        .values()
        .filter(|i| !i.give.is_empty())
        .map(|i| i.id.clone())
        .collect();
    let mut again = false;
    for id in ids {
        let Some(imp) = d.imported.get(&id).cloned() else {
            continue;
        };
        let fill = imp.alone.clone();
        let Some(fill_world) = fill.world.as_ref() else {
            imp.check(d, grid, diags);
            continue;
        };
        let Some(fg) = fill.decls.grid else {
            continue;
        };
        let mut gin = GivenIn::default();
        let mut field_needed = false;
        for (name, v) in &imp.decl.give {
            let Some(gd) = fill.decls.givens.get(name) else {
                let names: Vec<&str> = fill.decls.givens.keys().map(String::as_str).collect();
                let near = via_diag::nearest::nearest(name, names.iter().copied());
                let mut dg = text::at_path(
                    codes::import::GIVEN_UNKNOWN,
                    "import",
                    &id,
                    &["give", name],
                    &format!("`{id}` declares no `[given.{name}]`"),
                    &match near.found() {
                        Some(n) => format!("did you mean `{n}`?"),
                        None => "give a name the import declares in `[given]`".to_string(),
                    },
                );
                if let Some(n) = near.found() {
                    dg = dg.with("nearest", DValue::text(n.to_string()));
                }
                diags.push(dg);
                continue;
            };
            let form = give_form(d, placed, v);
            if form != gd.accepts {
                diags.push(
                    text::at_path(
                        codes::import::GIVEN_TYPE,
                        "import",
                        &id,
                        &["give", name],
                        &format!(
                            "`{id}`'s given `{name}` accepts a {}, and this gives a {form}",
                            gd.accepts
                        ),
                        &format!("give `{name}` a {}", gd.accepts),
                    )
                    .with("accepts", DValue::text(gd.accepts.clone()))
                    .with("given", DValue::text(form)),
                );
                continue;
            }
            // What a give may read: nothing of the fill that reaches the given.
            let reached = reach(fill_world.root(), name);
            let loops: Vec<String> = give_reads(world.root(), &id, v)
                .into_iter()
                .filter(|r| reached.contains(r.split('.').next().unwrap_or(r)))
                .collect();
            if let Some(r) = loops.first() {
                diags.push(
                    text::at_path(
                        codes::graph::CYCLE,
                        "import",
                        &id,
                        &["give", name],
                        &format!("what this gives `{id}` as `{name}` reads `{id}.{r}`, which reads the given `{name}`: a loop"),
                        &format!("draw `{name}` from what of `{id}` reads no given, or from this world's own places"),
                    )
                    .with("loop", DValue::texts([format!("{id}.given:{name}"), format!("{id}.{r}")])),
                );
                continue;
            }
            match (v, gd.accepts.as_str()) {
                (Give::Number(x), _) => {
                    gin.numbers.insert(name.clone(), *x);
                }
                (Give::Name(f), "field") => {
                    field_needed = true;
                    // The parent's field as it stands under this import (its own entries left
                    // out), so the fill does not read itself.
                    let held = d.imported.remove(&id);
                    let vals: Option<Vec<f32>> = {
                        let fields = crate::resolve::Fields::new(d, placed, grid);
                        fields
                            .get(f, crate::resolve::View::Build)
                            .and_then(|r| r.scalar.as_ref().map(|s| s.values().to_vec()))
                    };
                    if let Some(h) = held {
                        d.imported.insert(id.clone(), h);
                    }
                    let Some(vals) = vals else { continue };
                    let mut out = Vec::with_capacity((fg.nx * fg.ny) as usize);
                    for j in 0..fg.ny {
                        for i in 0..fg.nx {
                            let q = (
                                fg.extent[0] + i as i64 * fg.resolution,
                                fg.extent[1] + j as i64 * fg.resolution,
                            );
                            let v = imp
                                .to_parent(q)
                                .and_then(|p| sample_parent(&grid, &vals, p))
                                .unwrap_or(0.0);
                            out.push(v as f32);
                        }
                    }
                    gin.fields.insert(name.clone(), out);
                }
                (Give::Name(r), "region") => {
                    let g = match placed.shapes.get(r) {
                        Some(g) => Some(g.clone()),
                        None => match r
                            .split_once('.')
                            .and_then(|(h, x)| d.imported.get(h)?.spot(x))
                        {
                            Some(Spot::Long(g)) => Some(g),
                            _ => None,
                        },
                    };
                    if let Some(crate::decl::ShapeDecl::Region(ring)) =
                        g.as_ref().and_then(|g| to_fill_shape(&imp, g))
                    {
                        gin.regions.insert(name.clone(), ring);
                    }
                }
                (_, "features") => {
                    let ids: Vec<String> = match v {
                        Give::Name(s) => {
                            let k = s.strip_prefix("kind:").unwrap_or(s);
                            d.features
                                .iter()
                                .filter(|(_, f)| f.kind.as_deref() == Some(k))
                                .map(|(fid, _)| fid.clone())
                                .collect()
                        }
                        Give::Names(xs) => xs.clone(),
                        Give::Number(_) => Vec::new(),
                    };
                    let declared: BTreeSet<String> = gd
                        .kind
                        .as_ref()
                        .and_then(|k| fill.decls.kinds.get(k))
                        .map(|k| {
                            k.defaults
                                .keys()
                                .chain(k.requires.iter())
                                .chain(k.names.iter())
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    let ftab = world.root().table("feature");
                    let mut list = Vec::new();
                    for fid in ids {
                        let Some(shape) =
                            placed.shapes.get(&fid).and_then(|g| to_fill_shape(&imp, g))
                        else {
                            continue;
                        };
                        let mut attrs = BTreeMap::new();
                        let written = ftab.and_then(|t| t.table(&fid));
                        if let Some(fd) = d.features.get(&fid) {
                            for (a, val) in &fd.attrs {
                                // Written attributes only: the parent kind's defaults stay its own.
                                if written.and_then(|t| t.get(a)).is_none() {
                                    continue;
                                }
                                if declared.contains(a) {
                                    attrs.insert(a.clone(), val.clone());
                                } else {
                                    diags.push(
                                        text::at_key(
                                            codes::import::GIVEN_DROPPED,
                                            "feature",
                                            &fid,
                                            a,
                                            &format!("`{fid}` is given to `{id}` as `{name}`, whose kind `{}` declares no `{a}`: it is dropped", gd.kind.as_deref().unwrap_or("")),
                                            "declare the attribute on the fill's kind, or do not write it here",
                                        )
                                        .with("attribute", DValue::text(a.clone())),
                                    );
                                }
                            }
                        }
                        list.push(GivenFeature {
                            id: fid,
                            shape,
                            attrs,
                        });
                    }
                    gin.features.insert(name.clone(), list);
                }
                _ => {}
            }
        }
        // A given field must cover the fill as placed (section 8, "Frames").
        if field_needed {
            let ring: Vec<P> = extent_ring(&fg)
                .into_iter()
                .filter_map(|q| imp.to_parent(q))
                .collect();
            let [x0, y0, x1, y1] = grid.extent;
            let ext = [
                P::new(x0, y0),
                P::new(x1, y0),
                P::new(x1, y1),
                P::new(x0, y1),
            ];
            let out = overlap_area(&ring, &ring) - overlap_area(&ring, &ext);
            if out > 1e-9 {
                diags.push(
                    text::at_header(
                        codes::import::GIVE_OUTSIDE,
                        "import",
                        &id,
                        &format!("{} square metres of `{id}` lie past this world's extent, where no given field is defined", text::dbl(out)),
                        "move the fill inside this world's extent",
                    )
                    .with("area", DValue::double(out).unwrap_or(DValue::Int(0))),
                );
            }
        }
        let mut enc = via_graph::Enc::new();
        enc.str("via-given-1");
        enc.str(&format!("{gin:?}"));
        gin.digest = via_graph::Digest::of(&enc.finish()).hex();
        let path = lexical(&options.world_dir.join(&imp.decl.file));
        let alone = built(&path, &imp.decl, options, Some(Arc::new(gin)));
        let next = Imported {
            id: imp.id.clone(),
            decl: imp.decl.clone(),
            alone,
            shown: imp.shown.clone(),
            frame: imp.frame,
            footprint: imp.footprint.clone(),
        };
        next.check(d, grid, diags);
        d.imported.insert(id.clone(), Arc::new(next));
        again = true;
    }
    again
}

/// The pack an import's build used.
fn pack_of(a: &Alone) -> &crate::pack::Pack {
    &a.pack
}

/// A path from the parent's world folder, `/`-separated.
fn shown(dir: &Path, p: &Path) -> String {
    let dir = lexical(dir);
    let a: Vec<_> = dir.components().collect();
    let b: Vec<_> = p.components().collect();
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".to_string(); a.len() - same];
    parts.extend(
        b[same..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

impl Imported {
    /// What the join says before anything is resolved: datum (5), fields (6.2), coarser posts,
    /// a footprint past the extent (6.1).
    fn check(&self, d: &Decls, grid: Grid, diags: &mut Vec<Diag>) {
        let id = &self.id;
        let a = &self.alone;
        diags.extend(self.diags());
        if !a.failed.is_empty() {
            return;
        }
        // The datum (section 5).
        match (&a.decls.datum, &d.datum, self.decl.lift) {
            (Some(x), Some(y), None) if x != y => diags.push(
                text::at_header(
                    codes::import::DATUM_DIFFERS,
                    "import",
                    id,
                    &format!("`{id}` declares the datum `{x}` and this world `{y}`, and no `lift` says how far apart they are"),
                    "write `lift = <metres>`: how far the import's zero lies above this world's",
                )
                .with("datum", DValue::text(x.clone()))
                .with("parent_datum", DValue::text(y.clone())),
            ),
            (Some(x), Some(y), Some(_)) if x == y => diags.push(text::at_key(
                codes::import::LIFT_ON_DATUM,
                "import",
                id,
                "lift",
                &format!("`{id}` shares this world's datum `{x}`, so `lift` would move it off the shared zero"),
                "remove `lift`",
            )),
            (x, y, None) if x.is_none() || y.is_none() => diags.push(
                text::at_header(
                    codes::import::NO_DATUM,
                    "import",
                    id,
                    &format!(
                        "{} declares no `datum`, so nothing says the two heights share a zero",
                        if y.is_none() { "this world" } else { "the import" }
                    ),
                    "write `datum = \"<name>\"` in both worlds, or `lift` on the import",
                )
                .with(
                    "write",
                    DValue::text(format!(
                        "datum = \"{}\"",
                        x.clone().or_else(|| y.clone()).unwrap_or_else(|| "<name>".into())
                    )),
                ),
            ),
            _ => {}
        }
        // Fields (6.2); a given field is the parent's own, handed down, not the import's. A step's
        // own field (`<step>#<name>`, R32) is a produced name no parent can declare: it is read
        // inside the import, never matched (composition.md 6.2).
        for (name, f) in &a.decls.fields {
            if a.decls.givens.contains_key(name) || name.contains('#') {
                continue;
            }
            match d.fields.get(name) {
                None => diags.push(
                    text::at_header(
                        codes::import::FIELD_UNMATCHED,
                        "import",
                        id,
                        &format!("`{id}`'s field `{name}` has no field of that name here, so it is not brought"),
                        "declare the field in this world to bring it",
                    )
                    .with("field", DValue::text(name.clone()))
                    .with(
                        "write",
                        DValue::text(match f.role {
                            Some(r) => format!("[field.{name}]\nrole = \"{}\"", r.word()),
                            None => format!("[field.{name}]"),
                        }),
                    ),
                ),
                Some(p) if p.role != f.role || p.weights != f.weights || p.unit != f.unit => {
                    diags.push(
                        text::at_header(
                            codes::import::FIELD_DIFFERS,
                            "import",
                            id,
                            &format!("`{id}`'s field `{name}` and this world's differ in role, type or unit"),
                            "declare them alike, or rename one",
                        )
                        .with("field", DValue::text(name.clone()))
                        .related(via_diag::Related {
                            subject: Subject::entity("field", name.clone()),
                            site: Site::Header,
                            position: None,
                            note: via_diag::text!("this world's declaration"),
                        }),
                    );
                }
                _ => {}
            }
        }
        // Coarser posts (6.2).
        if let Some(g) = a.decls.grid {
            if grid.resolution > g.resolution {
                let ratio = grid.resolution as f64 / g.resolution as f64;
                diags.push(
                    text::at_header(
                        codes::import::COARSER,
                        "import",
                        id,
                        &format!("this world's posts are {}x coarser than `{id}`'s; detail between them is lost", text::dbl(ratio)),
                        "nothing if the detail does not matter; else a finer `resolution` here",
                    )
                    .with("ratio", DValue::double(ratio).unwrap_or(DValue::Int(1))),
                );
            }
        }
        // A footprint past the extent (6.1).
        if let Some(pc) = self.footprint.pieces.first() {
            let [x0, y0, x1, y1] = grid.extent;
            let ext = [
                P::new(x0, y0),
                P::new(x1, y0),
                P::new(x1, y1),
                P::new(x0, y1),
            ];
            let whole = overlap_area(&pc.shell, &pc.shell);
            let inside = overlap_area(&pc.shell, &ext);
            let out = whole - inside;
            if out > 1e-9 {
                diags.push(
                    text::at_header(
                        codes::import::OUTSIDE,
                        "import",
                        id,
                        &format!("{} square metres of `{id}`'s footprint lie past this world's extent and are not brought", text::dbl(out)),
                        "move the import inside, or narrow its `footprint`",
                    )
                    .with("area", DValue::double(out).unwrap_or(DValue::Int(0))),
                );
            }
        }
    }

    /// The import's instances and features under scoped names, taken through the frame (section
    /// 3): instances whose origin lies inside the footprint and the parent's extent, their heights
    /// found again on the parent's finished surface by `z_at`.
    pub fn bring_out(&self, grid: &Grid, out: &mut Build, z_at: &dyn Fn(P) -> Option<f64>) {
        let b = &self.alone.build;
        let [x0, y0, x1, y1] = grid.extent;
        for (name, i) in &b.instances {
            let Some(o) = self.to_parent(i.origin) else {
                continue;
            };
            if o.x < x0 || o.x > x1 || o.y < y0 || o.y > y1 {
                continue;
            }
            if self.footprint.side(o) == via_geom::pred::Side::Out {
                continue;
            }
            let mut i2 = i.clone();
            i2.origin = (o.x, o.y);
            i2.facing = self.frame.facing_to_world(i.facing);
            i2.footprint = i
                .footprint
                .iter()
                .filter_map(|q| self.to_parent(*q).map(|p| (p.x, p.y)))
                .collect();
            if let Some(z) = z_at(o) {
                i2.z = z;
            }
            i2.maker = i.maker.as_ref().map(|m| format!("{}.{m}", self.id));
            out.instances.insert(format!("{}.{name}", self.id), i2);
        }
        for (name, f) in &b.features {
            let mut f2 = f.clone();
            f2.points = f
                .points
                .iter()
                .filter_map(|q| self.to_parent(*q).map(|p| (p.x, p.y)))
                .collect();
            f2.rings = f
                .rings
                .iter()
                .map(|pc| {
                    pc.iter()
                        .map(|r| {
                            r.iter()
                                .filter_map(|q| self.to_parent(*q).map(|p| (p.x, p.y)))
                                .collect()
                        })
                        .collect()
                })
                .collect();
            f2.facing = self.face(f.facing);
            // A name attribute names a place of the import: scoped, and taken through the frame
            // (section 4), as the feature's own points are.
            f2.names = f
                .names
                .iter()
                .filter_map(|(k, (n, q))| {
                    let p = self.to_parent(*q)?;
                    Some((k.clone(), (format!("{}.{n}", self.id), (p.x, p.y))))
                })
                .collect();
            for (k, (n, _)) in &f.names {
                if let Some(v) = f2.attrs.get_mut(k).filter(|v| *v == n) {
                    *v = format!("{}.{n}", self.id);
                }
            }
            out.features.insert(format!("{}.{name}", self.id), f2);
        }
        for (a, facts) in &b.assets {
            out.assets.entry(a.clone()).or_insert_with(|| facts.clone());
        }
    }
}

/// The seam check (6.6) and places the join alters (6.7), after the parent's surface is final.
/// `w` is each import's weight per parent post.
pub(crate) fn join_checks(
    d: &Decls,
    grid: &Grid,
    surface: Option<(&str, &[f64])>,
    weights: &BTreeMap<String, Vec<f64>>,
    anchored1: &[bool],
    named: &BTreeSet<String>,
    diags: &mut Vec<Diag>,
) {
    let Some((sname, z)) = surface else { return };
    let nx = grid.nx as usize;
    let ny = grid.ny as usize;
    let r = via_num::metres(grid.resolution);
    for (id, imp) in &d.imported {
        let Some(w) = weights.get(id) else { continue };
        let mut worst: Option<(f64, usize)> = None;
        let mut count = 0u64;
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                for n in [(i + 1 < nx).then(|| k + 1), (j + 1 < ny).then(|| k + nx)]
                    .into_iter()
                    .flatten()
                {
                    if w[k] == w[n] || anchored1[k] || anchored1[n] {
                        continue;
                    }
                    let g = (z[k] - z[n]).abs() / r;
                    if g > imp.decl.max_grade {
                        count += 1;
                        // The post of the pair inside the footprint.
                        let at = if w[k] > 0.0 { k } else { n };
                        if worst.is_none_or(|(x, _)| g > x) {
                            worst = Some((g, at));
                        }
                    }
                }
            }
        }
        if let Some((g, at)) = worst {
            let (x, y) = via_raster::window::post_mm(grid, (at % nx) as u64, (at / nx) as u64);
            diags.push(
                text::at_header(
                    codes::import::SEAM,
                    "import",
                    id,
                    &format!("the ground steps across `{id}`'s edge at a grade of {} on `{sname}`, above `max_grade` {}", text::dbl(g), text::dbl(imp.decl.max_grade)),
                    "blend the import in (`blend`), own the ground with an anchored feature, or raise `max_grade`",
                )
                .at(text::pos(x, y))
                .with("grade", DValue::double(g).unwrap_or(DValue::Int(0)))
                .with("count", DValue::Int(count as i64)),
            );
        }
        // Places the parent names whose weight is below 1 (6.7).
        for name in named {
            let Some(rest) = name.strip_prefix(&format!("{id}.")) else {
                continue;
            };
            let Some(Spot::At { p, .. }) = imp.spot(rest) else {
                continue;
            };
            // The nearest post, half up; a place off the posts reads nothing.
            let near = |d: i64| (d + grid.resolution / 2).div_euclid(grid.resolution);
            let Ok(i) = u64::try_from(near(p.x - grid.extent[0])) else {
                continue;
            };
            let Ok(j) = u64::try_from(near(p.y - grid.extent[1])) else {
                continue;
            };
            if i >= grid.nx || j >= grid.ny {
                continue;
            }
            let k = (j * grid.nx + i) as usize;
            let Some(&wk) = w.get(k) else { continue };
            if wk < 1.0 {
                let own = imp
                    .field_values(sname)
                    .and_then(|v| imp.sample_length(&v, (p.x, p.y)))
                    .unwrap_or(0.0);
                let moved = z[k] - own;
                diags.push(
                    text::at_header(
                        codes::import::PLACE_BLENDED,
                        "import",
                        id,
                        &format!("`{name}` stands where `{id}`'s weight is {}: on ground the import did not make", text::dbl(wk)),
                        "move the place inside, or blend later (`blend`)",
                    )
                    .at(text::pos(p.x, p.y))
                    .with("place", DValue::text(name.clone()))
                    .with("weight", DValue::double(wk).unwrap_or(DValue::Int(0)))
                    .with("moved", DValue::double(moved).unwrap_or(DValue::Int(0))),
                );
            }
        }
    }
}

/// The text span a diagnostic about `subject` at `site` resolves to in `world`, as a site that
/// another subject may carry (an exit's subject is inside the import, its span the parent's).
fn text_site(world: &World, subject: Subject, site: Site) -> Site {
    let probe = text::diag(codes::diag::INTERNAL, subject, site, "probe", "probe");
    match probe.emit(world).location() {
        via_diag::Location::Span {
            file,
            line,
            column,
            bytes,
        } => Site::Text {
            file: file.clone(),
            line: *line,
            column: *column,
            bytes: *bytes,
        },
        _ => Site::World,
    }
}

/// The parent's named references (`documents.md` 5.1) that may answer an exit (section 7): each
/// name to the parent's features and instances that write it, as (table, id).
pub(crate) fn answers(d: &Decls, out: &Build) -> BTreeMap<String, Vec<(String, String)>> {
    let mut by: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut add = |name: &str, top: &str, id: &str| {
        let v = by.entry(name.to_string()).or_default();
        let e = (top.to_string(), id.to_string());
        if !v.contains(&e) {
            v.push(e);
        }
    };
    for (id, f) in &d.features {
        for n in [&f.starts_at, &f.ends_at].into_iter().flatten() {
            add(n, "feature", id);
        }
        if let crate::decl::ShapeDecl::Point(Place::On(o)) = &f.shape {
            add(&o.reference, "feature", id);
        }
        if let Some(fo) = out.features.get(id) {
            for (n, _) in fo.names.values() {
                add(n, "feature", id);
            }
        }
    }
    for (id, i) in &d.instances {
        if let Place::On(o) = &i.place {
            add(&o.reference, "instance", id);
        }
    }
    by
}

/// Two attribute values alike (section 7, "Matched"): numbers by value, else the text exactly.
fn same_value(a: &str, b: &str) -> bool {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// An attribute value as `data` holds it: a number when it reads as one.
fn data_value(v: &str) -> DValue {
    match v.parse::<f64>() {
        Ok(x) => DValue::double(x).unwrap_or_else(|| DValue::text(v.to_string())),
        Err(_) => match v {
            "true" => DValue::Bool(true),
            "false" => DValue::Bool(false),
            _ => DValue::text(v.to_string()),
        },
    }
}

/// Exits (section 7): every exit of every direct import lies inside its footprint, is named by a
/// parent reference, and each answering feature matches the attributes its kind lists.
pub(crate) fn exit_checks(world: &World, d: &Decls, out: &Build, diags: &mut Vec<Diag>) {
    if d.imported.is_empty() {
        return;
    }
    let by = answers(d, out);
    for (id, imp) in &d.imported {
        let a = &imp.alone;
        if !a.failed.is_empty() {
            continue;
        }
        for (fid, fd) in &a.decls.features {
            let Some(list) = fd
                .kind
                .as_ref()
                .and_then(|k| a.decls.kinds.get(k))
                .and_then(|k| k.exit.as_ref())
            else {
                continue;
            };
            let subject = scoped_subject(id, "feature", fid.clone());
            let exit_site = a.world.as_ref().map_or(Site::World, |w| {
                let s = text_site(w, Subject::entity("feature", fid.clone()), Site::Header);
                match s {
                    Site::Text {
                        file,
                        line,
                        column,
                        bytes,
                    } => Site::Text {
                        file: imp.shown_file(&file),
                        line,
                        column,
                        bytes,
                    },
                    other => other,
                }
            });
            let at_import = text_site(world, Subject::entity("import", id.clone()), Site::Header);
            let related = |note: &str| via_diag::Related {
                subject: subject.clone(),
                site: exit_site.clone(),
                position: None,
                note: via_diag::Text::new(note).unwrap_or_else(|_| via_diag::text!("the exit")),
            };
            // Where it lies: inside the footprint, edges included.
            let at = match a.placed.shapes.get(fid) {
                Some(Geo::Point { p, .. }) => imp.to_parent((p.x, p.y)),
                _ => None,
            };
            if let Some(p) = at {
                if imp.footprint.side(p) == via_geom::pred::Side::Out {
                    let foot_site = if imp.decl.footprint.is_some() {
                        text_site(
                            world,
                            Subject::entity("import", id.clone()),
                            Site::Key(via_diag::KeyPath::key("footprint")),
                        )
                    } else {
                        at_import.clone()
                    };
                    diags.push(
                        text::diag(
                            codes::import::EXIT_OUTSIDE,
                            subject.clone(),
                            foot_site,
                            &format!("the exit `{id}.{fid}` lies outside `{id}`'s footprint: it would leave from ground that is not brought"),
                            "widen the `footprint` to hold the exit, or move the exit inside it",
                        )
                        .at(text::pos(p.x, p.y))
                        .related(related("the exit")),
                    );
                }
            }
            let name = format!("{id}.{fid}");
            let Some(found) = by.get(&name) else {
                let write = format!("ends_at = \"{name}\"");
                let mut dg = text::diag(
                    codes::import::EXIT_UNANSWERED,
                    subject.clone(),
                    at_import.clone(),
                    &format!("nothing here names `{name}`, an exit of `{id}`: what leaves there goes nowhere"),
                    &format!("end a path there with `{write}`, or place a feature `on` it that ends it"),
                )
                .with("write", DValue::text(write))
                .related(related("the exit"));
                if let Some(p) = at {
                    dg = dg.at(text::pos(p.x, p.y));
                }
                diags.push(dg);
                continue;
            };
            // Matched: each listed attribute, after defaults and formulas.
            let Some(theirs) = a.build.features.get(fid) else {
                continue;
            };
            for attr in list {
                let Some(want) = theirs.attrs.get(attr) else {
                    continue;
                };
                for (top, aid) in found {
                    if top != "feature" {
                        continue;
                    }
                    let got = out.features.get(aid).and_then(|f| f.attrs.get(attr));
                    if got.is_some_and(|g| same_value(g, want)) {
                        continue;
                    }
                    let mut dg = text::at_header(
                        codes::import::EXIT_MISMATCH,
                        "feature",
                        aid,
                        &match got {
                            Some(g) => format!("`{aid}` answers the exit `{name}` with `{attr}` {g}; the exit has {want}"),
                            None => format!("`{aid}` answers the exit `{name}`, and its kind declares no `{attr}`; the exit has {want}"),
                        },
                        &format!("write `{attr} = {want}` on `{aid}`"),
                    )
                    .with("attribute", DValue::text(attr.clone()))
                    .with("exit_value", data_value(want))
                    .with("write", DValue::text(format!("{attr} = {want}")))
                    .related(related("the exit it answers"));
                    if let Some(g) = got {
                        dg = dg.with("answer_value", data_value(g));
                    }
                    diags.push(dg);
                }
            }
        }
    }
}

/// The role of a parent field the import brings, for `lift` (section 5).
pub(crate) fn is_surface(d: &Decls, field: &str) -> bool {
    d.fields
        .get(field)
        .is_some_and(|f| f.role == Some(Role::Surface))
}

/// Whether a field is a length (the `surface`, or unit `m`): only a length has wall cells (6.2).
pub(crate) fn is_length(d: &Decls, field: &str) -> bool {
    d.fields
        .get(field)
        .is_some_and(|f| f.role == Some(Role::Surface) || f.unit.as_deref() == Some("m"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_turn_takes_import_points_clockwise() {
        // composition.md case `turned`: (2, 2) placed at (20, 20) facing 90 is (22, 18).
        let f = Frame::new((20_000, 20_000), 90_000, 1.0).unwrap();
        assert_eq!(f.to_world((2_000, 2_000)).unwrap(), (22_000, 18_000));
        assert_eq!(f.to_world((4_000, 0)).unwrap(), (20_000, 16_000));
        // And back, unrounded.
        let (x, y) = to_import(&f, (22_000, 18_000));
        assert!((x - 2_000.0).abs() < 1e-9 && (y - 2_000.0).abs() < 1e-9);
    }

    #[test]
    fn a_half_turn_and_an_odd_bearing_round_once_to_the_lattice() {
        let f = Frame::new((10_000, 10_000), 180_000, 1.0).unwrap();
        assert_eq!(f.to_world((2_000, 2_000)).unwrap(), (8_000, 8_000));
        let g = Frame::new((0, 0), 30_000, 1.0).unwrap();
        let (x, y) = g.to_world((1_000, 0)).unwrap();
        // cos 30 = 0.866..., sin 30 = 0.5: (866, -500), each rounded once.
        assert_eq!((x, y), (866, -500));
        let (bx, by) = to_import(&g, (x, y));
        assert!((bx - 1_000.0).abs() < 1.0 && by.abs() < 1.0);
    }

    #[test]
    fn exit_values_compare_numbers_by_value_and_text_exactly() {
        assert!(same_value("6", "6.0"));
        assert!(!same_value("6", "4"));
        assert!(same_value("wide", "wide"));
        assert!(!same_value("wide", "Wide"));
    }

    #[test]
    fn words_split_selectors_and_keep_dotted_names() {
        let mut w = BTreeSet::new();
        words(
            &Value::Str(via_doc::Str {
                text: "= dist('town.edge', 8) + given:limits".into(),
                map: Vec::new(),
            }),
            &mut w,
        );
        assert!(w.contains("town.edge") && w.contains("limits") && w.contains("given"));
    }

    #[test]
    fn overlap_of_two_rectangles_is_their_common_area() {
        let r = |x0, y0, x1, y1| {
            vec![
                P::new(x0, y0),
                P::new(x1, y0),
                P::new(x1, y1),
                P::new(x0, y1),
            ]
        };
        let a = r(0, 0, 10_000, 8_000);
        let b = r(8_000, 0, 18_000, 8_000);
        assert!((overlap_area(&a, &b) - 16.0).abs() < 1e-9);
        assert_eq!(overlap_area(&a, &r(20_000, 0, 30_000, 8_000)), 0.0);
    }
}
