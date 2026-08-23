//! Measurements against the candidate gates in
//! docs/research/humanity/0009. Nothing here decides anything: bands are
//! quoted so the numbers can be read, and every one of them is a
//! cross-sectional or ensemble statistic being compared against a single
//! deterministic run — the protocol question 0011 leaves open.

use serde::Serialize;

use crate::fields::Land;
use crate::util::ols;

/// Single-linkage clustering of centers into agglomerations: centers
/// closer than `link_km` belong to the same place. Every distributional
/// gate in the literature is stated for a specific delineation, and
/// docs/research/humanity/0011 records that the algorithm must be frozen
/// before the numbers mean anything — this is the spike's declared choice,
/// not a settled one.
pub fn agglomerate(land: &Land, cells: &[u32], sizes: &[f64], link_km: f64) -> Vec<f64> {
    let n = cells.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let w = land.w as i64;
    let lim = (link_km * 1000.0 / land.dx).powi(2);
    for a in 0..n {
        for b in (a + 1)..n {
            let (xa, ya) = ((cells[a] as i64 % w) as f64, (cells[a] as i64 / w) as f64);
            let (xb, yb) = ((cells[b] as i64 % w) as f64, (cells[b] as i64 / w) as f64);
            if (xa - xb).powi(2) + (ya - yb).powi(2) <= lim {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    // Lower index wins, so the merge order cannot vary.
                    let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
                    parent[hi] = lo;
                }
            }
        }
    }
    let mut totals: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    for (i, &size) in sizes.iter().enumerate().take(n) {
        let r = find(&mut parent, i);
        *totals.entry(r).or_insert(0.0) += size;
    }
    let mut out: Vec<f64> = totals.into_values().collect();
    out.sort_by(|a, b| b.partial_cmp(a).unwrap());
    out
}

#[derive(Debug, Serialize)]
pub struct Measurements {
    pub centers: usize,
    pub candidates_offered: usize,
    pub people_allocated: f64,
    pub largest_center: f64,
    /// Largest center's share of settled population.
    pub primacy: f64,
    /// Gabaix–Ibragimov slope on the whole surviving set, and on the top
    /// decile (the tail the literature actually bands).
    pub rank_size_zeta_all: f64,
    pub rank_size_r2_all: f64,
    pub rank_size_zeta_tail: f64,
    pub rank_size_r2_tail: f64,
    pub rank_size_tail_n: usize,
    /// The same set clustered into agglomerations (single linkage at
    /// `delineation_km`), which is the unit the rank-size literature
    /// actually fits.
    pub agglomerations: usize,
    pub agglomeration_zeta: f64,
    pub agglomeration_r2: f64,
    pub largest_agglomeration: f64,
    /// Largest agglomeration's share of settled population — the primacy
    /// the literature reports, since it is agglomerations that are ranked.
    pub agglomeration_primacy: f64,
    pub delineation_km: f64,
    /// Nearest-neighbour spacing between centers (km).
    pub spacing_mean_km: f64,
    pub spacing_median_km: f64,
    /// Clark–Evans R (no edge correction — see limitations).
    pub clark_evans_r: f64,
    pub centers_per_1000km2: f64,
    /// Interaction-weighted travel time to the serving center (hours).
    pub mean_travel_hours: f64,
    /// Share of centers within 5 km of the sea.
    pub coastal_share: f64,
    /// Kvamme-style gain of center placement against the productivity
    /// field: 1 − (area share of the top productivity quartile) /
    /// (share of centers standing in it).
    pub kvamme_gain: f64,
    pub network_km: f64,
    pub network_reuse_frac: f64,
    pub network_km_per_center: f64,
    pub converged: bool,
    pub iterations: u32,
}

fn nearest_neighbour_km(land: &Land, cells: &[u32]) -> Vec<f64> {
    let w = land.w as i64;
    let mut out = Vec::with_capacity(cells.len());
    for (a, &ca) in cells.iter().enumerate() {
        let (xa, ya) = ((ca as i64 % w) as f64, (ca as i64 / w) as f64);
        let mut best = f64::INFINITY;
        for (b, &cb) in cells.iter().enumerate() {
            if a == b {
                continue;
            }
            let (xb, yb) = ((cb as i64 % w) as f64, (cb as i64 / w) as f64);
            let d = ((xa - xb).powi(2) + (ya - yb).powi(2)).sqrt();
            if d < best {
                best = d;
            }
        }
        out.push(best * land.dx / 1000.0);
    }
    out
}

