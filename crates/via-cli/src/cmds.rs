//! The commands other than `build` and `check` (`build.md` 14): `merge`, `explain`, `eval`,
//! `vectors`, `trust` and `config program`. Each returns its records; `main` prints them.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use via_diag::nearest::nearest;
use via_diag::{codes, Diag, NoWorld, Record, Site, Subject};
use via_doc::{ChangeRequest, LoadOptions};

use crate::build::{default_out, world_dir};
use crate::json::{obj, parse, J};
use crate::{steps, txt, usage};

/// `via merge WORLD`: the merged document on standard output.
pub fn merge(
    world: &Path,
    arg: u32,
    changes: Vec<ChangeRequest>,
    lib: Vec<PathBuf>,
    origins: bool,
) -> Vec<Record> {
    let opts = LoadOptions::from_env(lib, changes);
    let l = via_doc::load(world, arg, &opts);
    if let Some(w) = &l.world {
        let mut s = if origins {
            w.origins_json()
        } else {
            w.to_json()
        };
        if !s.ends_with('\n') {
            s.push('\n');
        }
        print!("{s}");
    }
    let mut records = l.records;
    // A merge with no document error also checks the names formulas read (`documents.md` 5.1).
    if let Some(w) = &l.world {
        if !records
            .iter()
            .any(|r| r.severity() == via_diag::Severity::Error)
        {
            records.extend(via_model::formula_direction(w).iter().map(|d| d.emit(w)));
            let mopts = via_model::Options::from_load(world, &opts);
            records.extend(via_model::scoped_names(w, &mopts).iter().map(|d| d.emit(w)));
        }
    }
    records
}

/// The world's top-level `name`, read as plain TOML: no include, no change, no merge
/// (`diagnostics.md` 2: `via explain` runs nothing and does not merge the world). `None` when the
/// file does not read or has no string `name`.
fn plain_name(world: &Path) -> Option<String> {
    let text = std::fs::read_to_string(world).ok()?;
    let t: toml::Table = toml::from_str(&text).ok()?;
    t.get("name")?.as_str().map(str::to_string)
}

/// The run report beside the last build of `WORLD` (default `--out`, or `out`), or the build
/// directory it was looked for in when there is none.
fn last_report(world: &Path, out: Option<&Path>) -> Result<J, PathBuf> {
    let out = match out {
        Some(o) => o.to_path_buf(),
        None => {
            let name = plain_name(world).unwrap_or_else(|| {
                world
                    .file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
            });
            default_out(world, &name, &[])
        }
    };
    std::fs::read_to_string(via_graph::report_path(&out))
        .ok()
        .and_then(|t| parse(&t).ok())
        .ok_or(out)
}

