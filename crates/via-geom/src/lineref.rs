//! Linear referencing and the local frame `t`, `d`, `sd` (`spec/geometry.md` 2.6 and 3.8,
//! `spec/numbers.md` 5.5).
//!
//! The nearest segment, and whether the nearest point is a vertex or an interior, are decided
//! exactly in integers; the measures are then the chapter's double expressions, in the order
//! written, so the kernel and formulas give the same bits.

use crate::diag::{Code, GeomDiag};
use crate::pred::{self, cross, dir, dot, Foot};
use crate::shape::{Region, P};
use std::cmp::Ordering;

/// A line to measure along: a path, or one closed ring (read with its closing segment).
#[derive(Clone, Debug)]
pub struct Line {
    /// The vertices; for a ring, the closing point is not repeated.
    pub pts: Vec<P>,
    /// True for a ring.
    pub closed: bool,
    /// Segment lengths `lk` (numbers.md 5.5).
    pub lens: Vec<f64>,
    /// Cumulative lengths `Ck`, with `C0 = 0` and the last `L`.
    pub cum: Vec<f64>,
}

impl Line {
    /// A line over a path or a ring. The caller has removed consecutive repeats.
    pub fn new(pts: Vec<P>, closed: bool) -> Line {
        let n = segment_count(pts.len(), closed);
        let mut lens = Vec::with_capacity(n);
        let mut cum = Vec::with_capacity(n + 1);
        cum.push(0.0);
        for k in 0..n {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            let u = dir(a, b);
            let l = (dot(u, u) as f64).sqrt();
            lens.push(l);
            let c = cum[k] + l;
            cum.push(c);
        }
        Line {
            pts,
            closed,
            lens,
            cum,
        }
    }

    /// The number of segments.
    pub fn segments(&self) -> usize {
        self.lens.len()
    }

    /// Segment `k`'s two ends.
    pub fn seg(&self, k: usize) -> (P, P) {
        (self.pts[k], self.pts[(k + 1) % self.pts.len()])
    }

    /// The total length `L`, millimetres.
    pub fn length(&self) -> f64 {
        self.cum[self.segments()]
    }

    /// The `t` of the nearest point to `p` on segment `k`, by `project`'s expressions restricted
    /// to that segment (`Ci / L` at a vertex; the end of a ring's closing segment is 1).
    pub fn t_on_segment(&self, k: usize, p: P) -> f64 {
        let (a, b) = self.seg(k);
        let l = self.length();
        match pred::dist2_segment(p, a, b).1 {
            Foot::Start => self.cum[k] / l,
            Foot::End => {
                if self.closed && k + 1 == self.segments() {
                    1.0
                } else {
                    self.cum[k + 1] / l
                }
            }
            Foot::Interior => {
                let u = dir(a, b);
                let w = dir(a, p);
                (self.cum[k] + (dot(u, w) as f64) / self.lens[k]) / l
            }
        }
    }
}

fn segment_count(n: usize, closed: bool) -> usize {
    if closed {
        n
    } else {
        n.saturating_sub(1)
    }
}

/// The measures of one point (3.8, `project`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure {
    /// The fraction along the line; `None` on a point or a region of several rings.
    pub t: Option<f64>,
    /// The distance from the feature, metres.
    pub d: f64,
    /// The signed distance, metres.
    pub sd: f64,
    /// The nearest point on the line, millimetres.
    pub nearest: (f64, f64),
}

/// The nearest place on a line: segment, where on it, the exact squared distance.
#[derive(Clone, Copy, Debug)]
pub struct Nearest {
    /// The segment index.
    pub k: usize,
    /// Start, end or interior.
    pub foot: Foot,
    /// The exact squared distance.
    pub dist: pred::Dist2,
}

