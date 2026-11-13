//! The planar arrangement of noded segments: half-edges, the cycles they bound, connected
//! components and how components nest. The network (3.2), booleans (3.3) and every operation
//! built on them read this one structure.

use crate::noding::Noded;
use crate::pred::{self, angle_cmp, dir, Side};
use crate::shape::{Piece, Region, P};
use std::collections::BTreeMap;

/// The arrangement.
#[derive(Clone, Debug)]
pub struct Arr {
    /// Vertices by `(x, y)`.
    pub verts: Vec<P>,
    /// Half-edge `2s` runs from segment `s`'s least end to its other end; `2s + 1` back.
    pub from: Vec<usize>,
    /// Outgoing half-edges of each vertex, counter-clockwise from the positive `x` axis.
    pub out: Vec<Vec<usize>>,
    /// The next half-edge along the cycle on each half-edge's left.
    pub next: Vec<usize>,
    /// The cycle of each half-edge.
    pub cycle_of: Vec<usize>,
    /// Each cycle's half-edges in order, starting at its least half-edge index.
    pub cycles: Vec<Vec<usize>>,
    /// Twice the signed area of each cycle.
    pub cycle_area: Vec<i128>,
    /// The component of each vertex.
    pub comp_of_vert: Vec<usize>,
    /// The number of components.
    pub comps: usize,
    /// The outer cycle of each component.
    pub outer: Vec<usize>,
    /// The bounded cycle each component lies directly inside, or `None` at the top level.
    pub parent: Vec<Option<usize>>,
}

impl Arr {
    /// The origin of a half-edge.
    pub fn org(&self, h: usize) -> P {
        self.verts[self.from[h]]
    }

    /// The destination of a half-edge.
    pub fn dst(&self, h: usize) -> P {
        self.verts[self.from[h ^ 1]]
    }

    /// The vertex index of a point.
    pub fn vid(&self, p: P) -> Option<usize> {
        self.verts.binary_search(&p).ok()
    }

    /// The points of a cycle, in order.
    pub fn cycle_points(&self, c: usize) -> Vec<P> {
        self.cycles[c].iter().map(|&h| self.org(h)).collect()
    }

    /// True for a bounded cycle (positive area).
    pub fn bounded(&self, c: usize) -> bool {
        self.cycle_area[c] > 0
    }

    /// Builds the arrangement of noded segments.
    pub fn build(n: &Noded) -> Arr {
        let verts = n.vertices.clone();
        let vid = |p: P| verts.binary_search(&p).unwrap_or(0);
        let ns = n.segments.len();
        let mut from = vec![0usize; 2 * ns];
        let mut out: Vec<Vec<usize>> = vec![Vec::new(); verts.len()];
        for (s, seg) in n.segments.iter().enumerate() {
            let (a, b) = (vid(seg.a), vid(seg.b));
            from[2 * s] = a;
            from[2 * s + 1] = b;
            out[a].push(2 * s);
            out[b].push(2 * s + 1);
        }
        for (v, list) in out.iter_mut().enumerate() {
            let o = verts[v];
            list.sort_by(|&h, &g| {
                let dh = dir(o, verts[from[h ^ 1]]);
                let dg = dir(o, verts[from[g ^ 1]]);
                angle_cmp(dh, dg)
            });
        }
        // Position of each half-edge in its origin's list.
        let mut pos = vec![0usize; 2 * ns];
        for list in &out {
            for (i, &h) in list.iter().enumerate() {
                pos[h] = i;
            }
        }
        let mut next = vec![0usize; 2 * ns];
        for h in 0..2 * ns {
            let t = h ^ 1;
            let v = from[t];
            let list = &out[v];
            let i = pos[t];
            next[h] = list[(i + list.len() - 1) % list.len()];
        }
        let mut cycle_of = vec![usize::MAX; 2 * ns];
        let mut cycles: Vec<Vec<usize>> = Vec::new();
        let mut cycle_area: Vec<i128> = Vec::new();
        for h0 in 0..2 * ns {
            if cycle_of[h0] != usize::MAX {
                continue;
            }
            let c = cycles.len();
            let mut cyc = Vec::new();
            let mut area: i128 = 0;
            let mut h = h0;
            loop {
                cycle_of[h] = c;
                cyc.push(h);
                let (p, q) = (verts[from[h]], verts[from[h ^ 1]]);
                area += i128::from(p.x) * i128::from(q.y) - i128::from(q.x) * i128::from(p.y);
                h = next[h];
                if h == h0 {
                    break;
                }
            }
            cycles.push(cyc);
            cycle_area.push(area);
        }
        // Components by union-find over segments.
        let mut uf: Vec<usize> = (0..verts.len()).collect();
        fn find(uf: &mut [usize], x: usize) -> usize {
            let mut r = x;
            while uf[r] != r {
                r = uf[r];
            }
            let mut y = x;
            while uf[y] != r {
                let nx = uf[y];
                uf[y] = r;
                y = nx;
            }
            r
        }
        for s in 0..ns {
            let (a, b) = (find(&mut uf, from[2 * s]), find(&mut uf, from[2 * s + 1]));
            if a != b {
                let (lo, hi) = (a.min(b), a.max(b));
                uf[hi] = lo;
            }
        }
        let mut comp_id: BTreeMap<usize, usize> = BTreeMap::new();
        let mut comp_of_vert = vec![0usize; verts.len()];
        for v in 0..verts.len() {
            let r = find(&mut uf, v);
            let next_id = comp_id.len();
            let id = *comp_id.entry(r).or_insert(next_id);
            comp_of_vert[v] = id;
        }
        let comps = comp_id.len();
        let mut outer = vec![usize::MAX; comps];
        for (c, cyc) in cycles.iter().enumerate() {
            if cycle_area[c] <= 0 {
                let k = comp_of_vert[from[cyc[0]]];
                // One outer cycle per component; keep the least area if ever two.
                if outer[k] == usize::MAX || cycle_area[c] < cycle_area[outer[k]] {
                    outer[k] = c;
                }
            }
        }
        let mut arr = Arr {
            verts,
            from,
            out,
            next,
            cycle_of,
            cycles,
            cycle_area,
            comp_of_vert,
            comps,
            outer,
            parent: vec![None; comps],
        };
        arr.nest();
        arr
    }

