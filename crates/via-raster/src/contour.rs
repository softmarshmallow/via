//! Mask to regions (`geometry.md` 3.7): the region where a formula's values at the posts are above
//! 0, keeping every piece. This is what `where` lowers to.
//!
//! The split of 3.7: this crate evaluates nothing but turns the posts into rounded rings, sorted
//! into pieces with their holes; the kernel (`via-geom`) nodes them and rebuilds them as a union,
//! through the [`Settle`] trait, so the result is valid and settled.
//!
//! Mechanism: marching squares at level 0 (Lorensen and Cline 1987, in two dimensions), as the
//! union of each grid cell's "above" polygon. Each grid edge's crossing is computed once from its
//! two posts and rounded to the lattice, so the polygons of neighbouring cells share it exactly
//! and their common sides cancel; what is left is the outline, closed along the window's edge.

use crate::cover::{inside, twice_area, Pt};
use crate::error::RasterError;
use crate::raster::Raster;
use crate::window::post_mm;
use std::collections::BTreeMap;
use via_num::Grid;

/// One piece: a shell ring, counter-clockwise, and its holes, clockwise (`geometry.md` 2.2). Each
/// ring starts at its least vertex, by `x` then `y`; the closing edge is implicit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub shell: Vec<Pt>,
    pub holes: Vec<Vec<Pt>>,
}

/// The rings of a mask, sorted into pieces, with what 3.7 reports about them.
#[derive(Clone, Debug, PartialEq)]
pub struct Masked {
    /// Every piece, by its shell's start vertex, then its second vertex.
    pub pieces: Vec<Piece>,
    /// The position of each piece smaller than one grid cell (`geom.mask_tiny`, info, one record
    /// per masked region with the count and these positions).
    pub tiny: Vec<Pt>,
    /// The position of each ring with no area after rounding (`geom.mask_dropped`, warning).
    pub dropped: Vec<Pt>,
    /// No post above 0: the least and greatest value over the posts (`geom.mask_empty`, warning).
    pub empty: Option<(f64, f64)>,
}

/// The kernel's half of 3.7: node the rounded rings and rebuild them as a union (3.3), so the
/// region is valid, settled and canonical. `via-geom` implements it.
pub trait Settle {
    /// The kernel's region.
    type Region;
    /// The kernel's error.
    type Error;
    /// The settled region from the pieces, every piece kept.
    fn settle(&self, pieces: &[Piece]) -> Result<Self::Region, Self::Error>;
}

/// A fault of [`mask_to_region`]: this crate's, or the kernel's.
#[derive(Clone, Debug, PartialEq)]
pub enum MaskFault<E> {
    Raster(RasterError),
    Settle(E),
}

/// The whole of 3.7: the rings of [`mask_rings`], settled by the kernel. The report comes back
/// beside the region, for the caller's diagnostics.
pub fn mask_to_region<S: Settle>(
    grid: &Grid,
    values: &Raster<f64>,
    kernel: &S,
) -> Result<(S::Region, Masked), MaskFault<S::Error>> {
    let m = mask_rings(grid, values).map_err(MaskFault::Raster)?;
    let r = kernel.settle(&m.pieces).map_err(MaskFault::Settle)?;
    Ok((r, m))
}

/// The crossing on the grid edge from post `p0` (first in post order, value `v0`) to the next post
/// along one axis (value `v1`): `f = v0 / (v0 - v1)`, at `p0 + f * r`, doubles in that order, then
/// rounded to the lattice.
fn crossing(p0: i64, v0: f64, v1: f64, r: i64) -> Result<i64, RasterError> {
    let f = v0 / (v0 - v1);
    via_num::round_to_lattice((p0 as f64) + f * (r as f64)).map_err(|_| RasterError::LatticeRange)
}

/// The angle class of `e` measured counter-clockwise from `d`: 0 along `d`, 1 in `(0, pi)`, 2 at
/// `pi`, 3 in `(pi, 2 pi)`. Exact.
fn half(d: (i128, i128), e: (i128, i128)) -> u8 {
    let cross = d.0 * e.1 - d.1 * e.0;
    let dot = d.0 * e.0 + d.1 * e.1;
    match (cross.signum(), dot > 0) {
        (0, true) => 0,
        (1, _) => 1,
        (0, false) => 2,
        _ => 3,
    }
}

