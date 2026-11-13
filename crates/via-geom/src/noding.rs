//! Noding by iterated snap rounding on the 1 mm lattice, with provenance
//! (`spec/geometry.md` 3.1; Hobby 1999, Halperin and Packer 2002).
//!
//! Every segment end and every crossing makes a hot pixel. Each segment is routed through the
//! centres of the hot pixels it passes through, in order along it; each resulting fragment is
//! routed again until no fragment passes through a hot pixel other than those of its two ends.
//! The pixel of `(x, y)` is `[x - 0.5, x + 0.5) × [y - 0.5, y + 0.5)`, half-open on the high side
//! of both axes. Since every fragment end is a hot pixel centre, noding the output again finds
//! the same hot pixels and the same fragments: the result is idempotent.

use crate::diag::{Code, GeomDiag};
use crate::lineref::Line;
use crate::pred::{self, Meet};
use crate::shape::P;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// One input of a noding: a path or a ring outline, with its source id.
#[derive(Clone, Debug)]
pub struct Input {
    /// The source id.
    pub source: String,
    /// The vertices, consecutive repeats removed; a ring's closing point is not repeated.
    pub pts: Vec<P>,
    /// True for a ring outline.
    pub closed: bool,
}

/// One provenance entry of a segment (3.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Prov {
    /// Index of the source in [`Noded::sources`].
    pub source: usize,
    /// The segment index on the source.
    pub seg: usize,
    /// The `t` on the source of the segment's two ends, least end first.
    pub t: [f64; 2],
    /// True when the source travels from the other end to the least end.
    pub reversed: bool,
}

/// A noded segment: least end, other end, provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct Seg {
    /// The least end.
    pub a: P,
    /// The other end.
    pub b: P,
    /// Every source it lies on, by source id then segment index.
    pub prov: Vec<Prov>,
}

/// Noding's output (3.1).
#[derive(Clone, Debug)]
pub struct Noded {
    /// The source ids, sorted bytewise; provenance indexes into this list.
    pub sources: Vec<String>,
    /// The inputs, in the order of `sources`.
    pub inputs: Vec<Input>,
    /// Every vertex, by `(x, y)`.
    pub vertices: Vec<P>,
    /// Segments, by (least end, other end).
    pub segments: Vec<Seg>,
}

/// A fraction `num / den` with `den > 0`.
#[derive(Clone, Copy, Debug)]
struct Frac {
    num: i128,
    den: i128,
}

impl Frac {
    fn new(num: i128, den: i128) -> Frac {
        if den < 0 {
            Frac {
                num: -num,
                den: -den,
            }
        } else {
            Frac { num, den }
        }
    }
    fn cmp(&self, o: &Frac) -> Ordering {
        (self.num * o.den).cmp(&(o.num * self.den))
    }
}

/// The entry and exit parameters of segment `a b` through the half-open pixel of `h`, or `None`
/// when it misses it.
fn pixel_hit(a: P, b: P, h: P) -> Option<(Frac, Frac)> {
    let (ax, ay) = (2 * i128::from(a.x), 2 * i128::from(a.y));
    let (dx, dy) = (2 * i128::from(b.x) - ax, 2 * i128::from(b.y) - ay);
    let (lox, hix) = (2 * i128::from(h.x) - 1, 2 * i128::from(h.x) + 1);
    let (loy, hiy) = (2 * i128::from(h.y) - 1, 2 * i128::from(h.y) + 1);
    let mut t0 = Frac::new(0, 1);
    let mut t1 = Frac::new(1, 1);
    for (s, d, lo, hi) in [(ax, dx, lox, hix), (ay, dy, loy, hiy)] {
        if d == 0 {
            if s < lo || s > hi {
                return None;
            }
            continue;
        }
        let (mut e, mut x) = (Frac::new(lo - s, d), Frac::new(hi - s, d));
        if d < 0 {
            std::mem::swap(&mut e, &mut x);
        }
        if e.cmp(&t0) == Ordering::Greater {
            t0 = e;
        }
        if x.cmp(&t1) == Ordering::Less {
            t1 = x;
        }
        if t0.cmp(&t1) == Ordering::Greater {
            return None;
        }
    }
    // Half-open: the closed hit fails when it lies wholly on a high edge.
    let at = |t: &Frac, s: i128, d: i128| s * t.den + t.num * d;
    let on_hix = at(&t0, ax, dx) == hix * t0.den && at(&t1, ax, dx) == hix * t1.den;
    let on_hiy = at(&t0, ay, dy) == hiy * t0.den && at(&t1, ay, dy) == hiy * t1.den;
    if on_hix || on_hiy {
        return None;
    }
    Some((t0, t1))
}

/// Hot pixels, sorted by `(x, y)`, with a range query by bounding box.
struct Hot {
    pts: Vec<P>,
}

