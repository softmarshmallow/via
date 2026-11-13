//! `via build WORLD` (`build.md` 13 and 14): read, build through the graph, run the lint steps,
//! write the directory whole or leave it untouched, and the run report beside it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use via_diag::{
    codes, document, exit_status, finish, split_world_run, Diag, FailOn, Lint, NoWorld, Record,
    Site, Subject,
};
use via_doc::{ChangeRequest, LoadOptions, World};
use via_graph::{BuildKeyParts, BuildRecord, Digest, NodeId, Store, Versions};

use crate::changes::{self, Chains, Side};
use crate::dir;
use crate::json::{obj, parse, J};
use crate::out::{self, EPOCH};
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
        epoch: EPOCH,
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

/// Writes the run report beside the directory, adding the lint steps' records and codes.
fn write_run_report(out: &Path, text: &str, step_records: &[J], step_codes: &[J]) {
    let mut j = parse(text).unwrap_or(J::Obj(J::obj()));
    if let J::Obj(m) = &mut j {
        m.insert("step_calls".into(), J::Arr(step_records.to_vec()));
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
                return finish_run(
                    loaded.records,
                    run_diags,
                    loaded.world.as_ref(),
                    Vec::new(),
                    None,
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
            Some((snap::get(&store, &d)?, id, key))
        });
        match found {
            Some((s, id, key)) => baseline = Some((s, J::Null, id, key)),
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
    let mut codes_of: BTreeMap<String, Result<steps::Code, Diag>> = BTreeMap::new();
    for s in &decls {
        codes_of.insert(
            s.id.clone(),
            steps::code_of(s, &roots, &wdir, sdk_digest.as_ref()),
        );
    }
    // The build key.
    let mut parts = BuildKeyParts {
        changes: world.changes().to_vec(),
        ..BuildKeyParts::default()
    };
    let mut inputs: Vec<(String, Digest)> = world
        .sources()
        .iter()
        .map(|s| (s.name.clone(), Digest::of(s.text().as_bytes())))
        .collect();
    inputs.sort_by(|x, y| x.0.as_bytes().cmp(y.0.as_bytes()));
    inputs.dedup_by(|x, y| x.0 == y.0);
    parts.files = inputs.clone();
    for (id, c) in &codes_of {
        if let Ok(c) = c {
            parts.trust.push((id.clone(), c.trust));
        }
    }
    let mut fail_sorted = a.fail_on.clone();
    fail_sorted.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
    fail_sorted.dedup();
    for c in &fail_sorted {
        parts.flags.push(("fail_on".into(), c.clone()));
    }
    let versions = versions();
    let key = via_graph::build_key(&versions, &parts);
    // The graph.
    let prev_record = prev_manifest
        .as_str()
        .and_then(Digest::from_hex)
        .and_then(|d| store.get_object(&d))
        .and_then(|b| manifest_ids(&b))
        .and_then(|(_, _, k)| Digest::from_hex(&k))
        .and_then(|k| BuildRecord::get(&store, &k));
    let mut gopts = via_graph::Options::new(versions.clone());
    gopts.threads = a
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
        .max(1);
    gopts.rerun = a.rerun.iter().cloned().collect();
    gopts.previous = prev_record.clone();
    let started = std::time::Instant::now();
    let base_build = baseline.as_ref().map(|b| &b.0.build);
    let (build, run) = via_model::build_graph(&world, &model_opts, base_build, &store, &gopts);
    let mut raised: Vec<Diag> = build.diagnostics.clone();
    if baseline_missing {
        raised.retain(|d| *d.code() != via_diag::DiagCode::Core(codes::identity::NO_PREVIOUS));
    }
    if let Some(r) = &run {
        run_diags.extend(r.run_diags.iter().cloned());
    }
    run_diags.extend(store.take_run_diags());
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
    // Lint steps: handed the whole build, so any error skips them (`graph.md` 4).
    let has_error = loaded.records.iter().any(Record::fails)
        || raised
            .iter()
            .any(|d| d.severity() == via_diag::Severity::Error);
    let mut lints = Vec::new();
    let mut step_records = Vec::new();
    let mut step_codes = Vec::new();
    for s in decls.iter().filter(|s| s.when == "lint" && !s.worker) {
        let code = match codes_of.get(&s.id) {
            Some(Ok(c)) => c.clone(),
            Some(Err(d)) => {
                raised.push(d.clone());
                lints.push(Lint {
                    step: s.id.clone(),
                    ran: false,
                });
                continue;
            }
            None => continue,
        };
        if has_error {
            lints.push(Lint {
                step: s.id.clone(),
                ran: false,
            });
            continue;
        }
        let call = steps::Call {
            world: &world,
            build: &build,
            store: &store,
            roots: &roots,
            config: config.as_deref(),
            sdk: a.sdk.as_deref(),
            sdk_digest: sdk_digest.as_ref(),
            seed: world_seed(&world),
            deadline: Duration::from_secs(a.deadline.unwrap_or(600)),
            pack: pack.as_ref().map(|p| &p.0),
        };
        let r = steps::run_lint(s, &code, trusted.contains(&code.trust.hex()), &call);
        let mut severity: BTreeMap<String, &'static str> = BTreeMap::new();
        for d in &r.diags {
            if let via_diag::DiagCode::Step {
                step,
                name,
                severity: sv,
            } = d.code()
            {
                severity.insert(format!("step.{step}.{name}"), sv.as_str());
            }
        }
        lints.push(Lint {
            step: s.id.clone(),
            ran: r.ran,
        });
        if r.record != J::Null {
            step_records.push(r.record);
        }
        for (c, e) in r.explains {
            let sv = severity.get(&c).copied().unwrap_or("info");
            step_codes.push(obj([
                ("code", J::s(c)),
                ("explain", J::opt(e.as_deref())),
                ("severity", J::s(sv)),
            ]));
        }
        raised.extend(r.diags);
    }
    // `diag.skipped` (`diagnostics.md` 4): in M1 a lint is handed the whole build, so every failed
    // entity has the skipped lints as its dependents; one record per failed entity, at its header.
    let skipped: Vec<String> = lints
        .iter()
        .filter(|l| !l.ran && has_error)
        .map(|l| format!("step.{}", l.step))
        .collect();
    if !skipped.is_empty() {
        let mut failed: BTreeSet<String> = BTreeSet::new();
        let mut extra = Vec::new();
        for d in raised
            .iter()
            .filter(|d| d.severity() == via_diag::Severity::Error)
        {
            let subject = d.subject().address();
            if !failed.insert(subject.to_string()) {
                continue;
            }
            let site = match subject {
                Subject::Entity { .. } => Site::Header,
                _ => Site::World,
            };
            let n = u32::try_from(skipped.len()).unwrap_or(u32::MAX);
            extra.push(
                Diag::new(
                    codes::diag::SKIPPED,
                    subject,
                    site,
                    via_diag::text!("nodes that read a failed node were not built"),
                    via_diag::text!(
                        "fix the error on this subject; what was skipped is built with it"
                    ),
                )
                .counted(n)
                .with(
                    "first",
                    via_diag::Value::List(
                        skipped
                            .iter()
                            .take(10)
                            .map(|s| via_diag::Value::Text(s.clone()))
                            .collect(),
                    ),
                ),
            );
        }
        raised.extend(extra);
    }
    // Every record, finished once.
    let mut all = loaded.records;
    for d in &raised {
        all.push(d.emit(&world));
    }
    for d in &run_diags {
        all.push(d.emit(&world));
    }
    let records = finish(all, &fail_on);
    let (world_r, run_r) = split_world_run(records.clone());
    let world_status = exit_status(&world_r);
    let diagnostics = document(&world_r, &lints, world_status);
    let head = |id: Option<String>, null: bool| via_graph::ReportHead {
        epoch: EPOCH,
        version: env!("CARGO_PKG_VERSION").to_string(),
        threads: u64::try_from(gopts.threads).unwrap_or(1),
        key,
        id,
        null,
        previous: prev_record.as_ref().and(
            prev_manifest
                .as_str()
                .and_then(Digest::from_hex)
                .and_then(|d| store.get_object(&d))
                .and_then(|b| manifest_ids(&b))
                .and_then(|(_, _, k)| Digest::from_hex(&k)),
        ),
        ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        run_records: run_r.iter().map(Record::to_canonical_json).collect(),
    };
    if world_r.iter().any(Record::fails) {
        let mut extra = Vec::new();
        if let Some(d) = write_failed_dir(&out_dir, diagnostics.as_bytes()) {
            extra.push(d.emit(&world));
        }
        if let Some(r) = &run {
            write_run_report(
                &out_dir,
                &via_graph::report(&head(None, false), r),
                &step_records,
                &step_codes,
            );
        }
        let mut recs = records;
        recs.extend(extra);
        return Outcome {
            records: recs,
            lints,
            out: Some(out_dir),
        };
    }
    // The files of the id.
    let written = match out::write(&out::Input {
        world: &world,
        build: &build,
        pack: pack.as_ref().map(|p| &p.0),
        name: &name,
        seed: world_seed(&world),
    }) {
        Ok(w) => w,
        Err(d) => {
            let mut recs = records;
            recs.push(d.emit(&world));
            return Outcome {
                records: finish(recs, &fail_on),
                lints,
                out: Some(out_dir),
            };
        }
    };
    let mut manifest = written.head;
    manifest.insert(
        "files".into(),
        J::Obj(
            written
                .files
                .iter()
                .map(|(p, b)| record_json(p, b))
                .collect(),
        ),
    );
    let id = Digest::of(J::Obj(manifest.clone()).canonical().as_bytes()).hex();
    manifest.insert("id".into(), J::s(id.clone()));
    // changes.json against the baseline.
    let changes_j = match (&baseline, &build.changes) {
        (Some((snap_b, _, bid, bkey)), Some(report)) => {
            let lines = snap::lines_of(&world, &build);
            let homes = snap::homes_of(&world, &build);
            let mut chains: Chains = BTreeMap::new();
            if let (Some(r), Some(grid), Some(prev)) = (
                &run,
                build.grid,
                // A malformed baseline key looks up nothing, rather than the all-zero key.
                Digest::from_hex(bkey).and_then(|k| BuildRecord::get(&store, &k)),
            ) {
                for e in report.entries.iter().filter(|e| e.table == "field") {
                    let node = NodeId::of("field", &e.name)
                        .view("build")
                        .window(0, 0, grid.nx, grid.ny);
                    let mut m = BTreeMap::new();
                    for c in via_graph::causes(r, &prev, &node) {
                        m.insert(
                            c.statement,
                            c.path.iter().map(ToString::to_string).collect(),
                        );
                    }
                    chains.insert(e.name.clone(), m);
                }
            }
            changes::write(
                report,
                &Side {
                    build: &build,
                    lines: &lines,
                    homes: &homes,
                },
                snap_b,
                (bid.clone(), bkey.clone()),
                &chains,
            )
        }
        _ => changes::without_baseline(),
    };
    let changes_bytes = changes_j.file();
    let diag_bytes = diagnostics.into_bytes();
    let mut flags = J::obj();
    flags.insert(
        "fail_on".into(),
        J::strs(fail_sorted.iter().map(String::as_str)),
    );
    flags.insert("stand_ins".into(), J::Bool(false));
    let mut reports = J::obj();
    for (p, b) in [
        ("changes.json", &changes_bytes),
        ("diagnostics.json", &diag_bytes),
    ] {
        let (k, v) = record_json(p, b);
        reports.insert(k, v);
    }
    manifest.insert(
        "run".into(),
        obj([
            (
                "changes",
                J::strs(world.changes().iter().map(String::as_str)),
            ),
            ("flags", J::Obj(flags)),
            (
                "inputs",
                J::Arr(
                    inputs
                        .iter()
                        .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
                        .collect(),
                ),
            ),
            ("key", J::s(key.hex())),
            ("reports", J::Obj(reports)),
            ("version", J::s(env!("CARGO_PKG_VERSION"))),
        ]),
    );
    let pack_root = pack
        .as_ref()
        .and_then(|(_, p)| p.parent())
        .and_then(|p| relative(&out_dir, p));
    let mut extra_run: Vec<Diag> = Vec::new();
    if pack.is_some() && pack_root.is_none() {
        extra_run.push(run_diag(
            codes::build::PACK_ROOT_FAR,
            "no relative path leads from the build directory to the pack's root".into(),
            "build onto the pack's drive, or tell the loader where the pack is",
        ));
    }
    manifest.insert(
        "locations".into(),
        obj([("pack_root", pack_root.map_or(J::Null, J::s))]),
    );
    let manifest_bytes = J::Obj(manifest).file();
    let manifest_digest = Digest::of(&manifest_bytes);
    // The store keeps every build (11.2).
    store.put_object(&manifest_bytes);
    let mut file_digests = BTreeMap::new();
    for (p, b) in &written.files {
        file_digests.insert(p.clone(), store.put_object(b));
    }
    snap::put(
        &store,
        &manifest_digest,
        &Snapshot {
            lines: snap::lines_of(&world, &build),
            homes: snap::homes_of(&world, &build),
            build: build.clone(),
        },
    );
    if let Some(r) = &run {
        let world_json: Vec<String> = world_r.iter().map(Record::to_canonical_json).collect();
        r.record(&versions, world_json.join("\n").into_bytes(), file_digests)
            .put(&store, &key);
    }
    // The directory: written whole, or left as it is when every byte already matches (13).
    let mut all_files = written.files;
    all_files.insert("manifest.json".into(), manifest_bytes);
    all_files.insert("changes.json".into(), changes_bytes);
    all_files.insert("diagnostics.json".into(), diag_bytes);
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
    if let Some(r) = &run {
        let mut h = head(Some(id), false);
        h.run_records = run_r.iter().map(Record::to_canonical_json).collect();
        write_run_report(
            &out_dir,
            &via_graph::report(&h, r),
            &step_records,
            &step_codes,
        );
    }
    Outcome {
        records,
        lints,
        out: Some(out_dir),
    }
}
