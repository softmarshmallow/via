//! The multimodal travel-cost surface — where era actually enters.
//!
//! One functional form for every era; only its parameters change. Land
//! speed decays exponentially with the gradient of the step being taken
//! (Tobler 1993's hiking function is this form with k ≈ 3.5) and the step
//! is refused above a hard grade cap — wagon roads ~5–8%, foot far
//! steeper (docs/research/humanity/0010). Because the cap is applied per
//! *step*, a route may still climb by traversing along the contour, which
//! is what real roads do; a per-cell cap would wall off whole ranges that
//! are in fact crossed by switchbacks.
//!
//! Water carries its own speed, and changing mode costs a transshipment
//! delay, so ports must earn their advantage rather than be given one.
//!
//! Costs are HOURS. That matters: the era's speed lives in the surface, so
//! a fixed time-decay in the interaction model reproduces Marchetti's
//! constant travel-time budget without a second era knob.
//!
//! Known simplification: the slope penalty is symmetric, so descending
//! costs what ascending costs (Tobler's function is not symmetric).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};

use crate::fields::Land;
use crate::util::{f64_key, for_neighbors8};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TravelConfig {
    /// Speed on flat ground (km/h) for the era's dominant land mode.
    pub land_speed_kmh: f64,
    /// Exponential slope penalty: v = v_flat·exp(−k·|S|).
    pub slope_penalty_k: f64,
    /// Gradient (rise/run) above which a single step cannot be taken.
    pub grade_cap: f64,
    /// Speed on navigable water (km/h): rivers, lakes, sea.
    pub water_speed_kmh: f64,
    /// Hours lost changing between land and water.
    pub transship_hours: f64,
    /// Hours lost crossing a navigable channel by ford, ferry or bridge.
    pub river_crossing_hours: f64,
}

impl Default for TravelConfig {
    fn default() -> Self {
        Self {
            land_speed_kmh: 4.5,
            slope_penalty_k: 3.5,
            grade_cap: 0.60,
            water_speed_kmh: 5.0,
            transship_hours: 4.0,
            river_crossing_hours: 1.0,
        }
    }
}

pub struct CostSurface {
    pub w: u32,
    pub h: u32,
    dx: f64,
    surface_m: Vec<f64>,
    /// Ground a land mode may stand on at all (dry land).
    dry: Vec<bool>,
    /// Open water: ocean or lake.
    water: Vec<bool>,
    /// Navigable for the water mode (open water or a big channel).
    navigable: Vec<bool>,
    /// A channel running across dry land — an obstacle to land travel.
    channel: Vec<bool>,
    cfg: TravelConfig,
}

impl CostSurface {
    pub fn build(land: &Land, cfg: &TravelConfig) -> Self {
        let n = land.w as usize * land.h as usize;
        let dry: Vec<bool> = (0..n).map(|i| land.land[i] && !land.lake[i]).collect();
        let water: Vec<bool> = (0..n).map(|i| !land.land[i] || land.lake[i]).collect();
        let navigable: Vec<bool> = (0..n).map(|i| land.navigable[i]).collect();
        let channel: Vec<bool> = (0..n).map(|i| dry[i] && land.navigable[i]).collect();
        Self {
            w: land.w,
            h: land.h,
            dx: land.dx,
            surface_m: land.surface_m.clone(),
            dry,
            water,
            navigable,
            channel,
            cfg: cfg.clone(),
        }
    }

    /// A cell the network can occupy at all.
    #[inline]
    pub fn passable(&self, i: usize) -> bool {
        self.dry[i] || (self.water[i] && self.navigable[i] && self.cfg.water_speed_kmh > 0.0)
    }

    /// Cost of the step i → j in hours: the mode that serves it, its
    /// gradient, and any penalty for changing mode or crossing water.
    /// Symmetric by construction (the slope term uses |Δh|).
    #[inline]
    fn step_hours(&self, i: usize, j: usize, len_fac: f64) -> f64 {
        let km = len_fac * self.dx / 1000.0;
        let boats = self.cfg.water_speed_kmh > 0.0;
        // Cells a boat can float on: open water, or a channel big enough to
        // carry one. Boarding happens wherever such a cell meets dry land —
        // any shore is a landing, not only a river mouth.
        let afloat_i = boats && (self.water[i] || self.channel[i]);
        let afloat_j = boats && (self.water[j] || self.channel[j]);

        if afloat_i && afloat_j {
            return km / self.cfg.water_speed_kmh;
        }
        if afloat_i != afloat_j {
            let (dry_cell, _) = if afloat_i { (j, i) } else { (i, j) };
            if !self.dry[dry_cell] || !self.land_step_allowed(i, j) {
                return f64::INFINITY;
            }
            let half_land = 0.5 * km / self.land_speed_on(i, j).max(1.0e-6);
            let half_water = 0.5 * km / self.cfg.water_speed_kmh;
            return half_land + half_water + self.cfg.transship_hours;
        }
        // Land step.
        if !self.dry[i] || !self.dry[j] {
            return f64::INFINITY;
        }
        if !self.land_step_allowed(i, j) {
            return f64::INFINITY;
        }
        let v = self.land_speed_on(i, j);
        if v <= 1.0e-6 {
            return f64::INFINITY;
        }
        let mut t = km / v;
        // A route that steps off a channel bank onto ordinary ground has
        // crossed water somewhere: ford, ferry, or bridge.
        if self.channel[i] != self.channel[j] {
            t += self.cfg.river_crossing_hours;
        }
        t
    }