    /// Finds, for each component, the smallest bounded cycle of another component that holds it.
    fn nest(&mut self) {
        if self.comps < 2 {
            return;
        }
        // Least vertex of each component.
        let mut rep = vec![usize::MAX; self.comps];
        for v in 0..self.verts.len() {
            let k = self.comp_of_vert[v];
            if rep[k] == usize::MAX {
                rep[k] = v;
            }
        }
        let bounded: Vec<usize> = (0..self.cycles.len())
            .filter(|&c| self.bounded(c))
            .collect();
        let pts: Vec<Vec<P>> = bounded.iter().map(|&c| self.cycle_points(c)).collect();
        let boxes: Vec<(P, P)> = pts
            .iter()
            .map(|r| crate::shape::bbox(r.iter().copied()).unwrap_or((P::new(0, 0), P::new(0, 0))))
            .collect();
        for k in 0..self.comps {
            let p = self.verts[rep[k]];
            let mut best: Option<usize> = None;
            for (i, &c) in bounded.iter().enumerate() {
                let ck = self.comp_of_vert[self.from[self.cycles[c][0]]];
                if ck == k {
                    continue;
                }
                let (lo, hi) = boxes[i];
                if p.x < lo.x || p.x > hi.x || p.y < lo.y || p.y > hi.y {
                    continue;
                }
                if matches!(pred::winding(p, &pts[i]), Some(w) if w != 0) {
                    let better = match best {
                        None => true,
                        Some(b) => self.cycle_area[c] < self.cycle_area[b],
                    };
                    if better {
                        best = Some(c);
                    }
                }
            }
            self.parent[k] = best;
        }
    }

    /// The order to visit components so that each comes after the one holding it.
    pub fn comp_order(&self) -> Vec<usize> {
        let mut depth = vec![0usize; self.comps];
        for k in 0..self.comps {
            let mut d = 0;
            let mut c = k;
            while let Some(pc) = self.parent[c] {
                d += 1;
                c = self.comp_of_vert[self.from[self.cycles[pc][0]]];
                if d > self.comps {
                    break;
                }
            }
            depth[k] = d;
        }
        let mut order: Vec<usize> = (0..self.comps).collect();
        order.sort_by_key(|&k| (depth[k], k));
        order
    }

    /// The holes of each bounded cycle: the outer cycles of the components directly inside it.
    pub fn holes_of(&self) -> BTreeMap<usize, Vec<usize>> {
        let mut m: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for k in 0..self.comps {
            if let Some(p) = self.parent[k] {
                m.entry(p).or_default().push(self.outer[k]);
            }
        }
        m
    }