/// Whether `e1` lies at a greater counter-clockwise angle from `d` than `e2`, exactly.
fn later(d: (i128, i128), e1: (i128, i128), e2: (i128, i128)) -> bool {
    let (h1, h2) = (half(d, e1), half(d, e2));
    if h1 != h2 {
        return h1 > h2;
    }
    // Same class: e1 is later when e2 turns left to reach it.
    e2.0 * e1.1 - e2.1 * e1.0 > 0
}

fn vec(a: Pt, b: Pt) -> (i128, i128) {
    (
        i128::from(b.0) - i128::from(a.0),
        i128::from(b.1) - i128::from(a.1),
    )
}

/// Removes consecutive repeats and every vertex in line with its neighbours (cyclically), until
/// none is left; then rotates the ring to start at its least vertex.
fn canonical(mut ring: Vec<Pt>) -> Vec<Pt> {
    loop {
        let n = ring.len();
        if n < 3 {
            ring.dedup();
            return ring;
        }
        let mut keep = Vec::with_capacity(n);
        let mut changed = false;
        for k in 0..n {
            let prev = ring[(k + n - 1) % n];
            let p = ring[k];
            let next = ring[(k + 1) % n];
            let (a, b) = (vec(prev, p), vec(p, next));
            if p == prev || a.0 * b.1 - a.1 * b.0 == 0 {
                changed = true;
                // Drop one vertex per pass so neighbours are judged against what remains.
                keep.extend_from_slice(&ring[k + 1..]);
                break;
            }
            keep.push(p);
        }
        ring = keep;
        if !changed {
            break;
        }
    }
    if let Some(start) = (0..ring.len()).min_by_key(|&k| ring[k]) {
        ring.rotate_left(start);
    }
    ring
}

/// Splits a closed walk at every vertex it visits twice, so each ring visits a vertex once (a
/// pinch is two rings sharing a vertex, `geometry.md` 2.2).
fn split_walk(walk: Vec<Pt>) -> Vec<Vec<Pt>> {
    let mut out = Vec::new();
    let mut stack: Vec<Pt> = Vec::new();
    let mut at: BTreeMap<Pt, usize> = BTreeMap::new();
    for p in walk {
        if let Some(&k) = at.get(&p) {
            let lobe: Vec<Pt> = stack.drain(k..).collect();
            for q in &lobe {
                at.remove(q);
            }
            out.push(lobe);
        }
        at.insert(p, stack.len());
        stack.push(p);
    }
    if !stack.is_empty() {
        out.push(stack);
    }
    out
}

