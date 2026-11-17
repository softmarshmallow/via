//! An import's own generate steps, run inside the parent's build (`composition.md` 12).
//!
//! The model builds an import when the parent's text is prepared; before it does, it hands the
//! import's merged world to [`hook`], which runs the import's generate steps rank by rank in the
//! parent's store, exactly as a world built alone runs its own, and joins what they return. The
//! step nodes are keyed on the import's own statements (its frame holds no parent key), so a
//! moved import finds every call in the store. Their outcomes are kept under the import's
//! prefix (`a.step:lay/i:<item>`) for the parent's run report.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use via_diag::{Diag, FailOn};
use via_graph::{Digest, NodeId, NodeOutcome, Store, Versions};
use via_model::{ImportSteps, StepsCall, StepsDone};

use super::graph::{self, CliPlan, LintStep};
use crate::steps;

/// What every import's steps are run with: the parent's store, roots, trust and flags.
pub struct Env {
    pub store_root: PathBuf,
    pub lib: Vec<PathBuf>,
    pub config: Option<PathBuf>,
    pub sdk: Option<PathBuf>,
    pub sdk_digest: Option<Digest>,
    /// The trust digests this machine trusts: an import's steps are trusted as the parent's are
    /// (`composition.md` Open, decided M4 slice 4).
    pub trusted: BTreeSet<String>,
    pub threads: usize,
    pub versions: Versions,
    pub deadline: Duration,
    pub rerun: BTreeSet<String>,
    pub trust_command: String,
    /// Each import step node's outcome, under its prefixed id, the last run of it in this process.
    pub outcomes: Mutex<BTreeMap<NodeId, NodeOutcome>>,
    /// The run diagnostics of the import's step runs.
    pub run_diags: Mutex<Vec<Diag>>,
}

