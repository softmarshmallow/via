//! Facts checked after (R14, `derived.md` 5): each stated lake and river against the finished build,
//! and, where one does not hold, where and what would hold instead (principle 8).
//!
//! This reads only what a lint reads: the finished ground (the surface), the finished water (the
//! field tagged `water_level`), the stated facts, and the courses the generate step returned. Where
//! a river's water goes and how much land drains to it are traced again over the finished ground
//! and water; a returned course only names the water in what the lint says. So it judges what the
//! build holds after every effect, not what the model made before them.

use via_num as num;

use crate::raster;
use crate::settle::{Hold, Lake, Posts, River};

/// A course the generate step returned, as the lint reads it back.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// Its name in the build (`<step>#<key>`).
    pub name: String,
    pub fact: String,
    /// Metres, source first.
    pub points: Vec<(f64, f64)>,
    pub end: String,
    /// The share of the land the step says drains to it; `None` when it did not say. The lint
    /// measures its own and only reports this beside it.
    pub catchment: Option<f64>,
}

/// What the lint is handed.
pub struct Build<'a> {
    pub posts: Posts,
    pub ground: &'a [f64],
    /// `None` when the world has no field tagged `water_level`.
    pub water: Option<&'a [f64]>,
    pub lakes: &'a [Lake],
    pub rivers: &'a [River],
    pub holds: &'a [Hold],
    /// The ground the generate step started from (its returned field tagged `settled_from`);
    /// `None` when it is not at hand, and then a hold without a `level` cannot be checked.
    pub start: Option<&'a [f64]>,
    pub found: &'a [Found],
}

/// The lint's parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct Limits {
    /// How far from its stated level a lake may stand, metres.
    pub level_within: f64,
    /// How far the water may stray from a stated river's line, metres.
    pub follow_within: f64,
    /// The least share of the land a river should gather.
    pub min_catchment: f64,
    /// How far the finished ground under a hold may stand from where it was held, metres.
    pub hold_within: f64,
    /// The sea's level, metres, as the generate step's `sea`: a river's water ends where it
    /// reaches ground at or below it.
    pub sea: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            level_within: 1.5,
            follow_within: 50.0,
            min_catchment: 0.02,
            hold_within: 0.5,
            sea: 0.0,
        }
    }
}

/// One finding: a step diagnostic (`protocol.md` 5), without the step's prefix on its code.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub severity: &'static str,
    pub code: &'static str,
    pub subject: String,
    pub message: String,
    pub hint: String,
    pub position: Option<(f64, f64)>,
    /// Canonical JSON text of an object: what would hold instead, in numbers.
    pub data: String,
}

/// The `explain` text of each code (`protocol.md` 3.3).
pub fn explain(code: &str) -> &'static str {
    match code {
        "lake_perched" => "A stated lake stands above the land around it: along much of its shore the ground falls away below its level, so holding the water there would take a dam. The terrain step holds a lake's shore at its level and floods the hollow inside; it cannot make a hollow where the land falls away.",
        "lake_dry" => "After the land settled, too little of a stated lake holds water: its hollow silted up, or the water ran out through a gap in its shore.",
        "lake_level" => "A stated lake stands further from its stated level than the lint allows (`level_within`).",
        "river_uphill" => "A stated river's mouth stands higher than its source on the finished ground, so its water cannot run from one to the other.",
        "river_leaves" => "The water that rises at a stated river's source leaves the stated line and ends far from its mouth: the land beside the line is lower than the groove the terrain step cut along it.",
        "river_wanders" => "The water strays from a stated river's line further than the lint allows (`follow_within`), then rejoins it.",
        "river_dry" => "Little of the land drains to a stated river: it runs along high ground, and its water flows away from it.",
        "river_lost" => "No course was found from a stated river's source: the water there stands in a pit or the sea, or the terrain step returned nothing.",
        "no_water" => "The world has no field tagged `water_level`, so no lake can be checked for its water.",
        "hold_moved" => "Ground stated to stay as it lies (a `terrain_hold`) stands higher or lower in the finished build than where the terrain step held it: an effect applied after the step (a channel, a pad, a way) or the settled land around it moved it.",
        "hold_unchecked" => "A `terrain_hold` without a `level` is checked against the ground the terrain step started from, and that ground is not at hand: the step returned no field tagged `settled_from`.",
        "grid_locked" => "The terrain model's channels run straight along its grid (a row, a column or a diagonal) over much of their length, so the gullies it cut and the wet ground a library paints from them read as ruler-straight trenches rather than drainage. A finer cell or more steps lets them follow the land.",
        _ => "A finding of the terrain library's facts lint.",
    }
}