    /// The bounded cycle (face) on the left of each half-edge, after nesting: a half-edge on a
    /// component's outer cycle has on its left the face that holds the component (`None` for
    /// the unbounded face).
    pub fn face_left(&self, h: usize) -> Option<usize> {
        let c = self.cycle_of[h];
        if self.bounded(c) {
            Some(c)
        } else {
            let k = self.comp_of_vert[self.from[h]];
            self.parent[k]
        }
    }
}

/// The loops a cycle's half-edges make once its spurs and bridges (edges walked both ways) are
/// taken out. `kept` is the rest, in cycle order. Each kept half-edge continues with the first
/// kept half-edge after it, in cycle order, that starts where it ends; so a hole joined to the
/// shell by a bridge comes out as a loop of its own. Loops are split at repeated vertices.
pub fn kept_loops(arr: &Arr, kept: &[usize]) -> Vec<Vec<P>> {
    let n = kept.len();
    let mut by_org: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, &h) in kept.iter().enumerate() {
        by_org.entry(arr.from[h]).or_default().push(i);
    }
    let next_of = |i: usize| -> usize {
        let v = arr.from[kept[i] ^ 1];
        let list = &by_org[&v];
        let k = list.partition_point(|&j| j <= i);
        if k < list.len() {
            list[k]
        } else {
            list[0]
        }
    };
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    for s in 0..n {
        if seen[s] {
            continue;
        }
        let mut pts = Vec::new();
        let mut i = s;
        while !seen[i] {
            seen[i] = true;
            pts.push(arr.org(kept[i]));
            if !by_org.contains_key(&arr.from[kept[i] ^ 1]) {
                break;
            }
            i = next_of(i);
        }
        out.extend(split_loops(&pts));
    }
    out
}

/// Splits a closed sequence of points at repeated vertices into simple loops.
pub fn split_loops(pts: &[P]) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    let mut stack: Vec<P> = Vec::new();
    for &p in pts {
        if let Some(i) = stack.iter().position(|&q| q == p) {
            let lp: Vec<P> = stack.drain(i..).collect();
            if lp.len() >= 3 {
                out.push(lp);
            }
        }
        stack.push(p);
    }
    if stack.len() >= 3 {
        out.push(stack);
    }
    out
}

/// Builds a region from rings: positive ones are shells, negative ones holes, each hole in the
/// smallest shell that holds it. Zero-area rings are dropped.
pub fn rings_to_region(rings: Vec<Vec<P>>) -> Region {
    let mut shells: Vec<Vec<P>> = Vec::new();
    let mut holes: Vec<Vec<P>> = Vec::new();
    for r in rings {
        for lp in split_loops(&r) {
            let a = pred::twice_area(&lp);
            if a > 0 {
                shells.push(lp);
            } else if a < 0 {
                holes.push(lp);
            }
        }
    }
    let areas: Vec<i128> = shells.iter().map(|s| pred::twice_area(s)).collect();
    let mut pieces: Vec<Piece> = shells
        .iter()
        .map(|s| Piece {
            shell: s.clone(),
            holes: Vec::new(),
        })
        .collect();
    for h in holes {
        let mut best: Option<usize> = None;
        for (i, s) in shells.iter().enumerate() {
            if ring_holds(s, &h) && best.is_none_or(|b| areas[i] < areas[b]) {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            pieces[i].holes.push(h);
        }
    }
    crate::shape::canonical(Region { pieces })
}

/// True when ring `inner` lies inside ring `outer` (touching at vertices allowed).
pub fn ring_holds(outer: &[P], inner: &[P]) -> bool {
    let reg = Region {
        pieces: vec![Piece {
            shell: outer.to_vec(),
            holes: Vec::new(),
        }],
    };
    for &p in inner {
        match reg.side(p) {
            Side::In => return true,
            Side::Out => return false,
            Side::On => {}
        }
    }
    let n = inner.len();
    for i in 0..n {
        let (a, b) = (inner[i], inner[(i + 1) % n]);
        let m = (
            i128::from(a.x) + i128::from(b.x),
            i128::from(a.y) + i128::from(b.y),
        );
        match reg.side_scaled(m, 2) {
            Side::In => return true,
            Side::Out => return false,
            Side::On => {}
        }
    }
    false
}
