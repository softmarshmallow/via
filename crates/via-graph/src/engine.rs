//! Running the graph (`spec/graph.md` 5, 6.1 and 8).
//!
//! Keys are worked out from the leaves up. One coordinating thread defines nodes, works out
//! keys, looks them up and settles results, always in id order; only [`Plan::compute`] runs on
//! other threads, in batches of ready nodes taken in id order, and so do the trace replays of
//! the ready steps ([`Plan::answer`], `graph.md` 5.1), whose matches are taken in id order.
//! Every status, cause and value is
//! then a function of the store and the text, never of which thread finished first: among nodes
//! of one build with equal keys the lowest id carries `ran`, and whatever combines several values
//! does so by id.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use via_diag::{codes, poison, Diag, Severity, Site, Subject, Text};

use crate::codec::{decode_object, encode_val, Val};
use crate::digest::{Dec, Digest, Enc};
use crate::id::NodeId;
use crate::key::{key, KeyParts, Versions};
use crate::plan::{Def, Defined, Inputs, Known, Output, Persist, Plan, Read, Reports, Settled};
use crate::record::{BuildRecord, RecordNode};
use crate::refgraph::Cycle;
use crate::store::{KeyRecord, Store};

/// A node's status (6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Its key is in the store.
    Reused,
    /// Its key is in the store, it is digest only, and something needed its value.
    Recomputed,
    /// Its key is not in the store: computed and stored.
    Ran,
    /// It reads a failed node, is a member of a cycle's group, or is gated.
    Skipped,
}

impl Status {
    /// The word the run report writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Reused => "reused",
            Status::Recomputed => "recomputed",
            Status::Ran => "ran",
            Status::Skipped => "skipped",
        }
    }
}

/// Why a `ran` node's key was not in the store (6.3); the first rule that holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// `--rerun-step` named it.
    Rerun,
    /// There is no previous build, or its record is not in the store.
    NoRecord,
    /// The record's versions differ.
    Versions,
    /// Its id is not in the previous record.
    NewNode,
    /// A statement it is made from has another digest.
    Statements,
    /// An input edge's node has another value digest, or is not in the record.
    Edge {
        /// The edge's label.
        label: String,
        /// The input's id.
        node: NodeId,
    },
    /// Its key differs for none of the reasons above.
    Key,
    /// A step whose static key equals the previous record's, whose stored trace no longer
    /// matches: the first recorded read whose answer differs now.
    Trace {
        /// The read's method.
        method: String,
        /// The digest of its normalised request.
        request: Digest,
    },
    /// Its key equals the previous record's, but the store does not hold it.
    NotStored,
}

impl Cause {
    /// The `reason` member.
    pub fn reason(&self) -> &'static str {
        match self {
            Cause::Rerun => "rerun",
            Cause::NoRecord => "no record",
            Cause::Versions => "versions",
            Cause::NewNode => "new node",
            Cause::Statements => "statements",
            Cause::Edge { .. } => "edge",
            Cause::Key => "key",
            Cause::Trace { .. } => "trace",
            Cause::NotStored => "not stored",
        }
    }
}

/// Who needed a `recomputed` node's value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NeededBy {
    /// The lowest id among the readers that ran or were recomputed.
    Node(NodeId),
    /// Only the expansion of the graph (2.1).
    Expansion,
}

/// A step node's facts for the run report (6.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepOutcome {
    /// The command.
    pub command: Vec<String>,
    /// The static key (5.1): the step node's key.
    pub static_key: Option<Digest>,
    /// The trace digest it took or added; `None` for an empty trace, or when it did not run
    /// or match.
    pub trace: Option<Digest>,
    /// Each read in the order made.
    pub reads: Vec<Read>,
    /// The exit status when it ran in this build.
    pub exit: Option<i64>,
    /// The value digest and the files of `out`; `None` when it returned nothing readable.
    pub returned: Option<(Digest, Vec<(String, Digest)>)>,
    /// Its log's digest.
    pub log: Option<Digest>,
}

/// One node of a run.
#[derive(Clone, Debug)]
pub struct NodeOutcome {
    /// Its status.
    pub status: Status,
    /// Its key, when worked out.
    pub key: Option<Digest>,
    /// Its value digest.
    pub value: Option<Digest>,
    /// True when its value holds an error, or it is a step that did not complete.
    pub failed: bool,
    /// For a `ran` node, why.
    pub cause: Option<Cause>,
    /// For a `recomputed` node, who needed it.
    pub needed_by: Option<NeededBy>,
    /// Whole milliseconds from its start to its value or status being known.
    pub ms: u64,
    /// For a step node.
    pub step: Option<StepOutcome>,
}

/// What a run is given besides the plan and the store.
#[derive(Clone, Debug)]
pub struct Options {
    /// The four versions.
    pub versions: Versions,
    /// `--threads`; at least 1.
    pub threads: usize,
    /// `--rerun-step`: step ids.
    pub rerun: BTreeSet<String>,
    /// The previous build's record, for causes (6.3).
    pub previous: Option<BuildRecord>,
    /// The reference graph's cycles (2.2): their members' nodes fail as one group.
    pub cycles: Vec<Cycle>,
}

impl Options {
    /// Options with the given versions, one thread, and nothing else.
    pub fn new(versions: Versions) -> Options {
        Options {
            versions,
            threads: 1,
            rerun: BTreeSet::new(),
            previous: None,
            cycles: Vec::new(),
        }
    }
}

/// The result of a run.
#[derive(Debug)]
pub struct Run {
    /// Every node of the graph, by id.
    pub nodes: BTreeMap<NodeId, NodeOutcome>,
    /// The world diagnostics: what each node reports (3.4), by node id, then the graph's own
    /// (`graph.cycle`, `diag.skipped`, and `diag.internal` for a defect of the plan).
    pub world_diags: Vec<Diag>,
    /// The run diagnostics (3.4): never in `diagnostics.json`.
    pub run_diags: Vec<Diag>,
    /// Whether every step completed.
    pub complete: bool,
    /// Whether the build has any error.
    pub has_error: bool,
    values: BTreeMap<NodeId, Val>,
    pub(crate) statements: BTreeMap<String, Digest>,
    direct: BTreeMap<NodeId, Vec<(String, Digest)>>,
    diags: BTreeMap<NodeId, Arc<Vec<Diag>>>,
    pub(crate) edges: BTreeMap<NodeId, Vec<(String, NodeId)>>,
    pub(crate) made_from: BTreeMap<NodeId, Vec<String>>,
}

