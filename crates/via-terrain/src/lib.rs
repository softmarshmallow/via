//! The erosion library (R14): stream-power landscape evolution on a regular grid, run as a step.
//!
//! Model (`docs/model.md`): ∂h/∂t = U(x) − K·Q^½·S + κ∇²h + D, with the fluvial term solved by the
//! Braun & Willett (2013) implicit scheme over a weighted multiple-flow-direction graph, flow
//! routed over depressions filled by priority-flood + ε (Barnes et al. 2014), and sediment
//! deposited by the G-form of Davy & Lague (2009). m = 0.5, n = 1 are fixed (Q^½ is `sqrt`,
//! correctly rounded). Every other float operation goes through `via-num` (R26), so a run is the
//! same bits at any thread count and on every system.
//!
//! As a step (`src/bin/via-terrain.rs`, declared by `lib/terrain`): [`settle`] turns stated facts
//! into the model's starting conditions and hands the settled land back on the world's posts;
//! [`facts`] checks each fact against the finished build; [`wire`] speaks the step protocol. The
//! scale-dependent parameters come from the cell size ([`scale`]). The core knows none of this
//! library's words (principle 5): it is library tier, outside the deny-list (`vocabulary.md` 7).

pub mod climate;
pub mod config;
pub mod erosion;
pub mod extract;
pub mod facts;
pub mod fields;
pub mod flow;
pub mod gates;
pub mod grid;
pub mod noise;
pub mod raster;
pub mod scale;
pub mod sediment;
pub mod settle;
pub mod stage;
pub mod wire;

pub use config::TerrainConfig;
pub use grid::Grid;
pub use stage::{run, run_given, Given, TerrainOutput};
