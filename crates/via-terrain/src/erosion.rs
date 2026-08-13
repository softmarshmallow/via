//! The three terms of ∂h/∂t = U − K·A^½·S + κ∇²h.

use rayon::prelude::*;

use crate::grid::Grid;

/// h += U·dt (pure per-cell map; parallel-safe). Subsidence (U < 0) stops
/// at `floor_m`: without a floor, open-ocean cells with negative uplift
/// deepen linearly forever, and hillslope diffusion then drags every
/// subsidence-coast cell down each step — a permanent churn band that
/// poisons convergence at any resolution.
pub fn apply_uplift(h: &mut [f64], uplift: &[f64], dt: f64, floor_m: f64) {
    h.par_iter_mut()
        .zip(uplift.par_iter())
        .for_each(|(hi, &u)| {
            if u >= 0.0 {
                *hi += u * dt;
            } else if *hi > floor_m {
                *hi = (*hi + u * dt).max(floor_m);
            }
        });
}

/// Braun & Willett (2013) implicit stream-power solve with n = 1, m = ½,
/// generalized over the MFD DAG (ADR 0005): walking the topological order
/// (receivers first), each cell's new elevation is the closed form
///   h⁺ = (h + Σᵢ cᵢ·hrᵢ⁺) / (1 + Σᵢ cᵢ),   cᵢ = K·√Q·dt·wᵢ / distᵢ,
/// which relaxes h toward the weight-blend of its (already-updated)
/// receivers. One out-edge reduces this exactly to the single-receiver
/// update.
///
/// Standing water (ADR 0004): `flooded` and `water_level` are frozen from
/// the pre-erosion state of the step. The mask MUST be static — deciding
/// "submerged" by comparing the evolving surface against the routed copy
/// re-classifies every freshly incised cell as flooded, and the routing
/// sweep then refills exactly what detachment cut (observed: deposition ≈
/// 89% of detachment, landscape smears). Flooded cells do not incise (the
/// implicit form would otherwise *raise* a lake-bottom cell toward its
/// across-lake receiver, which is deposition by the wrong mechanism).
/// Rivers grade to water surfaces, not submerged beds: a flooded
/// receiver's effective elevation is its water level, and a base-level
/// receiver's is max(h_rcv, sea_level).
///
/// Returns the detached depth (m, ≥ 0) per cell — the sediment supply for
/// the routing sweep.
#[allow(clippy::too_many_arguments)]
pub fn erode_stream_power(
    grid: &Grid,
    h: &mut [f64],
    mfd: &crate::flow::MfdGraph,
    discharge_cells: &[f64],
    is_base: &[bool],
    flooded: &[bool],
    water_level: &[f64],
    k_spl: f64,
    dt: f64,
    sea_level: f64,
) -> Vec<f64> {
    let mut detached = vec![0.0f64; h.len()];
    for &i in &mfd.order {
        let iu = i as usize;
        if is_base[iu] || flooded[iu] {
            continue;
        }
        let sqrt_q_m = grid.dx * discharge_cells[iu].sqrt();
        let hi = h[iu];
        let mut num = hi;
        let mut den = 1.0f64;
        for (r, w, dist) in mfd.edges(i) {
            let ru = r as usize;
            let hr = if is_base[ru] {
                h[ru].max(sea_level)
            } else if flooded[ru] {
                water_level[ru]
            } else {
                h[ru]
            };
            if hi > hr {
                let c = k_spl * sqrt_q_m * dt * w / dist;
                num += c * hr;
                den += c;
            }
        }
        if den > 1.0 {
            let hn = num / den;
            detached[iu] = hi - hn;
            h[iu] = hn;
        }
    }
    detached
}

/// Explicit linear diffusion (4-neighbour Laplacian), subcycled to its
/// stability limit. Base cells are Dirichlet boundaries; edges are Neumann.
pub fn diffuse(grid: &Grid, h: &mut [f64], is_base: &[bool], kappa: f64, dt: f64) {
    let alpha = kappa * dt / (grid.dx * grid.dx);
    if alpha == 0.0 {
        return;
    }
    let cycles = (alpha / 0.24).ceil().max(1.0) as u32;
    let a = alpha / cycles as f64;
    let (w, hh) = (grid.w as i64, grid.h as i64);
    for _ in 0..cycles {
        let src = h.to_vec();
        h.par_iter_mut().enumerate().for_each(|(i, out)| {
            if is_base[i] {
                return;
            }
            let x = (i as u32 % grid.w) as i64;
            let y = (i as u32 / grid.w) as i64;
            let c = src[i];
            let mut lap = 0.0;
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx >= 0 && ny >= 0 && nx < w && ny < hh {
                    lap += src[(ny * w + nx) as usize] - c;
                }
            }
            *out = c + a * lap;
        });
    }
}

/// max |a − b| via parallel reduce. `max` is associative and exact, so the
/// result is independent of partition order.
pub fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.par_iter()
        .zip(b.par_iter())
        .map(|(x, y)| (x - y).abs())
        .reduce(|| 0.0, f64::max)
}

/// mean |a − b|, summed sequentially: the value reaches stats output, so
/// its accumulation order must not depend on thread scheduling.
pub fn mean_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    let sum: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    sum / a.len() as f64
}
