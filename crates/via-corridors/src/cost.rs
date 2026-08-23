//! The era-0 movement model (ADR 0012 D2/D3): a node-split multimodal
//! graph over the terrain grid. Land nodes move 16-neighbour (Queen +
//! Knight, Knight subdivided) on Tobler time under the Llobera–Sluckin
//! switchback envelope; water nodes move 8-neighbour at canoe speed ±
//! current (coastal cells at a declared flat speed); land↔water
//! switches cost a flat declared transshipment penalty. Rivers are
//! barriers pierced where the ADR 0011 crossability spectrum passes
//! the Cox/AIDR stability caps; diagonal and Knight moves cannot
//! corner-cut a channel (the ADR 0012 straddle rule — realized here as
//! enterability of every geometrically traversed intermediate cell).
//!
//! Node ids: `0..n` are land nodes, `n..2n` water nodes (n = w·h).
//! All times are hours; all energies are J/kg (land legs only).

use crate::CorridorsConfig;

/// Tobler's own critical gradient under the L&S Eq. 20 criterion:
/// T ∝ e^(±3.5(s+0.05)) gives T − sT′ = T(1 ∓ 3.5s), root s = ±1/3.5.
/// Derived from the cited coefficient, not a free choice (ADR 0012 D2).
pub const TOBLER_TIME_CRIT: f64 = 1.0 / 3.5;

/// Herzog IA36 §5.1.4.3 sixth-degree refit of Minetti 2002, J/kg/m
/// (the primary's "kilo-joule" is a label slip — 0013 correction note).
fn herzog_j_per_kg_m(s: f64) -> f64 {
    ((((((1337.8 * s) + 278.19) * s - 517.39) * s - 78.199) * s + 93.419) * s + 19.825) * s + 1.64
}

/// Tobler walking time in hours per horizontal km at gradient `s`,
/// switchback-enveloped beyond ±1/3.5 (tangent at the seams).
pub fn tobler_hours_per_km(s: f64) -> f64 {
    let raw = |s: f64| (3.5 * (s + 0.05).abs()).exp() / 6.0;
    if s > TOBLER_TIME_CRIT {
        raw(TOBLER_TIME_CRIT) * (s / TOBLER_TIME_CRIT)
    } else if s < -TOBLER_TIME_CRIT {
        raw(-TOBLER_TIME_CRIT) * (-s / TOBLER_TIME_CRIT)
    } else {
        raw(s)
    }
}

/// Energy in J/kg per horizontal metre at gradient `s`: the clamped
/// Herzog polynomial inside the frozen L&S critical gradients, and
/// min(clamped polynomial, ray through the critical point) beyond
/// them (ADR 0012 D2 min form — the ray constants come from a
/// different polynomial, so the min restores cheaper-strategy
/// semantics).
pub fn energy_j_per_kg_m(s: f64, clamp: f64, crit_up: f64, crit_down: f64) -> f64 {
    let poly = herzog_j_per_kg_m(s.clamp(-clamp, clamp));
    if s > crit_up {
        poly.min(herzog_j_per_kg_m(crit_up) * (s / crit_up))
    } else if s < crit_down {
        poly.min(herzog_j_per_kg_m(crit_down) * (s / crit_down))
    } else {
        poly
    }
}

/// One directed move in the multimodal graph.
#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub to: u32,
    pub hours: f64,
}

/// Cell classification and per-cell admissibility, precomputed once.
pub struct MoveModel<'a> {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub heights_m: &'a [f64],
    pub receivers: &'a [u32],
    pub strahler: &'a [u32],
    pub water_depth: &'a [f64],
    pub ford_velocity: &'a [f32],
    /// A land node exists here (dry ground, or a river/standing-water
    /// cell passing the wading caps).
    pub land_node: Vec<bool>,
    /// A water node exists here (navigable predicate, or admissible
    /// coastal-water ribbon).
    pub water_node: Vec<bool>,
    /// Entering (or crossing, per the straddle rule) this cell on foot
    /// charges the wading delay.
    pub ford_entry: Vec<bool>,
    /// Ocean cell on the coastal ribbon admitted for the coastal mode.
    pub coastal: Vec<bool>,
    /// Ocean (self-receiver) mask.
    pub ocean: Vec<bool>,
    cfg: ModelParams,
}

