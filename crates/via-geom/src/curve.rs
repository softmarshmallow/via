//! `smooth = true`: a centripetal Catmull-Rom curve through a path's written points, sampled to
//! a lattice polyline (`spec/geometry.md` 2.4; Yuksel, Schaefer and Keyser 2011).

use crate::diag::GeomDiag;
use crate::lineref::round;
use crate::pred::{dir, dot};
use crate::shape::P;

/// The sampling step of 2.4, millimetres.
pub const STEP_MM: f64 = 250.0;

fn knot(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    // Centripetal: |b - a| ^ 0.5.
    ((dx * dx) + (dy * dy)).sqrt().sqrt()
}

fn lerp(a: (f64, f64), b: (f64, f64), ta: f64, tb: f64, t: f64) -> (f64, f64) {
    let span = tb - ta;
    if span == 0.0 {
        return a;
    }
    let wa = (tb - t) / span;
    let wb = (t - ta) / span;
    ((wa * a.0) + (wb * b.0), (wa * a.1) + (wb * b.1))
}

/// The smoothed polyline of a written path (2.4). Written points are kept exactly; a path of
/// two points stays straight.
pub fn smooth(source: &str, pts: &[P]) -> Result<Vec<P>, GeomDiag> {
    let n = pts.len();
    if n < 3 {
        return Ok(pts.to_vec());
    }
    let f = |p: P| (p.x as f64, p.y as f64);
    let mut ext: Vec<(f64, f64)> = Vec::with_capacity(n + 2);
    let (p0, p1) = (pts[0], pts[1]);
    ext.push((
        (2.0 * (p0.x as f64)) - (p1.x as f64),
        (2.0 * (p0.y as f64)) - (p1.y as f64),
    ));
    ext.extend(pts.iter().map(|&p| f(p)));
    let (pa, pb) = (pts[n - 2], pts[n - 1]);
    ext.push((
        (2.0 * (pb.x as f64)) - (pa.x as f64),
        (2.0 * (pb.y as f64)) - (pa.y as f64),
    ));
    let mut out: Vec<P> = vec![pts[0]];
    for i in 0..(n - 1) {
        let (q0, q1, q2, q3) = (ext[i], ext[i + 1], ext[i + 2], ext[i + 3]);
        let t0 = 0.0;
        let t1 = t0 + knot(q0, q1);
        let t2 = t1 + knot(q1, q2);
        let t3 = t2 + knot(q2, q3);
        let u = dir(pts[i], pts[i + 1]);
        let c = (dot(u, u) as f64).sqrt();
        let steps = (c / STEP_MM).ceil();
        let steps = if steps < 1.0 { 1.0 } else { steps };
        let m = steps as i64;
        for s in 1..m {
            let t = t1 + ((t2 - t1) * (s as f64)) / steps;
            let a1 = lerp(q0, q1, t0, t1, t);
            let a2 = lerp(q1, q2, t1, t2, t);
            let a3 = lerp(q2, q3, t2, t3, t);
            let b1 = lerp(a1, a2, t0, t2, t);
            let b2 = lerp(a2, a3, t1, t3, t);
            let c = lerp(b1, b2, t1, t2, t);
            let p = P::new(round(source, c.0)?, round(source, c.1)?);
            if out.last() != Some(&p) {
                out.push(p);
            }
        }
        if out.last() != Some(&pts[i + 1]) {
            out.push(pts[i + 1]);
        }
    }
    Ok(out)
}
