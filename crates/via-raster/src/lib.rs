//! Fields on the post lattice.
//!
//! Responsibility: the values of fields at posts: bases, `then` rows and effects evaluated by
//! window, raster files read as inputs, and searches over posts (a least-cost course). It works
//! by window so a local change re-runs a local region, and a window equals the same cut of the
//! whole world.
//!
//! Governed by `spec/fields.md`, with `spec/numbers.md` 4 and 6 (posts, windows, halos),
//! `spec/geometry.md` 3.7 (mask to regions, the post half) and `spec/build.md` 3.2, 5 and 6 (array
//! files, chunks, weights bytes).
//!
//! # What is here, and what is not
//!
//! - [`window`]: windows, halos, an effect's window and ring (`fields.md` 5.3), chunks.
//! - [`raster`]: values at posts, narrowing to 32-bit floats once (1.1), reading between posts
//!   (1.2).
//! - [`rows`]: one rule for `then` rows and effects, `v = (1 - m) * v + m * value` (5.1), weights
//!   with any number of materials, held sparse (5.2, 6.3), `keep_out`'s maximum (2.1).
//! - [`slope`]: `slope`, halo one post (`formulas.md` 8.3).
//! - [`file`]: a base from an array file, `nearest` and `linear` (4.3). PNG and `cubic` are M2.
//! - [`distance`]: `dist` and `sdist` by window with the cap as halo; the measures themselves are
//!   the kernel's, through [`distance::Measured`].
//! - [`cover`]: exact rasterising of regions and strokes from integer millimetres.
//! - [`contour`]: mask to rings and pieces; the kernel settles them through [`contour::Settle`].
//! - [`npy`] and [`chunk`]: the array file and a weights chunk's bytes.
//!
//! This crate evaluates no formula: callers hand it closures that do. Its errors are values
//! ([`RasterError`]) that the caller locates with [`RasterError::diag`].

pub mod chunk;
pub mod contour;
pub mod cover;
pub mod distance;
mod error;
pub mod file;
pub mod npy;
pub mod raster;
pub mod rows;
pub mod slope;
pub mod window;

pub use error::{file_site, RasterError};
pub use raster::Raster;
pub use window::Window;
