//! Plane geometry in metres. Everything is f64 in a local frame whose
//! origin is the study centre. Ported from spikes/townfabric (07a95b8);
//! generator-only helpers (half-plane clip, inset rectangles) were left
//! behind.

pub type P2 = [f64; 2];

#[inline]
pub fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
#[inline]
pub fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
#[inline]
pub fn mul(a: P2, k: f64) -> P2 {
    [a[0] * k, a[1] * k]
}
#[inline]
pub fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
#[inline]
pub fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
#[inline]
pub fn len(a: P2) -> f64 {
    dot(a, a).sqrt()
}
#[inline]
pub fn dist(a: P2, b: P2) -> f64 {
    len(sub(a, b))
}
#[inline]
pub fn norm(a: P2) -> P2 {
    let l = len(a);
    if l > 1.0e-12 {
        mul(a, 1.0 / l)
    } else {
        [0.0, 0.0]
    }
}
#[inline]
pub fn perp(a: P2) -> P2 {
    [-a[1], a[0]]
}

/// Closest point to `p` on segment ab, and its parameter t ∈ [0, 1].
pub fn closest_on_segment(p: P2, a: P2, b: P2) -> (P2, f64) {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    if l2 < 1.0e-18 {
        return (a, 0.0);
    }
    let t = (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0);
    (add(a, mul(ab, t)), t)
}

/// Proper intersection of segments ab and cd. Returns (point, t_ab, t_cd).
/// Endpoint-touching and collinear overlaps return None: those are handled
/// by snapping, not by splitting.
pub fn segment_intersection(a: P2, b: P2, c: P2, d: P2) -> Option<(P2, f64, f64)> {
    let r = sub(b, a);
    let s = sub(d, c);
    let denom = cross(r, s);
    if denom.abs() < 1.0e-12 {
        return None;
    }
    let t = cross(sub(c, a), s) / denom;
    let u = cross(sub(c, a), r) / denom;
    const E: f64 = 1.0e-9;
    if t > E && t < 1.0 - E && u > E && u < 1.0 - E {
        Some((add(a, mul(r, t)), t, u))
    } else {
        None
    }
}

pub fn polygon_area(poly: &[P2]) -> f64 {
    let mut s = 0.0;
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        s += cross(poly[i], poly[j]);
    }
    0.5 * s
}

pub fn polygon_centroid(poly: &[P2]) -> P2 {
    let a = polygon_area(poly);
    if a.abs() < 1.0e-9 {
        let n = poly.len().max(1) as f64;
        let mut c = [0.0, 0.0];
        for p in poly {
            c = add(c, *p);
        }
        return mul(c, 1.0 / n);
    }
    let mut c = [0.0, 0.0];
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        let w = cross(poly[i], poly[j]);
        c = add(c, mul(add(poly[i], poly[j]), w));
    }
    mul(c, 1.0 / (6.0 * a))
}

pub fn polygon_perimeter(poly: &[P2]) -> f64 {
    let mut s = 0.0;
    for i in 0..poly.len() {
        s += dist(poly[i], poly[(i + 1) % poly.len()]);
    }
    s
}

/// Oriented bounding box: (centre, axis u, axis v, half-extent along u,
/// half-extent along v), with |u| = |v| = 1 and u the long axis. Samples
/// 90 orientations, which is enough for the shape statistics built on it.
pub fn obb(poly: &[P2]) -> (P2, P2, P2, f64, f64) {
    let c = polygon_centroid(poly);
    let mut best = (f64::INFINITY, 0.0f64, 0.0f64, 0.0f64);
    const STEPS: u32 = 90;
    for k in 0..STEPS {
        let ang = std::f64::consts::PI * k as f64 / STEPS as f64;
        let u = [ang.cos(), ang.sin()];
        let v = perp(u);
        let (mut lo_u, mut hi_u) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut lo_v, mut hi_v) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in poly {
            let d = sub(*p, c);
            let (du, dv) = (dot(d, u), dot(d, v));
            lo_u = lo_u.min(du);
            hi_u = hi_u.max(du);
            lo_v = lo_v.min(dv);
            hi_v = hi_v.max(dv);
        }
        let area = (hi_u - lo_u) * (hi_v - lo_v);
        if area < best.0 {
            best = (area, ang, 0.5 * (hi_u + lo_u), 0.5 * (hi_v + lo_v));
        }
    }
    let ang = best.1;
    let u = [ang.cos(), ang.sin()];
    let v = perp(u);
    let centre = add(c, add(mul(u, best.2), mul(v, best.3)));
    let (mut hu, mut hv) = (0.0f64, 0.0f64);
    for p in poly {
        let d = sub(*p, centre);
        hu = hu.max(dot(d, u).abs());
        hv = hv.max(dot(d, v).abs());
    }
    if hu >= hv {
        (centre, u, v, hu, hv)
    } else {
        (centre, v, u, hv, hu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_square_area_centroid_perimeter() {
        let sq = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!((polygon_area(&sq) - 1.0).abs() < 1e-12);
        let c = polygon_centroid(&sq);
        assert!((c[0] - 0.5).abs() < 1e-12 && (c[1] - 0.5).abs() < 1e-12);
        assert!((polygon_perimeter(&sq) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn intersection_hits_interior_only() {
        // Proper crossing.
        let hit = segment_intersection([0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]);
        let (p, t, u) = hit.expect("crossing");
        assert!((p[0] - 1.0).abs() < 1e-9 && (p[1] - 1.0).abs() < 1e-9);
        assert!((t - 0.5).abs() < 1e-9 && (u - 0.5).abs() < 1e-9);
        // Endpoint touch is not a crossing (snapping handles it).
        assert!(segment_intersection([0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [1.0, 1.0]).is_none());
        // Parallel.
        assert!(segment_intersection([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]).is_none());
    }

    #[test]
    fn obb_of_axis_rectangle() {
        let r = [[0.0, 0.0], [4.0, 0.0], [4.0, 2.0], [0.0, 2.0]];
        let (_, _, _, hu, hv) = obb(&r);
        // Long half-axis 2, short half-axis 1 (within the 2° sampling).
        assert!((hu - 2.0).abs() < 0.05, "hu = {hu}");
        assert!((hv - 1.0).abs() < 0.05, "hv = {hv}");
    }
}