#[derive(Clone, Copy)]
struct ModelParams {
    offpath_factor: f64,
    canoe_kmh: f64,
    coastal_kmh: f64,
    transship_hours: f64,
    ford_delay_hours: f64,
    herzog_clamp: f64,
    ls_crit_up: f64,
    ls_crit_down: f64,
}

/// Inputs the model reads (all disk artifacts; ADR 0012 D3).
pub struct ModelInputs<'a> {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub heights_m: &'a [f64],
    pub receivers: &'a [u32],
    pub strahler: &'a [u32],
    pub water_depth: &'a [f64],
    pub navigable: &'a [u32],
    pub crossability: &'a [f32],
    pub ford_depth: &'a [f32],
    pub ford_velocity: &'a [f32],
    pub harbour_fetch: &'a [f32],
}

impl<'a> MoveModel<'a> {
    pub fn build(inp: &ModelInputs<'a>, cfg: &CorridorsConfig) -> MoveModel<'a> {
        let n = (inp.w * inp.h) as usize;
        let mut land_node = vec![false; n];
        let mut water_node = vec![false; n];
        let mut ford_entry = vec![false; n];
        let mut coastal = vec![false; n];
        let mut ocean = vec![false; n];
        for (i, o) in ocean.iter_mut().enumerate() {
            *o = inp.receivers[i] == i as u32;
        }
        for i in 0..n {
            if ocean[i] {
                // Coastal ribbon: ocean with a land D8 neighbour, under
                // the optional exposure cap (declared forcing).
                if cfg.coastal_mode {
                    let mut shore = false;
                    for_neighbors8(inp.w, inp.h, i as u32, |j, _| {
                        if !ocean[j as usize] {
                            shore = true;
                        }
                    });
                    let exposed = match cfg.coastal_exposure_cap_m {
                        Some(cap) => f64::from(inp.harbour_fetch[i]) > cap,
                        None => false,
                    };
                    if shore && !exposed {
                        coastal[i] = true;
                        water_node[i] = true;
                    }
                }
                continue;
            }
            // Land-node admissibility (ADR 0012 D3).
            if inp.water_depth[i] > 0.0 {
                // Standing water: wadeable iff the depth cap holds
                // (crossability equals depth there; velocity is zero).
                if inp.water_depth[i] <= cfg.ford_max_depth_m {
                    land_node[i] = true;
                    ford_entry[i] = true;
                }
            } else if inp.strahler[i] > 0 {
                // River channel: the Cox/AIDR caps, independently.
                let dv = f64::from(inp.crossability[i]);
                let d = f64::from(inp.ford_depth[i]);
                let v = f64::from(inp.ford_velocity[i]);
                if dv <= cfg.ford_max_dv_m2s
                    && d <= cfg.ford_max_depth_m
                    && v <= cfg.ford_max_velocity_ms
                {
                    land_node[i] = true;
                    ford_entry[i] = true;
                }
            } else {
                land_node[i] = true;
            }
            if inp.navigable[i] == 1 {
                water_node[i] = true;
            }
        }
        MoveModel {
            w: inp.w,
            h: inp.h,
            dx: inp.dx,
            heights_m: inp.heights_m,
            receivers: inp.receivers,
            strahler: inp.strahler,
            water_depth: inp.water_depth,
            ford_velocity: inp.ford_velocity,
            land_node,
            water_node,
            ford_entry,
            coastal,
            ocean,
            cfg: ModelParams {
                offpath_factor: cfg.offpath_factor,
                canoe_kmh: cfg.canoe_speed_kmh,
                coastal_kmh: cfg.coastal_speed_kmh,
                transship_hours: cfg.transship_hours,
                ford_delay_hours: cfg.ford_delay_hours,
                herzog_clamp: cfg.herzog_clamp,
                ls_crit_up: cfg.ls_crit_up,
                ls_crit_down: cfg.ls_crit_down,
            },
        }
    }

    #[inline]
    pub fn n_cells(&self) -> usize {
        (self.w * self.h) as usize
    }

    #[inline]
    pub fn n_nodes(&self) -> usize {
        2 * self.n_cells()
    }

    #[inline]
    pub fn cell_of(&self, node: u32) -> u32 {
        let n = self.w * self.h;
        if node >= n {
            node - n
        } else {
            node
        }
    }

    #[inline]
    pub fn is_water_node(&self, node: u32) -> bool {
        node >= self.w * self.h
    }

    /// Walking time in hours over horizontal distance `d_m` between
    /// ground heights `ha` → `hb`.
    fn walk_hours(&self, ha: f64, hb: f64, d_m: f64) -> f64 {
        let s = (hb - ha) / d_m;
        (d_m / 1000.0) * tobler_hours_per_km(s) / self.cfg.offpath_factor
    }

    /// Walking energy in J/kg over the same segment (spectrum only —
    /// never a solve currency).
    fn walk_energy(&self, ha: f64, hb: f64, d_m: f64) -> f64 {
        let s = (hb - ha) / d_m;
        d_m * energy_j_per_kg_m(
            s,
            self.cfg.herzog_clamp,
            self.cfg.ls_crit_up,
            self.cfg.ls_crit_down,
        )
    }

    /// Directed land move i→j over one Queen step (orthogonal or
    /// diagonal). Returns hours, or None if inadmissible.
    fn land_step(&self, i: u32, j: u32, diagonal: bool) -> Option<f64> {
        if !self.land_node[j as usize] {
            return None;
        }
        let mut extra = 0.0;
        if diagonal {
            match self.straddle(i, j)? {
                Straddle::Free => {}
                Straddle::Crossing => extra += self.cfg.ford_delay_hours,
            }
        }
        if self.ford_entry[j as usize] {
            extra += self.cfg.ford_delay_hours;
        }
        let d = if diagonal {
            self.dx * std::f64::consts::SQRT_2
        } else {
            self.dx
        };
        let (ia, ja) = (i as usize, j as usize);
        Some(self.walk_hours(self.heights_m[ia], self.heights_m[ja], d) + extra)
    }

    /// The straddle rule for a diagonal land move (ADR 0012 D3): when
    /// the corner pair carries a watercourse, the move is a crossing —
    /// admissible only if a corner passes the wading caps (it then
    /// carries a land node), and the wading delay is charged; when
    /// neither corner is enterable the move is inadmissible; otherwise
    /// passage is free.
    ///
    /// A corner pair carries a watercourse in either of two ways: the
    /// corners are consecutive on the drainage chain below a channel
    /// cell — the thread itself runs diagonally through the shared
    /// corner, which is how a river reaches the sea at a mouth — or
    /// both corners are water of any kind (channel, standing, ocean),
    /// so the segment cannot pass between them on dry ground.
    fn straddle(&self, i: u32, j: u32) -> Option<Straddle> {
        let (x1, y1) = (i % self.w, i / self.w);
        let (x2, y2) = (j % self.w, j / self.w);
        let c1 = (y1 * self.w + x2) as usize;
        let c2 = (y2 * self.w + x1) as usize;
        let chain_link = (self.receivers[c1] as usize == c2 && self.strahler[c1] > 0)
            || (self.receivers[c2] as usize == c1 && self.strahler[c2] > 0);
        // Wet = wading required (channel or standing water) or no land
        // node at all (ocean, or water failing the caps).
        let wet = |c: usize| self.ford_entry[c] || !self.land_node[c];
        if chain_link || (wet(c1) && wet(c2)) {
            if self.land_node[c1] || self.land_node[c2] {
                Some(Straddle::Crossing)
            } else {
                None
            }
        } else if !self.land_node[c1] && !self.land_node[c2] {
            None
        } else {
            Some(Straddle::Free)
        }
    }

    /// Directed Knight move i→j (offset (±1,±2)/(±2,±1)), subdivided at
    /// the midpoint (Herzog): both geometrically traversed intermediate
    /// cells must be enterable; river intermediates charge the wading
    /// delay each; slopes use the boundary midpoint height.
    fn knight_step(&self, i: u32, j: u32) -> Option<f64> {
        if !self.land_node[j as usize] {
            return None;
        }
        let (x1, y1) = ((i % self.w) as i64, (i / self.w) as i64);
        let (x2, y2) = ((j % self.w) as i64, (j / self.w) as i64);
        let (dxc, dyc) = (x2 - x1, y2 - y1);
        let (m1, m2) = if dxc.abs() == 2 {
            let mx = x1 + dxc / 2;
            ((y1 * self.w as i64 + mx), ((y1 + dyc) * self.w as i64 + mx))
        } else {
            let my = y1 + dyc / 2;
            ((my * self.w as i64 + x1), (my * self.w as i64 + x1 + dxc))
        };
        let (m1, m2) = (m1 as usize, m2 as usize);
        if !self.land_node[m1] || !self.land_node[m2] {
            return None;
        }
        let mut extra = 0.0;
        if self.ford_entry[m1] {
            extra += self.cfg.ford_delay_hours;
        }
        if self.ford_entry[m2] {
            extra += self.cfg.ford_delay_hours;
        }
        if self.ford_entry[j as usize] {
            extra += self.cfg.ford_delay_hours;
        }
        let half = self.dx * (5.0f64).sqrt() / 2.0;
        let hmid = (self.heights_m[m1] + self.heights_m[m2]) / 2.0;
        let (ia, ja) = (i as usize, j as usize);
        Some(
            self.walk_hours(self.heights_m[ia], hmid, half)
                + self.walk_hours(hmid, self.heights_m[ja], half)
                + extra,
        )
    }

    /// Directed water move i→j (Queen step): canoe ± current on river
    /// chains, base speed on still/transition water, coastal speed on
    /// the ribbon; upstream at net speed ≤ 0 is inadmissible; a
    /// diagonal blocked when both corners are land cells.
    fn water_step(&self, i: u32, j: u32, diagonal: bool) -> Option<f64> {
        if !self.water_node[j as usize] {
            return None;
        }
        let (ia, ja) = (i as usize, j as usize);
        // Spit rule: a diagonal between two water cells whose corners
        // are both plain dry land is blocked — unless the two cells
        // are consecutive on the receiver chain, where the channel
        // demonstrably flows through the corner (sub-cell geometry;
        // without this exemption every diagonally-stepping channel
        // would be unnavigable).
        if diagonal && self.receivers[ia] != j && self.receivers[ja] != i {
            let (x1, y1) = (i % self.w, i / self.w);
            let (x2, y2) = (j % self.w, j / self.w);
            let c1 = (y1 * self.w + x2) as usize;
            let c2 = (y2 * self.w + x1) as usize;
            let dry =
                |c: usize| !self.ocean[c] && self.water_depth[c] == 0.0 && self.strahler[c] == 0;
            if dry(c1) && dry(c2) {
                return None;
            }
        }
        let kmh = if self.ocean[ia] && self.ocean[ja] {
            self.cfg.coastal_kmh
        } else if self.ocean[ia] || self.ocean[ja] {
            self.cfg.canoe_kmh
        } else {
            let v_ms =
                (f64::from(self.ford_velocity[ia]) + f64::from(self.ford_velocity[ja])) / 2.0;
            let current_kmh = v_ms * 3.6;
            if self.receivers[ia] == j {
                self.cfg.canoe_kmh + current_kmh
            } else if self.receivers[ja] == i {
                self.cfg.canoe_kmh - current_kmh
            } else {
                self.cfg.canoe_kmh
            }
        };
        if kmh <= 0.0 {
            return None;
        }
        let d = if diagonal {
            self.dx * std::f64::consts::SQRT_2
        } else {
            self.dx
        };
        Some(d / 1000.0 / kmh)
    }

    /// Enumerate every admissible directed move from `node`, in a
    /// fixed deterministic order.
    pub fn for_moves(&self, node: u32, mut f: impl FnMut(Move)) {
        let n = self.w * self.h;
        let cell = self.cell_of(node);
        let (x, y) = ((cell % self.w) as i64, (cell / self.w) as i64);
        if !self.is_water_node(node) {
            if !self.land_node[cell as usize] {
                return;
            }
            // Queen moves.
            for (dxc, dyc) in QUEEN {
                if let Some(j) = self.at(x + dxc, y + dyc) {
                    let diagonal = dxc != 0 && dyc != 0;
                    if let Some(hours) = self.land_step(cell, j, diagonal) {
                        f(Move { to: j, hours });
                    }
                }
            }
            // Knight moves.
            for (dxc, dyc) in KNIGHT {
                if let Some(j) = self.at(x + dxc, y + dyc) {
                    if let Some(hours) = self.knight_step(cell, j) {
                        f(Move { to: j, hours });
                    }
                }
            }
            // Switches: same cell, then adjacent water nodes.
            if self.water_node[cell as usize] {
                f(Move {
                    to: n + cell,
                    hours: self.cfg.transship_hours,
                });
            }
            for (dxc, dyc) in QUEEN {
                if let Some(j) = self.at(x + dxc, y + dyc) {
                    if self.water_node[j as usize] {
                        f(Move {
                            to: n + j,
                            hours: self.cfg.transship_hours,
                        });
                    }
                }
            }
        } else {
            if !self.water_node[cell as usize] {
                return;
            }
            for (dxc, dyc) in QUEEN {
                if let Some(j) = self.at(x + dxc, y + dyc) {
                    let diagonal = dxc != 0 && dyc != 0;
                    if let Some(hours) = self.water_step(cell, j, diagonal) {
                        f(Move { to: n + j, hours });
                    }
                }
            }
            if self.land_node[cell as usize] {
                f(Move {
                    to: cell,
                    hours: self.cfg.transship_hours,
                });
            }
            for (dxc, dyc) in QUEEN {
                if let Some(j) = self.at(x + dxc, y + dyc) {
                    if self.land_node[j as usize] {
                        f(Move {
                            to: j,
                            hours: self.cfg.transship_hours,
                        });
                    }
                }
            }
        }
    }

    /// Recompute the cost of a specific directed move, or None if
    /// inadmissible — used by the trunk gates and stored-cost fields.
    pub fn move_cost(&self, from: u32, to: u32) -> Option<f64> {
        let mut found = None;
        self.for_moves(from, |m| {
            if m.to == to && found.is_none() {
                found = Some(m.hours);
            }
        });
        found
    }

    /// Land-leg energy of a directed step between cells (Queen or
    /// Knight geometry inferred from the offset); None for non-land
    /// steps.
    pub fn step_energy(&self, from_cell: u32, to_cell: u32) -> f64 {
        let (x1, y1) = ((from_cell % self.w) as i64, (from_cell / self.w) as i64);
        let (x2, y2) = ((to_cell % self.w) as i64, (to_cell / self.w) as i64);
        let (adx, ady) = ((x2 - x1).abs(), (y2 - y1).abs());
        let (ha, hb) = (
            self.heights_m[from_cell as usize],
            self.heights_m[to_cell as usize],
        );
        if adx <= 1 && ady <= 1 {
            let d = if adx + ady == 2 {
                self.dx * std::f64::consts::SQRT_2
            } else {
                self.dx
            };
            self.walk_energy(ha, hb, d)
        } else {
            // Knight: two submoves via the boundary midpoint.
            let (dxc, dyc) = (x2 - x1, y2 - y1);
            let (m1, m2) = if dxc.abs() == 2 {
                let mx = x1 + dxc / 2;
                (
                    (y1 * self.w as i64 + mx) as usize,
                    ((y1 + dyc) * self.w as i64 + mx) as usize,
                )
            } else {
                let my = y1 + dyc / 2;
                (
                    (my * self.w as i64 + x1) as usize,
                    (my * self.w as i64 + x1 + dxc) as usize,
                )
            };
            let hmid = (self.heights_m[m1] + self.heights_m[m2]) / 2.0;
            let half = self.dx * (5.0f64).sqrt() / 2.0;
            self.walk_energy(ha, hmid, half) + self.walk_energy(hmid, hb, half)
        }
    }

    #[inline]
    fn at(&self, x: i64, y: i64) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            None
        } else {
            Some(y as u32 * self.w + x as u32)
        }
    }
}