    #[inline]
    fn gradient(&self, i: usize, j: usize, len_fac: f64) -> f64 {
        let dz = (self.surface_m[i] - self.surface_m[j]).abs();
        dz / (len_fac * self.dx)
    }

    #[inline]
    fn land_step_allowed(&self, i: usize, j: usize) -> bool {
        let len_fac = if (i % self.w as usize != j % self.w as usize)
            && (i / self.w as usize != j / self.w as usize)
        {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        self.gradient(i, j, len_fac) <= self.cfg.grade_cap
    }

    #[inline]
    fn land_speed_on(&self, i: usize, j: usize) -> f64 {
        let len_fac = if (i % self.w as usize != j % self.w as usize)
            && (i / self.w as usize != j / self.w as usize)
        {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        let s = self.gradient(i, j, len_fac);
        self.cfg.land_speed_kmh * (-self.cfg.slope_penalty_k * s).exp()
    }

    /// The step cost, for callers walking the graph themselves.
    #[inline]
    pub fn link_hours(&self, i: usize, j: usize, len_fac: f64) -> f64 {
        self.step_hours(i, j, len_fac)
    }

    /// Travel time in hours from `source` to every reachable cell.
    /// Dijkstra; heap ties break on cell index, so the field is
    /// reproducible independent of scheduling.
    pub fn time_from(&self, source: u32) -> Vec<f64> {
        let n = self.w as usize * self.h as usize;
        let mut dist = vec![f64::INFINITY; n];
        if !self.passable(source as usize) {
            return dist;
        }
        let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
        dist[source as usize] = 0.0;
        heap.push(Reverse((f64_key(0.0), source)));
        while let Some(Reverse((k, i))) = heap.pop() {
            if k > f64_key(dist[i as usize]) {
                continue;
            }
            let di = dist[i as usize];
            for_neighbors8(self.w, self.h, i, |nb, fac| {
                let step = self.step_hours(i as usize, nb as usize, fac);
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
        dist
    }

    /// Least-cost path between two cells over a *discounted* copy of the
    /// surface: cells already carrying a route cost `reuse_mult` of their
    /// normal time, which is what makes routes bundle into corridors
    /// instead of running parallel (Stahlberg et al. 2023).
    pub fn least_cost_path(
        &self,
        from: u32,
        to: u32,
        built: &[bool],
        reuse_mult: f64,
    ) -> Option<Vec<u32>> {
        let n = self.w as usize * self.h as usize;
        if !self.passable(from as usize) || !self.passable(to as usize) {
            return None;
        }
        let mut dist = vec![f64::INFINITY; n];
        let mut prev = vec![u32::MAX; n];
        let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
        dist[from as usize] = 0.0;
        heap.push(Reverse((f64_key(0.0), from)));
        while let Some(Reverse((k, i))) = heap.pop() {
            if k > f64_key(dist[i as usize]) {
                continue;
            }
            if i == to {
                break;
            }
            let di = dist[i as usize];
            for_neighbors8(self.w, self.h, i, |nb, fac| {
                let mut step = self.step_hours(i as usize, nb as usize, fac);
                if !step.is_finite() {
                    return;
                }
                if built[nb as usize] {
                    step *= reuse_mult;
                }
                let nd = di + step;
                if nd < dist[nb as usize] {
                    dist[nb as usize] = nd;
                    prev[nb as usize] = i;
                    heap.push(Reverse((f64_key(nd), nb)));
                }
            });
        }
        if !dist[to as usize].is_finite() {
            return None;
        }
        let mut path = vec![to];
        let mut c = to;
        while c != from {
            c = prev[c as usize];
            if c == u32::MAX {
                return None;
            }
            path.push(c);
        }
        path.reverse();
        Some(path)
    }
}