/// The hook the model calls before it builds an import.
pub fn hook(env: Arc<Env>) -> ImportSteps {
    ImportSteps(Arc::new(move |c: &StepsCall<'_>| run_import(&env, c)))
}

/// The id of an import's node in the parent's build: `<prefix><id>` (`composition.md` 12).
pub fn prefixed(prefix: &str, id: &NodeId) -> NodeId {
    match id.as_str().split_once(':') {
        Some((family, rest)) => NodeId::of(&format!("{prefix}{family}"), rest),
        None => NodeId::of(prefix.trim_end_matches('.'), id.as_str()),
    }
}

/// What a generate step returned in one rank's run, joined over its items in item order: the
/// document text and the folder holding the files of its `out`. `None` when it returned nothing
/// or failed.
pub fn returned_of(
    run_k: &via_graph::Run,
    plan_k: &CliPlan<'_>,
    store: &Store,
    l: &LintStep,
) -> Option<(String, PathBuf)> {
    // A step's own value, or each item's under `for_each`, in item order.
    let mut ids: Vec<&NodeId> = run_k
        .nodes
        .keys()
        .filter(|n| graph::split_step(n).0 == l.decl.id)
        .filter(|n| l.decl.for_each.is_none() || graph::split_step(n).1.is_some())
        .collect();
    ids.sort();
    let mut text = String::new();
    let mut files: Vec<(String, Digest)> = Vec::new();
    for n in ids {
        let Some(v) = run_k.value(n, store, plan_k) else {
            continue;
        };
        if v.diags
            .iter()
            .any(|d| d.severity() == via_diag::Severity::Error)
        {
            continue;
        }
        let Some((doc, fs)) = graph::returned(&v.bytes) else {
            continue;
        };
        let Some(bytes) = store.get_object(&doc) else {
            continue;
        };
        text.push_str(&String::from_utf8_lossy(&via_graph::object_bytes(&bytes)));
        text.push('\n');
        files.extend(fs);
    }
    if text.trim().is_empty() {
        return None;
    }
    // The files of `out`, where a returned field's `file` is read. The folder is keyed by the
    // document text and every returned file's digest, so a run that returns new file bytes under
    // the same text never reads an older run's files.
    let mut key = text.clone().into_bytes();
    for (p, d) in &files {
        key.push(0);
        key.extend_from_slice(p.as_bytes());
        key.push(0);
        key.extend_from_slice(d.hex().as_bytes());
    }
    let dir = store.root().join("returned").join(Digest::of(&key).hex());
    let mut ok = std::fs::create_dir_all(&dir).is_ok();
    for (p, d) in &files {
        let target = dir.join(p);
        if target.is_file() {
            continue;
        }
        ok &= match store.get_object(d) {
            Some(b) => {
                if let Some(parent) = target.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&target, via_graph::object_bytes(&b)).is_ok()
            }
            None => false,
        };
    }
    ok.then_some((text, dir))
}

/// Runs the import's generate steps rank by rank and joins what they return.
fn run_import(env: &Env, c: &StepsCall<'_>) -> Option<StepsDone> {
    let wdir = crate::build::world_dir(c.path);
    let lint_steps: Vec<LintStep> = {
        let decls = steps::declared(c.world, &wdir);
        let roots = roots_of(env, c, &wdir);
        decls
            .into_iter()
            .filter(|s| s.when == "generate")
            .map(|s| {
                let code = steps::code_of(&s, &roots, &wdir, env.sdk_digest.as_ref());
                let trusted = code
                    .as_ref()
                    .is_ok_and(|k| env.trusted.contains(&k.trust.hex()));
                LintStep {
                    decl: s,
                    code,
                    trusted,
                }
            })
            .collect()
    };
    if lint_steps.is_empty() {
        return None;
    }
    let roots = roots_of(env, c, &wdir);
    let store = Store::open(&env.store_root);
    // The import's own pack (`composition.md` 10), for the instance taxa a step is handed.
    let pack = c
        .world
        .root()
        .get("pack")
        .and_then(|e| match &e.item.value {
            via_doc::Value::Str(s) => Some(s.text.clone()),
            _ => None,
        })
        .and_then(|written| {
            let r = via_model::pack::Roots {
                world_dir: c.options.world_dir.clone(),
                lib_roots: c.options.lib_roots.clone(),
                via_lib: c.options.via_lib.clone(),
                shipped: c.options.shipped_root.clone(),
            };
            let path = via_model::pack::find(&written, &r, &mut Vec::new())?;
            via_model::pack::read(&path, "pack").0
        });
    let fail_on = FailOn::none();
    let mut opts = c.options.clone();
    let mut ranks: Vec<i64> = lint_steps
        .iter()
        .filter_map(|l| c.world.layer_rank(&l.decl.layer))
        .collect();
    ranks.sort_unstable();
    ranks.dedup();
    let seed = opts
        .seed
        .unwrap_or_else(|| crate::build::world_seed(c.world));
    // What a step's answers are a function of besides the import's sources and inputs (the memo
    // of a trace's reads, `graph.md` 5.1): its seed and what the parent gave it.
    let key = Digest::of(
        format!(
            "via-import-steps-1 seed {seed} given {}",
            opts.given.as_ref().map_or("", |g| g.digest.as_str())
        )
        .as_bytes(),
    );
    let mut joined = c.world.clone();
    let mut returned: Vec<via_doc::Returned> = Vec::new();
    let mut diags: Vec<Diag> = Vec::new();
    for rank in ranks {
        let model_k = via_model::ModelPlan::new(&joined, &opts, None);
        let ctx_k = graph::Ctx {
            world: &joined,
            store: &store,
            roots: &roots,
            config: env.config.as_deref(),
            sdk: env.sdk.as_deref(),
            sdk_digest: env.sdk_digest.as_ref(),
            seed,
            deadline: env.deadline,
            pack: pack.as_ref(),
            trust_command: env.trust_command.clone(),
            name: "import",
            fail_on: &fail_on,
            loaded: &[],
            extra: Vec::new(),
            lints: lint_steps.clone(),
            baseline: None,
            baseline_record: None,
            key,
            version: env!("CARGO_PKG_VERSION").to_string(),
            pack_root: None,
            pack_roots: BTreeMap::new(),
            run_members: BTreeMap::new(),
        };
        let plan_k = CliPlan::new(&model_k, ctx_k);
        let at_rank: Vec<&LintStep> = lint_steps
            .iter()
            .filter(|l| joined.layer_rank(&l.decl.layer) == Some(rank))
            .collect();
        let step_roots: Vec<NodeId> = at_rank
            .iter()
            .map(|l| NodeId::of("step", &l.decl.id))
            .collect();
        let mut gopts = via_graph::Options::new(env.versions.clone());
        gopts.threads = env.threads;
        // `--rerun-step <id>.<step>` names an import's step; a bare id names the parent's only
        // (`composition.md` 12).
        gopts.rerun = env
            .rerun
            .iter()
            .filter_map(|r| r.strip_prefix(c.prefix).map(str::to_string))
            .collect();
        let run_k = via_graph::run(&plan_k, &store, &gopts, &step_roots);
        diags.extend(plan_k.stop_workers());
        // A step's own findings (untrusted, failed, refused) are the import's world diagnostics;
        // the run's are this build's run diagnostics.
        diags.extend(run_k.world_diags.iter().filter(|d| {
            matches!(d.subject(), via_diag::Subject::Entity { collection, .. } if collection == "step")
        }).cloned());
        env.run_diags
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(run_k.run_diags.iter().cloned());
        {
            let mut out = env
                .outcomes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (id, o) in &run_k.nodes {
                if id.family() != "step" {
                    continue;
                }
                out.insert(prefixed(c.prefix, id), o.clone());
                // What a step said (untrusted, failed) is the import's to report.
                if let Some(v) = run_k.value(id, &store, &plan_k) {
                    diags.extend(v.diags.iter().cloned());
                }
            }
        }
        for l in &at_rank {
            if let Some((text, dir)) = returned_of(&run_k, &plan_k, &store, l) {
                opts.step_out.insert(l.decl.id.clone(), dir);
                returned.push(via_doc::Returned {
                    step: l.decl.id.clone(),
                    layer: l.decl.layer.clone(),
                    text,
                });
            }
        }
        let (w, recs) = c.world.join(&returned);
        joined = w;
        let _ = recs;
    }
    Some(StepsDone {
        world: joined,
        step_out: opts.step_out,
        diags: via_model::dedupe(diags),
    })
}

fn roots_of(env: &Env, c: &StepsCall<'_>, wdir: &std::path::Path) -> steps::Roots {
    steps::Roots {
        world_dir: wdir.to_path_buf(),
        lib: env.lib.clone(),
        via_lib: c.options.via_lib.clone(),
        shipped: c.options.shipped_root.clone(),
        include: steps::includes(c.world),
    }
}

/// What one import adds to the parent's build key (`graph.md` 6.2): a digest of its text, read
/// as [`run_import`] and the model read it, never of its build. Its sources and their names, its
/// `seed` and `changes`, its leaves (pack, bound files, field files), its library lookups, each of
/// its generate steps' trust digest, trust on this machine, declared inputs and (untrusted) its
/// committed result, and its own imports' keys in turn. Each build is a function of these, so an
/// equal key is an equal import; an unchanged build reads each import's files and builds none
/// (M6 close: atlas's unchanged build spent 36 s building thirteen imports for its key).
pub fn text_key(
    imp: &via_model::ImportText,
    opts: &via_model::Options,
    lib: &[PathBuf],
    sdk: Option<&Digest>,
    trusted: &BTreeSet<String>,
    within: &mut Vec<PathBuf>,
) -> Digest {
    let mut e = via_graph::Enc::new();
    e.str("via-import-key-1")
        .str(&format!("seed {:?}", imp.seed))
        .len(imp.changes.len());
    for c in &imp.changes {
        e.str(c);
    }
    // An import that reaches itself is `import.cycle` at load; it is not followed again here.
    if within.contains(&imp.path) {
        e.str("cycle");
        return Digest::of(&e.finish());
    }
    // Loaded as the model loads it (`via_model::import`, `built`): a change file's path is from
    // the import's folder.
    let changes: Vec<via_doc::ChangeRequest> = imp
        .changes
        .iter()
        .map(|c| {
            let r = via_doc::ChangeRequest::new(c.clone(), 0);
            match imp.path.parent() {
                Some(dir) if r.is_path() => {
                    via_doc::ChangeRequest::new(dir.join(c).to_string_lossy().into_owned(), 0)
                }
                _ => r,
            }
        })
        .collect();
    let mut load = via_doc::LoadOptions::from_env(opts.lib_roots.clone(), changes);
    load.shipped_root.clone_from(&opts.shipped_root);
    let loaded = via_doc::load(&imp.path, 0, &load);
    let Some(world) = loaded.world else {
        // What failed to load is decided by the file's bytes and the records it gave.
        let bytes = std::fs::read(&imp.path).unwrap_or_default();
        e.str("failed")
            .str(&Digest::of(&bytes).hex())
            .str(&format!("{:?}", loaded.records));
        return Digest::of(&e.finish());
    };
    e.len(world.sources().iter().count());
    for s in world.sources().iter() {
        e.str(&s.name).str(&Digest::of(s.text().as_bytes()).hex());
    }
    e.len(world.changes().len());
    for c in world.changes() {
        e.str(c);
    }
    let wdir = crate::build::world_dir(&imp.path);
    let mut o = opts.clone();
    o.seed = imp.seed;
    o.world_dir = imp.path.parent().map(PathBuf::from).unwrap_or_default();
    o.step_out = BTreeMap::new();
    o.given = None;
    o.import_steps = None;
    let (leaves, nested) = via_model::leaves_own(&world, &o);
    e.len(leaves.len());
    for (p, d) in &leaves {
        e.str(p).str(&d.hex());
    }
    let roots = steps::Roots {
        world_dir: wdir.clone(),
        lib: lib.to_vec(),
        via_lib: o.via_lib.clone(),
        shipped: o.shipped_root.clone(),
        include: steps::includes(&world),
    };
    let found = super::lookups(&world, &roots);
    e.len(found.len());
    for l in &found {
        e.str(l);
    }
    let decls: Vec<steps::StepDecl> = steps::declared(&world, &wdir)
        .into_iter()
        .filter(|s| s.when == "generate")
        .collect();
    e.len(decls.len());
    for s in &decls {
        e.str(&s.id);
        match steps::code_of(s, &roots, &wdir, sdk) {
            Ok(c) => {
                let t = trusted.contains(&c.trust.hex());
                e.str(&c.trust.hex())
                    .str(if t { "trusted" } else { "untrusted" });
                if !t {
                    let gen = crate::commit::folder(&s.home, &s.id, None);
                    e.str(&steps::input_digest(&gen).map_or(String::new(), |d| d.hex()));
                }
            }
            Err(d) => {
                e.str(&format!("no code {:?}", d.code()));
            }
        }
        e.len(s.inputs.len());
        for i in &s.inputs {
            let d =
                steps::input_digest(&s.home.join(i)).unwrap_or_else(|| Digest::of(b"\0missing"));
            e.str(i).str(&d.hex());
        }
    }
    within.push(imp.path.clone());
    e.len(nested.len());
    for n in &nested {
        e.str(&n.id)
            .str(&text_key(n, &o, lib, sdk, trusted, within).hex());
    }
    within.pop();
    Digest::of(&e.finish())
}
