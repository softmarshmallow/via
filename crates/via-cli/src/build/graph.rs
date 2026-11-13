//! The command line's nodes around the model's (`graph.md` 1): `write` per file of the build
//! directory, `step` per lint step, `input` per declared input file. A lint is a node like any
//! other (`graph.md` 4): it reads the finished build through the data files' `write` nodes, so it
//! is reused when they are, and skipped when the build has an error.
//!
//! `write:diagnostics.json`, `write:changes.json` and `write:manifest.json` are defined last,
//! when every other node has settled and the build's world diagnostics are known.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use via_diag::{
    document, exit_status, finish, split_world_run, Diag, DiagCode, FailOn, Lint, Record,
};
use via_doc::World;
use via_graph::{
    object_bytes, BuildRecord, Def, Defined, Digest, Gate, Inputs, Known, NodeId, Output, Plan,
    Status, StepDef, StepRun, Store, Val,
};
use via_model::{FileReads, ModelPlan};

use crate::changes::{self, Chains, Side};
use crate::json::{obj, parse, J};
use crate::out;
use crate::snap::{self, Snapshot};
use crate::steps;

/// The files written last.
pub const DIAGNOSTICS: &str = "write:diagnostics.json";
pub const CHANGES: &str = "write:changes.json";
pub const MANIFEST: &str = "write:manifest.json";

/// One lint step as the plan runs it.
pub struct LintStep {
    pub decl: steps::StepDecl,
    pub code: Result<steps::Code, Diag>,
    pub trusted: bool,
}

/// The baseline of the change report.
pub struct Baseline<'a> {
    pub snapshot: &'a Snapshot,
    pub id: String,
    pub key: String,
}

/// What the command line's nodes are handed.
pub struct Ctx<'a> {
    pub world: &'a World,
    pub store: &'a Store,
    pub roots: &'a steps::Roots,
    pub config: Option<&'a Path>,
    pub sdk: Option<&'a Path>,
    pub sdk_digest: Option<&'a Digest>,
    pub seed: u64,
    pub deadline: Duration,
    pub pack: Option<&'a via_model::pack::Pack>,
    pub name: &'a str,
    pub fail_on: &'a FailOn,
    /// The reading stage's records.
    pub loaded: &'a [Record],
    /// World diagnostics of the command line's own: `identity.no_previous`.
    pub extra: Vec<Diag>,
    pub lints: Vec<LintStep>,
    pub baseline: Option<Baseline<'a>>,
    /// The baseline's build record, for the change report's causes (`graph.md` 6.4).
    pub baseline_record: Option<BuildRecord>,
    /// The build key, the engine version and the pack's root from the directory: parameters of
    /// `write:manifest.json`.
    pub key: Digest,
    pub version: String,
    pub pack_root: Option<String>,
    /// The manifest's `run` members other than `key`, `reports` and `version`.
    pub run_members: BTreeMap<String, J>,
}

/// The data files of the build directory: every file but the reports and the manifest.
fn data_files(model: &ModelPlan<'_>) -> Vec<(String, FileReads)> {
    let mut out = Vec::new();
    for (name, weights) in model.field_kinds() {
        let seg = out::segment(&name);
        if weights {
            for (cx, cy) in model.chunk_list() {
                out.push((
                    format!("weights/{seg}/{cx}_{cy}.npy"),
                    FileReads::Chunk(name.clone(), cx, cy),
                ));
            }
        } else {
            out.push((format!("fields/{seg}.npy"), FileReads::Field(name)));
        }
    }
    out.push(("features.json".into(), FileReads::Features));
    out.push(("meets.json".into(), FileReads::Meets));
    for f in [
        "instances/face.npy",
        "instances/names.txt",
        "instances/scale.npy",
        "instances/tint.npy",
        "instances/x.npy",
        "instances/y.npy",
        "instances/z.npy",
    ] {
        out.push((f.into(), FileReads::Instances));
    }
    out
}

/// The plan of `via build`: the model's nodes and the command line's.
pub struct CliPlan<'a> {
    pub model: &'a ModelPlan<'a>,
    pub ctx: Ctx<'a>,
    defs: BTreeMap<NodeId, Def>,
    data: Vec<String>,
    written: Mutex<Option<Arc<Result<out::Written, Diag>>>>,
    last: Mutex<BTreeMap<String, Vec<u8>>>,
    chains: Mutex<Chains>,
}

