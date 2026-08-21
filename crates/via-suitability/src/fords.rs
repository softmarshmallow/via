//! Ford crossability — the hydraulic chain (ADR 0011 D4, research 0013
//! §Fords): k_Q turns via's relative discharge into m³/s (the
//! lumped-calibration practice of the stream-power literature, Whipple &
//! Tucker 1999 — pure declared forcing, unverifiable from inside via);
//! channel width follows Finnegan et al. (2005),
//! W = [α(α+2)^(2/3)]^(3/8) · (nQ)^(3/8) · S^(−3/16), with α fit by
//! substrate in the paper (via defaults 20, a declared choice near the
//! cobble-bed 21); Manning (1891) closes depth and velocity in the
//! wide-channel form (R ≈ d): d = (n·(Q/W)/√S)^(3/5), v = Q/(W·d),
//! with n from the Chow (1959) tables. Slope is reach-averaged along
//! the receivers path over the water surface (single-cell slopes are
//! noisy at cm quantization). Each link is cited; the chain's assembly
//! is via's, stated as such (ADR 0011 D4).
//!
//! Crossability is the flume-verified D·V product (Cox, Shand & Blacka
//! 2010; AIDR Guideline 7-3), kept continuous — no bands are baked; the
//! published bands are consumer config. On the river graph the composite
//! is computed from the rounded f32 factors so the ADR 0011 D6 identity
//! (crossability == depth × velocity, exactly, in f32) holds on disk; on
//! standing water crossability is the still-water depth alone; elsewhere
//! it is zero (no water to cross).
//!
//! Declared rule for drowned reaches: a river cell carrying standing
//! water (water_depth > 0 — a lake over the channel) takes the
//! still-water branch, per the dossier's "D·V along the river graph,
//! still-water depth alone on lake cells".

/// Per-cell hydraulic fields. All f32 — the emitted artifact precision;
/// the crossability identity is exact in that precision.
pub struct FordFields {
    pub width_m: Vec<f32>,
    pub depth_m: Vec<f32>,
    pub velocity_ms: Vec<f32>,
    /// D·V (m²/s) on river cells; still-water depth (m) on standing
    /// water; 0 elsewhere.
    pub crossability: Vec<f32>,
    /// Reach-averaged channel slope on river cells (0 elsewhere). Kept
    /// for the navigability predicate; not an emitted artifact.
    pub channel_slope: Vec<f64>,
}

/// Config surface of the chain; every value declared with provenance in
/// the stage config (ADR 0011 D1).
pub struct FordParams {
    /// k_Q, m³/s per relative discharge unit (Tier-2 forcing).
    pub k_q_m3s_per_unit: f64,
    /// Manning roughness n (Chow 1959 natural streams ~0.030–0.050).
    pub manning_n: f64,
    /// Finnegan width-to-depth ratio α. The paper fits α by substrate
    /// (Fig. 1: 5 bedrock, 9 boulder, 21 cobble, 59 gravel); via's
    /// default 20 is a declared choice near the cobble-bed fit.
    pub finnegan_alpha: f64,
    /// Reach length, in cells along the receivers path, for the
    /// reach-averaged slope. Declared parameter.
    pub slope_reach_cells: u32,
    /// Numerical slope floor (declared): Finnegan has S^(−3/16) and
    /// Manning √S, so a flat routed reach must not divide by zero.
    pub min_channel_slope: f64,
}

/// Water-surface slope, reach-averaged along the receivers path: walk up
/// to `reach` steps downstream, stop before leaving land, divide total
/// drop by along-ground distance. Deterministic — a pure function of the
/// artifacts.
fn reach_slope(
    w: u32,
    i: u32,
    surface_m: &[f64],
    receivers: &[u32],
    land: &[bool],
    dx: f64,
    p: &FordParams,
) -> f64 {
    let mut c = i;
    let mut dist = 0.0f64;
    for _ in 0..p.slope_reach_cells {
        let r = receivers[c as usize];
        if r == c {
            break; // base level
        }
        let (cx, cy) = ((c % w) as i64, (c / w) as i64);
        let (rx, ry) = ((r % w) as i64, (r / w) as i64);
        let diag = (rx - cx).abs() == 1 && (ry - cy).abs() == 1;
        dist += if diag {
            std::f64::consts::SQRT_2 * dx
        } else {
            dx
        };
        c = r;
        if !land[c as usize] {
            break; // the step to base level ends the reach
        }
    }
    if dist <= 0.0 {
        return p.min_channel_slope;
    }
    let drop = surface_m[i as usize] - surface_m[c as usize];
    (drop / dist).max(p.min_channel_slope)
}

