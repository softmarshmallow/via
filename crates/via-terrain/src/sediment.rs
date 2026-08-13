//! The coupled erosion–deposition solve: Davy & Lague (2009) in the
//! G-coefficient form, by the implicit Gauss–Seidel method of Yuan et al.
//! (2019). ADR 0006 (scheme), ADR 0004 (physics contract).
//!
//! Within one timestep the erosion field E and deposition field D must
//! agree: E is solved implicitly on slopes that include D's aggradation,
//! D is routed from the sediment flux E supplies. The fixed point is
//! reached by iterating two sweeps until the assembled surface
//! h = ht + D − E stops moving:
//!
//! - **Sweep A** (receivers-first): the Braun & Willett implicit
//!   relaxation over the MFD DAG (ADR 0005), with the source elevation
//!   lifted by the previous iterate's deposition.
//! - **Sweep B** (donors-first): flux bookkeeping — dry fluvial cells
//!   keep the min(G/Q̃, 1) fraction of incoming flux, flooded cells trap
//!   up to the water level (lacustrine deltas), hillslopes pass through,
//!   base cells export. It mutates no heights; it produces the next D
//!   field and the budget, so mass closure is an identity of the final
//!   sweep — the budget always describes the heights actually applied.
//!
//! Both sweeps are sequential in fixed order and the convergence test is
//! a sequential fold, so the iteration count — and therefore the output
//! — is bitwise deterministic.

use crate::flow::MfdGraph;
use crate::grid::Grid;

/// Water depth (m) above which a cell counts as flooded for the physics.
/// A numerical guard, not an interpretation threshold: ε-fill raises
/// micro-depressions by ~1e-6 m and those must keep behaving as dry land.
pub const FLOOD_EPS_M: f64 = 1.0e-3;

/// Minimum maximum-ponding depth (m) for a depression to persist as a lake
/// on the true surface; shallower components are sub-grid noise and merge
/// back into the terrain (see `flow::merge_shallow_depressions`).
pub const MIN_LAKE_DEPTH_M: f64 = 0.5;

/// Gauss–Seidel exit: max |Δh| between iterates (m). Well below the
/// centimetre quantization of the heights artifact.
const GS_TOL_M: f64 = 1.0e-6;

/// Hard iteration cap. Yuan et al. report a handful of iterations for
/// G ≤ 1; a solve that stops converging is a defect, so exceeding the
/// cap panics rather than shipping an unconverged surface.
const GS_MAX_ITER: u32 = 100;

pub struct SolveResult {
    pub detached_m3: f64,
    pub deposited_m3: f64,
    pub exported_m3: f64,
    /// Detached depth E·dt (m, ≥ 0) per cell — what left each cell.
    pub detached_m: Vec<f64>,
    /// Deposited depth D·dt (m, ≥ 0) per cell — what landed on each cell.
    pub deposited_m: Vec<f64>,
    /// Gauss–Seidel iterations used this step.
    pub iterations: u32,
}

