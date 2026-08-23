//! Street-fabric growth: Courtat, Gloaguen & Douady (2011) in the form the
//! research dossier records, with the paper's flat plain replaced by via's
//! terrain patch.
//!
//! Each step places one settlement point and connects it to the network:
//!
//! - a hard rejection tube of radius λ₀ around existing streets, so new
//!   building sites do not land on top of the fabric that already exists;
//! - the site is chosen at the potential's best value with probability
//!   P_e (an organized town), otherwise anywhere admissible (an
//!   unorganized one);
//! - the shortest admissible connection is always built, and each further
//!   candidate connection is built with probability ω — ω ≈ 0 grows trees
//!   and dead ends, ω ≈ 1 grows loops and grids.
//!
//! Platting is a separate, one-shot *forcing* operator: a surveyed lattice
//! stamped over the buildable domain before organic growth continues.
//! That is the frontier plat and the modern subdivision; the literature is
//! explicit that platting is an institutional act, not an emergent one.
//!
//! The PRNG is a seeded SplitMix64 stream consumed in a fixed order, so a
//! seed reproduces a town bit for bit.

use serde::{Deserialize, Serialize};

use crate::geom::*;
use crate::graph::{Class, Graph};
use crate::patch::Patch;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatConfig {
    /// Block module (m) along and across the survey bearing.
    pub block_long_m: f64,
    pub block_cross_m: f64,
    /// Survey bearing (degrees). The lattice is terrain-blind by
    /// construction — that is what a survey-before-settlement is.
    pub bearing_deg: f64,
    /// Radius (m) of the platted area around the site.
    pub radius_m: f64,
}

impl Default for PlatConfig {
    fn default() -> Self {
        Self {
            block_long_m: 180.0,
            block_cross_m: 90.0,
            bearing_deg: 0.0,
            radius_m: 500.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GrowthConfig {
    /// Courtat P_e: share of sites placed at the potential's optimum.
    pub p_organized: f64,
    /// Courtat ω: probability of building each additional connection.
    pub omega: f64,
    /// Rejection tube radius λ₀ (m) around existing streets.
    pub reject_radius_m: f64,
    /// Share of sites placed against an inflated rejection radius
    /// (Courtat's sprawl term), and the inflation factor.
    pub f_ext: f64,
    pub k_ext: f64,
    /// Candidate sites evaluated per step.
    pub candidates: u32,
    /// How far a new site may sit from the fabric it grows off (m).
    pub reach_m: f64,
    /// Extra connections considered per site.
    pub extra_links: u32,
    /// Longest water crossing the era can bridge (m).
    pub max_bridge_m: f64,
    /// Steepest ground streets and buildings may occupy.
    pub max_build_slope: f64,
    /// Nodes closer than this are the same node (m).
    pub snap_m: f64,
    /// Share of the built envelope that ends up as lots rather than
    /// street, yard and public ground — sets how far the town must spread
    /// to house its people.
    pub packing: f64,
    /// The frontier may run this far beyond the radius the current
    /// population needs; beyond it, land is still farmland.
    pub frontier_slack: f64,
    /// Densification links attempted per growth round (Strano et al. 2012
    /// name densification and exploration as the two elementary processes
    /// of road-network growth; this spike had only exploration). A
    /// densification link joins two junctions that are close in space but
    /// far apart through the network — the missing connection people
    /// complain about until it is built.
    pub densify_per_round: u32,
    /// How much of a detour must exist before a link is worth building:
    /// network distance over straight-line distance.
    pub densify_detour: f64,
    /// One-shot surveyed lattice, if the era plats.
    pub plat: Option<PlatConfig>,
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            p_organized: 0.4,
            omega: 0.2,
            reject_radius_m: 28.0,
            f_ext: 0.0,
            k_ext: 3.0,
            candidates: 24,
            reach_m: 130.0,
            extra_links: 3,
            max_bridge_m: 40.0,
            max_build_slope: 0.22,
            snap_m: 7.0,
            packing: 0.55,
            frontier_slack: 1.2,
            densify_per_round: 3,
            densify_detour: 2.6,
            plat: None,
        }
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(via_artifact::seed::splitmix64(seed))
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        via_artifact::seed::splitmix64(self.0)
    }
    /// Uniform in [0, 1).
    #[inline]
    pub fn f01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    #[inline]
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f01()
    }
}