/// Compute the hydraulic fields. `surface_m` is the water surface:
/// effective heights (terrain + standing water) on land, sea level on
/// ocean — so a mouth reach drops to the sea, not through bathymetry.
#[allow(clippy::too_many_arguments)]
pub fn compute(
    w: u32,
    h: u32,
    dx: f64,
    surface_m: &[f64],
    receivers: &[u32],
    land: &[bool],
    strahler: &[u32],
    water_depth: &[f32],
    discharge: &[f32],
    p: &FordParams,
) -> FordFields {
    let n_cells = w as usize * h as usize;
    let mut width_m = vec![0.0f32; n_cells];
    let mut depth_m = vec![0.0f32; n_cells];
    let mut velocity_ms = vec![0.0f32; n_cells];
    let mut crossability = vec![0.0f32; n_cells];
    let mut channel_slope = vec![0.0f64; n_cells];

    // Finnegan's closed-form coefficient [α(α+2)^(2/3)]^(3/8).
    let alpha = p.finnegan_alpha;
    let c_w = (alpha * (alpha + 2.0).powf(2.0 / 3.0)).powf(3.0 / 8.0);

    for i in 0..n_cells as u32 {
        let ii = i as usize;
        if water_depth[ii] > 0.0 && land[ii] {
            // Still water (lake over land — possibly a drowned reach):
            // crossability is the still-water depth alone.
            depth_m[ii] = water_depth[ii];
            crossability[ii] = depth_m[ii];
            continue;
        }
        if strahler[ii] == 0 || !land[ii] {
            continue; // dry land or ocean: nothing to cross here
        }
        let q = p.k_q_m3s_per_unit * discharge[ii] as f64;
        if q <= 0.0 {
            continue;
        }
        let s = reach_slope(w, i, surface_m, receivers, land, dx, p);
        channel_slope[ii] = s;
        let width = c_w * (p.manning_n * q).powf(3.0 / 8.0) * s.powf(-3.0 / 16.0);
        let depth = (p.manning_n * (q / width) / s.sqrt()).powf(3.0 / 5.0);
        let velocity = q / (width * depth);
        width_m[ii] = width as f32;
        depth_m[ii] = depth as f32;
        velocity_ms[ii] = velocity as f32;
        // The composite from the rounded factors — the D6 identity is
        // exact in the emitted precision.
        crossability[ii] = depth_m[ii] * velocity_ms[ii];
    }
    FordFields {
        width_m,
        depth_m,
        velocity_ms,
        crossability,
        channel_slope,
    }
}

/// The four ford rasters as read back from disk — the emitted artifacts
/// the identity gate is evaluated on, never the in-memory copies.
pub struct FordDiskFields<'a> {
    pub width_m: &'a [f32],
    pub depth_m: &'a [f32],
    pub velocity_ms: &'a [f32],
    pub crossability: &'a [f32],
}

