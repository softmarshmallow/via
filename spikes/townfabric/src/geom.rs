//! Plane geometry in metres. Everything is f64 in a local frame whose
//! origin is the settlement site; nothing here knows about the 200 m
//! terrain grid.

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
#[inline]
pub fn rot(a: P2, ang: f64) -> P2 {
    let (s, c) = ang.sin_cos();
    [a[0] * c - a[1] * s, a[0] * s + a[1] * c]
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

pub fn point_in_polygon(p: P2, poly: &[P2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let j = (i + n - 1) % n;
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0];
            if p[0] < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Oriented bounding box: (centre, axis u, axis v, half-extent along u,
/// half-extent along v), with |u| = |v| = 1 and u the long axis. Rotating
/// calipers over the convex hull would be exact; this samples orientations,
/// which is enough to pick a split axis.
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

/// Clip a convex-or-simple polygon by the half-plane dot(p − o, n) ≤ 0
/// (Sutherland–Hodgman; exact for convex input, adequate for the blocks
/// this spike splits).
pub fn clip_halfplane(poly: &[P2], o: P2, n: P2) -> Vec<P2> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    let m = poly.len();
    for i in 0..m {
        let a = poly[i];
        let b = poly[(i + 1) % m];
        let da = dot(sub(a, o), n);
        let db = dot(sub(b, o), n);
        if da <= 0.0 {
            out.push(a);
        }
        if (da <= 0.0) != (db <= 0.0) {
            let t = da / (da - db);
            out.push(add(a, mul(sub(b, a), t)));
        }
    }
    out
}

/// Axis-aligned-in-OBB-frame rectangle inset by per-side margins, returned
/// as a polygon in world coordinates. Buildings are rectangles here: a true
/// polygon offset of an irregular parcel is a straight-skeleton problem the
/// spike does not need.
pub fn inset_rect(centre: P2, u: P2, v: P2, hu: f64, hv: f64, mu: f64, mv: f64) -> Option<Vec<P2>> {
    let (a, b) = (hu - mu, hv - mv);
    if a <= 0.2 || b <= 0.2 {
        return None;
    }
    Some(vec![
        add(centre, add(mul(u, a), mul(v, b))),
        add(centre, add(mul(u, -a), mul(v, b))),
        add(centre, add(mul(u, -a), mul(v, -b))),
        add(centre, add(mul(u, a), mul(v, -b))),
    ])
}