/// `via explain [CODE] [WORLD] [--json]`, and `via explain NODE WORLD`.
pub fn explain(
    code: Option<(&str, u32)>,
    world: Option<&Path>,
    out: Option<&Path>,
    json: bool,
) -> Vec<Record> {
    let Some((code, at)) = code else {
        print!("{}", via_diag::explain::list_text());
        return Vec::new();
    };
    // `diagnostics.md` 2: a world named that is not a file is `cli.no_world`; the world follows
    // the code on the command line.
    if let Some(w) = world.filter(|w| !w.is_file()) {
        return vec![Diag::new(
            codes::cli::NO_WORLD,
            Subject::Cli,
            Site::Argument(at + 1),
            txt(format!("the world file `{}` does not exist", w.display())),
            txt("name the world file of the build to explain".to_string()),
        )
        .emit(&NoWorld)];
    }
    if code.contains(':') {
        let Some(world) = world else {
            return vec![usage(
                at,
                format!("`{code}` is a node id; name the world too"),
                "write `via explain NODE WORLD`".into(),
            )
            .emit(&NoWorld)];
        };
        let report = match last_report(world, out) {
            Ok(r) => r,
            Err(dir) => {
                return vec![usage(
                    at,
                    format!(
                        "no build at `{}`: it holds no run report",
                        dir.to_string_lossy().replace('\\', "/")
                    ),
                    "run `via build` first, or name its directory with `--out`".into(),
                )
                .emit(&NoWorld)];
            }
        };
        let nodes: Vec<J> = report
            .get("nodes")
            .and_then(J::as_arr)
            .map(<[J]>::to_vec)
            .unwrap_or_default();
        let by_id: BTreeMap<&str, &J> = nodes
            .iter()
            .filter_map(|n| Some((n.get("id")?.as_str()?, n)))
            .collect();
        if !by_id.contains_key(code) {
            // The nearest full id; else the nearest by entity, the id with its view, window and
            // item taken off (`field:height` for `field:height/v:build/w:...`), naming every full
            // id of that entity (diagnostics.md 1.3 and 1.4).
            let near = nearest(code, by_id.keys().copied());
            let named: Vec<&str> = match near.found() {
                Some(n) => vec![n],
                None => {
                    let shorts: std::collections::BTreeSet<&str> =
                        by_id.keys().map(|k| entity_of(k)).collect();
                    match nearest(entity_of(code), shorts.iter().copied()).found() {
                        Some(e) => by_id
                            .keys()
                            .copied()
                            .filter(|k| entity_of(k) == e)
                            .collect(),
                        None => Vec::new(),
                    }
                }
            };
            let hint = if named.is_empty() {
                near.hint("name a node of the last build")
            } else {
                let shown: Vec<String> = named.iter().take(5).map(|n| format!("`{n}`")).collect();
                let more = named.len().saturating_sub(5);
                format!(
                    "write {}{}",
                    shown.join(" or "),
                    if more > 0 {
                        format!(", or one of {more} more of that entity")
                    } else {
                        String::new()
                    }
                )
            };
            let mut d = usage(at, format!("the last build held no node `{code}`"), hint);
            if let Some(n) = named.first() {
                d = d.with("nearest", *n);
            }
            return vec![d.emit(&NoWorld)];
        }
        let mut at_id = code.to_string();
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(at_id.clone()) {
            let Some(n) = by_id.get(at_id.as_str()) else {
                break;
            };
            let status = n.get("status").and_then(J::as_str).unwrap_or("");
            let cause = n.get("cause").cloned().unwrap_or(J::Null);
            println!("{at_id}: {status}, cause {}", cause.canonical());
            match cause.get("node").and_then(J::as_str) {
                Some(next) => at_id = next.to_string(),
                None => break,
            }
        }
        return Vec::new();
    }
    // Step codes recorded by the last build of WORLD.
    let mut recorded = Vec::new();
    if let Some(w) = world {
        // A step's code is known only from a build's run report (`build.md` 14): with no build
        // there, say so rather than that the code is unknown (M5 close: a fresh agent built with
        // `--out` elsewhere and was told `step.lay.laid` is not a code).
        if code.starts_with("step.") {
            if let Err(dir) = last_report(w, out) {
                return vec![usage(
                    at,
                    format!(
                        "no build at `{}`: a step's codes are read from the run report of its last build",
                        dir.to_string_lossy().replace('\\', "/")
                    ),
                    "run `via build` first, or name its directory with `--out`".into(),
                )
                .emit(&NoWorld)];
            }
        }
        if let Ok(r) = last_report(w, out) {
            for c in r.get("step_codes").and_then(J::as_arr).unwrap_or(&[]) {
                if let Some(name) = c.get("code").and_then(J::as_str) {
                    recorded.push(via_diag::explain::StepCode {
                        code: name.to_string(),
                        severity: match c.get("severity").and_then(J::as_str) {
                            Some("error") => via_diag::Severity::Error,
                            Some("warning") => via_diag::Severity::Warning,
                            _ => via_diag::Severity::Info,
                        },
                        explain: c.get("explain").and_then(J::as_str).map(str::to_string),
                    });
                }
            }
        }
    }
    match via_diag::explain::lookup(code, &recorded, at) {
        Ok(e) => {
            if json {
                print!("{}", via_diag::explain::json(e));
            } else {
                print!("{}", via_diag::explain::text(e));
            }
            Vec::new()
        }
        Err(d) => vec![d.emit(&NoWorld)],
    }
}

/// A node id without its view, window and item (`graph.md` 2): `<family>:<subject>`.
fn entity_of(id: &str) -> &str {
    ["/v:", "/w:", "/i:"]
        .iter()
        .filter_map(|m| id.find(m))
        .min()
        .map_or(id, |i| &id[..i])
}

/// A window `i0,j0,i1,j1` of posts.
pub fn parse_window(s: &str) -> Option<[u64; 4]> {
    let v: Vec<u64> = s
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
}