impl Run {
    /// A node's value: from memory when this run computed or loaded it, else from the store.
    pub fn value(&self, id: &NodeId, store: &Store, plan: &dyn Plan) -> Option<Val> {
        if let Some(v) = self.values.get(id) {
            return Some(v.clone());
        }
        let n = self.nodes.get(id)?;
        let vd = n.value?;
        let diags = self.diags.get(id).cloned().unwrap_or_default();
        decode_object(&store.get_object(&vd)?, &diags, plan.codec())
    }

    /// The build record of this run (6.2). The caller adds the emitted world diagnostics and the
    /// build files, which only it can write.
    pub fn record(
        &self,
        versions: &Versions,
        world_diags: Vec<u8>,
        files: BTreeMap<String, Digest>,
    ) -> BuildRecord {
        BuildRecord {
            versions: versions.clone(),
            nodes: self
                .nodes
                .iter()
                .map(|(id, n)| {
                    (
                        id.clone(),
                        RecordNode {
                            key: n.key,
                            value: n.value,
                            direct: self.direct.get(id).cloned().unwrap_or_default(),
                            inputs: {
                                let mut ins: Vec<NodeId> = self
                                    .edges
                                    .get(id)
                                    .into_iter()
                                    .flatten()
                                    .map(|(_, t)| t.clone())
                                    .collect();
                                ins.sort();
                                ins.dedup();
                                ins
                            },
                            made_from: self.made_from.get(id).cloned().unwrap_or_default(),
                        },
                    )
                })
                .collect(),
            statements: self.statements.clone(),
            world_diags,
            files,
            complete: self.complete,
            steps: self
                .nodes
                .iter()
                .filter_map(|(id, n)| {
                    let mut so = n.step.clone()?;
                    so.exit = None;
                    Some((id.clone(), so))
                })
                .collect(),
        }
    }

    /// The set of `ran` nodes, which conformance asserts exactly.
    pub fn ran(&self) -> BTreeSet<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.status == Status::Ran)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Undefined,
    Need(Vec<NodeId>),
    Last,
    Defined,
}

struct St {
    phase: Phase,
    def: Option<Def>,
    last: bool,
    status: Option<Status>,
    key: Option<Digest>,
    vd: Option<Digest>,
    diags: Option<Arc<Vec<Diag>>>,
    value: Option<Val>,
    failed: bool,
    created: bool,
    not_stored: bool,
    rerun: bool,
    needed: BTreeSet<Option<NodeId>>,
    ms: u64,
    step: Option<StepOutcome>,
    matched: Option<Digest>,
}

impl St {
    fn new() -> St {
        St {
            phase: Phase::Undefined,
            def: None,
            last: false,
            status: None,
            key: None,
            vd: None,
            diags: None,
            value: None,
            failed: false,
            created: false,
            not_stored: false,
            rerun: false,
            needed: BTreeSet::new(),
            ms: 0,
            step: None,
            matched: None,
        }
    }

    fn settled(&self) -> bool {
        self.status.is_some()
    }

    fn usable(&self) -> bool {
        matches!(
            self.status,
            Some(Status::Reused | Status::Recomputed | Status::Ran)
        ) && !self.failed
            && self.vd.is_some()
    }
}

/// One computation of a batch: the node, its definition, and its inputs.
type Job = (NodeId, Def, Vec<(String, NodeId, Val)>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Settle {
    Fresh,
    Restore,
    Recompute,
}

struct Engine<'a> {
    plan: &'a dyn Plan,
    store: &'a Store,
    opts: &'a Options,
    st: BTreeMap<NodeId, St>,
    blocked: BTreeMap<String, usize>,
    gates_open: bool,
    has_error: bool,
    world: Option<Vec<Diag>>,
    run_diags: Vec<Diag>,
    defects: Vec<Diag>,
    /// The trace each ready step matched, replayed on several threads before the ready nodes
    /// are looked up in id order: by node, the static key it was found under and the match.
    found: BTreeMap<NodeId, (Digest, Option<Found>)>,
    /// Key records of the ready nodes, read on several threads before the lookup (section 8):
    /// by key, the record or none.
    keys_read: BTreeMap<Digest, Option<KeyRecord>>,
    /// Objects of a batch's inputs, read and checked on several threads before the batch's
    /// inputs are materialized (section 8), by value digest.
    objects_read: BTreeMap<Digest, Option<Vec<u8>>>,
}

/// A trace that answers the same again: its name, its entry and its diagnostics.
type Found = (Digest, TraceEntry, Vec<Diag>);

/// Runs the graph from its roots: every node reachable from them along edges and needs.
pub fn run(plan: &dyn Plan, store: &Store, opts: &Options, roots: &[NodeId]) -> Run {
    let mut blocked = BTreeMap::new();
    for (i, c) in opts.cycles.iter().enumerate() {
        for v in &c.vertices {
            blocked.entry(v.clone()).or_insert(i);
        }
    }
    let mut e = Engine {
        plan,
        store,
        opts,
        st: BTreeMap::new(),
        blocked,
        gates_open: false,
        has_error: !opts.cycles.is_empty(),
        world: None,
        run_diags: Vec::new(),
        defects: Vec::new(),
        found: BTreeMap::new(),
        keys_read: BTreeMap::new(),
        objects_read: BTreeMap::new(),
    };
    for r in roots {
        e.discover(r);
    }
    e.drive();
    e.finish()
}

