//! A path along another feature (`spec/geometry.md` 3.5, the lowering of `follows`).
//!
//! The substring of the reference over the span, then its one-sided offset curve: the outline of
//! its one-sided stroke at `|offset|` with round joins, less the reference itself and the two
//! ends (the method of JTS `OffsetCurve`). Loops an inside bend would make fall inside the
//! stroke and so are cut out. If the curve is not one simple piece the result is
//! `geom.follow_broken`, with each gap and the widest offset that holds, found by bisection.

use crate::diag::{Code, GeomDiag};
use crate::lineref::{substring, Line};
use crate::offset::{offset, Join};
use crate::overlay::wind;
use crate::shape::{Region, P};
use std::collections::BTreeMap;
use via_diag::Value;

/// The reference of a `follows`.
#[derive(Clone, Debug)]
pub enum Reference {
    /// A path.
    Path(Vec<P>),
    /// A region of one ring.
    Ring(Vec<P>),
}

/// What a probe found: the curve's pieces.
fn curve_pieces(source: &str, sub: &[P], off: i64) -> Result<Vec<Vec<P>>, GeomDiag> {
    let ctx_src = source;
    let r = off.unsigned_abs() as f64;
    // The offset side, as a raw chain: right of the reversed substring for a left offset.
    let (base, side_chain) = if off > 0 {
        let rev: Vec<P> = sub.iter().rev().copied().collect();
        (sub.to_vec(), raw_side(ctx_src, &rev, r)?)
    } else {
        let rev: Vec<P> = sub.iter().rev().copied().collect();
        (rev, raw_side(ctx_src, sub, r)?)
    };
    // The one-sided stroke ring: base forward, then the offset chain back. Three groups so the
    // outline can be told apart: 0 the base, 1 the offset chain, 2 the two ends.
    let mut ring = base.clone();
    ring.extend(side_chain.iter().copied());
    let ov = wind(&[vec![ring]])?;
    let sel = ov.select(&|w| w[0] > 0);
    // Outline half-edges that come from the offset chain only.
    let nb = base.len();
    let ns = side_chain.len();
    let total = nb + ns;
    let from_chain = |seg: usize| seg >= nb && seg < total - 1;
    let mut keep_pts: Vec<Vec<P>> = Vec::new();
    for mut ringh in ov.boundary(&sel) {
        let is_chain = |h: usize| {
            ov.noded.segments[h / 2]
                .prov
                .iter()
                .all(|p| from_chain(p.seg))
        };
        match ringh.iter().position(|&h| !is_chain(h)) {
            Some(i) => ringh.rotate_left(i),
            None => {
                // A closed curve with nothing of the base on it: one closed piece.
                let mut pts: Vec<P> = ringh.iter().map(|&h| ov.arr.org(h)).collect();
                pts.push(pts[0]);
                keep_pts.push(pts);
                continue;
            }
        }
        let mut cur: Vec<P> = Vec::new();
        for &h in &ringh {
            if is_chain(h) {
                if cur.is_empty() {
                    cur.push(ov.arr.org(h));
                }
                cur.push(ov.arr.dst(h));
            } else if !cur.is_empty() {
                keep_pts.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            keep_pts.push(cur);
        }
    }
    // In the reference's direction.
    for k in keep_pts.iter_mut() {
        if off > 0 {
            k.reverse();
        }
    }
    keep_pts.sort();
    Ok(keep_pts)
}

/// The raw offset chain of `pts` to the right at `r` mm, round joins, repeats removed.
fn raw_side(source: &str, pts: &[P], r: f64) -> Result<Vec<P>, GeomDiag> {
    let mut v = crate::offset::raw_right(source, pts, r)?;
    v.dedup();
    Ok(v)
}

fn simple_one(pieces: &[Vec<P>]) -> bool {
    if pieces.len() != 1 {
        return false;
    }
    let p = &pieces[0];
    let mut seen: Vec<P> = p.clone();
    let closed = p.len() > 2 && p.first() == p.last();
    if closed {
        seen.pop();
    }
    seen.sort();
    let n = seen.len();
    seen.dedup();
    seen.len() == n
}

/// `follow` (3.5): a path along a reference, `offset` mm to the left of its travel.
pub fn follow(
    source: &str,
    reference: &Reference,
    from: f64,
    to: f64,
    off: i64,
) -> Result<(Vec<P>, Vec<GeomDiag>), GeomDiag> {
    let (line, is_ring) = match reference {
        Reference::Path(p) => (Line::new(p.clone(), false), false),
        Reference::Ring(r) => (Line::new(r.clone(), true), true),
    };
    if line.segments() == 0 || line.length() == 0.0 {
        return Err(GeomDiag::new(Code::NoLength, "the reference has no length").source(source));
    }
    let sub = substring(&line, source, from, to)?;
    if off == 0 {
        return Ok((sub, Vec::new()));
    }
    if is_ring && from == to {
        return whole_outline(source, &line, off);
    }
    let pieces = curve_pieces(source, &sub, off)?;
    if simple_one(&pieces) {
        return Ok((pieces.into_iter().next().unwrap_or_default(), Vec::new()));
    }
    Err(broken(source, &pieces, off, &|o| {
        curve_pieces(source, &sub, o)
            .map(|p| simple_one(&p))
            .unwrap_or(false)
    }))
}

fn whole_outline(source: &str, line: &Line, off: i64) -> Result<(Vec<P>, Vec<GeomDiag>), GeomDiag> {
    let region = Region::from_ring(line.pts.clone());
    let probe = |o: i64| -> Option<Vec<P>> {
        let (r, _) = offset(source, &region, -o, Join::Round).ok()?;
        if r.pieces.len() == 1 && r.pieces[0].holes.is_empty() {
            Some(r.pieces[0].shell.clone())
        } else {
            None
        }
    };
    match probe(off) {
        Some(mut shell) => {
            // Start nearest the reference's first point, and close.
            let p0 = line.pts[0];
            let (i, _) = shell
                .iter()
                .enumerate()
                .min_by_key(|(_, q)| {
                    let (dx, dy) = (i128::from(q.x - p0.x), i128::from(q.y - p0.y));
                    (dx * dx + dy * dy, **q)
                })
                .unwrap_or((0, &p0));
            shell.rotate_left(i);
            let first = shell[0];
            shell.push(first);
            Ok((shell, Vec::new()))
        }
        None => {
            let (r, _) = offset(source, &region, -off, Join::Round)?;
            let pieces: Vec<Vec<P>> = r.pieces.iter().map(|p| p.shell.clone()).collect();
            Err(broken(source, &pieces, off, &|o| probe(o).is_some()))
        }
    }
}

fn broken(source: &str, pieces: &[Vec<P>], off: i64, holds: &dyn Fn(i64) -> bool) -> GeomDiag {
    // Widest offset that holds, by bisection over whole millimetres.
    let sign = if off < 0 { -1 } else { 1 };
    let (mut lo, mut hi) = (0i64, off.abs());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if holds(sign * mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut gaps: Vec<Value> = Vec::new();
    let mut ends: BTreeMap<P, ()> = BTreeMap::new();
    for p in pieces {
        if let (Some(a), Some(b)) = (p.first(), p.last()) {
            ends.insert(*a, ());
            ends.insert(*b, ());
        }
    }
    for p in ends.keys() {
        gaps.push(Value::List(vec![Value::Length(p.x), Value::Length(p.y)]));
    }
    let at = pieces
        .first()
        .and_then(|p| p.last().copied())
        .unwrap_or(P::new(0, 0));
    GeomDiag::new(
        Code::FollowBroken,
        format!(
            "the path along the reference breaks into {} pieces",
            pieces.len().max(2)
        ),
    )
    .source(source)
    .at(at)
    .with("gaps", Value::List(gaps))
    .with("widest", Value::Length(sign * lo))
}