fn write_id(path: &str) -> NodeId {
    NodeId::of("write", path)
}

impl<'a> CliPlan<'a> {
    pub fn new(model: &'a ModelPlan<'a>, ctx: Ctx<'a>) -> CliPlan<'a> {
        let mut defs = BTreeMap::new();
        let mut data = Vec::new();
        for (path, reads) in data_files(model) {
            let def = Def {
                params: vec![("path".into(), path.clone())],
                edges: model.file_reads(&reads),
                gate: Gate {
                    skip_on_error: true,
                },
                ..Def::default()
            };
            defs.insert(write_id(&path), def);
            data.push(path);
        }
        for l in &ctx.lints {
            let s = &l.decl;
            let mut edges: Vec<(String, NodeId)> = data
                .iter()
                .map(|p| (format!("data {p}"), write_id(p)))
                .collect();
            let mut inputs: Vec<&String> = s.inputs.iter().collect();
            inputs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            for i in inputs {
                let (path, digest) = input_digest(ctx.world, &s.home, i, ctx.roots);
                let id = NodeId::of("input", &path);
                edges.push((format!("input {path}"), id.clone()));
                defs.insert(
                    id,
                    Def {
                        params: vec![("path".into(), path.clone())],
                        direct: vec![("bytes".into(), digest)],
                        ..Def::default()
                    },
                );
            }
            let statements = model
                .statements()
                .get(&format!("step.{}", s.id))
                .map(|d| vec![(format!("step.{}", s.id), *d)])
                .unwrap_or_default();
            let trust = l
                .code
                .as_ref()
                .map_or("none".to_string(), |c| c.trust.hex());
            defs.insert(
                NodeId::of("step", &s.id),
                Def {
                    subject: Some(via_diag::Subject::entity("step", s.id.clone())),
                    statements,
                    params: vec![
                        ("trust".into(), trust),
                        ("trusted".into(), l.trusted.to_string()),
                        (
                            "sdk".into(),
                            ctx.sdk_digest.map(Digest::hex).unwrap_or_default(),
                        ),
                        ("seed".into(), ctx.seed.to_string()),
                    ],
                    edges,
                    gate: Gate {
                        skip_on_error: true,
                    },
                    step: Some(StepDef {
                        step: s.id.clone(),
                        command: s.command.clone(),
                        lint: true,
                    }),
                    ..Def::default()
                },
            );
        }
        CliPlan {
            model,
            ctx,
            defs,
            data,
            written: Mutex::new(None),
            last: Mutex::new(BTreeMap::new()),
            chains: Mutex::new(BTreeMap::new()),
        }
    }

    /// Every node: the model's, the command line's, and the three defined last.
    pub fn roots(&self) -> Vec<NodeId> {
        let mut r = self.model.ids();
        r.extend(self.defs.keys().cloned());
        for l in [DIAGNOSTICS, CHANGES, MANIFEST] {
            r.push(NodeId::parse(l).expect("an id"));
        }
        r
    }

    /// The data files by path.
    pub fn data_paths(&self) -> &[String] {
        &self.data
    }

    /// The bytes a node defined last computed or was defined with in this run.
    pub fn last_bytes(&self, path: &str) -> Option<Vec<u8>> {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
    }

    fn files(&self) -> Arc<Result<out::Written, Diag>> {
        let mut slot = self
            .written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(w) = slot.as_ref() {
            return w.clone();
        }
        let build = self.model.build();
        let w = Arc::new(out::write(&out::Input {
            world: self.ctx.world,
            build: &build,
            pack: self.ctx.pack,
            name: self.ctx.name,
            seed: self.ctx.seed,
        }));
        *slot = Some(w.clone());
        w
    }

    /// The lint list of `diagnostics.json` (`diagnostics.md` 7), from how each step settled: a
    /// skipped lint has `ran = false`, and so has one that was not started (untrusted).
    pub fn lints(&self, settled: &dyn Fn(&NodeId) -> Option<Settled>) -> Vec<Lint> {
        self.ctx
            .lints
            .iter()
            .map(|l| {
                let id = NodeId::of("step", &l.decl.id);
                let ran = match settled(&id) {
                    Some((Status::Skipped, _)) | None => false,
                    Some((_, Some(vd))) => self
                        .ctx
                        .store
                        .get_object(&vd)
                        .and_then(|o| step_value(&object_bytes(&o)))
                        .is_some_and(|(ran, _)| ran),
                    Some((_, None)) => true,
                };
                Lint {
                    step: l.decl.id.clone(),
                    ran,
                }
            })
            .collect()
    }

    /// Every world record of the build, finished: the reading stage's, the nodes' (each once),
    /// and the command line's own.
    pub fn world_records(&self, node_diags: &[Diag]) -> Vec<Record> {
        let mut all: Vec<Record> = self.ctx.loaded.to_vec();
        for d in via_model::dedupe(node_diags.to_vec())
            .iter()
            .chain(self.ctx.extra.iter())
        {
            all.push(d.emit(self.ctx.world));
        }
        finish(all, self.ctx.fail_on)
    }

    fn define_last(&self, id: &NodeId, known: &Known<'_>) -> Defined {
        let Some(world_diags) = known.world_diags() else {
            return Defined::Last;
        };
        let merge = NodeId::parse("merge").expect("an id");
        let data_edges: Vec<(String, NodeId)> = self
            .data
            .iter()
            .map(|p| (format!("file {p}"), write_id(p)))
            .collect();
        match id.as_str() {
            DIAGNOSTICS => {
                let lints = self.lints(&|n| known.settled(n).map(|s| (s.status, s.value)));
                let records = self.world_records(world_diags);
                let (world_r, _) = split_world_run(records);
                let doc = document(&world_r, &lints, exit_status(&world_r)).into_bytes();
                let digest = Digest::of(&doc);
                self.stash("diagnostics.json", doc);
                Defined::Def(Def {
                    params: vec![("path".into(), "diagnostics.json".into())],
                    edges: vec![("merge".into(), merge)],
                    direct: vec![("document".into(), digest)],
                    ..Def::default()
                })
            }
            CHANGES => {
                if let Some(prev) = &self.ctx.baseline_record {
                    self.find_chains(prev, &|n| known.settled(n).and_then(|s| s.value));
                }
                // The baseline enters by what the report names of it, its `id` and `run.key`
                // (`build.md` 11.2): a baseline rebuilt with equal text is the same baseline.
                let base = self
                    .ctx
                    .baseline
                    .as_ref()
                    .map_or(Digest::of(b"no baseline"), |b| {
                        Digest::of(format!("{}\n{}", b.id, b.key).as_bytes())
                    });
                let mut edges = vec![("merge".to_string(), merge)];
                edges.extend(data_edges);
                Defined::Def(Def {
                    params: vec![("path".into(), "changes.json".into())],
                    edges,
                    direct: vec![("baseline".into(), base)],
                    gate: Gate {
                        skip_on_error: true,
                    },
                    ..Def::default()
                })
            }
            MANIFEST => {
                let mut edges = data_edges;
                edges.push((
                    "file changes.json".into(),
                    NodeId::parse(CHANGES).expect("an id"),
                ));
                edges.push((
                    "file diagnostics.json".into(),
                    NodeId::parse(DIAGNOSTICS).expect("an id"),
                ));
                for r in self.model.ids() {
                    if r.family() == "source" {
                        edges.push((format!("source {}", &r.as_str()[7..]), r));
                    }
                }
                for r in self.defs.keys() {
                    if r.family() == "input" {
                        edges.push((format!("input {}", &r.as_str()[6..]), r.clone()));
                    }
                }
                Defined::Def(Def {
                    params: vec![
                        ("path".into(), "manifest.json".into()),
                        ("version".into(), self.ctx.version.clone()),
                        ("build key".into(), self.ctx.key.hex()),
                        (
                            "pack root".into(),
                            self.ctx.pack_root.clone().unwrap_or_default(),
                        ),
                    ],
                    edges,
                    gate: Gate {
                        skip_on_error: true,
                    },
                    ..Def::default()
                })
            }
            _ => Defined::Def(Def::default()),
        }
    }

    fn stash(&self, path: &str, bytes: Vec<u8>) {
        self.last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_string(), bytes);
    }

