//! Navigability and head of navigation (ADR 0011 D4, research 0013
//! §Navigability).
//!
//! The continuous spectrum is Langbein's (1962) minimum specific
//! tractive force Ts = V²(f+0.6)/(1600·D^(4/3)) — V in ft/s, D in ft:
//! Ts is dimensionless but the 1600 is unit-bearing (it embeds imperial
//! Manning with the paper's n = 0.03), so SI inputs are converted
//! before use. Langbein's f is the shallow-water to deep-water
//! vessel-resistance ratio of his Fig. 8, evaluated at the paper's
//! draft = 0.7·D convention — f ≥ 1, a hull property, NOT the bed's
//! Darcy–Weisbach friction factor (a first landing wrongly used the
//! Darcy conversion from Manning n, understating Ts ~4–7×; caught in
//! review against the primary source). f ships as the declared config
//! curve of his Figure 8, digitized and validated against his own
//! figure. Published anchor: Ts > 0.002 usually considered unnavigable.
//!
//! The banded predicate combines that anchor with the Magirl & Olsen
//! (2009) slope bands (dimensionless, applied as-is: probably not
//! navigable above 0.0047; 0.0019–0.0047 indeterminate) and the
//! pre-modern depth anchor (Eckoldt 0.3–0.7 m for keelless barges,
//! quoted via Appel et al. 2024). All three thresholds are config with
//! this provenance. Still water is navigable by depth alone (V = 0 ⇒
//! Ts = 0).
//!
//! Head of navigation is compositional (ADR 0011 D6): the cells of the
//! mouth-connected navigable set — connected downstream to a river
//! mouth along the receivers tree, through navigable cells only — that
//! have no donor in that set. An isolated navigable pocket above an
//! unnavigable reach is not a head of navigation.
//!
//! The Filet et al. (2025) change-point cross-check adopted by ADR 0011
//! is not implemented here; its deferral to the chunk's
//! QA/visualization slice is recorded in ADR 0011's Consequences,
//! alongside the basin-boundary col diagnostic.

use serde::Serialize;

use crate::fords::FordFields;

const FT_PER_M: f64 = 3.280_839_895;

/// Per-cell navigability fields. Outside the water domain (dry land,
/// ocean) Ts carries the f32::MAX sentinel and the predicate is 0.
pub struct NavigabilityFields {
    /// Langbein Ts (dimensionless); 0 on still water; f32::MAX outside
    /// the water domain.
    pub ts: Vec<f32>,
    /// The banded predicate: 1 = navigable under the config thresholds.
    pub navigable: Vec<u32>,
}

pub struct NavParams {
    /// Langbein anchor: Ts above this is unnavigable (published 0.002).
    pub max_ts: f64,
    /// Magirl & Olsen slope band: probably not navigable above (0.0047).
    pub max_slope: f64,
    /// Pre-modern depth anchor, m (Eckoldt 0.3–0.7 band).
    pub min_depth_m: f64,
    /// Upper Froude number of Langbein's Figure 8. Above it the figure
    /// says nothing, so the reach is declared unnavigable by domain
    /// rather than assigned an extrapolated f (ADR 0011 D4 as amended).
    pub max_froude: f64,
}

/// Langbein (1962) Figure 8, digitized at the paper's draft convention
/// d/D = 0.7 and validated against his Figure 11 to ~15% (research 0015
/// §3): the shallow-water vessel-resistance ratio f against the Froude
/// number F = V/√(gD). Linear interpolation between the tabulated
/// curves; below the first curve the shallow-water effect has vanished
/// and f → 1 (deep-water resistance).
const LANGBEIN_FIG8: [(f64, f64); 4] = [(0.25, 1.21), (0.50, 2.48), (0.75, 4.71), (0.90, 7.8)];

pub fn langbein_f(froude: f64) -> f64 {
    if froude <= LANGBEIN_FIG8[0].0 {
        // The F = 0.25 curve merges with the f = 1 axis at low draft
        // ratios; interpolate down to unity rather than clamping, so a
        // still reach is not charged a shallow-water penalty.
        let (f0, v0) = LANGBEIN_FIG8[0];
        return 1.0 + (v0 - 1.0) * (froude / f0);
    }
    for pair in LANGBEIN_FIG8.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if froude <= x1 {
            return y0 + (y1 - y0) * (froude - x0) / (x1 - x0);
        }
    }
    LANGBEIN_FIG8[LANGBEIN_FIG8.len() - 1].1
}