/// Sweep A: implicit stream-power detachment on the deposition-lifted
/// source elevation elev = ht + dep. Receivers-first over the MFD
/// topological order, so every receiver's new height is final when its
/// donors relax toward it:
///   h⁺ = (elev + Σᵢ cᵢ·hrᵢ⁺) / (1 + Σᵢ cᵢ),   cᵢ = K·√Q·dt·wᵢ / distᵢ.
///
/// Standing water (ADR 0004): `flooded` and `water_level` are frozen from
/// the pre-erosion state of the step. The mask MUST be static — deciding
/// "submerged" by comparing the evolving surface against the routed copy
/// re-classifies every freshly incised cell as flooded, and the flux
/// sweep then refills exactly what detachment cut (observed: deposition ≈
/// 89% of detachment, landscape smears). Flooded cells do not incise (the
/// implicit form would otherwise *raise* a lake-bottom cell toward its
/// across-lake receiver, which is deposition by the wrong mechanism).
/// Rivers grade to water surfaces, not submerged beds: a flooded
/// receiver's effective elevation is its water level, and a base-level
/// receiver's is max(ht_rcv, sea_level).
#[allow(clippy::too_many_arguments)]
fn erosion_sweep(
    grid: &Grid,
    ht: &[f64],
    dep_m: &[f64],
    h_out: &mut [f64],
    det_out: &mut [f64],
    mfd: &MfdGraph,
    discharge_cells: &[f64],
    is_base: &[bool],
    flooded: &[bool],
    water_level: &[f64],
    k_spl: f64,
    dt: f64,
    sea_level: f64,
) {
    for &i in &mfd.order {
        let iu = i as usize;
        if is_base[iu] {
            h_out[iu] = ht[iu];
            det_out[iu] = 0.0;
            continue;
        }
        let elev = ht[iu] + dep_m[iu];
        if flooded[iu] {
            h_out[iu] = elev;
            det_out[iu] = 0.0;
            continue;
        }
        let sqrt_q_m = grid.dx * discharge_cells[iu].sqrt();
        let mut num = elev;
        let mut den = 1.0f64;
        for (r, w, dist) in mfd.edges(i) {
            let ru = r as usize;
            // Every out-edge descends strictly on the routed surface, so
            // ru precedes iu in the order and h_out[ru] is this
            // iteration's value.
            let hr = if is_base[ru] {
                ht[ru].max(sea_level)
            } else if flooded[ru] {
                water_level[ru]
            } else {
                h_out[ru]
            };
            if elev > hr {
                let c = k_spl * sqrt_q_m * dt * w / dist;
                num += c * hr;
                den += c;
            }
        }
        let hn = if den > 1.0 { num / den } else { elev };
        h_out[iu] = hn;
        det_out[iu] = elev - hn;
    }
}

/// Sweep B: route the detached volume down the MFD DAG in donors-first
/// order, producing the deposition field and the budget. Deposition acts
/// on the **fluvial domain only** (discharge ≥ `fluvial_min_cells`): the
/// ξ–q model is channel physics, and on Q̃ ≈ 1 hillslope cells the G/Q̃
/// fraction saturates at 1 — under the M3 explicit scheme every
/// hillslope cell became a perfect trap and the landscape smeared into a
/// dome (ADR 0004). Hillslope transport is the diffusion term's job.
/// The min(G/Q̃, 1) cap stays because the precipitation floor lets
/// rain-shadow cells run Q̃ < G, where an uncapped fraction would deposit
/// more than arrives. Flooded cells trap regardless of discharge, up to
/// headroom measured against the start-of-step surface (iteration-order
/// free). Returns (detached_m3, deposited_m3, exported_m3).
#[allow(clippy::too_many_arguments)]
fn flux_sweep(
    grid: &Grid,
    ht: &[f64],
    det_m: &[f64],
    dep_out: &mut [f64],
    flux_in: &mut [f64],
    mfd: &MfdGraph,
    discharge_cells: &[f64],
    is_base: &[bool],
    flooded: &[bool],
    water_level: &[f64],
    g: f64,
    fluvial_min_cells: f64,
) -> (f64, f64, f64) {
    let area = grid.dx * grid.dx;
    flux_in.fill(0.0);
    let (mut total_det, mut total_dep, mut total_exp) = (0.0f64, 0.0f64, 0.0f64);
    for &i in mfd.order.iter().rev() {
        let iu = i as usize;
        let phi = flux_in[iu];
        let det_vol = det_m[iu] * area;
        total_det += det_vol;
        if is_base[iu] {
            dep_out[iu] = 0.0;
            total_exp += phi;
            continue;
        }
        let dep_vol = if flooded[iu] {
            phi.min(((water_level[iu] - ht[iu]) * area).max(0.0))
        } else if discharge_cells[iu] >= fluvial_min_cells {
            phi * (g / discharge_cells[iu]).min(1.0)
        } else {
            0.0
        };
        dep_out[iu] = dep_vol / area;
        total_dep += dep_vol;
        let out = phi - dep_vol + det_vol;
        for (t, w, _) in mfd.edges(i) {
            flux_in[t as usize] += out * w;
        }
    }
    (total_det, total_dep, total_exp)
}