/// The nearest place on a line to `p`; ties to the lower `t`.
///
/// The segments are scanned in increasing order with `upper` an integer bound on the least
/// squared distance (to a vertex, or the ceiling of the best exact distance so far): a segment
/// whose box lies strictly farther is skipped without its exact test. Skipping never changes the
/// answer: such a segment is strictly worse than the best, ties included.
pub fn nearest(line: &Line, p: P) -> Nearest {
    let sq = |a: P| {
        let (dx, dy) = (i128::from(a.x - p.x), i128::from(a.y - p.y));
        dx * dx + dy * dy
    };
    // The nearest vertex first, so the bound prunes from the first segment on: a scan in order
    // would test exactly every segment until it came near the point.
    let mut upper: Option<i128> = line.pts.iter().map(|&a| sq(a)).min();
    let mut best: Option<Nearest> = None;
    let sq = |a: P| {
        let (dx, dy) = (i128::from(a.x - p.x), i128::from(a.y - p.y));
        dx * dx + dy * dy
    };
    for k in 0..line.segments() {
        let (a, b) = line.seg(k);
        let gx = gap(a.x.min(b.x), a.x.max(b.x), p.x);
        let gy = gap(a.y.min(b.y), a.y.max(b.y), p.y);
        if upper.is_some_and(|u| gx * gx + gy * gy > u) {
            continue;
        }
        let near_end = sq(a).min(sq(b));
        upper = Some(upper.map_or(near_end, |u| u.min(near_end)));
        let (d, foot) = pred::dist2_segment(p, a, b);
        let better = match &best {
            None => true,
            Some(bst) => d.cmp_exact(&bst.dist) == Ordering::Less,
        };
        if better {
            // A segment whose box lies strictly beyond the ceiling of this distance is strictly
            // farther than it.
            if let (Some(c), Some(u)) = (d.ceil(), upper) {
                upper = Some(u.min(c));
            }
            best = Some(Nearest { k, foot, dist: d });
        }
    }
    // A line has at least one segment.
    best.unwrap_or(Nearest {
        k: 0,
        foot: Foot::Start,
        dist: pred::dist2_segment(p, line.pts[0], line.pts[0]).0,
    })
}

/// How far `v` lies outside `[lo, hi]`, millimetres.
fn gap(lo: i64, hi: i64, v: i64) -> i128 {
    if v < lo {
        i128::from(lo - v)
    } else if v > hi {
        i128::from(v - hi)
    } else {
        0
    }
}

/// Segments by square cell of the line's box, to find the nearest place to many points
/// (geometry.md 3.8, How the nearest is found). A search only: [`Index::nearest`] returns what
/// [`nearest`] returns, segment, foot and exact distance, for every point.
#[derive(Clone, Debug)]
pub struct Index {
    /// The box's low corner, millimetres.
    x0: i64,
    y0: i64,
    /// The cell side, millimetres (at least 1).
    c: i64,
    nx: i64,
    ny: i64,
    /// Cell `(i, j)`'s segments are `items[start[j * nx + i]..start[j * nx + i + 1]]`, in
    /// increasing order; `None` for a line short enough to scan.
    cells: Option<(Vec<u32>, Vec<u32>)>,
}

/// Lines of at most this many segments are scanned, not indexed.
const SCAN_MAX: usize = 16;

