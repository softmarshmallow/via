//! What the graph asks of the node kinds (`spec/graph.md` 1 and 2).
//!
//! The graph knows no node kind. A [`Plan`] says, for each node id, what it is made from (its
//! statements, parameters and input edges) and how to compute its value from its inputs; the
//! graph does the rest: keys, the store, reuse, order, skipping and the run report. A node reads
//! only its inputs, which arrive along its edges: [`Plan::compute`] is handed them and nothing
//! else.

use std::collections::BTreeMap;

use via_diag::{Diag, Subject};

use crate::codec::{CoreCodec, DiagCodec, Val};
use crate::digest::Digest;
use crate::id::NodeId;

/// Whether a node's value is kept in the store or only its digest (1, "Kept in the store").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Persist {
    /// The value's bytes are an object in the store.
    Kept,
    /// Only the key record; the value is computed again when a consumer needs it.
    DigestOnly,
}

/// The line between kept and digest only, per family, as section 1's table fixes it for this
/// build format. A family the table does not list is kept.
pub fn default_persist(family: &str) -> Persist {
    match family {
        "source" | "input" | "attr" | "select" | "shape" | "instance" | "rule" | "edit"
        | "held" | "footprint" | "height" | "facts" => Persist::DigestOnly,
        _ => Persist::Kept,
    }
}

/// Which of a node's diagnostics the build reports (3.4, "Only the build reports").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Reports {
    /// Every diagnostic: a node of the build's own view.
    #[default]
    All,
    /// Errors only: a node of any other view, since an error fails its reader.
    ErrorsOnly,
}

/// When a node may run, beyond its edges (`diagnostics.md` 4 and graph.md 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Gate {
    /// Skipped when the build has any error (export steps, `write` nodes but the diagnostics
    /// file). Such a node waits until every ungated node is settled.
    pub skip_on_error: bool,
}

/// A step node's facts for the run report (6.3, the step object).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StepDef {
    /// The step's id, as `--rerun-step` names it.
    pub step: String,
    /// The command as the step's statement writes it.
    pub command: Vec<String>,
}

/// What a node is made from (3.1): everything its key holds besides the versions.
#[derive(Clone, Debug, Default)]
pub struct Def {
    /// The statement this node belongs to, for skipped lists (`diag.skipped`); `None` for a
    /// node made from no statement (`merge`, a `write`).
    pub subject: Option<Subject>,
    /// The statements it is made from, as (statement id, digest): kinds, the pack and inlined
    /// `let` statements included. The ids give the run report's `statements` cause.
    pub statements: Vec<(String, Digest)>,
    /// Its own parameters: window, item, operation arguments, path.
    pub params: Vec<(String, String)>,
    /// Its input edges, as (label, input node).
    pub edges: Vec<(String, NodeId)>,
    /// Digests that enter as edges with no node behind them: a leaf's file digest (`bytes`).
    pub direct: Vec<(String, Digest)>,
    /// Which of its diagnostics the build reports.
    pub reports: Reports,
    /// Its gate.
    pub gate: Gate,
    /// Set for a `step` node: its key is then the static key (5.1), and its results are traces.
    pub step: Option<StepDef>,
}

/// The answer to [`Plan::define`].
pub enum Defined {
    /// The node is made from this.
    Def(Def),
    /// Its edges depend on these nodes' values (2.1, the expansion of the node graph): ask
    /// again once they are known.
    Need(Vec<NodeId>),
    /// Ask again after every other node is settled, with the build's world diagnostics known
    /// (`write:diagnostics.json`, `write:manifest.json`).
    Last,
}

/// One read a step made (5.1, the trace): the engine call and the digests of the request and
/// of the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// The engine call; `data` for a plain command's one read.
    pub method: String,
    /// The digest of the request's canonical JSON.
    pub request: Digest,
    /// The digest of the answer's canonical JSON.
    pub answer: Digest,
}

/// What a step's run gave, beyond its value.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StepRun {
    /// Each read in the order made: the trace, replayed for reuse (5.1).
    pub reads: Vec<Read>,
    /// A plain command's one read, method `data`: its request without absolute paths. It is
    /// shown in the run report's `reads` but is no part of the trace, since the request is in
    /// the static key already.
    pub data: Option<Read>,
    /// The process's exit status, when it ran and was not killed by a signal.
    pub exit: Option<i64>,
    /// Each file of `out` as (path, digest), sorted by path.
    pub files: Vec<(String, Digest)>,
    /// The digest of its log in the store.
    pub log: Option<Digest>,
}

/// A computed value.
#[derive(Clone, Debug)]
pub struct Output {
    /// The value and its diagnostics.
    pub value: Val,
    /// False for a step that did not complete (failed, killed, nothing readable): it is not
    /// stored, has no value digest, fails its readers, and runs again next build.
    pub complete: bool,
    /// For a step node: its reads and returns.
    pub step: Option<StepRun>,
}

impl Output {
    /// A complete value.
    pub fn of(value: Val) -> Output {
        Output {
            value,
            complete: true,
            step: None,
        }
    }
}

/// The values a node's definition may look at: those it asked for with [`Defined::Need`], and,
/// for a node defined last, the build's world diagnostics.
pub struct Known<'a> {
    pub(crate) values: &'a BTreeMap<NodeId, Val>,
    pub(crate) world: Option<&'a [Diag]>,
    pub(crate) has_error: bool,
}

impl Known<'_> {
    /// A needed node's value.
    pub fn value(&self, id: &NodeId) -> Option<&Val> {
        self.values.get(id)
    }

    /// The build's world diagnostics, for a node defined last; `None` otherwise.
    pub fn world_diags(&self) -> Option<&[Diag]> {
        self.world
    }

    /// For a node defined last: whether the build has any error.
    pub fn has_error(&self) -> bool {
        self.has_error
    }
}

/// A node's inputs, in the order of its edges.
pub struct Inputs<'a> {
    pub(crate) edges: &'a [(String, NodeId, Val)],
    pub(crate) world: Option<&'a [Diag]>,
}

impl Inputs<'_> {
    /// The first input with this label.
    pub fn get(&self, label: &str) -> Option<&Val> {
        self.edges
            .iter()
            .find(|(l, _, _)| l == label)
            .map(|(_, _, v)| v)
    }

    /// Every input as (label, node, value), in the order of the definition's edges.
    pub fn all(&self) -> &[(String, NodeId, Val)] {
        self.edges
    }

    /// The build's world diagnostics, for a node defined last.
    pub fn world_diags(&self) -> Option<&[Diag]> {
        self.world
    }
}

/// The node kinds, as the model crate defines them.
pub trait Plan: Sync {
    /// What the node is made from. Called on the coordinating thread, in id order.
    fn define(&self, id: &NodeId, known: &Known<'_>) -> Defined;

    /// The node's value from its inputs. Called on any thread; must depend on nothing but `def`
    /// and `inputs`, so that a value is a function of its key (section 7).
    fn compute(&self, id: &NodeId, def: &Def, inputs: &Inputs<'_>) -> Output;

    /// Kept or digest only; by default section 1's table.
    fn persist(&self, id: &NodeId) -> Persist {
        default_persist(id.family())
    }

    /// How diagnostics are stored.
    fn codec(&self) -> &dyn DiagCodec {
        &CoreCodec
    }

    /// Answers one recorded read of a step again from the current graph, for trace reuse
    /// (5.1); `None` when it cannot, which fails the trace. A plain command's trace is empty,
    /// so M1 needs no answer.
    fn answer(&self, _step: &NodeId, _read: &Read) -> Option<Digest> {
        None
    }
}
