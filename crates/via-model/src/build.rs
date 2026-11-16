//! The one entry: [`build`] a merged world into an in-memory build, its diagnostics and its
//! change report against a baseline build.
//!
//! The order is the doctrine's (section 3), as the edges of `rules.md` 3.4 and `surfaces.md` 4.3
//! give it: the pack; declarations and formulas; shapes and plans; fields with every entry in
//! R21 order (footprint features included); producers, held by hand edits; heights; checks;
//! meetings; the change report. Everything is in ordered maps and visited in id order, so the
//! same text gives the same build (R26).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use via_diag::{codes, Diag, Site, Subject};
use via_doc::World;
use via_geom::{Region, P};
use via_num::Grid;

use crate::decl::{Attr, Decls, Role, ShapeDecl, Tier};
use crate::pack::{self, Pack, Roots};
use crate::place::{Geo, Placed, Placer};
use crate::report::{self, ChangeReport};
use crate::resolve::{Fields, View};
use crate::rules::{self, Placement};
use crate::text;

/// What a build is asked: where to find the pack, and the baseline to report changes against.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The folder of the world file.
    pub world_dir: PathBuf,
    /// `--lib` roots, in the order given.
    pub lib_roots: Vec<PathBuf>,
    /// The entries of `VIA_LIB`.
    pub via_lib: Vec<PathBuf>,
    /// The shipped root.
    pub shipped_root: Option<PathBuf>,
    /// Per generate step whose document is joined, the folder holding the files of its `out`
    /// (`protocol.md` 5): a returned field's `file` is read there.
    pub step_out: BTreeMap<String, PathBuf>,
    /// `--stand-ins` (`assets.md` 9): model assets take their placement facts from the facts file.
    pub stand_ins: bool,
    /// A seed that replaces the world's (`composition.md` 2, an import's `seed`).
    pub seed: Option<u64>,
    /// What a parent gives this world (`composition.md` 8): `None` builds each given's default.
    pub given: Option<std::sync::Arc<crate::import::GivenIn>>,
    /// Runs an import's own generate steps before it is built (`composition.md` 12): the caller
    /// that owns step running hands it in; `None` builds an import from its text alone.
    pub import_steps: Option<crate::import::ImportSteps>,
    /// The prefix of this world's node ids in the build that holds it: `""` for the world built,
    /// `<id>.` for an import, `<id>.<nested>.` for an import's import (`composition.md` 12).
    pub prefix: String,
}

impl Options {
    /// The options a build reads from the same roots the document reader used
    /// (`documents.md` 4): the world's folder, `--lib`, `VIA_LIB`, the shipped root.
    pub fn from_load(world_path: &std::path::Path, load: &via_doc::LoadOptions) -> Options {
        Options {
            world_dir: world_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default(),
            lib_roots: load.lib_roots.clone(),
            via_lib: load
                .via_lib
                .as_ref()
                .map(|v| {
                    std::env::split_paths(v)
                        .filter(|p| !p.as_os_str().is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            shipped_root: load.shipped_root.clone(),
            step_out: BTreeMap::new(),
            stand_ins: false,
            seed: None,
            given: None,
            import_steps: None,
            prefix: String::new(),
        }
    }
}

/// A field as built.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldOut {
    pub role: Option<String>,
    pub unit: Option<String>,
    pub tags: Vec<String>,
    /// Post order, rows from the south, each west to east; `None` for a weights field.
    pub values: Option<Vec<f32>>,
    /// Per material, for a weights field.
    pub weights: BTreeMap<String, Vec<f32>>,
}

/// A feature as built.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureOut {
    pub kind: Option<String>,
    pub tier: Option<Tier>,
    /// `point`, `path` or `region`.
    pub shape: String,
    /// The points: one for a point, the path's, or the region's outer ring.
    pub points: Vec<(i64, i64)>,
    pub facing: Option<i64>,
    pub attrs: BTreeMap<String, String>,
    /// Each attribute's type: `number`, `bool` or `text` (`protocol.md` 4.3's columns). Empty for
    /// a feature read back from a snapshot, which keeps the text only.
    pub attr_types: BTreeMap<String, String>,
    /// Name attributes resolved: attribute to (place name, plan point).
    pub names: BTreeMap<String, (String, (i64, i64))>,
    /// The kind's `output`, for the loader.
    pub output: Option<String>,
    /// A region's every piece, each its rings, shell first then holes, in canonical form
    /// (`geometry.md` 2.5). Empty for a point or a path.
    pub rings: Vec<Vec<Vec<(i64, i64)>>>,
}

impl FeatureOut {
    /// Every vertex: the points, and every ring of every piece.
    pub fn all_points(&self) -> Vec<(i64, i64)> {
        let mut v = self.points.clone();
        for pc in &self.rings {
            for r in pc {
                v.extend(r.iter().copied());
            }
        }
        v
    }
}

/// An instance as built: a hand instance, a pin's, or a produced one.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanceOut {
    /// The producer, `None` for a written thing.
    pub maker: Option<String>,
    pub asset: String,
    pub origin: (i64, i64),
    /// Metres, after `sink`.
    pub z: f64,
    pub facing: i64,
    pub scale: f64,
    pub tint: f64,
    pub footprint: Vec<(i64, i64)>,
}

/// A footprint feature as built (`rules.md` 2.1), keyed by its instance's name.
#[derive(Clone, Debug, PartialEq)]
pub struct FootprintOut {
    pub kind: String,
    pub ring: Vec<(i64, i64)>,
}

/// A meeting (`surfaces.md` 2.4): a path end and a place's default mount.
#[derive(Clone, Debug, PartialEq)]
pub struct Meeting {
    pub a: String,
    pub b: String,
    pub at: (i64, i64),
    pub z_a: f64,
    pub z_b: f64,
    pub gap: i64,
    /// Each side's world bearing, millidegrees, where it has a face (M5).
    pub face_a: Option<i64>,
    pub face_b: Option<i64>,
    /// 3.4: for a path end at a place with a face.
    pub turn: Option<i64>,
    pub connector_a: Option<String>,
    pub connector_b: Option<String>,
    /// True for a `meets` pair of one owner's mounts.
    pub pair: bool,
}

/// A plan point as a message writes it, `[x, y]` in metres.
fn pt_text(p: P) -> String {
    format!(
        "[{}, {}]",
        via_num::write_length(p.x),
        via_num::write_length(p.y)
    )
}

/// A whole build, in memory.
#[derive(Clone, Debug, Default)]
pub struct Build {
    pub grid: Option<Grid>,
    /// The world's `chunk` in millimetres when written and valid (`build.md` 5); `None` tiles by
    /// the default side.
    pub chunk: Option<i64>,
    pub fields: BTreeMap<String, FieldOut>,
    pub features: BTreeMap<String, FeatureOut>,
    pub footprints: BTreeMap<String, FootprintOut>,
    pub instances: BTreeMap<String, InstanceOut>,
    pub meetings: Vec<Meeting>,
    /// Every kind surface, by place name (`surfaces.md` 1).
    pub surfaces: BTreeMap<String, crate::surfaces::SurfaceOut>,
    /// The resolved facts of every asset the build uses (`build.md` 7).
    pub assets: BTreeMap<String, BTreeMap<String, String>>,
    /// Where each used asset's declared facts are written (`assets.md` 7): asset id to fact key to
    /// (file as a record names it, 1-based line); the key `""` is the asset's header.
    pub asset_lines: BTreeMap<String, BTreeMap<String, (String, u32)>>,
    /// Each produced instance a hand edit removed, to the edit's statement (`clear.<id>`,
    /// `pin.<id>`): the cause of its removal, or of its return when the edit goes.
    pub removed_by: BTreeMap<String, String>,
    /// The lint steps the caller runs after the build (M1 runs one, through the SDK).
    pub lint_steps: Vec<(String, Vec<String>)>,
    /// Every diagnostic, raised, not yet located: emit them against the merged world.
    pub diagnostics: Vec<Diag>,
    /// What each statement holds, key by key, for the next build's causes.
    pub statements: BTreeMap<String, BTreeMap<String, String>>,
    /// The statements each built thing depends on, by (table, name).
    pub deps: BTreeMap<(String, String), BTreeSet<String>>,
    /// The change report, when a baseline was given.
    pub changes: Option<ChangeReport>,
    /// What each selection by place resolved to: `<table>.<id>` to (produced name, its point)
    /// (`identity.md` 3), for `identity.reselected` against the next build.
    pub selections: BTreeMap<String, (String, (i64, i64))>,
    /// Per field, the largest mask of any `anchored` entry at each post: what this world hands a
    /// parent that imports it (`composition.md` 6.4).
    pub cover: BTreeMap<String, Vec<f64>>,
    /// The terrain's cut quads (`build.md` 6.5), when the world has a terrain.
    pub cut: Option<Vec<u8>>,
    /// Every kind surface's mesh (`build.md` 10).
    pub meshes: Vec<crate::surfaces::MeshOut>,
    /// The kind surface a point feature or a named place stands on (`surfaces.md` 4.1, 3.3), by
    /// feature id or place name: its `z` in `features.json` is that surface's.
    pub place_surface: BTreeMap<String, String>,
}

