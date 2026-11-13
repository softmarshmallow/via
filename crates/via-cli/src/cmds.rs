//! The commands other than `build` and `check` (`build.md` 14): `merge`, `explain`, `eval`,
//! `vectors`, `trust` and `config program`. Each returns its records; `main` prints them.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use via_diag::nearest::nearest;
use via_diag::{codes, Diag, NoWorld, Record, Site, Subject};
use via_doc::{ChangeRequest, LoadOptions};

use crate::build::{default_out, world_dir, world_name};
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
    let l = via_doc::load(world, arg, &LoadOptions::from_env(lib, changes));
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
    l.records
}

/// The run report beside the last build of `WORLD` (default `--out`, or `out`).
fn last_report(world: &Path, out: Option<&Path>) -> Option<J> {
    let out = match out {
        Some(o) => o.to_path_buf(),
        None => {
            let l = via_doc::load(world, 1, &LoadOptions::from_env(Vec::new(), Vec::new()));
            let name = l.world.as_ref().and_then(world_name).unwrap_or_else(|| {
                world
                    .file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
            });
            default_out(world, &name, &[])
        }
    };
    let text = std::fs::read_to_string(via_graph::report_path(&out)).ok()?;
    parse(&text).ok()
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
    if code.contains(':') {
        let Some(world) = world else {
            return vec![usage(
                at,
                format!("`{code}` is a node id; name the world too"),
                "write `via explain NODE WORLD`".into(),
            )
            .emit(&NoWorld)];
        };
        let report = last_report(world, out);
        let nodes: Vec<J> = report
            .as_ref()
            .and_then(|r| r.get("nodes"))
            .and_then(J::as_arr)
            .map(<[J]>::to_vec)
            .unwrap_or_default();
        let by_id: BTreeMap<&str, &J> = nodes
            .iter()
            .filter_map(|n| Some((n.get("id")?.as_str()?, n)))
            .collect();
        if !by_id.contains_key(code) {
            let near = nearest(code, by_id.keys().copied());
            let mut d = usage(
                at,
                format!("the last build held no node `{code}`"),
                near.hint("run `via build` first"),
            );
            if let Some(n) = near.found() {
                d = d.with("nearest", n);
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
        if let Some(r) = last_report(w, out) {
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
        Ok(outcomes) => {
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
    for s in steps::declared(&w, &wdir) {
        match steps::code_of(&s, &roots, &wdir, sdk_digest.as_ref()) {
            Ok(c) => {
                let folder = c.folder.as_ref().map(|f| {
                    f.strip_prefix(&wdir)
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
                    ("step", J::s(s.id.clone())),
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
