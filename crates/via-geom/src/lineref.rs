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
pub fn nearest(line: &Line, p: P) -> Nearest {
    let mut best: Option<Nearest> = None;
    // An exact integer bound on the best distance squared so far (to a vertex seen), so a segment
    // whose box lies strictly farther is skipped without its exact test. Skipping never changes
    // the answer: such a segment is strictly worse than the best, ties included.
    let sq = |a: P| {
        let (dx, dy) = (i128::from(a.x - p.x), i128::from(a.y - p.y));
        dx * dx + dy * dy
    };
    let mut upper: Option<i128> = None;
    for k in 0..line.segments() {
        let (a, b) = line.seg(k);
        let gap = |lo: i64, hi: i64, v: i64| -> i128 {
            if v < lo {
                i128::from(lo - v)
            } else if v > hi {
                i128::from(v - hi)
            } else {
                0
            }
        };
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
    let n = nearest(line, p);
    let (e, t, q) = e_t(line, p, &n);
    let s = path_side(line, p, &n);
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
    let mut best: Option<(Line, Nearest)> = None;
    for ring in region.rings() {
        let line = Line::new(ring.to_vec(), true);
        let n = nearest(&line, p);
        let better = match &best {
            None => true,
            Some((_, b)) => n.dist.cmp_exact(&b.dist) == Ordering::Less,
        };
        if better {
            best = Some((line, n));
        }
    }
    let (line, n) = best?;
    let (e, t, q) = e_t(&line, p, &n);
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