impl Build {
    /// Whether any diagnostic is an error (exit status 1).
    pub fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity() == via_diag::Severity::Error)
    }
}

fn ring_points(r: &Region) -> Vec<(i64, i64)> {
    r.pieces
        .first()
        .map(|p| p.shell.iter().map(|q| (q.x, q.y)).collect())
        .unwrap_or_default()
}

fn attr_text(a: &Attr) -> String {
    match a {
        Attr::Number(x) => text::dbl(*x),
        Attr::YesNo(b) => b.to_string(),
        Attr::Text(t) | Attr::Json(t) => t.clone(),
    }
}

/// The statements of the merged world, key by key (`graph.md` 1, by canonical text).
pub(crate) fn statements(world: &World) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (top, e) in world.root().iter() {
        match &e.item.value {
            via_doc::Value::Table(t) if top != "let" => {
                for (id, ent) in t.iter() {
                    let mut keys = BTreeMap::new();
                    match &ent.item.value {
                        via_doc::Value::Table(et) => {
                            for (k, v) in et.iter() {
                                keys.insert(k.clone(), text::canonical(&v.item.value));
                            }
                        }
                        v => {
                            keys.insert(String::new(), text::canonical(v));
                        }
                    }
                    out.insert(format!("{top}.{id}"), keys);
                }
            }
            v => {
                out.entry("world".into())
                    .or_default()
                    .insert(top.clone(), text::canonical(v));
            }
        }
    }
    out
}

/// Builds a merged world. `baseline` is the last complete build, for the change report
/// (`identity.md` 5); without one the report says so.
pub fn build(world: &World, options: &Options, baseline: Option<&Build>) -> Build {
    let p = prepare(world, options);
    finish(world, p, BTreeMap::new(), Vec::new(), baseline)
}

/// What every later stage reads: declarations, the pack, shapes and plans.
pub(crate) struct Prepared {
    pub decls: Decls,
    pub pack: Pack,
    pub placed: crate::place::Placed,
    pub diags: Vec<Diag>,
    /// The placement pass's own diagnostics (also in `diags`), when it ran: the graph routes them
    /// to the nodes that place without running the pass again.
    pub placing: Option<Vec<Diag>>,
}

/// Reads the world, finds and reads the pack, resolves shapes and plans.
pub(crate) fn prepare(world: &World, options: &Options) -> Prepared {
    let (mut decls, pk, mut diags) = declared(world, options);
    let mut placed_now = place_rounds(&mut decls, &pk);
    // A fill is given what the parent placed, then built again with it; what the parent placed
    // from it is placed again (`composition.md` 8).
    if let Some(grid) = decls.grid {
        if crate::import::give(
            world,
            &mut decls,
            &pk,
            &placed_now.0,
            grid,
            options,
            &mut diags,
        ) {
            placed_now = place_rounds(&mut decls, &pk);
        }
    }
    let (placed, placing) = placed_now;
    if let Some(pd) = &placing {
        diags.extend(pd.iter().cloned());
    }
    Prepared {
        decls,
        pack: pk,
        placed,
        diags,
        placing,
    }
}

/// What [`prepare`] reads before it places anything: the declarations with their files, the
/// pack and the imports built alone. The build key's leaves need no more (`graph.md` 6.2), so
/// they no longer place the world: city's unchanged rebuild spent 1.6 s of its 17 s placing
/// it only to list its files (M5 close).
pub(crate) fn declared(world: &World, options: &Options) -> (Decls, Pack, Vec<Diag>) {
    let mut diags: Vec<Diag> = Vec::new();
    let mut decls = Decls::read_given(world, options.given.as_deref());
    if let Some(s) = options.seed {
        decls.seed = s;
    }
    diags.extend(decls.diags.iter().cloned());
    read_files(&mut decls, options, &mut diags);
    // A given field's values from the parent stand as a base file (`composition.md` 8).
    let given_fields = std::mem::take(&mut decls.given_fields);
    decls.files.extend(given_fields);
    // The pack (assets.md 1.2).
    let roots = Roots {
        world_dir: options.world_dir.clone(),
        lib_roots: options.lib_roots.clone(),
        via_lib: options.via_lib.clone(),
        shipped: options.shipped_root.clone(),
    };
    let mut pk = Pack::default();
    let asks_ground =
        !decls.instances.is_empty() || !decls.scatters.is_empty() || !decls.pins.is_empty();
    if let Some(written) = &decls.pack {
        if let Some(path) = pack::find(written, &roots, &mut diags) {
            let display = format!(
                "pack:{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or(written)
            );
            let (p, pd) = pack::read(&path, &display);
            diags.extend(pd);
            match p {
                Some(p) => pk = p,
                None => pk.failed = true,
            }
            if options.stand_ins {
                // `build.no_facts` is a run diagnostic (`build.md` 14): the command line raises
                // it before any node runs.
                let _ = crate::asset_facts::stand_ins(&mut pk);
            }
        } else {
            pk.failed = true;
        }
    } else if asks_ground {
        diags.push(text::diag(
            codes::pack::NOT_FOUND,
            Subject::World,
            Site::World,
            "the world places assets and names no `pack`",
            "write `pack = \"<name>\"` at the top of the world",
        ));
    }
    // `assets.md` 2.6, `fields.md` 5: a `paint` on the `material` field names a material the pack
    // binds; on a weights field without a role it is any id.
    if !pk.failed && decls.pack.is_some() {
        let names = || pk.materials.keys().map(String::as_str);
        let unknown = |m: &str| !pk.materials.contains_key(m);
        let material = Some(crate::decl::Role::Material);
        for f in decls.fields.values().filter(|f| f.role == material) {
            for row in &f.rows {
                if let Some(m) = row.paint.as_deref().filter(|m| unknown(m)) {
                    diags.push(text::with_nearest(
                        text::at_path(
                            codes::pack::UNKNOWN_MATERIAL,
                            "field",
                            &f.name,
                            &["then", &row.id, "paint"],
                            &format!("the pack binds no material `{m}`"),
                            &text::near_hint(m, names(), "add `[material.<name>]` to the pack"),
                        ),
                        m,
                        names(),
                        |n: &str| pk.declaration("material", n),
                    ));
                }
            }
        }
        for k in decls.kinds.values() {
            let to_material = |e: &&crate::decl::Effect| {
                e.selector == crate::decl::Selector::Role(crate::decl::Role::Material)
            };
            for e in k.effects.values().filter(to_material) {
                if let Some(m) = e.paint.as_deref().filter(|m| unknown(m)) {
                    diags.push(text::with_nearest(
                        text::at_path(
                            codes::pack::UNKNOWN_MATERIAL,
                            "kind",
                            &k.id,
                            &["effect", &e.id, "paint"],
                            &format!("the pack binds no material `{m}`"),
                            &text::near_hint(m, names(), "add `[material.<name>]` to the pack"),
                        ),
                        m,
                        names(),
                        |n: &str| pk.declaration("material", n),
                    ));
                }
            }
        }
    }
    // Imports, built alone and placed (`composition.md` 2, 3).
    crate::import::bring(&mut decls, &pk, options, &mut diags);
    (decls, pk, diags)
}

/// Places the world's text in rounds (`authority.md` 2.2, A produced instance gives way): rounds
/// of placing and resolving until no produced instance's footprint passes its kind's
/// `max_change`. Each round removes every instance found at once. A round must remove one, or the
/// next would find the same: a found name that is no declared instance (a scatter's, should its
/// footprints ever write here) ends the rounds with `diag.internal` instead.
fn place_rounds(decls: &mut Decls, pk: &Pack) -> (Placed, Option<Vec<Diag>>) {
    let Some(grid) = decls.grid else {
        return (crate::place::Placed::default(), None);
    };
    let mut gave: Vec<Diag> = Vec::new();
    loop {
        let (placed, mut pd) = Placer::new(decls, pk).run();
        let found = give_way(decls, &placed, grid);
        let mut removed = 0;
        let mut stuck = Vec::new();
        for (inst, d) in found {
            if decls.instances.remove(&inst).is_some() {
                decls.gave_way.insert(inst);
                gave.push(d);
                removed += 1;
            } else {
                stuck.push(inst);
            }
        }
        if removed == 0 {
            for inst in stuck {
                pd.push(text::diag(
                    codes::diag::INTERNAL,
                    Subject::entity("instance", inst.clone()),
                    Site::Header,
                    &format!("`{inst}` passes its kind's `max_change` but cannot give way: it is no declared instance"),
                    "report it: this is a defect of the engine",
                ));
            }
            pd.extend(gave);
            return (placed, Some(pd));
        }
    }
}

/// The produced instances whose footprints pass their kind's `max_change` (`authority.md` 2.2),
/// by name, each with its `authority.gave_way` record. Only the fields such a footprint writes
/// through an effect with `max_change` are resolved, and only when there is one.
fn give_way(decls: &Decls, placed: &Placed, grid: Grid) -> BTreeMap<String, Diag> {
    let mut fields: BTreeSet<&str> = BTreeSet::new();
    let mut probe = Fields::new(decls, placed, grid);
    probe.collect_gave_way = true;
    for (name, (inst, kind)) in &placed.footprints {
        if !decls.produced_instance(inst) || !placed.shapes.contains_key(name) {
            continue;
        }
        let Some(k) = decls.kinds.get(kind) else {
            continue;
        };
        for e in k.effects.values().filter(|e| e.max_change.is_some()) {
            for f in probe.matching(&e.selector) {
                if let Some((n, _)) = decls.fields.get_key_value(f.as_str()) {
                    fields.insert(n.as_str());
                }
            }
        }
    }
    if fields.is_empty() {
        return BTreeMap::new();
    }
    for f in fields {
        let _ = probe.get(f, View::Build);
    }
    probe.gave_way.into_inner()
}