/// Seed the graph with the intercity corridors that met here: the road
/// came first, the town grew on it.
pub fn seed_corridors(g: &mut Graph, patch: &Patch, bearings_deg: &[f64], cfg: &GrowthConfig) {
    let r = patch.extent_m * 0.98;
    for &b in bearings_deg {
        let a = b.to_radians();
        let d = [a.cos(), a.sin()];
        // Walk out from the centre until the ground stops cooperating, so a
        // corridor does not simply ignore a river or a cliff.
        let mut ends = [[0.0, 0.0]; 2];
        for (k, sgn) in [1.0f64, -1.0].iter().enumerate() {
            let mut t = 0.0;
            let mut last = [0.0, 0.0];
            while t < r {
                t += 10.0;
                let p = mul(d, sgn * t);
                if !patch.in_domain(p) {
                    break;
                }
                if patch.is_water(p) {
                    // Bridge it if the era can.
                    let mut t2 = t;
                    while t2 < t + cfg.max_bridge_m && patch.is_water(mul(d, sgn * t2)) {
                        t2 += 10.0;
                    }
                    if t2 >= t + cfg.max_bridge_m {
                        break;
                    }
                    t = t2;
                    continue;
                }
                if patch.slope(p) > cfg.max_build_slope * 1.6 {
                    break;
                }
                last = p;
            }
            ends[k] = last;
        }
        g.insert_segment(ends[0], ends[1], Class::Corridor, cfg.snap_m);
    }
}

/// Stamp a surveyed lattice over the buildable domain (forcing, one shot).
pub fn stamp_plat(g: &mut Graph, patch: &Patch, cfg: &GrowthConfig, plat: &PlatConfig) {
    let a = plat.bearing_deg.to_radians();
    let u = [a.cos(), a.sin()];
    let v = perp(u);
    let r = plat.radius_m;
    // Lines along u, spaced by the cross module; then the transverse set.
    for (dir, other, spacing) in [(u, v, plat.block_cross_m), (v, u, plat.block_long_m)] {
        let n = (r / spacing).ceil() as i64;
        for k in -n..=n {
            let off = mul(other, k as f64 * spacing);
            // Walk the line, inserting only the runs that lie on ground the
            // era can build on: the survey is terrain-blind, the building
            // of it is not.
            let mut run_start: Option<P2> = None;
            let mut t = -r;
            while t <= r {
                let p = add(off, mul(dir, t));
                let ok = patch.buildable(p, cfg.max_build_slope) && len(p) <= r;
                match (ok, run_start) {
                    (true, None) => run_start = Some(p),
                    (false, Some(s)) => {
                        let e = add(off, mul(dir, t - 8.0));
                        if dist(s, e) > 30.0 {
                            g.insert_segment(s, e, Class::Street, cfg.snap_m);
                        }
                        run_start = None;
                    }
                    _ => {}
                }
                t += 8.0;
            }
            if let Some(s) = run_start {
                let e = add(off, mul(dir, r));
                if dist(s, e) > 30.0 {
                    g.insert_segment(s, e, Class::Street, cfg.snap_m);
                }
            }
        }
    }
}

