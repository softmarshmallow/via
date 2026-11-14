//! The land settled under stated facts (R14): facts in, fields and found courses out.
//!
//! What the author states becomes the model's starting conditions here, in the two forms the model
//! takes (`basin`'s lesson, R14):
//!
//! - **Shape in the starting surface.** A lake is a hollow under its region; a river is a groove
//!   that only ever falls, cut below all ground near its line. The model floods the hollow and
//!   sends water down the groove itself; it is never told where water must be.
//! - **Held cells.** Fixed, and water ends at them: a lake's shore at its level, a held region, the
//!   land at the world's edge. Never a river: held cells end water, so a river held cell by cell is
//!   a row of sinks (the spike's failed first try).
//!
//! The model then runs once (one call, one result: no "make many and pick one"), and what it made
//! is handed back on the world's posts: the settled ground as the change it made to the ground it
//! was handed, standing water, how much water passes, sediment, and for each river the course the
//! water took from its source, as a feature. Whether the facts held is checked afterwards, on the
//! finished build, by `facts`.

use via_num as num;

use crate::raster::{self, smoothstep};
use crate::scale;
use crate::stage::{run_given, Given};

/// The world's posts, as the request's `grid` gives them (`numbers.md` 4): post `(i, j)` stands at
/// `(x0 + i * res, y0 + j * res)` millimetres, rows from the south.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posts {
    pub x0: i64,
    pub y0: i64,
    pub res: i64,
    pub nx: usize,
    pub ny: usize,
}

impl Posts {
    pub fn len(&self) -> usize {
        self.nx * self.ny
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Post `(i, j)` in millimetres.
    pub fn at(&self, i: usize, j: usize) -> (i64, i64) {
        (self.x0 + i as i64 * self.res, self.y0 + j as i64 * self.res)
    }

    /// Post `(i, j)` in metres.
    pub fn metres(&self, i: usize, j: usize) -> (f64, f64) {
        let (x, y) = self.at(i, j);
        (num::metres(x), num::metres(y))
    }

    /// The largest coordinate on each axis, millimetres.
    pub fn far(&self) -> (i64, i64) {
        self.at(self.nx - 1, self.ny - 1)
    }

    /// Bilinear value of `v` (one per post) at a point in metres, clamped to the grid.
    pub fn sample(&self, v: &[f64], p: (f64, f64)) -> f64 {
        let r = num::metres(self.res);
        let u = (p.0 - num::metres(self.x0)) / r;
        let t = (p.1 - num::metres(self.y0)) / r;
        raster::bilinear(v, self.nx, self.ny, u, t)
    }
}

/// "A lake stands here, at this level": a region and its water's height.
#[derive(Clone, Debug, PartialEq)]
pub struct Lake {
    pub name: String,
    /// Outlines and holes, millimetres.
    pub rings: Vec<Vec<(i64, i64)>>,
    pub level: f64,
    /// The bed below the level, away from the shore, metres.
    pub depth: f64,
    /// The run over which the bed shelves from the shore to its depth, metres.
    pub shore: f64,
}

/// "A river runs from here to there": a line from its source to its mouth.
#[derive(Clone, Debug, PartialEq)]
pub struct River {
    pub name: String,
    /// Source first, millimetres.
    pub line: Vec<(i64, i64)>,
    /// How far below the ground near its line the groove begins, metres.
    pub incise: f64,
    /// The least fall of the groove, rise over run.
    pub fall: f64,
    /// The run of the valley's sides either side of the groove, metres.
    pub valley: f64,
    /// The channel's width at its mouth when the author states it, metres: the course takes it
    /// instead of the width from the area it drains (R14: the statement wins).
    pub width: Option<f64>,
}

/// "Hold the ground here": a point, path or region kept at its level (or as it lies). Water ends
/// at it.
#[derive(Clone, Debug, PartialEq)]
pub struct Hold {
    pub name: String,
    pub rings: Vec<Vec<(i64, i64)>>,
    /// 0 point, 1 path, 2 region (`protocol.md` 4.3).
    pub geometry: u8,
    pub level: Option<f64>,
}

/// The step's parameters (its `params` table), each optional in text.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// The model's cell, metres: a whole number of post spacings. Default: about 256 cells across.
    pub cell: Option<f64>,
    /// Cells simulated beyond each edge, so the model's own edge is out of sight.
    pub margin: usize,
    /// The sea's level, metres.
    pub sea: f64,
    /// `true` ("held"): the land at the edge carries on as it lies and water leaves there; `false`
    /// ("open"): beyond the margin the ground falls to the model's base, an ocean all round.
    pub edge_held: bool,
    /// A stated river's groove lies below all ground within this many cells of its line.
    pub hold_cells: usize,
    pub steps: Option<u32>,
    pub erodibility: Option<f64>,
    pub diffusivity: Option<f64>,
    pub deposition: Option<f64>,
    pub timestep: Option<f64>,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            cell: None,
            margin: 16,
            sea: 0.0,
            edge_held: true,
            hold_cells: 3,
            steps: None,
            erodibility: None,
            diffusivity: None,
            deposition: None,
            timestep: None,
        }
    }
}

