//! Harbour component spectra (ADR 0011 D4, research 0013 §Harbours).
//! Three components, shipped separately — no published composite harbour
//! index exists, so none is baked (ADR 0011 Rejected); any combination
//! is declared config in the consumer.
//!
//! - **Wave fetch F** (Burrows, Harvey & Robb 2008): per coastal-water
//!   cell, distance to nearest land in each of 16 angular sectors,
//!   capped at 200 km (the wave transition point gF/U² < 22,000);
//!   F = mean per-sector fetch, then the paper's neighbour-averaging
//!   smoothing step. Declared adaptations: the focal cells are coastal
//!   *water* cells (ocean cells with a land D8 neighbour); rays are
//!   marched exactly at cell resolution instead of the paper's
//!   three-scale hierarchical search (an accuracy-conservative
//!   substitution — the search approximates exactly this ray); a ray
//!   leaving the grid reads open ocean (the border ring is ocean by
//!   construction). The wind-weighted family degenerates to F under a
//!   uniform rose, which is why F is the honest form here.
//! - **Depth window**: linear ramp between published anchors — dead at
//!   ~1 m of water column (Salomon et al. 2016, Portus/Ostia), full at
//!   the large-ship draught ~4.5 m (Boetto 2010); ordinary merchantmen
//!   (~1–3.5 m draught) fall on the rising limb. The ramp joining the
//!   anchors is a declared interpolation. Ocean depth is
//!   sea_level − heights (the water_depth artifact is 0 on ocean).
//! - **Sediment-supply penalty** (named heuristic, motivated by
//!   Marriner & Morhange 2007): for ocean cells near river outlets,
//!   Σ over mouths within a declared radius of q_mouth / distance,
//!   with q in relative discharge units — a ranking, not a rate.

/// Per-cell harbour component spectra. All zero outside their domains
/// (fetch: coastal water; depth window: ocean; sediment: ocean near
/// outlets).
pub struct HarbourFields {
    /// Mean per-sector wave fetch, metres, in [0, cap].
    pub fetch_m: Vec<f32>,
    /// Depth-window ramp in [0, 1].
    pub depth_window: Vec<f32>,
    /// Sediment-supply penalty, relative units (ranking only).
    pub sediment: Vec<f32>,
}

pub struct HarbourParams {
    /// Angular sectors (Burrows et al. 2008 uses 16; 32 is the later
    /// data products' choice — a declared adaptation if used).
    pub sectors: u32,
    /// Fetch cap, metres (200 km, wave transition provenance).
    pub cap_m: f64,
    /// Water column at which a harbour is dead (~1 m).
    pub depth_dead_m: f64,
    /// Water column serving large ships (~4.5 m); ramp saturates here.
    pub depth_full_m: f64,
    /// Radius of the sediment penalty around river outlets, metres
    /// (declared heuristic scale).
    pub sediment_radius_m: f64,
}

/// March one ray from cell `c` at `angle` (radians, from grid north,
/// clockwise): distance in metres to the nearest land cell, capped;
/// leaving the grid is open ocean (cap).
fn ray_fetch(w: u32, h: u32, dx: f64, land: &[bool], c: u32, angle: f64, cap_m: f64) -> f64 {
    let (x0, y0) = (c % w, c / w);
    let (ux, uy) = (angle.sin(), -angle.cos()); // north = -y (row 0 is north)
    let steps = (cap_m / dx).ceil() as i64;
    for t in 1..=steps {
        let px = x0 as f64 + ux * t as f64;
        let py = y0 as f64 + uy * t as f64;
        let (cx, cy) = (px.round() as i64, py.round() as i64);
        if cx < 0 || cy < 0 || cx >= w as i64 || cy >= h as i64 {
            return cap_m; // off-grid: open ocean
        }
        if land[(cy * w as i64 + cx) as usize] {
            return (t as f64 * dx).min(cap_m);
        }
    }
    cap_m
}