enum Straddle {
    Free,
    Crossing,
}

const QUEEN: [(i64, i64); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

const KNIGHT: [(i64, i64); 8] = [
    (-1, -2),
    (1, -2),
    (-2, -1),
    (2, -1),
    (-2, 1),
    (2, 1),
    (-1, 2),
    (1, 2),
];

pub(crate) fn for_neighbors8(w: u32, h: u32, i: u32, mut f: impl FnMut(u32, f64)) {
    let (x, y) = ((i % w) as i64, (i / w) as i64);
    for (dxc, dyc) in QUEEN {
        let (nx, ny) = (x + dxc, y + dyc);
        if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
            let fac = if dxc != 0 && dyc != 0 {
                std::f64::consts::SQRT_2
            } else {
                1.0
            };
            f(ny as u32 * w + nx as u32, fac);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CorridorsConfig;

    /// A flat 4×4 test world. Cell (2,2) is ocean; (1,1) is a fordable
    /// channel cell draining diagonally into it (a river mouth);
    /// (0,2) and (1,3) hold wadeable standing water; everything else
    /// is dry ground. Heights are flat so every diagonal costs the
    /// same walking time and delays are the only difference.
    struct World {
        heights: Vec<f64>,
        receivers: Vec<u32>,
        strahler: Vec<u32>,
        water_depth: Vec<f64>,
        navigable: Vec<u32>,
        crossability: Vec<f32>,
        ford_depth: Vec<f32>,
        ford_velocity: Vec<f32>,
        fetch: Vec<f32>,
    }

    const W: u32 = 4;
    const fn idx(x: u32, y: u32) -> u32 {
        y * W + x
    }

    fn world() -> World {
        let n = 16usize;
        let mut w = World {
            heights: vec![1.0; n],
            receivers: (0..n as u32).collect(),
            strahler: vec![0; n],
            water_depth: vec![0.0; n],
            navigable: vec![0; n],
            crossability: vec![0.0; n],
            ford_depth: vec![0.0; n],
            ford_velocity: vec![0.0; n],
            fetch: vec![0.0; n],
        };
        // Ocean at (2,2): self-receiver, below sea level.
        w.heights[idx(2, 2) as usize] = -5.0;
        // Every other cell drains somewhere (non-self ⇒ land).
        for i in 0..n as u32 {
            if i != idx(2, 2) {
                w.receivers[i as usize] = if i == 0 { 1 } else { i - 1 };
            }
        }
        // The mouth: (1,1) is a channel cell draining diagonally into
        // the ocean cell, shallow enough to ford.
        w.strahler[idx(1, 1) as usize] = 1;
        w.receivers[idx(1, 1) as usize] = idx(2, 2);
        w.crossability[idx(1, 1) as usize] = 0.2;
        w.ford_depth[idx(1, 1) as usize] = 0.3;
        w.ford_velocity[idx(1, 1) as usize] = 0.6;
        // Wadeable standing water on the (0,2)/(1,3) corner pair.
        w.water_depth[idx(0, 2) as usize] = 0.5;
        w.water_depth[idx(1, 3) as usize] = 0.5;
        w
    }

    fn model_of<'a>(w: &'a World, cfg: &CorridorsConfig) -> MoveModel<'a> {
        MoveModel::build(
            &ModelInputs {
                w: W,
                h: 4,
                dx: 16.0,
                heights_m: &w.heights,
                receivers: &w.receivers,
                strahler: &w.strahler,
                water_depth: &w.water_depth,
                navigable: &w.navigable,
                crossability: &w.crossability,
                ford_depth: &w.ford_depth,
                ford_velocity: &w.ford_velocity,
                harbour_fetch: &w.fetch,
            },
            cfg,
        )
    }

    /// A diagonal land move may not slip between the two corner cells
    /// of a watercourse without paying the ford gate — including where
    /// the channel runs diagonally into the sea at its mouth, and
    /// where the corners hold standing water rather than a channel.
    #[test]
    fn diagonal_moves_cannot_corner_cut_a_watercourse() {
        let world = world();
        let cfg = CorridorsConfig::default();
        let model = model_of(&world, &cfg);
        let delay = cfg.ford_delay_hours;

        // Control: a diagonal whose corners are both dry ground.
        let control = model
            .move_cost(idx(2, 0), idx(3, 1))
            .expect("dry diagonal admissible");

        // Across the river mouth: corners are the channel cell and its
        // ocean receiver, so the thread runs through the shared corner.
        let mouth = model
            .move_cost(idx(2, 1), idx(1, 2))
            .expect("fordable mouth crossing admissible");
        assert!(
            (mouth - control - delay).abs() < 1e-12,
            "mouth diagonal charged {mouth} h, expected control {control} + delay {delay}"
        );

        // Across standing water: both corners are wadeable ponds.
        let ponds = model
            .move_cost(idx(0, 3), idx(1, 2))
            .expect("wadeable pond crossing admissible");
        assert!(
            (ponds - control - delay).abs() < 1e-12,
            "pond diagonal charged {ponds} h, expected control {control} + delay {delay}"
        );

        // With the caps tightened below the channel's hazard, the same
        // mouth diagonal becomes inadmissible rather than free.
        let strict = CorridorsConfig {
            ford_max_dv_m2s: 0.01,
            ..CorridorsConfig::default()
        };
        let strict_model = model_of(&world, &strict);
        assert_eq!(
            strict_model.move_cost(idx(2, 1), idx(1, 2)),
            None,
            "an unfordable mouth must block the diagonal, not pass it free"
        );
    }
}