/// ADR 0011 D6 spectrum-identity gate, evaluated on the emitted rasters
/// (the caller reads them back from disk): on channel cells crossability
/// == depth × velocity exactly in f32; on standing-water land cells
/// crossability == depth == the terrain water_depth artifact; everywhere
/// else all four fields are zero. Branch membership is recomputed from
/// the terrain artifacts.
pub fn verify_identities(
    land: &[bool],
    strahler: &[u32],
    water_depth: &[f32],
    fields: &FordDiskFields,
) -> bool {
    let n = land.len();
    for i in 0..n {
        let (d, v, c) = (
            fields.depth_m[i],
            fields.velocity_ms[i],
            fields.crossability[i],
        );
        let ok = if water_depth[i] > 0.0 && land[i] {
            c == d && d == water_depth[i] && v == 0.0
        } else if strahler[i] > 0 && land[i] {
            c == d * v
        } else {
            c == 0.0 && d == 0.0 && v == 0.0 && fields.width_m[i] == 0.0
        };
        if !ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> FordParams {
        FordParams {
            k_q_m3s_per_unit: 1.0,
            manning_n: 0.035,
            finnegan_alpha: 20.0,
            slope_reach_cells: 5,
            min_channel_slope: 1.0e-5,
        }
    }

    // (w, h, surface_m, receivers, land, strahler, water_depth, discharge)
    type Fixture = (
        u32,
        u32,
        Vec<f64>,
        Vec<u32>,
        Vec<bool>,
        Vec<u32>,
        Vec<f32>,
        Vec<f32>,
    );

    // 6×3, all land except the west column (ocean): a river along row 1
    // dropping 1 m per 100 m cell, discharge 100 units everywhere.
    fn fixture() -> Fixture {
        let (w, h) = (6u32, 3u32);
        let n = (w * h) as usize;
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        let mut surface = vec![10.0f64; n];
        let mut receivers: Vec<u32> = (0..n as u32).collect();
        let mut land = vec![true; n];
        let mut strahler = vec![0u32; n];
        let water = vec![0.0f32; n];
        let discharge = vec![100.0f32; n];
        for y in 0..h {
            land[idx(0, y)] = false;
            surface[idx(0, y)] = 0.0; // sea level
        }
        for x in 1..w {
            let i = idx(x, 1);
            receivers[i] = idx(x - 1, 1) as u32;
            strahler[i] = 1;
            surface[i] = x as f64; // 1 m drop per cell
        }
        (w, h, surface, receivers, land, strahler, water, discharge)
    }

    #[test]
    fn manning_and_continuity_close_consistently() {
        let (w, h, surface, receivers, land, strahler, water, discharge) = fixture();
        let f = compute(
            w,
            h,
            100.0,
            &surface,
            &receivers,
            &land,
            &strahler,
            &water,
            &discharge,
            &params(),
        );
        let i = (w + 3) as usize; // river cell (3,1)
        let (wd, d, v) = (
            f.width_m[i] as f64,
            f.depth_m[i] as f64,
            f.velocity_ms[i] as f64,
        );
        // Continuity: Q = W·d·v.
        assert!((wd * d * v - 100.0).abs() / 100.0 < 1e-5);
        // Manning: v = (1/n)·d^(2/3)·√S with the reach slope 0.01.
        let v_manning = (1.0 / 0.035) * d.powf(2.0 / 3.0) * 0.01f64.sqrt();
        assert!((v - v_manning).abs() / v_manning < 1e-5);
        // The composite is the exact f32 product of the emitted factors.
        assert_eq!(f.crossability[i], f.depth_m[i] * f.velocity_ms[i]);
        assert!(verify_identities(
            &land,
            &strahler,
            &water,
            &FordDiskFields {
                width_m: &f.width_m,
                depth_m: &f.depth_m,
                velocity_ms: &f.velocity_ms,
                crossability: &f.crossability,
            },
        ));
    }

    #[test]
    fn reach_slope_averages_over_the_path() {
        let (w, _h, mut surface, receivers, land, _strahler, _water, _discharge) = fixture();
        // Make the local step at (4,1) flat but keep drop farther down.
        surface[(w + 4) as usize] = surface[(w + 3) as usize];
        let s = reach_slope(w, w + 4, &surface, &receivers, &land, 100.0, &params());
        // 4 steps: (4,1)->(3,1)->(2,1)->(1,1)->(0,1 ocean, stop): drop
        // 3 m (surface 3->0) over 400 m.
        assert!((s - 3.0 / 400.0).abs() < 1e-12);
        // A reach of 1 sees only the flat step and hits the floor.
        let mut short = params();
        short.slope_reach_cells = 1;
        let s1 = reach_slope(w, w + 4, &surface, &receivers, &land, 100.0, &short);
        assert_eq!(s1, 1.0e-5);
    }

    #[test]
    fn standing_water_takes_the_still_water_branch() {
        let (w, h, surface, receivers, land, strahler, mut water, discharge) = fixture();
        water[(w + 3) as usize] = 1.5; // lake over the channel at (3,1)
        let f = compute(
            w,
            h,
            100.0,
            &surface,
            &receivers,
            &land,
            &strahler,
            &water,
            &discharge,
            &params(),
        );
        let i = (w + 3) as usize;
        assert_eq!(f.crossability[i], 1.5);
        assert_eq!(f.depth_m[i], 1.5);
        assert_eq!(f.velocity_ms[i], 0.0);
        assert!(verify_identities(
            &land,
            &strahler,
            &water,
            &FordDiskFields {
                width_m: &f.width_m,
                depth_m: &f.depth_m,
                velocity_ms: &f.velocity_ms,
                crossability: &f.crossability,
            },
        ));
        // Dry non-river land stays zero.
        assert_eq!(f.crossability[2], 0.0);
    }

    #[test]
    fn identity_gate_rejects_tampered_composites() {
        let (w, h, surface, receivers, land, strahler, water, discharge) = fixture();
        let mut f = compute(
            w,
            h,
            100.0,
            &surface,
            &receivers,
            &land,
            &strahler,
            &water,
            &discharge,
            &params(),
        );
        let i = (w + 3) as usize;
        f.crossability[i] += 0.25;
        assert!(!verify_identities(
            &land,
            &strahler,
            &water,
            &FordDiskFields {
                width_m: &f.width_m,
                depth_m: &f.depth_m,
                velocity_ms: &f.velocity_ms,
                crossability: &f.crossability,
            },
        ));
    }
}