impl Index {
    /// The index of `line`'s segments.
    pub fn new(line: &Line) -> Index {
        let n = line.segments();
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for q in &line.pts {
            x0 = x0.min(q.x);
            y0 = y0.min(q.y);
            x1 = x1.max(q.x);
            y1 = y1.max(q.y);
        }
        let scan = Index {
            x0,
            y0,
            c: 1,
            nx: 1,
            ny: 1,
            cells: None,
        };
        if n <= SCAN_MAX || u32::try_from(n).is_err() {
            return scan;
        }
        let (w, h) = (
            u128::try_from(x1 - x0).unwrap_or(0) + 1,
            u128::try_from(y1 - y0).unwrap_or(0) + 1,
        );
        // About one cell per segment: the side of a square of the box's area over `n`, and never
        // so small that the long side holds more than `4 n` cells.
        let n128 = n as u128;
        let by_area = isqrt((w * h).div_ceil(n128)).max(1);
        let by_side = w.max(h).div_ceil(4 * n128).max(1);
        let c = i64::try_from(by_area.max(by_side)).unwrap_or(i64::MAX);
        let nx = (x1 - x0) / c + 1;
        let ny = (y1 - y0) / c + 1;
        let Some(cells) = usize::try_from(nx)
            .ok()
            .zip(usize::try_from(ny).ok())
            .and_then(|(a, b)| a.checked_mul(b))
            .filter(|&k| k <= 16 * n + 16)
        else {
            return scan;
        };
        let span = |k: usize| {
            let (a, b) = line.seg(k);
            (
                (a.x.min(b.x) - x0) / c,
                (a.x.max(b.x) - x0) / c,
                (a.y.min(b.y) - y0) / c,
                (a.y.max(b.y) - y0) / c,
            )
        };
        let mut count = vec![0u32; cells + 1];
        for k in 0..n {
            let (i0, i1, j0, j1) = span(k);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    count[(j * nx + i) as usize + 1] += 1;
                }
            }
        }
        for i in 0..cells {
            count[i + 1] += count[i];
        }
        let mut fill = count.clone();
        let mut items = vec![0u32; count[cells] as usize];
        for k in 0..n {
            let (i0, i1, j0, j1) = span(k);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let cell = (j * nx + i) as usize;
                    items[fill[cell] as usize] = k as u32;
                    fill[cell] += 1;
                }
            }
        }
        Index {
            x0,
            y0,
            c,
            nx,
            ny,
            cells: Some((count, items)),
        }
    }

    /// The nearest place on `line` (the line this index was built over) to `p`: [`nearest`]'s
    /// answer, the least exact distance with ties to the lower segment.
    ///
    /// Cells are visited in square rings around `p`'s cell. A segment not yet met lies wholly in
    /// cells outside the square visited, so its box, and the segment, is at least the gap from
    /// `p` to the square's nearest open side; once the best exact distance is strictly below
    /// that gap squared, no unmet segment can equal it.
    pub fn nearest(&self, line: &Line, p: P) -> Nearest {
        let Some((start, items)) = &self.cells else {
            return nearest(line, p);
        };
        let c = self.c;
        let ci = (p.x - self.x0).div_euclid(c);
        let cj = (p.y - self.y0).div_euclid(c);
        let mut best: Option<Nearest> = None;
        let mut ceil: Option<i128> = None;
        let mut visit = |cell: i64, best: &mut Option<Nearest>| {
            let (a0, a1) = (
                start[cell as usize] as usize,
                start[cell as usize + 1] as usize,
            );
            for &k in &items[a0..a1] {
                let k = k as usize;
                let (a, b) = line.seg(k);
                let gx = gap(a.x.min(b.x), a.x.max(b.x), p.x);
                let gy = gap(a.y.min(b.y), a.y.max(b.y), p.y);
                if ceil.is_some_and(|u| gx * gx + gy * gy > u) {
                    continue;
                }
                let (d, foot) = pred::dist2_segment(p, a, b);
                let better = match best.as_ref() {
                    None => true,
                    Some(bst) => match d.cmp_exact(&bst.dist) {
                        Ordering::Less => true,
                        Ordering::Equal => k < bst.k,
                        Ordering::Greater => false,
                    },
                };
                if better {
                    ceil = d.ceil();
                    *best = Some(Nearest { k, foot, dist: d });
                }
            }
        };
        // Rings that hold no cell of the grid are passed over.
        let r0 = 0
            .max(-ci)
            .max(ci - (self.nx - 1))
            .max(-cj)
            .max(cj - (self.ny - 1));
        let mut r = r0;
        loop {
            let (lo_i, hi_i) = ((ci - r).max(0), (ci + r).min(self.nx - 1));
            let (lo_j, hi_j) = ((cj - r).max(0), (cj + r).min(self.ny - 1));
            for j in [cj - r, cj + r] {
                if (0..self.ny).contains(&j) {
                    for i in lo_i..=hi_i {
                        visit(j * self.nx + i, &mut best);
                    }
                }
                if r == 0 {
                    break;
                }
            }
            if r > 0 {
                for i in [ci - r, ci + r] {
                    if (0..self.nx).contains(&i) {
                        for j in (cj - r + 1).max(lo_j)..=(cj + r - 1).min(hi_j) {
                            visit(j * self.nx + i, &mut best);
                        }
                    }
                }
            }
            // The gap from `p` to each side of the square with grid cells beyond it.
            let mut bound: Option<i128> = None;
            let mut side = |open: bool, g: i64| {
                if open {
                    let g = i128::from(g);
                    bound = Some(bound.map_or(g, |b| b.min(g)));
                }
            };
            side(ci - r > 0, p.x - (self.x0 + (ci - r) * c));
            side(ci + r < self.nx - 1, self.x0 + (ci + r + 1) * c - p.x);
            side(cj - r > 0, p.y - (self.y0 + (cj - r) * c));
            side(cj + r < self.ny - 1, self.y0 + (cj + r + 1) * c - p.y);
            let Some(b) = bound else {
                break;
            };
            if let Some(bst) = &best {
                let b2 = pred::Dist2 {
                    num: pred::Wide::from_u128((b * b) as u128),
                    den: pred::Wide::from_u128(1),
                };
                if bst.dist.cmp_exact(&b2) == Ordering::Less {
                    break;
                }
            }
            r += 1;
        }
        best.unwrap_or_else(|| nearest(line, p))
    }
}

