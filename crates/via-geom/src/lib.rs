//! The geometry kernel (R19d).
//!
//! Responsibility: geometry on the millimetre lattice: noding, networks and the areas between
//! their paths, cells along a frontage, a path that follows another feature, masked regions
//! that keep every piece, and the nearest-point frame `d`, `sd`, `t`. Exact integer predicates
//! come first so that a moved feature renames nothing it did not touch.
//!
//! Governed by `spec/geometry.md`, and `spec/identity.md` for the names of what it makes.
//!
//! # Map
//!
//! | Module | Chapter |
//! |---|---|
//! | [`pred`] | 1, exact predicates |
//! | [`shape`] | 2.2, 2.3, 2.5: shapes, reading, canonical form |
//! | [`curve`] | 2.4, `smooth` |
//! | [`lineref`] | 2.6, 3.8: the local frame and linear referencing |
//! | [`noding`] | 3.1, iterated snap rounding with provenance |
//! | [`arrange`] | the planar arrangement every later operation reads |
//! | [`network`] | 3.2, junctions, edges, areas, representative points |
//! | [`overlay`] | 3.3, region booleans |
//! | [`offset`] | 3.4, offset and stroke |
//! | [`follow`] | 3.5, a path along another feature |
//! | [`cells`] | 3.6, cells along a frontage |
//! | [`mask`] | 3.7, mask to regions |
//! | [`triangulate`] | 3.10, triangulation for export |
//! | [`names`] | identity.md 2: place keys of junctions, edges, areas, cells |
//! | [`held`] | identity.md 4: pins and clears on a producer's instances |
//! | [`kernel`] | 7, the `via kernel` command's files |

// Parallel arrays indexed by vertex, half-edge and cycle are the arrangement's form; an index loop
// over them reads plainer than zipped iterators.
#![allow(clippy::needless_range_loop)]

pub mod arrange;
pub mod cells;
pub mod curve;
pub mod diag;
pub mod follow;
pub mod held;
pub mod json;
pub mod kernel;
pub mod lineref;
pub mod mask;
pub mod names;
pub mod network;
pub mod noding;
pub mod offset;
pub mod overlay;
pub mod pred;
pub mod shape;
pub mod triangulate;

pub use diag::{Code, GeomDiag};
pub use shape::{Piece, Region, P};