/// The folder of the document a record names `source` (`documents.md` 2.1): a path under the
/// world's folder, or `lib:<id>/...` under the root that holds library `id` (section 4).
fn source_folder(source: &str, options: &Options) -> Option<PathBuf> {
    // A world named without a folder lies in the working folder; `.` keeps the case walk
    // (`documents.md` 7.2) able to list it.
    let world_dir = if options.world_dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        options.world_dir.clone()
    };
    if let Some(rest) = source.strip_prefix("lib:") {
        let (lib, file) = rest.split_once('/').unwrap_or((rest, ""));
        let root = std::iter::once(options.world_dir.join("lib"))
            .chain(options.lib_roots.iter().cloned())
            .chain(options.via_lib.iter().cloned())
            .chain(options.shipped_root.iter().cloned())
            .map(|r| r.join(lib))
            .find(|p| p.is_dir())?;
        let parent = std::path::Path::new(file).parent().map(PathBuf::from);
        return Some(parent.map_or(root.clone(), |p| root.join(p)));
    }
    if let Some(rest) = source.strip_prefix("step:") {
        // A returned document's files lie in its step's `out` (`protocol.md` 5).
        let step = rest.split_once('/').map_or(rest, |(s, _)| s);
        return options.step_out.get(step).cloned();
    }
    if source.contains(':') {
        // A change file names its folder on the command line only; text in it is read
        // against the world's folder.
        return Some(world_dir);
    }
    let parent = std::path::Path::new(source).parent().map(PathBuf::from);
    Some(parent.map_or(world_dir.clone(), |p| world_dir.join(p)))
}

/// Reads every field's `file` (`fields.md` 4): found by `documents.md` 7.2, an array file of
/// 4.1, placed by `extent`. A fault is said at the `file` key and the field has no base.
fn read_files(decls: &mut Decls, options: &Options, diags: &mut Vec<Diag>) {
    let Some(grid) = decls.grid else { return };
    let mut found = BTreeMap::new();
    for (name, fd) in &decls.fields {
        let Some(f) = &fd.file else { continue };
        let fault = |code, message: &str, hint: &str| {
            text::at_key(code, "field", name, "file", message, hint)
        };
        let Some(folder) = source_folder(&f.source, options) else {
            diags.push(fault(
                codes::doc::FILE_MISSING,
                &format!("the folder of `{}` is not found", f.source),
                "check the library roots",
            ));
            continue;
        };
        let path = match via_doc::open_file(&folder, &f.written) {
            Ok(p) => p,
            Err((code, message, hint)) => {
                diags.push(fault(code, &message, &hint));
                continue;
            }
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                diags.push(fault(
                    codes::doc::FILE_UNREADABLE,
                    &format!("the file `{}` cannot be read: {e}", f.written),
                    "check the file's permissions",
                ));
                continue;
            }
        };
        let png = f.written.to_ascii_lowercase().ends_with(".png");
        let samples = if png {
            let unit_m = decls.fields.get(name).is_some_and(|fd| {
                fd.unit.as_deref() == Some("m")
                    || (fd.unit.is_none() && fd.role == Some(crate::decl::Role::Surface))
            });
            match png_samples(name, f, &bytes, unit_m, diags) {
                Some(s) => s,
                None => continue,
            }
        } else {
            match via_raster::npy::read_samples(&bytes, via_num::POST_LIMIT) {
                Ok(s) => s,
                Err(e) => {
                    let hint = match e.code() {
                        c if c == codes::field::FILE_TOO_LARGE => {
                            "use a coarser raster, or tile the world"
                        }
                        c if c == codes::field::FILE_NONFINITE => {
                            "write a finite value at that sample"
                        }
                        _ => "write a 2-D array file of `<f4`, at least 2 by 2 (`build.md` 3.2)",
                    };
                    diags.push(fault(e.code(), &format!("`{}`: {e}", f.written), hint));
                    continue;
                }
            }
        };
        let extent = f.extent.unwrap_or(grid.extent);
        if via_raster::file::offworld(&grid, extent) {
            diags.push(text::at_key(
                codes::field::FILE_OFFWORLD,
                "field",
                name,
                if f.extent.is_some() { "extent" } else { "file" },
                "the raster's extent lies wholly outside the world's: every post takes `outside`",
                "correct `extent`",
            ));
        }
        found.insert(
            name.clone(),
            crate::decl::FileBase {
                samples,
                placement: via_raster::file::Placement {
                    extent,
                    outside: f.outside,
                    filter: f.filter,
                },
                digest: via_graph::Digest::of(&bytes).to_string(),
            },
        );
    }
    decls.files = found;
}

/// A PNG field file's samples (`fields.md` 4.1, 4.2): the one channel, its levels mapped by
/// `range`, rows from the south. Faults are said at the field's keys; `None` when it is not read.
fn png_samples(
    name: &str,
    f: &crate::decl::FileDecl,
    bytes: &[u8],
    unit_m: bool,
    diags: &mut Vec<Diag>,
) -> Option<via_raster::npy::Samples> {
    let at = |code, key: &str, message: &str, hint: &str| {
        text::at_key(code, "field", name, key, message, hint)
    };
    let p = match via_raster::png::read(bytes, via_num::POST_LIMIT) {
        Ok(p) => p,
        Err(e) => {
            diags.push(at(
                codes::field::FILE_FORMAT,
                "file",
                &format!("`{}`: {e}", f.written),
                "save as an 8- or 16-bit grey PNG, or an array file",
            ));
            return None;
        }
    };
    if p.width < 2 || p.height < 2 {
        diags.push(at(
            codes::field::FILE_FORMAT,
            "file",
            &format!(
                "`{}` is {} by {}: a raster is at least 2 by 2",
                f.written, p.width, p.height
            ),
            "save a picture at least 2 by 2",
        ));
        return None;
    }
    let ch = match (f.channel.as_deref(), p.channels) {
        (None | Some("grey"), 1 | 2) => 0,
        (Some("a"), 2) => 1,
        (Some("r"), 3 | 4) => 0,
        (Some("g"), 3 | 4) => 1,
        (Some("b"), 3 | 4) => 2,
        (Some("a"), 4) => 3,
        (None, _) => {
            diags.push(at(
                codes::field::FILE_KEY,
                "file",
                &format!(
                    "`{}` has colour channels and the field names none",
                    f.written
                ),
                "write `channel = \"r\"`, `\"g\"`, `\"b\"` or `\"a\"`",
            ));
            return None;
        }
        (Some(c), _) => {
            diags.push(at(
                codes::field::FILE_KEY,
                "channel",
                &format!("`{}` has no channel `{c}`", f.written),
                "name a channel the picture has",
            ));
            return None;
        }
    };
    let top = if p.depth == 8 { 255.0 } else { 65535.0 };
    let (lo, hi) = f.range.unwrap_or((0.0, 1.0));
    let step = (hi - lo).abs() / top;
    if unit_m && step > 0.1 {
        let d = at(
            codes::field::FILE_COARSE,
            "file",
            &format!(
                "`{}` steps {} m a level on a field in metres",
                f.written,
                (step * 1000.0).round_ties_even() / 1000.0
            ),
            "use 16-bit, or a narrower range",
        );
        // `fields.md` 4.3: the step, in metres.
        diags.push(match via_diag::Finite::new(step) {
            Some(v) => d.with("step", v),
            None => d,
        });
    }
    if !p.unapplied.is_empty() {
        diags.push(
            at(
                codes::field::FILE_PROFILE,
                "file",
                &format!(
                    "`{}`'s {} not applied: levels are read as stored",
                    f.written,
                    p.unapplied.join(", ")
                ),
                "none needed; levels are read as stored",
            )
            // `fields.md` 4.1: what was ignored, by chunk name.
            .with(
                "chunks",
                via_diag::Value::texts(p.unapplied.iter().cloned()),
            ),
        );
    }
    let (w, h) = (p.width as usize, p.height as usize);
    let mut values = Vec::with_capacity(w * h);
    // Row 0 of the samples is the south: the picture's bottom row.
    for r in (0..h).rev() {
        for c in 0..w {
            let level = f64::from(p.levels[(r * w + c) * p.channels + ch]);
            let v = lo + (hi - lo) * (level / top);
            values.push(via_num::store_f32(v).unwrap_or(0.0));
        }
    }
    Some(via_raster::npy::Samples {
        rows: p.height,
        columns: p.width,
        values,
    })
}