/// The integer square root, rounded down.
fn isqrt(v: u128) -> u128 {
    let mut x = (v as f64).sqrt() as u128;
    while x * x > v {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= v {
        x += 1;
    }
    x
}

/// `e` (metres), `t` and the nearest point for a nearest place.
fn e_t(line: &Line, p: P, n: &Nearest) -> (f64, f64, (f64, f64)) {
    let (a, b) = line.seg(n.k);
    let l = line.length();
    match n.foot {
        Foot::Interior => {
            let u = dir(a, b);
            let w = dir(a, p);
            let lk = line.lens[n.k];
            let e = ((cross(u, w).unsigned_abs() as f64) / lk) / 1000.0;
            let dw = dot(u, w) as f64;
            let t = (line.cum[n.k] + dw / lk) / l;
            let f = dw / (dot(u, u) as f64);
            let q = (
                (a.x as f64) + f * (u.0 as f64),
                (a.y as f64) + f * (u.1 as f64),
            );
            (e, t, q)
        }
        Foot::Start | Foot::End => {
            let (v, i) = if n.foot == Foot::Start {
                (a, n.k)
            } else {
                (b, n.k + 1)
            };
            let w = dir(v, p);
            let e = (dot(w, w) as f64).sqrt() / 1000.0;
            let t = if line.closed && i == line.segments() {
                0.0
            } else {
                line.cum[i] / l
            };
            (e, t, (v.x as f64, v.y as f64))
        }
    }
}

/// The side of a path, exact (numbers.md 5.5): +1 left (or zero), -1 right.
fn path_side(line: &Line, p: P, n: &Nearest) -> i32 {
    let sign = |v: i128| if v < 0 { -1 } else { 1 };
    let (a, b) = line.seg(n.k);
    match n.foot {
        Foot::Interior => sign(cross(dir(a, b), dir(a, p))),
        Foot::Start | Foot::End => {
            let i = if n.foot == Foot::Start { n.k } else { n.k + 1 };
            let last = line.pts.len() - 1;
            if i == 0 {
                let (s0, s1) = line.seg(0);
                sign(cross(dir(s0, s1), dir(s0, p)))
            } else if i == last {
                let (s0, s1) = line.seg(last - 1);
                sign(cross(dir(s0, s1), dir(s1, p)))
            } else {
                let v = line.pts[i];
                let a_dir = dir(line.pts[i - 1], v);
                let b_dir = dir(v, line.pts[i + 1]);
                let turn = cross(a_dir, b_dir);
                if turn != 0 {
                    -sign(turn)
                } else {
                    sign(cross(a_dir, dir(v, p)))
                }
            }
        }
    }
}

/// `project` on a path (3.8).
pub fn project_path(line: &Line, p: P) -> Measure {
    path_measure(line, p, &nearest(line, p))
}

/// [`project_path`] with the nearest place found through `index`, built over `line`: the same
/// measures, bit for bit (geometry.md 3.8, How the nearest is found).
pub fn project_path_in(line: &Line, index: &Index, p: P) -> Measure {
    path_measure(line, p, &index.nearest(line, p))
}

fn path_measure(line: &Line, p: P, n: &Nearest) -> Measure {
    let (e, t, q) = e_t(line, p, n);
    let s = path_side(line, p, n);
    // No negative zero leaves the kernel (`numbers.md` 8, rule 8): on the path, `sd` is `+0`.
    let sd = if s < 0 && e != 0.0 { -e } else { e };
    Measure {
        t: Some(t),
        d: e,
        sd,
        nearest: q,
    }
}

/// `project` on a region: `t` only when it has one ring.
pub fn project_region(region: &Region, p: P) -> Option<Measure> {
    let rings: Vec<Line> = region
        .rings()
        .into_iter()
        .map(|ring| Line::new(ring.to_vec(), true))
        .collect();
    region_measure(region, &rings, p, |_, line| nearest(line, p))
}

/// A region's rings as lines, each with its [`Index`]: what [`project_region_in`] reads.
pub fn region_rings(region: &Region) -> Vec<(Line, Index)> {
    region
        .rings()
        .into_iter()
        .map(|ring| {
            let line = Line::new(ring.to_vec(), true);
            let index = Index::new(&line);
            (line, index)
        })
        .collect()
}

/// [`project_region`] with each ring's nearest place found through its index ([`region_rings`]):
/// the same measures, bit for bit.
pub fn project_region_in(region: &Region, rings: &[(Line, Index)], p: P) -> Option<Measure> {
    let lines: Vec<&Line> = rings.iter().map(|(l, _)| l).collect();
    region_measure(region, &lines, p, |i, line| rings[i].1.nearest(line, p))
}

fn region_measure<L: std::borrow::Borrow<Line>>(
    region: &Region,
    rings: &[L],
    p: P,
    near: impl Fn(usize, &Line) -> Nearest,
) -> Option<Measure> {
    let mut best: Option<(&Line, Nearest)> = None;
    for (i, line) in rings.iter().enumerate() {
        let line = line.borrow();
        let n = near(i, line);
        let better = match &best {
            None => true,
            Some((_, b)) => n.dist.cmp_exact(&b.dist) == Ordering::Less,
        };
        if better {
            best = Some((line, n));
        }
    }
    let (line, n) = best?;
    let (e, t, q) = e_t(line, p, &n);
    let inside = region.side(p) != pred::Side::Out;
    let (sd, d) = if inside { (e, 0.0) } else { (-e, e) };
    Some(Measure {
        t: if region.ring_count() == 1 {
            Some(t)
        } else {
            None
        },
        d,
        sd,
        nearest: q,
    })
}

/// `project` on a point.
pub fn project_point(q: P, p: P) -> Measure {
    let w = dir(q, p);
    let e = (dot(w, w) as f64).sqrt() / 1000.0;
    Measure {
        t: None,
        d: e,
        sd: if e == 0.0 { 0.0 } else { -e },
        nearest: (q.x as f64, q.y as f64),
    }
}

/// The point at `t` with an offset, and the segment that holds it (numbers.md 5.5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct At {
    /// The point, rounded to the lattice.
    pub point: P,
    /// The segment that holds `t`.
    pub k: usize,
    /// The bearing of that segment's travel, millidegrees.
    pub bearing: i64,
}