fn json_num(v: f64) -> String {
    // Three decimals, written from an integer, so the text is the same on every system.
    let m = (v * 1000.0).round_ties_even() as i64;
    let sign = if m < 0 { "-" } else { "" };
    let a = m.unsigned_abs();
    let s = format!("{sign}{}.{:03}", a / 1000, a % 1000);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

fn point(p: (f64, f64)) -> String {
    format!("[{}, {}]", json_num(p.0), json_num(p.1))
}

/// Metres as a short number for a message.
fn m1(v: f64) -> String {
    json_num((v * 10.0).round_ties_even() / 10.0)
}

/// Checks every fact; findings in the order of the facts, lakes first.
pub fn check(b: &Build<'_>, lim: &Limits) -> Vec<Finding> {
    let mut out = Vec::new();
    if !b.lakes.is_empty() && b.water.is_none() {
        out.push(Finding {
            severity: "warning",
            code: "no_water",
            subject: format!("feature.{}", b.lakes[0].name),
            message: "no field is tagged `water_level`, so no stated lake's water can be checked"
                .into(),
            hint: "declare a field with `tags = [\"water_level\"]` and `base = \"= f('height')\"`"
                .into(),
            position: lake_point(&b.lakes[0]),
            data: format!("{{\"lakes\": {}, \"water_level\": null}}", b.lakes.len()),
        });
    }
    if !b.lakes.is_empty() {
        // The level water at each post would stand at before it can leave the extent, over the
        // finished ground: a lake keeps no more than its lowest outlet.
        let spill = route(b.posts, b.ground).filled;
        for lake in b.lakes {
            check_lake(b, &spill, lake, lim, &mut out);
        }
    }
    if !b.rivers.is_empty() {
        let flow = Flow::of(b);
        for river in b.rivers {
            check_river(b, &flow, river, lim, &mut out);
        }
    }
    for hold in b.holds {
        check_hold(b, hold, lim, &mut out);
    }
    out
}

/// A lake's first vertex in metres, for a finding about the lake as a whole.
fn lake_point(lake: &Lake) -> Option<(f64, f64)> {
    let p = lake.rings.first()?.first()?;
    Some((num::metres(p.0), num::metres(p.1)))
}

/// The posts a hold holds: inside a region, or within one post spacing of a point or a line.
fn hold_posts(ps: Posts, hold: &Hold) -> Vec<usize> {
    let r = num::metres(ps.res);
    let line: Vec<(f64, f64)> = hold
        .rings
        .first()
        .map(|ring| {
            ring.iter()
                .map(|&(x, y)| (num::metres(x), num::metres(y)))
                .collect()
        })
        .unwrap_or_default();
    (0..ps.len())
        .filter(|&k| {
            let (i, j) = (k % ps.nx, k / ps.nx);
            match hold.geometry {
                2 => raster::in_rings(&hold.rings, ps.at(i, j)),
                _ if line.len() == 1 => raster::length(ps.metres(i, j), line[0]) <= r,
                _ if line.len() >= 2 => raster::to_polyline(ps.metres(i, j), &line) <= r,
                _ => false,
            }
        })
        .collect()
}

/// R14 for a hold: the finished ground under it against where the step held it (its `level`, or
/// the ground the step started from).
fn check_hold(b: &Build<'_>, hold: &Hold, lim: &Limits, out: &mut Vec<Finding>) {
    let ps = b.posts;
    let posts = hold_posts(ps, hold);
    let subject = format!("feature.{}", hold.name);
    let held = |k: usize| -> Option<f64> { hold.level.or_else(|| b.start.map(|s| s[k])) };
    if hold.level.is_none() && b.start.is_none() {
        let at = posts.first().map(|&k| ps.metres(k % ps.nx, k / ps.nx));
        out.push(Finding {
            severity: "warning",
            code: "hold_unchecked",
            subject,
            message: "it has no `level`, and the ground the terrain step started from is not at hand, so whether it held cannot be checked".into(),
            hint: "give it a `level`, or check it with the lint of the terrain step that read it (`checks`)".into(),
            position: at,
            data: "{\"level\": null, \"settled_from\": null}".into(),
        });
        return;
    }
    let mut worst: Option<(usize, f64)> = None;
    for &k in &posts {
        let Some(h) = held(k) else { continue };
        let moved = b.ground[k] - h;
        if (moved).abs() > lim.hold_within && worst.is_none_or(|(_, w)| (moved).abs() > (w).abs()) {
            worst = Some((k, moved));
        }
    }
    let Some((k, moved)) = worst else { return };
    let at = ps.metres(k % ps.nx, k / ps.nx);
    let stated = held(k).unwrap_or(b.ground[k]);
    out.push(Finding {
        severity: "error",
        code: "hold_moved",
        subject,
        message: format!(
            "the ground under it was held at {} m and stands at {} m at ({}, {}), {} m {}",
            m1(stated),
            m1(b.ground[k]),
            m1(at.0),
            m1(at.1),
            m1((moved).abs()),
            if moved < 0.0 { "lower" } else { "higher" }
        ),
        hint: format!(
            "move what changes the ground after the terrain step away from it, or let it stand at {} m by its `level`, or allow more than {} m with the lint's `hold_within`",
            m1(b.ground[k]),
            m1(lim.hold_within)
        ),
        position: Some(at),
        data: format!(
            "{{\"height\": {}, \"moved\": {}, \"stated\": {}}}",
            json_num(b.ground[k]),
            json_num(moved),
            json_num(stated)
        ),
    });
}

fn check_lake(b: &Build<'_>, spill: &[f64], lake: &Lake, lim: &Limits, out: &mut Vec<Finding>) {
    let ps = b.posts;
    let n = ps.len();
    let inside: Vec<bool> = (0..n)
        .map(|k| raster::in_rings(&lake.rings, ps.at(k % ps.nx, k / ps.nx)))
        .collect();
    if !inside.iter().any(|&x| x) {
        return;
    }
    let r = num::metres(ps.res);
    let d = raster::distance(&inside, ps.nx, ps.ny);
    // The land just beyond the shore: from 10 m to 25 m out, at least one post wide.
    let (lo, hi) = (num::max(10.0, r), num::max(25.0, 2.0 * r));
    let mut outer = Vec::new();
    for k in 0..n {
        if inside[k] {
            continue;
        }
        let dist = d[k].map_or(f64::INFINITY, |e| e.0.sqrt() * r);
        if dist >= lo && dist <= hi {
            outer.push(k);
        }
    }
    let subject = format!("feature.{}", lake.name);
    let at = |k: usize| ps.metres(k % ps.nx, k / ps.nx);
    let outer_ground: Vec<f64> = outer.iter().map(|&k| b.ground[k]).collect();
    let below: Vec<usize> = outer
        .iter()
        .copied()
        .filter(|&k| b.ground[k] < lake.level - 1.0)
        .collect();
    let falls = below.len() as f64 / outer.len().max(1) as f64;
    // What the lake keeps: the spill level of its deepest post, the lowest outlet on its rim.
    let deepest = (0..n)
        .filter(|&k| inside[k])
        .min_by(|&x, &y| b.ground[x].total_cmp(&b.ground[y]).then(x.cmp(&y)))
        .unwrap_or(0);
    // Water at the level runs out through its lowest outlet; or, where the land beyond the shore
    // falls below the level along much of it, the shore the step held stands as a dam, and the
    // level that ground holds is the land's (a tank, not a lake).
    let leaks = spill[deepest] < lake.level - lim.level_within;
    let tank = falls > 0.4;
    let would = if leaks {
        spill[deepest]
    } else {
        num::min(
            raster::quantile(&outer_ground, 0.3).unwrap_or(lake.level),
            lake.level,
        )
    };
    if leaks || tank {
        // The outlet: the lowest post just outside the shore that the kept water reaches.
        let outlet = outer
            .iter()
            .copied()
            .filter(|&k| spill[k] <= would)
            .min_by(|&x, &y| b.ground[x].total_cmp(&b.ground[y]).then(x.cmp(&y)))
            .or_else(|| {
                below
                    .iter()
                    .copied()
                    .min_by(|&x, &y| b.ground[x].total_cmp(&b.ground[y]).then(x.cmp(&y)))
            })
            .unwrap_or(deepest);
        out.push(Finding {
            severity: "error",
            code: "lake_perched",
            subject,
            message: if leaks {
                format!(
                    "water at its level of {} m runs out over the finished ground and keeps {} m, {} m lower, through ({}, {})",
                    m1(lake.level),
                    m1(would),
                    m1(lake.level - would),
                    m1(at(outlet).0),
                    m1(at(outlet).1)
                )
            } else {
                format!(
                    "the land falls away below its level of {} m along {}% of its shore, by {} m at ({}, {}): it stands above its surroundings like a tank",
                    m1(lake.level),
                    (falls * 100.0).round_ties_even(),
                    m1(lake.level - b.ground[outlet]),
                    m1(at(outlet).0),
                    m1(at(outlet).1)
                )
            },
            hint: format!(
                "state a level of about {} m, which this ground holds; or close its shore, or move the lake into a hollow",
                m1(would)
            ),
            position: Some(at(outlet)),
            data: format!(
                "{{\"falls\": {}, \"level\": {}, \"would_hold\": {}}}",
                json_num(falls),
                json_num(lake.level),
                json_num(would)
            ),
        });
        return;
    }
    let Some(water) = b.water else { return };
    let mut wet = Vec::new();
    let (mut cx, mut cy, mut count) = (0.0, 0.0, 0.0);
    for k in 0..n {
        if !inside[k] {
            continue;
        }
        let p = at(k);
        cx += p.0;
        cy += p.1;
        count += 1.0;
        if water[k] - b.ground[k] > 0.2 {
            wet.push(water[k]);
        }
    }
    let centre = (cx / count, cy / count);
    let flooded = wet.len() as f64 / count;
    if flooded < 0.7 {
        let would = raster::quantile(&outer_ground, 0.1).unwrap_or(lake.level);
        out.push(Finding {
            severity: "error",
            code: "lake_dry",
            subject,
            message: format!(
                "after the land settled, only {}% of it holds water: it silted up or ran out",
                (flooded * 100.0).round_ties_even()
            ),
            hint: format!(
                "lower its level toward {} m, the lowest ground around it, or state it in a hollow",
                m1(would)
            ),
            position: Some(centre),
            data: format!(
                "{{\"flooded\": {}, \"would_hold\": {}}}",
                json_num(flooded),
                json_num(would)
            ),
        });
        return;
    }
    let stands = raster::quantile(&wet, 0.5).unwrap_or(lake.level);
    if (stands - lake.level).abs() > lim.level_within {
        out.push(Finding {
            severity: "error",
            code: "lake_level",
            subject,
            message: format!(
                "it was stated at {} m and stands at {} m",
                m1(lake.level),
                m1(stands)
            ),
            hint: format!(
                "state {} m, or allow more than {} m with the lint's `level_within`",
                m1(stands),
                m1(lim.level_within)
            ),
            position: Some(centre),
            data: format!(
                "{{\"stands\": {}, \"stated\": {}}}",
                json_num(stands),
                json_num(lake.level)
            ),
        });
    }
}

/// Where water goes over a surface on the posts: priority flood from the extent's edge (Barnes,
/// Lehman and Mulla 2014, with a first-in first-out order among equal heights so flats are crossed
/// by the shortest way), each post's receiver the post it was reached from, and the level each
/// post's water would stand at before it can leave the extent (`filled`).
struct Route {
    filled: Vec<f64>,
    receiver: Vec<usize>,
    /// Every post after its receiver.
    order: Vec<usize>,
}

/// A total order on finite heights that a heap can use.
fn height_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 {
        !b
    } else {
        b ^ (1u64 << 63)
    }
}

