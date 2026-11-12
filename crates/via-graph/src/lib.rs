//! The build graph.
//!
//! Responsibility: every statement as a node and every reference as an edge, with one cycle
//! report; content keys; the write-once store; and running only what read a change, with the
//! run report saying why each node ran. A clean build must equal the incremental one, so the
//! key is the only thing that decides reuse.
//!
//! Governed by `spec/graph.md`.