    /// The causes of each field's change (`graph.md` 6.4): from every chunk whose value differs
    /// from the baseline's, the shortest edge path back to each changed statement.
    fn find_chains(&self, prev: &BuildRecord, value: &dyn Fn(&NodeId) -> Option<Digest>) {
        let mut made_from: BTreeMap<NodeId, Vec<String>> = BTreeMap::new();
        let mut edges: BTreeMap<NodeId, Vec<(String, NodeId)>> = BTreeMap::new();
        for id in self.model.ids() {
            if let Some(d) = self.model.def(&id) {
                made_from.insert(
                    id.clone(),
                    d.statements.iter().map(|(s, _)| s.clone()).collect(),
                );
                edges.insert(id.clone(), d.edges.clone());
            }
        }
        let mut chains: Chains = BTreeMap::new();
        for (name, _) in self.model.field_kinds() {
            let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (_, chunk) in self.model.file_reads(&FileReads::Field(name.clone())) {
                let now = value(&chunk);
                let before = prev.nodes.get(&chunk).and_then(|n| n.value);
                if now == before {
                    continue;
                }
                for c in via_graph::causes_from(
                    self.model.statements(),
                    &made_from,
                    &edges,
                    prev,
                    &chunk,
                ) {
                    let path: Vec<String> = c.path.iter().map(ToString::to_string).collect();
                    let better = m.get(&c.statement).is_none_or(|p| path.len() < p.len());
                    if better {
                        m.insert(c.statement, path);
                    }
                }
            }
            chains.insert(name, m);
        }
        *self
            .chains
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = chains;
    }