/// Everything one call reads.
#[derive(Clone, Debug)]
pub struct Input {
    pub posts: Posts,
    /// The ground it was handed (the surface's head view), one value per post, post order.
    pub ground: Vec<f64>,
    pub lakes: Vec<Lake>,
    pub rivers: Vec<River>,
    pub holds: Vec<Hold>,
    pub params: Params,
    pub seed: u64,
}

/// Where a found course ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Sea,
    Lake,
    Held,
    Edge,
    Pit,
}

impl End {
    pub fn word(self) -> &'static str {
        match self {
            End::Sea => "sea",
            End::Lake => "lake",
            End::Held => "held",
            End::Edge => "edge",
            End::Pit => "pit",
        }
    }
}

/// The course the model's water took from a river's source.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    /// The stated river it was followed from.
    pub fact: String,
    /// Millimetres, source first, inside the extent.
    pub points: Vec<(i64, i64)>,
    pub end: End,
    /// The share of the land in the world that drains through its last cell on land.
    pub catchment: f64,
    /// The channel's width and depth at its mouth, metres, from the area it drains.
    pub width: f64,
    pub depth: f64,
}

/// What one call returns, on the world's posts.
#[derive(Clone, Debug)]
pub struct Settled {
    /// The settled ground, metres.
    pub eroded: Vec<f64>,
    /// Standing water over the settled ground, metres (0 where dry).
    pub standing: Vec<f64>,
    /// The level the model's standing water stands at, metres: constant over a lake, read only
    /// from the cells where water stands; one metre below the settled ground where none is near.
    pub level: Vec<f64>,
    /// How damp the ground is, 0 to 1: `wet` spread over about 30 m and normalised, so high on
    /// valley floors and their lower sides and low on ridges.
    pub damp: Vec<f64>,
    /// How much water passes, 0 to 1: 1 where a stream runs.
    pub wet: Vec<f64>,
    /// Sediment settled, metres.
    pub silt: Vec<f64>,
    pub courses: Vec<Course>,
    /// The model's share of channel cells locked to the grid (`gates::GatesReport::axis_runs`).
    pub axis_runs: f64,
    /// Model cells across and up, with the margin, and the cell size in metres.
    pub model: (usize, usize, f64),
}

/// The model's grid over the posts: node `(a, b)` stands on post `((a - pad) * k, (b - pad) * k)`.
#[derive(Clone, Copy, Debug)]
struct Model {
    posts: Posts,
    k: i64,
    pad: usize,
    w: usize,
    h: usize,
    /// Nodes across and up the extent itself.
    ew: usize,
    eh: usize,
}

impl Model {
    fn cell_mm(&self) -> i64 {
        self.k * self.posts.res
    }

    fn cell_m(&self) -> f64 {
        num::metres(self.cell_mm())
    }

    fn node_mm(&self, a: usize, b: usize) -> (i64, i64) {
        let c = self.cell_mm();
        (
            self.posts.x0 + (a as i64 - self.pad as i64) * c,
            self.posts.y0 + (b as i64 - self.pad as i64) * c,
        )
    }

    fn node_m(&self, a: usize, b: usize) -> (f64, f64) {
        let (x, y) = self.node_mm(a, b);
        (num::metres(x), num::metres(y))
    }

