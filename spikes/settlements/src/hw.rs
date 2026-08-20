//! Harris & Wilson (1978) center dynamics — the settlement engine.
//!
//! Two coupled parts, iterated to a fixed point:
//!
//!   T_ij = O_i · A_j·exp(−β c_ij) / Σ_k A_k·exp(−β c_ik)   (allocation)
//!   dW_j/dt = ε (D_j − κ W_j),  D_j = Σ_i T_ij             (growth)
//!
//! with attractiveness A_j = Q_j · W_j^α. α > 1 is the agglomeration
//! return that new economic geography supplies the warrant for: it makes
//! centers compete, so a hierarchy has to be won rather than assigned.
//! Centers that fall below a viability floor die and stay dead — the
//! culling half of "founding and culling are separate mechanisms".
//!
//! Everything is a fixed-order sequential fold: no float accumulation
//! whose order could vary with scheduling. Matrices are block-major
//! (`[block][site]`) so the hot inner loop runs contiguously.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DynamicsConfig {
    /// Agglomeration return on size.
    pub alpha: f64,
    /// Time-decay of interaction (1/hour). Held era-invariant on purpose:
    /// the era's speed already lives in the cost surface, which is
    /// Marchetti's constant expressed as a mechanism.
    pub beta_per_hour: f64,
    /// Relaxation rate per iteration.
    pub epsilon: f64,
    /// People below which a center is not viable and dies.
    pub viability_floor: f64,
    /// Congestion size (people) at which a center's attractiveness is
    /// halved: A_j = Q_j·W_j^α / (1 + (W_j/w_congestion)^congestion_exp).
    /// 0 disables it. This is the reduced form of the dispersion force
    /// Fujita & Ogawa (1982) carry explicitly as internal commuting cost —
    /// centers here are points, so the crowding they suffer has to be
    /// stated rather than derived. FREE PARAMETER: no empirical value.
    pub w_congestion: f64,
    pub congestion_exp: f64,
    /// Sweeps before any culling: seeded equal, every center is below the
    /// floor at first, and culling immediately would empty the map in one
    /// sweep — an artifact of simultaneous death, not of viability.
    pub warmup_iters: u32,
    /// Largest share of the live centers that may die in one sweep, so
    /// demand can redistribute to the survivors between cullings.
    pub cull_fraction: f64,
    pub max_iter: u32,
    /// Convergence when max |ΔW| / max W falls below this.
    pub tol: f64,
}

impl Default for DynamicsConfig {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta_per_hour: 0.9,
            epsilon: 0.35,
            viability_floor: 200.0,
            w_congestion: 0.0,
            congestion_exp: 2.0,
            warmup_iters: 60,
            cull_fraction: 0.04,
            max_iter: 3000,
            tol: 1.0e-7,
        }
    }
}

pub struct Equilibrium {
    /// Size (people) of every candidate; 0 for the dead.
    pub w: Vec<f64>,
    pub alive: Vec<bool>,
    pub iterations: u32,
    pub converged: bool,
    /// Interaction-weighted mean travel time to the serving center (hours)
    /// — the quantity Marchetti's budget speaks about.
    pub mean_travel_hours: f64,
    /// Population actually allocated (should match the food base).
    pub allocated_people: f64,
}

