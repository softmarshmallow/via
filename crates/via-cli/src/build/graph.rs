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

/// One step as the plan runs it: a lint or a generate step, plain or worker.
#[derive(Clone)]
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
    /// Read from an `--against` directory's files, the store holding no record of it: the
    /// report compares the files and its causes are `null` (`graph.md` 6.4).
    pub files_only: bool,
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
    /// The `via trust` command that trusts this build's steps, with its flags
    /// (`protocol.untrusted_skipped`'s hint).
    pub trust_command: String,
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
    /// `locations.pack_roots`: each import whose pack brought a model asset, to its root's path
    /// from the directory (`build.md` 4).
    pub pack_roots: BTreeMap<String, String>,
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
    // The terrain's cut (`build.md` 6.5) and the kind surfaces' meshes (10).
    if let Some(t) = model.terrain() {
        out.push((out::cut_path(&t), FileReads::Cut));
    }
    for (place, owner) in model.kind_surface_places() {
        out.push((out::mesh_path(&place), FileReads::Mesh(owner)));
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

/// What the export steps wrote, by build path, and the manifest's `exports` (`build.md` 11.4).
pub type Exported = (BTreeMap<String, Vec<u8>>, BTreeMap<String, J>);

/// The plan of `via build`: the model's nodes and the command line's.
/// How every node settled and what each step read, kept for [`CliPlan::find_chains`].
type ChainInputs = (
    BTreeMap<NodeId, via_graph::Settled>,
    BTreeMap<NodeId, Vec<via_graph::Read>>,
);

pub struct CliPlan<'a> {
    pub model: &'a ModelPlan<'a>,
    pub ctx: Ctx<'a>,
    defs: BTreeMap<NodeId, Def>,
    /// Each `for_each` step's join, its items listed when it is first defined (5.3).
    each: BTreeMap<NodeId, Each>,
    /// The item nodes of each join defined so far.
    items: Mutex<BTreeMap<NodeId, Def>>,
    data: Vec<String>,
    written: Mutex<Option<Arc<Result<out::Written, Diag>>>>,
    last: Mutex<BTreeMap<String, Vec<u8>>>,
    chains: Mutex<Chains>,
    /// What the chains are found from, kept when `write:changes.json` is defined and used only
    /// when it runs: finding them reads the whole build, which a reused report never needs.
    chain_inputs: Mutex<Option<ChainInputs>>,
    /// What failed or was skipped, once every ungated node settled: a traced lint's reads are
    /// checked against it (`diagnostics.md` 4).
    bad: Mutex<Option<Arc<crate::stepview::Bad>>>,
    /// The generate steps some lint's `checks` names (`derived.md` 5).
    checked: BTreeSet<String>,
    /// The build's workers, shut down when the plan is dropped (`protocol.md` 3.2).
    pool: crate::stepwork::Pool,
    /// The model's build, made once: what every step call and trace replay reads.
    build: std::sync::OnceLock<Arc<via_model::Build>>,
    /// The digest of everything the model's build is made from ([`CliPlan::leaves_digest`]).
    leaves: std::sync::OnceLock<Digest>,
}

/// A `for_each` generate step (`graph.md` 5.3): what each item node is made from, and the join's
/// statements, selector and rank.
struct Each {
    item: Def,
    statements: Vec<(String, Digest)>,
    kind: String,
    rank: Option<i64>,
}

/// A `for_each` step's items as a store object.
fn encode_items(list: &[String]) -> Vec<u8> {
    let mut e = via_graph::Enc::new();
    e.str("via-items-1").len(list.len());
    for i in list {
        e.str(i);
    }
    e.finish()
}

fn decode_items(b: &[u8]) -> Option<Vec<String>> {
    let mut d = via_graph::Dec::new(b);
    if d.str()? != "via-items-1" {
        return None;
    }
    let n = d.count()?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(d.str()?);
    }
    d.done().then_some(out)
}

fn write_id(path: &str) -> NodeId {
    NodeId::of("write", path)
}