impl Hot {
    fn hits(&self, a: P, b: P) -> Vec<P> {
        let (x0, x1) = (a.x.min(b.x) - 1, a.x.max(b.x) + 1);
        let (y0, y1) = (a.y.min(b.y) - 1, a.y.max(b.y) + 1);
        let lo = self.pts.partition_point(|p| p.x < x0);
        let mut found: Vec<(Frac, Frac, P)> = Vec::new();
        for &h in &self.pts[lo..] {
            if h.x > x1 {
                break;
            }
            if h.y < y0 || h.y > y1 {
                continue;
            }
            if let Some((e, x)) = pixel_hit(a, b, h) {
                found.push((e, x, h));
            }
        }
        found.sort_by(|p, q| p.0.cmp(&q.0).then(p.1.cmp(&q.1)).then(p.2.cmp(&q.2)));
        found.into_iter().map(|f| f.2).collect()
    }
}

/// Routes one segment through the hot pixels, iterating until stable; returns the fragments in
/// order along `a` to `b`.
fn route(hot: &Hot, a: P, b: P) -> Vec<P> {
    let mut out: Vec<P> = vec![a];
    // A stack of fragments still to check, in reverse order so they come out in order.
    let mut stack: Vec<(P, P)> = vec![(a, b)];
    let mut guard = 0usize;
    while let Some((p, q)) = stack.pop() {
        guard += 1;
        let hits = hot.hits(p, q);
        let inner: Vec<P> = hits.into_iter().filter(|&h| h != p && h != q).collect();
        if inner.is_empty() || guard > 1_000_000 {
            if out.last() != Some(&q) {
                out.push(q);
            }
            continue;
        }
        let mut chain = vec![p];
        chain.extend(inner);
        chain.push(q);
        for w in chain.windows(2).rev() {
            if w[0] != w[1] {
                stack.push((w[0], w[1]));
            }
        }
    }
    out
}

/// Checks the batch has no repeated source id (3, `geom.duplicate_source`).
pub fn check_sources<'a>(ids: impl Iterator<Item = &'a str>) -> Result<(), GeomDiag> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(GeomDiag::new(
                Code::DuplicateSource,
                format!("two inputs have source `{id}`"),
            )
            .source(id));
        }
    }
    Ok(())
}

/// Nodes a batch of paths and ring outlines (3.1). Inputs must have distinct sources.
pub fn node(mut inputs: Vec<Input>) -> Result<Noded, GeomDiag> {
    check_sources(inputs.iter().map(|i| i.source.as_str()))?;
    inputs.sort_by(|a, b| a.source.as_bytes().cmp(b.source.as_bytes()));
    // Raw segments.
    struct Raw {
        a: P,
        b: P,
        src: usize,
        seg: usize,
    }
    let mut raw: Vec<Raw> = Vec::new();
    for (i, inp) in inputs.iter().enumerate() {
        let n = inp.pts.len();
        let m = if inp.closed { n } else { n.saturating_sub(1) };
        for k in 0..m {
            let (a, b) = (inp.pts[k], inp.pts[(k + 1) % n]);
            if a != b {
                raw.push(Raw {
                    a,
                    b,
                    src: i,
                    seg: k,
                });
            }
        }
    }
    // Hot pixels: ends and crossings.
    let mut hot: BTreeSet<P> = BTreeSet::new();
    for r in &raw {
        hot.insert(r.a);
        hot.insert(r.b);
    }
    let mut order: Vec<usize> = (0..raw.len()).collect();
    order.sort_by_key(|&i| (raw[i].a.x.min(raw[i].b.x), i));
    for (oi, &i) in order.iter().enumerate() {
        let ri = &raw[i];
        let maxx = ri.a.x.max(ri.b.x);
        let (y0, y1) = (ri.a.y.min(ri.b.y), ri.a.y.max(ri.b.y));
        for &j in &order[oi + 1..] {
            let rj = &raw[j];
            if rj.a.x.min(rj.b.x) > maxx {
                break;
            }
            if rj.a.y.max(rj.b.y) < y0 || rj.a.y.min(rj.b.y) > y1 {
                continue;
            }
            if let Meet::Point(p) = pred::meet(ri.a, ri.b, rj.a, rj.b) {
                hot.insert(p);
            }
        }
    }
    let hot = Hot {
        pts: hot.into_iter().collect(),
    };
    // Route every raw segment and gather fragments with provenance.
    let lines: Vec<Line> = inputs
        .iter()
        .map(|i| Line::new(i.pts.clone(), i.closed))
        .collect();
    let mut frags: BTreeMap<(P, P), Vec<Prov>> = BTreeMap::new();
    for r in &raw {
        let chain = route(&hot, r.a, r.b);
        for w in chain.windows(2) {
            let (p, q) = (w[0], w[1]);
            if p == q {
                continue;
            }
            let (lo, hi, reversed) = if p < q { (p, q, false) } else { (q, p, true) };
            let line = &lines[r.src];
            let t = [line.t_on_segment(r.seg, lo), line.t_on_segment(r.seg, hi)];
            frags.entry((lo, hi)).or_default().push(Prov {
                source: r.src,
                seg: r.seg,
                t,
                reversed,
            });
        }
    }
    let mut segments: Vec<Seg> = Vec::with_capacity(frags.len());
    let mut verts: BTreeSet<P> = BTreeSet::new();
    for ((a, b), mut prov) in frags {
        prov.sort_by(|x, y| {
            x.source
                .cmp(&y.source)
                .then(x.seg.cmp(&y.seg))
                .then(x.reversed.cmp(&y.reversed))
        });
        prov.dedup_by(|x, y| x.source == y.source && x.seg == y.seg && x.reversed == y.reversed);
        verts.insert(a);
        verts.insert(b);
        segments.push(Seg { a, b, prov });
    }
    Ok(Noded {
        sources: inputs.iter().map(|i| i.source.clone()).collect(),
        inputs,
        vertices: verts.into_iter().collect(),
        segments,
    })
}

