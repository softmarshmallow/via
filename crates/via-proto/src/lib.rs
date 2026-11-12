//! The engine side of the step protocol (`via-step-1`).
//!
//! Responsibility: running a step's process, the request and the reply, the data plane for
//! arrays, callbacks into the engine, and trust. A step is another process (R19, R20), so this
//! crate is the only door between the engine and user code.
//!
//! Governed by `spec/protocol.md`.
//!
//! What is here now is the M0 probe of that chapter (its "What arrives when" table): sections 3.1
//! (the wire), 4.1 and 4.2 (blobs and field files) and 6.2 (lifecycle, process trees), enough to
//! send a field out and back, answer a call back and survive a killed worker. It runs from its own
//! harness (`examples/probe.rs`, `tests/probe.rs`), not from the parser. The M2 implementation
//! replaces the parts marked as probe-only; the framing, the store and the process tree are meant
//! to stay.

pub mod column;
pub mod diag;
pub mod frame;
pub mod npy;
pub mod probe;
pub mod process;
pub mod program;
pub mod store;
pub mod worker;

/// The one protocol version this engine speaks. Checked exactly, never negotiated (3.2).
pub const PROTOCOL: &str = "via-step-1";