pub fn measure(
    land: &Land,
    candidates_offered: usize,
    centers: &[(usize, u32, f64)],
    mean_travel_hours: f64,
    allocated: f64,
    network_km: f64,
    network_reuse_km: f64,
    converged: bool,
    iterations: u32,
    delineation_km: f64,
) -> Measurements {
    let n = centers.len();
    let cells: Vec<u32> = centers.iter().map(|&(_, c, _)| c).collect();
    let mut sizes: Vec<f64> = centers.iter().map(|&(_, _, w)| w).collect();
    sizes.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let total: f64 = sizes.iter().sum();

    // Gabaix–Ibragimov: log(rank − 1/2) against log size; the slope's
    // magnitude is ζ.
    let fit = |s: &[f64]| -> (f64, f64) {
        if s.len() < 4 {
            return (f64::NAN, f64::NAN);
        }
        let x: Vec<f64> = s.iter().map(|v| v.ln()).collect();
        let y: Vec<f64> = (0..s.len())
            .map(|r| ((r as f64 + 1.0) - 0.5).ln())
            .collect();
        let (b, _, r2) = ols(&x, &y);
        (-b, r2)
    };
    let (zeta_all, r2_all) = fit(&sizes);
    let agg = agglomerate(
        land,
        &cells,
        &centers.iter().map(|&(_, _, w)| w).collect::<Vec<_>>(),
        delineation_km,
    );
    let (zeta_agg, r2_agg) = fit(&agg);
    let tail_n = (n / 10).max(4).min(n);
    let (zeta_tail, r2_tail) = fit(&sizes[..tail_n]);

    let spacing = if n > 1 {
        nearest_neighbour_km(land, &cells)
    } else {
        Vec::new()
    };
    let spacing_mean_km = if spacing.is_empty() {
        f64::NAN
    } else {
        spacing.iter().sum::<f64>() / spacing.len() as f64
    };
    let mut sorted = spacing.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let spacing_median_km = if sorted.is_empty() {
        f64::NAN
    } else {
        sorted[sorted.len() / 2]
    };

    let land_cells = land
        .land
        .iter()
        .zip(land.lake.iter())
        .filter(|&(&l, &k)| l && !k)
        .count();
    let land_area_km2 = land_cells as f64 * land.cell_area_km2;
    let density = n as f64 / land_area_km2;
    let clark_evans_r = if n > 1 && density > 0.0 {
        spacing_mean_km / (0.5 / density.sqrt())
    } else {
        f64::NAN
    };

    // Coastal = within 5 km of ocean, measured on the straight-line grid.
    let coast_cells: Vec<u32> = (0..land.w * land.h)
        .filter(|&i| !land.land[i as usize])
        .collect();
    let coastal = if coast_cells.is_empty() {
        0
    } else {
        let w = land.w as i64;
        let r_cells = (5000.0 / land.dx).powi(2);
        cells
            .iter()
            .filter(|&&c| {
                let (x, y) = ((c as i64 % w) as f64, (c as i64 / w) as f64);
                coast_cells.iter().any(|&o| {
                    let (ox, oy) = ((o as i64 % w) as f64, (o as i64 / w) as f64);
                    (x - ox).powi(2) + (y - oy).powi(2) <= r_cells
                })
            })
            .count()
    };

    // Kvamme gain against the productivity field's top quartile.
    let mut prod: Vec<f64> = (0..land.w as usize * land.h as usize)
        .filter(|&i| land.land[i] && !land.lake[i])
        .map(|i| land.productivity[i])
        .collect();
    prod.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q75 = if prod.is_empty() {
        f64::NAN
    } else {
        prod[(prod.len() * 3) / 4]
    };
    let area_share = 0.25;
    let site_share = if n > 0 {
        cells
            .iter()
            .filter(|&&c| land.productivity[c as usize] >= q75)
            .count() as f64
            / n as f64
    } else {
        f64::NAN
    };
    let kvamme_gain = if site_share > 0.0 {
        1.0 - area_share / site_share
    } else {
        f64::NAN
    };

    Measurements {
        centers: n,
        candidates_offered,
        people_allocated: allocated,
        largest_center: sizes.first().copied().unwrap_or(0.0),
        primacy: if total > 0.0 {
            sizes.first().copied().unwrap_or(0.0) / total
        } else {
            f64::NAN
        },
        rank_size_zeta_all: zeta_all,
        rank_size_r2_all: r2_all,
        rank_size_zeta_tail: zeta_tail,
        rank_size_r2_tail: r2_tail,
        rank_size_tail_n: tail_n,
        agglomerations: agg.len(),
        agglomeration_zeta: zeta_agg,
        agglomeration_r2: r2_agg,
        largest_agglomeration: agg.first().copied().unwrap_or(0.0),
        agglomeration_primacy: if total > 0.0 {
            agg.first().copied().unwrap_or(0.0) / total
        } else {
            f64::NAN
        },
        delineation_km,
        spacing_mean_km,
        spacing_median_km,
        clark_evans_r,
        centers_per_1000km2: density * 1000.0,
        mean_travel_hours,
        coastal_share: if n > 0 {
            coastal as f64 / n as f64
        } else {
            f64::NAN
        },
        kvamme_gain,
        network_km,
        network_reuse_frac: if network_km > 0.0 {
            network_reuse_km / network_km
        } else {
            f64::NAN
        },
        network_km_per_center: if n > 0 {
            network_km / n as f64
        } else {
            f64::NAN
        },
        converged,
        iterations,
    }
}
