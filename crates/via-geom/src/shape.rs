//! Points, paths, rings and regions, their canonical form, and reading written shapes
//! (`spec/geometry.md` 2.2, 2.3, 2.5).

use crate::diag::{Code, GeomDiag};
use crate::pred::{self, orient, twice_area};
use via_num::LATTICE_MAX;

/// A lattice point in millimetres. Ordered by `x`, then `y` (2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct P {
    /// East, millimetres.
    pub x: i64,
    /// North, millimetres.
    pub y: i64,
}

impl P {
    /// The point `(x, y)`.
    pub const fn new(x: i64, y: i64) -> P {
        P { x, y }
    }

    /// True when both coordinates lie in the lattice range (1, Range).
    pub fn in_range(self) -> bool {
        self.x.abs() <= LATTICE_MAX && self.y.abs() <= LATTICE_MAX
    }
}

/// One piece of a region: a counter-clockwise shell and clockwise holes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Piece {
    /// The outer ring, counter-clockwise, closing point not repeated.
    pub shell: Vec<P>,
    /// Hole rings, clockwise.
    pub holes: Vec<Vec<P>>,
}

impl Piece {
    /// Twice the area of the piece: shell less holes.
    pub fn twice_area(&self) -> i128 {
        twice_area(&self.shell) + self.holes.iter().map(|h| twice_area(h)).sum::<i128>()
    }

    /// Every ring, shell first.
    pub fn rings(&self) -> Vec<&[P]> {
        let mut v: Vec<&[P]> = vec![&self.shell];
        v.extend(self.holes.iter().map(Vec::as_slice));
        v
    }
}

/// A region: a set of pieces (2.2). An empty region is a legal value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Region {
    /// The pieces, in canonical order.
    pub pieces: Vec<Piece>,
}

impl Region {
    /// A region of one ring, read counter-clockwise.
    pub fn from_ring(ring: Vec<P>) -> Region {
        let mut r = ring;
        if twice_area(&r) < 0 {
            r.reverse();
        }
        Region {
            pieces: vec![Piece {
                shell: r,
                holes: Vec::new(),
            }],
        }
    }

    /// Twice the area, exact.
    pub fn twice_area(&self) -> i128 {
        self.pieces.iter().map(Piece::twice_area).sum()
    }

    /// Every ring of every piece.
    pub fn rings(&self) -> Vec<&[P]> {
        self.pieces.iter().flat_map(Piece::rings).collect()
    }

    /// Number of rings.
    pub fn ring_count(&self) -> usize {
        self.pieces.iter().map(|p| 1 + p.holes.len()).sum()
    }

    /// True when the region has no piece.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// Where `p` lies: inside, on the outline, or outside. Holes are outside.
    pub fn side(&self, p: P) -> pred::Side {
        self.side_scaled((i128::from(p.x), i128::from(p.y)), 1)
    }

    /// As [`Region::side`], for a point scaled by `k` against the region.
    pub fn side_scaled(&self, p: (i128, i128), k: i128) -> pred::Side {
        let mut w = 0;
        for ring in self.rings() {
            match pred::winding_scaled(p, k, ring) {
                None => return pred::Side::On,
                Some(n) => w += n,
            }
        }
        if w != 0 {
            pred::Side::In
        } else {
            pred::Side::Out
        }
    }

    /// The least and greatest corner of the bounding box, or `None` when empty.
    pub fn bbox(&self) -> Option<(P, P)> {
        bbox(self.rings().into_iter().flatten().copied())
    }
}

/// The bounding box of a set of points.
pub fn bbox(points: impl Iterator<Item = P>) -> Option<(P, P)> {
    let mut b: Option<(P, P)> = None;
    for p in points {
        b = Some(match b {
            None => (p, p),
            Some((lo, hi)) => (
                P::new(lo.x.min(p.x), lo.y.min(p.y)),
                P::new(hi.x.max(p.x), hi.y.max(p.y)),
            ),
        });
    }
    b
}

/// Removes consecutive repeats and vertices on the straight segment between their neighbours,
/// keeping the vertices in `keep`. Treats the ring as closed.
pub fn simplify_ring(ring: &[P], keep: &dyn Fn(P) -> bool) -> Vec<P> {
    let mut v: Vec<P> = Vec::with_capacity(ring.len());
    for &p in ring {
        if v.last() != Some(&p) {
            v.push(p);
        }
    }
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    loop {
        let n = v.len();
        if n < 3 {
            return v;
        }
        let mut out: Vec<P> = Vec::with_capacity(n);
        let mut changed = false;
        for i in 0..n {
            let a = v[(i + n - 1) % n];
            let b = v[i];
            let c = v[(i + 1) % n];
            let straight = orient(a, b, c) == 0 && pred::dot(pred::dir(a, b), pred::dir(b, c)) > 0;
            if straight && !keep(b) {
                changed = true;
                continue;
            }
            out.push(b);
        }
        if !changed {
            return out;
        }
        v = out;
    }
}

/// Rotates a ring to start at its least vertex (2.5).
pub fn rotate_least(ring: &mut [P]) {
    if let Some((i, _)) = ring.iter().enumerate().min_by_key(|(_, p)| **p) {
        ring.rotate_left(i);
    }
}