    /// The nearest node to a point in millimetres, clamped to the grid; ties to the higher node.
    fn node_of(&self, p: (i64, i64)) -> (usize, usize) {
        let c = self.cell_mm();
        let near = |d: i64| (2 * d + c).div_euclid(2 * c);
        let a = (near(p.0 - self.posts.x0) + self.pad as i64).clamp(0, self.w as i64 - 1);
        let b = (near(p.1 - self.posts.y0) + self.pad as i64).clamp(0, self.h as i64 - 1);
        (a as usize, b as usize)
    }

    fn node_of_m(&self, p: (f64, f64)) -> (usize, usize) {
        let mm = |v: f64| (v * 1000.0).round_ties_even() as i64;
        self.node_of((mm(p.0), mm(p.1)))
    }

    /// Fractional node coordinates of post `(i, j)`.
    fn of_post(&self, i: usize, j: usize) -> (f64, f64) {
        (
            i as f64 / self.k as f64 + self.pad as f64,
            j as f64 / self.k as f64 + self.pad as f64,
        )
    }

    fn in_extent(&self, a: usize, b: usize) -> bool {
        a >= self.pad && b >= self.pad && a < self.pad + self.ew && b < self.pad + self.eh
    }
}

/// The model cell when `params.cell` is not given: a whole number of post spacings, about 256
/// cells across the longer side.
pub const TARGET_CELLS: usize = 256;

fn model_grid(posts: Posts, p: &Params) -> Result<Model, String> {
    let k = match p.cell {
        Some(c) => {
            let mm = (c * 1000.0).round_ties_even() as i64;
            if c <= 0.0 || mm % posts.res != 0 || mm < posts.res {
                return Err(format!(
                    "`cell` = {c} m is not a whole number of post spacings ({} m)",
                    num::metres(posts.res)
                ));
            }
            mm / posts.res
        }
        None => {
            let span = posts.nx.max(posts.ny).saturating_sub(1).max(1);
            span.div_ceil(TARGET_CELLS).max(1) as i64
        }
    };
    let ew = (posts.nx.saturating_sub(1)).div_ceil(k as usize) + 1;
    let eh = (posts.ny.saturating_sub(1)).div_ceil(k as usize) + 1;
    Ok(Model {
        posts,
        k,
        pad: p.margin,
        w: ew + 2 * p.margin,
        h: eh + 2 * p.margin,
        ew,
        eh,
    })
}