/// What `via eval` is asked.
pub struct EvalArgs {
    pub world: PathBuf,
    pub world_arg: u32,
    pub changes: Vec<ChangeRequest>,
    pub lib: Vec<PathBuf>,
    pub field: (String, u32),
    pub window: Option<([u64; 4], u32)>,
    pub tile: Option<u64>,
    pub out: PathBuf,
}

/// `via eval` (`build.md` 14): one scalar field in the build's own view, written as an array
/// file. It uses no store and writes no build directory.
///
/// The field is evaluated over the whole extent and the window cut from it; by the window rule
/// (`numbers.md` 6) that is the same bytes as evaluating the window with its halo, which the
/// model does not yet expose. `--tile` is accepted and changes nothing, as the chapter requires.
pub fn eval(a: &EvalArgs) -> Vec<Record> {
    let opts = LoadOptions::from_env(a.lib.clone(), a.changes.clone());
    let l = via_doc::load(&a.world, a.world_arg, &opts);
    let mut records = l.records;
    let Some(world) = l.world else {
        return records;
    };
    let b = via_model::build(
        &world,
        &via_model::Options::from_load(&a.world, &opts),
        None,
    );
    for d in &b.diagnostics {
        if *d.code() != via_diag::DiagCode::Core(codes::identity::NO_PREVIOUS) {
            records.push(d.emit(&world));
        }
    }
    if records.iter().any(Record::fails) {
        return records;
    }
    let Some(grid) = b.grid else { return records };
    let scalar: Vec<&str> = b
        .fields
        .iter()
        .filter(|(_, f)| f.values.is_some())
        .map(|(n, _)| n.as_str())
        .collect();
    let Some(values) = b.fields.get(&a.field.0).and_then(|f| f.values.as_ref()) else {
        let near = nearest(&a.field.0, scalar.iter().copied());
        let mut d = usage(
            a.field.1,
            format!("`{}` is not a scalar field of this world", a.field.0),
            near.hint("name a scalar field"),
        );
        if let Some(n) = near.found() {
            d = d.with("nearest", n);
        }
        records.push(d.emit(&NoWorld));
        return records;
    };
    let (nx, ny) = (grid.nx, grid.ny);
    let [i0, j0, i1, j1] = match a.window {
        None => [0, 0, nx, ny],
        Some((w, at)) => {
            if w[0] >= w[2] || w[1] >= w[3] || w[2] > nx || w[3] > ny {
                records.push(
                    usage(
                        at,
                        format!(
                            "the window {},{},{},{} is empty or not inside the grid of {nx} by {ny} posts",
                            w[0], w[1], w[2], w[3]
                        ),
                        "write `--window i0,j0,i1,j1` with i0 < i1 <= nx and j0 < j1 <= ny".into(),
                    )
                    .emit(&NoWorld),
                );
                return records;
            }
            w
        }
    };
    let _ = a.tile;
    let mut cut = Vec::new();
    for j in j0..j1 {
        for i in i0..i1 {
            cut.push(values[usize::try_from(j * nx + i).unwrap_or(0)]);
        }
    }
    if let Some(i) = cut.iter().position(|v| !v.is_finite()) {
        records.push(
            Diag::new(
                codes::build::INTERNAL,
                Subject::World,
                Site::World,
                txt(format!("sample {i} of the window is not finite")),
                via_diag::text!("a defect of ours: report it with the world that gives it"),
            )
            .emit(&NoWorld),
        );
        return records;
    }
    let bytes = crate::npy::f4(
        &[
            usize::try_from(j1 - j0).unwrap_or(0),
            usize::try_from(i1 - i0).unwrap_or(0),
        ],
        &cut,
    );
    if let Err(e) = std::fs::write(&a.out, bytes) {
        records.push(crate::dir::write_failed(&a.out, &e).emit(&NoWorld));
    }
    records
}