/// Checks a `t` (3.8): outside `[0, 1]` or `nan` is `geom.t_range`.
pub fn check_t(source: &str, t: f64) -> Result<(), GeomDiag> {
    if t.is_nan() || !(0.0..=1.0).contains(&t) {
        return Err(GeomDiag::new(Code::TRange, "t lies outside 0 to 1").source(source));
    }
    Ok(())
}

/// The segment that holds arc length `s` (ties to the later segment; `t = 1` the last).
pub fn holding(line: &Line, s: f64, at_end: bool) -> usize {
    let n = line.segments();
    if at_end {
        return n - 1;
    }
    let mut k = 0;
    while k + 1 < n && line.cum[k + 1] <= s {
        k += 1;
    }
    k
}

/// `interpolate`: the point at `t`, moved `offset` mm along the left normal (numbers.md 5.5).
pub fn interpolate(line: &Line, source: &str, t: f64, offset: i64) -> Result<At, GeomDiag> {
    check_t(source, t)?;
    let s = t * line.length();
    at_length(line, source, s, t == 1.0, offset)
}

/// The point at arc length `s` (numbers.md 5.5, used by 6 and by cells).
pub fn at_length(
    line: &Line,
    source: &str,
    s: f64,
    at_end: bool,
    offset: i64,
) -> Result<At, GeomDiag> {
    let k = holding(line, s, at_end);
    let (a, b) = line.seg(k);
    let (dx, dy) = dir(a, b);
    let lk = line.lens[k];
    let f = if at_end { 1.0 } else { (s - line.cum[k]) / lk };
    let qx = (a.x as f64) + (f * (dx as f64));
    let qy = (a.y as f64) + (f * (dy as f64));
    let nx = (-(dy as f64)) / lk;
    let ny = (dx as f64) / lk;
    let o = offset as f64;
    let px = round(source, qx + (o * nx))?;
    let py = round(source, qy + (o * ny))?;
    Ok(At {
        point: P::new(px, py),
        k,
        bearing: via_num::bearing(dx, dy).unwrap_or(0),
    })
}