/// Compute the three component spectra.
#[allow(clippy::too_many_arguments)]
pub fn compute(
    w: u32,
    h: u32,
    dx: f64,
    heights_m: &[f64],
    sea_level_m: f64,
    land: &[bool],
    receivers: &[u32],
    strahler: &[u32],
    discharge: &[f32],
    p: &HarbourParams,
) -> HarbourFields {
    let n = w as usize * h as usize;
    let mut fetch_m = vec![0.0f32; n];
    let mut depth_window = vec![0.0f32; n];
    let mut sediment = vec![0.0f32; n];

    // Domain: coastal water = ocean cells with a land D8 neighbour.
    let mut coastal = vec![false; n];
    for i in 0..n as u32 {
        if land[i as usize] {
            continue;
        }
        let mut has_land_nb = false;
        crate::for_neighbors8(w, h, i, |nb, _| {
            if land[nb as usize] {
                has_land_nb = true;
            }
        });
        coastal[i as usize] = has_land_nb;
    }

    // Fetch: exact ray march per sector, then the neighbour-averaging
    // smoothing step over the coastal-water domain.
    let mut raw_fetch = vec![0.0f64; n];
    for i in 0..n as u32 {
        let ii = i as usize;
        if !coastal[ii] {
            continue;
        }
        let mut sum = 0.0f64;
        for k in 0..p.sectors {
            let angle = std::f64::consts::TAU * k as f64 / p.sectors as f64;
            sum += ray_fetch(w, h, dx, land, i, angle, p.cap_m);
        }
        raw_fetch[ii] = sum / p.sectors as f64;
    }
    for i in 0..n as u32 {
        let ii = i as usize;
        if !coastal[ii] {
            continue;
        }
        let (mut sum, mut count) = (raw_fetch[ii], 1u32);
        crate::for_neighbors8(w, h, i, |nb, _| {
            if coastal[nb as usize] {
                sum += raw_fetch[nb as usize];
                count += 1;
            }
        });
        fetch_m[ii] = (sum / count as f64) as f32;
    }

    // Depth window on ocean cells: linear ramp between the anchors.
    for i in 0..n {
        if land[i] {
            continue;
        }
        let depth = sea_level_m - heights_m[i];
        let t = (depth - p.depth_dead_m) / (p.depth_full_m - p.depth_dead_m);
        depth_window[i] = t.clamp(0.0, 1.0) as f32;
    }

    // Sediment penalty: river mouths are ocean cells with a river donor;
    // each contributes q_donor / distance within the declared radius
    // (straight-line over water — a heuristic ranking, land blocking
    // ignored).
    let radius_cells = (p.sediment_radius_m / dx).ceil() as i64;
    let mut acc = vec![0.0f64; n];
    for m in 0..n as u32 {
        let mi = m as usize;
        if land[mi] {
            continue;
        }
        let mut q_mouth = 0.0f64;
        crate::for_neighbors8(w, h, m, |nb, _| {
            let nbi = nb as usize;
            if receivers[nbi] == m && nb != m && strahler[nbi] > 0 {
                q_mouth = q_mouth.max(discharge[nbi] as f64);
            }
        });
        if q_mouth <= 0.0 {
            continue;
        }
        let (mx, my) = ((m % w) as i64, (m / w) as i64);
        for gy in (my - radius_cells).max(0)..=(my + radius_cells).min(h as i64 - 1) {
            for gx in (mx - radius_cells).max(0)..=(mx + radius_cells).min(w as i64 - 1) {
                let i = (gy * w as i64 + gx) as usize;
                if land[i] {
                    continue;
                }
                let dist = (((gx - mx).pow(2) + (gy - my).pow(2)) as f64).sqrt() * dx;
                if dist > p.sediment_radius_m {
                    continue;
                }
                acc[i] += q_mouth / dist.max(dx);
            }
        }
    }
    for i in 0..n {
        sediment[i] = acc[i] as f32;
    }

    HarbourFields {
        fetch_m,
        depth_window,
        sediment,
    }
}