/// Standard gravity, m/s² — for the Froude number the f lookup keys on.
const G: f64 = 9.806_65;

/// One head-of-navigation site, in ascending cell order (ADR 0011 D3).
#[derive(Clone, Debug, Serialize)]
pub struct HeadOfNavSite {
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    /// Water depth at the head (channel depth, or still-water depth).
    pub depth_m: f32,
    pub velocity_ms: f32,
    /// Langbein Ts at the head.
    pub ts: f32,
}

/// Compute the Langbein spectrum and the banded predicate.
pub fn compute(
    land: &[bool],
    strahler: &[u32],
    water_depth: &[f32],
    ford: &FordFields,
    p: &NavParams,
) -> NavigabilityFields {
    let n = land.len();
    let mut ts = vec![f32::MAX; n];
    let mut navigable = vec![0u32; n];
    for i in 0..n {
        if !land[i] {
            continue;
        }
        if water_depth[i] > 0.0 {
            // Still water: V = 0 ⇒ Ts = 0; navigable by depth alone.
            ts[i] = 0.0;
            navigable[i] = u32::from(water_depth[i] as f64 >= p.min_depth_m);
            continue;
        }
        if strahler[i] == 0 {
            continue;
        }
        let d = ford.depth_m[i] as f64;
        let v = ford.velocity_ms[i] as f64;
        if d <= 0.0 {
            continue;
        }
        // Langbein eq. 15 in imperial units, with f read from his
        // Figure 8 at the reach's own Froude number (see module doc).
        // Beyond the figure's domain the reach is a rapid: declare it
        // unnavigable rather than extrapolate the curve.
        let froude = v / (G * d).sqrt();
        if froude > p.max_froude {
            ts[i] = f32::MAX;
            continue;
        }
        let f = langbein_f(froude);
        let v_ft = v * FT_PER_M;
        let d_ft = d * FT_PER_M;
        let t = v_ft * v_ft * (f + 0.6) / (1600.0 * d_ft.powf(4.0 / 3.0));
        ts[i] = t as f32;
        let ok = d >= p.min_depth_m && ford.channel_slope[i] <= p.max_slope && t <= p.max_ts;
        navigable[i] = u32::from(ok);
    }
    NavigabilityFields { ts, navigable }
}

/// The mouth-connected navigable set: BFS upstream from navigable cells
/// whose receiver is ocean, following donor edges through navigable
/// cells. Connectivity is along the receivers tree — the flow-path
/// reading of "continuously connected downstream to the river mouth".
fn mouth_connected(
    w: u32,
    h: u32,
    receivers: &[u32],
    land: &[bool],
    navigable: &[u32],
) -> Vec<bool> {
    let n = land.len();
    let mut connected = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    for i in 0..n as u32 {
        let ii = i as usize;
        if navigable[ii] == 1 && !land[receivers[ii] as usize] {
            connected[ii] = true;
            queue.push_back(i);
        }
    }
    while let Some(c) = queue.pop_front() {
        crate::for_neighbors8(w, h, c, |nb, _| {
            let nbi = nb as usize;
            if receivers[nbi] == c && nb != c && navigable[nbi] == 1 && !connected[nbi] {
                connected[nbi] = true;
                queue.push_back(nb);
            }
        });
    }
    connected
}

/// Head-of-navigation sites: mouth-connected navigable cells with no
/// donor in that set (equivalently, no navigable donor — a navigable
/// donor of a connected cell is itself connected).
pub fn head_of_navigation(
    w: u32,
    h: u32,
    receivers: &[u32],
    land: &[bool],
    water_depth: &[f32],
    nav: &NavigabilityFields,
    ford: &FordFields,
) -> Vec<HeadOfNavSite> {
    let connected = mouth_connected(w, h, receivers, land, &nav.navigable);
    let mut sites = Vec::new();
    for c in 0..land.len() as u32 {
        let ci = c as usize;
        if !connected[ci] {
            continue;
        }
        let mut has_donor_in_set = false;
        crate::for_neighbors8(w, h, c, |nb, _| {
            if receivers[nb as usize] == c && nb != c && connected[nb as usize] {
                has_donor_in_set = true;
            }
        });
        if !has_donor_in_set {
            let depth = if water_depth[ci] > 0.0 {
                water_depth[ci]
            } else {
                ford.depth_m[ci]
            };
            sites.push(HeadOfNavSite {
                cell: c,
                x: c % w,
                y: c / w,
                depth_m: depth,
                velocity_ms: ford.velocity_ms[ci],
                ts: nav.ts[ci],
            });
        }
    }
    sites
}