fn route(ps: Posts, surface: &[f64]) -> Route {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let n = ps.len();
    let mut filled = surface.to_vec();
    let mut receiver: Vec<usize> = (0..n).collect();
    let mut seen = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut heap: BinaryHeap<Reverse<(u64, u64, usize)>> = BinaryHeap::new();
    let mut seq = 0u64;
    for k in 0..n {
        let (i, j) = (k % ps.nx, k / ps.nx);
        if i == 0 || j == 0 || i + 1 == ps.nx || j + 1 == ps.ny {
            seen[k] = true;
            heap.push(Reverse((height_key(filled[k]), seq, k)));
            seq += 1;
        }
    }
    while let Some(Reverse((_, _, c))) = heap.pop() {
        order.push(c);
        let (ci, cj) = ((c % ps.nx) as i64, (c / ps.nx) as i64);
        for (di, dj) in [
            (0i64, -1i64),
            (-1, 0),
            (1, 0),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let (ni, nj) = (ci + di, cj + dj);
            if ni < 0 || nj < 0 || ni >= ps.nx as i64 || nj >= ps.ny as i64 {
                continue;
            }
            let nb = nj as usize * ps.nx + ni as usize;
            if seen[nb] {
                continue;
            }
            seen[nb] = true;
            receiver[nb] = c;
            filled[nb] = num::max(filled[nb], filled[c]);
            heap.push(Reverse((height_key(filled[nb]), seq, nb)));
            seq += 1;
        }
    }
    // Where the filled surface falls, the water takes the steepest way down it (D8); only across
    // a flat (a filled hollow, standing water) does it follow the flood back to the outlet. A post
    // lower on the filled surface was taken from the heap earlier, so `order` stays topological.
    for k in 0..n {
        let (ci, cj) = ((k % ps.nx) as i64, (k / ps.nx) as i64);
        if ci == 0 || cj == 0 || ci + 1 == ps.nx as i64 || cj + 1 == ps.ny as i64 {
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for (di, dj) in [
            (0i64, -1i64),
            (-1, 0),
            (1, 0),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let nb = (cj + dj) as usize * ps.nx + (ci + di) as usize;
            let len = if di != 0 && dj != 0 {
                std::f64::consts::SQRT_2
            } else {
                1.0
            };
            let drop = (filled[k] - filled[nb]) / len;
            if drop > 0.0 && best.is_none_or(|(_, d)| drop > d) {
                best = Some((nb, drop));
            }
        }
        if let Some((nb, _)) = best {
            receiver[k] = nb;
        }
    }
    Route {
        filled,
        receiver,
        order,
    }
}

/// How deep water must stand in a dry hollow of the finished build before the lint says the
/// river's water is lost there, metres.
const PIT_DEPTH: f64 = 1.0;
/// Standing water deeper than this is water the build holds, metres.
const WET: f64 = 0.2;

/// How a traced river ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ends {
    /// It reached the stated mouth.
    Mouth,
    /// It reached the sea.
    Sea,
    /// It ran into a stated lake.
    Lake,
    /// It left the extent.
    Edge,
    /// It stands in a hollow of the finished ground that holds no water.
    Pit,
    /// Its source is outside the extent.
    Outside,
}

