//! Fields on the post lattice.
//!
//! Responsibility: the values of fields at posts: bases, `then` rows and effects evaluated by
//! window, raster files read as inputs, and searches over posts (a least-cost course). It works
//! by window so a local change re-runs a local region, and a window equals the same cut of the
//! whole world.
//!
//! Governed by `spec/fields.md`.
