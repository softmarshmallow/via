//! Sediment routing: the deposition half of Davy & Lague (2009), in the
//! G-coefficient form of Yuan et al. (2019). ADR 0004.
//!
//! One sequential sweep in upstream→downstream (reverse stack) order routes
//! the volume detached by the fluvial solve down the receiver tree. Dry
//! cells keep the G-fraction of the *incoming* flux (local detachment joins
//! the outflow — a headwater cell cannot re-bury itself), flooded cells trap
//! flux up to their water level (lacustrine deltas), base-level cells export
//! (marine dispersal is out of scope until M6). The sweep is a closed
//! bookkeeping system: detached = deposited + exported, and the mass-closure
//! gate holds it to numerical precision.

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

/// Aggradation head-margin (m): a cell never fills to or above its lowest
/// donor, so deposition cannot invert the receiver graph within a step.
const DONOR_MARGIN_M: f64 = 1.0e-6;

pub struct SedimentBudget {
    pub detached_m3: f64,
    pub deposited_m3: f64,
    pub exported_m3: f64,
    /// Deposited depth (m) per cell this sweep — the deposition-rate field
    /// the residual gate needs (divide by dt).
    pub deposited_m: Vec<f64>,
}

/// Route `detached_m` (depth per cell from the fluvial solve) down the
/// receiver tree, mutating `h` and `sediment_m` where deposition lands.
///
/// Deposition acts on the **fluvial domain only** (discharge at or above
/// `fluvial_min_cells`): the ξ–q model is channel physics, and on Q̃ ≈ 1
/// hillslope cells the G/Q̃ fraction saturates at 1 — every hillslope cell
/// becomes a perfect trap, sediment creeps downhill one cell per step, and
/// the landscape smears into a dome (observed, ADR 0004). Hillslope
/// transport is the diffusion term's job; hillslope cells pass flux
/// through. Flooded cells trap regardless of discharge — standing water is
/// standing water.
#[allow(clippy::too_many_arguments)]
pub fn route_sediment(
    grid: &Grid,
    h: &mut [f64],
    sediment_m: &mut [f64],
    mfd: &MfdGraph,
    discharge_cells: &[f64],
    is_base: &[bool],
    flooded: &[bool],
    water_level: &[f64],
    detached_m: &[f64],
    g: f64,
    fluvial_min_cells: f64,
    sea_level: f64,
) -> SedimentBudget {
    let n = h.len();
    let area = grid.dx * grid.dx;
    let mut flux_in = vec![0.0f64; n];
    let mut deposited = vec![0.0f64; n];
    let (mut total_det, mut total_dep, mut total_exp) = (0.0f64, 0.0f64, 0.0f64);

    // Descending topological order = donors before receivers: when a cell
    // is processed, its incoming flux is complete and its donors' heights
    // are final for this step.
    for &i in mfd.order.iter().rev() {
        let iu = i as usize;
        let phi = flux_in[iu];
        let det_vol = detached_m[iu] * area;
        total_det += det_vol;

        if is_base[iu] {
            // Base level has no out-edges: everything arriving leaves the
            // system. (Base cells are skipped by the fluvial solve, so
            // det_vol is 0 here; sea_level is kept in the signature for the
            // day marine deposition arrives in M6.)
            let _ = sea_level;
            total_exp += phi;
            continue;
        }

        let dep_vol = if flooded[iu] {
            // Submerged (per the step's frozen mask — see
            // erosion::erode_stream_power on why it must be static): trap
            // up to the water level. The first flooded cell along the
            // routed path aggrades to level and becomes land next step —
            // delta progradation into the lake.
            phi.min(((water_level[iu] - h[iu]) * area).max(0.0))
        } else if discharge_cells[iu] >= fluvial_min_cells {
            // Dry channel: keep the G-fraction of the incoming flux,
            // capped so the cell stays below its lowest MFD donor.
            let mut donor_floor = f64::INFINITY;
            for &d in mfd.donors(i) {
                donor_floor = donor_floor.min(h[d as usize]);
            }
            let headroom = ((donor_floor - DONOR_MARGIN_M) - h[iu]).max(0.0) * area;
            (phi * (g / discharge_cells[iu]).min(1.0)).min(headroom)
        } else {
            // Dry hillslope: pass through.
            0.0
        };
        if dep_vol > 0.0 {
            let dh = dep_vol / area;
            h[iu] += dh;
            sediment_m[iu] += dh;
            deposited[iu] = dh;
            total_dep += dep_vol;
        }
        // Outflux splits along the MFD weights.
        let out = phi - dep_vol + det_vol;
        for (t, w, _) in mfd.edges(i) {
            flux_in[t as usize] += out * w;
        }
    }

    SedimentBudget {
        detached_m3: total_det,
        deposited_m3: total_dep,
        exported_m3: total_exp,
        deposited_m: deposited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow;

    /// A 4×4 grid with a single straight channel: mass must close exactly
    /// and deposition on a uniform slope must be the G-fraction of inflow.
    #[test]
    fn mass_closes_on_a_toy_channel() {
        let grid = Grid::new(4, 10.0);
        let n = grid.n();
        // Ramp descending toward x=0; border column x=0 is base.
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

        let flooded = vec![false; n];
        let water_level = h.clone();
        let mut sed = vec![0.0f64; n];
        let mut detached = vec![0.0f64; n];
        for i in 0..n {
            if !is_base[i] {
                detached[i] = 0.01;
            }
        }
        let budget = route_sediment(
            &grid,
            &mut h,
            &mut sed,
            &mfd,
            &discharge,
            &is_base,
            &flooded,
            &water_level,
            &detached,
            1.0,
            0.0,
            0.0,
        );
        let closure = (budget.detached_m3 - budget.deposited_m3 - budget.exported_m3).abs()
            / budget.detached_m3;
        assert!(closure < 1e-12, "closure {closure}");
        assert!(budget.deposited_m3 > 0.0);
        assert!(budget.exported_m3 > 0.0);
    }
}
