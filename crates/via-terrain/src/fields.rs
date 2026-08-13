//! Boundary-condition fields: uplift and the initial surface. These are the
//! only exogenous inputs of the stage (README: the causal chain is rooted at
//! uplift/climate; precipitation becomes a field of its own in M2).

use rayon::prelude::*;

use crate::config::TerrainConfig;
use crate::grid::Grid;
use crate::noise;
use via_artifact::seed;

#[inline]
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Uplift rate field (m/yr). Signed: positive fBm regions rise, negative
/// regions subside, so the coastline is the emergent U ≈ 0 noise contour —
/// not the radial mask, which only guarantees the border ring stays ocean.
/// Domain warping breaks the isotropy of raw fBm; a finer octave carries
/// range-scale texture.
pub fn build_uplift(cfg: &TerrainConfig, grid: &Grid) -> Vec<f64> {
    let s_shape = seed::derive(cfg.seed, "terrain", 0);
    let s_warp = seed::derive(cfg.seed, "terrain", 3);
    let s_tex = seed::derive(cfg.seed, "terrain", 4);
    let (w, h) = (grid.w as f64, grid.h as f64);
    (0..grid.n() as u32)
        .into_par_iter()
        .map(|i| {
            let (x, y) = grid.xy(i);
            let u = (x as f64 + 0.5) / w;
            let v = (y as f64 + 0.5) / h;
            let rx = (u - 0.5) * 2.0;
            let ry = (v - 0.5) * 2.0;
            let r = (rx * rx + ry * ry).sqrt();
            // Hard guard: nothing survives near the border ring.
            let guard = 1.0 - smoothstep(0.85, 0.97, r);
            if guard <= 0.0 {
                return 0.0;
            }
            let wx = noise::fbm(s_warp, u * 2.0, v * 2.0, 3);
            let wy = noise::fbm(s_warp, u * 2.0 + 7.31, v * 2.0 + 3.77, 3);
            let (uu, vv) = (u + 0.18 * wx, v + 0.18 * wy);
            let broad = noise::fbm(s_shape, uu * 2.6, vv * 2.6, 4);
            let tex = noise::fbm(s_tex, uu * 6.0, vv * 6.0, 4);
            // fBm normalized by amplitude sum has σ ≈ 0.25; stretch it so
            // the U = 0 coastline contour cuts real bays out of the mask.
            let field = 0.72 * broad + 0.28 * tex;
            // The land/sea offset falls with radius: near the centre almost
            // any noise is land, near the rim only strong highs survive, so
            // the coastline wanders between radii instead of tracing one.
            let r_t = smoothstep(0.30, 0.95, r);
            let shaped = (guard * (2.8 * field + 0.60 - 1.25 * r_t)).clamp(-0.25, 1.0);
            cfg.uplift_max_m_per_yr * shaped
        })
        .collect()
}

/// Initial surface: a submerged plain with gentle fBm undulation and per-cell
/// jitter to break the symmetry the drainage pattern grows from.
pub fn initial_heights(cfg: &TerrainConfig, grid: &Grid) -> Vec<f64> {
    let s_fbm = seed::derive(cfg.seed, "terrain", 1);
    let s_jit = seed::derive(cfg.seed, "terrain", 2);
    let (w, h) = (grid.w as f64, grid.h as f64);
    (0..grid.n() as u32)
        .into_par_iter()
        .map(|i| {
            let (x, y) = grid.xy(i);
            let u = (x as f64 + 0.5) / w;
            let v = (y as f64 + 0.5) / h;
            cfg.base_depth_m
                + 3.0 * noise::fbm(s_fbm, u * 6.0, v * 6.0, 4)
                + 0.05 * noise::cell_jitter(s_jit, i)
        })
        .collect()
}
