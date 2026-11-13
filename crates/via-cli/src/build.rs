//! `via build WORLD` (`build.md` 13 and 14): read, build through the graph, run the lint steps,
//! write the directory whole or leave it untouched, and the run report beside it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use via_diag::{
    codes, document, exit_status, finish, split_world_run, Diag, FailOn, Lint, NoWorld, Record,
    Site, Subject,
};

mod graph;
use via_doc::{ChangeRequest, LoadOptions, World};
use via_graph::{BuildKeyParts, BuildRecord, Digest, NodeId, Store, Versions};

use crate::dir;
use crate::json::{obj, parse, J};
use crate::out::{self, epoch};
use crate::snap::{self, Snapshot};
use crate::steps;
use crate::txt;

/// What `via build` is asked.
#[derive(Clone, Debug, Default)]
pub struct Args {
    pub world: PathBuf,
    pub world_arg: u32,
    pub changes: Vec<ChangeRequest>,
    pub out: Option<PathBuf>,
    pub lib: Vec<PathBuf>,
    pub threads: Option<usize>,
    pub store: Option<PathBuf>,
    pub against: Option<PathBuf>,
    pub fail_on: Vec<String>,
    pub rerun: Vec<String>,
    pub deadline: Option<u64>,
    pub sdk: Option<PathBuf>,
}

/// What a build gives the command line: every record, finished, and the lint list.
pub struct Outcome {
    pub records: Vec<Record>,
    pub lints: Vec<Lint>,
    /// The build directory, when one was known (a `--diagnostics FILE` inside it is refused).
    pub out: Option<PathBuf>,
}

fn run_diag(code: via_diag::Code, message: String, hint: &'static str) -> Diag {
    Diag::new(
        code,
        Subject::World,
        Site::World,
        txt(message),
        txt(hint.to_string()),
    )
}

/// The versions of `graph.md` 3.3.
pub fn versions() -> Versions {
    Versions {
        document: via_doc::LANGUAGE.to_string(),
        protocol: via_proto::PROTOCOL.to_string(),
        build: out::FORMAT.to_string(),
        epoch: epoch(),
    }
}

/// The world's `name`, when it reads as a string.
pub fn world_name(world: &World) -> Option<String> {
    match &world.root().get("name")?.item.value {
        via_doc::Value::Str(s) => Some(s.text.clone()),
        _ => None,
    }
}

fn world_seed(world: &World) -> u64 {
    match world.root().get("seed").map(|e| &e.item.value) {
        Some(via_doc::Value::Num(n)) => n.norm.parse::<i64>().map_or(0, |v| v as u64),
        _ => 0,
    }
}

/// The default `--out` (14): `build/<name>` beside the world file, `+<change>` per change,
/// sorted, a file by its stem.
pub fn default_out(world_path: &Path, name: &str, changes: &[ChangeRequest]) -> PathBuf {
    let mut parts: Vec<String> = changes
        .iter()
        .map(|c| {
            if c.is_path() {
                Path::new(&c.written)
                    .file_stem()
                    .map_or_else(|| c.written.clone(), |s| s.to_string_lossy().into_owned())
            } else {
                c.written.clone()
            }
        })
        .collect();
    parts.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let mut leaf = name.to_string();
    for p in parts {
        leaf.push('+');
        leaf.push_str(&p);
    }
    world_dir(world_path).join("build").join(leaf)
}