/// Rounds a computed coordinate to the lattice; out of range is `geom.range`.
pub fn round(source: &str, v: f64) -> Result<i64, GeomDiag> {
    via_num::round_to_lattice(v).map_err(|_| {
        GeomDiag::new(
            Code::Range,
            "a computed point lies outside the lattice range",
        )
        .source(source)
    })
}

/// `substring`: the path between `from` and `to` along a line, in its direction. On a closed
/// line `from > to` runs through `t = 0` and `from = to` is the whole outline (3.5).
pub fn substring(line: &Line, source: &str, from: f64, to: f64) -> Result<Vec<P>, GeomDiag> {
    check_t(source, from)?;
    check_t(source, to)?;
    if !line.closed && from >= to {
        return Err(
            GeomDiag::new(Code::SpanOrder, "the span's from is not less than its to")
                .source(source),
        );
    }
    let l = line.length();
    let start = at_length(line, source, from * l, from == 1.0, 0)?;
    let mut out = vec![start.point];
    let push = |out: &mut Vec<P>, p: P| {
        if out.last() != Some(&p) {
            out.push(p);
        }
    };
    if !line.closed {
        let end = at_length(line, source, to * l, to == 1.0, 0)?;
        for k in (start.k + 1)..=end.k {
            push(&mut out, line.pts[k]);
        }
        push(&mut out, end.point);
        return Ok(out);
    }
    let n = line.segments();
    if from < to {
        let end = at_length(line, source, to * l, false, 0)?;
        let end_k = if to == 1.0 { n - 1 } else { end.k };
        for k in (start.k + 1)..=end_k {
            push(&mut out, line.pts[k % n]);
        }
        push(&mut out, end.point);
    } else {
        // Through t = 0 (or the whole outline when from = to).
        let end = at_length(line, source, to * l, false, 0)?;
        let mut k = start.k + 1;
        while k <= end.k + n {
            push(&mut out, line.pts[k % n]);
            k += 1;
        }
        push(&mut out, end.point);
    }
    Ok(out)
}

/// `reverse`.
pub fn reverse(path: &[P]) -> Vec<P> {
    path.iter().rev().copied().collect()
}