/// `via vectors FILE`: each vector computed through `via-num`, one line each.
pub fn vectors(file: &Path, at: u32) -> Vec<Record> {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            return vec![usage(
                at,
                format!("cannot read `{}`: {e}", file.display()),
                "name a vector file".into(),
            )
            .emit(&NoWorld)];
        }
    };
    match via_num::vectors::evaluate_file(&text) {
        Ok(mut outcomes) => {
            // `point_at`, `measure` and `samples` belong to the geometry kernel (`numbers.md`
            // 10): via-num leaves them, and the binary computes them here with via-geom.
            let table: Option<toml::Table> = text.parse().ok();
            for o in &mut outcomes {
                if o.got.is_none() {
                    let args = table
                        .as_ref()
                        .and_then(|t| t.get("vector")?.get(&o.id)?.get("args")?.as_array());
                    if let Some(g) = args.and_then(|a| delegated(&o.func, a)) {
                        o.got = Some(g.to_json());
                    }
                }
            }
            let mut out = std::io::stdout().lock();
            for o in &outcomes {
                if let Some(line) = o.line() {
                    let _ = writeln!(out, "{line}");
                }
            }
            Vec::new()
        }
        Err(via_num::vectors::VectorError::NotOurs { id, func }) => vec![usage(
            at,
            format!("vector `{id}` names `{func}`, which this binary does not compute"),
            "remove the vector, or run it with the crate that owns it".into(),
        )
        .with("vector", id)
        .emit(&NoWorld)],
        Err(e) => vec![Diag::new(
            codes::doc::MISSING_KEY,
            Subject::World,
            Site::File {
                file: file.to_string_lossy().replace('\\', "/"),
                locator: BTreeMap::new(),
            },
            txt(e.to_string()),
            via_diag::text!("write each vector as `numbers.md` 10 does"),
        )
        .emit(&NoWorld)],
    }
}

fn config_or_fail(at: u32) -> Result<PathBuf, Box<Record>> {
    via_proto::program::config_dir().ok_or_else(|| {
        Box::new(
            usage(
                at,
                "no user config folder is known on this machine".into(),
                "set `VIA_CONFIG_DIR`".into(),
            )
            .emit(&NoWorld),
        )
    })
}

fn write_toml(path: &Path, t: &toml::Table) -> Result<(), Box<Record>> {
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(path, t.to_string())
        .map_err(|e| Box::new(crate::dir::write_failed(path, &e).emit(&NoWorld)))
}

fn read_toml(path: &Path) -> toml::Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default()
}

/// `via config program NAME PATH`.
pub fn config_program(name: &str, path: &str, at: u32) -> Vec<Record> {
    let p = Path::new(path);
    if !p.is_absolute() || !p.exists() {
        return vec![Diag::new(
            codes::protocol::NO_PROGRAM,
            Subject::Cli,
            Site::Argument(at),
            txt(format!(
                "`{path}` is not an absolute path to an existing program"
            )),
            via_diag::text!("give the program's absolute path"),
        )
        .emit(&NoWorld)];
    }
    let dir = match config_or_fail(at) {
        Ok(d) => d,
        Err(r) => return vec![*r],
    };
    let file = dir.join("config.toml");
    let mut t = read_toml(&file);
    let programs = t
        .entry("program")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(pt) = programs {
        let mut e = toml::Table::new();
        e.insert("path".into(), toml::Value::String(path.to_string()));
        pt.insert(name.to_string(), toml::Value::Table(e));
    }
    write_toml(&file, &t)
        .err()
        .map(|r| *r)
        .into_iter()
        .collect()
}

