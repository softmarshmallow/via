//! Offsetting and stroking (`spec/geometry.md` 3.4): a raw offset curve with joins, then a
//! union by positive winding (after Chen and McMains 2005). Our fallback of the bake-off.

use crate::diag::{Code, GeomDiag};
use crate::lineref::round;
use crate::overlay::wind;
use crate::pred::{cross, dir, dot};
use crate::shape::{canonical, Region, P};

/// A join (3.4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Join {
    /// An arc about the input vertex.
    Round,
    /// The offset lines extended to their tip, cut square to the bisector past `limit × |d|`.
    Mitre(f64),
    /// The chord between the two offset ends.
    Bevel,
}

/// A cap (3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    /// Square at the end.
    Butt,
    /// An arc about the end.
    Round,
}

/// The arc tolerance of 3.4, millimetres.
pub const ARC_TOLERANCE_MM: f64 = 5.0;

/// The number of chords of an arc of angle `a` radians at radius `r` mm (3.4, Arcs).
pub fn arc_chords(a: f64, r: f64) -> u64 {
    if r <= ARC_TOLERANCE_MM {
        return 1;
    }
    let q = 1.0 - ARC_TOLERANCE_MM / r;
    let h = 2.0 * via_num::acos(q);
    let n = (a / h).ceil();
    if n < 1.0 {
        1
    } else {
        n as u64
    }
}

struct Ctx<'a> {
    source: &'a str,
}

impl Ctx<'_> {
    fn pt(&self, x: f64, y: f64) -> Result<P, GeomDiag> {
        Ok(P::new(round(self.source, x)?, round(self.source, y)?))
    }
}

/// The unit right normal of a direction, as doubles.
fn right_normal(d: (i64, i64)) -> (f64, f64) {
    let l = (dot(d, d) as f64).sqrt();
    ((d.1 as f64) / l, (-(d.0 as f64)) / l)
}

fn unit(d: (i64, i64)) -> (f64, f64) {
    let l = (dot(d, d) as f64).sqrt();
    ((d.0 as f64) / l, (d.1 as f64) / l)
}

/// Appends an arc about `v` of radius `r` from unit direction `u0` to `u1`, counter-clockwise
/// when `ccw`, without its first point and with its last point `end`.
#[allow(clippy::too_many_arguments)] // centre, radius, two directions, sense and end: one arc
fn arc(
    ctx: &Ctx,
    out: &mut Vec<P>,
    v: P,
    r: f64,
    u0: (f64, f64),
    u1: (f64, f64),
    ccw: bool,
    end: P,
) -> Result<(), GeomDiag> {
    let a0 = via_num::atan2(u0.1, u0.0);
    let a1 = via_num::atan2(u1.1, u1.0);
    let two_pi = 2.0 * via_num::PI;
    let mut sweep = if ccw { a1 - a0 } else { a0 - a1 };
    while sweep <= 0.0 {
        sweep += two_pi;
    }
    while sweep > two_pi {
        sweep -= two_pi;
    }
    let n = arc_chords(sweep, r);
    for i in 1..n {
        let step = (sweep * (i as f64)) / (n as f64);
        let a = if ccw { a0 + step } else { a0 - step };
        let p = ctx.pt(
            (v.x as f64) + (r * via_num::cos(a)),
            (v.y as f64) + (r * via_num::sin(a)),
        )?;
        out.push(p);
    }
    out.push(end);
    Ok(())
}