pub fn world_dir(world_path: &Path) -> PathBuf {
    match world_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn relative(from: &Path, to: &Path) -> Option<String> {
    let (f, t) = (absolute(from), absolute(to));
    let fc: Vec<_> = f.components().collect();
    let tc: Vec<_> = t.components().collect();
    if fc.first() != tc.first() {
        return None;
    }
    let common = fc.iter().zip(&tc).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_string(); fc.len() - common];
    parts.extend(
        tc[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn previous_of(out: &Path) -> (J, J) {
    std::fs::read_to_string(out.join(".previous"))
        .ok()
        .and_then(|t| parse(&t).ok())
        .map_or((J::Null, J::Null), |j| {
            (
                j.get("id").cloned().unwrap_or(J::Null),
                j.get("manifest").cloned().unwrap_or(J::Null),
            )
        })
}

fn previous_file(id: &J, manifest: &J) -> Vec<u8> {
    obj([
        ("format", J::s(dir::PREVIOUS_FORMAT)),
        ("id", id.clone()),
        ("manifest", manifest.clone()),
    ])
    .file()
}

fn record_json(path: &str, bytes: &[u8]) -> (String, J) {
    (
        path.to_string(),
        obj([
            ("bytes", J::count(bytes.len())),
            ("digest", J::s(Digest::of(bytes).hex())),
        ]),
    )
}

/// The baseline manifest's `(id, run.key)` and its digest.
fn manifest_ids(bytes: &[u8]) -> Option<(J, String, String)> {
    let m = parse(std::str::from_utf8(bytes).ok()?).ok()?;
    if !m
        .get("format")
        .and_then(J::as_str)
        .is_some_and(|f| f.starts_with("via-build-"))
    {
        return None;
    }
    let id = m.get("id")?.as_str()?.to_string();
    let key = m
        .get("run")
        .and_then(|r| r.get("key"))
        .and_then(J::as_str)
        .unwrap_or("")
        .to_string();
    Some((m, id, key))
}

/// Writes the run report beside the directory, adding the lint steps' codes for `via explain`.
fn write_run_report(out: &Path, text: &str, step_codes: &[J]) {
    let mut j = parse(text).unwrap_or(J::Obj(J::obj()));
    if let J::Obj(m) = &mut j {
        m.insert("step_codes".into(), J::Arr(step_codes.to_vec()));
    }
    let mut s = j.canonical();
    s.push('\n');
    let _ = via_graph::write_report(out, &s);
}

/// The diagnostics-only directory of a failed build (13): `diagnostics.json` and `.previous`.
fn write_failed_dir(out: &Path, diagnostics: &[u8]) -> Option<Diag> {
    let (id, manifest) = previous_of(out);
    let mut files = BTreeMap::new();
    files.insert(".previous".to_string(), previous_file(&id, &manifest));
    files.insert("diagnostics.json".to_string(), diagnostics.to_vec());
    dir::replace(out, &files).err()
}

/// Runs `via build`.
#[allow(clippy::too_many_lines)]
pub fn run(a: &Args) -> Outcome {
    let mut fail_on = FailOn::none();
    for c in &a.fail_on {
        fail_on.insert(c.clone());
    }
    let wdir = world_dir(&a.world);
    let load_opts = LoadOptions::from_env(a.lib.clone(), a.changes.clone());
    let loaded = via_doc::load(&a.world, a.world_arg, &load_opts);
    let name = loaded
        .world
        .as_ref()
        .and_then(world_name)
        .unwrap_or_else(|| {
            a.world
                .file_stem()
                .map_or_else(|| "world".to_string(), |s| s.to_string_lossy().into_owned())
        });
    let out_dir = a
        .out
        .clone()
        .unwrap_or_else(|| default_out(&a.world, &name, &a.changes));
    let store_root = a
        .store
        .clone()
        .unwrap_or_else(|| wdir.join(via_graph::DEFAULT_STORE));
    let mut run_diags: Vec<Diag> = Vec::new();
    let finish_run = |records: Vec<Record>,
                      run_diags: Vec<Diag>,
                      world: Option<&World>,
                      lints: Vec<Lint>,
                      out: Option<PathBuf>| {
        let mut all = records;
        for d in run_diags {
            all.push(match world {
                Some(w) => d.emit(w),
                None => d.emit(&NoWorld),
            });
        }
        Outcome {
            records: finish(all, &fail_on),
            lints,
            out,
        }
    };
    // The store may not live inside the directory every build replaces (13).
    if absolute(&store_root).starts_with(absolute(&out_dir)) {
        run_diags.push(run_diag(
            codes::build::STORE_INSIDE,
            format!(
                "the store `{}` resolves inside the build directory",
                store_root.display()
            ),
            "drop `--store`, or name a folder outside `--out`",
        ));
        return finish_run(
            loaded.records,
            run_diags,
            loaded.world.as_ref(),
            Vec::new(),
            None,
        );
    }
    if let Err(d) = dir::check_out(&out_dir, &wdir) {
        run_diags.push(d);
        return finish_run(
            loaded.records,
            run_diags,
            loaded.world.as_ref(),
            Vec::new(),
            None,
        );
    }
    let _lock = match via_graph::DirLock::acquire(&out_dir) {
        Ok(l) => l,
        Err(d) => {
            run_diags.push(d);
            return finish_run(
                loaded.records,
                run_diags,
                loaded.world.as_ref(),
                Vec::new(),
                None,
            );
        }
    };
    dir::recover(&out_dir);
    let store = Store::open(&store_root);
    // `--against` (11.2) is checked before anything is built.
    let against = match &a.against {
        Some(d) => match std::fs::read(d.join("manifest.json"))
            .ok()
            .and_then(|b| manifest_ids(&b).map(|m| (b, m)))
        {
            Some(x) => Some(x),
            None => {
                run_diags.push(run_diag(
                    codes::build::AGAINST_NOT_BUILD,
                    format!(
                        "`--against {}` names a folder with no manifest of a known format",
                        d.display()
                    ),
                    "name a complete build directory",
                ));
                // Nothing is built; as with any error, the directory holds `diagnostics.json`
                // (13), here the world's records only: this fault is the run's.
                let records = finish(loaded.records.clone(), &fail_on);
                let (world_r, _) = split_world_run(records);
                let doc = document(&world_r, &[], 1);
                if let Some(d) = write_failed_dir(&out_dir, doc.as_bytes()) {
                    run_diags.push(d);
                }
                return finish_run(
                    loaded.records,
                    run_diags,
                    loaded.world.as_ref(),
                    Vec::new(),
                    Some(out_dir),
                );
            }
        },
        None => None,
    };
    let Some(world) = loaded.world else {
        // Nothing to build: the reading errors, in a diagnostics-only directory.
        let records = finish(loaded.records, &fail_on);
        let (world_r, _) = split_world_run(records.clone());
        let doc = document(&world_r, &[], exit_status(&world_r));
        if let Some(d) = write_failed_dir(&out_dir, doc.as_bytes()) {
            run_diags.push(d);
        }
        return finish_run(records, run_diags, None, Vec::new(), Some(out_dir));
    };
    let model_opts = via_model::Options::from_load(&a.world, &load_opts);
    let roots = steps::Roots {
        world_dir: wdir.clone(),
        lib: a.lib.clone(),
        via_lib: model_opts.via_lib.clone(),
        shipped: model_opts.shipped_root.clone(),
        include: steps::includes(&world),
    };
    // The baseline (11.2).
    let (prev_id, prev_manifest) = previous_of(&out_dir);
    let mut baseline: Option<(Snapshot, J, String, String)> = None;
    let mut baseline_missing = false;
    if let Some((bytes, (_m, id, key))) = &against {
        match snap::get(&store, &Digest::of(bytes)) {
            Some(s) => baseline = Some((s, J::Null, id.clone(), key.clone())),
            None => baseline_missing = true,
        }
    } else if let Some(hex) = prev_manifest.as_str() {
        let found = Digest::from_hex(hex).and_then(|d| {
            let mb = store.get_object(&d)?;
            let (_m, id, key) = manifest_ids(&mb)?;
            Some((snap::get(&store, &d)?, id, key, d))
        });
        match found {
            Some((s, id, key, _)) => baseline = Some((s, J::Null, id, key)),
            None => baseline_missing = true,
        }
    }
    if baseline_missing {
        run_diags.push(run_diag(
            codes::build::BASELINE_MISSING,
            "the baseline build is not in the store; the change report has no baseline".into(),
            "rebuild the earlier text, or pass `--against`",
        ));
    }
    // Steps: their trust digests enter the build key (`graph.md` 6.2).
    let config = via_proto::program::config_dir();
    let trusted = config.as_deref().map(steps::trusted).unwrap_or_default();
    let sdk_digest = a.sdk.as_deref().map(|d| steps::folder_digest(d).0);
    let decls = steps::declared(&world, &wdir);
    let mut lint_steps: Vec<graph::LintStep> = Vec::new();
    for s in decls.iter().filter(|s| s.when == "lint" && !s.worker) {
        let code = steps::code_of(s, &roots, &wdir, sdk_digest.as_ref());
        let is_trusted = code
            .as_ref()
            .is_ok_and(|c| trusted.contains(&c.trust.hex()));
        lint_steps.push(graph::LintStep {
            decl: s.clone(),
            code,
            trusted: is_trusted,
        });
    }
    // The build key: sources, declared inputs, trust, and the flags that change what is built.
    let mut parts = BuildKeyParts {
        changes: world.changes().to_vec(),
        ..BuildKeyParts::default()
    };
    let mut inputs: Vec<(String, Digest)> = world
        .sources()
        .iter()
        .map(|s| (s.name.clone(), Digest::of(s.text().as_bytes())))
        .collect();
    // The pack and each field's file are read too (`graph.md` 1, `source` and `input`).
    inputs.extend(via_model::leaves(&world, &model_opts));
    inputs.sort_by(|x, y| x.0.as_bytes().cmp(y.0.as_bytes()));
    inputs.dedup_by(|x, y| x.0 == y.0);
    parts.files = inputs.clone();
    for l in &lint_steps {
        let s = &l.decl;
        for i in &s.inputs {
            let p = s.home.join(i);
            let d = if p.is_dir() {
                steps::folder_digest(&p).0
            } else {
                std::fs::read(&p).map_or_else(|_| Digest::of(b"\0missing"), |b| Digest::of(&b))
            };
            parts.files.push((format!("input {}/{i}", s.id), d));
        }
        if let Ok(c) = &l.code {
            parts.trust.push((s.id.clone(), c.trust));
        }
        // Trust is recorded on this machine (`protocol.md` 7): a newly trusted lint must run.
        parts
            .flags
            .push((format!("trusted {}", s.id), l.trusted.to_string()));
    }
    if let Some(d) = &sdk_digest {
        parts.flags.push(("sdk".into(), d.hex()));
    }

    // The pack, read again for the instance taxa a step is handed and for the files of the id.
    let pack = world
        .root()
        .get("pack")
        .and_then(|e| match &e.item.value {
            via_doc::Value::Str(s) => Some(s.text.clone()),
            _ => None,
        })
        .and_then(|written| {
            let r = via_model::pack::Roots {
                world_dir: model_opts.world_dir.clone(),
                lib_roots: model_opts.lib_roots.clone(),
                via_lib: model_opts.via_lib.clone(),
                shipped: model_opts.shipped_root.clone(),
            };
            let path = via_model::pack::find(&written, &r, &mut Vec::new())?;
            let (p, _) = via_model::pack::read(&path, "pack");
            Some((p?, path))
        });
    let pack_root = pack
        .as_ref()
        .and_then(|(_, p)| p.parent())
        .and_then(|p| relative(&out_dir, p));
    let pack_far = pack.is_some() && pack_root.is_none();
    // The manifest's `locations.pack_root` is the path from this directory to the pack's root:
    // relative, so it enters by place (`graph.md` 6.2), and a directory at another depth has
    // another build key, never another directory's manifest.
    parts.flags.push((
        "pack_root".into(),
        match (&pack, &pack_root) {
            (None, _) => "none".to_string(),
            (Some(_), None) => "far".to_string(),
            (Some(_), Some(r)) => format!("path {r}"),
        },
    ));
    let mut fail_sorted = a.fail_on.clone();
    fail_sorted.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
    fail_sorted.dedup();
    for c in &fail_sorted {
        parts.flags.push(("fail_on".into(), c.clone()));
    }
    let versions = versions();
    let key = via_graph::build_key(&versions, &parts);
    let threads = a
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
        .max(1);
    let started = std::time::Instant::now();
    let manifest_key = std::fs::read(out_dir.join("manifest.json"))
        .ok()
        .and_then(|b| manifest_ids(&b))
        .and_then(|(_, _, k)| Digest::from_hex(&k));
    // The previous build (`graph.md` 6.3): the one the run report beside the directory names,
    // else the one the directory's manifest names.
    let previous_key = std::fs::read_to_string(via_graph::report_path(&out_dir))
        .ok()
        .and_then(|t| parse(&t).ok())
        .and_then(|j| {
            j.get("build")
                .and_then(|b| b.get("key"))
                .and_then(J::as_str)
                .and_then(Digest::from_hex)
        })
        .filter(|k| BuildRecord::get(&store, k).is_some())
        .or(manifest_key);
    let prev_record = previous_key.and_then(|k| BuildRecord::get(&store, &k));
    let head = |id: Option<String>, null: bool, run_records: Vec<String>| via_graph::ReportHead {
        epoch: epoch(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        threads: u64::try_from(threads).unwrap_or(1),
        key,
        id,
        null,
        previous: prev_record.as_ref().and(previous_key),
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        run_records,
    };
    // The null build (`graph.md` 6.2): nothing is touched; the record is the build.
    if a.rerun.is_empty() {
        if let Some(null) = null_build(
            &store,
            &key,
            manifest_key.as_ref(),
            a.against.is_some(),
            &out_dir,
        ) {
            let (diags, lints, restored) = null;
            run_diags.extend(store.take_run_diags());
            let mut all = loaded.records;
            for d in &diags {
                all.push(d.emit(&world));
            }
            for d in run_diags.iter().chain(restored.iter()) {
                all.push(d.emit(&world));
            }
            let records = finish(all, &fail_on);
            let (_, run_r) = split_world_run(records.clone());
            if let Some(rec) = BuildRecord::get(&store, &key) {
                let id = std::fs::read(out_dir.join("manifest.json"))
                    .ok()
                    .and_then(|b| manifest_ids(&b))
                    .map(|(_, id, _)| id);
                let text = via_graph::null_report(
                    &head(
                        id,
                        true,
                        run_r.iter().map(Record::to_canonical_json).collect(),
                    ),
                    &rec,
                );
                write_run_report(&out_dir, &text, &[]);
            }
            return Outcome {
                records,
                lints,
                out: Some(out_dir),
            };
        }
    }
    // The graph.
    let base_build = baseline.as_ref().map(|b| &b.0.build);
    let model = via_model::ModelPlan::new(&world, &model_opts, base_build);
    let mut extra: Vec<Diag> = Vec::new();
    if baseline.is_none() && !baseline_missing {
        extra.push(Diag::new(
            codes::identity::NO_PREVIOUS,
            Subject::World,
            Site::World,
            via_diag::text!("no baseline build to report changes against"),
            via_diag::text!("none needed; the next build has one"),
        ));
    }
    let mut flags = J::obj();
    flags.insert(
        "fail_on".into(),
        J::strs(fail_sorted.iter().map(String::as_str)),
    );
    flags.insert("stand_ins".into(), J::Bool(false));
    let mut run_members = J::obj();
    run_members.insert(
        "changes".into(),
        J::strs(world.changes().iter().map(String::as_str)),
    );
    run_members.insert("flags".into(), J::Obj(flags));
    run_members.insert(
        "inputs".into(),
        J::Arr(
            inputs
                .iter()
                .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
                .collect(),
        ),
    );
    let baseline_record = baseline
        .as_ref()
        .and_then(|(_, _, _, bkey)| Digest::from_hex(bkey))
        .and_then(|k| BuildRecord::get(&store, &k));
    let ctx = graph::Ctx {
        world: &world,
        store: &store,
        roots: &roots,
        config: config.as_deref(),
        sdk: a.sdk.as_deref(),
        sdk_digest: sdk_digest.as_ref(),
        seed: world_seed(&world),
        deadline: Duration::from_secs(a.deadline.unwrap_or(600)),
        pack: pack.as_ref().map(|p| &p.0),
        name: &name,
        fail_on: &fail_on,
        loaded: &loaded.records,
        extra,
        lints: lint_steps,
        baseline: baseline.as_ref().map(|(s, _, id, k)| graph::Baseline {
            snapshot: s,
            id: id.clone(),
            key: k.clone(),
        }),
        baseline_record,
        key,
        version: env!("CARGO_PKG_VERSION").to_string(),
        pack_root: pack_root.clone(),
        run_members,
    };
    let plan = graph::CliPlan::new(&model, ctx);
    let mut gopts = via_graph::Options::new(versions.clone());
    gopts.threads = threads;
    gopts.rerun = a.rerun.iter().cloned().collect();
    gopts.previous = prev_record.clone();
    let run = via_graph::run(&plan, &store, &gopts, &plan.roots());
    run_diags.extend(run.run_diags.iter().cloned());
    run_diags.extend(store.take_run_diags());
    // Every record, finished once.
    let node_diags = via_model::dedupe(run.world_diags.clone());
    let world_records = plan.world_records(&node_diags);
    let lints = plan.lints(&|n| run.nodes.get(n).map(|o| (o.status, o.value)));
    let mut all = world_records;
    for d in &run_diags {
        all.push(d.emit(&world));
    }
    let records = finish(all, &fail_on);
    let (world_r, run_r) = split_world_run(records.clone());
    let diagnostics = plan
        .last_bytes("diagnostics.json")
        .unwrap_or_else(|| document(&world_r, &lints, exit_status(&world_r)).into_bytes());
    // The lint steps' explain texts, for `via explain` (the run report's `step_codes`).
    let mut explains = Vec::new();
    for l in &plan.ctx.lints {
        let id = NodeId::of("step", &l.decl.id);
        if let Some(v) = run.value(&id, &store, &plan) {
            if let Some((_, e)) = graph::step_value(&v.bytes) {
                explains.extend(e);
            }
        }
    }
    let step_codes = graph::step_codes(&explains, &node_diags);
    // `identity.no_previous` is about the run, never replayed (`graph.md` 6.2).
    let world_kept: Vec<Diag> = node_diags.clone();
    let put_record = |files: BTreeMap<String, Digest>| {
        run.record(&versions, graph::encode_world(&world_kept, &lints), files)
            .put(&store, &key);
    };
    if world_r.iter().any(Record::fails) {
        let mut extra = Vec::new();
        if let Some(d) = write_failed_dir(&out_dir, &diagnostics) {
            extra.push(d.emit(&world));
        }
        put_record(BTreeMap::new());
        let text = via_graph::report(
            &head(
                None,
                false,
                run_r.iter().map(Record::to_canonical_json).collect(),
            ),
            &run,
        );
        write_run_report(&out_dir, &text, &step_codes);
        let mut recs = records;
        recs.extend(extra);
        return Outcome {
            records: recs,
            lints,
            out: Some(out_dir),
        };
    }
    // The files: each data file from its node's value, then the reports and the manifest.
    let mut all_files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for p in plan.data_paths() {
        let id = NodeId::of("write", p);
        if let Some(v) = run.value(&id, &store, &plan) {
            // A weights chunk with no material painted has no file (`build.md` 6.2); no other
            // file is ever left out, and some are empty (`instances/names.txt`).
            if !(v.bytes.is_empty() && p.starts_with("weights/")) {
                all_files.insert(p.clone(), v.bytes.as_ref().clone());
            }
        }
    }
    let last = |path: &str| -> Option<Vec<u8>> {
        let id = NodeId::of("write", path);
        run.value(&id, &store, &plan)
            .map(|v| v.bytes.as_ref().clone())
            .or_else(|| plan.last_bytes(path))
    };
    let mut extra_run: Vec<Diag> = Vec::new();
    if pack_far {
        extra_run.push(run_diag(
            codes::build::PACK_ROOT_FAR,
            "no relative path leads from the build directory to the pack's root".into(),
            "build onto the pack's drive, or tell the loader where the pack is",
        ));
    }
    let (Some(changes_bytes), Some(manifest_bytes)) = (last("changes.json"), last("manifest.json"))
    else {
        extra_run.push(run_diag(
            codes::build::INTERNAL,
            "the build ran without error and wrote no manifest".into(),
            "a defect of ours: report it with the world that gives it",
        ));
        let mut recs = records;
        recs.extend(extra_run.iter().map(|d| d.emit(&world)));
        return Outcome {
            records: finish(recs, &fail_on),
            lints,
            out: Some(out_dir),
        };
    };
    let manifest_digest = Digest::of(&manifest_bytes);
    let id = manifest_ids(&manifest_bytes)
        .map(|(_, id, _)| id)
        .unwrap_or_default();
    // The store keeps every build (11.2).
    // The record keeps the digests of `files` only: the manifest and the reports depend on this
    // directory's place and baseline too, and are found from its `.previous` (`graph.md` 6.2).
    store.put_object(&manifest_bytes);
    let mut file_digests = BTreeMap::new();
    for (p, b) in &all_files {
        file_digests.insert(p.clone(), store.put_object(b));
    }
    store.put_object(&changes_bytes);
    store.put_object(&diagnostics);
    if snap::get(&store, &manifest_digest).is_none() {
        let build = model.build();
        snap::put(
            &store,
            &manifest_digest,
            &Snapshot {
                lines: snap::lines_of(&world, &build),
                homes: snap::homes_of(&world, &build),
                build,
            },
        );
    }
    put_record(file_digests);
    all_files.insert("manifest.json".into(), manifest_bytes);
    all_files.insert("changes.json".into(), changes_bytes);
    all_files.insert("diagnostics.json".into(), diagnostics);
    all_files.insert(
        ".previous".into(),
        previous_file(&J::s(id.clone()), &J::s(manifest_digest.hex())),
    );
    // An unchanged rebuild (13): the key equals the directory's `run.key`, its `id` is this
    // build's, and every file of `files` and `run.reports` matches its record. Then nothing is
    // written, `changes.json` included.
    let old = std::fs::read(out_dir.join("manifest.json"))
        .ok()
        .and_then(|b| manifest_ids(&b));
    let same_key = old
        .as_ref()
        .is_some_and(|(_, oid, okey)| *okey == key.hex() && *oid == id);
    let records_match = old.as_ref().is_some_and(|(m, _, _)| {
        let mut listed: Vec<(String, J)> = Vec::new();
        if let Some(J::Obj(f)) = m.get("files") {
            listed.extend(f.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(J::Obj(f)) = m.get("run").and_then(|r| r.get("reports")) {
            listed.extend(f.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        let on_disk = dir::listing(&out_dir);
        on_disk.len() == listed.len() + 2
            && listed.iter().all(|(p, rec)| {
                std::fs::read(out_dir.join(p))
                    .ok()
                    .is_some_and(|b| record_json(p, &b).1 == *rec)
            })
    });
    let unchanged = a.against.is_none() && same_key && records_match;
    let _ = prev_id;
    if !unchanged {
        if same_key {
            for p in all_files
                .iter()
                .filter(|(p, _)| out::is_listed_file(p))
                .filter(|(p, b)| std::fs::read(out_dir.join(p)).ok().as_ref() != Some(*b))
                .map(|(p, _)| p)
            {
                extra_run.push(
                    run_diag(
                        codes::graph::BUILD_FILE_RESTORED,
                        format!("`{p}` differed from its record and was written again"),
                        "nothing to do; the build directory is not edited by hand",
                    )
                    .with("file", p.clone()),
                );
            }
        }
        if let Err(d) = dir::replace(&out_dir, &all_files) {
            extra_run.push(d);
        }
    }
    let mut recs = records;
    for d in extra_run {
        recs.push(d.emit(&world));
    }
    let records = finish(recs, &fail_on);
    let (_, run_r) = split_world_run(records.clone());
    let text = via_graph::report(
        &head(
            Some(id),
            false,
            run_r.iter().map(Record::to_canonical_json).collect(),
        ),
        &run,
    );
    write_run_report(&out_dir, &text, &step_codes);
    Outcome {
        records,
        lints,
        out: Some(out_dir),
    }
}

/// The null build (`graph.md` 6.2): when it applies, the record's world diagnostics and lint
/// list, and a `graph.build_file_restored` for each build file written again from the store.
/// `None` when it does not apply: the key is not the directory's, the record is missing or
/// incomplete, a file's object is missing, or the record's diagnostics cannot be replayed.
///
/// The record keyed by the build key holds the digests of `files` only, which are a function of
/// that key. The manifest and the reports also depend on where the directory is and on its
/// baseline, so they are checked against this directory's own manifest: the one its `.previous`
/// names, kept in the store by digest. Another directory's record never overwrites them.
fn null_build(
    store: &Store,
    key: &Digest,
    manifest_key: Option<&Digest>,
    against: bool,
    out: &Path,
) -> Option<(Vec<Diag>, Vec<Lint>, Vec<Diag>)> {
    let rec = via_graph::null_build(store, key, manifest_key, against)?;
    let (diags, lints) = graph::decode_world(&rec.world_diags)?;
    if rec.files.is_empty() {
        return None;
    }
    // This directory's manifest, as its `.previous` names it.
    let previous = std::fs::read(out.join(".previous")).ok()?;
    let (_, named) = previous_of(out);
    let manifest_digest = Digest::from_hex(named.as_str()?)?;
    let manifest = store.get_object(&manifest_digest)?;
    let (m, _, mkey) = manifest_ids(&manifest)?;
    if mkey != key.hex() {
        return None;
    }
    let listed = |j: Option<&J>| -> Option<BTreeMap<String, Digest>> {
        let mut out = BTreeMap::new();
        if let Some(J::Obj(f)) = j {
            for (p, r) in f {
                out.insert(p.clone(), Digest::from_hex(r.get("digest")?.as_str()?)?);
            }
        }
        Some(out)
    };
    // Its `files` are the record's: the same key gave the same data files.
    if listed(m.get("files"))? != rec.files {
        return None;
    }
    let reports = listed(m.get("run").and_then(|r| r.get("reports")))?;
    let mut expected: BTreeMap<String, Digest> = rec.files.clone();
    expected.extend(reports);
    expected.insert("manifest.json".into(), manifest_digest);
    let mut restore: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (p, d) in &expected {
        let on_disk = std::fs::read(out.join(p)).ok();
        if on_disk.as_deref().map(Digest::of) != Some(*d) {
            restore.insert(p.clone(), store.get_object(d)?);
        }
    }
    let mut restored = Vec::new();
    if !restore.is_empty() {
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for p in expected.keys() {
            match restore.get(p) {
                Some(b) => files.insert(p.clone(), b.clone()),
                None => files.insert(p.clone(), std::fs::read(out.join(p)).ok()?),
            };
        }
        files.insert(".previous".into(), previous);
        if let Err(d) = dir::replace(out, &files) {
            restored.push(d);
        }
        for p in restore.keys() {
            restored.push(
                run_diag(
                    codes::graph::BUILD_FILE_RESTORED,
                    format!("`{p}` differed from its record and was written again"),
                    "nothing to do; the build directory is not edited by hand",
                )
                .with("file", p.clone()),
            );
        }
    }
    Some((diags, lints, restored))
}