impl<'a> CliPlan<'a> {
    pub fn new(model: &'a ModelPlan<'a>, ctx: Ctx<'a>) -> CliPlan<'a> {
        let mut defs = BTreeMap::new();
        let mut data = Vec::new();
        for (path, reads) in data_files(model) {
            let mut params = vec![("path".into(), path.clone())];
            // The instance columns are sorted by chunk (`build.md` 8): the chunk side is a
            // parameter of theirs, so a changed `chunk` never reuses rows in the old order.
            if matches!(reads, FileReads::Instances) {
                params.push((
                    "chunk".into(),
                    model.chunk_side_mm().unwrap_or(0).to_string(),
                ));
            }
            let def = Def {
                params,
                edges: model.file_reads(&reads),
                statements: model.file_statements(&reads),
                gate: Gate {
                    skip_on_error: true,
                    traced: false,
                },
                ..Def::default()
            };
            defs.insert(write_id(&path), def);
            data.push(path);
        }
        // The budget (`build.md` 5.1): one node, when the world writes `[budget]`, reading what
        // the instance columns read (every row's place and asset, so its chunk and its load),
        // the frame (the chunk side), the pack (each asset's triangles) and the budget's own
        // statements. Its value is its diagnostics. Not `names.txt` alone: names in chunk order
        // stay the same when a row crosses a chunk edge or swaps its asset, and a warm build
        // then replayed a clean build's `build.over_budget` as nothing (M6 slice 1 review).
        if model.budget().is_some() {
            let mut statements: Vec<(String, Digest)> = model
                .statements()
                .iter()
                .filter(|(k, _)| {
                    k.as_str() == "world"
                        || k.as_str() == via_model::PACK
                        || k.starts_with("budget.")
                })
                .map(|(k, d)| (k.clone(), *d))
                .collect();
            statements.extend(model.file_statements(&FileReads::Instances));
            statements.sort();
            statements.dedup();
            defs.insert(
                NodeId::of("budget", "chunks"),
                Def {
                    params: vec![
                        ("budget".into(), "chunks".into()),
                        (
                            "chunk".into(),
                            model.chunk_side_mm().unwrap_or(0).to_string(),
                        ),
                    ],
                    edges: model.file_reads(&FileReads::Instances),
                    statements,
                    // Not gated (`build.md` 5.1, "When it is checked"): it settles as soon as what
                    // it reads has, before the gates open, so a chunk over budget skips the
                    // writes, lints and exports instead of running beside them (atlas
                    // `over_budget` failed only after 175 s, TODO "From M6 close").
                    gate: Gate {
                        skip_on_error: false,
                        traced: false,
                    },
                    ..Def::default()
                },
            );
        }
        // The diagnostics a lint returns are mapped to the world's entities by id
        // (`protocol.md` 5): the set of ids enters its key, so a renamed or removed entity
        // never replays a finding mapped to the old one.
        let entities = {
            let mut e = via_graph::Enc::new();
            e.str("via-entities-1");
            let mut all: Vec<String> = Vec::new();
            for (top, ent) in ctx.world.root().iter() {
                if let via_doc::Value::Table(t) = &ent.item.value {
                    // Produced names (`<step>#<key>`) are left out: a step's diagnostics map to what
                    // was written, and a joined document must not change its own step's key.
                    for id in t.keys().filter(|id| !id.contains('#')) {
                        all.push(format!("{top}.{id}"));
                    }
                }
            }
            all.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            e.len(all.len());
            for a in &all {
                e.str(a);
            }
            Digest::of(&e.finish()).hex()
        };
        let generate: Vec<&str> = ctx
            .lints
            .iter()
            .filter(|l| l.decl.when == "generate")
            .map(|l| l.decl.id.as_str())
            .collect();
        let mut checked: BTreeSet<String> = BTreeSet::new();
        let infos: Vec<via_model::derived_checks::StepInfo> = ctx
            .lints
            .iter()
            .map(|l| via_model::derived_checks::StepInfo {
                id: l.decl.id.clone(),
                when: l.decl.when.clone(),
                checks: (!l.decl.checks.is_empty()).then(|| l.decl.checks.clone()),
            })
            .collect();
        let mut each: BTreeMap<NodeId, Each> = BTreeMap::new();
        for l in &ctx.lints {
            let s = &l.decl;
            let lint = s.when == "lint";
            // Statically found faults of the step's own keys (`derived.md` 5, `protocol.md` 1.2).
            let mut found: Vec<Diag> = Vec::new();
            let key_diag = |code: via_diag::Code, key: &str, msg: String, hint: String| {
                Diag::new(
                    code,
                    via_diag::Subject::entity("step", s.id.clone()),
                    via_diag::Site::Key(via_diag::KeyPath::key(key)),
                    crate::txt(msg),
                    crate::txt(hint),
                )
            };
            // The `checks` key (`derived.md` 5): the one definition, in via-model.
            let only_mine: Vec<via_model::derived_checks::StepInfo> = infos
                .iter()
                .map(|i| via_model::derived_checks::StepInfo {
                    checks: if i.id == s.id { i.checks.clone() } else { None },
                    ..i.clone()
                })
                .collect();
            found.extend(via_model::derived_checks::check_keys(&only_mine));
            if lint {
                for c in &s.checks {
                    if generate.contains(&c.as_str()) {
                        checked.insert(c.clone());
                    }
                }
            }
            if s.for_each.is_some() && (s.for_each_kind().is_none() || s.when != "generate") {
                found.push(key_diag(
                    via_diag::codes::protocol::FOR_EACH_FORM,
                    "for_each",
                    format!(
                        "step `{}`: `for_each` takes `kind:<name>` or `given:<name>`, on a generate step",
                        s.id
                    ),
                    "write `for_each = \"kind:<name>\"` on a generate step".into(),
                ));
            }
            // A plain lint or export reads the finished build through the data files; a worker
            // reads what it asks for, so its key holds no data (`graph.md` 5.1).
            let reader = lint || s.when == "export";
            let mut edges: Vec<(String, NodeId)> = if reader && !s.worker {
                data.iter()
                    .map(|p| (format!("data {p}"), write_id(p)))
                    .collect()
            } else {
                Vec::new()
            };
            if lint {
                let mut cs: Vec<&String> = s.checks.iter().collect();
                cs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
                cs.dedup();
                for c in cs {
                    if generate.contains(&c.as_str()) {
                        edges.push((format!("checks {c}"), NodeId::of("step", c)));
                    }
                }
            }
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
            // The step's statement and the frame (`graph.md` 5.1): the world's top-level keys,
            // which hold the extent and resolution, are the `world` statement.
            let statements: Vec<(String, Digest)> = [format!("step.{}", s.id), "world".into()]
                .into_iter()
                .filter_map(|st| model.statements().get(&st).map(|d| (st, *d)))
                .collect();
            let trust = l
                .code
                .as_ref()
                .map_or("none".to_string(), |c| c.trust.hex());
            let mut params = vec![
                ("trust".to_string(), trust),
                ("trusted".to_string(), l.trusted.to_string()),
                (
                    "sdk".to_string(),
                    ctx.sdk_digest.map(Digest::hex).unwrap_or_default(),
                ),
                ("seed".to_string(), ctx.seed.to_string()),
            ];
            // A command of one word, a configured program, is itself the step's code (a library
            // shipped as a program, `protocol.md` 7): its file's digest is in the static key
            // (`graph.md` 5.1), so a revised program runs the step again.
            if let ([program], Some(config)) = (s.command.as_slice(), ctx.config) {
                if !program.starts_with("lib:") && !program.contains('/') {
                    let d = via_proto::program::resolve(program, config)
                        .ok()
                        .and_then(|p| std::fs::read(p).ok())
                        .map_or_else(|| "missing".to_string(), |b| Digest::of(&b).hex());
                    params.push(("program".to_string(), d));
                }
            }
            // A plain command's findings are mapped to entities by id, so the set of ids is in
            // its key. A worker's key is what it read (`graph.md` 5.1): a renamed entity it read
            // changes an answer in its trace, and one it did not read leaves it reused.
            if !s.worker {
                params.push(("entities".to_string(), entities.clone()));
            }
            // A generate step this machine does not trust builds from its committed result
            // (`protocol.md` 7.1): the folder's bytes are in its key, so a new commit is read.
            if !l.trusted && s.when == "generate" {
                if let Some(d) = steps::input_digest(&crate::commit::folder(&s.home, &s.id, None)) {
                    params.push(("gen".to_string(), d.hex()));
                }
            }
            let def = Def {
                subject: Some(via_diag::Subject::entity("step", s.id.clone())),
                statements: statements.clone(),
                params,
                edges,
                // A worker lint's reads are traced, so it runs beside a failure it does not
                // read (`diagnostics.md` 4, M2); a plain one sees the whole build.
                gate: Gate {
                    skip_on_error: !(s.when == "generate" || s.worker && s.when == "lint"),
                    traced: s.worker && s.when == "lint",
                },
                step: Some(StepDef {
                    step: s.id.clone(),
                    command: s.command.clone(),
                    lint,
                }),
                found,
                ..Def::default()
            };
            let kind = s.for_each_kind().filter(|_| s.when == "generate");
            match kind {
                Some(k) if def.found.is_empty() => {
                    // One node per item, and the join that reads them in item order (5.3). The
                    // items are listed when the join is first defined ([`CliPlan::define`]): the
                    // list reads the whole build, which a rank whose run never reaches this step
                    // need not make (M6 close: atlas made the city's whole build for its streets
                    // rank only to list the houses steps' blocks).
                    each.insert(
                        NodeId::of("step", &s.id),
                        Each {
                            item: def,
                            statements,
                            kind: k.to_string(),
                            rank: ctx.world.layer_rank(&s.layer),
                        },
                    );
                }
                _ => {
                    defs.insert(NodeId::of("step", &s.id), def);
                }
            }
        }
        CliPlan {
            model,
            ctx,
            defs,
            each,
            items: Mutex::new(BTreeMap::new()),
            data,
            checked,
            pool: crate::stepwork::Pool::default(),
            build: std::sync::OnceLock::new(),
            leaves: std::sync::OnceLock::new(),
            written: Mutex::new(None),
            last: Mutex::new(BTreeMap::new()),
            chains: Mutex::new(BTreeMap::new()),
            chain_inputs: Mutex::new(None),
            bad: Mutex::new(None),
        }
    }

    /// Shuts the build's workers down; what a slow exit gave (`protocol.md` 6.2).
    pub fn stop_workers(&self) -> Vec<Diag> {
        self.pool.stop()
    }

    /// What failed or was skipped, for a traced lint; `None` for any other step.
    fn bad_for(&self, worker_lint: bool) -> Option<Arc<crate::stepview::Bad>> {
        if !worker_lint {
            return None;
        }
        self.bad
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Every node: the model's, the command line's, and the three defined last.
    pub fn roots(&self) -> Vec<NodeId> {
        let mut r = self.model.ids();
        r.extend(self.defs.keys().cloned());
        r.extend(self.each.keys().cloned());
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
            budget: self.model.budget(),
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
            .filter(|l| l.decl.when == "lint")
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

    /// `authority.unchecked` (`derived.md` 5) for each generate step that ran (or used its
    /// committed result), read a stated fact, returned rows or features, and is checked by no
    /// lint of this build. Said from each step's settled value after the run, never kept with the
    /// step's trace: adding or removing a `checks` lint changes no key of the step, so a
    /// diagnostic stored with the step would replay stale (TODO, "From building quay").
    pub fn unchecked_diags(&self, value: &dyn Fn(&NodeId) -> Option<Vec<u8>>) -> Vec<Diag> {
        let mut out = Vec::new();
        for l in &self.ctx.lints {
            if l.decl.when != "generate" || self.checked.contains(&l.decl.id) {
                continue;
            }
            let Some(bytes) = value(&NodeId::of("step", &l.decl.id)) else {
                continue;
            };
            let Ok(j) = parse(std::str::from_utf8(&bytes).unwrap_or("")) else {
                continue;
            };
            let ran = matches!(j.get("ran"), Some(J::Bool(true)));
            let returns = matches!(j.get("returns"), Some(J::Bool(true)));
            let facts = step_facts(&bytes).unwrap_or_default();
            if ran && returns && !facts.is_empty() {
                out.push(unchecked(&l.decl.id, &facts));
            }
        }
        out
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
                if self.ctx.baseline_record.is_some() {
                    *self
                        .chain_inputs
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((
                        known.settled_all().cloned().unwrap_or_default(),
                        known.step_reads_all().cloned().unwrap_or_default(),
                    ));
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
                // How the baseline was read changes the report's bytes (`graph.md` 6.4): from
                // the `--against` directory's files alone (causes `null`) or from the store,
                // and then by the baseline's build record (the causes and `via` chains), or
                // none found. A files-only report and a store-backed one never share a key.
                let mode = Digest::of(match &self.ctx.baseline {
                    None => b"none".as_slice(),
                    Some(b) if b.files_only => b"files only".as_slice(),
                    Some(_) => b"store".as_slice(),
                });
                // The baseline's own report and manifest nodes are keyed by *its* baseline, and
                // no chain passes through them; left in, a baseline rebuilt with equal text
                // against another baseline would change this key though the report's bytes are
                // the same (`graph.md` 7, toggle).
                let record =
                    self.ctx
                        .baseline_record
                        .as_ref()
                        .map_or(Digest::of(b"no record"), |r| {
                            let mut r = r.clone();
                            for own in [CHANGES, MANIFEST] {
                                r.nodes.remove(&NodeId::parse(own).expect("an id"));
                            }
                            Digest::of(&r.encode())
                        });
                let mut edges = vec![("merge".to_string(), merge)];
                edges.extend(data_edges);
                Defined::Def(Def {
                    params: vec![("path".into(), "changes.json".into())],
                    edges,
                    direct: vec![
                        ("baseline".into(), base),
                        ("baseline mode".into(), mode),
                        ("baseline record".into(), record),
                    ],
                    gate: Gate {
                        skip_on_error: true,
                        traced: false,
                    },
                    ..Def::default()
                })
            }
            MANIFEST => {
                let mut edges = data_edges;
                // What each export step wrote is listed (`build.md` 11.4).
                for l in self.ctx.lints.iter().filter(|l| l.decl.when == "export") {
                    edges.push((
                        format!("export {}", l.decl.id),
                        NodeId::of("step", &l.decl.id),
                    ));
                }
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
                        (
                            "pack roots".into(),
                            self.ctx
                                .pack_roots
                                .iter()
                                .map(|(k, v)| format!("{k}={v}"))
                                .collect::<Vec<_>>()
                                .join(" "),
                        ),
                    ],
                    edges,
                    gate: Gate {
                        skip_on_error: true,
                        traced: false,
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

    /// The chain of each cause of each changed thing (`graph.md` 6.4, `build.md` 11.2): the
    /// shortest edge path, over the build's real node edges, from the node made from each changed
    /// statement to the thing's own node: `shape:` or `attr:` for a feature (the shorter, ties by
    /// node id), `instance:` (a pin's `edit:` or `height:`) or the maker's `held:` for an instance, `footprint:` for a
    /// footprint, and for a field its first chunk in chunk order whose value differs from the
    /// baseline's. A statement with no path in this build's graph takes its path in the
    /// baseline's, from the baseline's record. Keyed by [`changes::chain_key`].
    fn find_chains(
        &self,
        prev: &BuildRecord,
        value: &dyn Fn(&NodeId) -> Option<Digest>,
        step_reads: &dyn Fn(&NodeId) -> Option<Vec<via_graph::Read>>,
    ) {
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
        self.step_chain_nodes(&mut made_from, &mut edges, step_reads);
        let prev_edges = prev.edges();
        let prev_made_from = prev.made_from();
        let now = via_graph::CauseGraph::new(self.model.statements(), &made_from, &edges, prev);
        let before =
            via_graph::CauseGraph::new(self.model.statements(), &prev_made_from, &prev_edges, prev);
        let chains_to = |targets: &[NodeId]| -> BTreeMap<String, Vec<String>> {
            let mut m: BTreeMap<String, (usize, NodeId, Vec<String>)> = BTreeMap::new();
            let mut ts: Vec<&NodeId> = targets
                .iter()
                .filter(|t| self.model.def(t).is_some())
                .collect();
            ts.sort();
            for t in ts {
                for c in now.causes(t) {
                    let path: Vec<String> = c.path.iter().map(ToString::to_string).collect();
                    let better = m.get(&c.statement).is_none_or(|(n, _, _)| path.len() < *n);
                    if better {
                        m.insert(c.statement, (path.len(), t.clone(), path));
                    }
                }
            }
            // A statement with no path in this build's graph: the dependency was only in the
            // baseline's (a change undone), so its chain is in the baseline's graph (6.4).
            let mut ts: Vec<&NodeId> = targets
                .iter()
                .filter(|t| prev.nodes.contains_key(*t))
                .collect();
            ts.sort();
            let mut old: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
            for t in ts {
                for c in before.causes(t) {
                    if m.contains_key(&c.statement) {
                        continue;
                    }
                    let path: Vec<String> = c.path.iter().map(ToString::to_string).collect();
                    if old.get(&c.statement).is_none_or(|(n, _)| path.len() < *n) {
                        old.insert(c.statement, (path.len(), path));
                    }
                }
            }
            m.into_iter()
                .map(|(s, (_, _, p))| (s, p))
                .chain(old.into_iter().map(|(s, (_, p))| (s, p)))
                .collect()
        };
        let mut chains: Chains = BTreeMap::new();
        for (name, _) in self.model.field_kinds() {
            // In chunk order (`build.md` 5): rows of chunks, `cy` then `cx`.
            let mut chunks: Vec<NodeId> = self
                .model
                .file_reads(&FileReads::Field(name.clone()))
                .into_iter()
                .map(|(_, chunk)| chunk)
                .collect();
            chunks.sort_by_key(|c| {
                let w: Vec<u64> = c
                    .as_str()
                    .rsplit_once("/w:")
                    .map(|(_, w)| w.split('.').filter_map(|x| x.parse().ok()).collect())
                    .unwrap_or_default();
                (w.get(1).copied(), w.first().copied())
            });
            let first = chunks
                .into_iter()
                .find(|chunk| value(chunk) != prev.nodes.get(chunk).and_then(|n| n.value));
            if let Some(chunk) = first {
                chains.insert(changes::chain_key("field", &name), chains_to(&[chunk]));
            }
        }
        let build = self.model.build();
        let base = self.ctx.baseline.as_ref().map(|b| &b.snapshot.build);
        let mut memo: BTreeMap<Vec<NodeId>, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        if let Some(report) = &build.changes {
            for e in &report.entries {
                let v = |f: &str, n: &str| NodeId::of(f, n).view("build");
                let targets: Vec<NodeId> = match e.table.as_str() {
                    "feature" => vec![v("shape", &e.name), NodeId::of("attr", &e.name)],
                    "footprint" => vec![v("footprint", &e.name)],
                    "instance" => {
                        // A hand instance's own `instance:` node; a pin's is its `edit:` node,
                        // or its `height:` (`build.md` 11.2, `graph.md` 1). `height:` is a
                        // target only for a pin: a hand instance's chain ends at `instance:`.
                        let pin = self.model.def(&v("edit", &e.name)).is_some();
                        let families: &[&str] = if pin {
                            &["instance", "edit", "height"]
                        } else {
                            &["instance"]
                        };
                        // A set's item `<instance>.<item>` has its placing instance's nodes.
                        let mut own: Vec<NodeId> = families
                            .iter()
                            .map(|f| v(f, &e.name))
                            .filter(|n| self.model.def(n).is_some())
                            .collect();
                        if own.is_empty() {
                            if let Some((h, _)) = e.name.split_once('.') {
                                own.extend(
                                    [v("instance", h)]
                                        .into_iter()
                                        .filter(|n| self.model.def(n).is_some()),
                                );
                            }
                        }
                        if !own.is_empty() {
                            own
                        } else {
                            build
                                .instances
                                .get(&e.name)
                                .or_else(|| base.and_then(|b| b.instances.get(&e.name)))
                                .and_then(|i| i.maker.clone())
                                .map(|m| vec![v("held", &m)])
                                .unwrap_or_default()
                        }
                    }
                    _ => continue,
                };
                // A thing brought from an import (`a.hut`) has no node of its own in the
                // parent: its chain ends at the import as placed (`composition.md` 11, 12).
                let targets = if targets.iter().all(|n| self.model.def(n).is_none()) {
                    e.name
                        .split_once('.')
                        .map(|(h, _)| NodeId::of("placed", h))
                        .filter(|n| self.model.def(n).is_some())
                        .map_or(targets, |n| vec![n])
                } else {
                    targets
                };
                // Every instance of one producer has the same targets, its `held:` node: the
                // chains are worked out once per set of targets (a scatter of thousands is one).
                let mut found = memo
                    .entry(targets.clone())
                    .or_insert_with(|| chains_to(&targets))
                    .clone();
                // A hand instance stands on the ground: what moved it only up or down reaches it
                // through its `height:` node, which a statement not found above is chained to.
                if e.table == "instance" {
                    let head = match e.name.split_once('.') {
                        Some((h, _)) if self.model.def(&v("height", &e.name)).is_none() => h,
                        _ => e.name.as_str(),
                    };
                    let h = v("height", head);
                    if self.model.def(&h).is_some() && !targets.contains(&h) {
                        let more = memo
                            .entry(vec![h.clone()])
                            .or_insert_with(|| chains_to(&[h]))
                            .clone();
                        for (st, path) in more {
                            found.entry(st).or_insert(path);
                        }
                    }
                }
                chains.insert(changes::chain_key(&e.table, &e.name), found);
            }
        }
        *self
            .chains
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = chains;
    }

    /// The steps as the chains see them (`graph.md` 5.1 and 6.4): each step node made from its
    /// own statement and the frame, with an edge to what each read of its trace named (a field's
    /// chunks, a feature's shapes) and made from those things' statements; and every node made
    /// from a statement the step returned (`<table>.<step>#...`) with an edge to the step.
    fn step_chain_nodes(
        &self,
        made_from: &mut BTreeMap<NodeId, Vec<String>>,
        edges: &mut BTreeMap<NodeId, Vec<(String, NodeId)>>,
        step_reads: &dyn Fn(&NodeId) -> Option<Vec<via_graph::Read>>,
    ) {
        let build = self.model.build();
        let stmts = self.model.statements();
        // Model nodes by family and subject, for a read that names a thing.
        let mut by_subject: BTreeMap<(String, String), Vec<NodeId>> = BTreeMap::new();
        for id in self.model.ids() {
            let s = id.as_str();
            let (fam, rest) = s.split_once(':').unwrap_or((s, ""));
            let subject = rest.split('/').next().unwrap_or(rest);
            by_subject
                .entry((fam.to_string(), subject.to_string()))
                .or_default()
                .push(id.clone());
        }
        let fields_of = |sel: &str| -> Vec<String> {
            if let Some(r) = sel.strip_prefix("role:") {
                return build
                    .fields
                    .iter()
                    .filter(|(_, f)| f.role.as_deref() == Some(r))
                    .map(|(n, _)| n.clone())
                    .collect();
            }
            if let Some(t) = sel.strip_prefix("tag:") {
                return build
                    .fields
                    .iter()
                    .filter(|(_, f)| f.tags.iter().any(|x| x == t))
                    .map(|(n, _)| n.clone())
                    .collect();
            }
            if build.fields.contains_key(sel) {
                vec![sel.to_string()]
            } else {
                Vec::new()
            }
        };
        let features_of = |sel: &str| -> Vec<String> {
            if sel == "*" {
                return build.features.keys().cloned().collect();
            }
            if let Some(k) = sel.strip_prefix("kind:") {
                return build
                    .features
                    .iter()
                    .filter(|(_, f)| f.kind.as_deref() == Some(k))
                    .map(|(n, _)| n.clone())
                    .collect();
            }
            let prefix = format!("{sel}#");
            build
                .features
                .keys()
                .filter(|n| *n == sel || n.starts_with(&prefix))
                .cloned()
                .collect()
        };
        let strings = |v: Option<&serde_json::Value>| -> Vec<String> {
            match v {
                Some(serde_json::Value::String(s)) => vec![s.clone()],
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect(),
                Some(_) => Vec::new(),
                None => vec!["*".to_string()],
            }
        };
        let mut step_ids: Vec<String> = self.ctx.lints.iter().map(|l| l.decl.id.clone()).collect();
        step_ids.sort();
        step_ids.dedup();
        for sid in &step_ids {
            let node = NodeId::of("step", sid);
            let mut from: Vec<String> = [format!("step.{sid}"), "world".to_string()]
                .into_iter()
                .filter(|st| stmts.contains_key(st))
                .collect();
            let mut out: Vec<(String, NodeId)> = Vec::new();
            for r in step_reads(&node).unwrap_or_default() {
                let Some(bytes) = self.ctx.store.get_object(&r.request) else {
                    continue;
                };
                let Ok(req) =
                    serde_json::from_slice::<serde_json::Value>(&via_graph::object_bytes(&bytes))
                        .or_else(|_| serde_json::from_slice::<serde_json::Value>(&bytes))
                else {
                    continue;
                };
                match r.method.as_str() {
                    "build.field" => {
                        for sel in strings(req.get("fields")) {
                            for f in fields_of(&sel) {
                                from.push(format!("field.{f}"));
                                out.extend(
                                    self.model
                                        .file_reads(&FileReads::Field(f.clone()))
                                        .into_iter()
                                        .map(|(_, c)| (format!("read field {f}"), c)),
                                );
                            }
                        }
                    }
                    "build.features" => {
                        for sel in strings(req.get("select")) {
                            for f in features_of(&sel) {
                                from.push(format!("feature.{f}"));
                                for fam in ["shape", "attr"] {
                                    for n in by_subject
                                        .get(&(fam.to_string(), f.clone()))
                                        .into_iter()
                                        .flatten()
                                    {
                                        out.push((format!("read feature {f}"), n.clone()));
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            from.retain(|st| stmts.contains_key(st));
            from.sort();
            from.dedup();
            out.sort();
            out.dedup();
            made_from.insert(node.clone(), from);
            edges.insert(node, out);
        }
        // What a step returned is made from its document: each such node reads the step.
        let ids: Vec<NodeId> = made_from.keys().cloned().collect();
        for id in ids {
            let returned: Vec<String> = made_from[&id]
                .iter()
                .filter_map(|st| {
                    let (_, rest) = st.split_once('.')?;
                    let (step, _) = rest.split_once('#')?;
                    step_ids
                        .contains(&step.to_string())
                        .then(|| step.to_string())
                })
                .collect();
            for step in returned {
                let target = NodeId::of("step", &step);
                if target != id {
                    edges
                        .entry(id.clone())
                        .or_default()
                        .push((format!("returned by {step}"), target));
                }
            }
        }
    }

    fn compute_changes(&self) -> Output {
        let Some(b) = &self.ctx.baseline else {
            let bytes = changes::without_baseline().file();
            self.stash("changes.json", bytes.clone());
            return Output::of(Val::plain(bytes));
        };
        let build = self.model.build();
        if b.files_only {
            // Both sides read back from their files (`build.md` 11.2, `graph.md` 6.4).
            let bytes = match self.files().as_ref() {
                Ok(w) => {
                    let head = J::Obj(w.head.clone());
                    match crate::filebase::snapshot(&|p| w.files.get(p).cloned(), &head) {
                        Some(new) => {
                            let report =
                                via_model::report::compare(Some(&b.snapshot.build), &new.build);
                            changes::write(
                                &report,
                                &Side {
                                    build: &new.build,
                                    lines: &new.lines,
                                    homes: &new.homes,
                                },
                                b.snapshot,
                                (b.id.clone(), b.key.clone()),
                                &BTreeMap::new(),
                                false,
                            )
                            .file()
                        }
                        None => changes::without_baseline().file(),
                    }
                }
                Err(_) => changes::without_baseline().file(),
            };
            self.stash("changes.json", bytes.clone());
            return Output::of(Val::plain(bytes));
        }
        let kept = self
            .chain_inputs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let (Some(prev), Some((settled, reads))) = (&self.ctx.baseline_record, kept) {
            self.find_chains(prev, &|n| settled.get(n).and_then(|s| s.value), &|n| {
                reads.get(n).cloned()
            });
        }
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
                    true,
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
            } else if !(v.bytes.is_empty()
                && (path.starts_with("weights/") || path.starts_with("meshes/")))
            {
                files.insert(path, v.bytes.as_ref().clone());
            }
        }
        let value = |n: &NodeId| {
            inputs
                .all()
                .iter()
                .find(|(_, node, _)| node == n)
                .map(|(_, _, v)| v.bytes.as_ref().clone())
        };
        let mut head = head;
        match self.export_files(&value) {
            Ok((written, exports)) => {
                files.extend(written);
                if !exports.is_empty() {
                    head.insert("exports".into(), J::Obj(exports));
                }
            }
            Err(d) => {
                return Output::of(Val {
                    bytes: Arc::new(Vec::new()),
                    diags: Arc::new(vec![d]),
                })
            }
        }
        let bytes = manifest_bytes(head, &files, &reports, &self.ctx);
        self.stash("manifest.json", bytes.clone());
        Output::of(Val::plain(bytes))
    }

    /// What the export steps wrote and the manifest's `exports` (`build.md` 11.4): for each
    /// export step that ran, every file its value lists but `diagnostics.json`, under
    /// `export/<step id>/`, read from the store by digest. A step that did not run (untrusted)
    /// has no entry: a reader finds nothing it would look for. An object the store no longer
    /// holds is `build.internal`: the step's value says it was written.
    pub fn export_files(
        &self,
        value: &dyn Fn(&NodeId) -> Option<Vec<u8>>,
    ) -> Result<Exported, Diag> {
        let mut files = BTreeMap::new();
        let mut exports = BTreeMap::new();
        for l in self.ctx.lints.iter().filter(|l| l.decl.when == "export") {
            let sid = &l.decl.id;
            let Some(bytes) = value(&NodeId::of("step", sid)) else {
                continue;
            };
            let Ok(j) = parse(std::str::from_utf8(&bytes).unwrap_or("")) else {
                continue;
            };
            if !matches!(j.get("ran"), Some(J::Bool(true))) {
                continue;
            }
            let folder = format!("export/{sid}/");
            for f in j.get("files").and_then(J::as_arr).unwrap_or(&[]) {
                let (Some(path), Some(d)) = (
                    f.get("path").and_then(J::as_str),
                    f.get("digest")
                        .and_then(J::as_str)
                        .and_then(Digest::from_hex),
                ) else {
                    continue;
                };
                if path == "diagnostics.json" {
                    continue;
                }
                let Some(b) = self.ctx.store.get_object(&d) else {
                    return Err(Diag::new(
                        via_diag::codes::build::INTERNAL,
                        via_diag::Subject::entity("step", sid.clone()),
                        via_diag::Site::World,
                        crate::txt(format!(
                            "export step `{sid}` wrote `{path}`, which the store no longer holds"
                        )),
                        crate::txt("build again with `--rerun-step` naming the step".to_string()),
                    ));
                };
                // The escape of 3.4: a device name is written with `-` after it.
                let escaped: Vec<String> = path.split('/').map(crate::out::segment).collect();
                files.insert(format!("{folder}{}", escaped.join("/")), b);
            }
            exports.insert(
                sid.clone(),
                obj([
                    ("folder", J::s(folder)),
                    ("output", J::opt(l.decl.output.as_deref())),
                ]),
            );
        }
        Ok((files, exports))
    }

    /// A `for_each` step's join (`graph.md` 5.3): its items listed from the build, each item's
    /// node kept for [`Plan::define`], and the join that reads them in item order.
    fn join_def(&self, id: &NodeId, e: &Each) -> Def {
        let (sid, _) = split_step(id);
        // Kept under the model's leaves (`graph.md` 5.3), so a build that reuses every step lists
        // them without making the model's build: atlas's unchanged build into another folder
        // spent most of its time outside any node making it (TODO, "From M6 close").
        let memo = self.items_memo(&NodeId::of("step", &sid), e);
        let kept = self
            .memo_get(&memo)
            .and_then(|d| self.ctx.store.get_object(&d))
            .and_then(|b| decode_items(&b));
        let list = kept.unwrap_or_else(|| {
            let list = crate::stepview::items(self.ctx.world, &self.the_build(), e.rank, &e.kind);
            let d = self.ctx.store.put_object(&encode_items(&list));
            self.memo_put(&memo, d);
            list
        });
        let mut join_edges = Vec::new();
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for item in &list {
            let iid = NodeId::of("step", &sid).item(item);
            let mut d = e.item.clone();
            d.params.push(("item".into(), item.clone()));
            if let Some(sd) = self.model.statements().get(&format!("feature.{item}")) {
                d.statements.push((format!("feature.{item}"), *sd));
            }
            join_edges.push((format!("item {item}"), iid.clone()));
            items.insert(iid, d);
        }
        Def {
            subject: Some(via_diag::Subject::entity("step", sid.clone())),
            statements: e.statements.clone(),
            params: vec![
                ("join".into(), "true".into()),
                ("for_each".into(), e.kind.clone()),
            ],
            edges: join_edges,
            ..Def::default()
        }
    }

    /// Makes the model's build ahead of the first node that asks for it, on the caller's thread
    /// (`graph.md` 8): run beside the graph, it overlaps the lookups that come before any step.
    pub fn warm(&self) {
        let _ = self.the_build();
    }

    /// True when a `for_each` step among `steps` has no items kept under this graph's leaves, so
    /// the graph will make the model's build to list them.
    pub fn lists_items(&self, steps: &[&LintStep]) -> bool {
        steps.iter().any(|l| {
            let id = NodeId::of("step", &l.decl.id);
            self.each
                .get(&id)
                .is_some_and(|e| self.memo_get(&self.items_memo(&id, e)).is_none())
        })
    }

    /// The memo key of a `for_each` step's items: they are a function of the model's leaves.
    fn items_memo(&self, id: &NodeId, e: &Each) -> Digest {
        let rank = e.rank.map_or(String::new(), |r| r.to_string());
        self.memo_key("items", id, None, &[&e.kind, &rank])
    }

    /// The model's build, made once.
    fn the_build(&self) -> Arc<via_model::Build> {
        self.build
            .get_or_init(|| Arc::new(self.model.build()))
            .clone()
    }

    /// The digest of everything the model's build is made from (`graph.md` 5.1, Reuse): the
    /// build key (versions, sources, declared inputs, trust, lookups and flags) and every leaf
    /// of this graph's model, the text of each source and the bytes of each input. The joined
    /// world of a stratum differs from the world the build key names; its leaves say how.
    fn leaves_digest(&self) -> Digest {
        *self.leaves.get_or_init(|| {
            let mut e = via_graph::Enc::new();
            e.str("via-leaves-1").str(&self.ctx.key.hex());
            let mut ids: Vec<NodeId> = self
                .model
                .ids()
                .into_iter()
                .chain(self.defs.keys().cloned())
                .filter(|i| matches!(i.family(), "source" | "input"))
                .collect();
            ids.sort();
            ids.dedup();
            e.len(ids.len());
            for id in &ids {
                let def = self.model.def(id).or_else(|| self.defs.get(id));
                e.str(id.as_str());
                let direct = def.map_or(&[][..], |d| d.direct.as_slice());
                e.len(direct.len());
                for (k, v) in direct {
                    e.str(k).str(&v.hex());
                }
            }
            Digest::of(&e.finish())
        })
    }

    /// The store key of a memo of a step's read or request (`graph.md` 5.1, Reuse): what it is
    /// a function of, so an equal key holds an equal digest and the build is not made to find it.
    fn memo_key(
        &self,
        what: &str,
        step: &NodeId,
        bad: Option<&crate::stepview::Bad>,
        rest: &[&str],
    ) -> Digest {
        let mut e = via_graph::Enc::new();
        e.str("via-memo-1")
            .str(what)
            .str(&self.leaves_digest().hex())
            .str(step.as_str());
        match bad {
            None => {
                e.byte(0);
            }
            Some(b) => {
                e.byte(1);
                for set in [&b.fields, &b.features, &b.steps] {
                    e.len(set.len());
                    for x in set {
                        e.str(x);
                    }
                }
                e.byte(u8::from(b.instances));
            }
        }
        e.len(rest.len());
        for r in rest {
            e.str(r);
        }
        Digest::of(&e.finish())
    }

    /// The memo key of one read of a step's trace.
    fn answer_memo(
        &self,
        step: &NodeId,
        bad: Option<&crate::stepview::Bad>,
        read: &via_graph::Read,
    ) -> Digest {
        self.memo_key("answer", step, bad, &[&read.method, &read.request.hex()])
    }

    fn memo_get(&self, key: &Digest) -> Option<Digest> {
        self.ctx.store.get_key(key).map(|r| r.value)
    }

    fn memo_put(&self, key: &Digest, value: Digest) {
        self.ctx.store.put_key(
            key,
            &via_graph::KeyRecord {
                value,
                diags: Vec::new(),
            },
        );
    }

    /// What a step call is handed beyond the step.
    fn call<'b>(
        &'b self,
        build: &'b via_model::Build,
        item: Option<&'b str>,
        checks: J,
        bad: Option<&'b crate::stepview::Bad>,
    ) -> steps::Call<'b> {
        steps::Call {
            world: self.ctx.world,
            build,
            store: self.ctx.store,
            roots: self.ctx.roots,
            config: self.ctx.config,
            sdk: self.ctx.sdk,
            sdk_digest: self.ctx.sdk_digest,
            seed: self.ctx.seed,
            deadline: self.ctx.deadline,
            pack: self.ctx.pack,
            trust_command: &self.ctx.trust_command,
            pool: Some(&self.pool),
            item,
            checks,
            stratum: Some(self.model),
            bad,
        }
    }

    /// A lint's definition with its request in the static key (`graph.md` 5.1): for a plain
    /// command, the request with its absolute paths left out, made before the lookup. It holds
    /// what no data file does (each field's role, tags and unit; the instances' asset, taxon and
    /// `by`), so a change to any of it runs the lint again. A lint that will not be started
    /// (untrusted, or its code not found) is handed nothing, and its key says why already.
    fn step_def(&self, id: &NodeId, def: &Def) -> Def {
        let mut def = def.clone();
        if def.step.is_none() {
            return def;
        }
        let (sid, item) = split_step(id);
        let Some(l) = self.ctx.lints.iter().find(|l| l.decl.id == sid) else {
            return def;
        };
        if l.decl.worker {
            // A worker's key holds no request: what it reads is its trace (`graph.md` 5.1).
            return def;
        }
        // A plain generate step with a committed result is held to the request a trusted build
        // makes (`protocol.md` 7.1), so it is made too.
        let committed = !l.trusted
            && l.decl.when == "generate"
            && crate::commit::folder(&l.decl.home, &sid, None).is_dir();
        let request = if (l.trusted || committed) && l.code.is_ok() {
            // The request is a function of the model's leaves and the step (its statement is a
            // source): kept under them, so a build with the same leaves does not make the build.
            let memo = self.memo_key("request", id, None, &[]);
            self.memo_get(&memo).unwrap_or_else(|| {
                let build = self.the_build();
                let r =
                    steps::request_key(&l.decl, &self.call(&build, item.as_deref(), J::Null, None));
                self.memo_put(&memo, r);
                r
            })
        } else {
            Digest::of(b"via-request-1 not started")
        };
        def.direct.push(("request".into(), request));
        def
    }

    /// A lint's `checks` member (`derived.md` 5): per named step, sorted by id, the stated facts
    /// that step read, from its value.
    fn checks_member(&self, s: &steps::StepDecl, inputs: &Inputs<'_>) -> J {
        if s.checks.is_empty() {
            return J::Null;
        }
        let facts: BTreeMap<String, BTreeSet<String>> = s
            .checks
            .iter()
            .filter_map(|g| {
                let f = inputs
                    .get(&format!("checks {g}"))
                    .and_then(|v| step_facts(&v.bytes))?;
                Some((g.clone(), f.into_iter().collect()))
            })
            .collect();
        let info = via_model::derived_checks::StepInfo {
            id: s.id.clone(),
            when: s.when.clone(),
            checks: Some(s.checks.clone()),
        };
        J::Arr(
            via_model::derived_checks::checks_member(&info, &facts)
                .into_iter()
                .map(|c| {
                    obj([
                        ("facts", J::strs(c.facts.iter().map(String::as_str))),
                        ("step", J::s(c.step)),
                    ])
                })
                .collect(),
        )
    }

    /// The join of a `for_each` step (5.3): every item's call in item order; the keys two items
    /// both returned are `identity.key_collision`; none matched is `graph.for_each_empty`.
    fn compute_join(&self, id: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output {
        let (sid, _) = split_step(id);
        let mut diags: Vec<Diag> = Vec::new();
        let mut explains: Vec<(String, Option<String>)> = Vec::new();
        let mut facts: BTreeSet<String> = BTreeSet::new();
        let mut owners: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        let mut ran = false;
        let mut unjoined = false;
        let mut returns = false;
        for (label, _, v) in inputs.all() {
            let item = label.trim_start_matches("item ").to_string();
            let Ok(j) = parse(std::str::from_utf8(&v.bytes).unwrap_or("")) else {
                continue;
            };
            ran |= matches!(j.get("ran"), Some(J::Bool(true)));
            unjoined |= matches!(j.get("unjoined"), Some(J::Bool(true)));
            returns |= matches!(j.get("returns"), Some(J::Bool(true)));
            if let Some((_, e)) = step_value(&v.bytes) {
                explains.extend(e);
            }
            facts.extend(step_facts(&v.bytes).unwrap_or_default());
            if let Some(J::Obj(keys)) = j.get("keys") {
                for (table, ks) in keys {
                    for k in ks.as_arr().unwrap_or(&[]) {
                        if let Some(k) = k.as_str() {
                            owners
                                .entry((table.clone(), k.to_string()))
                                .or_default()
                                .push(item.clone());
                        }
                    }
                }
            }
        }
        for ((table, key), who) in &owners {
            if who.len() > 1 {
                diags.push(Diag::new(
                    via_diag::codes::identity::KEY_COLLISION,
                    via_diag::Subject::entity("step", sid.clone()),
                    via_diag::Site::Header,
                    crate::txt(format!(
                        "step `{sid}`: the items `{}` and `{}` both returned `[{table}.{key}]`",
                        who[0], who[1]
                    )),
                    crate::txt("build each returned key from the item's name, so keys differ between items".into()),
                ));
            }
        }
        let _ = unjoined;
        // `derived.md` 5, over the items (the union of the facts they read), is said after the
        // run from this value's `facts` and `returns` ([`CliPlan::unchecked_diags`]).
        if inputs.all().is_empty() {
            let kind = def
                .params
                .iter()
                .find(|(k, _)| k == "for_each")
                .map_or("", |(_, v)| v.as_str());
            // `given:<name>` is written whole; a kind is written after `kind:`.
            let written = if kind.starts_with("given:") {
                kind.to_string()
            } else {
                format!("kind:{kind}")
            };
            diags.push(Diag::new(
                via_diag::codes::graph::FOR_EACH_EMPTY,
                via_diag::Subject::entity("step", sid.clone()),
                via_diag::Site::Key(via_diag::KeyPath::key("for_each")),
                crate::txt(format!(
                    "step `{sid}`: `for_each = \"{written}\"` matches no feature in its view; the step did not run"
                )),
                crate::txt(match kind.strip_prefix("given:") {
                    Some(g) => format!("give `{g}` a feature, or remove the step"),
                    None => format!("add a feature of kind `{kind}` at or below the step's layer, or remove the step"),
                }),
            ));
        }
        let facts: Vec<String> = facts.into_iter().collect();
        Output::of(Val {
            bytes: Arc::new(
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
                    ("facts", J::strs(facts.iter().map(String::as_str))),
                    ("files", J::Arr(Vec::new())),
                    ("ran", J::Bool(ran)),
                    ("returns", J::Bool(returns)),
                ])
                .file(),
            ),
            diags: Arc::new(diags),
        })
    }

    /// An untrusted generate step's committed result (`protocol.md` 7.1): `None` when it has
    /// none (the step is `protocol.untrusted`); else the result as a completed call when its
    /// static key and every read of its trace match this build, answered from the current graph
    /// as reuse is, with `protocol.untrusted_result`; else `protocol.untrusted_stale`, naming the
    /// first difference, not completed. Nothing runs.
    fn committed_result(
        &self,
        id: &NodeId,
        l: &LintStep,
        code: &steps::Code,
        item: Option<&str>,
        key_parts: &BTreeMap<String, String>,
        build: &via_model::Build,
    ) -> Option<steps::Ran> {
        let sid = &l.decl.id;
        let dir = crate::commit::folder(&l.decl.home, sid, item);
        let read = crate::commit::Committed::read(&dir)?;
        let mut ran = steps::Ran::new();
        let label = crate::commit::label(sid, item);
        let stale = |why: &str| crate::commit::stale(sid, &label, why, &self.ctx.trust_command);
        let c = match read {
            Ok(c) => c,
            Err(why) => {
                ran.diags.push(stale(&why));
                return Some(ran);
            }
        };
        if let Some(why) = c.key_difference(&code.trust, key_parts) {
            ran.diags.push(stale(&why));
            return Some(ran);
        }
        // The blobs the trace's requests name, into this machine's store (`protocol.md` 7.1).
        let restored = via_proto::store::Store::open(self.ctx.store.root().join("objects"))
            .map_err(|_| "the store's objects cannot be opened".to_string())
            .and_then(|objects| crate::commit::restore_blobs(&dir, &objects));
        if let Err(why) = restored {
            ran.diags.push(stale(&why));
            return Some(ran);
        }
        let mut reads = Vec::new();
        for (method, request, answer) in c.trace() {
            let req = self.ctx.store.put_object(request.canonical().as_bytes());
            let rd = via_graph::Read {
                method: method.clone(),
                request: req,
                answer: answer.unwrap_or(Digest::of(b"\0no answer")),
            };
            if answer.is_none() || self.answer(id, &rd) != answer {
                ran.diags.push(stale(&format!(
                    "the read `{method}` of request {} gives another answer now",
                    req.hex()
                )));
                return Some(ran);
            }
            reads.push(rd);
        }
        let grid = build.grid?;
        let got = crate::stepout::collect(
            &dir,
            sid,
            "generate",
            None,
            &grid,
            &crate::stepout::Names {
                world: self.ctx.world,
                rank: self.ctx.world.layer_rank(&l.decl.layer),
            },
            self.ctx.store,
        );
        let files: Vec<(String, Digest)> = got
            .files
            .iter()
            .filter(|(p, _)| {
                p != "result.json" && !p.starts_with(&format!("{}/", crate::commit::BLOBS))
            })
            .cloned()
            .collect();
        let want = c.files();
        let have: BTreeMap<String, Option<Digest>> = files
            .iter()
            .filter(|(p, _)| p != "document.toml")
            .map(|(p, d)| (p.clone(), Some(*d)))
            .collect();
        if want != have {
            ran.diags
                .push(stale("its files differ from those `result.json` lists"));
            return Some(ran);
        }
        // What the step raised itself when it ran, and its explain texts, replayed: the build
        // uses the result exactly as a trusted build does, plus `protocol.untrusted_result`.
        let Some(emitted) = c.diags() else {
            ran.diags
                .push(stale("its `diags` cannot be read as the build's records"));
            return Some(ran);
        };
        ran.diags.extend(emitted.iter().cloned());
        ran.emitted = emitted;
        ran.explains = c.explains();
        ran.diags.extend(got.diags.iter().cloned());
        ran.diags.push(crate::commit::used(sid, &label));
        ran.rows_or_features = got.returns_rows_or_features();
        ran.keys = got.keys.clone();
        ran.unjoined = got.ok && got.has_tables();
        ran.document = got.document.clone();
        ran.facts = c.facts();
        ran.reads = reads;
        ran.complete = got.ok;
        ran.record = obj([(
            "returned",
            J::Arr(
                files
                    .iter()
                    .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
                    .collect(),
            ),
        )]);
        Some(ran)
    }

    fn compute_step(&self, id: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output {
        let (sid, item) = split_step(id);
        let Some(l) = self.ctx.lints.iter().find(|l| l.decl.id == sid) else {
            return Output::of(Val::plain(Vec::new()));
        };
        let code = match &l.code {
            Ok(c) => c,
            Err(d) => {
                // Not started (an unknown library, an unreadable code folder): not complete,
                // so not stored, and it is tried again next build (`graph.md` 3.4).
                return Output {
                    value: Val {
                        bytes: Arc::new(step_bytes(false, &[], &[])),
                        diags: Arc::new(vec![d.clone()]),
                    },
                    complete: false,
                    step: Some(StepRun::default()),
                    skipped: false,
                };
            }
        };
        let build = self.the_build();
        let checks = self.checks_member(&l.decl, inputs);
        let bad = self.bad_for(l.decl.worker && l.decl.when == "lint");
        let key_parts = crate::commit::key_parts(def, inputs);
        // An untrusted generate step with a committed result (`protocol.md` 7.1).
        let committed = if !l.trusted && l.decl.when == "generate" {
            self.committed_result(id, l, code, item.as_deref(), &key_parts, &build)
        } else {
            None
        };
        let from_gen = committed.as_ref().is_some_and(|r| r.complete);
        let r = match committed {
            Some(r) => r,
            None => steps::run_step(
                &l.decl,
                code,
                l.trusted,
                &self.call(&build, item.as_deref(), checks, bad.as_deref()),
            ),
        };
        if r.skipped {
            return Output {
                value: Val::plain(Vec::new()),
                complete: false,
                step: Some(StepRun::default()),
                skipped: true,
            };
        }
        // `derived.md` 5 (`authority.unchecked`) is said after the run, from the step's value
        // ([`CliPlan::unchecked_diags`]): whether a lint checks the step is in no key of the
        // step, so a diagnostic kept with its trace would replay after a lint was added.
        // The returned document is kept in the store, so the build joins it to the step's layer
        // from the step's value (`protocol.md` 5, Its layer).
        if let Some(d) = &r.document {
            self.ctx.store.put_object(d.as_bytes());
        }
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
        // Complete (`graph.md` 3.4): a call that ran, exited 0 and returned readable output; or
        // the deliberate skip of an untrusted lint, whose key holds `trusted=false`; or a missing
        // declared input, whose digest is in the key. Anything else that did not start (no
        // program, a spawn that failed, a call folder that could not be written) is not stored:
        // what kept it from starting is in no key, so it is tried again next build.
        let input_missing = !r.ran
            && !r.diags.is_empty()
            && r.diags
                .iter()
                .all(|d| *d.code() == DiagCode::Core(via_diag::codes::graph::INPUT_MISSING));
        let complete = if r.ran || from_gen {
            r.complete
        } else {
            (!l.trusted && l.decl.when != "generate") || input_missing
        };
        // Each read a completed run made, as the memo a trace check finds (`graph.md` 5.1,
        // Reuse): the answer the run was given is the one a check computes again.
        if complete && r.ran && !from_gen {
            for rd in &r.reads {
                self.memo_put(&self.answer_memo(id, bad.as_deref(), rd), rd.answer);
            }
        }
        // A plain command's one read, method `data`: its request without absolute paths, which
        // is also all it is answered.
        let data = r.data.map(|d| via_graph::Read {
            method: "data".into(),
            request: d,
            answer: d,
        });
        let mut value = parse(
            std::str::from_utf8(&step_bytes(r.ran || from_gen, &r.explains, &files))
                .unwrap_or("{}"),
        )
        .unwrap_or(J::Null);
        if let J::Obj(m) = &mut value {
            if l.decl.when == "generate" {
                m.insert("facts".into(), J::strs(r.facts.iter().map(String::as_str)));
                // Each fact's statement as read: a moved fact changes this value, so a lint that
                // `checks` the step runs again even when the step returns the same document
                // (`derived.md` 5: the fact is checked again).
                m.insert(
                    "fact_statements".into(),
                    J::Arr(
                        r.facts
                            .iter()
                            .map(|f| {
                                J::s(
                                    self.model
                                        .statements()
                                        .get(&format!("feature.{f}"))
                                        .map_or_else(String::new, Digest::hex),
                                )
                            })
                            .collect(),
                    ),
                );
                m.insert(
                    "keys".into(),
                    J::Obj(
                        r.keys
                            .iter()
                            .map(|(t, ks)| (t.clone(), J::strs(ks.iter().map(String::as_str))))
                            .collect(),
                    ),
                );
                m.insert("unjoined".into(), J::Bool(r.unjoined));
                m.insert("returns".into(), J::Bool(r.rows_or_features));
                m.insert(
                    "document".into(),
                    r.document
                        .as_ref()
                        .map_or(J::Null, |d| J::s(Digest::of(d.as_bytes()).hex())),
                );
                // What `via step commit` writes to `result.json` (`protocol.md` 7.1).
                m.insert(
                    "commit".into(),
                    obj([
                        (
                            "key",
                            J::Obj(
                                key_parts
                                    .iter()
                                    .map(|(k, v)| (k.clone(), J::s(v.clone())))
                                    .collect(),
                            ),
                        ),
                        (
                            "static_key",
                            J::s(crate::commit::static_key(&key_parts).hex()),
                        ),
                        (
                            "trace",
                            J::Arr(
                                r.reads
                                    .iter()
                                    .map(|rd| {
                                        obj([
                                            ("answer", J::s(rd.answer.hex())),
                                            ("request", J::s(rd.request.hex())),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                        ("trust_digest", J::s(code.trust.hex())),
                        // What the step raised itself and its explain texts, so a build that
                        // uses the committed result reports them as a trusted build does: the
                        // records as the build holds them (`diags`, replayed), and each as text
                        // for the reviewer (`diagnostics`).
                        (
                            "diags",
                            J::s(crate::commit::encode_diags(&r.emitted).unwrap_or_default()),
                        ),
                        (
                            "diagnostics",
                            J::Arr(
                                r.emitted
                                    .iter()
                                    .map(|d| {
                                        obj([
                                            ("code", J::s(d.code().to_string())),
                                            ("message", J::s(d.message().as_str().to_string())),
                                            ("severity", J::s(d.severity().as_str())),
                                            ("subject", J::s(d.subject().to_string())),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                        (
                            "explains",
                            J::Arr(
                                r.explains
                                    .iter()
                                    .map(|(c, e)| {
                                        J::Arr(vec![J::s(c.clone()), J::opt(e.as_deref())])
                                    })
                                    .collect(),
                            ),
                        ),
                    ]),
                );
            }
        }
        Output {
            value: Val {
                bytes: Arc::new(value.file()),
                diags: Arc::new(r.diags),
            },
            complete,
            step: Some(StepRun {
                exit,
                files,
                log,
                data,
                reads: r.reads,
            }),
            skipped: false,
        }
    }
}

/// `authority.unchecked` (`derived.md` 5) for one generate step that read `facts`, returned rows
/// or features and is checked by no lint: the one definition, in via-model.
fn unchecked(sid: &str, facts: &[String]) -> Diag {
    let step = via_model::derived_checks::StepInfo {
        id: sid.to_string(),
        when: "generate".into(),
        checks: None,
    };
    let read: BTreeMap<String, BTreeSet<String>> =
        [(sid.to_string(), facts.iter().cloned().collect())].into();
    let returned: BTreeSet<String> = [sid.to_string()].into();
    via_model::derived_checks::unchecked(&[step], &read, &returned)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            Diag::new(
                via_diag::codes::diag::INTERNAL,
                via_diag::Subject::entity("step", sid),
                via_diag::Site::Header,
                crate::txt("the unchecked step gave no diagnostic".into()),
                crate::txt("report this as a bug in via".into()),
            )
        })
}

/// A step node's id as (step, item).
pub fn split_step(id: &NodeId) -> (String, Option<String>) {
    let rest = id.as_str().strip_prefix("step:").unwrap_or(id.as_str());
    match rest.split_once("/i:") {
        Some((s, i)) => (s.to_string(), Some(i.to_string())),
        None => (rest.to_string(), None),
    }
}

/// The stated facts a generate step's value records (`derived.md` 5).
fn step_facts(bytes: &[u8]) -> Option<Vec<String>> {
    let j = parse(std::str::from_utf8(bytes).ok()?).ok()?;
    Some(
        j.get("facts")?
            .as_arr()?
            .iter()
            .filter_map(|f| f.as_str().map(str::to_string))
            .collect(),
    )
}

/// A lint step's value: whether its process was started, the explain text of each of its
/// codes, and each file it returned as (path, digest), as canonical JSON. The files are part of
/// the value (`graph.md` 1, `step`), so a rerun that writes other bytes beside its diagnostics is
/// `graph.step_unstable` (5.2).
fn step_bytes(
    ran: bool,
    explains: &[(String, Option<String>)],
    files: &[(String, Digest)],
) -> Vec<u8> {
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
        (
            "files",
            J::Arr(
                files
                    .iter()
                    .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
                    .collect(),
            ),
        ),
        ("ran", J::Bool(ran)),
    ])
    .file()
}

/// A generate call's returned document and the files of its `out`, from its value: the
/// document's digest and each file as (path, digest). `None` when it returned no document.
pub fn returned(bytes: &[u8]) -> Option<(Digest, Vec<(String, Digest)>)> {
    let j = parse(std::str::from_utf8(bytes).ok()?).ok()?;
    let doc = Digest::from_hex(j.get("document")?.as_str()?)?;
    let files = j
        .get("files")
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
    Some((doc, files))
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
/// the digest of a folder by `graph.md` 5.1's rule (every file, symbolic links as their
/// target's text, bytes as they are); a missing path digests as missing.
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
    let digest = steps::input_digest(&p).unwrap_or_else(|| Digest::of(b"\0missing"));
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
    manifest.insert("locations".into(), {
        let mut l = obj([("pack_root", ctx.pack_root.clone().map_or(J::Null, J::s))]);
        if !ctx.pack_roots.is_empty() {
            if let J::Obj(m) = &mut l {
                m.insert(
                    "pack_roots".into(),
                    J::Obj(
                        ctx.pack_roots
                            .iter()
                            .map(|(k, v)| (k.clone(), J::s(v.clone())))
                            .collect(),
                    ),
                );
            }
        }
        l
    });
    J::Obj(manifest).file()
}

impl Plan for CliPlan<'_> {
    fn answer(&self, step: &NodeId, read: &via_graph::Read) -> Option<Digest> {
        let (sid, _) = split_step(step);
        let l = self.ctx.lints.iter().find(|l| l.decl.id == sid)?;
        let bad = self.bad_for(l.decl.worker && l.decl.when == "lint");
        // An answer is a function of the model's leaves, the step, what failed and the request
        // (`graph.md` 5.1, Reuse): kept under them, it is checked without making the build.
        let memo = self.answer_memo(step, bad.as_deref(), read);
        if let Some(a) = self.memo_get(&memo) {
            return Some(a);
        }
        let build = self.the_build();
        let grid = build.grid?;
        let objects = via_proto::store::Store::open(self.ctx.store.root().join("objects")).ok()?;
        let view = crate::stepview::View {
            world: self.ctx.world,
            build: &build,
            grid,
            objects: &objects,
            pack: self.ctx.pack,
            step: &l.decl,
            rank: (l.decl.when == "generate")
                .then(|| self.ctx.world.layer_rank(&l.decl.layer))
                .flatten(),
            seed: via_num::world_seed(i64::try_from(self.ctx.seed).unwrap_or(0)).unwrap_or(0),
            stratum: Some(self.model),
            bad: bad.as_deref(),
        };
        let answer = crate::stepwork::replay(&view, self.ctx.store, read)?;
        self.memo_put(&memo, answer);
        Some(answer)
    }

    fn note_bad(&self, bad: &BTreeSet<NodeId>) {
        *self
            .bad
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Arc::new(crate::stepview::Bad::of(bad)));
    }

    fn define(&self, id: &NodeId, known: &Known<'_>) -> Defined {
        if self.model.def(id).is_some() {
            return self.model.define(id, known);
        }
        if let Some(d) = self.defs.get(id) {
            if id.family() == "step" {
                return Defined::Def(self.step_def(id, d));
            }
            return Defined::Def(d.clone());
        }
        if let Some(e) = self.each.get(id) {
            return Defined::Def(self.join_def(id, e));
        }
        let item = self
            .items
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(id)
            .cloned();
        if let Some(d) = item {
            return Defined::Def(self.step_def(id, &d));
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
            "step" if def.params.iter().any(|(k, _)| k == "join") => {
                self.compute_join(id, def, inputs)
            }
            "step" => self.compute_step(id, def, inputs),
            "budget" => match self.files().as_ref() {
                Ok(w) => Output::of(Val {
                    bytes: Arc::new(Vec::new()),
                    diags: Arc::new(w.budget.clone()),
                }),
                Err(d) => Output::of(Val {
                    bytes: Arc::new(Vec::new()),
                    diags: Arc::new(vec![d.clone()]),
                }),
            },
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