    fn compute_changes(&self) -> Output {
        let Some(b) = &self.ctx.baseline else {
            let bytes = changes::without_baseline().file();
            self.stash("changes.json", bytes.clone());
            return Output::of(Val::plain(bytes));
        };
        let build = self.model.build();
        let bytes = match &build.changes {
            Some(report) => {
                let lines = snap::lines_of(self.ctx.world, &build);
                let homes = snap::homes_of(self.ctx.world, &build);
                let chains = self
                    .chains
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                changes::write(
                    report,
                    &Side {
                        build: &build,
                        lines: &lines,
                        homes: &homes,
                    },
                    b.snapshot,
                    (b.id.clone(), b.key.clone()),
                    &chains,
                )
                .file()
            }
            None => changes::without_baseline().file(),
        };
        self.stash("changes.json", bytes.clone());
        Output::of(Val::plain(bytes))
    }

    fn compute_manifest(&self, inputs: &Inputs<'_>) -> Output {
        let w = self.files();
        let head = match w.as_ref() {
            Ok(w) => w.head.clone(),
            Err(d) => {
                return Output::of(Val {
                    bytes: Arc::new(Vec::new()),
                    diags: Arc::new(vec![d.clone()]),
                })
            }
        };
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut reports: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (_, node, v) in inputs.all() {
            if node.family() != "write" {
                continue;
            }
            let path = node.as_str()["write:".len()..].to_string();
            if path == "diagnostics.json" || path == "changes.json" {
                reports.insert(path, v.bytes.as_ref().clone());
            } else if !(v.bytes.is_empty() && path.starts_with("weights/")) {
                files.insert(path, v.bytes.as_ref().clone());
            }
        }
        let bytes = manifest_bytes(head, &files, &reports, &self.ctx);
        self.stash("manifest.json", bytes.clone());
        Output::of(Val::plain(bytes))
    }

    fn compute_step(&self, id: &NodeId) -> Output {
        let sid = &id.as_str()["step:".len()..];
        let Some(l) = self.ctx.lints.iter().find(|l| l.decl.id == sid) else {
            return Output::of(Val::plain(Vec::new()));
        };
        let code = match &l.code {
            Ok(c) => c,
            Err(d) => {
                return Output::of(Val {
                    bytes: Arc::new(step_bytes(false, &[])),
                    diags: Arc::new(vec![d.clone()]),
                })
            }
        };
        let build = self.model.build();
        let call = steps::Call {
            world: self.ctx.world,
            build: &build,
            store: self.ctx.store,
            roots: self.ctx.roots,
            config: self.ctx.config,
            sdk: self.ctx.sdk,
            sdk_digest: self.ctx.sdk_digest,
            seed: self.ctx.seed,
            deadline: self.ctx.deadline,
            pack: self.ctx.pack,
        };
        let r = steps::run_lint(&l.decl, code, l.trusted, &call);
        let exit = r.record.get("exit").and_then(J::as_i64);
        let files: Vec<(String, Digest)> = r
            .record
            .get("returned")
            .and_then(J::as_arr)
            .unwrap_or(&[])
            .iter()
            .filter_map(|f| {
                Some((
                    f.get("path")?.as_str()?.to_string(),
                    Digest::from_hex(f.get("digest")?.as_str()?)?,
                ))
            })
            .collect();
        let log = r
            .record
            .get("log")
            .and_then(J::as_str)
            .and_then(Digest::from_hex);
        // A step that ran and did not exit 0, or was killed, did not complete: it is not
        // stored and runs again next build (`graph.md` 3.4). Nor did one an error kept from
        // starting (`protocol.no_program`): the program's path is in no key, so configuring it
        // must not replay the error. An untrusted skip is a warning, and trust is keyed.
        let complete = if r.ran {
            exit == Some(0)
        } else {
            !r.diags
                .iter()
                .any(|d| d.severity() == via_diag::Severity::Error)
        };
        Output {
            value: Val {
                bytes: Arc::new(step_bytes(r.ran, &r.explains)),
                diags: Arc::new(r.diags),
            },
            complete,
            step: Some(StepRun {
                exit,
                files,
                log,
                ..StepRun::default()
            }),
        }
    }
}