/// The rest of a build, with fields already resolved elsewhere (`seeds`, from the graph's
/// `field` nodes, whose diagnostics come in `seeded_diags`) taken as they are.
/// `surface.gap`'s hint (`surfaces.md` 3.4): change either side, or set the ground at the side
/// that stands on it to the other's height. A side a step returned (a produced name, R3: text may
/// not write it) is named with its producer, whose inputs are what to change (M5 close: a fresh
/// agent was told to change two names it could not write).
fn gap_hint(m: &Meeting, surfaces: &BTreeMap<String, crate::surfaces::SurfaceOut>) -> String {
    let side = |n: &str| match n.rsplit_once('#') {
        Some((producer, _)) => format!(
            "`{n}` (returned by `{producer}`: change what it read, the feature it was made for)"
        ),
        None => format!("`{n}`"),
    };
    let on_ground = |n: &str| !surfaces.contains_key(n);
    let ground = match (on_ground(&m.a), on_ground(&m.b)) {
        (true, false) => Some((&m.a, m.z_b)),
        (false, true) => Some((&m.b, m.z_a)),
        _ => None,
    };
    let mut h = format!(
        "make the two sides equal: change {} or {}",
        side(&m.a),
        side(&m.b)
    );
    if let Some((g, z)) = ground {
        h.push_str(&format!(
            ", or set the ground at `{g}` to {} m by an effect of its owner (another effect that \
             reaches the end, a way's bed, may win there: move the thing away from it)",
            text::dbl(z)
        ));
    }
    h
}

pub(crate) fn finish(
    world: &World,
    p: Prepared,
    seeds: BTreeMap<String, Option<crate::resolve::Resolved>>,
    seeded_diags: Vec<Diag>,
    baseline: Option<&Build>,
) -> Build {
    finish_from(world, &p, seeds, seeded_diags, baseline)
}

