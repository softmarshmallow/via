//! The network and its areas (`spec/geometry.md` 3.2): junctions, edges, areas with the sides
//! of the sources that bound them, holes, spurs and representative points, with Euler and
//! closure checked on every build.

use crate::arrange::{kept_loops, Arr};
use crate::diag::{Code, GeomDiag};
use crate::noding::{node, Input, Noded, Seg};
use crate::overlay::internal;
use crate::pred::{self, Side as PSide};
use crate::shape::{canonical, Piece, Region, P};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// A side of a source's travel. A ring source's inside is its left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// Left of travel.
    Left,
    /// Right of travel.
    Right,
}

impl Side {
    /// The word written.
    pub fn word(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// The world's post grid as the kernel reads it: origin and spacing in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostGrid {
    /// `[x0, y0, x1, y1]`.
    pub extent: [i64; 4],
    /// Post spacing.
    pub resolution: i64,
}

/// An edge of the network.
#[derive(Clone, Debug)]
pub struct Edge {
    /// Its two end junctions (indexes into `junctions`), least first.
    pub ends: (usize, usize),
    /// The path from its least end junction.
    pub path: Vec<P>,
    /// The half-edges along `path`.
    pub half: Vec<usize>,
    /// The sources it lies on (indexes into the noding's sources), sorted.
    pub sources: Vec<usize>,
}

/// An area (3.2).
#[derive(Clone, Debug)]
pub struct Area {
    /// The area as a region of one piece.
    pub region: Region,
    /// `(source, side)` of the shell's edges, sorted.
    pub sides: Vec<(usize, Side)>,
    /// The shell's edges, counter-clockwise from its start vertex.
    pub edges: Vec<usize>,
    /// Each hole ring's edges.
    pub holes: Vec<Vec<usize>>,
    /// Edges with this area on both sides.
    pub spurs: Vec<usize>,
    /// The representative point.
    pub point: P,
    /// Naming points strictly inside, sorted.
    pub naming: Vec<String>,
    /// The arrangement cycle of its shell.
    pub cycle: usize,
}

/// A network (3.2).
#[derive(Clone, Debug)]
pub struct Network {
    /// The noding (fine level), clipped when a clip was given.
    pub noded: Noded,
    /// The arrangement.
    pub arr: Arr,
    /// Junction points, by `(x, y)`.
    pub junctions: Vec<P>,
    /// Edges, in 2.5's order.
    pub edges: Vec<Edge>,
    /// The edge of each half-edge.
    pub edge_of_half: Vec<usize>,
    /// Areas, in 2.5's order.
    pub areas: Vec<Area>,
    /// Connected components.
    pub components: usize,
    /// Diagnostics (dead ends, no area).
    pub diags: Vec<GeomDiag>,
}

/// The source id the clip outline takes (identity.md 2.3).
pub const CLIP_SOURCE: &str = "#clip";

/// Builds a network over paths and ring outlines (3.2). `clip` is the index in `inputs` of a ring
/// whose outline clips the rest; it is renamed [`CLIP_SOURCE`].
pub fn network(
    mut inputs: Vec<Input>,
    clip: Option<usize>,
    naming: &[(String, P)],
    grid: PostGrid,
) -> Result<Network, GeomDiag> {
    let mut clip_region: Option<Region> = None;
    if let Some(ci) = clip {
        let r = Region::from_ring(inputs[ci].pts.clone());
        clip_region = Some(r);
        inputs[ci].source = CLIP_SOURCE.to_string();
    }
    network_clipped(inputs, clip_region.as_ref(), naming, grid)
}

/// As [`network`], clipped by a region given directly: segments outside it are dropped. Its
/// outline must be among the inputs for the areas to close along it.
pub fn network_clipped(
    inputs: Vec<Input>,
    clip_region: Option<&Region>,
    naming: &[(String, P)],
    grid: PostGrid,
) -> Result<Network, GeomDiag> {
    let mut noded = node(inputs)?;
    if let Some(cr) = clip_region {
        noded.segments.retain(|s| {
            let m = (
                i128::from(s.a.x) + i128::from(s.b.x),
                i128::from(s.a.y) + i128::from(s.b.y),
            );
            cr.side_scaled(m, 2) != PSide::Out
        });
        let mut vs: BTreeSet<P> = BTreeSet::new();
        for s in &noded.segments {
            vs.insert(s.a);
            vs.insert(s.b);
        }
        noded.vertices = vs.into_iter().collect();
    }
    build(noded, naming, grid)
}

fn source_set(seg: &Seg) -> Vec<usize> {
    let mut v: Vec<usize> = seg.prov.iter().map(|p| p.source).collect();
    v.dedup();
    v
}

/// Builds the network of an existing noding.
pub fn build(noded: Noded, naming: &[(String, P)], grid: PostGrid) -> Result<Network, GeomDiag> {
    let arr = Arr::build(&noded);
    let nv = arr.verts.len();
    // Ends of sources.
    let mut ends: BTreeSet<P> = BTreeSet::new();
    for inp in &noded.inputs {
        if !inp.closed {
            if let (Some(&a), Some(&b)) = (inp.pts.first(), inp.pts.last()) {
                ends.insert(a);
                ends.insert(b);
            }
        }
    }
    let mut is_j = vec![false; nv];
    for v in 0..nv {
        let out = &arr.out[v];
        if out.len() != 2 || ends.contains(&arr.verts[v]) {
            is_j[v] = true;
            continue;
        }
        let s0 = source_set(&noded.segments[out[0] / 2]);
        let s1 = source_set(&noded.segments[out[1] / 2]);
        if s0 != s1 {
            is_j[v] = true;
        }
    }
    // A ring of segments with no junction gets one at its least vertex.
    {
        let mut has = vec![false; arr.comps];
        let mut least = vec![usize::MAX; arr.comps];
        for v in 0..nv {
            let k = arr.comp_of_vert[v];
            has[k] |= is_j[v];
            if least[k] == usize::MAX {
                least[k] = v;
            }
        }
        for k in 0..arr.comps {
            if !has[k] && least[k] != usize::MAX {
                is_j[least[k]] = true;
            }
        }
    }
    let junctions: Vec<P> = (0..nv).filter(|&v| is_j[v]).map(|v| arr.verts[v]).collect();
    let jidx = |v: usize| junctions.binary_search(&arr.verts[v]).unwrap_or(0);
    // Edges.
    let nh = arr.from.len();
    let mut edge_of_half = vec![usize::MAX; nh];
    let mut raw_edges: Vec<Edge> = Vec::new();
    for v in 0..nv {
        if !is_j[v] {
            continue;
        }
        for &h0 in &arr.out[v] {
            if edge_of_half[h0] != usize::MAX {
                continue;
            }
            let mut half = vec![h0];
            let mut h = h0;
            loop {
                let w = arr.from[h ^ 1];
                if is_j[w] {
                    break;
                }
                // Degree 2: continue on the other outgoing half-edge.
                let out = &arr.out[w];
                let g = if out[0] == (h ^ 1) { out[1] } else { out[0] };
                half.push(g);
                h = g;
            }
            let e = raw_edges.len();
            for &g in &half {
                edge_of_half[g] = e;
                edge_of_half[g ^ 1] = e;
            }
            let a = arr.from[half[0]];
            let b = arr.from[half[half.len() - 1] ^ 1];
            let mut path: Vec<P> = half.iter().map(|&g| arr.org(g)).collect();
            path.push(arr.verts[b]);
            let (ja, jb) = (jidx(a), jidx(b));
            let flip = if ja != jb {
                jb < ja
            } else {
                // A loop: from its junction, the direction with the lesser second vertex.
                path.len() > 2 && path[path.len() - 2] < path[1]
            };
            if flip {
                path.reverse();
                half = half.iter().rev().map(|&g| g ^ 1).collect();
            }
            let mut sources: BTreeSet<usize> = BTreeSet::new();
            for &g in &half {
                for p in &noded.segments[g / 2].prov {
                    sources.insert(p.source);
                }
            }
            raw_edges.push(Edge {
                ends: (ja.min(jb), ja.max(jb)),
                path,
                half,
                sources: sources.into_iter().collect(),
            });
        }
    }
    // Order edges (2.5).
    let mut order: Vec<usize> = (0..raw_edges.len()).collect();
    order.sort_by(|&x, &y| {
        let (ex, ey) = (&raw_edges[x], &raw_edges[y]);
        ex.ends
            .cmp(&ey.ends)
            .then(ex.path.get(1).cmp(&ey.path.get(1)))
            .then(ex.path.cmp(&ey.path))
    });
    let mut renum = vec![0usize; raw_edges.len()];
    for (new, &old) in order.iter().enumerate() {
        renum[old] = new;
    }
    let mut slots: Vec<Option<Edge>> = raw_edges.into_iter().map(Some).collect();
    let edges: Vec<Edge> = order.iter().filter_map(|&o| slots[o].take()).collect();
    for e in edge_of_half.iter_mut() {
        if *e != usize::MAX {
            *e = renum[*e];
        }
    }
    // Areas.
    let holes_of = arr.holes_of();
    let mut areas: Vec<Area> = Vec::new();
    for c in 0..arr.cycles.len() {
        if !arr.bounded(c) {
            continue;
        }
        let in_face = |h: usize| arr.face_left(h) == Some(c);
        let shell_half: Vec<usize> = arr.cycles[c]
            .iter()
            .copied()
            .filter(|&h| !in_face(h ^ 1))
            .collect();
        let shell_pts: Vec<P> = shell_half.iter().map(|&h| arr.org(h)).collect();
        let _ = &shell_pts;
        let mut rings: Vec<Vec<P>> = kept_loops(&arr, &shell_half);
        let hole_cycles = holes_of.get(&c).cloned().unwrap_or_default();
        let mut hole_edges: Vec<Vec<usize>> = Vec::new();
        for &hc in &hole_cycles {
            let hh: Vec<usize> = arr.cycles[hc]
                .iter()
                .copied()
                .filter(|&h| !in_face(h ^ 1))
                .collect();
            for lp in kept_loops(&arr, &hh) {
                if pred::twice_area(&lp) != 0 {
                    rings.push(lp);
                }
            }
            let mut es = edges_of(&hh, &edge_of_half);
            if !es.is_empty() {
                es.dedup();
                hole_edges.push(es);
            }
        }
        // Shell loops: positive; loops of the shell's own pinches with negative area are holes.
        let mut shells: Vec<Vec<P>> = Vec::new();
        let mut holes: Vec<Vec<P>> = Vec::new();
        for r in rings {
            let a = pred::twice_area(&r);
            if a > 0 {
                shells.push(r);
            } else if a < 0 {
                holes.push(r);
            }
        }
        shells.sort_by_key(|s| std::cmp::Reverse(pred::twice_area(s)));
        let shell = match shells.into_iter().next() {
            Some(s) => s,
            None => continue,
        };
        let region = canonical(Region {
            pieces: vec![Piece { shell, holes }],
        });
        // Sides.
        let mut sides: BTreeSet<(usize, Side)> = BTreeSet::new();
        for &h in &shell_half {
            for p in &noded.segments[h / 2].prov {
                let along = (h % 2 == 0) != p.reversed;
                sides.insert((p.source, if along { Side::Left } else { Side::Right }));
            }
        }
        let mut sides: Vec<(usize, Side)> = sides.into_iter().collect();
        sides.sort_by(|x, y| {
            noded.sources[x.0]
                .as_bytes()
                .cmp(noded.sources[y.0].as_bytes())
                .then(x.1.cmp(&y.1))
        });
        // Shell edges from the start vertex.
        let start = region.pieces[0].shell[0];
        let pos = shell_half
            .iter()
            .position(|&h| arr.org(h) == start)
            .unwrap_or(0);
        let mut rot = shell_half.clone();
        rot.rotate_left(pos);
        let mut es = edges_of(&rot, &edge_of_half);
        while es.len() > 1 && es.first() == es.last() {
            es.pop();
        }
        // Spurs.
        let mut spurs: BTreeSet<usize> = BTreeSet::new();
        for &h in arr.cycles[c]
            .iter()
            .chain(hole_cycles.iter().flat_map(|&hc| arr.cycles[hc].iter()))
        {
            if in_face(h) && in_face(h ^ 1) {
                spurs.insert(edge_of_half[h]);
            }
        }
        let point = representative(&region, grid);
        let mut names: Vec<String> = naming
            .iter()
            .filter(|(_, p)| region.side(*p) == PSide::In)
            .map(|(n, _)| n.clone())
            .collect();
        names.sort();
        areas.push(Area {
            region,
            sides,
            edges: es,
            holes: hole_edges,
            spurs: spurs.into_iter().collect(),
            point,
            naming: names,
            cycle: c,
        });
    }
    areas.sort_by(|a, b| {
        let (sa, sb) = (&a.region.pieces[0].shell, &b.region.pieces[0].shell);
        crate::shape::ring_key(sa).cmp(&crate::shape::ring_key(sb))
    });
    let mut diags = Vec::new();
    // Guarantees: Euler at both levels, closure.
    let comps = arr.comps;
    let (v, e, a) = (nv as i64, noded.segments.len() as i64, areas.len() as i64);
    if v - e + a != comps as i64 {
        return Err(internal(
            "network",
            &format!("Euler at the fine level: {v} - {e} + {a} != {comps}"),
        ));
    }
    let (jv, je) = (junctions.len() as i64, edges.len() as i64);
    if jv - je + a != comps as i64 {
        return Err(internal(
            "network",
            &format!("Euler at the coarse level: {jv} - {je} + {a} != {comps}"),
        ));
    }
    let sum_areas: i128 = areas.iter().map(|x| x.region.twice_area()).sum();
    let mut sum_top: i128 = 0;
    for k in 0..comps {
        if arr.parent[k].is_none() && arr.outer[k] != usize::MAX {
            sum_top -= arr.cycle_area[arr.outer[k]];
        }
    }
    if sum_areas != sum_top && std::env::var("VIA_GEOM_DEBUG").is_ok() {
        for c in 0..arr.cycles.len() {
            eprintln!(
                "cycle {c} area {} comp {} pts {:?}",
                arr.cycle_area[c],
                arr.comp_of_vert[arr.from[arr.cycles[c][0]]],
                arr.cycle_points(c)
            );
        }
        eprintln!("parents {:?} outer {:?}", arr.parent, arr.outer);
        for a in &areas {
            eprintln!(
                "area cycle {} twice {} rings {}",
                a.cycle,
                a.region.twice_area(),
                a.region.ring_count()
            );
        }
    }
    if sum_areas != sum_top {
        return Err(internal(
            "network",
            &format!("closure: areas {sum_areas} != enclosed {sum_top}"),
        ));
    }
    let dead = (0..nv).filter(|&v| arr.out[v].len() == 1).count();
    if dead > 0 {
        diags.push(
            GeomDiag::new(Code::DeadEnd, format!("the network has {dead} dead ends"))
                .with("count", dead as i64),
        );
    }
    if areas.is_empty() {
        diags.push(GeomDiag::new(Code::NoArea, "the network encloses no area"));
    }
    Ok(Network {
        noded,
        arr,
        junctions,
        edges,
        edge_of_half,
        areas,
        components: comps,
        diags,
    })
}

fn edges_of(half: &[usize], edge_of_half: &[usize]) -> Vec<usize> {
    let mut es: Vec<usize> = Vec::new();
    for &h in half {
        let e = edge_of_half[h];
        if es.last() != Some(&e) {
            es.push(e);
        }
    }
    es
}

/// The representative point of a region (3.2): the grid post strictly inside it farthest from
/// its outline, ties to the lowest `y` then `x`; the spacing halved, anchored at the grid's
/// origin, while no post is inside; else the shell's start vertex. Candidates are the lattice
/// of the post grid extended past the extent (shapes beyond it are kept whole, 2.8).
///
/// Distances are compared exactly. A double estimate first discards candidates far from the
/// best, with a margin a million times wider than the estimate's error; every comparison that
/// decides the result is exact.
pub fn representative(region: &Region, grid: PostGrid) -> P {
    let Some((lo, hi)) = region.bbox() else {
        return P::new(0, 0);
    };
    let rings = region.rings();
    let segs: Vec<(P, P)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    let (x0, y0) = (grid.extent[0], grid.extent[1]);
    let mut s = grid.resolution.max(1);
    loop {
        let first = |o: i64, v: i64| o + (v - o).div_euclid(s) * s;
        let (fx, fy) = (first(x0, lo.x), first(y0, lo.y));
        let mut cands: Vec<(f64, P)> = Vec::new();
        // The greatest estimate so far, and the segment nearest the last candidate (tried first).
        let (mut most, mut hint) = (0.0f64, 0usize);
        let mut y = fy;
        while y <= hi.y {
            // The side of each post on the row from the edges whose span holds the row: the
            // others add nothing to the winding number (`pred::winding_of_edges`).
            let row: Vec<(P, P)> = segs
                .iter()
                .filter(|(a, b)| a.y.min(b.y) <= y && y <= a.y.max(b.y))
                .copied()
                .collect();
            let inside = |p: P| {
                pred::winding_of_edges((i128::from(p.x), i128::from(p.y)), 1, row.iter().copied())
                    .is_some_and(|w| w != 0)
            };
            let mut x = fx;
            while x <= hi.x {
                let p = P::new(x, y);
                if x > lo.x && x < hi.x && y > lo.y && y < hi.y && inside(p) {
                    // A candidate whose estimate falls below the floor of the greatest so far is
                    // below the final floor too (the floor only rises): it is left out unmeasured,
                    // and the candidates kept below are the same.
                    if let Some(d) = approx_dist2_above(p, &segs, &mut hint, most * (1.0 - 1e-9)) {
                        if d > most {
                            most = d;
                        }
                        cands.push((d, p));
                    }
                }
                x += s;
            }
            y += s;
        }
        if !cands.is_empty() {
            let best = cands
                .iter()
                .map(|c| c.0)
                .fold(0.0f64, |m, d| if d > m { d } else { m });
            let floor = best * (1.0 - 1e-9);
            let mut close: Vec<P> = cands.iter().filter(|c| c.0 >= floor).map(|c| c.1).collect();
            close.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
            let mut win = close[0];
            let mut wd = pred::dist2_rings(win, &rings);
            for &p in &close[1..] {
                let d = pred::dist2_rings(p, &rings);
                if let (Some(dn), Some(dw)) = (&d, &wd) {
                    if dn.cmp_exact(dw) == Ordering::Greater {
                        win = p;
                        wd = d;
                    }
                }
            }
            return win;
        }
        if s == 1 {
            return region.pieces[0].shell[0];
        }
        s /= 2;
    }
}

/// [`approx_dist2`] when it is at or above `cut`, else `None`, found by trying segment `hint`
/// first and stopping once the least so far is below `cut`; `hint` becomes the nearest segment.
fn approx_dist2_above(p: P, segs: &[(P, P)], hint: &mut usize, cut: f64) -> Option<f64> {
    let h = (*hint).min(segs.len().saturating_sub(1));
    let mut best = f64::INFINITY;
    for k in std::iter::once(h).chain((0..segs.len()).filter(|&k| k != h)) {
        let d = approx_seg2(p, segs[k]);
        if d < best {
            best = d;
            *hint = k;
            if best < cut {
                return None;
            }
        }
    }
    Some(best)
}

/// The double estimate of the squared distance from `p` to one segment.
fn approx_seg2(p: P, (a, b): (P, P)) -> f64 {
    let (ux, uy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let (wx, wy) = ((p.x - a.x) as f64, (p.y - a.y) as f64);
    let t = (ux * wx) + (uy * wy);
    let uu = (ux * ux) + (uy * uy);
    if t <= 0.0 {
        (wx * wx) + (wy * wy)
    } else if t >= uu {
        let (vx, vy) = ((p.x - b.x) as f64, (p.y - b.y) as f64);
        (vx * vx) + (vy * vy)
    } else {
        let c = (ux * wy) - (uy * wx);
        (c * c) / uu
    }
}

/// The network's areas, keyed by start vertex, for lookups.
pub fn area_by_start(n: &Network) -> BTreeMap<P, usize> {
    n.areas
        .iter()
        .enumerate()
        .map(|(i, a)| (a.region.pieces[0].shell[0], i))
        .collect()
}

#[cfg(test)]
mod representative_tests {
    use super::*;

    fn approx_dist2(p: P, segs: &[(P, P)]) -> f64 {
        let mut best = f64::INFINITY;
        for &(a, b) in segs {
            let (ux, uy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
            let (wx, wy) = ((p.x - a.x) as f64, (p.y - a.y) as f64);
            let t = (ux * wx) + (uy * wy);
            let uu = (ux * ux) + (uy * uy);
            let d = if t <= 0.0 {
                (wx * wx) + (wy * wy)
            } else if t >= uu {
                let (vx, vy) = ((p.x - b.x) as f64, (p.y - b.y) as f64);
                (vx * vx) + (vy * vy)
            } else {
                let c = (ux * wy) - (uy * wx);
                (c * c) / uu
            };
            if d < best {
                best = d;
            }
        }
        best
    }

    /// The search as written before the estimates were pruned: every candidate measured.
    fn representative_scan(region: &Region, grid: PostGrid) -> P {
        let (lo, hi) = region.bbox().unwrap();
        let rings = region.rings();
        let segs: Vec<(P, P)> = rings
            .iter()
            .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
            .collect();
        let (x0, y0) = (grid.extent[0], grid.extent[1]);
        let mut s = grid.resolution.max(1);
        loop {
            let first = |o: i64, v: i64| o + (v - o).div_euclid(s) * s;
            let mut cands: Vec<(f64, P)> = Vec::new();
            let mut y = first(y0, lo.y);
            while y <= hi.y {
                let mut x = first(x0, lo.x);
                while x <= hi.x {
                    let p = P::new(x, y);
                    if x > lo.x
                        && x < hi.x
                        && y > lo.y
                        && y < hi.y
                        && region.side(p) == pred::Side::In
                    {
                        cands.push((approx_dist2(p, &segs), p));
                    }
                    x += s;
                }
                y += s;
            }
            if !cands.is_empty() {
                let best = cands
                    .iter()
                    .map(|c| c.0)
                    .fold(0.0f64, |m, d| if d > m { d } else { m });
                let floor = best * (1.0 - 1e-9);
                let mut close: Vec<P> =
                    cands.iter().filter(|c| c.0 >= floor).map(|c| c.1).collect();
                close.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
                let mut win = close[0];
                let mut wd = pred::dist2_rings(win, &rings);
                for &p in &close[1..] {
                    let d = pred::dist2_rings(p, &rings);
                    if let (Some(dn), Some(dw)) = (&d, &wd) {
                        if dn.cmp_exact(dw) == Ordering::Greater {
                            win = p;
                            wd = d;
                        }
                    }
                }
                return win;
            }
            if s == 1 {
                return region.pieces[0].shell[0];
            }
            s /= 2;
        }
    }

    /// The pruned search names the post the full one does, on seeded stars, squares (whose
    /// centre rows tie) and thin slivers.
    #[test]
    fn pruned_equals_scan() {
        let mut seed = 5u64;
        let mut next = |n: i64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) % n as u64) as i64
        };
        let grid = PostGrid {
            extent: [0, 0, 256_000, 256_000],
            resolution: 1000,
        };
        let mut shapes: Vec<Vec<P>> = vec![
            vec![
                P::new(10_000, 10_000),
                P::new(50_000, 10_000),
                P::new(50_000, 50_000),
                P::new(10_000, 50_000),
            ],
            vec![
                P::new(0, 0),
                P::new(90_000, 0),
                P::new(90_000, 1_500),
                P::new(0, 1_200),
            ],
        ];
        for _ in 0..40 {
            let n = 5 + next(60) as usize;
            let (cx, cy) = (60_000 + next(100_000), 60_000 + next(100_000));
            shapes.push(
                (0..n)
                    .map(|i| {
                        // A point of a square's outline, pulled toward the centre.
                        let q = n as i64;
                        let side = (4 * i as i64) / q;
                        let u = (4 * i as i64 % q) * 96_000 / q - 48_000;
                        let (dx, dy) =
                            [(u, -48_000), (48_000, u), (-u, 48_000), (-48_000, -u)][side as usize];
                        let f = 20 + next(80);
                        P::new(cx + dx * f / 100, cy + dy * f / 100)
                    })
                    .collect(),
            );
        }
        for ring in shapes {
            let region = Region::from_ring(ring);
            assert_eq!(
                representative(&region, grid),
                representative_scan(&region, grid)
            );
        }
    }
}