impl Engine<'_> {
    fn discover(&mut self, id: &NodeId) {
        self.st.entry(id.clone()).or_insert_with(St::new);
    }

    fn get(&self, id: &NodeId) -> Option<&St> {
        self.st.get(id)
    }

    fn defect(&mut self, id: &NodeId, why: &str) {
        let message = format!("the build graph could not settle `{id}`: {why}");
        self.defects.push(
            Diag::new(
                codes::diag::INTERNAL,
                Subject::World,
                Site::World,
                text(format!("a node could not be built: {message}")),
                via_diag::text!("report it: this is a defect of the engine, not of the text"),
            )
            .with("message", message),
        );
        self.has_error = true;
        if let Some(s) = self.st.get_mut(id) {
            s.phase = Phase::Defined;
            s.status = Some(Status::Skipped);
            s.failed = true;
        }
    }

    fn drive(&mut self) {
        loop {
            let mut progress = false;
            let ids: Vec<NodeId> = self
                .st
                .iter()
                .filter(|(_, s)| s.phase == Phase::Undefined)
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids {
                progress = true;
                self.define(&id, BTreeMap::new());
            }
            let waiting: Vec<(NodeId, Vec<NodeId>)> = self
                .st
                .iter()
                .filter_map(|(id, s)| match (&s.phase, s.status) {
                    (Phase::Need(n), None) => Some((id.clone(), n.clone())),
                    _ => None,
                })
                .collect();
            for (id, needs) in waiting {
                if !needs.iter().all(|n| self.get(n).is_some_and(St::settled)) {
                    continue;
                }
                progress = true;
                if !needs.iter().all(|n| self.get(n).is_some_and(St::usable)) {
                    self.skip(&id);
                    continue;
                }
                let mut values = BTreeMap::new();
                let mut ok = true;
                for n in &needs {
                    match self.materialize(n, None) {
                        Some(v) => {
                            values.insert(n.clone(), v);
                        }
                        None => ok = false,
                    }
                }
                if ok {
                    self.define(&id, values);
                } else {
                    self.skip(&id);
                }
            }
            let mut batch = Vec::new();
            let ready: Vec<NodeId> = self
                .st
                .iter()
                .filter(|(_, s)| s.phase == Phase::Defined && !s.settled())
                .map(|(id, _)| id.clone())
                .collect();
            self.replay_ready(&ready);
            self.read_keys_ahead(&ready);
            for id in ready {
                if self.lookup(&id) {
                    progress = true;
                } else if self
                    .get(&id)
                    .is_some_and(|s| s.key.is_some() && !s.settled())
                {
                    batch.push(id);
                }
            }
            self.found.clear();
            self.keys_read.clear();
            if !batch.is_empty() {
                progress = true;
                self.run_batch(batch);
            }
            if progress {
                continue;
            }
            if !self.gates_open && self.ungated_settled() {
                self.gates_open = true;
                self.has_error |= self.st.values().any(|s| s.failed);
                let bad: std::collections::BTreeSet<NodeId> = self
                    .st
                    .iter()
                    .filter(|(_, s)| s.failed || s.status == Some(Status::Skipped))
                    .map(|(id, _)| id.clone())
                    .collect();
                self.plan.note_bad(&bad);
                continue;
            }
            if self.world.is_none() && self.gates_open && self.non_last_settled() {
                self.has_error |= self.st.values().any(|s| s.failed);
                self.world = Some(self.world_diags());
                let lasts: Vec<NodeId> = self
                    .st
                    .iter()
                    .filter(|(_, s)| s.phase == Phase::Last)
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in lasts {
                    if let Some(s) = self.st.get_mut(&id) {
                        s.phase = Phase::Undefined;
                        s.last = true;
                    }
                }
                continue;
            }
            break;
        }
        let left: Vec<NodeId> = self
            .st
            .iter()
            .filter(|(_, s)| !s.settled())
            .map(|(id, _)| id.clone())
            .collect();
        for id in left {
            self.defect(&id, "it waits on itself, or on a node defined after it");
        }
    }

    fn ungated_settled(&self) -> bool {
        self.st.values().all(|s| {
            s.settled()
                || s.phase == Phase::Last
                || s.last
                || s.def
                    .as_ref()
                    .is_some_and(|d| d.gate.skip_on_error || d.gate.traced)
        })
    }

    fn non_last_settled(&self) -> bool {
        self.st
            .values()
            .all(|s| s.settled() || s.phase == Phase::Last || s.last)
    }

    fn skip(&mut self, id: &NodeId) {
        if let Some(s) = self.st.get_mut(id) {
            s.phase = Phase::Defined;
            s.status = Some(Status::Skipped);
        }
    }

    fn define(&mut self, id: &NodeId, values: BTreeMap<NodeId, Val>) {
        if self.blocked.contains_key(&id.vertex()) {
            self.skip(id);
            return;
        }
        let in_last = self.world.is_some();
        let settled: Option<BTreeMap<NodeId, Settled>> = in_last.then(|| {
            self.st
                .iter()
                .filter_map(|(n, s)| {
                    s.status.map(|status| {
                        (
                            n.clone(),
                            Settled {
                                status,
                                value: s.vd,
                                failed: s.failed,
                            },
                        )
                    })
                })
                .collect()
        });
        let step_reads: Option<BTreeMap<NodeId, Vec<Read>>> = in_last.then(|| {
            self.st
                .iter()
                .filter(|(_, s)| s.status.is_some())
                .filter_map(|(n, s)| s.step.as_ref().map(|so| (n.clone(), so.reads.clone())))
                .collect()
        });
        let defined = {
            let known = Known {
                values: &values,
                world: self.world.as_deref(),
                has_error: self.has_error,
                settled: settled.as_ref(),
                step_reads: step_reads.as_ref(),
            };
            self.plan.define(id, &known)
        };
        match defined {
            Defined::Def(def) => {
                for (_, t) in &def.edges {
                    self.discover(t);
                }
                if let Some(s) = self.st.get_mut(id) {
                    s.phase = Phase::Defined;
                    s.def = Some(def);
                    s.last |= in_last;
                }
            }
            Defined::Need(needs) => {
                for n in &needs {
                    self.discover(n);
                }
                if let Some(s) = self.st.get_mut(id) {
                    s.phase = Phase::Need(needs);
                }
            }
            Defined::Last if !in_last => {
                if let Some(s) = self.st.get_mut(id) {
                    s.phase = Phase::Last;
                }
            }
            Defined::Last => self.defect(id, "it asked to be defined last twice"),
        }
    }

    /// Works out the key and settles the node when it can without computing; returns true when
    /// it settled. A node left unsettled with a key goes into the batch.
    fn lookup(&mut self, id: &NodeId) -> bool {
        let Some(s) = self.get(id) else {
            return false;
        };
        let Some(d) = s.def.as_ref() else {
            return false;
        };
        if (d.gate.skip_on_error || d.gate.traced) && !self.gates_open {
            return false;
        }
        // Asked again each pass until its edges settle: said without copying its definition.
        if !d.found.iter().any(|f| f.severity() == Severity::Error)
            && !d
                .edges
                .iter()
                .all(|(_, t)| self.get(t).is_some_and(St::settled))
        {
            return false;
        }
        let def = d.clone();
        if def.found.iter().any(|d| d.severity() == Severity::Error) {
            return self.settle_found(id, &def);
        }
        let Some(k) = self.key_of(id, &def) else {
            return false;
        };
        let gated = def.gate.skip_on_error && self.has_error;
        if let Some(s) = self.st.get_mut(id) {
            s.key = k;
            if let Some(sd) = &def.step {
                s.step = Some(StepOutcome {
                    command: sd.command.clone(),
                    static_key: k,
                    trace: None,
                    reads: Vec::new(),
                    exit: None,
                    returned: None,
                    log: None,
                });
            }
        }
        let Some(k) = k else {
            self.skip(id);
            return true;
        };
        if gated {
            self.skip(id);
            return true;
        }
        if let Some(sd) = &def.step {
            let rerun = self.opts.rerun.contains(&sd.step);
            let found = match self.found.remove(id) {
                Some((pk, f)) if pk == k => f,
                _ => self.find_trace(id, &k),
            };
            let Some(s) = self.st.get_mut(id) else {
                return false;
            };
            s.rerun = rerun;
            match found {
                Some((trace, entry, diags)) => {
                    let vd = entry.value;
                    s.matched = Some(vd);
                    if rerun {
                        return false;
                    }
                    s.failed = has_error(&diags);
                    s.vd = Some(vd);
                    s.diags = Some(Arc::new(diags));
                    s.status = Some(Status::Reused);
                    if let Some(so) = s.step.as_mut() {
                        so.trace = (!entry.reads.is_empty()).then_some(trace);
                        so.reads = entry.data.into_iter().chain(entry.reads).collect();
                        so.returned = Some((vd, entry.files));
                        so.log = entry.log;
                    }
                    true
                }
                None => false,
            }
        } else {
            let read = match self.keys_read.remove(&k) {
                Some(r) => r,
                None => self.store.get_key(&k),
            };
            let Some(rec) = read else {
                return false;
            };
            let Some(diags) = self.plan.codec().decode(&rec.diags) else {
                self.store.drop_key(&k);
                return false;
            };
            let Some(s) = self.st.get_mut(id) else {
                return false;
            };
            s.failed = has_error(&diags);
            s.vd = Some(rec.value);
            s.diags = Some(Arc::new(diags));
            s.status = Some(Status::Reused);
            true
        }
    }

    /// The node's key once every edge settled: `Some(None)` when an edge is not usable.
    #[allow(clippy::option_option)]
    fn key_of(&self, id: &NodeId, def: &Def) -> Option<Option<Digest>> {
        let mut edges = Vec::with_capacity(def.edges.len() + def.direct.len());
        let mut any_bad = false;
        for (l, t) in &def.edges {
            let ts = self.get(t).filter(|ts| ts.settled())?;
            if ts.usable() {
                if let Some(vd) = ts.vd {
                    edges.push((l.clone(), vd));
                }
            } else {
                any_bad = true;
            }
        }
        Some((!any_bad).then(|| {
            edges.extend(def.direct.iter().cloned());
            key(
                &self.opts.versions,
                &KeyParts {
                    family: id.family().to_string(),
                    statements: def.statements.iter().map(|(_, d)| *d).collect(),
                    params: def.params.clone(),
                    edges,
                },
            )
        }))
    }

    /// Replays the traces of the ready steps whose edges have settled on several threads, each
    /// a pure function of the store and the plan's answers, so that [`Engine::lookup`] takes
    /// each match in id order as if it had replayed it there. Steps sharing a static key, or
    /// forced by `--rerun-step`, are left to the lookup.
    fn replay_ready(&mut self, ready: &[NodeId]) {
        let mut jobs: Vec<(NodeId, Digest)> = Vec::new();
        let mut seen: BTreeMap<Digest, usize> = BTreeMap::new();
        for id in ready {
            let Some(d) = self.get(id).and_then(|s| s.def.as_ref()) else {
                continue;
            };
            let Some(sd) = d.step.as_ref() else {
                continue;
            };
            if (d.gate.skip_on_error || d.gate.traced) && !self.gates_open
                || d.gate.skip_on_error && self.has_error
                || d.found.iter().any(|f| f.severity() == Severity::Error)
                || self.opts.rerun.contains(&sd.step)
            {
                continue;
            }
            let Some(Some(k)) = self.key_of(id, d) else {
                continue;
            };
            *seen.entry(k).or_default() += 1;
            jobs.push((id.clone(), k));
        }
        jobs.retain(|(_, k)| seen.get(k) == Some(&1));
        let threads = self.opts.threads.max(1).min(jobs.len());
        if threads <= 1 {
            return;
        }
        let (plan, store) = (self.plan, self.store);
        let next = AtomicUsize::new(0);
        let slots: Vec<Mutex<Option<Option<Found>>>> =
            jobs.iter().map(|_| Mutex::new(None)).collect();
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= jobs.len() {
                        break;
                    }
                    let r = find_trace(plan, store, &jobs[i].0, &jobs[i].1);
                    if let Ok(mut slot) = slots[i].lock() {
                        *slot = Some(r);
                    }
                });
            }
        });
        for ((id, k), slot) in jobs.into_iter().zip(slots) {
            if let Some(r) = slot.into_inner().ok().flatten() {
                self.found.insert(id, (k, r));
            }
        }
    }

    /// Reads the key records of the ready nodes that are not steps on several threads, so that
    /// [`Engine::lookup`] takes each as if it had read it there (section 8). A record is a pure
    /// function of the store; the lookup still settles the nodes one by one in id order.
    fn read_keys_ahead(&mut self, ready: &[NodeId]) {
        let mut keys: BTreeSet<Digest> = BTreeSet::new();
        for id in ready {
            let Some(d) = self.get(id).and_then(|s| s.def.as_ref()) else {
                continue;
            };
            if d.step.is_some()
                || (d.gate.skip_on_error || d.gate.traced) && !self.gates_open
                || d.gate.skip_on_error && self.has_error
                || d.found.iter().any(|f| f.severity() == Severity::Error)
            {
                continue;
            }
            if let Some(Some(k)) = self.key_of(id, d) {
                keys.insert(k);
            }
        }
        let keys: Vec<Digest> = keys.into_iter().collect();
        let store = self.store;
        let got = parallel(self.opts.threads, &keys, |k| store.get_key(k));
        if let Some(got) = got {
            self.keys_read.extend(keys.into_iter().zip(got));
        }
    }

    /// Reads and checks the stored objects a batch's inputs will be materialized from, on several
    /// threads (section 8): those of kept nodes whose value is not in memory.
    fn read_objects_ahead(&mut self, batch: &[NodeId]) {
        let mut want: BTreeSet<Digest> = BTreeSet::new();
        for id in batch {
            let Some(d) = self.get(id).and_then(|s| s.def.as_ref()) else {
                continue;
            };
            for (_, t) in &d.edges {
                let Some(ts) = self.get(t) else {
                    continue;
                };
                if ts.value.is_some() || !ts.usable() {
                    continue;
                }
                let kept = ts.def.as_ref().is_some_and(|d| d.step.is_some())
                    || self.plan.persist(t) == Persist::Kept;
                if let (true, Some(vd)) = (kept, ts.vd) {
                    want.insert(vd);
                }
            }
        }
        let want: Vec<Digest> = want.into_iter().collect();
        let store = self.store;
        let got = parallel(self.opts.threads, &want, |d| store.get_object(d));
        if let Some(got) = got {
            self.objects_read.extend(want.into_iter().zip(got));
        }
    }

    /// A node the static checks failed (`Def::found`): keyed by its statements, parameters and
    /// errors alone, reused when that key is stored, else settled as `ran` with the errors as its
    /// value's diagnostics.
    fn settle_found(&mut self, id: &NodeId, def: &Def) -> bool {
        let found = self.plan.codec().encode(&def.found);
        let marker = match &found {
            Some(b) => Digest::of(b),
            None => Digest::of(format!("unstorable {:?}", def.found).as_bytes()),
        };
        let k = key(
            &self.opts.versions,
            &KeyParts {
                family: id.family().to_string(),
                statements: def.statements.iter().map(|(_, d)| *d).collect(),
                params: def.params.clone(),
                edges: vec![("static errors".to_string(), marker)],
            },
        );
        if let Some(s) = self.st.get_mut(id) {
            s.key = Some(k);
        }
        if let Some(rec) = self.store.get_key(&k) {
            if let Some(diags) = self.plan.codec().decode(&rec.diags) {
                if let Some(s) = self.st.get_mut(id) {
                    s.failed = has_error(&diags);
                    s.vd = Some(rec.value);
                    s.diags = Some(Arc::new(diags));
                    s.status = Some(Status::Reused);
                }
                return true;
            }
        }
        let out = Output::of(Val {
            bytes: Arc::new(Vec::new()),
            diags: Arc::new(def.found.clone()),
        });
        self.settle(id, out, 0, Settle::Fresh);
        true
    }

    /// The newest trace under the static key whose every read answers the same again.
    fn find_trace(&self, id: &NodeId, static_key: &Digest) -> Option<Found> {
        find_trace(self.plan, self.store, id, static_key)
    }

    /// The value of a settled, usable node: from memory, from its object, or computed again.
    fn materialize(&mut self, id: &NodeId, by: Option<NodeId>) -> Option<Val> {
        let s = self.st.get_mut(id)?;
        s.needed.insert(by);
        if let Some(v) = &s.value {
            return Some(v.clone());
        }
        if !s.usable() {
            return None;
        }
        let vd = s.vd?;
        let diags = s.diags.clone().unwrap_or_default();
        let kept = s.def.as_ref().is_some_and(|d| d.step.is_some())
            || self.plan.persist(id) == Persist::Kept;
        if kept {
            let read = match self.objects_read.remove(&vd) {
                Some(r) => r,
                None => self.store.get_object(&vd),
            };
            if let Some(obj) = read {
                if let Some(v) = decode_object(&obj, &diags, self.plan.codec()) {
                    if let Some(s) = self.st.get_mut(id) {
                        s.value = Some(v.clone());
                    }
                    return Some(v);
                }
            }
            // The object is missing or damaged: the node runs again (cause `not stored`).
            let out = self.compute_now(id)?;
            self.settle(id, out.0, out.1, Settle::Restore);
        } else {
            let out = self.compute_now(id)?;
            self.settle(id, out.0, out.1, Settle::Recompute);
        }
        self.st.get(id).and_then(|s| s.value.clone())
    }

    fn inputs_of(&mut self, id: &NodeId) -> Option<Vec<(String, NodeId, Val)>> {
        let def = self.st.get(id)?.def.clone()?;
        let mut out = Vec::with_capacity(def.edges.len());
        for (l, t) in &def.edges {
            let v = self.materialize(t, Some(id.clone()))?;
            out.push((l.clone(), t.clone(), v));
        }
        Some(out)
    }

    fn compute_now(&mut self, id: &NodeId) -> Option<(Output, u64)> {
        let inputs = self.inputs_of(id)?;
        let def = self.st.get(id)?.def.clone()?;
        let t = Instant::now();
        let out = self.plan.compute(
            id,
            &def,
            &Inputs {
                edges: &inputs,
                world: self.world.as_deref(),
            },
        );
        Some((out, elapsed_ms(t)))
    }

    fn run_batch(&mut self, batch: Vec<NodeId>) {
        // One computation per key: the lowest id computes, the others take its result.
        let mut first: BTreeMap<Digest, NodeId> = BTreeMap::new();
        let mut copies: Vec<(NodeId, NodeId)> = Vec::new();
        let mut jobs: Vec<Job> = Vec::new();
        self.read_objects_ahead(&batch);
        for id in batch {
            let Some(k) = self.get(&id).and_then(|s| s.key) else {
                continue;
            };
            if let Some(f) = first.get(&k) {
                copies.push((id, f.clone()));
                continue;
            }
            first.insert(k, id.clone());
            match self.inputs_of(&id) {
                Some(inputs) => {
                    let def = self
                        .get(&id)
                        .and_then(|s| s.def.clone())
                        .unwrap_or_default();
                    jobs.push((id, def, inputs));
                }
                None => self.skip(&id),
            }
        }
        self.objects_read.clear();
        let outs = self.compute_all(&jobs);
        for ((id, _, _), (out, ms)) in jobs.iter().zip(outs) {
            self.settle(id, out, ms, Settle::Fresh);
        }
        for (id, from) in copies {
            let src = self.get(&from).map(|s| {
                (
                    s.status,
                    s.vd,
                    s.diags.clone(),
                    s.value.clone(),
                    s.failed,
                    s.step.clone(),
                )
            });
            if let (Some(s), Some((status, vd, diags, value, failed, step))) =
                (self.st.get_mut(&id), src)
            {
                s.status = status.map(|st| {
                    if st == Status::Ran {
                        Status::Reused
                    } else {
                        st
                    }
                });
                s.vd = vd;
                s.diags = diags;
                s.value = value;
                s.failed = failed;
                if s.step.is_some() {
                    s.step = step;
                }
            }
        }
    }

    fn compute_all(&self, jobs: &[Job]) -> Vec<(Output, u64)> {
        let world = self.world.as_deref();
        let plan = self.plan;
        let one = |j: &Job| {
            let t = Instant::now();
            let out = plan.compute(&j.0, &j.1, &Inputs { edges: &j.2, world });
            (out, elapsed_ms(t))
        };
        let threads = self.opts.threads.max(1).min(jobs.len());
        if threads <= 1 {
            return jobs.iter().map(one).collect();
        }
        let next = AtomicUsize::new(0);
        let slots: Vec<Mutex<Option<(Output, u64)>>> =
            jobs.iter().map(|_| Mutex::new(None)).collect();
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= jobs.len() {
                        break;
                    }
                    let r = one(&jobs[i]);
                    if let Ok(mut slot) = slots[i].lock() {
                        *slot = Some(r);
                    }
                });
            }
        });
        slots
            .into_iter()
            .zip(jobs)
            .map(|(m, j)| m.into_inner().ok().flatten().unwrap_or_else(|| one(j)))
            .collect()
    }

    fn settle(&mut self, id: &NodeId, out: Output, ms: u64, how: Settle) {
        // A traced lint whose read reached a failed or skipped node: skipped, nothing kept.
        if out.skipped {
            self.skip(id);
            return;
        }
        let codec = self.plan.codec();
        let enc = encode_val(&out.value, codec);
        let Some(s) = self.st.get(id) else {
            return;
        };
        let key = s.key;
        let is_step = s.def.as_ref().is_some_and(|d| d.step.is_some());
        let kept = is_step || self.plan.persist(id) == Persist::Kept;
        let rerun = s.rerun;
        let matched = s.matched;
        let step_id = s
            .def
            .as_ref()
            .and_then(|d| d.step.as_ref())
            .map(|sd| sd.step.clone());
        // A value computed again must give the digest its readers were keyed on. One that does
        // not is a defect of the node's computation (it reads something that is not an edge):
        // the node fails, so no reader takes bytes other than those its key names.
        if out.complete && how != Settle::Fresh && is_step {
            // A step whose object was lost ran again: what it gives is the step's (5.2).
            if let Some(old) = s.vd.filter(|old| *old != enc.digest) {
                self.run_diags.push(unstable(
                    step_id.as_deref().unwrap_or(""),
                    &old,
                    &enc.digest,
                ));
            }
        } else if out.complete && how != Settle::Fresh {
            if let Some(old) = s.vd.filter(|old| *old != enc.digest) {
                let message = format!(
                    "`{id}` gave another value when computed again: {old} then {}",
                    enc.digest
                );
                self.defects.push(
                    Diag::new(
                        codes::diag::INTERNAL,
                        Subject::World,
                        Site::World,
                        text(format!(
                            "a node of the build graph is not stable: {message}"
                        )),
                        via_diag::text!(
                            "report it: this is a defect of the engine, not of the text"
                        ),
                    )
                    .with("message", message),
                );
                self.has_error = true;
                if let Some(s) = self.st.get_mut(id) {
                    s.ms = ms;
                    s.phase = Phase::Defined;
                    s.failed = true;
                    s.vd = None;
                    s.value = None;
                    s.diags = Some(out.value.diags.clone());
                    s.status = Some(if how == Settle::Recompute {
                        Status::Recomputed
                    } else {
                        Status::Ran
                    });
                    s.not_stored = how == Settle::Restore;
                }
                return;
            }
        }
        let mut trace = None;
        if out.complete && how != Settle::Recompute {
            if let Some(k) = key {
                if kept {
                    self.store.put_object(&enc.object);
                }
                if let Some(diags) = &enc.diags {
                    if is_step {
                        let run = out.step.clone().unwrap_or_default();
                        let entry = TraceEntry {
                            reads: run.reads.clone(),
                            data: run.data.clone(),
                            value: enc.digest,
                            diags: diags.clone(),
                            files: run.files.clone(),
                            log: run.log,
                        };
                        let td = trace_digest(&run.reads);
                        if rerun {
                            // `--rerun-step` (5.2): the new value is used, now and later.
                            self.store.replace_trace(&k, &td, &entry.encode());
                        } else {
                            self.store.put_trace(&k, &td, &entry.encode());
                        }
                        trace = (!run.reads.is_empty()).then_some(td);
                    } else {
                        self.store.put_key(
                            &k,
                            &KeyRecord {
                                value: enc.digest,
                                diags: diags.clone(),
                            },
                        );
                    }
                }
            }
        }
        if out.complete && rerun {
            if let Some(old) = matched {
                if old != enc.digest {
                    self.run_diags.push(unstable(
                        step_id.as_deref().unwrap_or(""),
                        &old,
                        &enc.digest,
                    ));
                }
            }
        }
        let Some(s) = self.st.get_mut(id) else {
            return;
        };
        s.ms = ms;
        s.phase = Phase::Defined;
        s.diags = Some(out.value.diags.clone());
        if out.complete {
            s.failed = out.value.has_error();
            s.vd = Some(enc.digest);
            s.value = Some(out.value);
        } else {
            s.failed = true;
            s.vd = None;
            s.value = None;
        }
        s.status = Some(match how {
            Settle::Recompute => Status::Recomputed,
            Settle::Fresh | Settle::Restore => Status::Ran,
        });
        s.created = how == Settle::Fresh;
        s.not_stored = how == Settle::Restore;
        if let (Some(so), Some(run)) = (s.step.as_mut(), out.step) {
            so.trace = trace;
            so.exit = run.exit;
            so.log = run.log;
            so.returned = s.vd.map(|vd| (vd, run.files.clone()));
            so.reads = run.data.into_iter().chain(run.reads).collect();
        }
    }

    fn readers(&self) -> BTreeMap<NodeId, BTreeSet<NodeId>> {
        let mut r: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for (id, s) in &self.st {
            if let Some(d) = &s.def {
                for (_, t) in &d.edges {
                    r.entry(t.clone()).or_default().insert(id.clone());
                }
            }
            if let Phase::Need(ns) = &s.phase {
                for t in ns {
                    r.entry(t.clone()).or_default().insert(id.clone());
                }
            }
        }
        r
    }

    fn subject_of(&self, id: &NodeId) -> Option<&Subject> {
        self.st.get(id)?.def.as_ref()?.subject.as_ref()
    }

    /// What the build reports: each node's diagnostics by id, then the graph's own.
    fn world_diags(&self) -> Vec<Diag> {
        let mut out = Vec::new();
        for s in self.st.values() {
            if s.status == Some(Status::Skipped) {
                continue;
            }
            let Some(diags) = &s.diags else {
                continue;
            };
            let all = s.def.as_ref().is_none_or(|d| d.reports == Reports::All);
            out.extend(
                diags
                    .iter()
                    .filter(|d| all || d.severity() == Severity::Error)
                    .cloned(),
            );
        }
        // Who reads whom, made only when a cycle or a failed node is to be traced through it.
        let traced = !self.opts.cycles.is_empty()
            || self
                .st
                .values()
                .any(|s| s.failed && s.status != Some(Status::Skipped));
        let readers = if traced {
            self.readers()
        } else {
            BTreeMap::new()
        };
        let skipped_ids: BTreeSet<&NodeId> = self
            .st
            .iter()
            .filter(|(_, s)| s.status == Some(Status::Skipped))
            .map(|(id, _)| id)
            .collect();
        for (i, c) in self.opts.cycles.iter().enumerate() {
            out.push(c.diag.clone());
            let group: BTreeSet<NodeId> = self
                .st
                .keys()
                .filter(|id| self.blocked.get(&id.vertex()) == Some(&i))
                .cloned()
                .collect();
            let deps = poison::dependents(&readers, &group);
            let subjects = deps
                .iter()
                .filter(|d| skipped_ids.contains(d))
                .map(|d| self.subject_of(d));
            if let Some(d) = poison::skipped(c.diag.subject(), subjects) {
                out.push(d);
            }
        }
        for (id, s) in &self.st {
            if !s.failed || s.status == Some(Status::Skipped) {
                continue;
            }
            let first_error = s
                .diags
                .iter()
                .flat_map(|d| d.iter())
                .filter(|d| d.severity() == Severity::Error)
                .map(|d| d.subject().clone())
                .min_by_key(|s| s.to_string());
            let subject = first_error
                .or_else(|| self.subject_of(id).cloned())
                .unwrap_or(Subject::World);
            let deps = poison::dependents(&readers, &[id.clone()].into());
            let subjects = deps
                .iter()
                .filter(|d| skipped_ids.contains(d))
                .map(|d| self.subject_of(d));
            if let Some(d) = poison::skipped(&subject, subjects) {
                out.push(d);
            }
        }
        let gated_steps = self
            .st
            .values()
            .filter(|s| {
                s.status == Some(Status::Skipped)
                    && s.def.as_ref().is_some_and(|d| {
                        d.gate.skip_on_error && d.step.as_ref().is_some_and(|sd| !sd.lint)
                    })
                    && self.has_error
            })
            .count();
        if let Some(d) = poison::exports_skipped(u32::try_from(gated_steps).unwrap_or(u32::MAX)) {
            out.push(d);
        }
        out.extend(self.defects.iter().cloned());
        out
    }

    fn cause(&self, id: &NodeId, s: &St) -> Cause {
        if s.rerun {
            return Cause::Rerun;
        }
        let Some(prev) = &self.opts.previous else {
            return Cause::NoRecord;
        };
        if prev.versions != self.opts.versions {
            return Cause::Versions;
        }
        let Some(pn) = prev.nodes.get(id) else {
            return Cause::NewNode;
        };
        let def = s.def.clone().unwrap_or_default();
        if def
            .statements
            .iter()
            .any(|(sid, d)| prev.statements.get(sid) != Some(d))
        {
            return Cause::Statements;
        }
        let mut edges: Vec<(&String, &NodeId)> = def.edges.iter().map(|(l, t)| (l, t)).collect();
        edges.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()).then(a.1.cmp(b.1)));
        for (l, t) in edges {
            let now = self.st.get(t).and_then(|ts| ts.vd);
            let before = prev.nodes.get(t).and_then(|n| n.value);
            if now != before || before.is_none() {
                return Cause::Edge {
                    label: l.clone(),
                    node: t.clone(),
                };
            }
        }
        let mut direct: Vec<&(String, Digest)> = def.direct.iter().collect();
        direct.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()).then(a.1.cmp(&b.1)));
        for (l, d) in direct {
            if !pn.direct.iter().any(|(pl, pd)| pl == l && pd == d) {
                return Cause::Edge {
                    label: l.clone(),
                    node: id.clone(),
                };
            }
        }
        if pn.key != s.key {
            return Cause::Key;
        }
        // A step's trace that no longer matches (5.1): its first read whose answer differs, in
        // the trace the previous record names for this node; when it names none, in the first
        // other stored trace by name. The trace this run has just added is never the one.
        if def.step.is_some() {
            if let Some(k) = s.key {
                let mine = s.step.as_ref().and_then(|so| so.trace);
                let named = prev.steps.get(id).and_then(|so| so.trace);
                let mut stored = self.store.traces(&k);
                stored.sort_by(|a, b| a.0.hex().cmp(&b.0.hex()));
                let differs = |reads: &[Read]| {
                    reads
                        .iter()
                        .find(|r| self.plan.answer(id, r) != Some(r.answer))
                        .map(|r| Cause::Trace {
                            method: r.method.clone(),
                            request: r.request,
                        })
                };
                if let Some(td) = named {
                    let entry = stored
                        .iter()
                        .find(|(n, _)| *n == td)
                        .and_then(|(_, b)| TraceEntry::decode(b));
                    let found = match entry {
                        Some(e) => differs(&e.reads),
                        None => prev.steps.get(id).and_then(|so| {
                            let reads: Vec<Read> = so
                                .reads
                                .iter()
                                .filter(|r| r.method != "data")
                                .cloned()
                                .collect();
                            differs(&reads)
                        }),
                    };
                    if let Some(c) = found {
                        return c;
                    }
                } else {
                    for (name, bytes) in &stored {
                        if Some(*name) == mine {
                            continue;
                        }
                        let Some(entry) = TraceEntry::decode(bytes) else {
                            continue;
                        };
                        if let Some(c) = differs(&entry.reads) {
                            return c;
                        }
                    }
                }
            }
        }
        Cause::NotStored
    }

    fn finish(mut self) -> Run {
        // Equal keys: the lowest id carries `ran`, the others `reused` (6.1).
        let mut groups: BTreeMap<Digest, Vec<NodeId>> = BTreeMap::new();
        let created: BTreeSet<Digest> = self
            .st
            .values()
            .filter(|s| s.created)
            .filter_map(|s| s.key)
            .collect();
        for (id, s) in &self.st {
            if let (Some(k), Some(Status::Ran | Status::Reused)) = (s.key, s.status) {
                if created.contains(&k) && !s.not_stored {
                    groups.entry(k).or_default().push(id.clone());
                }
            }
        }
        for ids in groups.values() {
            for (i, id) in ids.iter().enumerate() {
                if let Some(s) = self.st.get_mut(id) {
                    s.status = Some(if i == 0 { Status::Ran } else { Status::Reused });
                }
            }
        }
        if self.world.is_none() {
            self.has_error |= self.st.values().any(|s| s.failed);
        }
        let world_diags = self.world_diags();
        let has_error =
            self.has_error || world_diags.iter().any(|d| d.severity() == Severity::Error);
        let mut run_diags = self.store.take_run_diags();
        run_diags.append(&mut self.run_diags);
        let complete = self
            .st
            .values()
            .filter(|s| s.def.as_ref().is_some_and(|d| d.step.is_some()))
            .all(|s| s.status != Some(Status::Ran) || s.vd.is_some());

        let mut nodes = BTreeMap::new();
        for (id, s) in &self.st {
            let status = s.status.unwrap_or(Status::Skipped);
            let cause = (status == Status::Ran).then(|| self.cause(id, s));
            let needed_by = (status == Status::Recomputed).then(|| {
                s.needed
                    .iter()
                    .flatten()
                    .filter(|r| {
                        self.st.get(*r).is_some_and(|rs| {
                            matches!(rs.status, Some(Status::Ran | Status::Recomputed))
                        })
                    })
                    .min()
                    .cloned()
                    .map_or(NeededBy::Expansion, NeededBy::Node)
            });
            let value = if status == Status::Skipped {
                None
            } else {
                s.vd
            };
            nodes.insert(
                id.clone(),
                NodeOutcome {
                    status,
                    key: s.key,
                    value,
                    failed: s.failed,
                    cause,
                    needed_by,
                    ms: s.ms,
                    step: s.step.clone(),
                },
            );
        }
        let mut statements = BTreeMap::new();
        let mut direct = BTreeMap::new();
        let mut values = BTreeMap::new();
        let mut diags = BTreeMap::new();
        let mut edges = BTreeMap::new();
        let mut made_from = BTreeMap::new();
        for (id, s) in self.st {
            if let Some(d) = &s.def {
                edges.insert(id.clone(), d.edges.clone());
                made_from.insert(
                    id.clone(),
                    d.statements.iter().map(|(sid, _)| sid.clone()).collect(),
                );
                for (sid, dg) in &d.statements {
                    statements.insert(sid.clone(), *dg);
                }
                if !d.direct.is_empty() {
                    direct.insert(id.clone(), d.direct.clone());
                }
            }
            if let Some(v) = s.value {
                values.insert(id.clone(), v);
            }
            if let Some(d) = s.diags {
                diags.insert(id, d);
            }
        }
        Run {
            nodes,
            world_diags,
            run_diags,
            complete,
            has_error,
            values,
            statements,
            direct,
            diags,
            edges,
            made_from,
        }
    }
}

