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
///
/// `cum_applied_m` accumulates the uplift actually applied per cell —
/// exact bookkeeping for the material-frame lithology lookup (ADR 0007):
/// where the floor clamps, U·t would overstate the advection.
pub fn apply_uplift(
    h: &mut [f64],
    uplift: &[f64],
    dt: f64,
    floor_m: f64,
    cum_applied_m: &mut [f64],
) {
    h.par_iter_mut()
        .zip(uplift.par_iter())
        .zip(cum_applied_m.par_iter_mut())
        .for_each(|((hi, &u), cum)| {
            let before = *hi;
            if u >= 0.0 {
                *hi += u * dt;
            } else if *hi > floor_m {
                *hi = (*hi + u * dt).max(floor_m);
            }
            *cum += *hi - before;
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

/// Explicit diffusion with per-cell κ (ADR 0007): the flux form
/// ∇·(κ∇h) with the symmetric edge coefficient κᵢⱼ = ½(κᵢ + κⱼ). Every
/// edge exchange is pairwise antisymmetric in exact arithmetic, so mass
/// is conserved up to the final per-cell rounding of h + a·flux (the
/// same order of drift the uniform path carries), with the same
/// Dirichlet base / Neumann edge behavior. Subcycled to the max-κ
/// stability limit.
///
/// Callers with uniform κ must use [`diffuse`]: this function computes
/// the same physics but not the same floating-point expressions, and
/// the homogeneous path is bitwise-frozen by the determinism contract.
pub fn diffuse_variable(grid: &Grid, h: &mut [f64], is_base: &[bool], kappa_cell: &[f64], dt: f64) {
    let kmax = kappa_cell.iter().copied().fold(0.0f64, f64::max);
    let alpha_max = kmax * dt / (grid.dx * grid.dx);
    if alpha_max == 0.0 {
        return;
    }
    let cycles = (alpha_max / 0.24).ceil().max(1.0) as u32;
    let a = dt / cycles as f64 / (grid.dx * grid.dx);
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
            let ki = kappa_cell[i];
            let mut flux = 0.0;
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx >= 0 && ny >= 0 && nx < w && ny < hh {
                    let j = (ny * w + nx) as usize;
                    flux += 0.5 * (ki + kappa_cell[j]) * (src[j] - c);
                }
            }
            *out = c + a * flux;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn checker_field(grid: &Grid) -> (Vec<f64>, Vec<f64>, Vec<bool>) {
        let n = grid.n();
        let h: Vec<f64> = (0..n as u32)
            .map(|i| {
                let (x, y) = grid.xy(i);
                ((x * 7 + y * 13) % 11) as f64 * 0.9 + (x as f64) * 0.1
            })
            .collect();
        let kappa: Vec<f64> = (0..n as u32)
            .map(|i| {
                let (x, y) = grid.xy(i);
                if (x / 3 + y / 3) % 2 == 0 {
                    0.05
                } else {
                    0.01
                }
            })
            .collect();
        (h, kappa, vec![false; n])
    }

    /// Closed Neumann domain (no base cells): the symmetric edge form must
    /// conserve total height to rounding.
    #[test]
    fn variable_diffusion_conserves_mass() {
        let grid = Grid::new(16, 100.0);
        let (mut h, kappa, is_base) = checker_field(&grid);
        let before: f64 = h.iter().sum();
        diffuse_variable(&grid, &mut h, &is_base, &kappa, 5.0e4);
        let after: f64 = h.iter().sum();
        assert!(
            (before - after).abs() <= 1e-9 * before.abs().max(1.0),
            "mass drift {} over {} cells",
            before - after,
            grid.n()
        );
    }

    /// With a constant κ field the flux form must agree with the uniform
    /// path physically (not bitwise — different expressions by design).
    #[test]
    fn variable_diffusion_matches_uniform_on_constant_kappa() {
        let grid = Grid::new(16, 100.0);
        let (h0, _, is_base) = checker_field(&grid);
        let kappa = vec![0.05f64; grid.n()];
        let mut ha = h0.clone();
        let mut hb = h0.clone();
        diffuse(&grid, &mut ha, &is_base, 0.05, 5.0e4);
        diffuse_variable(&grid, &mut hb, &is_base, &kappa, 5.0e4);
        let worst = max_abs_diff(&ha, &hb);
        assert!(worst < 1e-9, "uniform vs flux form diverge by {worst} m");
    }

    /// A κ step must slow the flux across the contact: the low-κ side
    /// keeps more of its relief than the high-κ side after the same time.
    #[test]
    fn kappa_contrast_shows_in_relaxation() {
        let grid = Grid::new(16, 100.0);
        let n = grid.n();
        let mut h = vec![0.0f64; n];
        let mut kappa = vec![0.0f64; n];
        for i in 0..n as u32 {
            let (x, _) = grid.xy(i);
            h[i as usize] = if (2..6).contains(&x) || (10..14).contains(&x) {
                10.0
            } else {
                0.0
            };
            kappa[i as usize] = if x < 8 { 0.05 } else { 0.005 };
        }
        let is_base = vec![false; n];
        let before = h.clone();
        diffuse_variable(&grid, &mut h, &is_base, &kappa, 2.0e4);
        let decay = |xs: std::ops::Range<u32>| -> f64 {
            let mut lost = 0.0;
            for i in 0..n as u32 {
                let (x, _) = grid.xy(i);
                if xs.contains(&x) {
                    lost += before[i as usize] - h[i as usize];
                }
            }
            lost
        };
        assert!(
            decay(2..6) > 3.0 * decay(10..14),
            "high-κ block relaxed {} vs low-κ {} — contrast lost",
            decay(2..6),
            decay(10..14)
        );
    }
}
