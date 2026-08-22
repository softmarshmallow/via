//! Stage 4 — settlement systems (ADR 0013).
//!
//! Harris–Wilson allocation over the ADR 0012 corridor cost field,
//! integrated as a gradient flow in `x = ln W`. The population budget
//! is Tier-2 forcing (ADR 0013 Decision 3): the engine distributes it
//! and cannot supply it.

pub mod cost;
pub mod hw;