/// The newest trace under the static key whose every read answers the same again.
fn find_trace(plan: &dyn Plan, store: &Store, id: &NodeId, static_key: &Digest) -> Option<Found> {
    for (name, bytes) in store.traces(static_key) {
        let Some(entry) = TraceEntry::decode(&bytes) else {
            store.drop_trace(static_key, &name, "is not a trace entry");
            continue;
        };
        if trace_digest(&entry.reads) != name {
            store.drop_trace(static_key, &name, "does not match its name");
            continue;
        }
        let Some(diags) = plan.codec().decode(&entry.diags) else {
            store.drop_trace(static_key, &name, "holds diagnostics that cannot be read");
            continue;
        };
        if entry
            .reads
            .iter()
            .all(|r| plan.answer(id, r) == Some(r.answer))
        {
            return Some((name, entry, diags));
        }
    }
    None
}

fn has_error(diags: &[Diag]) -> bool {
    diags.iter().any(|d| d.severity() == Severity::Error)
}

/// `f` of each item on up to `threads` threads, in the items' order; `None` when one thread or
/// fewer than two items leave nothing to share (the caller then asks one by one).
fn parallel<T: Sync, R: Send>(
    threads: usize,
    items: &[T],
    f: impl Fn(&T) -> R + Sync,
) -> Option<Vec<R>> {
    let threads = threads.min(items.len());
    if threads <= 1 {
        return None;
    }
    let next = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let r = f(&items[i]);
                if let Ok(mut slot) = slots[i].lock() {
                    *slot = Some(r);
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|m| m.into_inner().ok().flatten())
        .collect()
}