/// Solve one timestep of coupled erosion–deposition on `h` (entered
/// post-uplift, mutated to the step's final surface ht + D − E).
///
/// The M3 explicit scheme was exactly one iteration of this loop (with
/// in-place height mutation); the fixed point removes both of its costs:
/// the erode/deposit splitting oscillation that bounded dt, and the
/// donor-floor aggradation cap that protected a mutating pass. If
/// converged deposition fills a reach above a donor, that is real
/// aggradation — the next step's priority-flood floods or reroutes it.
#[allow(clippy::too_many_arguments)]
pub fn solve_implicit(
    grid: &Grid,
    h: &mut [f64],
    mfd: &MfdGraph,
    discharge_cells: &[f64],
    is_base: &[bool],
    flooded: &[bool],
    water_level: &[f64],
    k_spl: f64,
    g: f64,
    dt: f64,
    fluvial_min_cells: f64,
    sea_level: f64,
) -> SolveResult {
    let n = h.len();
    let ht = h.to_vec();
    let mut dep = vec![0.0f64; n];
    let mut det = vec![0.0f64; n];
    let mut h_sweep = vec![0.0f64; n];
    let mut flux_in = vec![0.0f64; n];
    let mut budget = (0.0f64, 0.0f64, 0.0f64);
    let mut iterations = 0u32;
    let mut converged = false;
    while iterations < GS_MAX_ITER {
        iterations += 1;
        erosion_sweep(
            grid,
            &ht,
            &dep,
            &mut h_sweep,
            &mut det,
            mfd,
            discharge_cells,
            is_base,
            flooded,
            water_level,
            k_spl,
            dt,
            sea_level,
        );
        budget = flux_sweep(
            grid,
            &ht,
            &det,
            &mut dep,
            &mut flux_in,
            mfd,
            discharge_cells,
            is_base,
            flooded,
            water_level,
            g,
            fluvial_min_cells,
        );
        // Assemble the iterate and test convergence in one sequential
        // pass (order-fixed: the count must be deterministic).
        let mut max_dh = 0.0f64;
        for i in 0..n {
            let hi = ht[i] + dep[i] - det[i];
            let d = (hi - h[i]).abs();
            if d > max_dh {
                max_dh = d;
            }
            h[i] = hi;
        }
        if max_dh <= GS_TOL_M {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "erosion–deposition Gauss–Seidel failed to converge in {GS_MAX_ITER} \
         iterations (G = {g}, dt = {dt}); the fixed point should take a \
         handful of sweeps — this is a defect, not a tuning problem"
    );
    let (detached_m3, deposited_m3, exported_m3) = budget;
    SolveResult {
        detached_m3,
        deposited_m3,
        exported_m3,
        detached_m: det,
        deposited_m: dep,
        iterations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow;

    /// Ramp toward the base column x = 0; returns everything a solve needs.
    struct Toy {
        grid: Grid,
        h: Vec<f64>,
        is_base: Vec<bool>,
        mfd: MfdGraph,
        discharge: Vec<f64>,
    }

    fn toy_ramp(side: u32) -> Toy {
        let grid = Grid::new(side, 10.0);
        let n = grid.n();
        let mut h: Vec<f64> = (0..n as u32)
            .map(|i| {
                let (x, _) = grid.xy(i);
                x as f64 * 1.0
            })
            .collect();
        let is_base: Vec<bool> = (0..n as u32)
            .map(|i| {
                let (x, _) = grid.xy(i);
                x == 0
            })
            .collect();
        flow::priority_flood_eps(&grid, &mut h, &is_base, 1e-6);
        let mfd = MfdGraph::build(&grid, &h, &is_base, 1.1);
        let weights = vec![1.0f64; n];
        let discharge = flow::accumulate_discharge_mfd(&mfd, &weights);
        Toy {
            grid,
            h,
            is_base,
            mfd,
            discharge,
        }
    }

    /// Mass must close exactly and the budget must describe the applied
    /// heights: Σ(h − ht) = deposited − detached.
    #[test]
    fn mass_closes_and_budget_matches_heights() {
        let mut t = toy_ramp(4);
        let ht = t.h.clone();
        let n = t.grid.n();
        let flooded = vec![false; n];
        let water_level = t.h.clone();
        let r = solve_implicit(
            &t.grid,
            &mut t.h,
            &t.mfd,
            &t.discharge,
            &t.is_base,
            &flooded,
            &water_level,
            1e-3,
            1.0,
            100.0,
            0.0,
            0.0,
        );
        let closure =
            (r.detached_m3 - r.deposited_m3 - r.exported_m3).abs() / r.detached_m3.max(1e-30);
        assert!(closure < 1e-12, "closure {closure}");
        assert!(r.deposited_m3 > 0.0);
        assert!(r.exported_m3 > 0.0);
        let area = t.grid.dx * t.grid.dx;
        let dv: f64 = t.h.iter().zip(ht.iter()).map(|(a, b)| (a - b) * area).sum();
        let budget_dv = r.deposited_m3 - r.detached_m3;
        assert!(
            (dv - budget_dv).abs() <= 1e-9 * r.detached_m3,
            "height change {dv} vs budget {budget_dv}"
        );
        assert!(r.iterations >= 2, "fixed point needs at least two sweeps");
    }

    /// G = 0 must reduce exactly to detachment-limited SPL: nothing
    /// deposits, everything detached exports.
    #[test]
    fn g_zero_is_pure_detachment() {
        let mut t = toy_ramp(4);
        let n = t.grid.n();
        let flooded = vec![false; n];
        let water_level = t.h.clone();
        let r = solve_implicit(
            &t.grid,
            &mut t.h,
            &t.mfd,
            &t.discharge,
            &t.is_base,
            &flooded,
            &water_level,
            1e-3,
            0.0,
            100.0,
            0.0,
            0.0,
        );
        assert_eq!(r.deposited_m3, 0.0);
        assert!(
            (r.detached_m3 - r.exported_m3).abs() <= 1e-12 * r.detached_m3,
            "with G = 0 every detached volume must export"
        );
    }

    /// A flooded column traps flux up to its water level and never above.
    #[test]
    fn flooded_cells_trap_to_water_level() {
        let mut t = toy_ramp(6);
        let n = t.grid.n();
        let ht = t.h.clone();
        let mut flooded = vec![false; n];
        let mut water_level = t.h.clone();
        for i in 0..n as u32 {
            let (x, _) = t.grid.xy(i);
            if x == 2 {
                flooded[i as usize] = true;
                water_level[i as usize] = ht[i as usize] + 0.4;
            }
        }
        let r = solve_implicit(
            &t.grid,
            &mut t.h,
            &t.mfd,
            &t.discharge,
            &t.is_base,
            &flooded,
            &water_level,
            1e-2,
            1.0,
            1000.0,
            0.0,
            0.0,
        );
        let mut trapped = 0.0f64;
        for i in 0..n {
            if flooded[i] {
                trapped += r.deposited_m[i];
                assert!(
                    t.h[i] <= water_level[i] + 1e-12,
                    "flooded cell {i} aggraded above its water level"
                );
                assert_eq!(r.detached_m[i], 0.0, "flooded cell {i} incised");
            }
        }
        assert!(trapped > 0.0, "the flooded column received no sediment");
    }
}
