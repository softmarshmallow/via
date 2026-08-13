//! Uplift, hillslope diffusion, and convergence metrics. The coupled
//! fluvial term (detachment + deposition) lives in `sediment` — one
//! implicit Gauss–Seidel solve since ADR 0006.

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