/// One growth step: place a site, connect it. Returns the site point if
/// one was built. `built_radius` is how far out the town has any business
/// being, given how many households it has already housed.
fn grow_step(
    g: &mut Graph,
    patch: &Patch,
    cfg: &GrowthConfig,
    rng: &mut Rng,
    built_radius: f64,
) -> Option<P2> {
    if g.nodes.is_empty() {
        return None;
    }
    let sprawl = rng.f01() < cfg.f_ext;
    let lambda = cfg.reject_radius_m * if sprawl { cfg.k_ext } else { 1.0 };

    // Candidates grow off the existing fabric: pick an anchor node, then a
    // point in the annulus [λ, reach] around it.
    let mut best: Option<(f64, P2)> = None;
    let mut first: Option<P2> = None;
    let mut rej = [0u32; 5];
    for _ in 0..cfg.candidates {
        let anchor = g.nodes[(rng.next_u64() % g.nodes.len() as u64) as usize];
        let ang = rng.range(0.0, std::f64::consts::TAU);
        let rad = rng.range(lambda, cfg.reach_m.max(lambda * 1.5));
        let p = add(anchor, [rad * ang.cos(), rad * ang.sin()]);
        if !patch.buildable(p, cfg.max_build_slope) {
            rej[0] += 1;
            continue;
        }
        // Beyond the built frontier the land is worth more as farmland
        // than as lots: this is what gives a town an edge instead of a
        // uniform smear (the von Thünen conversion threshold, reduced to
        // its simplest form).
        // A soft frontier: past the built edge, conversion from farmland
        // gets steadily less likely instead of stopping dead. A hard circle
        // would draw a suspiciously round town.
        let over = len(p) - built_radius;
        if over > 0.0 {
            let accept = (-over / (cfg.reject_radius_m * 3.0)).exp();
            if rng.f01() > accept {
                rej[1] += 1;
                continue;
            }
        }
        let Some((_, _, d_net)) = g.nearest_on_edges(p) else {
            rej[2] += 1;
            continue;
        };
        if d_net < lambda {
            rej[3] += 1;
            continue; // inside the rejection tube
        }
        if d_net > cfg.reach_m {
            rej[4] += 1;
            continue;
        }
        // Courtat's potential, terrain-modulated: close to the network is
        // good (beyond the tube), gentle ground is good, and the town
        // centre still pulls.
        let score = 1.0 / (1.0 + d_net / lambda)
            + 0.6 * (1.0 - (patch.slope(p) / cfg.max_build_slope).min(1.0))
            + 0.4 / (1.0 + len(p) / (patch.extent_m * 0.5));
        if first.is_none() {
            first = Some(p);
        }
        if best.map(|(s, _)| score > s).unwrap_or(true) {
            best = Some((score, p));
        }
    }
    if std::env::var("TOWNFABRIC_DEBUG").is_ok() && best.is_none() {
        eprintln!(
            "no candidate: unbuildable {} beyond-frontier {} no-net {} in-tube {} too-far {} (R {:.0})",
            rej[0], rej[1], rej[2], rej[3], rej[4], built_radius
        );
    }
    let organized = rng.f01() < cfg.p_organized;
    let site = if organized {
        best.map(|(_, p)| p)
    } else {
        first
    }?;

    // Always build the shortest connection: to the nearest point on the
    // network, splitting that edge if it lands mid-street.
    let (_, foot, _) = g.nearest_on_edges(site)?;
    if patch.crossing_len_m(site, foot) > cfg.max_bridge_m {
        return None;
    }
    g.insert_segment(foot, site, Class::Lane, cfg.snap_m);

    // Then each further candidate connection with probability ω. These are
    // the links that close loops and turn a tree into a mesh.
    let mut cands: Vec<(f64, usize)> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, &q)| (dist(site, q), i))
        .filter(|&(d, _)| d > 1.0 && d < cfg.reach_m * 1.6)
        .collect();
    cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
    let mut built = 0;
    for (_, i) in cands.into_iter().take(cfg.extra_links as usize * 3) {
        if built >= cfg.extra_links {
            break;
        }
        if rng.f01() >= cfg.omega {
            continue;
        }
        let q = g.nodes[i];
        if g.crosses_any(site, q) {
            continue;
        }
        if patch.crossing_len_m(site, q) > cfg.max_bridge_m {
            continue;
        }
        // Refuse to run a street up a wall: sample the segment.
        let steps = (dist(site, q) / 15.0).ceil().max(1.0) as usize;
        let ok = (0..=steps).all(|k| {
            let t = k as f64 / steps as f64;
            let x = [
                site[0] + (q[0] - site[0]) * t,
                site[1] + (q[1] - site[1]) * t,
            ];
            patch.buildable(x, cfg.max_build_slope) || patch.is_water(x)
        });
        if !ok {
            continue;
        }
        g.insert_segment(site, q, Class::Lane, cfg.snap_m);
        built += 1;
    }
    Some(site)
}

