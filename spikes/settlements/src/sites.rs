//! Candidate sites: the places a settlement *could* stand.
//!
//! Deliberately generous and era-blind — the same candidate set is handed
//! to every era, so any difference in the settlement pattern is the
//! dynamics' doing, not the candidate list's. Selection is greedy by local
//! score with a thinning radius that must stay far below the spacing we
//! expect to emerge (day-walk markets are 5–10 km apart; the thinning is
//! ~1.5 km), or the spike would be measuring its own discretization.

use serde::{Deserialize, Serialize};

use crate::fields::Land;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    /// Minimum separation between candidates (m).
    pub thin_radius_m: f64,
    /// Radius (m) over which a candidate's local food base is summed.
    pub catchment_radius_m: f64,
    /// Hard cap on candidates (cost matrix is O(sites × blocks)).
    pub max_sites: u32,
    /// Candidates must clear this buildability.
    pub min_site_quality: f64,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            thin_radius_m: 1500.0,
            catchment_radius_m: 3000.0,
            max_sites: 1200,
            min_site_quality: 0.25,
        }
    }
}

pub struct Sites {
    /// Cell index of each candidate.
    pub cell: Vec<u32>,
    /// Buildability at the site, the fixed part of its attractiveness.
    pub quality: Vec<f64>,
    /// Food base within the catchment radius (people).
    pub local_capacity: Vec<f64>,
}

impl Sites {
    pub fn len(&self) -> usize {
        self.cell.len()
    }
}

pub fn select(land: &Land, cfg: &SiteConfig) -> Sites {
    let n = land.w as usize * land.h as usize;
    let r_cells = (cfg.catchment_radius_m / land.dx).round() as i64;

    // Local food base by box sum (a disc would be tidier; the box is
    // separable and this is a candidate score, not a physical quantity).
    let (w, h) = (land.w as i64, land.h as i64);
    let mut row_sum = vec![0.0f64; n];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for k in (x - r_cells).max(0)..=(x + r_cells).min(w - 1) {
                s += land.population_capacity((y * w + k) as usize);
            }
            row_sum[(y * w + x) as usize] = s;
        }
    }
    let mut local = vec![0.0f64; n];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for k in (y - r_cells).max(0)..=(y + r_cells).min(h - 1) {
                s += row_sum[(k * w + x) as usize];
            }
            local[(y * w + x) as usize] = s;
        }
    }

    // Score: a buildable spot with food around it. Sorted descending, ties
    // on cell index.
    let mut ranked: Vec<(u64, u32)> = (0..n)
        .filter(|&i| land.land[i] && !land.lake[i] && land.site_quality[i] >= cfg.min_site_quality)
        .map(|i| {
            let score = land.site_quality[i] * local[i];
            (crate::util::f64_key(score), i as u32)
        })
        .collect();
    ranked.sort_unstable_by_key(|&(k, i)| (std::cmp::Reverse(k), i));

    let thin_cells = (cfg.thin_radius_m / land.dx).max(1.0);
    let thin2 = thin_cells * thin_cells;
    // Bucket accepted sites by a coarse grid so the separation test stays
    // local rather than scanning every accepted site.
    let bucket = thin_cells.ceil() as i64;
    let bw = (w / bucket + 1) as usize;
    let bh = (h / bucket + 1) as usize;
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); bw * bh];

    let mut cell = Vec::new();
    for &(_, i) in &ranked {
        if cell.len() as u32 >= cfg.max_sites {
            break;
        }
        let (x, y) = (i as i64 % w, i as i64 / w);
        let (bx, by) = (x / bucket, y / bucket);
        let mut ok = true;
        'outer: for ny in (by - 1).max(0)..=(by + 1).min(bh as i64 - 1) {
            for nx in (bx - 1).max(0)..=(bx + 1).min(bw as i64 - 1) {
                for &c in &buckets[(ny * bw as i64 + nx) as usize] {
                    let (cx, cy) = (c as i64 % w, c as i64 / w);
                    let (ddx, ddy) = ((x - cx) as f64, (y - cy) as f64);
                    if ddx * ddx + ddy * ddy < thin2 {
                        ok = false;
                        break 'outer;
                    }
                }
            }
        }
        if ok {
            buckets[(by * bw as i64 + bx) as usize].push(i);
            cell.push(i);
        }
    }

    let quality = cell
        .iter()
        .map(|&c| land.site_quality[c as usize])
        .collect();
    let local_capacity = cell.iter().map(|&c| local[c as usize]).collect();
    Sites {
        cell,
        quality,
        local_capacity,
    }
}