impl Ends {
    fn word(self) -> &'static str {
        match self {
            Ends::Mouth => "mouth",
            Ends::Sea => "sea",
            Ends::Lake => "lake",
            Ends::Edge => "edge",
            Ends::Pit => "pit",
            Ends::Outside => "outside",
        }
    }
}

/// The finished build's water, as the lint re-derives it: the surface the water runs on (the
/// finished ground, or the finished water where it stands above it), its route, and how many dry
/// posts drain through each post.
struct Flow {
    route: Route,
    surface: Vec<f64>,
    dry: Vec<bool>,
    upstream: Vec<u64>,
    dry_total: u64,
    lake_posts: Vec<bool>,
}

impl Flow {
    fn of(b: &Build<'_>) -> Flow {
        let ps = b.posts;
        let surface: Vec<f64> = match b.water {
            Some(w) => b
                .ground
                .iter()
                .zip(w)
                .map(|(&g, &l)| num::max(g, l))
                .collect(),
            None => b.ground.to_vec(),
        };
        let dry: Vec<bool> = match b.water {
            Some(w) => b
                .ground
                .iter()
                .zip(w)
                .map(|(&g, &l)| l - g <= WET)
                .collect(),
            None => vec![true; ps.len()],
        };
        let route = route(ps, &surface);
        let mut upstream: Vec<u64> = dry.iter().map(|&d| u64::from(d)).collect();
        for &c in route.order.iter().rev() {
            let r = route.receiver[c];
            if r != c {
                upstream[r] += upstream[c];
            }
        }
        let dry_total =
            upstream
                .iter()
                .zip(&route.receiver)
                .enumerate()
                .fold(
                    0u64,
                    |acc, (k, (&u, &r))| if r == k { acc + u } else { acc },
                );
        let lake_posts: Vec<bool> = (0..ps.len())
            .map(|k| {
                b.lakes
                    .iter()
                    .any(|l| raster::in_rings(&l.rings, ps.at(k % ps.nx, k / ps.nx)))
            })
            .collect();
        Flow {
            route,
            surface,
            dry,
            upstream,
            dry_total,
            lake_posts,
        }
    }