/// The rounded rings of the region where `values` is above 0, over the values' window, sorted
/// into pieces with their holes (3.7 up to the kernel's settling). A post exactly at 0 is not
/// above. A saddle cell is settled by the mean of its four posts, summed in post order: above 0
/// joins the high corners. A piece reaching the window's edge is closed along it, so masking a
/// window gives the whole's rings clipped to the window.
pub fn mask_rings(grid: &Grid, values: &Raster<f64>) -> Result<Masked, RasterError> {
    let w = values.window();
    let r = grid.resolution;
    let v = |i: u64, j: u64| values.get(i, j).unwrap_or(0.0);
    if let Some(k) = values.values().iter().position(|x| !x.is_finite()) {
        let (i, j) = w.posts().nth(k).unwrap_or((w.i0, w.j0));
        return Err(RasterError::NotFinite {
            at: post_mm(grid, i, j),
            bits: values.values()[k].to_bits(),
        });
    }
    if !values.values().iter().any(|&x| x > 0.0) {
        let lo = via_num::minimum(values.values()).map_err(|e| RasterError::from_num(e, (0, 0)))?;
        let hi = via_num::maximum(values.values()).map_err(|e| RasterError::from_num(e, (0, 0)))?;
        return Ok(Masked {
            pieces: Vec::new(),
            tiny: Vec::new(),
            dropped: Vec::new(),
            empty: Some((lo, hi)),
        });
    }

    // Directed edges, with cancellation of each against its reverse.
    let mut edges: BTreeMap<(Pt, Pt), i64> = BTreeMap::new();
    let mut add = |a: Pt, b: Pt| {
        if a == b {
            return;
        }
        if let Some(c) = edges.get_mut(&(b, a)) {
            *c -= 1;
            if *c == 0 {
                edges.remove(&(b, a));
            }
            return;
        }
        *edges.entry((a, b)).or_insert(0) += 1;
    };

    // A grid with one column or one row has no cells: a post above 0 there has no area.
    let one_line = w.width() < 2 || w.height() < 2;
    if !one_line {
        for j in w.j0..w.j1 - 1 {
            for i in w.i0..w.i1 - 1 {
                let (v00, v10, v01, v11) = (v(i, j), v(i + 1, j), v(i, j + 1), v(i + 1, j + 1));
                let (a, b, c, d) = (v00 > 0.0, v10 > 0.0, v11 > 0.0, v01 > 0.0);
                if !(a || b || c || d) {
                    continue;
                }
                let p00 = post_mm(grid, i, j);
                let p10 = (p00.0 + r, p00.1);
                let p11 = (p00.0 + r, p00.1 + r);
                let p01 = (p00.0, p00.1 + r);
                let xb = || crossing(p00.0, v00, v10, r).map(|x| (x, p00.1));
                let xr = || crossing(p10.1, v10, v11, r).map(|y| (p10.0, y));
                let xt = || crossing(p01.0, v01, v11, r).map(|x| (x, p01.1));
                let xl = || crossing(p00.1, v00, v01, r).map(|y| (p00.0, y));
                let saddle = (a && c && !b && !d) || (b && d && !a && !c);
                let polys: Vec<Vec<Pt>> = if saddle && (v00 + v10 + v01 + v11) / 4.0 <= 0.0 {
                    if a {
                        vec![vec![p00, xb()?, xl()?], vec![p11, xt()?, xr()?]]
                    } else {
                        vec![vec![p10, xr()?, xb()?], vec![p01, xl()?, xt()?]]
                    }
                } else {
                    // Walk the cell counter-clockwise: corners above, and crossings between.
                    let mut poly = Vec::with_capacity(8);
                    if a {
                        poly.push(p00);
                    }
                    if a != b {
                        poly.push(xb()?);
                    }
                    if b {
                        poly.push(p10);
                    }
                    if b != c {
                        poly.push(xr()?);
                    }
                    if c {
                        poly.push(p11);
                    }
                    if c != d {
                        poly.push(xt()?);
                    }
                    if d {
                        poly.push(p01);
                    }
                    if d != a {
                        poly.push(xl()?);
                    }
                    vec![poly]
                };
                for poly in polys {
                    let n = poly.len();
                    for k in 0..n {
                        add(poly[k], poly[(k + 1) % n]);
                    }
                }
            }
        }
    }

    // Chain the outline into rings. At a vertex several rings touch, an incoming edge is paired
    // with the outgoing edge first reached turning clockwise from the way back, which keeps the
    // region on the left in the smallest wedge, so touching pieces stay apart.
    let mut out: BTreeMap<Pt, Vec<Pt>> = BTreeMap::new();
    for (&(a, b), &count) in &edges {
        for _ in 0..count.max(0) {
            out.entry(a).or_default().push(b);
        }
    }
    let total: usize = out.values().map(Vec::len).sum();
    // Takes the outgoing edge `k` of `v`, dropping the vertex once it has none left.
    let take = |out: &mut BTreeMap<Pt, Vec<Pt>>, v: Pt, k: usize| -> Option<Pt> {
        let targets = out.get_mut(&v)?;
        let t = (k < targets.len()).then(|| targets.remove(k));
        if targets.is_empty() {
            out.remove(&v);
        }
        t
    };
    let mut rings: Vec<Vec<Pt>> = Vec::new();
    while let Some(&start) = out.keys().next() {
        let Some(mut next) = take(&mut out, start, 0) else {
            break;
        };
        let mut walk = vec![start];
        let mut prev = start;
        let mut guard = total;
        while next != start && guard > 0 {
            guard -= 1;
            walk.push(next);
            let back = vec(next, prev);
            let Some(targets) = out.get(&next) else { break };
            let mut best = 0;
            for k in 1..targets.len() {
                if later(back, vec(next, targets[k]), vec(next, targets[best])) {
                    best = k;
                }
            }
            prev = next;
            match take(&mut out, next, best) {
                Some(t) => next = t,
                None => break,
            }
        }
        if next != start {
            return Err(RasterError::Internal {
                reason: format!("mask outline does not close at ({}, {})", start.0, start.1),
            });
        }
        rings.extend(split_walk(walk));
    }

    // Canonical rings; zero area dropped; shells and holes by orientation.
    let mut dropped = Vec::new();
    let mut shells: Vec<(Vec<Pt>, i128)> = Vec::new();
    let mut holes: Vec<(Vec<Pt>, i128)> = Vec::new();
    for ring in rings {
        let first = ring[0];
        let ring = canonical(ring);
        let a2 = if ring.len() >= 3 {
            twice_area(&ring)
        } else {
            0
        };
        match a2.signum() {
            1 => shells.push((ring, a2)),
            -1 => holes.push((ring, a2)),
            _ => dropped.push(first),
        }
    }
    let mut pieces: Vec<(Piece, i128)> = shells
        .iter()
        .map(|(s, a)| {
            (
                Piece {
                    shell: s.clone(),
                    holes: Vec::new(),
                },
                *a,
            )
        })
        .collect();
    for (hole, ha) in holes {
        // The smallest shell holding a vertex of the hole that is not on that shell's outline.
        let mut best: Option<usize> = None;
        for (k, (s, a)) in shells.iter().enumerate() {
            let one = [s.clone()];
            let probe = hole.iter().find(|&&p| !on_ring(s, p));
            let holds = match probe {
                Some(&p) => inside(&one, p),
                None => true,
            };
            if holds && best.is_none_or(|b| *a < shells[b].1) {
                best = Some(k);
            }
        }
        match best {
            Some(k) => {
                pieces[k].0.holes.push(hole);
                pieces[k].1 += ha;
            }
            None => dropped.push(hole[0]),
        }
    }
    let cell2 = 2 * i128::from(r) * i128::from(r);
    let mut tiny = Vec::new();
    let mut pieces: Vec<Piece> = pieces
        .into_iter()
        .map(|(mut p, net)| {
            if net < cell2 {
                tiny.push(p.shell[0]);
            }
            p.holes.sort_by_key(|h| (h[0], h[1]));
            p
        })
        .collect();
    pieces.sort_by_key(|p| (p.shell[0], p.shell[1]));
    tiny.sort();
    dropped.sort();
    Ok(Masked {
        pieces,
        tiny,
        dropped,
        empty: None,
    })
}