/// `via trust WORLD [--change C]... [--yes] [--json]` (`protocol.md` 6.3, 7).
pub fn trust(
    world: &Path,
    arg: u32,
    changes: Vec<ChangeRequest>,
    lib: Vec<PathBuf>,
    sdk: Option<&Path>,
    yes: bool,
    json: bool,
) -> (Vec<Record>, u8) {
    let lib_roots = lib.clone();
    let opts = LoadOptions::from_env(lib.clone(), changes);
    let l = via_doc::load(world, arg, &opts);
    let mut records = l.records;
    let Some(w) = l.world else {
        return (records, 1);
    };
    let wdir = world_dir(world);
    let mopts = via_model::Options::from_load(world, &opts);
    let roots = steps::Roots {
        world_dir: wdir.clone(),
        lib,
        via_lib: mopts.via_lib,
        shipped: mopts.shipped_root,
        include: steps::includes(&w),
    };
    let sdk_digest = sdk.map(|d| steps::folder_digest(d).0);
    let config = match config_or_fail(arg) {
        Ok(c) => c,
        Err(r) => {
            records.push(*r);
            return (records, 1);
        }
    };
    let trusted = steps::trusted(&config);
    let mut rows = Vec::new();
    let mut pending: Vec<(String, J)> = Vec::new();
    // The world's steps, then each import's, at any depth: an import's steps are trusted as the
    // parent's are, since the parent names its file in its own text (`composition.md` Open).
    let mut worlds: Vec<(String, via_doc::World, PathBuf, steps::Roots)> = Vec::new();
    for (prefix, iw, idir) in steps::import_worlds(&w, &wdir, &lib_roots) {
        let r = steps::Roots {
            world_dir: idir.clone(),
            include: steps::includes(&iw),
            ..roots.clone()
        };
        worlds.push((prefix, iw, idir, r));
    }
    let mut all: Vec<(String, steps::StepDecl, PathBuf, &steps::Roots)> =
        steps::declared(&w, &wdir)
            .into_iter()
            .map(|s| (String::new(), s, wdir.clone(), &roots))
            .collect();
    // Only an import's generate steps run in the parent; its lints never do (`composition.md` 12).
    for (prefix, iw, idir, r) in &worlds {
        for s in steps::declared(iw, idir) {
            if s.when == "generate" {
                all.push((prefix.clone(), s, idir.clone(), r));
            }
        }
    }
    for (prefix, s, sdir, roots) in all {
        let wdir = &sdir;
        match steps::code_of(&s, roots, wdir, sdk_digest.as_ref()) {
            Ok(c) => {
                let folder = c.folder.as_ref().map(|f| {
                    f.strip_prefix(wdir)
                        .map_or_else(
                            |_| f.to_string_lossy().into_owned(),
                            |r| r.to_string_lossy().into_owned(),
                        )
                        .replace('\\', "/")
                });
                let row = obj([
                    (
                        "code_folder",
                        folder
                            .clone()
                            .map_or_else(|| J::s("none: installed code is not covered"), J::s),
                    ),
                    ("command", J::strs(s.command.iter().map(String::as_str))),
                    ("files", J::count(c.files)),
                    ("step", J::s(format!("{prefix}{}", s.id))),
                    ("trust", J::s(c.trust.hex())),
                    ("trusted", J::Bool(trusted.contains(&c.trust.hex()))),
                ]);
                if !trusted.contains(&c.trust.hex()) {
                    pending.push((c.trust.hex(), row.clone()));
                }
                rows.push(row);
            }
            Err(d) => records.push(d.emit(&w)),
        }
    }
    if json {
        println!("{}", J::Arr(rows.clone()).canonical());
    } else {
        // `protocol.md` 8.1: the SDK folder enters every step's trust digest, so it is listed once.
        if let Some(d) = sdk {
            let (digest, files) = steps::folder_digest(d);
            eprintln!(
                "sdk {} | {files} files | digest {}",
                d.to_string_lossy().replace('\\', "/"),
                digest.hex()
            );
        }
        for r in &rows {
            eprintln!(
                "step {}: {} | code folder {} | {} files | trust {}",
                r.get("step").and_then(J::as_str).unwrap_or(""),
                r.get("command").map(J::canonical).unwrap_or_default(),
                r.get("code_folder").and_then(J::as_str).unwrap_or(""),
                r.get("files").map(J::canonical).unwrap_or_default(),
                r.get("trust").and_then(J::as_str).unwrap_or("")
            );
        }
    }
    if records.iter().any(Record::fails) {
        return (records, 1);
    }
    if pending.is_empty() {
        return (records, 0);
    }
    let agreed = yes || {
        eprint!("Trust these steps on this machine? [y/N] ");
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        matches!(line.trim(), "y" | "Y" | "yes")
    };
    if !agreed {
        return (records, 1);
    }
    let file = config.join("trust.toml");
    let mut t = read_toml(&file);
    let table = t
        .entry("trusted")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let world_shown = world.to_string_lossy().replace('\\', "/");
    if let toml::Value::Table(tt) = table {
        for (digest, row) in pending {
            let mut e = toml::Table::new();
            e.insert(
                "step".into(),
                toml::Value::String(row.get("step").and_then(J::as_str).unwrap_or("").into()),
            );
            e.insert("world".into(), toml::Value::String(world_shown.clone()));
            e.insert(
                "code_folder".into(),
                toml::Value::String(
                    row.get("code_folder")
                        .and_then(J::as_str)
                        .unwrap_or("")
                        .into(),
                ),
            );
            e.insert(
                "files".into(),
                toml::Value::Integer(row.get("files").and_then(J::as_i64).unwrap_or(0)),
            );
            tt.insert(digest, toml::Value::Table(e));
        }
    }
    match write_toml(&file, &t) {
        Ok(()) => (records, 0),
        Err(r) => {
            records.push(*r);
            (records, 1)
        }
    }
}