    /// The water from a point over the finished build, to where it ends: the posts passed and how
    /// it ends. It stops within `reach` metres of `mouth`, when given.
    fn trace(
        &self,
        b: &Build<'_>,
        from: (f64, f64),
        mouth: Option<((f64, f64), f64)>,
        sea: f64,
    ) -> (Vec<usize>, Ends) {
        let ps = b.posts;
        let r = num::metres(ps.res);
        let i = ((from.0 - num::metres(ps.x0)) / r).round_ties_even();
        let j = ((from.1 - num::metres(ps.y0)) / r).round_ties_even();
        if i < 0.0 || j < 0.0 || i >= ps.nx as f64 || j >= ps.ny as f64 {
            return (Vec::new(), Ends::Outside);
        }
        let mut k = j as usize * ps.nx + i as usize;
        let mut path = vec![k];
        loop {
            if let Some((m, reach)) = mouth {
                if raster::length(ps.metres(k % ps.nx, k / ps.nx), m) <= reach {
                    return (path, Ends::Mouth);
                }
            }
            if self.lake_posts[k] && path.len() > 1 {
                return (path, Ends::Lake);
            }
            if b.ground[k] <= sea && path.len() > 1 {
                return (path, Ends::Sea);
            }
            if self.dry[k] && self.route.filled[k] - self.surface[k] > PIT_DEPTH {
                return (path, Ends::Pit);
            }
            let next = self.route.receiver[k];
            if next == k || path.len() >= ps.len() {
                return (path, Ends::Edge);
            }
            k = next;
            path.push(k);
        }
    }
}

/// `river_lost`'s data: where the water from the source does go over the finished build (the end
/// of its course, and how it ends), and the nearest point of the stated line from which the water
/// reaches the mouth.
fn lost_data(
    b: &Build<'_>,
    flow: &Flow,
    src: (f64, f64),
    line: &[(f64, f64)],
    reach: f64,
    sea: f64,
) -> String {
    let ps = b.posts;
    let at = |k: usize| ps.metres(k % ps.nx, k / ps.nx);
    let r = num::metres(ps.res);
    let mouth = line[line.len() - 1];
    let (path, ends) = flow.trace(b, src, None, sea);
    let goes = path.last().map_or("null".to_string(), |&k| point(at(k)));
    // Points along the line, every post spacing, nearest the source first.
    let mut reaches = "null".to_string();
    'outer: for w in line.windows(2) {
        let len = raster::length(w[0], w[1]);
        let steps = num::max(1.0, (len / r).ceil()) as usize;
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let p = (
                w[0].0 + t * (w[1].0 - w[0].0),
                w[0].1 + t * (w[1].1 - w[0].1),
            );
            if flow.trace(b, p, Some((mouth, reach)), sea).1 == Ends::Mouth {
                reaches = point(p);
                break 'outer;
            }
        }
    }
    format!(
        "{{\"ends\": \"{}\", \"goes\": {goes}, \"reaches_mouth_from\": {reaches}}}",
        ends.word()
    )
}