/// The raw offset of a chain by `d` mm to the right of travel (negative to the left), with joins.
/// An open chain gives the offset from its first segment's start to its last segment's end.
fn raw_offset(ctx: &Ctx, pts: &[P], closed: bool, d: f64, join: Join) -> Result<Vec<P>, GeomDiag> {
    let n = pts.len();
    let m = if closed { n } else { n - 1 };
    let r = d.abs();
    let sgn = if d < 0.0 { -1.0 } else { 1.0 };
    // Offset ends of each segment.
    let mut ends: Vec<(P, P, (f64, f64))> = Vec::with_capacity(m);
    for k in 0..m {
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        let nr = right_normal(dir(a, b));
        let u = (sgn * nr.0, sgn * nr.1);
        let pa = ctx.pt((a.x as f64) + (u.0 * r), (a.y as f64) + (u.1 * r))?;
        let pb = ctx.pt((b.x as f64) + (u.0 * r), (b.y as f64) + (u.1 * r))?;
        ends.push((pa, pb, u));
    }
    let mut out: Vec<P> = Vec::new();
    let joins = if closed { m } else { m - 1 };
    if !closed {
        out.push(ends[0].0);
    }
    for j in 0..joins {
        // Join between segment j and j + 1, at vertex pts[j + 1].
        let k0 = j;
        let k1 = (j + 1) % m;
        let v = pts[(j + 1) % n];
        let (a_dir, b_dir) = (dir(pts[k0], v), dir(v, pts[(k1 + 1) % n]));
        let turn = cross(a_dir, b_dir);
        let (e0, u0) = (ends[k0].1, ends[k0].2);
        let (e1, u1) = (ends[k1].0, ends[k1].2);
        out.push(e0);
        let straight = turn == 0 && dot(a_dir, b_dir) > 0;
        let gap =
            (turn > 0 && d > 0.0) || (turn < 0 && d < 0.0) || (turn == 0 && dot(a_dir, b_dir) < 0);
        if straight || r == 0.0 {
            // Nothing to join.
        } else if !gap {
            out.push(v);
        } else {
            match join {
                Join::Bevel => {}
                Join::Round => {
                    arc(ctx, &mut out, v, r, u0, u1, d > 0.0, e1)?;
                    continue;
                }
                Join::Mitre(limit) => {
                    let s = (u0.0 + u1.0, u0.1 + u1.1);
                    let s2 = (s.0 * s.0) + (s.1 * s.1);
                    // Ratio 2 / |u0 + u1|, compared squared: ratio > limit when 4 > limit² s2.
                    if 4.0 <= (limit * limit) * s2 {
                        let k = (2.0 * r) / s2;
                        out.push(ctx.pt((v.x as f64) + (s.0 * k), (v.y as f64) + (s.1 * k))?);
                    } else {
                        let t0 = unit(a_dir);
                        let t1 = unit(b_dir);
                        let sl = s2.sqrt();
                        let (cos_h, b) = if sl > 0.0 {
                            (sl / 2.0, (s.0 / sl, s.1 / sl))
                        } else {
                            (0.0, t0)
                        };
                        let sin_h = (t0.0 * b.0) + (t0.1 * b.1);
                        let x = (r * (limit - cos_h)) / sin_h;
                        let p0 = ctx.pt(
                            (v.x as f64) + (u0.0 * r) + (t0.0 * x),
                            (v.y as f64) + (u0.1 * r) + (t0.1 * x),
                        )?;
                        let p1 = ctx.pt(
                            (v.x as f64) + (u1.0 * r) - (t1.0 * x),
                            (v.y as f64) + (u1.1 * r) - (t1.1 * x),
                        )?;
                        out.push(p0);
                        out.push(p1);
                    }
                }
            }
        }
        out.push(e1);
    }
    if !closed {
        out.push(ends[m - 1].1);
    }
    Ok(out)
}

/// The raw offset chain of an open path, `r` mm to the right of travel, round joins (3.5).
pub fn raw_right(source: &str, pts: &[P], r: f64) -> Result<Vec<P>, GeomDiag> {
    raw_offset(&Ctx { source }, pts, false, r, Join::Round)
}

fn positive(rings: Vec<Vec<P>>) -> Result<Region, GeomDiag> {
    let ov = wind(&[rings])?;
    let sel = ov.select(&|w| w[0] > 0);
    Ok(ov.region(&sel))
}