fn elapsed_ms(t: Instant) -> u64 {
    u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn text(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| via_diag::text!("a node of the build graph"))
}

fn unstable(step: &str, old: &Digest, new: &Digest) -> Diag {
    Diag::new(
        codes::graph::STEP_UNSTABLE,
        Subject::entity("step", step),
        Site::Header,
        text(format!(
            "step `{step}` gave another value for a trace it had before: {old} then {new}"
        )),
        via_diag::text!(
            "the step reads something it does not declare (a file, the clock, the network); declare it in `inputs`, or remove the read"
        ),
    )
}

/// The digest of a trace: the reads in the order made (5.1).
pub fn trace_digest(reads: &[Read]) -> Digest {
    let mut e = Enc::new();
    e.str("via-trace").len(reads.len());
    for r in reads {
        e.str(&r.method).digest(&r.request).digest(&r.answer);
    }
    Digest::of(&e.finish())
}

/// One trace in the store: the reads, the value they gave, its diagnostics, its files and log.
struct TraceEntry {
    reads: Vec<Read>,
    data: Option<Read>,
    value: Digest,
    diags: Vec<u8>,
    files: Vec<(String, Digest)>,
    log: Option<Digest>,
}

impl TraceEntry {
    fn encode(&self) -> Vec<u8> {
        let mut e = Enc::new();
        e.str("via-trace-entry-1").len(self.reads.len());
        for r in &self.reads {
            e.str(&r.method).digest(&r.request).digest(&r.answer);
        }
        match &self.data {
            None => e.byte(0),
            Some(r) => e
                .byte(1)
                .str(&r.method)
                .digest(&r.request)
                .digest(&r.answer),
        };
        e.digest(&self.value)
            .bytes(&self.diags)
            .len(self.files.len());
        for (p, d) in &self.files {
            e.str(p).digest(d);
        }
        match &self.log {
            None => e.byte(0),
            Some(d) => e.byte(1).digest(d),
        };
        e.finish()
    }

    fn decode(b: &[u8]) -> Option<TraceEntry> {
        let mut d = Dec::new(b);
        if d.str()? != "via-trace-entry-1" {
            return None;
        }
        let mut reads = Vec::new();
        for _ in 0..d.count()? {
            reads.push(Read {
                method: d.str()?,
                request: d.digest()?,
                answer: d.digest()?,
            });
        }
        let data = match d.byte()? {
            0 => None,
            1 => Some(Read {
                method: d.str()?,
                request: d.digest()?,
                answer: d.digest()?,
            }),
            _ => return None,
        };
        let value = d.digest()?;
        let diags = d.bytes()?.to_vec();
        let mut files = Vec::new();
        for _ in 0..d.count()? {
            files.push((d.str()?, d.digest()?));
        }
        let log = match d.byte()? {
            0 => None,
            1 => Some(d.digest()?),
            _ => return None,
        };
        d.done().then_some(TraceEntry {
            reads,
            data,
            value,
            diags,
            files,
            log,
        })
    }
}