fn on_ring(ring: &[Pt], p: Pt) -> bool {
    let n = ring.len();
    (0..n).any(|k| crate::cover::on_segment(ring[k], ring[(k + 1) % n], p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cover::region_cover;
    use crate::window::Window;

    fn rings_of(m: &Masked) -> Vec<Vec<Pt>> {
        m.pieces
            .iter()
            .flat_map(|p| std::iter::once(p.shell.clone()).chain(p.holes.iter().cloned()))
            .collect()
    }

    fn field(g: &Grid, w: Window, f: impl Fn(f64, f64) -> f64) -> Raster<f64> {
        Raster::from_fn(w, |i, j| {
            let (x, y) = post_mm(g, i, j);
            Ok::<_, ()>(f(via_num::metres(x), via_num::metres(y)))
        })
        .unwrap()
    }

    #[test]
    fn a_disc_is_one_piece_near_its_area() {
        let g = Grid::new([0, 0, 40_000, 40_000], 500).unwrap();
        let v = field(&g, Window::whole(&g), |x, y| {
            100.0 - ((x - 20.0) * (x - 20.0) + (y - 20.0) * (y - 20.0))
        });
        let m = mask_rings(&g, &v).unwrap();
        assert_eq!(m.pieces.len(), 1);
        assert!(m.pieces[0].holes.is_empty());
        let a = twice_area(&m.pieces[0].shell) as f64 / 2.0 / 1e6;
        assert!((a - via_num::PI * 100.0).abs() < 1.0, "{a}");
        assert!(m.tiny.is_empty() && m.dropped.is_empty() && m.empty.is_none());
    }

    #[test]
    fn every_piece_and_hole_is_kept() {
        // An annulus, a small blob far away, and a sliver at the edge.
        let g = Grid::new([0, 0, 40_000, 30_000], 500).unwrap();
        let v = field(&g, Window::whole(&g), |x, y| {
            let r2 = (x - 15.0) * (x - 15.0) + (y - 15.0) * (y - 15.0);
            let ring = if (16.0..64.0).contains(&r2) {
                1.0
            } else {
                -1.0
            };
            let blob = 0.09 - ((x - 33.0) * (x - 33.0) + (y - 25.0) * (y - 25.0));
            let edge = if x < 0.4 { 1.0 } else { -1.0 };
            via_num::max(via_num::max(ring, blob), edge)
        });
        let m = mask_rings(&g, &v).unwrap();
        assert_eq!(m.pieces.len(), 3);
        let holes: usize = m.pieces.iter().map(|p| p.holes.len()).sum();
        assert_eq!(holes, 1);
        // The blob is smaller than a grid cell: kept, and reported.
        assert_eq!(m.tiny.len(), 1);
        for p in &m.pieces {
            assert!(twice_area(&p.shell) > 0);
            for h in &p.holes {
                assert!(twice_area(h) < 0);
            }
        }
    }

    #[test]
    fn every_post_above_is_inside_and_window_equals_cut() {
        let g = Grid::new([0, 0, 30_000, 20_000], 500).unwrap();
        let all = Window::whole(&g);
        let v = field(&g, all, |x, y| {
            via_num::sin(x * 0.7) * via_num::cos(y * 0.5) + 0.2 * via_num::sin(x * y * 0.05)
        });
        let whole = mask_rings(&g, &v).unwrap();
        let cover = region_cover(&g, all, &rings_of(&whole));
        for (i, j) in all.posts() {
            if v.get(i, j).unwrap() > 0.0 {
                assert_eq!(cover.get(i, j), Some(1), "post ({i}, {j})");
            }
        }
        for w in [
            Window::new(0, 0, 2, 2).unwrap(),
            Window::new(10, 7, 33, 29).unwrap(),
            Window::new(40, 0, 61, 41).unwrap(),
            Window::new(3, 30, 58, 41).unwrap(),
        ] {
            let part = mask_rings(&g, &v.cut(w).unwrap()).unwrap();
            let c = region_cover(&g, w, &rings_of(&part));
            assert_eq!(c, cover.cut(w).unwrap(), "window {}", w.key());
        }
    }

    #[test]
    fn saddles_follow_the_mean() {
        let g = Grid::new([0, 0, 1_000, 1_000], 1_000).unwrap();
        let w = Window::whole(&g);
        let joined = Raster::from_values(w, vec![1.0, -0.5, -0.5, 1.0]).unwrap();
        assert_eq!(mask_rings(&g, &joined).unwrap().pieces.len(), 1);
        let apart = Raster::from_values(w, vec![1.0, -1.5, -1.5, 1.0]).unwrap();
        assert_eq!(mask_rings(&g, &apart).unwrap().pieces.len(), 2);
    }

    #[test]
    fn empty_and_full() {
        let g = Grid::new([0, 0, 4_000, 3_000], 1_000).unwrap();
        let w = Window::whole(&g);
        let none = Raster::from_fn(w, |i, _| Ok::<_, ()>(-(i as f64))).unwrap();
        let m = mask_rings(&g, &none).unwrap();
        assert!(m.pieces.is_empty());
        assert_eq!(m.empty, Some((-4.0, 0.0)));
        let full = Raster::filled(w, 1.0);
        let m = mask_rings(&g, &full).unwrap();
        assert_eq!(
            m.pieces,
            vec![Piece {
                shell: vec![(0, 0), (4_000, 0), (4_000, 3_000), (0, 3_000)],
                holes: vec![]
            }]
        );
    }

    #[test]
    fn a_saddle_between_blocks_keeps_them_apart() {
        let g = Grid::new([0, 0, 3_000, 3_000], 1_000).unwrap();
        let w = Window::whole(&g);
        let mut vals = vec![-1.0; 16];
        for (i, j) in [
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (2, 2),
            (3, 2),
            (2, 3),
            (3, 3),
        ] {
            vals[j * 4 + i] = 1.0;
        }
        // The cell between them is a saddle whose mean is 0, so the two blocks stay apart.
        let m = mask_rings(&g, &Raster::from_values(w, vals).unwrap()).unwrap();
        assert_eq!(m.pieces.len(), 2);
    }

    struct Count;
    impl Settle for Count {
        type Region = usize;
        type Error = ();
        fn settle(&self, pieces: &[Piece]) -> Result<usize, ()> {
            Ok(pieces.len())
        }
    }

    #[test]
    fn the_kernel_settles_through_the_trait() {
        let g = Grid::new([0, 0, 4_000, 3_000], 1_000).unwrap();
        let full = Raster::filled(Window::whole(&g), 1.0);
        assert_eq!(mask_to_region(&g, &full, &Count).unwrap().0, 1);
    }
}