/// `offset` (3.4): grows by `d` mm when positive, insets when negative.
pub fn offset(
    source: &str,
    region: &Region,
    d: i64,
    join: Join,
) -> Result<(Region, Vec<GeomDiag>), GeomDiag> {
    if let Join::Mitre(l) = join {
        if l.is_nan() || l < 1.0 {
            return Err(
                GeomDiag::new(Code::BadParameter, "the mitre limit must be at least 1")
                    .source(source)
                    .with("parameter", "limit"),
            );
        }
    }
    if d == 0 || region.is_empty() {
        return Ok((canonical(region.clone()), Vec::new()));
    }
    let ctx = Ctx { source };
    let mut raws = Vec::new();
    for ring in region.rings() {
        raws.push(raw_offset(&ctx, ring, true, d as f64, join)?);
    }
    let out = positive(raws)?;
    let mut diags = Vec::new();
    if out.is_empty() {
        diags.push(GeomDiag::new(Code::OffsetEmpty, "the inset left nothing").source(source));
    } else if out.pieces.len() > region.pieces.len() && d < 0 {
        diags.push(
            GeomDiag::new(
                Code::OffsetSplit,
                format!("the inset split into {} pieces", out.pieces.len()),
            )
            .source(source)
            .with("count", out.pieces.len() as i64),
        );
    }
    Ok((out, diags))
}

/// `stroke` (3.4): the region swept by a path, `left` and `right` mm to each side.
pub fn stroke(
    source: &str,
    path: &[P],
    left: i64,
    right: i64,
    join: Join,
    cap: Cap,
) -> Result<Region, GeomDiag> {
    if left < 0 || right < 0 || (left == 0 && right == 0) {
        return Err(GeomDiag::new(
            Code::BadParameter,
            "stroke widths must be at least 0 and not both 0",
        )
        .source(source)
        .with("parameter", if left < 0 { "left" } else { "right" }));
    }
    if path.len() < 2 {
        return Err(GeomDiag::new(Code::NoLength, "a stroke needs a path").source(source));
    }
    let ctx = Ctx { source };
    let rev: Vec<P> = path.iter().rev().copied().collect();
    let right_side = if right > 0 {
        raw_offset(&ctx, path, false, right as f64, join)?
    } else {
        path.to_vec()
    };
    let left_side = if left > 0 {
        raw_offset(&ctx, &rev, false, left as f64, join)?
    } else {
        rev.clone()
    };
    let mut ring: Vec<P> = right_side.clone();
    let n = path.len();
    let end = path[n - 1];
    let start = path[0];
    if cap == Cap::Round {
        let t_end = unit(dir(path[n - 2], end));
        let nr = (t_end.1, -t_end.0);
        let nl = (-t_end.1, t_end.0);
        if right > 0 {
            let fwd_pt = ctx.pt(
                (end.x as f64) + (t_end.0 * (right as f64)),
                (end.y as f64) + (t_end.1 * (right as f64)),
            )?;
            arc(&ctx, &mut ring, end, right as f64, nr, t_end, true, fwd_pt)?;
        }
        if left > 0 {
            if right == 0 || left != right {
                let fwd_pt = ctx.pt(
                    (end.x as f64) + (t_end.0 * (left as f64)),
                    (end.y as f64) + (t_end.1 * (left as f64)),
                )?;
                ring.push(fwd_pt);
            }
            arc(
                &ctx,
                &mut ring,
                end,
                left as f64,
                t_end,
                nl,
                true,
                left_side[0],
            )?;
        }
    }
    ring.extend(left_side.iter().copied());
    if cap == Cap::Round {
        let t_st = unit(dir(start, path[1]));
        let back = (-t_st.0, -t_st.1);
        let nr = (t_st.1, -t_st.0);
        let nl = (-t_st.1, t_st.0);
        if left > 0 {
            let back_pt = ctx.pt(
                (start.x as f64) + (back.0 * (left as f64)),
                (start.y as f64) + (back.1 * (left as f64)),
            )?;
            arc(&ctx, &mut ring, start, left as f64, nl, back, true, back_pt)?;
        }
        if right > 0 {
            if left == 0 || left != right {
                let back_pt = ctx.pt(
                    (start.x as f64) + (back.0 * (right as f64)),
                    (start.y as f64) + (back.1 * (right as f64)),
                )?;
                ring.push(back_pt);
            }
            arc(
                &ctx,
                &mut ring,
                start,
                right as f64,
                back,
                nr,
                true,
                right_side[0],
            )?;
            ring.pop(); // the ring closes on its first point
        }
    }
    positive(vec![ring])
}