/// A vector of the geometry kernel's (`numbers.md` 10): `point_at` (path, `t` as decimal text,
/// offset) to the point and the bearing; `measure` (shape word, points, point) to `t`, `d`, `sd`;
/// `samples` (path, `r`) to the sample points of 5.5. `None` when the arguments do not fit.
fn delegated(func: &str, args: &[toml::Value]) -> Option<via_num::vectors::Got> {
    use via_geom::lineref::{self, Line};
    use via_geom::{Region, P};
    use via_num::vectors::Got;
    let int = |v: &toml::Value| v.as_integer();
    let point = |v: &toml::Value| -> Option<P> {
        let a = v.as_array()?;
        Some(P::new(int(a.first()?)?, int(a.get(1)?)?))
    };
    let points = |v: &toml::Value| -> Option<Vec<P>> { v.as_array()?.iter().map(point).collect() };
    let pt = |p: P| Got::List(vec![Got::Int(p.x), Got::Int(p.y)]);
    match func {
        "point_at" => {
            let line = Line::new(points(args.first()?)?, false);
            let t = via_num::parse_double(args.get(1)?.as_str()?).ok()?;
            let at = lineref::interpolate(&line, "v", t, int(args.get(2)?)?).ok()?;
            Some(Got::List(vec![pt(at.point), Got::Int(at.bearing)]))
        }
        "measure" => {
            let shape = points(args.get(1)?)?;
            let p = point(args.get(2)?)?;
            let m = match args.first()?.as_str()? {
                "path" => lineref::project_path(&Line::new(shape, false), p),
                "region" => lineref::project_region(&Region::from_ring(shape), p)?,
                "point" => lineref::project_point(*shape.first()?, p),
                _ => return None,
            };
            Some(Got::List(vec![
                Got::Double(m.t.unwrap_or(0.0)),
                Got::Double(m.d),
                Got::Double(m.sd),
            ]))
        }
        "samples" => {
            let line = Line::new(points(args.first()?)?, false);
            let r = int(args.get(1)?)? as f64;
            let l = line.length();
            let mut out = Vec::new();
            let mut i = 0.0;
            loop {
                let s = i * r;
                if s >= l {
                    break;
                }
                out.push(pt(lineref::at_length(&line, "v", s, false, 0).ok()?.point));
                i += 1.0;
            }
            out.push(pt(lineref::at_length(&line, "v", l, true, 0).ok()?.point));
            Some(Got::List(out))
        }
        // `identity.md` 10: `[class, [part, ...]]`, the digest as 16 lowercase hex digits.
        "digest" => {
            let class = args.first()?.as_str()?;
            let parts = args
                .get(1)?
                .as_array()?
                .iter()
                .map(digest_part)
                .collect::<Option<Vec<_>>>()?;
            Some(Got::Text(via_geom::names::hex(via_geom::names::digest(
                class, &parts,
            ))))
        }
        _ => None,
    }
}

/// One part of a `digest` vector, written as `identity.md` 10 says: `{ name }`, `{ int }`,
/// `{ side }`, `{ role }`, `{ pair = [p, q] }` or `{ set = [p, ...] }`.
fn digest_part(v: &toml::Value) -> Option<via_geom::names::Part> {
    use via_geom::names::{Part, Role};
    use via_geom::network::Side;
    let t = v.as_table()?;
    if t.len() != 1 {
        return None;
    }
    let (k, x) = t.iter().next()?;
    Some(match k.as_str() {
        "name" => Part::name(x.as_str()?),
        "int" => Part::Int(x.as_integer()?),
        "side" => Part::Side(match x.as_str()? {
            "left" => Side::Left,
            "right" => Side::Right,
            _ => return None,
        }),
        "role" => Part::Role(match x.as_str()? {
            "start" => Role::Start,
            "end" => Role::End,
            "pass" => Role::Pass,
            _ => return None,
        }),
        "pair" => {
            let a = x.as_array()?;
            if a.len() != 2 {
                return None;
            }
            Part::pair(digest_part(&a[0])?, digest_part(&a[1])?)
        }
        "set" => Part::Set(
            x.as_array()?
                .iter()
                .map(digest_part)
                .collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}
