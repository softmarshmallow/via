//! Typed artifacts: the only channel stages may communicate through.
//!
//! See ADR 0001 for the format. A run is a directory of `.vrast` binary
//! rasters plus JSON manifests; blake3 content hashes are the determinism
//! witness.

pub mod manifest;
pub mod raster;
pub mod seed;

pub use manifest::{ArtifactEntry, RunManifest};
pub use raster::{Raster, Scalar};