/// `cost[b][j]` = hours from origin block b to site j; `origin[b]` = people
/// the block feeds; `quality[j]` = the site's fixed attractiveness.
pub fn solve(
    cost: &[Vec<f64>],
    origin: &[f64],
    quality: &[f64],
    cfg: &DynamicsConfig,
) -> Equilibrium {
    let n_blocks = origin.len();
    let n_sites = quality.len();
    let total_pop: f64 = origin.iter().sum();

    // exp(−β c) is fixed for the run; pay for it once.
    let decay: Vec<Vec<f64>> = cost
        .iter()
        .map(|row| {
            row.iter()
                .map(|&c| {
                    if c.is_finite() {
                        (-cfg.beta_per_hour * c).exp()
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();

    // Start every candidate equal: no site is handed a head start.
    let seed = (total_pop / n_sites as f64).max(cfg.viability_floor * 2.0);
    let mut w = vec![seed; n_sites];
    let mut alive = vec![true; n_sites];
    let mut live: Vec<usize> = (0..n_sites).collect();
    let mut attract = vec![0.0f64; n_sites];
    let mut demand = vec![0.0f64; n_sites];

    let attract_of = |q: f64, wj: f64| -> f64 {
        let base = q * wj.powf(cfg.alpha);
        if cfg.w_congestion > 0.0 {
            base / (1.0 + (wj / cfg.w_congestion).powf(cfg.congestion_exp))
        } else {
            base
        }
    };

    let mut iterations = 0;
    let mut converged = false;
    for it in 1..=cfg.max_iter {
        iterations = it;
        for &j in &live {
            attract[j] = attract_of(quality[j], w[j]);
            demand[j] = 0.0;
        }
        for b in 0..n_blocks {
            let o = origin[b];
            if o <= 0.0 {
                continue;
            }
            let row = &decay[b];
            let mut denom = 0.0;
            for &j in &live {
                denom += attract[j] * row[j];
            }
            if denom <= 0.0 {
                continue;
            }
            let scale = o / denom;
            for &j in &live {
                demand[j] += scale * attract[j] * row[j];
            }
        }
        let mut max_dw = 0.0f64;
        let mut max_w = 0.0f64;
        for &j in &live {
            let dw = cfg.epsilon * (demand[j] - w[j]);
            w[j] += dw;
            max_dw = max_dw.max(dw.abs());
            max_w = max_w.max(w[j]);
        }

        // Culling: after the warm-up, the smallest non-viable centers die,
        // at most `cull_fraction` of the live set per sweep. Ties break on
        // index, so the order of death is reproducible.
        let mut died = false;
        if it > cfg.warmup_iters {
            let mut doomed: Vec<usize> = live
                .iter()
                .copied()
                .filter(|&j| w[j] < cfg.viability_floor)
                .collect();
            if !doomed.is_empty() {
                doomed.sort_by(|&a, &b| w[a].partial_cmp(&w[b]).unwrap().then(a.cmp(&b)));
                let quota = ((live.len() as f64 * cfg.cull_fraction).ceil() as usize).max(1);
                // Never empty the map in one sweep: something must survive.
                let quota = quota.min(doomed.len()).min(live.len().saturating_sub(1));
                for &j in doomed.iter().take(quota) {
                    alive[j] = false;
                    w[j] = 0.0;
                    died = true;
                }
            }
        }
        if died {
            live.retain(|&j| alive[j]);
        }
        if live.is_empty() {
            break;
        }
        // A death is a discrete event; the fixed point is only claimed on a
        // sweep where nothing died and nothing moved.
        if !died && max_w > 0.0 && max_dw / max_w < cfg.tol {
            converged = true;
            break;
        }
    }

    for j in 0..n_sites {
        attract[j] = if alive[j] {
            attract_of(quality[j], w[j])
        } else {
            0.0
        };
    }
    let mut time_weighted = 0.0;
    let mut weight = 0.0;
    for b in 0..n_blocks {
        let o = origin[b];
        if o <= 0.0 {
            continue;
        }
        let row = &decay[b];
        let crow = &cost[b];
        let mut denom = 0.0;
        for &j in &live {
            denom += attract[j] * row[j];
        }
        if denom <= 0.0 {
            continue;
        }
        for &j in &live {
            if crow[j].is_finite() {
                let t = o * attract[j] * row[j] / denom;
                time_weighted += t * crow[j];
                weight += t;
            }
        }
    }

    Equilibrium {
        w,
        alive,
        iterations,
        converged,
        mean_travel_hours: if weight > 0.0 {
            time_weighted / weight
        } else {
            f64::NAN
        },
        allocated_people: weight,
    }
}