/// [`finish`] reading the prepared stage in place, so the graph finishes from its own pass.
// The body predates the borrow and keeps its `&decls`-style reads.
#[allow(clippy::needless_borrow)]
pub(crate) fn finish_from(
    world: &World,
    p: &Prepared,
    seeds: BTreeMap<String, Option<crate::resolve::Resolved>>,
    seeded_diags: Vec<Diag>,
    baseline: Option<&Build>,
) -> Build {
    let mut out = Build {
        statements: statements(world),
        ..Build::default()
    };
    let (decls, pk, placed) = (&p.decls, &p.pack, &p.placed);
    let mut diags = p.diags.clone();
    diags.extend(seeded_diags);
    for (id, s) in &decls.steps {
        if s.when == "lint" {
            out.lint_steps.push((id.clone(), s.command.clone()));
        }
    }
    let Some(grid) = decls.grid else {
        out.diagnostics = diags;
        return out;
    };
    out.grid = Some(grid);
    out.chunk = decls.chunk;
    // Fields, every one in the build's view.
    let fields = Fields::new(&decls, &placed, grid);
    for (name, r) in seeds {
        fields.seed(&name, View::Build, r);
    }
    fields.report_no_field();
    for name in decls.fields.keys() {
        let _ = fields.get(name, View::Build);
    }
    // Producers and hand edits, then heights and checks.
    let mut hand = rules::hand(&decls, &placed, &grid, &mut diags);
    crate::sets::runs_give_way(&decls, &pk, &placed.shapes, &mut hand, &mut diags);
    let produced = rules::scatter_all(&decls, &pk, &placed, &fields, &hand);
    diags.extend(produced.diags);
    // Pins placed on a scatter's instance stand where any pin does (`rules::hand`'s check).
    let mut late: Vec<Placement> = produced.pins;
    let mut late_diags = Vec::new();
    late.retain(|p| {
        let [x0, y0, x1, y1] = grid.extent;
        let o = p.origin;
        let inside = o.x >= x0 && o.x <= x1 && o.y >= y0 && o.y <= y1;
        if !inside {
            late_diags.push(
                text::at_header(
                    codes::place::OUTSIDE,
                    "pin",
                    &p.name,
                    &format!("`{}` stands outside the extent; not placed", p.name),
                    "move it inside the extent, or widen the extent",
                )
                .at(text::pos(o.x, o.y)),
            );
        }
        inside
    });
    diags.extend(late_diags);
    for (who, (name, p)) in placed.selections.iter().chain(produced.selections.iter()) {
        out.selections
            .insert(who.clone(), (name.clone(), (p.x, p.y)));
    }
    out.removed_by = produced.removed_by;
    let mut all: Vec<Placement> = Vec::new();
    hand.retain(|p| !produced.stepped.contains(&p.name));
    all.append(&mut hand);
    all.append(&mut late);
    all.extend(produced.instances);
    rules::ground_and_check(&decls, &pk, &placed, &fields, &mut all, &mut diags);
    // Kind surfaces (`surfaces.md` 4.3, step 3), then grounding on them (step 4).
    let failed_surfaces;
    (out.surfaces, failed_surfaces) =
        crate::surfaces::kind_surfaces(&decls, &placed.shapes, &mut diags);
    let mut grounded = crate::surfaces::ground_on_surfaces(
        &decls,
        &pk,
        &out.surfaces,
        &failed_surfaces,
        &mut all,
        &mut diags,
    );
    crate::surfaces::host_missing(&decls, &all, &mut grounded.gone, &mut diags);
    all.retain(|p| !grounded.gone.contains(&p.name));
    crate::surfaces::through(
        &decls,
        &pk,
        &out.surfaces,
        &all,
        &grounded.stands,
        &mut diags,
    );
    diags.extend(fields.diags.borrow().iter().cloned());
    // Outputs.
    for (name, fd) in &decls.fields {
        if let Some(r) = fields.get(name, View::Build) {
            out.fields.insert(
                name.clone(),
                FieldOut {
                    role: fd.role.map(|r| r.word().to_string()),
                    unit: fd.unit.clone().or_else(|| {
                        fd.role
                            .map(|r| if r == Role::Surface { "m" } else { "ratio" }.to_string())
                    }),
                    tags: fd.tags.clone(),
                    values: r.scalar.as_ref().map(|s| s.values().to_vec()),
                    weights: r
                        .weights
                        .iter()
                        .map(|(k, v)| (k.clone(), v.values().to_vec()))
                        .collect(),
                },
            );
            let mut deps: BTreeSet<String> = frame_deps();
            deps.insert(format!("field.{name}"));
            for w in &r.writers {
                deps.insert(w.clone());
            }
            for f in &r.reads {
                deps.insert(format!("field.{f}"));
            }
            // A field's `file` is a cause of its own, `file:<path>` (`identity.md` 5), held as
            // a statement of the file's digest so an edit in place is found (`build.md` 11.2).
            if let (Some(f), Some(b)) = (&fd.file, decls.files.get(name)) {
                let key = format!("file:{}", crate::graph::input_path(&f.source, &f.written));
                out.statements
                    .entry(key.clone())
                    .or_default()
                    .insert(String::new(), b.digest.clone());
                deps.insert(key);
            }
            out.deps.insert(("field".into(), name.clone()), deps);
        }
    }
    // The terrain's cut and the kind surfaces' meshes (`build.md` 6.5, 10).
    if let Some(t) = fields.surface().and_then(|t| out.fields.get(t)) {
        let quads = t
            .values
            .as_ref()
            .map(|v| crate::surfaces::cut_quads(&grid, v, &out.surfaces, &mut diags));
        out.cut = quads;
    }
    out.meshes = crate::surfaces::meshes(&grid, &out.surfaces, out.cut.as_deref(), &pk, &mut diags);
    out.place_surface = crate::surfaces::place_surfaces(&decls, &placed, &out.surfaces, &mut diags);
    for (id, f) in &decls.features {
        let Some(g) = placed.shapes.get(id) else {
            continue;
        };
        let (shape, points, facing) = match g {
            Geo::Point { p, facing } => ("point", vec![(p.x, p.y)], *facing),
            Geo::Path { line } => ("path", line.pts.iter().map(|q| (q.x, q.y)).collect(), None),
            Geo::Region { region, .. } => ("region", ring_points(region), None),
        };
        let rings = match g {
            Geo::Region { region, .. } => region
                .pieces
                .iter()
                .map(|pc| {
                    std::iter::once(&pc.shell)
                        .chain(pc.holes.iter())
                        .map(|r| r.iter().map(|q| (q.x, q.y)).collect())
                        .collect()
                })
                .collect(),
            _ => Vec::new(),
        };
        let kind = f.kind.as_ref().and_then(|k| decls.kinds.get(k));
        let names = placed
            .names
            .iter()
            .filter(|((fid, _), _)| fid == id)
            .map(|((_, a), (n, p))| (a.clone(), (n.clone(), (p.x, p.y))))
            .collect();
        out.features.insert(
            id.clone(),
            FeatureOut {
                kind: f.kind.clone(),
                tier: f.kind.as_ref().map(|_| {
                    f.tier
                        .or(kind.and_then(|k| k.authority))
                        .unwrap_or(Tier::Derived)
                }),
                shape: shape.into(),
                points,
                facing,
                attrs: f
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), attr_text(v)))
                    .collect(),
                attr_types: f
                    .attrs
                    .iter()
                    .map(|(k, v)| {
                        let t = match v {
                            Attr::Number(_) => "number",
                            Attr::YesNo(_) => "bool",
                            Attr::Text(_) => "text",
                            // `protocol.md` 4.3: a str column of canonical JSON texts.
                            Attr::Json(j) if j.starts_with('[') => "list",
                            Attr::Json(_) => "table",
                        };
                        (k.clone(), t.to_string())
                    })
                    .collect(),
                names,
                output: kind.and_then(|k| k.output.clone()),
                rings,
            },
        );
    }
    // A network's areas and a cells table's cells and rest pieces (`build.md` 9.1, `geometry.md`
    // 8): region features under their produced names, of the table's kind (rest: none).
    for (name, kind, region) in crate::makers::region_features(&placed.made) {
        let k = kind.as_ref().and_then(|k| decls.kinds.get(k));
        out.features.insert(
            name,
            FeatureOut {
                tier: kind
                    .as_ref()
                    .map(|_| k.and_then(|k| k.authority).unwrap_or(Tier::Derived)),
                kind,
                shape: "region".into(),
                points: ring_points(&region),
                facing: None,
                attrs: BTreeMap::new(),
                attr_types: BTreeMap::new(),
                names: BTreeMap::new(),
                output: k.and_then(|k| k.output.clone()),
                rings: region
                    .pieces
                    .iter()
                    .map(|pc| {
                        std::iter::once(&pc.shell)
                            .chain(pc.holes.iter())
                            .map(|r| r.iter().map(|q| (q.x, q.y)).collect())
                            .collect()
                    })
                    .collect(),
            },
        );
    }
    for (name, (inst, kind)) in &placed.footprints {
        if let Some(Geo::Region { region, .. }) = placed.shapes.get(name) {
            out.footprints.insert(
                inst.clone(),
                FootprintOut {
                    kind: kind.clone(),
                    ring: ring_points(region),
                },
            );
        }
    }
    let mut used: BTreeSet<String> = BTreeSet::new();
    for p in &all {
        used.insert(p.asset.clone());
        out.instances.insert(
            p.name.clone(),
            InstanceOut {
                maker: p.maker.clone(),
                asset: p.asset.clone(),
                origin: (p.origin.x, p.origin.y),
                z: p.z,
                facing: p.facing,
                scale: p.scale,
                tint: p.tint,
                footprint: ring_points(&p.footprint),
            },
        );
    }
    // `assets.md` 8.3 and 4.2: each used asset's own faults, and its revision against the facts
    // file, naming every instance of it in this build.
    diags.extend(
        pk.used_diagnostics(
            &used,
            out.instances
                .iter()
                .map(|(n, i)| (n.as_str(), i.asset.as_str())),
        ),
    );
    for a in &used {
        if let Some(asset) = pk.assets.get(a) {
            out.assets.insert(a.clone(), asset.facts());
        }
        if let Some(lines) = pk.lines.get(a) {
            out.asset_lines.insert(
                a.clone(),
                lines
                    .iter()
                    .map(|(k, l)| (k.clone(), (pk.display.clone(), *l)))
                    .collect(),
            );
        }
    }
    out.cover = fields.covers.borrow().clone();
    // Imports (`composition.md` 3, 6.6, 6.7): their instances and features under scoped names,
    // standing on this world's finished surface; then the seam and the places the join alters.
    if !decls.imported.is_empty() {
        let surface = fields.surface();
        let z_at = |p: P| surface.and_then(|s| fields.read(s, View::Build, p));
        for imp in decls.imported.values() {
            imp.bring_out(&grid, &mut out, &z_at);
        }
        let mut weights = BTreeMap::new();
        for id in decls.imported.keys() {
            if let Some(w) = fields.import_weight(id) {
                weights.insert(id.clone(), w.as_ref().clone());
            }
        }
        let sz: Option<(String, Vec<f64>)> = surface.and_then(|s| {
            let r = fields.get(s, View::Build)?;
            let v = r
                .scalar
                .as_ref()?
                .values()
                .iter()
                .map(|x| f64::from(*x))
                .collect();
            Some((s.to_string(), v))
        });
        // A field taken from the graph's nodes was not resolved here, so its cover was not
        // gathered: resolve the surface once more to have it.
        if let Some(s) = surface.filter(|s| !out.cover.contains_key(*s)) {
            let again = Fields::new(&decls, &placed, grid);
            let _ = again.get(s, View::Build);
            let c = again.covers.borrow().get(s).cloned();
            if let Some(c) = c {
                out.cover.insert(s.to_string(), c);
            }
        }
        let anchored1: Vec<bool> = surface
            .and_then(|s| out.cover.get(s))
            .map(|c| c.iter().map(|m| *m >= 1.0).collect())
            .unwrap_or_else(|| vec![false; grid.len() as usize]);
        let mut named: BTreeSet<String> = BTreeSet::new();
        for f in decls.features.values() {
            named.extend(f.starts_at.iter().cloned());
            named.extend(f.ends_at.iter().cloned());
            if let ShapeDecl::Point(crate::decl::Place::On(o)) = &f.shape {
                named.insert(o.reference.clone());
            }
        }
        for i in decls.instances.values() {
            if let crate::decl::Place::On(o) = &i.place {
                named.insert(o.reference.clone());
            }
        }
        crate::import::join_checks(
            decls,
            &grid,
            sz.as_ref().map(|(n, v)| (n.as_str(), v.as_slice())),
            &weights,
            &anchored1,
            &named,
            &mut diags,
        );
        crate::import::exit_checks(world, decls, &out, &mut diags);
    }
    // Meetings (surfaces.md 3.3, 3.4): path ends at a place, and `meets` pairs.
    let surface = fields.surface();
    let terrain = |p: P| surface.and_then(|s| fields.read(s, View::Build, p));
    let mut mounts =
        crate::surfaces::kind_mounts(&decls, &placed.shapes, &out.surfaces, &terrain, &mut diags);
    // The surface a placed instance stands on (4.1), sampled at a plan point: a kind surface's
    // `base` there, else the terrain (3.2, 3.3).
    let stands_z = |iid: &str, p: P, diags: &mut Vec<Diag>| -> Option<f64> {
        match grounded.stands.get(iid).and_then(|n| out.surfaces.get(n)) {
            Some(s) => s.z_in_build((p.x, p.y), diags).ok().flatten(),
            None => terrain(p),
        }
    };
    // Asset mounts of placed instances (3.2): on the surface the instance stands on. An instance
    // not placed offers no mounts (4.1).
    for (iid, pl) in &placed.plans {
        if pl.asset.is_empty() || grounded.gone.contains(iid) {
            continue;
        }
        let Some(asset) = pk.assets.get(&pl.asset) else {
            continue;
        };
        let Ok(f) = via_num::Frame::new((pl.origin.x, pl.origin.y), pl.facing, pl.scale) else {
            continue;
        };
        for (mn, (at, face)) in &asset.mounts {
            let Ok(w) = f.to_world(*at) else { continue };
            let p = P::new(w.0, w.1);
            mounts.insert(
                format!("{iid}.{mn}"),
                crate::surfaces::Side {
                    p: Some(p),
                    z: stands_z(iid, p, &mut diags),
                    face: Some(f.facing_to_world(*face)),
                    connector: asset.connectors.get(mn).cloned(),
                    mount: true,
                },
            );
        }
    }
    let round_mm = |z: f64| via_num::round_to_lattice(z * 1000.0).unwrap_or(0);
    let path_end = |id: &str, end: &str| -> Option<(P, i64)> {
        let Some(Geo::Path { line }) = placed.shapes.get(id) else {
            return None;
        };
        let n = line.pts.len();
        let (p, q) = if end == "end" {
            (line.pts[n - 1], line.pts[n - 2])
        } else {
            (line.pts[0], line.pts[1])
        };
        // The arrival bearing (3.4): along the end tangent for `ends_at`, the reversed start
        // tangent for `starts_at`; both point from the path into the place.
        let b = via_num::bearing(p.x - q.x, p.y - q.y).unwrap_or(0);
        Some((p, b))
    };
    let mut which_said: BTreeSet<String> = BTreeSet::new();
    let mut end_z = |id: &str, end: &str, p: P, diags: &mut Vec<Diag>| -> Option<f64> {
        match crate::surfaces::own_surface(&decls, &out.surfaces, id) {
            Ok(None) => terrain(p),
            Ok(Some(s)) => match s.z_in_build((p.x, p.y), diags) {
                Ok(Some(z)) => Some(z),
                Ok(None) => {
                    // A path end outside its own surface's bound (3.3, 4.1).
                    diags.push(
                        text::diag(
                            codes::surface::OUTSIDE,
                            Subject::entity("feature", id),
                            Site::Header,
                            &format!(
                                "the {end} of `{id}` at {} lies outside the bound of `{}`",
                                pt_text(p),
                                s.place
                            ),
                            &format!("move the {end} onto `{}`", s.place),
                        )
                        .with("surface", via_diag::Value::text(s.place.clone())),
                    );
                    None
                }
                Err(()) => None,
            },
            Err(names) => {
                if which_said.insert(id.to_string()) {
                    diags.push(text::diag(
                        codes::surface::WHICH,
                        Subject::entity("feature", id),
                        Site::Header,
                        &format!(
                            "the ends of `{id}` lie on one of its kind's surfaces, and it has {}",
                            names.join(", ")
                        ),
                        "end the path at a mount",
                    ));
                }
                None
            }
        }
    };
    let mut meetings = Vec::new();
    for (id, f) in &decls.features {
        let ShapeDecl::Path(_) = f.shape else {
            continue;
        };
        for (end, place) in [("start", &f.starts_at), ("end", &f.ends_at)] {
            let Some(place) = place else { continue };
            let Some((p, arrival)) = path_end(id, end) else {
                continue;
            };
            let a = format!("{id}.{end}");
            let Some(z_a) = end_z(id, end, p, &mut diags) else {
                continue;
            };
            let connector_a = crate::surfaces::path_connector(&decls, id);
            // The place's default mount (3.3). A place on an instance that was not placed offers
            // nothing: the path end is `surface.host_missing` (4.1).
            let head = place.split('.').next().unwrap_or(place);
            if grounded.gone.contains(head) {
                diags.push(
                    text::diag(
                        codes::surface::HOST_MISSING,
                        Subject::entity("feature", id.as_str()),
                        Site::Key(text::path(&[if end == "start" {
                            "starts_at"
                        } else {
                            "ends_at"
                        }])),
                        &format!(
                            "the {end} of `{id}` is at `{place}`, and `{head}` was not placed"
                        ),
                        &format!("fix `{head}` first: what meets it is checked with it"),
                    )
                    .related(via_diag::Related {
                        subject: Subject::entity("instance", head),
                        site: Site::Header,
                        position: None,
                        note: via_diag::text!("the host, not placed"),
                    }),
                );
                continue;
            }
            let side = if let Some(m) = mounts.get(place.as_str()) {
                m.clone()
            } else if let Some(s) = out.surfaces.get(place.as_str()) {
                match s.z_in_build((p.x, p.y), &mut diags) {
                    Err(()) => continue,
                    Ok(Some(z)) => crate::surfaces::Side {
                        p: Some(p),
                        z: Some(z),
                        ..Default::default()
                    },
                    Ok(None) => {
                        diags.push(
                            text::diag(
                                codes::surface::OUTSIDE,
                                Subject::entity("feature", id.as_str()),
                                Site::Key(text::path(&[if end == "start" {
                                    "starts_at"
                                } else {
                                    "ends_at"
                                }])),
                                &format!(
                                    "the {end} of `{id}` at {} lies outside the bound of `{place}`",
                                    pt_text(p)
                                ),
                                &format!("move the {end} onto `{place}`"),
                            )
                            .with("place", via_diag::Value::text(place.clone())),
                        );
                        continue;
                    }
                }
            } else if let Some((other, oend)) = place
                .strip_suffix(".start")
                .map(|o| (o, "start"))
                .or_else(|| place.strip_suffix(".end").map(|o| (o, "end")))
                .filter(|(o, _)| decls.features.contains_key(*o))
            {
                let q = path_end(other, oend).map(|x| x.0).unwrap_or(p);
                crate::surfaces::Side {
                    p: Some(q),
                    z: end_z(other, oend, q, &mut diags),
                    face: None,
                    connector: crate::surfaces::path_connector(&decls, other),
                    mount: false,
                }
            } else if let Some(Geo::Point { p: q, facing }) = placed.shapes.get(place.as_str()) {
                // A point feature: its face when it writes one (M5).
                let written = decls.features.get(place.as_str()).and_then(|f| f.face);
                // On the surface it stands on (3.3, 4.1): its `surface`, or what its `on` names.
                let z = match out
                    .place_surface
                    .get(place.as_str())
                    .and_then(|n| out.surfaces.get(n))
                {
                    Some(s) => match s.z_in_build((q.x, q.y), &mut diags) {
                        Ok(Some(z)) => Some(z),
                        Ok(None) => s.base.eval(&[(q.x, q.y)]).ok().map(|v| v[0]),
                        Err(()) => continue,
                    },
                    None => terrain(*q),
                };
                crate::surfaces::Side {
                    p: Some(*q),
                    z,
                    face: written.and(*facing),
                    ..Default::default()
                }
            } else if let Some(pl) = placed.plans.get(place.as_str()) {
                crate::surfaces::Side {
                    p: Some(pl.origin),
                    z: stands_z(place, pl.origin, &mut diags),
                    face: Some(pl.facing),
                    ..Default::default()
                }
            } else {
                crate::surfaces::Side {
                    p: Some(p),
                    z: terrain(p),
                    ..Default::default()
                }
            };
            let z_b = side.z.unwrap_or(z_a);
            let gap = (round_mm(z_a) - round_mm(z_b)).abs();
            meetings.push(Meeting {
                a,
                b: place.clone(),
                at: (p.x, p.y),
                z_a,
                z_b,
                gap,
                face_a: None,
                face_b: side.face,
                turn: side.face.map(|fc| crate::surfaces::turn(arrival, fc)),
                connector_a,
                connector_b: side.connector,
                pair: false,
            });
        }
    }
    // `meets` pairs (3.1): one meeting per pair, `a` and `b` ordered bytewise.
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for (id, f) in &decls.features {
        let Some(k) = f.kind.as_ref().and_then(|k| decls.kinds.get(k)) else {
            continue;
        };
        for m in k.mounts.values() {
            let Some(other) = m.meets.as_ref().filter(|o| k.mounts.contains_key(*o)) else {
                continue;
            };
            if *other == m.name {
                continue;
            }
            let (x, y) = (format!("{id}.{}", m.name), format!("{id}.{other}"));
            let (a, b) = if x.as_bytes() < y.as_bytes() {
                (x, y)
            } else {
                (y, x)
            };
            pairs.insert((a, b));
        }
    }
    for (a, b) in pairs {
        let (Some(sa), Some(sb)) = (mounts.get(&a), mounts.get(&b)) else {
            continue;
        };
        let (Some(pa), Some(pb)) = (sa.p, sb.p) else {
            continue;
        };
        if pa != pb {
            diags.push(
                text::diag(
                    codes::surface::APART,
                    crate::surfaces::place_subject(&decls, &a),
                    Site::Header,
                    &format!(
                        "the paired mounts `{a}` at {} and `{b}` at {} lie apart",
                        pt_text(pa),
                        pt_text(pb)
                    ),
                    "write the two mounts at one plan point",
                )
                .with("place", via_diag::Value::text(a.clone())),
            );
            continue;
        }
        let (Some(z_a), Some(z_b)) = (sa.z, sb.z) else {
            continue;
        };
        if let (Some(fa), Some(fb)) = (sa.face, sb.face) {
            if (fa - fb).rem_euclid(360_000) != 180_000 {
                diags.push(
                    text::diag(
                        codes::surface::FACING,
                        crate::surfaces::place_subject(&decls, &a),
                        Site::Header,
                        &format!(
                            "`{a}` faces {} and `{b}` faces {}; paired mounts face opposite ways",
                            via_num::write_facing(fa),
                            via_num::write_facing(fb)
                        ),
                        "turn one mount's `face` by 180 degrees from the other's",
                    )
                    .with("place", via_diag::Value::text(a.clone()))
                    .with("face_a", via_diag::Value::Int(fa))
                    .with("face_b", via_diag::Value::Int(fb)),
                );
            }
        }
        meetings.push(Meeting {
            a,
            b,
            at: (pa.x, pa.y),
            z_a,
            z_b,
            gap: (round_mm(z_a) - round_mm(z_b)).abs(),
            face_a: sa.face,
            face_b: sb.face,
            turn: None,
            connector_a: sa.connector.clone(),
            connector_b: sb.connector.clone(),
            pair: true,
        });
    }
    // What must hold at each (3.4): height and joint.
    for m in &meetings {
        if m.gap != 0 {
            diags.push(
                text::diag(
                    codes::surface::GAP,
                    crate::surfaces::place_subject(&decls, &m.a),
                    Site::Header,
                    &format!(
                        "`{}` is at {} m and `{}` at {} m: a gap of {} mm",
                        m.a,
                        text::dbl(m.z_a),
                        m.b,
                        text::dbl(m.z_b),
                        m.gap
                    ),
                    &gap_hint(m, &out.surfaces),
                )
                .with("place", via_diag::Value::text(m.a.clone()))
                .with("gap", via_diag::Value::Int(m.gap)),
            );
        }
        if let (Some(ca), Some(cb)) = (&m.connector_a, &m.connector_b) {
            if ca != cb {
                diags.push(
                    text::diag(
                        codes::surface::CONNECTOR,
                        crate::surfaces::place_subject(&decls, &m.a),
                        Site::Header,
                        &format!("`{}` has connector `{ca}` and `{}` has `{cb}`", m.a, m.b),
                        &format!("meet `{}` with a `{ca}`, or change one connector", m.b),
                    )
                    .with("place", via_diag::Value::text(m.a.clone()))
                    .with("connector_a", via_diag::Value::text(ca.clone()))
                    .with("connector_b", via_diag::Value::text(cb.clone())),
                );
            }
        }
    }
    crate::surfaces::unmet(&decls, &mounts, &meetings, &mut diags);
    out.meetings = meetings;
    out.meetings
        .sort_by(|x, y| (x.a.as_bytes(), x.b.as_bytes()).cmp(&(y.a.as_bytes(), y.b.as_bytes())));
    unowned_crossings(&decls, &placed, &mut diags);
    // Dependencies, for causes.
    deps_of(&decls, &placed, &mut out);
    // What comes from an import depends on its table (`composition.md` 11: causes in the
    // parent's terms where the import moved); so does what names a place inside one.
    if !decls.imported.is_empty() {
        let tables = ["feature", "instance", "field"];
        let layers: Vec<String> = out
            .statements
            .keys()
            .filter(|k| k.starts_with("layer."))
            .cloned()
            .collect();
        let names: Vec<(String, String)> = out
            .features
            .keys()
            .map(|n| ("feature".to_string(), n.clone()))
            .chain(
                out.instances
                    .keys()
                    .map(|n| ("instance".to_string(), n.clone())),
            )
            .chain(out.fields.keys().map(|n| ("field".to_string(), n.clone())))
            .collect();
        let refs = |t: &str, n: &str| -> BTreeSet<String> {
            let mut v = BTreeSet::new();
            if let Some((h, _)) = n.split_once('.') {
                if decls.imported.contains_key(h) {
                    v.insert(format!("import.{h}"));
                }
            }
            if t == "feature" {
                if let Some(f) = decls.features.get(n) {
                    for r in f.starts_at.iter().chain(f.ends_at.iter()) {
                        if let Some((h, _)) = r.split_once('.') {
                            if decls.imported.contains_key(h) {
                                v.insert(format!("import.{h}"));
                            }
                        }
                    }
                }
            }
            if t == "field" {
                for (id, imp) in &decls.imported {
                    if imp.alone.decls.fields.contains_key(n) {
                        v.insert(format!("import.{id}"));
                        // Layer rank orders the import's entries against the parent's (R29).
                        v.extend(layers.iter().cloned());
                    }
                }
            }
            v
        };
        for (t, n) in names {
            let mut add = refs(&t, &n);
            // A brought thing also depends on the import's own statements, in the import's
            // terms (`composition.md` 11: `b.feature.landing.path` where it changed inside).
            if let Some((h, rest)) = n.split_once('.') {
                if let Some(imp) = decls.imported.get(h) {
                    if let Some(theirs) = imp.alone.build.deps.get(&(t.clone(), rest.to_string())) {
                        add.extend(
                            theirs
                                .iter()
                                .filter(|s| !s.starts_with("asset:"))
                                .map(|s| format!("{h}.{s}")),
                        );
                    }
                }
            }
            if add.is_empty() || !tables.contains(&t.as_str()) {
                continue;
            }
            out.deps.entry((t, n)).or_default().extend(add);
        }
        for (id, imp) in &decls.imported {
            for (s, m) in &imp.alone.build.statements {
                out.statements.insert(format!("{id}.{s}"), m.clone());
            }
            // The import's build as a whole: a cause in the parent's terms (`import.<id>`) of
            // what changed inside it where no statement of its own says more.
            out.statements
                .entry(format!("import.{id}"))
                .or_default()
                .insert(String::new(), imp.alone.digest.clone());
        }
    }
    out.diagnostics = diags;
    out.changes = Some(report::compare(baseline, &out));
    // A selection that resolved to another name than in the baseline (`identity.md` 3): a run
    // diagnostic, never in `diagnostics.json`.
    if let Some(b) = baseline {
        for (who, (name, p)) in &out.selections {
            let Some((old, q)) = b.selections.get(who) else {
                continue;
            };
            if old == name {
                continue;
            }
            let (top, id) = who.split_once('.').unwrap_or(("world", who));
            let moved = via_num::hypot((p.0 - q.0) as f64, (p.1 - q.1) as f64) / 1000.0;
            out.diagnostics.push(
                text::at_key(
                    codes::identity::RESELECTED,
                    top,
                    id,
                    "on",
                    &format!(
                        "the selection now resolves to `{name}`, not `{old}` as in the baseline; moved {} m",
                        text::dbl(moved)
                    ),
                    "check that the thing meant is still the one selected; move `near` if not",
                )
                .at(text::pos(p.0, p.1)),
            );
        }
    }
    if baseline.is_none() {
        out.diagnostics.push(text::diag(
            codes::identity::NO_PREVIOUS,
            Subject::World,
            Site::World,
            "no baseline build to report changes against",
            "none needed; the next build has one",
        ));
    }
    out
}