/// A lint step's value: whether its process was started, and the explain text of each of its
/// codes, as canonical JSON.
fn step_bytes(ran: bool, explains: &[(String, Option<String>)]) -> Vec<u8> {
    obj([
        (
            "explains",
            J::Arr(
                explains
                    .iter()
                    .map(|(c, e)| J::Arr(vec![J::s(c.clone()), J::opt(e.as_deref())]))
                    .collect(),
            ),
        ),
        ("ran", J::Bool(ran)),
    ])
    .file()
}

/// How a node settled: its status and, when it has one, its value's digest.
pub type Settled = (Status, Option<Digest>);

/// A lint's explain texts: each code with its explain, when it has one.
pub type Explains = Vec<(String, Option<String>)>;

/// A lint step's value read back: whether it ran, and its explain texts.
pub fn step_value(bytes: &[u8]) -> Option<(bool, Explains)> {
    let j = parse(std::str::from_utf8(bytes).ok()?).ok()?;
    let ran = matches!(j.get("ran"), Some(J::Bool(true)));
    let explains = j
        .get("explains")
        .and_then(J::as_arr)
        .unwrap_or(&[])
        .iter()
        .filter_map(|e| {
            let a = e.as_arr()?;
            Some((
                a.first()?.as_str()?.to_string(),
                a.get(1).and_then(J::as_str).map(str::to_string),
            ))
        })
        .collect();
    Some((ran, explains))
}

/// An input's id path and digest: relative to the world's folder with `/` where it can be, and
/// the digest of a folder's listing (`graph.md` 5.1); a missing path digests as missing.
fn input_digest(
    world: &World,
    home: &Path,
    written: &str,
    roots: &steps::Roots,
) -> (String, Digest) {
    let _ = world;
    let p = home.join(written);
    let rel = std::path::absolute(&p)
        .ok()
        .and_then(|a| {
            std::path::absolute(&roots.world_dir)
                .ok()
                .and_then(|w| a.strip_prefix(w).ok().map(Path::to_path_buf))
        })
        .map_or_else(
            || written.to_string(),
            |r| {
                r.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            },
        );
    let digest = if p.is_dir() {
        steps::folder_digest(&p).0
    } else {
        std::fs::read(&p).map_or_else(|_| Digest::of(b"\0missing"), |b| Digest::of(&b))
    };
    (rel, digest)
}

/// `manifest.json` (`build.md` 4): the world keys, every data file's record, the run, and the
/// pack's root; its `id` is the digest of the rest (`build.md` 12).
pub fn manifest_bytes(
    head: BTreeMap<String, J>,
    files: &BTreeMap<String, Vec<u8>>,
    reports: &BTreeMap<String, Vec<u8>>,
    ctx: &Ctx<'_>,
) -> Vec<u8> {
    let record = |bytes: &[u8]| {
        obj([
            ("bytes", J::count(bytes.len())),
            ("digest", J::s(Digest::of(bytes).hex())),
        ])
    };
    let mut manifest = head;
    manifest.insert(
        "files".into(),
        J::Obj(files.iter().map(|(p, b)| (p.clone(), record(b))).collect()),
    );
    let id = Digest::of(J::Obj(manifest.clone()).canonical().as_bytes()).hex();
    manifest.insert("id".into(), J::s(id));
    let mut run = ctx.run_members.clone();
    run.insert("key".into(), J::s(ctx.key.hex()));
    run.insert(
        "reports".into(),
        J::Obj(
            reports
                .iter()
                .map(|(p, b)| (p.clone(), record(b)))
                .collect(),
        ),
    );
    run.insert("version".into(), J::s(ctx.version.clone()));
    manifest.insert("run".into(), J::Obj(run));
    manifest.insert(
        "locations".into(),
        obj([("pack_root", ctx.pack_root.clone().map_or(J::Null, J::s))]),
    );
    J::Obj(manifest).file()
}