fn check_river(b: &Build<'_>, flow: &Flow, river: &River, lim: &Limits, out: &mut Vec<Finding>) {
    let subject = format!("feature.{}", river.name);
    let line: Vec<(f64, f64)> = river
        .line
        .iter()
        .map(|&(x, y)| (num::metres(x), num::metres(y)))
        .collect();
    if line.len() < 2 {
        return;
    }
    let (src, mouth) = (line[0], line[line.len() - 1]);
    let (zs, zm) = (
        b.posts.sample(b.ground, src),
        b.posts.sample(b.ground, mouth),
    );
    if zm > zs + 0.5 {
        out.push(Finding {
            severity: "error",
            code: "river_uphill",
            subject,
            message: format!(
                "the ground at its mouth ({}, {}) is {} m and at its source ({}, {}) is {} m: it would run uphill by {} m",
                m1(mouth.0),
                m1(mouth.1),
                m1(zm),
                m1(src.0),
                m1(src.1),
                m1(zs),
                m1(zm - zs)
            ),
            hint: "swap its ends (`starts_at` and `ends_at`), or move the mouth to lower ground"
                .into(),
            position: Some(mouth),
            data: format!(
                "{{\"mouth\": {}, \"rise\": {}, \"source\": {}}}",
                json_num(zm),
                json_num(zm - zs),
                json_num(zs)
            ),
        });
        return;
    }
    // The course the generate step returned only names the water in what the lint says; where
    // the water goes is read again from the finished build.
    let found = b.found.iter().find(|f| f.fact == river.name);
    let course = found.map_or("null".to_string(), |f| format!("\"{}\"", f.name));
    let r = num::metres(b.posts.res);
    let reach = num::max(2.0 * r, 0.5 * lim.follow_within);
    let (path, ends) = flow.trace(b, src, Some((mouth, reach)), lim.sea);
    let ps = b.posts;
    let at = |k: usize| ps.metres(k % ps.nx, k / ps.nx);
    if matches!(ends, Ends::Pit | Ends::Outside) {
        let stop = path.last().map_or(src, |&k| at(k));
        out.push(Finding {
            severity: "error",
            code: "river_lost",
            subject,
            message: format!(
                "the water from its source stands in a dry hollow of the finished ground at ({}, {}) and goes no further",
                m1(stop.0),
                m1(stop.1)
            ),
            hint: "move its source onto ground that falls toward its mouth, or lower what dams its course after the terrain step".into(),
            position: Some(stop),
            data: format!(
                "{{\"course\": {course}, {}",
                &lost_data(b, flow, src, &line, reach, lim.sea)[1..]
            ),
        });
        return;
    }
    let points: Vec<(f64, f64)> = path.iter().map(|&k| at(k)).collect();
    let stated = raster::polyline_length(&line);
    let off: Vec<f64> = points
        .iter()
        .map(|&p| raster::to_polyline(p, &line))
        .collect();
    let stray = off.iter().position(|&o| o > lim.follow_within);
    let end = *points.last().unwrap_or(&src);
    let end_gap = raster::length(end, mouth);
    let named = found.map_or_else(
        || "the course of its water".to_string(),
        |f| format!("`{}`", f.name),
    );
    if let Some(s) = stray {
        let kept = &points[..s.max(1)];
        let followed = raster::polyline_length(kept);
        let at = points[s];
        if ends != Ends::Mouth && end_gap > 4.0 * lim.follow_within {
            out.push(Finding {
                severity: "error",
                code: "river_leaves",
                subject: subject.clone(),
                message: format!(
                    "the water follows it for {} m of {} m, then leaves it at ({}, {}) and ends at ({}, {}), {} m from its stated mouth",
                    m1(followed),
                    m1(stated),
                    m1(at.0),
                    m1(at.1),
                    m1(end.0),
                    m1(end.1),
                    m1(end_gap)
                ),
                hint: format!(
                    "the land beside the line is lower near ({}, {}): move the line into it, or give the river `route` so its course is found over the land; the course the water took is {named}",
                    m1(at.0),
                    m1(at.1),
                ),
                position: Some(at),
                data: format!(
                    "{{\"course\": {course}, \"ends\": {}, \"followed\": {}, \"stated\": {}, \"stops\": \"{}\"}}",
                    point(end),
                    json_num(followed),
                    json_num(stated),
                    ends.word()
                ),
            });
            return;
        }
        let most = off.iter().copied().fold(0.0, num::max);
        out.push(Finding {
            severity: "warning",
            code: "river_wanders",
            subject: subject.clone(),
            message: format!(
                "the water strays up to {} m from its line, first at ({}, {}), and rejoins it",
                m1(most),
                m1(at.0),
                m1(at.1)
            ),
            hint: format!(
                "move the line toward the course the water took ({named}), or allow more than {} m with the lint's `follow_within`",
                m1(lim.follow_within)
            ),
            position: Some(at),
            data: format!(
                "{{\"course\": {course}, \"most\": {}}}",
                json_num(most)
            ),
        });
    }
    // The share of the dry land that drains through where its water ends.
    let last = *path.last().unwrap_or(&0);
    let catchment = if flow.dry_total > 0 {
        num::min(flow.upstream[last] as f64 / flow.dry_total as f64, 1.0)
    } else {
        0.0
    };
    if catchment < lim.min_catchment {
        let mid = line[line.len() / 2];
        let returned = found
            .and_then(|f| f.catchment)
            .map_or("null".to_string(), json_num);
        out.push(Finding {
            severity: "warning",
            code: "river_dry",
            subject,
            message: format!(
                "only {}% of the land drains to it",
                json_num(catchment * 100.0)
            ),
            hint: format!(
                "lead it along lower ground, or allow less than {} with the lint's `min_catchment`",
                json_num(lim.min_catchment)
            ),
            position: Some(mid),
            data: format!(
                "{{\"catchment\": {}, \"course\": {course}, \"returned\": {returned}, \"would_accept\": {}}}",
                json_num(catchment),
                json_num(lim.min_catchment)
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posts() -> Posts {
        Posts {
            x0: 0,
            y0: 0,
            res: 2000,
            nx: 101,
            ny: 101,
        }
    }

    fn square(c: (i64, i64), r: i64) -> Vec<Vec<(i64, i64)>> {
        vec![vec![
            (c.0 - r, c.1 - r),
            (c.0 + r, c.1 - r),
            (c.0 + r, c.1 + r),
            (c.0 - r, c.1 + r),
        ]]
    }

    fn lake(level: f64) -> Lake {
        Lake {
            name: "tarn".into(),
            rings: square((100_000, 100_000), 30_000),
            level,
            depth: 4.0,
            shore: 30.0,
        }
    }

    /// A bowl: ground rises from 0 at the centre.
    fn bowl(ps: Posts) -> Vec<f64> {
        (0..ps.len())
            .map(|k| {
                let (x, y) = ps.metres(k % ps.nx, k / ps.nx);
                ((x - 100.0) * (x - 100.0) + (y - 100.0) * (y - 100.0)) / 100.0
            })
            .collect()
    }

    #[test]
    fn a_lake_that_holds_says_nothing() {
        let ps = posts();
        let ground = bowl(ps);
        let lake = [lake(15.0)];
        let water: Vec<f64> = ground.iter().map(|&g| num::max(g, 15.0)).collect();
        let b = Build {
            posts: ps,
            ground: &ground,
            water: Some(&water),
            lakes: &lake,
            rivers: &[],
            holds: &[],
            start: None,
            found: &[],
        };
        assert_eq!(check(&b, &Limits::default()), vec![]);
    }

    #[test]
    fn a_lake_above_its_ground_is_perched_with_a_level_that_holds() {
        let ps = posts();
        let ground: Vec<f64> = vec![10.0; ps.len()];
        let lake = [lake(30.0)];
        let water: Vec<f64> = vec![30.0; ps.len()];
        let b = Build {
            posts: ps,
            ground: &ground,
            water: Some(&water),
            lakes: &lake,
            rivers: &[],
            holds: &[],
            start: None,
            found: &[],
        };
        let f = check(&b, &Limits::default());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "lake_perched");
        assert!(f[0].data.contains("\"would_hold\": 10"), "{}", f[0].data);
    }

    #[test]
    fn a_lake_whose_shore_leaks_through_a_narrow_gap_is_perched() {
        let ps = posts();
        let mut ground = bowl(ps);
        // A cut 4 m wide from the lake's middle to the east edge, floored at 5 m.
        for (k, g) in ground.iter_mut().enumerate() {
            let (x, y) = ps.metres(k % ps.nx, k / ps.nx);
            if x >= 100.0 && (y - 100.0).abs() <= 2.0 {
                *g = num::min(*g, 5.0);
            }
        }
        let lake = [lake(15.0)];
        let water: Vec<f64> = ground.iter().map(|&g| num::max(g, 15.0)).collect();
        let b = Build {
            posts: ps,
            ground: &ground,
            water: Some(&water),
            lakes: &lake,
            rivers: &[],
            holds: &[],
            start: None,
            found: &[],
        };
        let f = check(&b, &Limits::default());
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].code, "lake_perched");
        assert!(f[0].data.contains("\"would_hold\": 5"), "{}", f[0].data);
    }

    #[test]
    fn a_river_whose_water_leaves_names_the_course() {
        let ps = posts();
        // The land falls south and, more steeply, east: the water from the source runs
        // south-east and leaves by the east edge, away from the line due south.
        let ground: Vec<f64> = (0..ps.len())
            .map(|k| {
                let (x, y) = ps.metres(k % ps.nx, k / ps.nx);
                100.0 + 0.1 * y - 0.2 * x
            })
            .collect();
        let river = [River {
            name: "river".into(),
            line: vec![(100_000, 190_000), (100_000, 10_000)],
            incise: 2.0,
            fall: 0.003,
            valley: 70.0,
            width: None,
        }];
        let found = [Found {
            name: "terrain#river_course".into(),
            fact: "river".into(),
            points: vec![(100.0, 190.0), (100.0, 120.0), (190.0, 60.0), (195.0, 10.0)],
            end: "edge".into(),
            catchment: Some(0.3),
        }];
        let b = Build {
            posts: ps,
            ground: &ground,
            water: None,
            lakes: &[],
            rivers: &river,
            holds: &[],
            start: None,
            found: &found,
        };
        let tight = Limits {
            follow_within: 20.0,
            ..Limits::default()
        };
        let f = check(&b, &tight);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].code, "river_leaves");
        // Within four times the limit of its mouth, it only wanders.
        assert_eq!(check(&b, &Limits::default())[0].code, "river_wanders");
        assert!(f[0].hint.contains("terrain#river_course"));
        assert!(
            f[0].data.contains("\"course\": \"terrain#river_course\"")
                && f[0].data.contains("\"stops\": \"edge\""),
            "{}",
            f[0].data
        );
        // On ground that falls along the line the water holds, whatever course was returned.
        let along: Vec<f64> = (0..ps.len())
            .map(|k| {
                let (x, y) = ps.metres(k % ps.nx, k / ps.nx);
                0.1 * y + 0.05 * (x - 100.0).abs()
            })
            .collect();
        let held = Build {
            ground: &along,
            ..b
        };
        assert_eq!(check(&held, &tight), vec![]);
        // A dam across the line after the step: the water stands in a dry hollow above it.
        let mut dammed = along.clone();
        for i in 0..ps.nx {
            dammed[50 * ps.nx + i] = 30.0;
        }
        let damb = Build {
            ground: &dammed,
            ..b
        };
        assert_eq!(check(&damb, &tight)[0].code, "river_lost");
        // Swapped ends run uphill.
        let up = [River {
            line: vec![(100_000, 10_000), (100_000, 190_000)],
            ..river[0].clone()
        }];
        let b = Build { rivers: &up, ..b };
        assert_eq!(check(&b, &Limits::default())[0].code, "river_uphill");
    }

    #[test]
    fn a_hold_the_finished_ground_moved_is_reported_with_both_heights() {
        let ps = posts();
        let start: Vec<f64> = vec![20.0; ps.len()];
        let mut ground = start.clone();
        // A channel cut 2 m through the middle of the hold after the step.
        for j in 0..ps.ny {
            ground[j * ps.nx + 50] = 18.0;
        }
        let hold = [Hold {
            name: "green".into(),
            rings: square((100_000, 100_000), 20_000),
            geometry: 2,
            level: None,
        }];
        let b = Build {
            posts: ps,
            ground: &ground,
            water: None,
            lakes: &[],
            rivers: &[],
            holds: &hold,
            start: Some(&start),
            found: &[],
        };
        let f = check(&b, &Limits::default());
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].code, "hold_moved");
        assert_eq!(f[0].position.map(|p| p.0), Some(100.0));
        assert!(f[0].data.contains("\"height\": 18"), "{}", f[0].data);
        assert!(f[0].data.contains("\"stated\": 20"), "{}", f[0].data);
        // Held where it lies: nothing to say. Unchecked without a start or a level.
        let b2 = Build {
            ground: &start,
            ..b
        };
        assert!(check(&b2, &Limits::default()).is_empty());
        let b3 = Build { start: None, ..b };
        assert_eq!(check(&b3, &Limits::default())[0].code, "hold_unchecked");
        // A stated level is checked without a start.
        let level = [Hold {
            level: Some(18.0),
            ..hold[0].clone()
        }];
        let b4 = Build {
            holds: &level,
            start: None,
            ground: &start,
            ..b
        };
        assert_eq!(check(&b4, &Limits::default())[0].code, "hold_moved");
    }

    #[test]
    fn a_lost_river_says_where_its_water_goes() {
        let ps = posts();
        // A pit at (40, 160) beside the source; the line falls south to its mouth on the edge.
        let ground: Vec<f64> = (0..ps.len())
            .map(|k| {
                let (x, y) = ps.metres(k % ps.nx, k / ps.nx);
                let pit = ((x - 40.0) * (x - 40.0) + (y - 160.0) * (y - 160.0)).sqrt() / 2.0;
                num::min(pit, 10.0) + 0.1 * y
            })
            .collect();
        let river = [River {
            name: "river".into(),
            line: vec![(44_000, 160_000), (100_000, 0)],
            incise: 2.0,
            fall: 0.003,
            valley: 70.0,
            width: None,
        }];
        let b = Build {
            posts: ps,
            ground: &ground,
            water: None,
            lakes: &[],
            rivers: &river,
            holds: &[],
            start: None,
            found: &[],
        };
        let f = check(&b, &Limits::default());
        assert_eq!(f[0].code, "river_lost");
        assert!(f[0].data.contains("\"ends\": \"pit\""), "{}", f[0].data);
        assert!(
            !f[0].data.contains("\"reaches_mouth_from\": null"),
            "{}",
            f[0].data
        );
    }

    #[test]
    fn numbers_are_written_from_integers() {
        assert_eq!(json_num(1.0), "1");
        assert_eq!(json_num(-0.0004), "0");
        assert_eq!(json_num(2.5), "2.5");
        assert_eq!(json_num(-12.3456), "-12.346");
    }
}
