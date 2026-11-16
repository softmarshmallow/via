//! The build graph.
//!
//! Responsibility: every statement as a node and every reference as an edge, with one cycle
//! report; content keys; the write-once store; and running only what read a change, with the
//! run report saying why each node ran. A clean build must equal the incremental one, so the
//! key is the only thing that decides reuse.
//!
//! Governed by `spec/graph.md`.
//!
//! # Shape
//!
//! The crate knows no node kind. The model crate implements [`Plan`]: for each node id, what it
//! is made from ([`Def`]: statements, parameters, labelled edges) and how to compute its value
//! from the inputs handed to it. This crate does the rest:
//!
//! - [`RefGraph`]: the reference graph and its one `graph.cycle` per loop, in source order (2.2).
//! - [`key`]: the key of 3.1 with the four versions of 3.3; the vector is a unit test.
//! - [`Store`]: write-once objects, key records, traces and build records (3.2).
//! - [`run`]: keys from the leaves up, reuse, `recomputed` for values kept digest only,
//!   diagnostics replayed on a hit (3.4), skipping, gates, and parallel computation that never
//!   changes a byte or a status (8).
//! - [`BuildRecord`], [`build_key`] and [`null_build`] (6.2); [`report`] (6.3); [`causes`] (6.4);
//!   [`DirLock`] (8).

mod changes;
mod codec;
mod digest;
mod engine;
mod id;
mod key;
mod lock;
mod plan;
mod record;
mod refgraph;
mod report;
mod store;

#[cfg(test)]
mod tests;

pub use changes::{causes, causes_from, changed_statements, CauseGraph, ChangeCause};
pub use codec::{object_bytes, CoreCodec, DiagCodec, Val};
pub use digest::{Dec, Digest, Enc};
pub use engine::{
    run, trace_digest, Cause, NeededBy, NodeOutcome, Options, Run, Status, StepOutcome,
};
pub use id::{BadId, NodeId};
pub use key::{digest_of_parts, encode_key, key, KeyParts, Versions};
pub use lock::{lock_path, DirLock};
pub use plan::{
    default_persist, Def, Defined, Gate, Inputs, Known, Output, Persist, Plan, Read, Reports,
    Settled, StepDef, StepRun,
};
pub use record::{
    build_key, null_build, null_build_fresh, put_fresh, BuildKeyParts, BuildRecord, RecordNode,
};
pub use refgraph::{Cycle, Member, RefEdge, RefGraph, SourceOrder};
pub use report::{null_report, report, report_path, write_report, Json, ReportHead, RUN_FORMAT};
pub use store::{KeyRecord, Store, DEFAULT_STORE};