impl Plan for CliPlan<'_> {
    fn define(&self, id: &NodeId, known: &Known<'_>) -> Defined {
        if self.model.def(id).is_some() {
            return self.model.define(id, known);
        }
        if let Some(d) = self.defs.get(id) {
            return Defined::Def(d.clone());
        }
        self.define_last(id, known)
    }

    fn compute(&self, id: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output {
        if self.model.def(id).is_some() {
            return self.model.compute(id, def, inputs);
        }
        match id.family() {
            "input" => Output::of(Val::plain(
                def.direct
                    .first()
                    .map(|(_, d)| d.bytes().to_vec())
                    .unwrap_or_default(),
            )),
            "step" => self.compute_step(id),
            "write" => match id.as_str() {
                DIAGNOSTICS => Output::of(Val::plain(
                    self.last_bytes("diagnostics.json").unwrap_or_default(),
                )),
                CHANGES => self.compute_changes(),
                MANIFEST => self.compute_manifest(inputs),
                _ => {
                    let path = &id.as_str()["write:".len()..];
                    match self.files().as_ref() {
                        Ok(w) => {
                            Output::of(Val::plain(w.files.get(path).cloned().unwrap_or_default()))
                        }
                        Err(d) => Output::of(Val {
                            bytes: Arc::new(Vec::new()),
                            diags: Arc::new(vec![d.clone()]),
                        }),
                    }
                }
            },
            _ => Output::of(Val::plain(Vec::new())),
        }
    }
}

/// The world diagnostics of a build as its record keeps them (`graph.md` 6.2), for the null
/// build to replay: the nodes' and the command line's diagnostics, and the lint list.
pub fn encode_world(diags: &[Diag], lints: &[Lint]) -> Vec<u8> {
    use via_graph::DiagCodec;
    let mut e = via_graph::Enc::new();
    e.str("via-world-1");
    match via_graph::CoreCodec.encode(diags) {
        Some(b) => {
            e.byte(1).bytes(&b);
        }
        None => {
            e.byte(0);
        }
    }
    e.len(lints.len());
    for l in lints {
        e.str(&l.step).byte(u8::from(l.ran));
    }
    e.finish()
}

/// [`encode_world`] read back; `None` when the diagnostics could not be kept, so the null
/// build cannot replay them.
pub fn decode_world(bytes: &[u8]) -> Option<(Vec<Diag>, Vec<Lint>)> {
    use via_graph::DiagCodec;
    let mut d = via_graph::Dec::new(bytes);
    if d.str()? != "via-world-1" || d.byte()? != 1 {
        return None;
    }
    let diags = via_graph::CoreCodec.decode(d.bytes()?)?;
    let mut lints = Vec::new();
    for _ in 0..d.count()? {
        let step = d.str()?;
        let ran = d.byte()? == 1;
        lints.push(Lint { step, ran });
    }
    d.done().then_some((diags, lints))
}

/// The step codes a run report lists for `via explain`: each code a lint sent, its explain text
/// and its severity.
pub fn step_codes(explains: &[(String, Option<String>)], diags: &[Diag]) -> Vec<J> {
    let mut severity: BTreeMap<String, &'static str> = BTreeMap::new();
    for d in diags {
        if let DiagCode::Step {
            step,
            name,
            severity: sv,
        } = d.code()
        {
            severity.insert(format!("step.{step}.{name}"), sv.as_str());
        }
    }
    let mut seen = BTreeSet::new();
    explains
        .iter()
        .filter(|(c, _)| seen.insert(c.clone()))
        .map(|(c, e)| {
            obj([
                ("code", J::s(c.clone())),
                ("explain", J::opt(e.as_deref())),
                ("severity", J::s(severity.get(c).copied().unwrap_or("info"))),
            ])
        })
        .collect()
}
