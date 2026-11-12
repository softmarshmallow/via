//! The geometry kernel (R19d).
//!
//! Responsibility: geometry on the millimetre lattice: noding, networks and the areas between
//! their paths, cells along a frontage, a path that follows another feature, masked regions
//! that keep every piece, and the nearest-point frame `d`, `sd`, `t`. Exact integer predicates
//! come first so that a moved feature renames nothing it did not touch.
//!
//! Governed by `spec/geometry.md`.