/// Puts a region into canonical form (2.5): ring orientation, start vertex, sort order. Collinear
/// vertices are dropped unless another ring of the region shares them.
pub fn canonical(mut region: Region) -> Region {
    let mut shared: std::collections::BTreeMap<P, u32> = std::collections::BTreeMap::new();
    for ring in region.rings() {
        let mut seen: Vec<P> = ring.to_vec();
        seen.sort();
        seen.dedup();
        for p in seen {
            *shared.entry(p).or_insert(0) += 1;
        }
    }
    let keep = |p: P| shared.get(&p).copied().unwrap_or(0) > 1;
    for piece in &mut region.pieces {
        piece.shell = simplify_ring(&piece.shell, &keep);
        if twice_area(&piece.shell) < 0 {
            piece.shell.reverse();
        }
        rotate_least(&mut piece.shell);
        for h in &mut piece.holes {
            *h = simplify_ring(h, &keep);
            if twice_area(h) > 0 {
                h.reverse();
            }
            rotate_least(h);
        }
        piece.holes.sort_by_key(|a| ring_key(a));
    }
    region
        .pieces
        .sort_by(|a, b| ring_key(&a.shell).cmp(&ring_key(&b.shell)));
    region
}

/// The sort key of a ring: its start vertex, then its second.
pub fn ring_key(r: &[P]) -> (P, P) {
    (r[0], *r.get(1).unwrap_or(&r[0]))
}

/// A path read from text: as written, repeats removed (2.2, 2.3).
pub fn read_path(source: &str, points: &[P], diags: &mut Vec<GeomDiag>) -> Option<Vec<P>> {
    let mut out: Vec<P> = Vec::with_capacity(points.len());
    for &p in points {
        if out.last() == Some(&p) {
            diags.push(
                GeomDiag::new(
                    Code::RepeatedPoint,
                    "a point repeats the one before it; removed",
                )
                .source(source)
                .at(p),
            );
            continue;
        }
        out.push(p);
    }
    if out.len() < 2 {
        diags.push(
            GeomDiag::new(Code::Degenerate, "a path needs two distinct points")
                .source(source)
                .at(*points.first().unwrap_or(&P::new(0, 0))),
        );
        return None;
    }
    Some(out)
}

/// A ring read from text (2.3): counter-clockwise, its first written point kept as `t = 0`;
/// repeats removed; a ring that crosses itself, visits a point twice or doubles back is an error,
/// never repaired.
pub fn read_ring(source: &str, points: &[P], diags: &mut Vec<GeomDiag>) -> Option<Vec<P>> {
    let mut v: Vec<P> = Vec::with_capacity(points.len());
    for &p in points {
        if v.last() == Some(&p) {
            diags.push(
                GeomDiag::new(
                    Code::RepeatedPoint,
                    "a point repeats the one before it; removed",
                )
                .source(source)
                .at(p),
            );
            continue;
        }
        v.push(p);
    }
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    let mut distinct = v.clone();
    distinct.sort();
    distinct.dedup();
    if distinct.len() < 3 {
        diags.push(
            GeomDiag::new(Code::Degenerate, "a ring needs three distinct points")
                .source(source)
                .at(*points.first().unwrap_or(&P::new(0, 0))),
        );
        return None;
    }
    if v.windows(3).all(|w| orient(w[0], w[1], w[2]) == 0)
        && orient(v[v.len() - 2], v[v.len() - 1], v[0]) == 0
    {
        diags.push(
            GeomDiag::new(Code::Degenerate, "the ring has no area")
                .source(source)
                .at(v[0]),
        );
        return None;
    }
    if let Some(at) = first_fault(&v) {
        diags.push(
            GeomDiag::new(
                Code::SelfCrossing,
                "the ring crosses itself, visits a point twice or doubles back",
            )
            .source(source)
            .at(at),
        );
        return None;
    }
    if twice_area(&v) == 0 {
        diags.push(
            GeomDiag::new(Code::Degenerate, "the ring has no area")
                .source(source)
                .at(v[0]),
        );
        return None;
    }
    if twice_area(&v) < 0 {
        // Reverse, keeping the first written point first.
        v[1..].reverse();
    }
    Some(v)
}

/// The first fault that makes a ring not simple, in written order: a point visited twice, two
/// edges that overlap, or two edges that cross.
pub fn first_fault(v: &[P]) -> Option<P> {
    let n = v.len();
    for i in 0..n {
        for j in 0..i {
            if v[i] == v[j] {
                return Some(v[i]);
            }
        }
    }
    let mut first: Option<(usize, P)> = None;
    for i in 0..n {
        let (a, b) = (v[i], v[(i + 1) % n]);
        for j in (i + 1)..n {
            let (c, d) = (v[j], v[(j + 1) % n]);
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            let m = pred::meet(a, b, c, d);
            let fault = match m {
                pred::Meet::None => None,
                pred::Meet::Overlap => {
                    // The shared vertex of adjacent edges that double back.
                    Some(if adjacent {
                        if j == i + 1 {
                            b
                        } else {
                            a
                        }
                    } else {
                        c
                    })
                }
                pred::Meet::Point(p) => {
                    if adjacent {
                        let shared = if j == i + 1 { b } else { a };
                        if p == shared && !touches_other(a, b, c, d, shared) {
                            None
                        } else {
                            Some(shared)
                        }
                    } else {
                        Some(p)
                    }
                }
            };
            if let Some(p) = fault {
                if first.is_none_or(|(k, _)| i < k) {
                    first = Some((i, p));
                }
                break;
            }
        }
    }
    first.map(|(_, p)| p)
}

fn touches_other(a: P, b: P, c: P, d: P, shared: P) -> bool {
    // Adjacent edges meet only at their shared vertex unless collinear (handled as overlap).
    let others = [a, b, c, d].into_iter().filter(|&p| p != shared).count();
    others < 2
}

/// Checks every computed point is in range (1, Range), giving `geom.range` at the first that is not.
pub fn check_range(source: &str, pts: impl Iterator<Item = P>) -> Result<(), GeomDiag> {
    for p in pts {
        if !p.in_range() {
            return Err(GeomDiag::new(
                Code::Range,
                "a computed point lies outside the lattice range",
            )
            .source(source)
            .at(p));
        }
    }
    Ok(())
}