/// The frame's keys as dependencies (`world.<key>`, `graph.md` 1): a world key enters a thing's
/// causes only when it is a key of the frame, or `pack` for a thing that resolves an asset name.
pub(crate) fn frame_deps() -> BTreeSet<String> {
    crate::graph::FRAME_KEYS
        .iter()
        .map(|k| format!("world.{k}"))
        .collect()
}

/// The statements a feature, instance or pin depends on, following its placement references.
pub(crate) fn feature_deps(d: &Decls, id: &str, seen: &mut BTreeSet<String>) -> BTreeSet<String> {
    let mut s = frame_deps();
    if !seen.insert(id.to_string()) {
        return s;
    }
    let base = id.split('.').next().unwrap_or(id);
    // A network's or cells table's output (`geometry.md` 8): its table and what its keys read.
    let maker = base.split('#').next().unwrap_or(base);
    if let Some(top) = d.maker_top(maker) {
        s.insert(format!("{top}.{maker}"));
        for n in d.maker_reads(maker) {
            s.extend(feature_deps(d, &n, seen));
        }
        return s;
    }
    if let Some(f) = d.features.get(base) {
        s.insert(format!("feature.{base}"));
        if let Some(k) = &f.kind {
            s.insert(format!("kind.{k}"));
        }
        let mut refs: Vec<String> = Vec::new();
        match &f.shape {
            ShapeDecl::Point(crate::decl::Place::On(o)) => refs.push(o.reference.clone()),
            ShapeDecl::Point(crate::decl::Place::Of(o)) => refs.push(o.of.clone()),
            ShapeDecl::Follows(fl) => refs.push(fl.reference.clone()),
            ShapeDecl::Where(_) | ShapeDecl::Path(_) => {
                let w = match &f.shape {
                    ShapeDecl::Where(w) => Some(*w),
                    _ => f.route,
                };
                if let Some(p) = w.and_then(|w| d.formulas.programs.get(&w)) {
                    for (class, sel) in &p.reads.selectors {
                        match class {
                            via_expr::Class::Field => {
                                for fd in d.fields.values() {
                                    let hit = match sel.strip_prefix("tag:") {
                                        Some(t) => fd.tags.iter().any(|x| x == t),
                                        None => fd.name == *sel,
                                    };
                                    if hit {
                                        s.insert(format!("field.{}", fd.name));
                                    }
                                }
                            }
                            via_expr::Class::Shape if !sel.starts_with("kind:") => {
                                refs.push(sel.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        refs.extend(f.starts_at.clone());
        refs.extend(f.ends_at.clone());
        refs.extend(f.surface.clone());
        refs.extend(f.way_names().into_iter().map(str::to_string));
        if let Some(k) = f.kind.as_ref().and_then(|k| d.kinds.get(k)) {
            for n in &k.names {
                if let Some(Attr::Text(t)) = f.attrs.get(n) {
                    refs.push(t.clone());
                }
            }
        }
        for r in refs {
            s.extend(feature_deps(d, &r, seen));
        }
    } else if let Some(i) = d.instances.get(base) {
        s.insert(format!("instance.{base}"));
        s.extend(d.set_stmts(i.set.as_deref()));
        s.insert(crate::graph::PACK.to_string());
        s.insert(format!("asset:{}", i.asset));
        if let Some(g) = &i.ground {
            s.insert(format!("kind.{g}"));
        }
        // The kind surface it stands on (`surfaces.md` 4.1): its owner, as an `on` reference.
        if let Some(sn) = &i.surface {
            s.extend(feature_deps(d, sn, seen));
        }
        match &i.place {
            crate::decl::Place::On(o) => s.extend(feature_deps(d, &o.reference, seen)),
            crate::decl::Place::Of(o) => s.extend(feature_deps(d, &o.of, seen)),
            crate::decl::Place::At(_) => {}
        }
    } else if let Some(p) = d.pins.get(base) {
        s.insert(format!("pin.{base}"));
        s.insert(crate::graph::PACK.to_string());
        s.insert(format!("asset:{}", p.asset));
        match &p.place {
            crate::decl::Place::On(o) => s.extend(feature_deps(d, &o.reference, seen)),
            crate::decl::Place::Of(o) => s.extend(feature_deps(d, &o.of, seen)),
            crate::decl::Place::At(_) => {}
        }
    } else if d.scatters.contains_key(base) {
        s.insert(format!("scatter.{base}"));
    }
    s
}

/// The statements each feature, instance and footprint depends on, following placements.
fn deps_of(d: &Decls, placed: &crate::place::Placed, out: &mut Build) {
    let mut feature_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for id in d.features.keys() {
        let s = feature_deps(d, id, &mut BTreeSet::new());
        feature_sets.insert(id.clone(), s.clone());
        out.deps.insert(("feature".into(), id.clone()), s);
    }
    for m in placed.made.values() {
        for t in &m.things {
            if matches!(t.geo, Geo::Region { .. }) {
                let s = feature_deps(d, &t.name, &mut BTreeSet::new());
                out.deps.insert(("feature".into(), t.name.clone()), s);
            }
        }
    }
    for id in d.instances.keys().chain(d.pins.keys()) {
        let s = feature_deps(d, id, &mut BTreeSet::new());
        out.deps.insert(("instance".into(), id.clone()), s.clone());
        if placed.footprints.contains_key(&format!("{id}.ground")) {
            out.deps.insert(("footprint".into(), id.clone()), s.clone());
            feature_sets.insert(format!("{id}.ground"), s);
        }
    }
    // Fields: expand the writers they hold into their statements.
    let field_keys: Vec<(String, String)> = out
        .deps
        .keys()
        .filter(|(t, _)| t == "field")
        .cloned()
        .collect();
    for _ in 0..field_keys.len().max(1) {
        for k in &field_keys {
            let cur = out.deps[k].clone();
            let mut next = BTreeSet::new();
            for s in &cur {
                if let Some(fs) = feature_sets.get(s) {
                    next.extend(fs.iter().cloned());
                } else if let Some(name) = s.strip_prefix("field.") {
                    next.insert(s.clone());
                    if let Some(other) = out.deps.get(&("field".to_string(), name.to_string())) {
                        next.extend(
                            other
                                .iter()
                                .filter(|x| !feature_sets.contains_key(*x))
                                .cloned(),
                        );
                    }
                } else {
                    next.insert(s.clone());
                }
            }
            out.deps.insert(k.clone(), next);
        }
    }
    // Produced instances: their producer, the surface and keep_out fields, the hand instances
    // they give way to, and the hand edits that speak to them.
    let mut ground_fields: BTreeSet<String> = BTreeSet::new();
    for (name, fd) in &d.fields {
        if matches!(fd.role, Some(Role::Surface) | Some(Role::KeepOut)) {
            if let Some(s) = out.deps.get(&("field".to_string(), name.clone())) {
                ground_fields.extend(s.iter().cloned());
            }
        }
    }
    let names: Vec<(String, Option<String>)> = out
        .instances
        .iter()
        .map(|(n, i)| (n.clone(), i.maker.clone()))
        .collect();
    for (name, maker) in names {
        let mut s = match &maker {
            Some(m) => {
                let mut s = frame_deps();
                match d.runs.get(m) {
                    // A run's member reads the run, the sets it places and its stretch.
                    Some(r) => {
                        s.insert(format!("run.{m}"));
                        if let crate::sets::What::Set(set) = &r.what {
                            s.extend(d.set_stmts(Some(set)));
                        }
                        if let Some(g) = &r.ground {
                            s.insert(format!("kind.{g}"));
                        }
                        s.extend(feature_deps(d, r.reference(), &mut BTreeSet::new()));
                    }
                    None => {
                        s.insert(format!("scatter.{m}"));
                        // `surface` (`surfaces.md` 4.1): the owner of the surface it stands on.
                        if let Some(sn) = d.scatters.get(m).and_then(|x| x.surface.as_ref()) {
                            s.extend(feature_deps(d, sn, &mut BTreeSet::new()));
                        }
                    }
                }
                s.insert(crate::graph::PACK.to_string());
                s.extend(ground_fields.iter().cloned());
                for i in d.instances.keys() {
                    s.extend(
                        out.deps
                            .get(&("instance".to_string(), i.clone()))
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
                for c in d.clears.values() {
                    if c.by.as_ref().is_none_or(|b| b.contains(m)) {
                        s.insert(format!("clear.{}", c.id));
                    }
                }
                for p in d.pins.values() {
                    if p.by.as_ref().is_none_or(|b| b.contains(m)) {
                        s.insert(format!("pin.{}", p.id));
                    }
                }
                s
            }
            // A set's item reads what its placing instance reads (`rules.md` 4).
            None => out
                .deps
                .get(&("instance".to_string(), name.clone()))
                .or_else(|| {
                    let root = name.split('.').next().unwrap_or(&name);
                    out.deps.get(&("instance".to_string(), root.to_string()))
                })
                .cloned()
                .unwrap_or_default(),
        };
        if let Some(i) = out.instances.get(&name) {
            s.insert(format!("asset:{}", i.asset));
        }
        if let Some(sf) = d.fields.values().find(|f| f.role == Some(Role::Surface)) {
            if let Some(fs) = out.deps.get(&("field".to_string(), sf.name.clone())) {
                s.extend(fs.iter().cloned());
            }
        }
        out.deps.insert(("instance".into(), name), s);
    }
    let _ = P::new(0, 0);
}

/// `geometry.md` 2.7 (R18): every point where two path features with kinds cross or touch must be
/// owned: the end of one named onto the other or onto a point on it (`starts_at`, `ends_at`), or
/// one following the other. Networks own their junctions (M3). Any other meeting is
/// `geom.unowned_crossing`, at the point, naming both. Guides (no kind) are left out.
fn unowned_crossings(decls: &Decls, placed: &Placed, diags: &mut Vec<Diag>) {
    let paths: Vec<(&String, &crate::decl::FeatureDecl, &via_geom::lineref::Line)> = decls
        .features
        .iter()
        .filter(|(_, f)| f.kind.is_some())
        .filter_map(|(id, f)| match placed.shapes.get(id) {
            Some(Geo::Path { line }) => Some((id, f, line)),
            _ => None,
        })
        .collect();
    // The feature a place name stands for: a path's end is the path; a point feature `on` a
    // path is on that path.
    let base = |r: &str| -> String {
        r.strip_suffix(".start")
            .or_else(|| r.strip_suffix(".end"))
            .unwrap_or(r)
            .to_string()
    };
    let lies_on = |name: &str, other: &str| -> bool {
        let b = base(name);
        if b == other {
            return true;
        }
        match decls.features.get(&b).map(|f| &f.shape) {
            Some(ShapeDecl::Point(crate::decl::Place::On(o))) => base(&o.reference) == other,
            _ => false,
        }
    };
    // The place `a`'s end at `p` is named onto, when `p` is such an end.
    let named_end = |a: &crate::decl::FeatureDecl, line: &via_geom::lineref::Line, p: P| {
        let first = line.pts[0];
        let last = line.pts[line.pts.len() - 1];
        let mut out = Vec::new();
        if p == first {
            out.extend(a.starts_at.as_deref().map(base));
        }
        if p == last {
            out.extend(a.ends_at.as_deref().map(base));
        }
        out
    };
    let follows = |a: &crate::decl::FeatureDecl, other: &str| matches!(&a.shape, ShapeDecl::Follows(fl) if base(&fl.reference) == other);
    // A path that names the other in an attribute its kind lists in `names` owns every meeting
    // with it: one was found from the other (a course and the river it was followed from).
    let names_other = |a: &crate::decl::FeatureDecl, other: &str| {
        let Some(k) = a.kind.as_ref().and_then(|k| decls.kinds.get(k)) else {
            return false;
        };
        k.names
            .iter()
            .any(|n| matches!(a.attrs.get(n), Some(crate::decl::Attr::Text(t)) if base(t) == other))
    };
    // Every two inputs of one network own their crossings (`geometry.md` 8.1, 2.7).
    let in_one_network = |a: &str, b: &str| {
        placed
            .made
            .values()
            .any(|m| m.inputs.iter().any(|x| x == a) && m.inputs.iter().any(|x| x == b))
    };
    for (k, (ia, fa, la)) in paths.iter().enumerate() {
        for (ib, fb, lb) in &paths[k + 1..] {
            if in_one_network(ia, ib) {
                continue;
            }
            if follows(fa, ib) || follows(fb, ia) || names_other(fa, ib) || names_other(fb, ia) {
                continue;
            }
            let mut points: BTreeSet<(i64, i64)> = BTreeSet::new();
            for sa in 0..la.segments() {
                let (a0, a1) = la.seg(sa);
                for sb in 0..lb.segments() {
                    let (b0, b1) = lb.seg(sb);
                    match via_geom::pred::meet(a0, a1, b0, b1) {
                        via_geom::pred::Meet::Point(p) => {
                            points.insert((p.x, p.y));
                        }
                        via_geom::pred::Meet::Overlap => {
                            // A shared stretch: reported at its first shared point.
                            for q in [a0, a1, b0, b1] {
                                if via_geom::pred::on_segment(q, a0, a1)
                                    && via_geom::pred::on_segment(q, b0, b1)
                                {
                                    points.insert((q.x, q.y));
                                    break;
                                }
                            }
                        }
                        via_geom::pred::Meet::None => {}
                    }
                }
            }
            for (x, y) in points {
                let p = P::new(x, y);
                // Owned: an end named onto the other or onto a point on it, or both ends named
                // onto one place, which owns the junction (ford's lanes both end at the ford).
                let (na, nb) = (named_end(fa, la, p), named_end(fb, lb, p));
                if na.iter().any(|t| lies_on(t, ib))
                    || nb.iter().any(|t| lies_on(t, ia))
                    || na.iter().any(|t| nb.contains(t))
                {
                    continue;
                }
                diags.push(
                    text::at_header(
                        codes::geom::UNOWNED_CROSSING,
                        "feature",
                        ib,
                        &format!("`{ia}` and `{ib}` meet here and neither owns the meeting"),
                        &format!(
                            "end one on the other (`ends_at = \"{ia}.end\"`), or on a point feature placed `on` it"
                        ),
                    )
                    .at(text::pos(x, y))
                    .related(via_diag::Related {
                        subject: Subject::entity("feature", ia.as_str()),
                        site: Site::Header,
                        position: None,
                        note: via_diag::text!("the other path"),
                    }),
                );
            }
        }
    }
}
