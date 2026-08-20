//! Corridor network: the roads that follow the traffic.
//!
//! Links are considered in descending interaction volume
//! F_ij = W_i·W_j·exp(−β c_ij) and built as least-cost paths over a
//! surface where already-built cells are discounted, so routes bundle
//! into corridors instead of running parallel (the reuse discount of
//! Stahlberg et al. 2023; the build-by-benefit ordering is Louf et al.).
//! A link is skipped when the network already connects its endpoints
//! within a detour tolerance — the difference between densifying and
//! exploring, which is what keeps the result a network rather than a
//! clique.
//!
//! Sequential and order-fixed: the network depends on what was built
//! before it, so the build order is the algorithm, not an implementation
//! detail.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};

use crate::cost::CostSurface;
use crate::util::f64_key;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Cost multiplier on cells that already carry a route.
    pub reuse_mult: f64,
    /// Links per surviving center (the build budget).
    pub links_per_center: f64,
    /// Skip a pair already connected within this factor of its own
    /// least-cost time over the built network.
    pub detour_tolerance: f64,
    /// Ignore pairs whose interaction is below this fraction of the
    /// strongest pair's.
    pub min_flow_frac: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            reuse_mult: 0.35,
            links_per_center: 2.0,
            detour_tolerance: 1.35,
            min_flow_frac: 1.0e-4,
        }
    }
}

pub struct Network {
    /// Built cells, in build order (for rendering as one mask).
    pub built: Vec<bool>,
    /// Each built link: (site a, site b, path cells).
    pub links: Vec<(usize, usize, Vec<u32>)>,
    pub total_length_km: f64,
    /// Length that reused an existing route (km) — the bundling measure.
    pub reused_length_km: f64,
}

/// Travel time between two centers over the *built* network only, used for
/// the redundancy test. Runs on the built mask as a sparse graph.
fn network_time(
    surface: &CostSurface,
    built: &[bool],
    from: u32,
    to: u32,
    limit: f64,
) -> Option<f64> {
    let n = built.len();
    if !built[from as usize] || !built[to as usize] {
        return None;
    }
    let mut dist = vec![f64::INFINITY; n];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    dist[from as usize] = 0.0;
    heap.push(Reverse((f64_key(0.0), from)));
    while let Some(Reverse((k, i))) = heap.pop() {
        if k > f64_key(dist[i as usize]) {
            continue;
        }
        if i == to {
            return Some(dist[i as usize]);
        }
        let di = dist[i as usize];
        if di > limit {
            return None;
        }
        crate::util::for_neighbors8(surface.w, surface.h, i, |nb, fac| {
            if !built[nb as usize] {
                return;
            }
            let step = surface.link_hours(i as usize, nb as usize, fac);
            if !step.is_finite() {
                return;
            }
            let nd = di + step;
            if nd < dist[nb as usize] {
                dist[nb as usize] = nd;
                heap.push(Reverse((f64_key(nd), nb)));
            }
        });
    }
    None
}

/// `alive_sites` are (site index, cell, size); `pair_hours[a][b]` is the
/// free-surface travel time between candidate sites.
pub fn build(
    surface: &CostSurface,
    alive_sites: &[(usize, u32, f64)],
    pair_hours: &[Vec<f64>],
    beta_per_hour: f64,
    dx: f64,
    cfg: &NetworkConfig,
) -> Network {
    let n_cells = surface.w as usize * surface.h as usize;
    let mut built = vec![false; n_cells];
    let mut links = Vec::new();
    let mut total_length_km = 0.0;
    let mut reused_length_km = 0.0;

    // Every center is a node of the network from the start, so the first
    // link between two of them has somewhere to attach.
    for &(_, cell, _) in alive_sites {
        built[cell as usize] = true;
    }

    let mut pairs: Vec<(u64, usize, usize)> = Vec::new();
    for a in 0..alive_sites.len() {
        for b in (a + 1)..alive_sites.len() {
            let (ja, _, wa) = alive_sites[a];
            let (jb, _, wb) = alive_sites[b];
            let c = pair_hours[ja][jb];
            if !c.is_finite() {
                continue;
            }
            let f = wa * wb * (-beta_per_hour * c).exp();
            if f > 0.0 {
                pairs.push((f64_key(f), a, b));
            }
        }
    }
    pairs.sort_unstable_by_key(|&(k, a, b)| (Reverse(k), a, b));

    let budget = (cfg.links_per_center * alive_sites.len() as f64).ceil() as usize;
    let f_max = pairs.first().map(|&(k, _, _)| k).unwrap_or(0);
    for &(k, a, b) in &pairs {
        if links.len() >= budget {
            break;
        }
        // f64_key is monotone, so comparing keys compares flows; recover
        // the flow only for the threshold test.
        if f_max > 0 {
            let fa = f64::from_bits(if k >> 63 == 1 { k ^ (1u64 << 63) } else { !k });
            let fm = f64::from_bits(if f_max >> 63 == 1 {
                f_max ^ (1u64 << 63)
            } else {
                !f_max
            });
            if fm > 0.0 && fa / fm < cfg.min_flow_frac {
                break;
            }
        }
        let (_, cell_a, _) = alive_sites[a];
        let (_, cell_b, _) = alive_sites[b];
        let direct = pair_hours[alive_sites[a].0][alive_sites[b].0];
        if let Some(existing) = network_time(
            surface,
            &built,
            cell_a,
            cell_b,
            direct * cfg.detour_tolerance,
        ) {
            if existing <= direct * cfg.detour_tolerance {
                continue; // already connected well enough — densify, don't duplicate
            }
        }
        let Some(path) = surface.least_cost_path(cell_a, cell_b, &built, cfg.reuse_mult) else {
            continue;
        };
        let mut new_km = 0.0;
        let mut old_km = 0.0;
        for pair in path.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            let diag = (p % surface.w != q % surface.w) && (p / surface.w != q / surface.w);
            let seg = dx / 1000.0 * if diag { std::f64::consts::SQRT_2 } else { 1.0 };
            if built[q as usize] {
                old_km += seg;
            } else {
                new_km += seg;
            }
        }
        for &c in &path {
            built[c as usize] = true;
        }
        total_length_km += new_km + old_km;
        reused_length_km += old_km;
        links.push((alive_sites[a].0, alive_sites[b].0, path));
    }

    Network {
        built,
        links,
        total_length_km,
        reused_length_km,
    }
}