/// `geom.overlap` records (3.1): one per contiguous shared stretch per pair of sources, a source
/// running back over itself paired with itself.
pub fn overlaps(n: &Noded) -> Vec<GeomDiag> {
    // pair -> segments
    let mut pairs: BTreeMap<(usize, usize), Vec<(P, P)>> = BTreeMap::new();
    for s in &n.segments {
        for i in 0..s.prov.len() {
            for j in (i + 1)..s.prov.len() {
                let (x, y) = (&s.prov[i], &s.prov[j]);
                if x.source == y.source && x.seg == y.seg {
                    continue;
                }
                let key = (x.source.min(y.source), x.source.max(y.source));
                let v = pairs.entry(key).or_default();
                if v.last() != Some(&(s.a, s.b)) {
                    v.push((s.a, s.b));
                }
            }
        }
    }
    let mut out = Vec::new();
    for ((s1, s2), segs) in pairs {
        for (a, b) in stretches(&segs) {
            let (id1, id2) = (&n.sources[s1], &n.sources[s2]);
            out.push(
                GeomDiag::new(
                    Code::Overlap,
                    format!("`{id1}` and `{id2}` run along one another"),
                )
                .source(id1)
                .at(a)
                .with("other", id2.as_str())
                .with(
                    "from",
                    via_diag::Value::List(vec![
                        via_diag::Value::Length(a.x),
                        via_diag::Value::Length(a.y),
                    ]),
                )
                .with(
                    "to",
                    via_diag::Value::List(vec![
                        via_diag::Value::Length(b.x),
                        via_diag::Value::Length(b.y),
                    ]),
                ),
            );
        }
    }
    out
}

/// The contiguous stretches of a set of segments, each by its two ends (a closed stretch by its
/// least vertex twice), sorted.
fn stretches(segs: &[(P, P)]) -> Vec<(P, P)> {
    let mut adj: BTreeMap<P, Vec<P>> = BTreeMap::new();
    for &(a, b) in segs {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    let mut seen: BTreeSet<P> = BTreeSet::new();
    let mut out = Vec::new();
    for &start in adj.keys() {
        if seen.contains(&start) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        seen.insert(start);
        while let Some(v) = stack.pop() {
            comp.push(v);
            for &w in &adj[&v] {
                if seen.insert(w) {
                    stack.push(w);
                }
            }
        }
        let mut ends: Vec<P> = comp.iter().copied().filter(|v| adj[v].len() == 1).collect();
        ends.sort();
        comp.sort();
        match ends.len() {
            0 => out.push((comp[0], comp[0])),
            _ => out.push((ends[0], *ends.last().unwrap_or(&ends[0]))),
        }
    }
    out.sort();
    out
}

/// True when no two segments cross or overlap, and they meet only at shared vertices (3.1,
/// Noded). Used by the guarantee checks and the tests.
pub fn is_noded(segs: &[(P, P)]) -> bool {
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by_key(|&i| (segs[i].0.x.min(segs[i].1.x), i));
    for (oi, &i) in order.iter().enumerate() {
        let (a, b) = segs[i];
        let maxx = a.x.max(b.x);
        for &j in &order[oi + 1..] {
            let (c, d) = segs[j];
            if c.x.min(d.x) > maxx {
                break;
            }
            match pred::meet(a, b, c, d) {
                Meet::None => {}
                Meet::Overlap => return false,
                Meet::Point(p) => {
                    let exact_shared = (p == a || p == b) && (p == c || p == d);
                    if !exact_shared {
                        return false;
                    }
                    // A rounded crossing that is not an end of both is a fault even if the rounding
                    // lands on an end: check the exact predicate too.
                    if pred::orient(a, b, c) != 0 && pred::orient(a, b, d) != 0 {
                        let s1 = pred::orient(a, b, c).signum() != pred::orient(a, b, d).signum();
                        let s2 = pred::orient(c, d, a).signum() != pred::orient(c, d, b).signum();
                        if s1 && s2 && pred::orient(c, d, a) != 0 && pred::orient(c, d, b) != 0 {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}