/// Radius a town needs to house `lots` households at this lot size.
pub fn radius_for(lots: u32, lot_area_m2: f64, cfg: &GrowthConfig) -> f64 {
    let need = (lots.max(1) as f64 * lot_area_m2) / cfg.packing.max(0.05);
    (need / std::f64::consts::PI).sqrt().max(110.0) * cfg.frontier_slack
}

/// Network distance between two nodes over the existing graph, capped so
/// a hopeless search stops early.
fn network_dist(g: &Graph, from: usize, to: usize, limit: f64) -> f64 {
    let n = g.nodes.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut heap = std::collections::BinaryHeap::new();
    dist[from] = 0.0;
    heap.push(std::cmp::Reverse((0u64, from)));
    let key = |x: f64| -> u64 { (x * 64.0) as u64 };
    while let Some(std::cmp::Reverse((k, v))) = heap.pop() {
        if k > key(dist[v]) {
            continue;
        }
        if v == to {
            return dist[v];
        }
        if dist[v] > limit {
            return f64::INFINITY;
        }
        for &e in &g.incident[v] {
            let w = if g.edges[e].a == v {
                g.edges[e].b
            } else {
                g.edges[e].a
            };
            let nd = dist[v] + dist_between(g, v, w);
            if nd < dist[w] {
                dist[w] = nd;
                heap.push(std::cmp::Reverse((key(nd), w)));
            }
        }
    }
    dist[to]
}

#[inline]
fn dist_between(g: &Graph, a: usize, b: usize) -> f64 {
    dist(g.nodes[a], g.nodes[b])
}

/// Densification: find the worst detour between two junctions that are
/// near each other in space, and build the link that fixes it.
fn densify(g: &mut Graph, patch: &Patch, cfg: &GrowthConfig, rng: &mut Rng) -> bool {
    if g.nodes.len() < 4 {
        return false;
    }
    let mut best: Option<(f64, usize, usize)> = None;
    for _ in 0..40 {
        let a = (rng.next_u64() % g.nodes.len() as u64) as usize;
        let b = (rng.next_u64() % g.nodes.len() as u64) as usize;
        if a == b {
            continue;
        }
        let (pa, pb) = (g.nodes[a], g.nodes[b]);
        let straight = dist(pa, pb);
        if straight < cfg.reach_m * 0.8 || straight > cfg.reach_m * 3.0 {
            continue;
        }
        // A densification link may cross existing streets: that is what
        // makes it a junction rather than a bypass. It may not cross
        // water it cannot bridge, or ground it cannot climb.
        if patch.crossing_len_m(pa, pb) > cfg.max_bridge_m {
            continue;
        }
        let steps = (straight / 15.0).ceil().max(1.0) as usize;
        let ok = (0..=steps).all(|k| {
            let t = k as f64 / steps as f64;
            let x = [pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t];
            patch.buildable(x, cfg.max_build_slope) || patch.is_water(x)
        });
        if !ok {
            continue;
        }
        let nd = network_dist(g, a, b, straight * cfg.densify_detour * 1.5);
        let detour = nd / straight.max(1.0e-6);
        if detour >= cfg.densify_detour && best.map(|(d, _, _)| detour > d).unwrap_or(true) {
            best = Some((detour, a, b));
        }
    }
    if let Some((_, a, b)) = best {
        let (pa, pb) = (g.nodes[a], g.nodes[b]);
        g.insert_segment(pa, pb, Class::Street, cfg.snap_m);
        true
    } else {
        false
    }
}

/// One round of street growth inside `radius`. A growth site is a place
/// where the fabric extends — not a household: the households arrive when
/// the blocks these streets enclose are subdivided, which is why the
/// caller alternates growth with subdivision rather than counting sites.
pub fn grow_round(
    g: &mut Graph,
    patch: &Patch,
    cfg: &GrowthConfig,
    radius: f64,
    attempts: u32,
    rng: &mut Rng,
) -> u32 {
    let mut placed = 0;
    for _ in 0..attempts {
        if grow_step(g, patch, cfg, rng, radius.min(patch.extent_m)).is_some() {
            placed += 1;
        }
    }
    // Exploration has run; now densification.
    for _ in 0..cfg.densify_per_round {
        densify(g, patch, cfg, rng);
    }
    placed
}