/// ADR 0011 D6 gate: fetch values lie in [0, cap]. Evaluated on the
/// emitted raster.
pub fn verify_fetch_bounds(fetch_m: &[f32], cap_m: f64) -> bool {
    fetch_m
        .iter()
        .all(|&f| f.is_finite() && f >= 0.0 && f as f64 <= cap_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> HarbourParams {
        HarbourParams {
            sectors: 16,
            cap_m: 200_000.0,
            depth_dead_m: 1.0,
            depth_full_m: 4.5,
            sediment_radius_m: 10_000.0,
        }
    }

    // 16×16, sea level 0: a land block in the north with a one-cell bay
    // notch, open ocean to the south.
    fn fixture() -> (u32, u32, Vec<f64>, Vec<bool>) {
        let (w, h) = (16u32, 16u32);
        let n = (w * h) as usize;
        let mut heights = vec![-10.0f64; n];
        let mut land = vec![false; n];
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        for y in 0..6 {
            for x in 0..w {
                land[idx(x, y)] = true;
                heights[idx(x, y)] = 5.0;
            }
        }
        // The bay: a water notch at (8, 5) surrounded by land on three
        // sides (its mouth opens south).
        land[idx(8, 5)] = false;
        heights[idx(8, 5)] = -3.0;
        (w, h, heights, land)
    }

    #[test]
    fn bay_is_more_sheltered_than_open_coast() {
        let (w, h, heights, land) = fixture();
        let receivers: Vec<u32> = (0..(w * h)).collect();
        let strahler = vec![0u32; (w * h) as usize];
        let discharge = vec![0.0f32; (w * h) as usize];
        let f = compute(
            w,
            h,
            100.0,
            &heights,
            0.0,
            &land,
            &receivers,
            &strahler,
            &discharge,
            &params(),
        );
        let bay = (5 * w + 8) as usize;
        let open = (6 * w + 2) as usize; // coastal cell on the straight shore
        assert!(f.fetch_m[bay] > 0.0);
        assert!(
            f.fetch_m[bay] < f.fetch_m[open],
            "bay {} !< open coast {}",
            f.fetch_m[bay],
            f.fetch_m[open]
        );
        assert!(verify_fetch_bounds(&f.fetch_m, 200_000.0));
        // Non-coastal cells carry no fetch.
        assert_eq!(f.fetch_m[(12 * w) as usize], 0.0);
    }

    #[test]
    fn depth_window_ramps_between_the_anchors() {
        let (w, h, mut heights, land) = fixture();
        let receivers: Vec<u32> = (0..(w * h)).collect();
        let strahler = vec![0u32; (w * h) as usize];
        let discharge = vec![0.0f32; (w * h) as usize];
        heights[(10 * w + 1) as usize] = -0.5; // shallower than dead
        heights[(10 * w + 2) as usize] = -2.75; // mid-ramp: t = 0.5
        heights[(10 * w + 3) as usize] = -8.0; // beyond full
        let f = compute(
            w,
            h,
            100.0,
            &heights,
            0.0,
            &land,
            &receivers,
            &strahler,
            &discharge,
            &params(),
        );
        assert_eq!(f.depth_window[(10 * w + 1) as usize], 0.0);
        assert!((f.depth_window[(10 * w + 2) as usize] - 0.5).abs() < 1e-6);
        assert_eq!(f.depth_window[(10 * w + 3) as usize], 1.0);
        // Land carries no window.
        assert_eq!(f.depth_window[0], 0.0);
    }

    #[test]
    fn sediment_penalty_ranks_by_outlet_proximity() {
        let (w, h, heights, mut land) = fixture();
        let n = (w * h) as usize;
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        // A river reaching the shore at (4,5): make (4,5) a land river
        // cell draining into ocean (4,6).
        land[idx(4, 5)] = true;
        let mut receivers: Vec<u32> = (0..(w * h)).collect();
        let mut strahler = vec![0u32; n];
        let mut discharge = vec![0.0f32; n];
        receivers[idx(4, 5)] = idx(4, 6) as u32;
        strahler[idx(4, 5)] = 1;
        discharge[idx(4, 5)] = 500.0;
        let f = compute(
            w,
            h,
            100.0,
            &heights,
            0.0,
            &land,
            &receivers,
            &strahler,
            &discharge,
            &params(),
        );
        let near = idx(4, 7);
        let far = idx(12, 15);
        assert!(f.sediment[near] > 0.0);
        assert!(f.sediment[near] > f.sediment[far]);
    }

    #[test]
    fn fetch_bounds_gate_rejects_out_of_range() {
        assert!(verify_fetch_bounds(&[0.0, 100.0, 200_000.0], 200_000.0));
        assert!(!verify_fetch_bounds(&[-1.0], 200_000.0));
        assert!(!verify_fetch_bounds(&[200_001.0], 200_000.0));
        assert!(!verify_fetch_bounds(&[f32::NAN], 200_000.0));
    }
}
