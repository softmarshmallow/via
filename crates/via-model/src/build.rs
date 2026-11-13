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
use crate::place::{Geo, Placer};
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
}

/// A whole build, in memory.
#[derive(Clone, Debug, Default)]
pub struct Build {
    pub grid: Option<Grid>,
    pub fields: BTreeMap<String, FieldOut>,
    pub features: BTreeMap<String, FeatureOut>,
    pub footprints: BTreeMap<String, FootprintOut>,
    pub instances: BTreeMap<String, InstanceOut>,
    pub meetings: Vec<Meeting>,
    /// The resolved facts of every asset the build uses (`build.md` 7).
    pub assets: BTreeMap<String, BTreeMap<String, String>>,
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
        Attr::Text(t) => t.clone(),
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
}

/// Reads the world, finds and reads the pack, resolves shapes and plans.
pub(crate) fn prepare(world: &World, options: &Options) -> Prepared {
    let mut diags: Vec<Diag> = Vec::new();
    let mut decls = Decls::read(world);
    diags.extend(decls.diags.iter().cloned());
    read_files(&mut decls, options, &mut diags);
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
                    diags.push(text::at_path(
                        codes::pack::UNKNOWN_MATERIAL,
                        "field",
                        &f.name,
                        &["then", &row.id, "paint"],
                        &format!("the pack binds no material `{m}`"),
                        &text::near_hint(m, names(), "add `[material.<name>]` to the pack"),
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
                    diags.push(text::at_path(
                        codes::pack::UNKNOWN_MATERIAL,
                        "kind",
                        &k.id,
                        &["effect", &e.id, "paint"],
                        &format!("the pack binds no material `{m}`"),
                        &text::near_hint(m, names(), "add `[material.<name>]` to the pack"),
                    ));
                }
            }
        }
    }
    let placed = match decls.grid {
        Some(_) => {
            let (placed, pd) = Placer::new(&decls, &pk).run();
            diags.extend(pd);
            placed
        }
        None => crate::place::Placed::default(),
    };
    Prepared {
        decls,
        pack: pk,
        placed,
        diags,
    }
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
        let samples = match via_raster::npy::read_samples(&bytes, via_num::POST_LIMIT) {
            Ok(s) => s,
            Err(e) => {
                let hint = match e.code() {
                    c if c == codes::field::FILE_TOO_LARGE => {
                        "use a coarser raster, or tile the world"
                    }
                    c if c == codes::field::FILE_NONFINITE => "write a finite value at that sample",
                    _ => "write a 2-D array file of `<f4`, at least 2 by 2 (`build.md` 3.2)",
                };
                diags.push(fault(e.code(), &format!("`{}`: {e}", f.written), hint));
                continue;
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

/// The rest of a build, with fields already resolved elsewhere (`seeds`, from the graph's
/// `field` nodes, whose diagnostics come in `seeded_diags`) taken as they are.
pub(crate) fn finish(
    world: &World,
    p: Prepared,
    seeds: BTreeMap<String, Option<crate::resolve::Resolved>>,
    seeded_diags: Vec<Diag>,
    baseline: Option<&Build>,
) -> Build {
    let mut out = Build {
        statements: statements(world),
        ..Build::default()
    };
    let Prepared {
        decls,
        pack: pk,
        placed,
        mut diags,
    } = p;
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
    let mut all: Vec<Placement> = Vec::new();
    all.append(&mut hand);
    all.append(&mut late);
    all.extend(produced.instances);
    rules::ground_and_check(&decls, &pk, &placed, &fields, &mut all, &mut diags);
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
            let mut deps: BTreeSet<String> =
                BTreeSet::from([format!("field.{name}"), "world".to_string()]);
            for w in &r.writers {
                deps.insert(w.clone());
            }
            for f in &r.reads {
                deps.insert(format!("field.{f}"));
            }
            out.deps.insert(("field".into(), name.clone()), deps);
        }
    }
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
                names,
                output: kind.and_then(|k| k.output.clone()),
                rings,
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
    for a in &used {
        if let Some(asset) = pk.assets.get(a) {
            out.assets.insert(a.clone(), asset.facts());
        }
    }
    // Meetings (surfaces.md 3.3): path ends at a place, on the terrain in M1.
    let surface = fields.surface();
    for (id, f) in &decls.features {
        let ShapeDecl::Path(_) = f.shape else {
            continue;
        };
        let Some(Geo::Path { line }) = placed.shapes.get(id) else {
            continue;
        };
        for (end, place) in [("start", &f.starts_at), ("end", &f.ends_at)] {
            let Some(place) = place else { continue };
            let p = if end == "start" {
                line.pts[0]
            } else {
                line.pts[line.pts.len() - 1]
            };
            let z = surface
                .and_then(|s| fields.read(s, View::Build, p))
                .unwrap_or(0.0);
            // `surfaces.md` 3.3: `a` is the path end, `b` the place's default mount.
            let (a, b) = (format!("{id}.{end}"), place.clone());
            out.meetings.push(Meeting {
                a,
                b,
                at: (p.x, p.y),
                z_a: z,
                z_b: z,
                gap: 0,
            });
        }
    }
    out.meetings
        .sort_by(|x, y| (x.a.as_bytes(), x.b.as_bytes()).cmp(&(y.a.as_bytes(), y.b.as_bytes())));
    // Dependencies, for causes.
    deps_of(&decls, &placed, &mut out);
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

/// The statements a feature, instance or pin depends on, following its placement references.
pub(crate) fn feature_deps(d: &Decls, id: &str, seen: &mut BTreeSet<String>) -> BTreeSet<String> {
    let mut s = BTreeSet::from(["world".to_string()]);
    if !seen.insert(id.to_string()) {
        return s;
    }
    let base = id.split('.').next().unwrap_or(id);
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
            ShapeDecl::Where(w) => {
                if let Some(p) = d.formulas.programs.get(w) {
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
        s.insert(format!("asset:{}", i.asset));
        if let Some(g) = &i.ground {
            s.insert(format!("kind.{g}"));
        }
        match &i.place {
            crate::decl::Place::On(o) => s.extend(feature_deps(d, &o.reference, seen)),
            crate::decl::Place::Of(o) => s.extend(feature_deps(d, &o.of, seen)),
            crate::decl::Place::At(_) => {}
        }
    } else if let Some(p) = d.pins.get(base) {
        s.insert(format!("pin.{base}"));
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
                let mut s = BTreeSet::from([format!("scatter.{m}"), "world".to_string()]);
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
            None => out
                .deps
                .get(&("instance".to_string(), name.clone()))
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
