//! Terrain stage: stream-power landscape evolution on a regular grid.
//!
//! Model: ∂h/∂t = U(x) − K·A^½·S + κ∇²h, with the fluvial term solved by the
//! Braun & Willett (2013) O(n) implicit scheme on the D8 receiver tree and
//! depressions removed by priority-flood + ε (Barnes et al. 2014).
//! m = 0.5, n = 1 are fixed in M1 (A^½ is `sqrt`, which is IEEE-exact and
//! therefore deterministic across platforms as well as across thread counts).

pub mod climate;
pub mod config;
pub mod erosion;
pub mod extract;
pub mod fields;
pub mod flow;
pub mod gates;
pub mod grid;
pub mod noise;
pub mod sediment;
pub mod stage;

pub use config::TerrainConfig;
pub use grid::Grid;
pub use stage::{run, write_run, TerrainOutput};
