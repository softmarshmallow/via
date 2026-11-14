//! The erosion library's seed derivation, carried over from the old workspace (`docs/plan.md`,
//! "Carried over"). The `.vrast` rasters and run manifests it also held are retired: a step returns
//! field files (`spec/protocol.md` 4.2), and the store names them by digest.
//!
//! The mixing function is `via-num`'s SplitMix64 (`spec/numbers.md` 7.2), so the whole library
//! draws from the one maths module (R26).

pub mod seed;