/// The emitted rasters a head-of-navigation site's payload is checked
/// against (read back from disk by the caller).
pub struct HeadCheckFields<'a> {
    pub water_depth: &'a [f32],
    pub ford_depth: &'a [f32],
    pub ford_velocity: &'a [f32],
    pub ts: &'a [f32],
}

/// ADR 0011 D6 head-of-navigation gate: recompute the mouth-connected
/// navigable set from the emitted predicate and the receivers artifact —
/// by downstream walks with memoization, a different traversal than the
/// upstream BFS — and require the emitted sites to be exactly the cells
/// of that set with no donor in it, carrying payloads that match the
/// emitted rasters. A cyclic receivers artifact fails the gate rather
/// than hanging it (unevaluable is fail).
pub fn verify_heads(
    w: u32,
    receivers: &[u32],
    land: &[bool],
    navigable: &[u32],
    fields: &HeadCheckFields,
    sites: &[HeadOfNavSite],
) -> bool {
    let n = land.len();
    // 0 = unknown, 1 = connected, 2 = not connected.
    let mut state = vec![0u8; n];
    for start in 0..n as u32 {
        if state[start as usize] != 0 {
            continue;
        }
        // Walk downstream through navigable cells until a decided cell,
        // a mouth, or a break.
        let mut path = Vec::new();
        let mut c = start;
        let verdict = loop {
            let ci = c as usize;
            if state[ci] != 0 {
                break state[ci];
            }
            if navigable[ci] != 1 {
                break 2u8;
            }
            path.push(c);
            if path.len() > n {
                return false; // cyclic receivers artifact: corrupt
            }
            let r = receivers[ci];
            if !land[r as usize] {
                break 1u8; // reached a mouth
            }
            if r == c {
                break 2u8; // defensive: a land self-receiver cannot connect
            }
            c = r;
        };
        for &pc in &path {
            state[pc as usize] = verdict;
        }
    }
    // Donor-in-set counts by full receiver sweep.
    let mut donor_in_set = vec![false; n];
    for i in 0..n as u32 {
        let r = receivers[i as usize] as usize;
        if r != i as usize && state[i as usize] == 1 {
            donor_in_set[r] = true;
        }
    }
    let expected: Vec<u32> = (0..n as u32)
        .filter(|&c| state[c as usize] == 1 && !donor_in_set[c as usize])
        .collect();
    let emitted: Vec<u32> = sites.iter().map(|s| s.cell).collect();
    if emitted != expected {
        return false;
    }
    // Site payloads must match the emitted rasters.
    sites.iter().all(|s| {
        let ci = s.cell as usize;
        let depth = if fields.water_depth[ci] > 0.0 {
            fields.water_depth[ci]
        } else {
            fields.ford_depth[ci]
        };
        s.x == s.cell % w
            && s.y == s.cell / w
            && s.depth_m == depth
            && s.velocity_ms == fields.ford_velocity[ci]
            && s.ts == fields.ts[ci]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> NavParams {
        NavParams {
            max_ts: 0.002,
            max_slope: 0.0047,
            min_depth_m: 0.5,
            max_froude: 0.9,
        }
    }

    // (w, h, receivers, land, strahler, water_depth, ford)
    type Fixture = (
        u32,
        u32,
        Vec<u32>,
        Vec<bool>,
        Vec<u32>,
        Vec<f32>,
        FordFields,
    );

    // 8×3: ocean at x=0; a river along row 1 from x=7 down to the mouth
    // at x=1. Ford fields are hand-built per cell.
    fn fixture() -> Fixture {
        let (w, h) = (8u32, 3u32);
        let n = (w * h) as usize;
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        let mut receivers: Vec<u32> = (0..n as u32).collect();
        let mut land = vec![true; n];
        let mut strahler = vec![0u32; n];
        let water = vec![0.0f32; n];
        for y in 0..h {
            land[idx(0, y)] = false;
        }
        for x in 1..w {
            receivers[idx(x, 1)] = idx(x - 1, 1) as u32;
            strahler[idx(x, 1)] = 1;
        }
        let mut ford = FordFields {
            width_m: vec![0.0; n],
            depth_m: vec![0.0; n],
            velocity_ms: vec![0.0; n],
            crossability: vec![0.0; n],
            channel_slope: vec![0.0; n],
        };
        for x in 1..w {
            let i = idx(x, 1);
            // Deep, slow, flat: comfortably navigable.
            ford.depth_m[i] = 1.5;
            ford.velocity_ms[i] = 0.8;
            ford.channel_slope[i] = 0.001;
        }
        (w, h, receivers, land, strahler, water, ford)
    }

    #[test]
    fn navigable_river_has_one_head_at_the_top() {
        let (w, h, receivers, land, strahler, water, ford) = fixture();
        let nav = compute(&land, &strahler, &water, &ford, &params());
        assert!(nav.ts[(w + 3) as usize] < 0.002);
        let heads = head_of_navigation(w, h, &receivers, &land, &water, &nav, &ford);
        assert_eq!(heads.len(), 1);
        assert_eq!((heads[0].x, heads[0].y), (7, 1));
        let fields = HeadCheckFields {
            water_depth: &water,
            ford_depth: &ford.depth_m,
            ford_velocity: &ford.velocity_ms,
            ts: &nav.ts,
        };
        assert!(verify_heads(
            w,
            &receivers,
            &land,
            &nav.navigable,
            &fields,
            &heads
        ));
    }

    #[test]
    fn isolated_pocket_above_a_break_is_not_a_head() {
        let (w, h, receivers, land, strahler, water, mut ford) = fixture();
        // A steep, fast reach at x=4 breaks navigability mid-river.
        let brk = (w + 4) as usize;
        ford.depth_m[brk] = 0.3;
        ford.velocity_ms[brk] = 3.5;
        ford.channel_slope[brk] = 0.02;
        let nav = compute(&land, &strahler, &water, &ford, &params());
        assert_eq!(nav.navigable[brk], 0);
        let heads = head_of_navigation(w, h, &receivers, &land, &water, &nav, &ford);
        // The mouth-connected set ends at x=3; the navigable pocket at
        // x=5..7 is isolated and contributes no head.
        assert_eq!(heads.len(), 1);
        assert_eq!((heads[0].x, heads[0].y), (3, 1));
        let fields = HeadCheckFields {
            water_depth: &water,
            ford_depth: &ford.depth_m,
            ford_velocity: &ford.velocity_ms,
            ts: &nav.ts,
        };
        assert!(verify_heads(
            w,
            &receivers,
            &land,
            &nav.navigable,
            &fields,
            &heads
        ));
    }

    #[test]
    fn verification_rejects_a_pocket_head() {
        let (w, h, receivers, land, strahler, water, mut ford) = fixture();
        let brk = (w + 4) as usize;
        ford.channel_slope[brk] = 0.02;
        let nav = compute(&land, &strahler, &water, &ford, &params());
        let mut heads = head_of_navigation(w, h, &receivers, &land, &water, &nav, &ford);
        heads.push(HeadOfNavSite {
            cell: w + 7,
            x: 7,
            y: 1,
            depth_m: 1.5,
            velocity_ms: 0.8,
            ts: 0.001,
        });
        let fields = HeadCheckFields {
            water_depth: &water,
            ford_depth: &ford.depth_m,
            ford_velocity: &ford.velocity_ms,
            ts: &nav.ts,
        };
        assert!(!verify_heads(
            w,
            &receivers,
            &land,
            &nav.navigable,
            &fields,
            &heads
        ));
    }

    #[test]
    fn still_water_is_navigable_by_depth_alone() {
        let (w, _h, _receivers, land, strahler, mut water, ford) = fixture();
        let lake = (2 * w + 5) as usize; // off-river land cell
        water[lake] = 2.0;
        let nav = compute(&land, &strahler, &water, &ford, &params());
        assert_eq!(nav.ts[lake], 0.0);
        assert_eq!(nav.navigable[lake], 1);
        let shallow = (2 * w + 6) as usize;
        let mut water2 = vec![0.0f32; land.len()];
        water2[shallow] = 0.2;
        let nav2 = compute(&land, &strahler, &water2, &ford, &params());
        assert_eq!(nav2.navigable[shallow], 0);
    }
}
