//! Statistical gates: the quantitative half of the QA contract (README:
//! statistics as gates, not post-hoc evaluation). Target ranges are the
//! empirical norms for fluvial landscapes; see the reading list.

use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;

use crate::climate::Wind;
use crate::config::TerrainConfig;
use crate::flow::{DonorGraph, MfdGraph};
use crate::grid::Grid;
use crate::sediment::FLOOD_EPS_M;

#[derive(Clone, Debug, Serialize)]
pub struct Gate {
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub pass: bool,
    /// Advisory gates are reported but do not affect `overall_pass`.
    pub advisory: bool,
}

impl Gate {
    fn new(value: f64, lo: f64, hi: f64, advisory: bool) -> Self {
        Self {
            value,
            lo,
            hi,
            pass: value >= lo && value <= hi,
            advisory,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HortonRow {
    pub order: u32,
    pub streams: u32,
    pub mean_length_km: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GatesReport {
    /// Slope–area concavity θ from S ∝ A^−θ over fluvial cells. The
    /// regression is restricted to cells whose uplift rate lies in a narrow
    /// band around the median: the θ ≈ m/n prediction assumes uniform
    /// uplift, exactly as field studies restrict to uniform-uplift
    /// catchments.
    pub slope_area_theta: Gate,
    pub slope_area_r2: f64,
    pub slope_area_n_cells: u64,
    /// The exposed unit the regression was restricted to (ADR 0007);
    /// 0 on homogeneous substrates.
    pub slope_area_modal_unit: u32,
    /// Fluvial cells excluded from the regression as ε-fill flats
    /// (S below the slope floor), as on real DEMs.
    pub slope_area_flat_cells_excluded: u64,
    /// Median |(K·√A·S − κ∇²h) / U − 1| over fluvial cells: the direct
    /// steady-state self-consistency test of ∂h/∂t = 0 (≈0 when converged).
    /// The diffusion term matters at fine cell sizes, where hillslope flux
    /// into valleys rivals uplift.
    pub spl_residual_median: Gate,
    /// Hack's law exponent h from L ∝ A^h over all river subbasins.
    pub hack_exponent: Option<Gate>,
    pub hack_r2: Option<f64>,
    /// Number of subbasins (river cells) in the Hack fit.
    pub hack_n_basins: u32,
    /// Horton bifurcation ratio from Strahler stream counts.
    pub horton_bifurcation: Option<Gate>,
    pub horton_length_ratio: Option<f64>,
    pub hypsometric_integral: Gate,
    pub pit_cells: Gate,
    /// Fraction of land cells whose outlet is open (border-connected) water.
    pub drainage_completeness: Gate,
    pub interior_water_cells: u64,
    /// Mean land precipitation upwind-half / downwind-half of the wind axis:
    /// the orographic rain shadow, measured not assumed.
    pub rain_shadow_ratio: Gate,
    /// |Σdetached − Σdeposited − Σexported| / Σdetached over the whole run:
    /// the sediment-routing bookkeeping audit (ADR 0004, core).
    pub sediment_mass_closure: Gate,
    pub sediment_detached_m3: f64,
    pub sediment_deposited_m3: f64,
    pub sediment_exported_m3: f64,
    pub sediment_mean_land_m: f64,
    /// Shallow-pond merge volume over the run (m³): the declared sub-grid
    /// mass source (ADR 0004), reported so its magnitude is auditable
    /// against the fluvial budget.
    pub pond_merge_m3: f64,
    /// Largest water-surface spread within one connected standing-water
    /// body (m): every lake must sit level at its spill elevation.
    pub lake_level_spread_m: Gate,
    /// Cells with ≥ 0.3 m standing water (the reference lake threshold).
    pub lake_cells: u64,
    /// Median slope of thick-sediment land cells over median bedrock-channel
    /// slope: floodplains should lie flatter than the channels feeding them.
    /// None when the run built no floodplains to measure.
    pub floodplain_slope_ratio: Option<Gate>,
    /// max/min across exposed units of the per-unit median steady-state
    /// balance (K√Q·S − κ∇²h − D)/U — unit-independent near 1 when each
    /// unit obeys the incision law it was evolved under (ADR 0007,
    /// advisory). None when fewer than two units carry enough fluvial
    /// cells.
    pub unit_spl_consistency: Option<Gate>,
    /// Units that qualified for the consistency measurement.
    pub unit_spl_units: u32,
    pub wind: (i32, i32),
    pub land_fraction: f64,
    pub max_elevation_m: f64,
    pub relief_m: f64,
    pub overall_pass: bool,
    /// blake3 of each quantized artifact raster; the determinism witness.
    pub artifact_blake3: BTreeMap<String, String>,
}

/// Raw series behind the gates, for plotting. Written as `qa_samples.json`.
#[derive(Clone, Debug, Serialize)]
pub struct GateSamples {
    /// Decimated (log10 A [m²], log10 S) pairs over fluvial cells.
    pub slope_area_log10: Vec<(f64, f64)>,
    /// (fraction of land area at or above elevation, normalized elevation).
    pub hypsometric_curve: Vec<(f64, f64)>,
    pub horton: Vec<HortonRow>,
    /// (basin area [km²], mainstream length [km]).
    pub hack_points: Vec<(f64, f64)>,
}

pub struct GateInputs<'a> {
    pub grid: &'a Grid,
    pub cfg: &'a TerrainConfig,
    pub h: &'a [f64],
    pub uplift: &'a [f64],
    /// The channel tree (max-weight receivers); tree statistics live here.
    pub receivers: &'a [u32],
    /// The full MFD graph — the residual's fluvial term must sum what
    /// erosion actually applied over all out-edges (ADR 0005).
    pub mfd: &'a MfdGraph,
    pub stack: &'a [u32],
    pub donors: &'a DonorGraph,
    pub area_cells: &'a [u64],
    /// Precipitation-weighted MFD accumulation in equivalent cells; the
    /// flow metric erosion actually used. Physics gates regress on this.
    pub discharge_cells: &'a [f64],
    /// The same weights accumulated along the channel tree — monotone
    /// downstream, so river-membership thresholds are downstream-closed.
    /// Extraction-flavoured gates (Hack subbasin membership) use this
    /// (ADR 0005).
    pub tree_discharge_cells: &'a [f64],
    pub precip: &'a [f64],
    pub wind: Wind,
    pub strahler: &'a [u32],
    pub basin: &'a [u32],
    pub dist_m: &'a [f64],
    /// Longest along-flow path from each cell up to a divide (m).
    pub mainstream_m: &'a [f64],
    pub land: &'a [bool],
    /// Standing-water depth (m): routed surface − true surface.
    pub water_depth: &'a [f64],
    /// Sediment thickness (m).
    pub sediment_m: &'a [f64],
    /// Deposition rate (m/yr) from the diagnostic replay of one step on the
    /// final surface; the residual balance needs it.
    pub deposition_rate_m_per_yr: &'a [f64],
    /// Per-cell erodibility on the final surface (ADR 0007): the residual
    /// must test each cell against the K it actually eroded with.
    pub k_cell: &'a [f64],
    /// Per-cell hillslope diffusivity on the final surface.
    pub kappa_cell: &'a [f64],
    /// Exposed stratigraphic unit per cell on the final surface.
    pub exposed_unit: &'a [u32],
    pub run_detached_m3: f64,
    pub run_deposited_m3: f64,
    pub run_exported_m3: f64,
    /// Volume created by the shallow-pond merge over the run (m³): the
    /// declared sub-grid mass source, metered so the fake stays quantified.
    pub run_pond_merge_m3: f64,
}

/// Upper median after a total-order sort; deterministic.
fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

/// Ordinary least squares over an iterator of points. Sequential, fixed
/// order. Returns (slope, intercept, r², n).
fn linfit(points: impl Iterator<Item = (f64, f64)>) -> Option<(f64, f64, f64, u64)> {
    let (mut n, mut sx, mut sy, mut sxx, mut sxy, mut syy) = (0u64, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (x, y) in points {
        n += 1;
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
        syy += y * y;
    }
    if n < 3 {
        return None;
    }
    let nf = n as f64;
    let cxx = sxx - sx * sx / nf;
    let cxy = sxy - sx * sy / nf;
    let cyy = syy - sy * sy / nf;
    if cxx <= 0.0 {
        return None;
    }
    let slope = cxy / cxx;
    let intercept = (sy - slope * sx) / nf;
    let r2 = if cyy > 0.0 {
        (cxy * cxy) / (cxx * cyy)
    } else {
        1.0
    };
    Some((slope, intercept, r2, n))
}

pub fn compute(inp: &GateInputs) -> (GatesReport, GateSamples) {
    let n = inp.grid.n();
    let cell_m2 = inp.cfg.cell_area_m2();
    let sea = inp.cfg.sea_level_m;

    let land_count = inp.land.iter().filter(|&&l| l).count() as u64;
    let land_fraction = land_count as f64 / n as f64;

    // --- Slope–area ---------------------------------------------------
    // ε-filled flats have S ≈ ε/dx ~ 1e-11; as log-slope outliers they
    // would dominate the regression, so they are excluded (standard
    // practice on filled real-world DEMs).
    const SLOPE_FLOOR: f64 = 1.0e-5;
    const UPLIFT_BAND: (f64, f64) = (0.75, 1.25);
    // Slope–area runs on discharge (the flow metric erosion used): with
    // spatially variable precipitation the steady state is S = U/(K√Q), and
    // regressing against raw area would mix climates.
    let fluvial_min_cells = (inp.cfg.fluvial_min_area_km2 * 1.0e6) / cell_m2;
    // Flooded cells stay out of the regression and the residual (their bed
    // slopes are not stream-power statements). Sediment-covered channels
    // stay IN: under the single-K G-model every steady channel obeys
    // S = U(1+G)/(K√Q) regardless of cover — channel cells carry an
    // in-transit layer of order G·U·dt every step, and excluding them
    // would measure the model wrong. The bedrock/alluvial split becomes
    // physical with M4's erodibility contrast (ADR 0004).
    // Persistent floodplains for the advisory gate are cells well above the
    // in-transit layer.
    let alluvial_sed_m =
        (2.0 * inp.cfg.g_deposition * inp.cfg.uplift_max_m_per_yr * inp.cfg.dt_years).max(0.5);
    // (discharge m²-equivalent, slope, uplift) for every fluvial cell above
    // the floor, with the cell index kept for the residual's Laplacian.
    let mut fluvial: Vec<(f64, f64, f64)> = Vec::new();
    let mut fluvial_idx: Vec<usize> = Vec::new();
    let mut flat_excluded = 0u64;
    for i in 0..n {
        let r = inp.receivers[i] as usize;
        if !inp.land[i] || r == i || inp.discharge_cells[i] < fluvial_min_cells {
            continue;
        }
        if inp.water_depth[i] > FLOOD_EPS_M {
            continue;
        }
        // Rivers grade to the surface they actually meet — sea level at the
        // coast, the water surface at a lake, the bed elsewhere — matching
        // the effective receiver elevation the erosion solve uses.
        let hr_eff = if !inp.land[r] {
            inp.h[r].max(sea)
        } else if inp.water_depth[r] > FLOOD_EPS_M {
            inp.h[r] + inp.water_depth[r]
        } else {
            inp.h[r]
        };
        let s = (inp.h[i] - hr_eff) / inp.grid.step_dist_m(i as u32, r as u32);
        if s <= 0.0 {
            continue;
        }
        if s < SLOPE_FLOOR {
            flat_excluded += 1;
            continue;
        }
        let q_m2 = inp.discharge_cells[i] * cell_m2;
        fluvial.push((q_m2, s, inp.uplift[i]));
        fluvial_idx.push(i);
    }
    let mut uplifts: Vec<f64> = fluvial
        .iter()
        .map(|&(_, _, u)| u)
        .filter(|&u| u > 0.0)
        .collect();
    let u_median = median(&mut uplifts);
    // With layered K a single log-log regression across units measures
    // the column, not the incision law: the fit restricts to the modal
    // exposed unit (ADR 0007). Homogeneous runs have one unit and the
    // restriction is a no-op — bitwise-identical samples.
    let mut unit_counts: BTreeMap<u32, u64> = BTreeMap::new();
    for &idx in &fluvial_idx {
        *unit_counts.entry(inp.exposed_unit[idx]).or_insert(0) += 1;
    }
    let modal_unit = unit_counts
        .iter()
        .max_by_key(|&(u, &c)| (c, std::cmp::Reverse(*u)))
        .map(|(&u, _)| u)
        .unwrap_or(0);
    let sa_points: Vec<(f64, f64)> = match u_median {
        Some(um) => fluvial
            .iter()
            .zip(fluvial_idx.iter())
            .filter(|&(&(_, _, u), &idx)| {
                u >= UPLIFT_BAND.0 * um
                    && u <= UPLIFT_BAND.1 * um
                    && inp.exposed_unit[idx] == modal_unit
            })
            .map(|(&(a, s, _), _)| (a.log10(), s.log10()))
            .collect(),
        None => Vec::new(),
    };
    let sa_fit = linfit(sa_points.iter().copied());
    let (theta, sa_r2, sa_n) = match sa_fit {
        Some((slope, _, r2, m)) => (-slope, r2, m),
        None => (f64::NAN, 0.0, 0),
    };
    let stride = (sa_points.len() / 4000).max(1);
    let slope_area_log10: Vec<(f64, f64)> = sa_points.iter().step_by(stride).copied().collect();

    // Steady-state self-consistency: at ∂h/∂t = 0, K·√A·S − κ∇²h = U.
    let laplacian = |i: usize| -> f64 {
        let (x, y) = inp.grid.xy(i as u32);
        let (x, y) = (x as i64, y as i64);
        let (w, hh) = (inp.grid.w as i64, inp.grid.h as i64);
        let c = inp.h[i];
        let mut lap = 0.0;
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx >= 0 && ny >= 0 && nx < w && ny < hh {
                lap += inp.h[(ny * w + nx) as usize] - c;
            }
        }
        lap / (inp.grid.dx * inp.grid.dx)
    };
    // With deposition the steady balance is U = K·√Q·S − κ∇²h − G·q_s/Q;
    // the deposition rate comes from the diagnostic replay on the final
    // surface, so all three sinks/sources are measured the same way.
    let mut residuals: Vec<f64> = Vec::with_capacity(fluvial_idx.len());
    // Per-unit steady-state balances for the advisory consistency gate
    // (ADR 0007): each unit's median of (K√Q·S − κ∇²h − D)/U should be a
    // unit-independent constant near 1.
    let mut unit_balance: BTreeMap<u32, Vec<f64>> = BTreeMap::new();
    for (k, &(a, _s, u)) in fluvial.iter().enumerate() {
        if u > 0.0 {
            let idx = fluvial_idx[k];
            // The fluvial term as erosion applied it: K·√Q·Σ wᵢ·Sᵢ over
            // the MFD out-edges, each graded to the surface it meets.
            let mut s_weighted = 0.0f64;
            for (r, w, dist) in inp.mfd.edges(idx as u32) {
                let ru = r as usize;
                let hr = if !inp.land[ru] {
                    inp.h[ru].max(sea)
                } else if inp.water_depth[ru] > FLOOD_EPS_M {
                    inp.h[ru] + inp.water_depth[ru]
                } else {
                    inp.h[ru]
                };
                let s_edge = (inp.h[idx] - hr) / dist;
                if s_edge > 0.0 {
                    s_weighted += w * s_edge;
                }
            }
            let fluvial_term = inp.k_cell[idx] * a.sqrt() * s_weighted;
            let diff_term = inp.kappa_cell[idx] * laplacian(idx);
            let dep_term = inp.deposition_rate_m_per_yr[idx];
            let balance = (fluvial_term - diff_term - dep_term) / u;
            residuals.push((balance - 1.0).abs());
            unit_balance
                .entry(inp.exposed_unit[idx])
                .or_default()
                .push(balance);
        }
    }
    let spl_residual = median(&mut residuals).unwrap_or(f64::NAN);

    // Advisory unit consistency: reported only when at least two units
    // carry enough fluvial cells to give a stable median. Transient
    // knickzones crossing contacts legitimately deviate — that is why it
    // is advisory (ADR 0007).
    const UNIT_MIN_CELLS: usize = 50;
    let mut unit_medians: Vec<f64> = Vec::new();
    for v in unit_balance.values_mut() {
        if v.len() >= UNIT_MIN_CELLS {
            if let Some(m) = median(v) {
                unit_medians.push(m);
            }
        }
    }
    let unit_spl_units = unit_medians.len() as u32;
    let unit_spl_consistency = if unit_medians.len() >= 2 {
        let mx = unit_medians
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let mn = unit_medians.iter().copied().fold(f64::INFINITY, f64::min);
        (mn > 0.0).then(|| Gate::new(mx / mn, 1.0, 2.0, true))
    } else {
        None
    };

    // --- Hack's law over subbasins ---------------------------------------
    // Every river cell defines a subbasin; fitting across all of them spans
    // three-plus decades of area (standard practice, e.g. Rigon et al.
    // 1996). Outlet-only fits on an island are dominated by
    // radius-truncated coastal basins and read systematically low.
    let river_min_cells = inp.cfg.river_min_cells() as f64;
    let mut hack_all: Vec<(f64, f64)> = Vec::new(); // (A km², L km)
    for i in 0..n {
        if !inp.land[i] || inp.tree_discharge_cells[i] < river_min_cells {
            continue;
        }
        let l_km = inp.mainstream_m[i] / 1000.0;
        if l_km <= 0.0 {
            continue;
        }
        let a_km2 = inp.area_cells[i] as f64 * cell_m2 / 1.0e6;
        hack_all.push((a_km2, l_km));
    }
    let hack_fit = if hack_all.len() >= 10 {
        linfit(hack_all.iter().map(|&(a, l)| (a.log10(), l.log10())))
    } else {
        None
    };
    let hack_exponent = hack_fit.map(|(slope, _, _, _)| Gate::new(slope, 0.45, 0.70, false));
    let hack_r2 = hack_fit.map(|(_, _, r2, _)| r2);
    let hack_stride = (hack_all.len() / 3000).max(1);
    let hack_points: Vec<(f64, f64)> = hack_all.iter().step_by(hack_stride).copied().collect();

    // --- Horton ratios --------------------------------------------------
    // Horton's laws are stated per drainage basin (Strahler); pooling the
    // whole landmass lets order-1 coastal microbasins inflate N₁, so the
    // ratios are measured inside the largest basin only.
    let mut basin_cells: BTreeMap<u32, u64> = BTreeMap::new();
    for i in 0..n {
        if inp.land[i] {
            *basin_cells.entry(inp.basin[i]).or_insert(0) += 1;
        }
    }
    let largest_basin = basin_cells
        .iter()
        .max_by_key(|&(label, &cells)| (cells, std::cmp::Reverse(*label)))
        .map(|(&label, _)| label);
    // A stream head of order ω is a river cell of order ω none of whose
    // river donors has order ω. Each head begins one Strahler stream.
    let mut heads: Vec<u32> = Vec::new();
    for i in 0..n {
        let o = inp.strahler[i];
        if o == 0 || Some(inp.basin[i]) != largest_basin {
            continue;
        }
        let mut same_order_donor = false;
        for &d in inp.donors.donors(i as u32) {
            if inp.strahler[d as usize] == o {
                same_order_donor = true;
                break;
            }
        }
        if !same_order_donor {
            heads.push(i as u32);
        }
    }
    let max_order = inp.strahler.iter().copied().max().unwrap_or(0);
    let mut counts = vec![0u32; max_order as usize + 1];
    let mut lengths = vec![0.0f64; max_order as usize + 1];
    for &head in &heads {
        let o = inp.strahler[head as usize];
        counts[o as usize] += 1;
        // Walk downstream while the order stays ω.
        let mut c = head;
        let mut len = 0.0;
        loop {
            let r = inp.receivers[c as usize];
            if r == c {
                break;
            }
            len += inp.grid.step_dist_m(c, r);
            if inp.strahler[r as usize] != o {
                break;
            }
            c = r;
        }
        lengths[o as usize] += len;
    }
    let horton: Vec<HortonRow> = (1..=max_order)
        .filter(|&o| counts[o as usize] > 0)
        .map(|o| HortonRow {
            order: o,
            streams: counts[o as usize],
            mean_length_km: lengths[o as usize] / counts[o as usize] as f64 / 1000.0,
        })
        .collect();
    let rb_fit = if horton.len() >= 3 {
        linfit(
            horton
                .iter()
                .map(|row| (row.order as f64, (row.streams as f64).ln())),
        )
    } else {
        None
    };
    let horton_bifurcation =
        rb_fit.map(|(slope, _, _, _)| Gate::new((-slope).exp(), 3.0, 5.0, false));
    let rl_fit = if horton.len() >= 3 {
        linfit(
            horton
                .iter()
                .filter(|row| row.mean_length_km > 0.0)
                .map(|row| (row.order as f64, row.mean_length_km.ln())),
        )
    } else {
        None
    };
    let horton_length_ratio = rl_fit.map(|(slope, _, _, _)| slope.exp());

    // --- Hypsometry -------------------------------------------------------
    let mut h_min = f64::INFINITY;
    let mut h_max = f64::NEG_INFINITY;
    for i in 0..n {
        if inp.land[i] {
            h_min = h_min.min(inp.h[i]);
            h_max = h_max.max(inp.h[i]);
        }
    }
    let range = (h_max - h_min).max(1e-9);
    const BINS: usize = 1024;
    let mut hist = vec![0u64; BINS];
    let mut norm_sum = 0.0f64;
    for i in 0..n {
        if inp.land[i] {
            let t = ((inp.h[i] - h_min) / range).clamp(0.0, 1.0);
            norm_sum += t;
            let b = ((t * (BINS as f64 - 1.0)) as usize).min(BINS - 1);
            hist[b] += 1;
        }
    }
    let hypso_integral = if land_count > 0 {
        norm_sum / land_count as f64
    } else {
        f64::NAN
    };
    // Suffix cumulative: fraction of land at or above each elevation level.
    let mut above = vec![0u64; BINS + 1];
    for b in (0..BINS).rev() {
        above[b] = above[b + 1] + hist[b];
    }
    let hypsometric_curve: Vec<(f64, f64)> = (0..=100)
        .map(|k| {
            let t = k as f64 / 100.0;
            let b = ((t * (BINS as f64 - 1.0)) as usize).min(BINS - 1);
            (above[b] as f64 / land_count.max(1) as f64, t)
        })
        .collect();

    // --- Pits and drainage completeness -----------------------------------
    let pit_count = (0..n)
        .filter(|&i| inp.land[i] && inp.receivers[i] as usize == i)
        .count() as f64;

    // Open water = water cells reachable from the border through water.
    let mut open = vec![false; n];
    let mut queue = VecDeque::new();
    for i in 0..n as u32 {
        if inp.grid.is_border(i) && !inp.land[i as usize] {
            open[i as usize] = true;
            queue.push_back(i);
        }
    }
    while let Some(c) = queue.pop_front() {
        inp.grid.for_neighbors(c, |nb, _| {
            let nbu = nb as usize;
            if !open[nbu] && !inp.land[nbu] {
                open[nbu] = true;
                queue.push_back(nb);
            }
        });
    }
    let interior_water_cells = (0..n).filter(|&i| !inp.land[i] && !open[i]).count() as u64;
    let drained = (0..n)
        .filter(|&i| inp.land[i] && open[inp.basin[i] as usize])
        .count() as u64;
    let completeness = if land_count > 0 {
        drained as f64 / land_count as f64
    } else {
        1.0
    };

    // --- Rain shadow ------------------------------------------------------
    // Split land on the median projection along the wind axis; air enters
    // at low projection, so the upwind half should be wetter.
    let (wdx, wdy) = (inp.wind.dx as f64, inp.wind.dy as f64);
    let mut projs: Vec<f64> = Vec::new();
    for i in 0..n {
        if inp.land[i] {
            let (x, y) = inp.grid.xy(i as u32);
            projs.push(x as f64 * wdx + y as f64 * wdy);
        }
    }
    let shadow_ratio = match median(&mut projs) {
        Some(mid) => {
            let (mut up_s, mut up_n, mut dn_s, mut dn_n) = (0.0f64, 0u64, 0.0f64, 0u64);
            for i in 0..n {
                if inp.land[i] {
                    let (x, y) = inp.grid.xy(i as u32);
                    let p = x as f64 * wdx + y as f64 * wdy;
                    if p <= mid {
                        up_s += inp.precip[i];
                        up_n += 1;
                    } else {
                        dn_s += inp.precip[i];
                        dn_n += 1;
                    }
                }
            }
            if up_n > 0 && dn_n > 0 && dn_s > 0.0 {
                (up_s / up_n as f64) / (dn_s / dn_n as f64)
            } else {
                f64::NAN
            }
        }
        None => f64::NAN,
    };

    // --- Sediment mass closure --------------------------------------------
    let closure = if inp.run_detached_m3 > 0.0 {
        (inp.run_detached_m3 - inp.run_deposited_m3 - inp.run_exported_m3).abs()
            / inp.run_detached_m3
    } else {
        0.0
    };
    let mut sed_sum = 0.0f64;
    for i in 0..n {
        if inp.land[i] {
            sed_sum += inp.sediment_m[i];
        }
    }
    let sed_mean_land = if land_count > 0 {
        sed_sum / land_count as f64
    } else {
        0.0
    };

    // --- Standing water ----------------------------------------------------
    // Every connected flooded body must sit level at its spill: measure the
    // worst water-surface spread. Also count reference-threshold lake cells.
    let mut lake_spread = 0.0f64;
    let mut lake_cells = 0u64;
    {
        let mut seen = vec![false; n];
        for start in 0..n as u32 {
            let su = start as usize;
            if inp.water_depth[su] >= 0.3 {
                lake_cells += 1;
            }
            if seen[su] || inp.water_depth[su] <= FLOOD_EPS_M {
                continue;
            }
            let (mut lo_s, mut hi_s) = (f64::INFINITY, f64::NEG_INFINITY);
            let mut queue = VecDeque::new();
            seen[su] = true;
            queue.push_back(start);
            while let Some(c) = queue.pop_front() {
                let cu = c as usize;
                let surf = inp.h[cu] + inp.water_depth[cu];
                lo_s = lo_s.min(surf);
                hi_s = hi_s.max(surf);
                inp.grid.for_neighbors(c, |nb, _| {
                    let nbu = nb as usize;
                    if !seen[nbu] && inp.water_depth[nbu] > FLOOD_EPS_M {
                        seen[nbu] = true;
                        queue.push_back(nb);
                    }
                });
            }
            lake_spread = lake_spread.max(hi_s - lo_s);
        }
    }

    // --- Floodplain slopes (advisory) ---------------------------------------
    // Population: thick-sediment cells within the fluvial domain. Colluvial
    // hillslope piles (diffusion bookkeeping) also exceed the thickness
    // threshold but are not floodplains and would dominate the median at
    // coarse dt (measured: they flipped the ratio above 1 at research dt).
    let mut flood_slopes: Vec<f64> = Vec::new();
    for i in 0..n {
        let r = inp.receivers[i] as usize;
        if !inp.land[i]
            || r == i
            || inp.water_depth[i] > FLOOD_EPS_M
            || inp.sediment_m[i] <= alluvial_sed_m
            || inp.discharge_cells[i] < fluvial_min_cells
        {
            continue;
        }
        let s = (inp.h[i] - inp.h[r]).max(0.0) / inp.grid.step_dist_m(i as u32, r as u32);
        flood_slopes.push(s);
    }
    let mut bedrock_slopes: Vec<f64> = fluvial.iter().map(|&(_, s, _)| s).collect();
    let floodplain_slope_ratio = match (median(&mut flood_slopes), median(&mut bedrock_slopes)) {
        (Some(f), Some(b)) if b > 0.0 => Some(Gate::new(f / b, 0.0, 1.0, true)),
        _ => None,
    };

    let report = GatesReport {
        slope_area_theta: Gate::new(theta, 0.40, 0.60, false),
        slope_area_r2: sa_r2,
        slope_area_n_cells: sa_n,
        slope_area_modal_unit: modal_unit,
        slope_area_flat_cells_excluded: flat_excluded,
        // Core, not advisory: this is the tripwire for operator-splitting
        // violations (ADR 0002) — a config whose dt breaks the κ∇²h·dt ≲
        // U·dt limit must fail, not warn.
        spl_residual_median: Gate::new(spl_residual, 0.0, 0.35, false),
        hack_exponent,
        hack_r2,
        hack_n_basins: hack_all.len() as u32,
        horton_bifurcation,
        horton_length_ratio,
        hypsometric_integral: Gate::new(hypso_integral, 0.35, 0.60, true),
        pit_cells: Gate::new(pit_count, 0.0, 0.0, false),
        drainage_completeness: Gate::new(completeness, 1.0, 1.0, false),
        interior_water_cells,
        rain_shadow_ratio: Gate::new(shadow_ratio, 1.05, 100.0, true),
        sediment_mass_closure: Gate::new(closure, 0.0, 1.0e-9, false),
        sediment_detached_m3: inp.run_detached_m3,
        sediment_deposited_m3: inp.run_deposited_m3,
        sediment_exported_m3: inp.run_exported_m3,
        sediment_mean_land_m: sed_mean_land,
        pond_merge_m3: inp.run_pond_merge_m3,
        lake_level_spread_m: Gate::new(lake_spread, 0.0, 0.05, false),
        lake_cells,
        floodplain_slope_ratio,
        unit_spl_consistency,
        unit_spl_units,
        wind: (inp.wind.dx, inp.wind.dy),
        land_fraction,
        max_elevation_m: h_max,
        relief_m: h_max - sea,
        overall_pass: false, // set below
        artifact_blake3: BTreeMap::new(),
    };
    let mut report = report;
    // A core gate that could not be evaluated (too little data) is a FAIL,
    // not a pass-by-absence: the run failed to demonstrate compliance.
    let core = [
        Some(&report.slope_area_theta),
        Some(&report.spl_residual_median),
        report.hack_exponent.as_ref(),
        report.horton_bifurcation.as_ref(),
        Some(&report.pit_cells),
        Some(&report.drainage_completeness),
        Some(&report.sediment_mass_closure),
        Some(&report.lake_level_spread_m),
    ];
    report.overall_pass = core.iter().all(|g| g.is_some_and(|g| g.pass));

    let samples = GateSamples {
        slope_area_log10,
        hypsometric_curve,
        horton,
        hack_points,
    };
    (report, samples)
}