/// Settles the land under the input's facts.
pub fn settle(input: &Input) -> Result<Settled, String> {
    let posts = input.posts;
    if posts.nx < 2 || posts.ny < 2 || input.ground.len() != posts.len() {
        return Err("the ground must be a grid of at least 2 by 2 posts".into());
    }
    let p = &input.params;
    let m = model_grid(posts, p)?;
    let (w, h) = (m.w, m.h);
    let n = w * h;
    let cell = m.cell_m();

    // The ground it was handed, on the model's nodes; beyond the extent it carries on level.
    let mut h0 = vec![0.0; n];
    for b in 0..h {
        for a in 0..w {
            let i = ((a as i64 - m.pad as i64) * m.k).clamp(0, posts.nx as i64 - 1) as usize;
            let j = ((b as i64 - m.pad as i64) * m.k).clamp(0, posts.ny as i64 - 1) as usize;
            h0[b * w + a] = input.ground[j * posts.nx + i];
        }
    }

    let mut start = h0.clone();
    let mut held = vec![f64::NAN; n];
    if p.edge_held {
        for b in 0..h {
            for a in 0..w {
                let border = a == 0 || b == 0 || a == w - 1 || b == h - 1;
                let v = h0[b * w + a];
                if border && v > p.sea + 0.5 {
                    held[b * w + a] = v;
                }
            }
        }
    }

    // Lakes: a hollow under the region, its shore held at the level.
    let mut lake_ring = vec![false; n];
    let mut near_lake = vec![false; n];
    for lake in &input.lakes {
        let mut inside: Vec<bool> = (0..n)
            .map(|k| raster::in_rings(&lake.rings, m.node_mm(k % w, k / w)))
            .collect();
        if !inside.iter().any(|&x| x) {
            if let Some(&first) = lake.rings.first().and_then(|r| r.first()) {
                let (a, b) = m.node_of(first);
                inside[b * w + a] = true;
            }
        }
        let grown = raster::dilate(&inside, w, h);
        let outside: Vec<bool> = inside.iter().map(|&x| !x).collect();
        let din = raster::distance(&outside, w, h);
        for k in 0..n {
            if inside[k] {
                let d = din[k].map_or(0.0, |e| e.0.sqrt()) * cell;
                let bed = lake.level - lake.depth * smoothstep(0.0, lake.shore, d);
                start[k] = num::min(start[k], bed);
            } else if grown[k] {
                start[k] = lake.level;
                held[k] = lake.level;
                lake_ring[k] = true;
            }
        }
        let mut wide = grown;
        for _ in 0..2 {
            wide = raster::dilate(&wide, w, h);
        }
        for k in 0..n {
            near_lake[k] |= wide[k];
        }
    }

    // Holds: kept at their level, or as they lie.
    for hold in &input.holds {
        for k in 0..n {
            let (a, b) = (k % w, k / w);
            let here = match hold.geometry {
                2 => raster::in_rings(&hold.rings, m.node_mm(a, b)),
                _ => false,
            };
            if here {
                held[k] = hold.level.unwrap_or(h0[k]);
            }
        }
        if hold.geometry != 2 {
            let line: Vec<(f64, f64)> = hold
                .rings
                .iter()
                .flatten()
                .map(|&(x, y)| (num::metres(x), num::metres(y)))
                .collect();
            for (pt, _) in raster::resample(&line, cell * 0.4) {
                let (a, b) = m.node_of_m(pt);
                let k = b * w + a;
                held[k] = hold.level.unwrap_or(h0[k]);
            }
        }
    }

    // Rivers: a groove that only falls, below all ground near its line.
    let lowest = raster::min_filter(&h0, w, h, p.hold_cells);
    for river in &input.rivers {
        let line: Vec<(f64, f64)> = river
            .line
            .iter()
            .map(|&(x, y)| (num::metres(x), num::metres(y)))
            .collect();
        if line.len() < 2 {
            continue;
        }
        let pts = raster::resample(&line, cell * 0.4);
        let mouth = *river.line.last().unwrap_or(&(0, 0));
        let ends_in = input
            .lakes
            .iter()
            .find(|l| raster::in_rings(&l.rings, mouth))
            .map(|l| l.level);
        // The floor falls at least `fall` per metre and never rises; it reaches the water it ends
        // in (a lake's level, or just below the sea) at the mouth and not before, so the sea does
        // not run up the groove as an inlet.
        let total = pts.last().map_or(0.0, |e| e.1);
        let base = ends_in.unwrap_or(p.sea - 1.0);
        let mut floor = Vec::with_capacity(pts.len());
        let mut run_min = f64::INFINITY;
        let mut first = 0.0;
        for (k, &(pt, run)) in pts.iter().enumerate() {
            let (a, b) = m.node_of_m(pt);
            run_min = num::min(run_min, lowest[b * w + a] - river.incise);
            if k == 0 {
                first = run_min;
            }
            let f = num::min(run_min, first - river.fall * run);
            floor.push(num::max(f, base + river.fall * (total - run)));
        }
        let mut marked = vec![false; n];
        let mut floor_on = vec![0.0; n];
        for (&(pt, _), &f) in pts.iter().zip(&floor) {
            let (a, b) = m.node_of_m(pt);
            let k = b * w + a;
            floor_on[k] = if marked[k] {
                num::min(floor_on[k], f)
            } else {
                f
            };
            marked[k] = true;
        }
        let d = raster::distance(&marked, w, h);
        for k in 0..n {
            if near_lake[k] {
                continue;
            }
            let Some((d2, src)) = d[k] else { continue };
            let near = floor_on[src as usize];
            if start[k] > near {
                let dist = d2.sqrt() * cell;
                let s = smoothstep(cell * 0.7, cell * 0.7 + river.valley, dist);
                start[k] = num::min(start[k], near + (start[k] - near) * s);
            }
        }
    }

    // One run of the model.
    let mut cfg = scale::preset(cell);
    cfg.seed = input.seed;
    cfg.size = w as u32;
    cfg.rows = h as u32;
    cfg.sea_level_m = p.sea;
    cfg.base_depth_m = p.sea - 30.0;
    if let Some(s) = p.steps {
        cfg.steps = s;
    }
    if let Some(v) = p.erodibility {
        cfg.k_spl = v;
    }
    if let Some(v) = p.diffusivity {
        cfg.kappa = v;
    }
    if let Some(v) = p.deposition {
        cfg.g_deposition = v;
    }
    if let Some(v) = p.timestep {
        cfg.dt_years = v;
    }
    cfg.validate()?;
    let given = Given {
        initial_m: Some(start),
        uplift_m_per_yr: Some(vec![0.0; n]),
        held_m: Some(held.clone()),
    };
    let out = run_given(&cfg, &given, &mut |_, _| {});

    // Back on the posts.
    let delta: Vec<f64> = out.heights_m.iter().zip(&h0).map(|(a, b)| a - b).collect();
    let stream_cells = cfg.river_min_area_km2 * 1.0e6 / cfg.cell_area_m2();
    let full = num::log10(1.0 + 4.0 * stream_cells);
    let wetn: Vec<f64> = out
        .discharge_cells
        .iter()
        .map(|&q| num::clamp(num::log10(1.0 + q) / full, 0.0, 1.0))
        .collect();
    let surface: Vec<f64> = out
        .heights_m
        .iter()
        .zip(&out.water_depth_m)
        .map(|(&z, &d)| {
            if z < p.sea {
                num::max(z + d, p.sea)
            } else {
                z + d
            }
        })
        .collect();
    // Where water stands: over 5 cm deep, or the sea however shallow, so the coast's last
    // shallow nodes still carry the sea's level and its edge crosses 0 m between nodes.
    let ponded: Vec<bool> = out
        .water_depth_m
        .iter()
        .zip(&out.heights_m)
        .map(|(&d, &z)| d > 0.05 || z < p.sea)
        .collect();
    let spread = num::max(1.0, (30.0 / cell).round_ties_even()) as usize;
    let blurred = raster::box_blur(&wetn, w, h, spread);
    let most = blurred
        .iter()
        .enumerate()
        .filter(|(k, _)| m.in_extent(k % w, k / w))
        .fold(0.0, |a, (_, &v)| num::max(a, v));
    let dampn: Vec<f64> = blurred
        .iter()
        .map(|&v| {
            if most > 0.0 {
                num::clamp((v / most).sqrt(), 0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect();
    let mut eroded = vec![0.0; posts.len()];
    let mut standing = vec![0.0; posts.len()];
    let mut level = vec![0.0; posts.len()];
    let mut damp = vec![0.0; posts.len()];
    let mut wet = vec![0.0; posts.len()];
    let mut silt = vec![0.0; posts.len()];
    let mut near_water = vec![false; posts.len()];
    for j in 0..posts.ny {
        for i in 0..posts.nx {
            let (u, t) = m.of_post(i, j);
            let k = j * posts.nx + i;
            eroded[k] = input.ground[k] + raster::bicubic(&delta, w, h, u, t);
            // Water stands where the post's own ground lies below the level of the water near
            // it, so its edge follows the ground between the model's nodes, crossing the level
            // where the ground does, never in steps of a node.
            match raster::bilinear_where(&surface, &ponded, w, h, u, t) {
                Some(l) => {
                    level[k] = l;
                    standing[k] = num::max(l - eroded[k], 0.0);
                    near_water[k] = true;
                }
                None => level[k] = eroded[k] - 1.0,
            }
            damp[k] = raster::bilinear(&dampn, w, h, u, t);
            wet[k] = raster::bilinear(&wetn, w, h, u, t);
            silt[k] = num::max(raster::bilinear(&out.sediment_m, w, h, u, t), 0.0);
        }
    }

    // A lake's shore is its stated region, at post resolution: the model held it on its own
    // nodes, so its edge there is the nodes' outline. Near the shore, where the model holds water,
    // the bed shelves from the region's own edge below the level and the water stands at it;
    // outside the region, out to a few nodes, no water stands. The ground outside is left as the
    // model made it, so what the lint reads of a shore that cannot hold is the model's.
    for lake in &input.lakes {
        let mut w = Water {
            eroded: &mut eroded,
            standing: &mut standing,
            level: &mut level,
            near: &near_water,
        };
        shore_on_posts(lake, &m, &mut w);
    }

    // The courses the water took.
    let mut land_cells = 0u64;
    for b in 0..h {
        for a in 0..w {
            if m.in_extent(a, b) && out.heights_m[b * w + a] > p.sea {
                land_cells += 1;
            }
        }
    }
    // Who drains into each node, to count a course's catchment over the world's own land: the
    // free margin around the extent drains too, but it is not land of the world.
    let donors = if input.rivers.is_empty() {
        Vec::new()
    } else {
        donors_of(&out.receivers)
    };
    let counts = |k: usize| m.in_extent(k % w, k / w) && out.heights_m[k] > p.sea;
    let mut courses = Vec::new();
    for river in &input.rivers {
        let Some(&source) = river.line.first() else {
            continue;
        };
        let (mut a, mut b) = m.node_of(source);
        let mut path = vec![(a, b)];
        let mut seen = std::collections::BTreeSet::new();
        let end = loop {
            let k = b * w + a;
            if !m.in_extent(a, b) {
                break End::Edge;
            }
            if lake_ring[k] {
                break End::Lake;
            }
            if held[k].is_finite() && path.len() > 1 {
                break End::Held;
            }
            // The model's base: the sea (ground below it, joined to the edge) or a pit.
            let r = out.receivers[k] as usize;
            if r == k {
                break if out.heights_m[k] < p.sea {
                    End::Sea
                } else {
                    End::Pit
                };
            }
            if !seen.insert(k) {
                break End::Pit;
            }
            (a, b) = (r % w, r / w);
            path.push((a, b));
        };
        let last_land = path
            .iter()
            .rev()
            .map(|&(a, b)| b * w + a)
            .find(|&k| out.heights_m[k] > p.sea)
            .unwrap_or(path[0].1 * w + path[0].0);
        let catchment = if land_cells > 0 {
            num::min(
                upstream_where(&donors, last_land, counts) as f64 / land_cells as f64,
                1.0,
            )
        } else {
            0.0
        };
        let area_km2 = out.discharge_cells[last_land] * cfg.cell_area_m2() / 1.0e6;
        let width = river
            .width
            .unwrap_or_else(|| num::clamp(5.0 * area_km2.sqrt(), 1.5, 60.0));
        let depth = num::clamp(0.3 + 0.06 * width, 0.3, 3.0);
        courses.push(Course {
            fact: river.name.clone(),
            points: course_points(&m, &path, source),
            end,
            catchment,
            width,
            depth,
        });
    }

    Ok(Settled {
        eroded,
        standing,
        level,
        damp,
        wet,
        silt,
        courses,
        axis_runs: out.gates.axis_runs.value,
        model: (w, h, cell),
    })
}

/// The band outside a lake's region, in model cells, where the model's held ring and the nodes
/// inside the region still shape the ground the posts read.
const SHORE_BAND_CELLS: f64 = 2.5;

/// The water on the posts, as [`settle`] hands it back, and where the model holds water near.
struct Water<'a> {
    eroded: &'a mut [f64],
    standing: &'a mut [f64],
    level: &'a mut [f64],
    near: &'a [bool],
}

/// One lake's shore on the posts (see [`settle`]): the wet edge lies within one post spacing of
/// the stated region wherever the model holds the lake.
fn shore_on_posts(lake: &Lake, m: &Model, w: &mut Water<'_>) {
    let posts = m.posts;
    let band = SHORE_BAND_CELLS * m.cell_m();
    let rings: Vec<Vec<(f64, f64)>> = lake
        .rings
        .iter()
        .filter(|r| !r.is_empty())
        .map(|r| {
            let mut v: Vec<(f64, f64)> = r
                .iter()
                .map(|&(x, y)| (num::metres(x), num::metres(y)))
                .collect();
            v.push(v[0]);
            v
        })
        .collect();
    let Some(first) = rings.first().and_then(|r| r.first()) else {
        return;
    };
    let mut lo = *first;
    let mut hi = *first;
    for &(x, y) in rings.iter().flatten() {
        lo = (num::min(lo.0, x), num::min(lo.1, y));
        hi = (num::max(hi.0, x), num::max(hi.1, y));
    }
    let res = num::metres(posts.res);
    let x0 = num::metres(posts.x0);
    let y0 = num::metres(posts.y0);
    let index = |v: f64, origin: f64, n: usize, up: bool| -> usize {
        let f = (v - origin) / res;
        let i = if up { f.ceil() } else { f.floor() };
        num::clamp(i, 0.0, (n - 1) as f64) as usize
    };
    let (i0, i1) = (
        index(lo.0 - band, x0, posts.nx, false),
        index(hi.0 + band, x0, posts.nx, true),
    );
    let (j0, j1) = (
        index(lo.1 - band, y0, posts.ny, false),
        index(hi.1 + band, y0, posts.ny, true),
    );
    for j in j0..=j1 {
        for i in i0..=i1 {
            let k = j * posts.nx + i;
            let pmm = (
                posts.x0 + i as i64 * posts.res,
                posts.y0 + j as i64 * posts.res,
            );
            let p = (num::metres(pmm.0), num::metres(pmm.1));
            let edge = rings
                .iter()
                .map(|r| raster::to_polyline(p, r))
                .fold(f64::INFINITY, num::min);
            if edge > band {
                continue;
            }
            if raster::in_rings(&lake.rings, pmm) {
                // Only where the model holds water near: a hollow that silted up, or a lake that
                // cannot hold, stays as the model left it for the lint to read.
                if w.near[k] {
                    let bed = lake.level - lake.depth * smoothstep(0.0, lake.shore, edge);
                    w.eroded[k] = num::min(w.eroded[k], bed);
                    w.level[k] = num::max(w.level[k], lake.level);
                    w.standing[k] = num::max(w.level[k] - w.eroded[k], 0.0);
                }
            } else {
                w.standing[k] = 0.0;
                w.level[k] = num::min(w.level[k], w.eroded[k] - 1.0);
            }
        }
    }
}

/// A walk from node to node, eased by a running mean over five nodes (the ends kept), rounded to
/// millimetres, clipped to the extent, and starting at the stated source.
fn course_points(m: &Model, path: &[(usize, usize)], source: (i64, i64)) -> Vec<(i64, i64)> {
    let raw: Vec<(f64, f64)> = path.iter().map(|&(a, b)| m.node_m(a, b)).collect();
    let n = raw.len();
    let far = m.posts.far();
    let mut out: Vec<(i64, i64)> = vec![source];
    for k in 1..n {
        let p = if k + 1 == n {
            raw[k]
        } else {
            let (lo, hi) = (k.saturating_sub(2), (k + 2).min(n - 1));
            let c = (hi - lo + 1) as f64;
            let sx: f64 = raw[lo..=hi].iter().map(|p| p.0).sum();
            let sy: f64 = raw[lo..=hi].iter().map(|p| p.1).sum();
            (sx / c, sy / c)
        };
        let q = (
            ((p.0 * 1000.0).round_ties_even() as i64).clamp(m.posts.x0, far.0),
            ((p.1 * 1000.0).round_ties_even() as i64).clamp(m.posts.y0, far.1),
        );
        if out.last() != Some(&q) {
            out.push(q);
        }
    }
    out
}

/// Each node's donors: the nodes whose receiver it is.
fn donors_of(receivers: &[u32]) -> Vec<Vec<u32>> {
    let mut donors = vec![Vec::new(); receivers.len()];
    for (k, &r) in receivers.iter().enumerate() {
        if r as usize != k {
            donors[r as usize].push(k as u32);
        }
    }
    donors
}

/// The nodes that drain to `root`, `root` included, for which `counts` holds.
fn upstream_where(donors: &[Vec<u32>], root: usize, counts: impl Fn(usize) -> bool) -> u64 {
    let mut n = 0u64;
    let mut todo = vec![root];
    while let Some(k) = todo.pop() {
        if counts(k) {
            n += 1;
        }
        todo.extend(donors[k].iter().map(|&d| d as usize));
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posts(n: usize, res: i64) -> Posts {
        Posts {
            x0: 0,
            y0: 0,
            res,
            nx: n,
            ny: n,
        }
    }

    #[test]
    fn default_cell_is_a_whole_number_of_posts() {
        let m = model_grid(posts(1281, 2500), &Params::default()).unwrap();
        assert_eq!((m.k, m.ew, m.w), (5, 257, 289));
        let m = model_grid(posts(65, 1000), &Params::default()).unwrap();
        assert_eq!((m.k, m.ew), (1, 65));
        let bad = Params {
            cell: Some(3.0),
            ..Params::default()
        };
        assert!(model_grid(posts(65, 2000), &bad).is_err());
    }

    #[test]
    fn catchment_counts_only_the_nodes_asked_for() {
        // 0 <- 1 <- 2 <- 3, and 4 -> 2: nodes 3 and 4 stand in the margin.
        let receivers = [0u32, 0, 1, 2, 2];
        let donors = donors_of(&receivers);
        assert_eq!(upstream_where(&donors, 1, |_| true), 4);
        assert_eq!(upstream_where(&donors, 1, |k| k < 3), 2);
        assert_eq!(upstream_where(&donors, 0, |k| k < 3), 3);
    }

    /// The wet edge of a lake lies within one post spacing of its stated region, whatever the
    /// model's nodes made of the ground near it: here the ground dips below the level in node-sized
    /// steps both inside and outside an oval region.
    #[test]
    fn lake_edge_follows_the_region_at_post_resolution() {
        let res = 2500;
        let ps = posts(129, res);
        let p = Params {
            cell: Some(12.5),
            ..Params::default()
        };
        let m = model_grid(ps, &p).unwrap();
        let ring: Vec<(i64, i64)> = (0..32)
            .map(|q| {
                let a = q as f64 * std::f64::consts::TAU / 32.0;
                (
                    (160_000.0 + 61_000.0 * num::cos(a)).round_ties_even() as i64,
                    (160_000.0 + 43_000.0 * num::sin(a)).round_ties_even() as i64,
                )
            })
            .collect();
        let lake = Lake {
            name: "tarn".into(),
            rings: vec![ring],
            level: 10.0,
            depth: 3.0,
            shore: 20.0,
        };
        let mut eroded = vec![0.0; ps.len()];
        let mut standing = vec![0.0; ps.len()];
        let mut level = vec![0.0; ps.len()];
        let near = vec![true; ps.len()];
        for j in 0..ps.ny {
            for i in 0..ps.nx {
                let k = j * ps.nx + i;
                // The model's view: inside on the node the post falls nearest to, a step below
                // the level a node wide around it.
                let (a, b) = m.node_of((i as i64 * res, j as i64 * res));
                let node_in = raster::in_rings(&lake.rings, m.node_mm(a, b));
                eroded[k] = if node_in { 8.0 } else { 9.7 };
                standing[k] = num::max(10.0 - eroded[k], 0.0);
                level[k] = 10.0;
            }
        }
        shore_on_posts(
            &lake,
            &m,
            &mut Water {
                eroded: &mut eroded,
                standing: &mut standing,
                level: &mut level,
                near: &near,
            },
        );
        let ring_m: Vec<(f64, f64)> = lake.rings[0]
            .iter()
            .chain(lake.rings[0].first())
            .map(|&(x, y)| (num::metres(x), num::metres(y)))
            .collect();
        let post_m = num::metres(res);
        for j in 0..ps.ny {
            for i in 0..ps.nx {
                let k = j * ps.nx + i;
                let pmm = (i as i64 * res, j as i64 * res);
                let edge = raster::to_polyline((num::metres(pmm.0), num::metres(pmm.1)), &ring_m);
                let inside = raster::in_rings(&lake.rings, pmm);
                if !inside && edge > SHORE_BAND_CELLS * m.cell_m() {
                    continue; // beyond the shore: the model's own water
                }
                // Wet where the water stands above the ground: `level`, as the water field reads it.
                if level[k] > eroded[k] {
                    assert!(inside || edge <= post_m, "wet at {pmm:?}, {edge} m outside");
                }
                if inside && edge >= post_m {
                    assert!(standing[k] > 0.0, "dry at {pmm:?}, {edge} m inside");
                }
            }
        }
    }

    #[test]
    fn node_of_is_nearest() {
        let m = model_grid(posts(65, 1000), &Params::default()).unwrap();
        assert_eq!(m.node_of((0, 0)), (16, 16));
        assert_eq!(m.node_of((1499, 500)), (17, 17));
        assert_eq!(m.node_mm(16, 16), (0, 0));
    }
}
