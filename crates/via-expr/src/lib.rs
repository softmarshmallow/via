//! Formulas.
//!
//! Responsibility: the formula language: parsing, the name, type, rate and halo checks made
//! before anything runs, and evaluation through `via-num`, with dispatch generated from the
//! function table (`spec/functions.toml`) so the evaluator, the reference page and the SDK's
//! list cannot drift apart.
//!
//! Governed by `spec/formulas.md`.
