//! Exact predicates on the millimetre lattice (`spec/geometry.md` 1).
//!
//! Every decision here is made in integers: 128-bit for orientation, crossings and squared
//! point distances, and a small fixed-width unsigned integer for the comparisons the chapter
//! gives 256 bits. No float enters a predicate.

use crate::shape::P;
use std::cmp::Ordering;

/// `cross(b - a, c - a)`: positive when `a, b, c` turn left.
pub fn orient(a: P, b: P, c: P) -> i128 {
    let (ux, uy) = (i128::from(b.x - a.x), i128::from(b.y - a.y));
    let (vx, vy) = (i128::from(c.x - a.x), i128::from(c.y - a.y));
    ux * vy - uy * vx
}

/// `cross(u, v)` of two integer directions.
pub fn cross(u: (i64, i64), v: (i64, i64)) -> i128 {
    i128::from(u.0) * i128::from(v.1) - i128::from(u.1) * i128::from(v.0)
}

/// `dot(u, v)` of two integer directions.
pub fn dot(u: (i64, i64), v: (i64, i64)) -> i128 {
    i128::from(u.0) * i128::from(v.0) + i128::from(u.1) * i128::from(v.1)
}

/// The direction from `a` to `b`.
pub fn dir(a: P, b: P) -> (i64, i64) {
    (b.x - a.x, b.y - a.y)
}

/// Twice the signed area of a ring (shoelace), exact. Positive for counter-clockwise.
pub fn twice_area(ring: &[P]) -> i128 {
    let n = ring.len();
    let mut s: i128 = 0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y);
    }
    s
}

/// True when `p` lies on the closed segment `a b`.
pub fn on_segment(p: P, a: P, b: P) -> bool {
    orient(a, b, p) == 0
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
}

/// Where a point lies against a ring or a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Strictly inside.
    In,
    /// On the outline.
    On,
    /// Strictly outside.
    Out,
}

/// The winding number of `ring` about `p`, or `None` when `p` is on the ring. Points are given
/// scaled by `k` against the ring (so a segment midpoint is tested with `k = 2`).
pub fn winding_scaled(p: (i128, i128), k: i128, ring: &[P]) -> Option<i32> {
    let n = ring.len();
    winding_of_edges(p, k, (0..n).map(|i| (ring[i], ring[(i + 1) % n])))
}

/// [`winding_scaled`] summed over a set of directed edges, or `None` on one of them. An edge whose
/// `y` span leaves out the point's adds nothing, so a caller may pass only the edges that span it.
pub fn winding_of_edges(
    p: (i128, i128),
    k: i128,
    edges: impl Iterator<Item = (P, P)>,
) -> Option<i32> {
    let mut w = 0i32;
    for (a, b) in edges {
        let (ax, ay) = (i128::from(a.x) * k, i128::from(a.y) * k);
        let (bx, by) = (i128::from(b.x) * k, i128::from(b.y) * k);
        let o = (bx - ax) * (p.1 - ay) - (by - ay) * (p.0 - ax);
        if o == 0
            && p.0 >= ax.min(bx)
            && p.0 <= ax.max(bx)
            && p.1 >= ay.min(by)
            && p.1 <= ay.max(by)
        {
            return None;
        }
        if ay <= p.1 {
            if by > p.1 && o > 0 {
                w += 1;
            }
        } else if by <= p.1 && o < 0 {
            w -= 1;
        }
    }
    Some(w)
}

/// The winding number of `ring` about the lattice point `p`, or `None` on the ring.
pub fn winding(p: P, ring: &[P]) -> Option<i32> {
    winding_scaled((i128::from(p.x), i128::from(p.y)), 1, ring)
}

/// Rounds `n / d` to the nearest integer, ties to even. `d` is not zero.
pub fn div_round_even(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let q = n.div_euclid(d);
    let r = n.rem_euclid(d);
    match (2 * r).cmp(&d) {
        Ordering::Less => q,
        Ordering::Greater => q + 1,
        Ordering::Equal => {
            if q % 2 == 0 {
                q
            } else {
                q + 1
            }
        }
    }
}

/// The lattice point whose hot pixel holds `n / d` (3.1: `[x - 0.5, x + 0.5)`): nearest, a tie
/// going up. `d` is not zero.
pub fn div_round_pixel(n: i128, d: i128) -> i128 {
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    (2 * n + d).div_euclid(2 * d)
}

/// How two closed segments meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meet {
    /// They do not meet.
    None,
    /// They meet in one point (rounded to the lattice, ties to even, per axis).
    Point(P),
    /// They are collinear and overlap along a stretch of positive length.
    Overlap,
}

/// Whether segments `a b` and `c d` meet, and where. The meeting point of a proper crossing is
/// the exact rational, rounded to the lattice point whose hot pixel holds it (`geometry.md` 1,
/// Crossings, and 3.1).
pub fn meet(a: P, b: P, c: P, d: P) -> Meet {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    if d1 == 0 && d2 == 0 {
        // Collinear: overlap, a shared end, or apart.
        let along = |p: P| -> i128 {
            let u = dir(a, b);
            dot(u, dir(a, p))
        };
        let (s0, s1) = (0i128, along(b));
        let (t0, t1) = {
            let (x, y) = (along(c), along(d));
            if x <= y {
                (x, y)
            } else {
                (y, x)
            }
        };
        let lo = s0.max(t0);
        let hi = s1.min(t1);
        return match lo.cmp(&hi) {
            Ordering::Greater => Meet::None,
            Ordering::Equal => {
                // One shared point: an end of one of them.
                for p in [a, b, c, d] {
                    if along(p) == lo && on_segment(p, a, b) && on_segment(p, c, d) {
                        return Meet::Point(p);
                    }
                }
                Meet::None
            }
            Ordering::Less => Meet::Overlap,
        };
    }
    if (d1 > 0 && d2 > 0) || (d1 < 0 && d2 < 0) || (d3 > 0 && d4 > 0) || (d3 < 0 && d4 < 0) {
        return Meet::None;
    }
    if d1 == 0 {
        return Meet::Point(a);
    }
    if d2 == 0 {
        return Meet::Point(b);
    }
    if d3 == 0 {
        return Meet::Point(c);
    }
    if d4 == 0 {
        return Meet::Point(d);
    }
    // Proper crossing: a + (b - a) * num / den.
    let num = cross(dir(a, c), dir(c, d));
    let den = cross(dir(a, b), dir(c, d));
    let (ux, uy) = (i128::from(b.x - a.x), i128::from(b.y - a.y));
    let x = div_round_pixel(i128::from(a.x) * den + ux * num, den);
    let y = div_round_pixel(i128::from(a.y) * den + uy * num, den);
    Meet::Point(P::new(x as i64, y as i64))
}

/// A fixed-width unsigned integer of 384 bits, enough for the products of 1, Predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wide([u64; 6]);

impl Wide {
    /// From a `u128`.
    pub fn from_u128(v: u128) -> Wide {
        Wide([v as u64, (v >> 64) as u64, 0, 0, 0, 0])
    }

    /// The product, which must fit in 384 bits (it always does for the chapter's widths).
    pub fn mul_wide(self, o: Wide) -> Wide {
        // Only the significant limbs: a zero limb adds nothing to the schoolbook sum, so the
        // product is the same, and most operands hold two limbs of six.
        let used = |w: &Wide| w.0.iter().rposition(|&l| l != 0).map_or(0, |i| i + 1);
        let (ni, nj) = (used(&self), used(&o));
        let mut r = [0u64; 12];
        for i in 0..ni {
            let mut carry: u128 = 0;
            for j in 0..nj {
                if i + j >= 12 {
                    break;
                }
                let cur = u128::from(r[i + j]) + u128::from(self.0[i]) * u128::from(o.0[j]) + carry;
                r[i + j] = cur as u64;
                carry = cur >> 64;
            }
            let mut k = i + nj;
            while carry != 0 && k < 12 {
                let cur = u128::from(r[k]) + carry;
                r[k] = cur as u64;
                carry = cur >> 64;
                k += 1;
            }
        }
        let mut out = [0u64; 6];
        out.copy_from_slice(&r[..6]);
        Wide(out)
    }
}

impl PartialOrd for Wide {
    fn partial_cmp(&self, other: &Wide) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Wide {
    fn cmp(&self, other: &Wide) -> Ordering {
        for i in (0..6).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => {}
                o => return o,
            }
        }
        Ordering::Equal
    }
}

/// A squared distance held exactly as `num / den` (`den > 0`).
#[derive(Clone, Copy, Debug)]
pub struct Dist2 {
    /// Numerator (a squared cross product, or a squared length with `den = 1`).
    pub num: Wide,
    /// Denominator.
    pub den: Wide,
}

impl Dist2 {
    /// Exact comparison by cross-multiplication.
    pub fn cmp_exact(&self, o: &Dist2) -> Ordering {
        self.num.mul_wide(o.den).cmp(&o.num.mul_wide(self.den))
    }

    /// The least integer at or above `num / den`, when both fit in 127 bits; else `None`.
    pub fn ceil(&self) -> Option<i128> {
        let small = |w: &Wide| {
            (w.0[2..].iter().all(|&l| l == 0) && w.0[1] >> 63 == 0)
                .then(|| i128::from(w.0[0]) | (i128::from(w.0[1]) << 64))
        };
        let (n, d) = (small(&self.num)?, small(&self.den)?);
        (d > 0).then(|| n / d + i128::from(n % d != 0))
    }
}

/// Where on a segment the nearest point to `p` lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foot {
    /// The start vertex.
    Start,
    /// The end vertex.
    End,
    /// The interior.
    Interior,
}

/// The exact squared distance from `p` to segment `a b`, and where the nearest point lies
/// (`numbers.md` 5.5, The nearest point).
pub fn dist2_segment(p: P, a: P, b: P) -> (Dist2, Foot) {
    let u = dir(a, b);
    let w = dir(a, p);
    let t = dot(u, w);
    let uu = dot(u, u);
    if t <= 0 {
        let v = dot(w, w) as u128;
        return (
            Dist2 {
                num: Wide::from_u128(v),
                den: Wide::from_u128(1),
            },
            Foot::Start,
        );
    }
    if t >= uu {
        let w2 = dir(b, p);
        let v = dot(w2, w2) as u128;
        return (
            Dist2 {
                num: Wide::from_u128(v),
                den: Wide::from_u128(1),
            },
            Foot::End,
        );
    }
    let c = cross(u, w).unsigned_abs();
    let cw = Wide::from_u128(c);
    (
        Dist2 {
            num: cw.mul_wide(cw),
            den: Wide::from_u128(uu as u128),
        },
        Foot::Interior,
    )
}

/// The exact squared distance from `p` to the outline of a set of rings.
pub fn dist2_rings(p: P, rings: &[&[P]]) -> Option<Dist2> {
    let mut best: Option<Dist2> = None;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (d, _) = dist2_segment(p, ring[i], ring[(i + 1) % n]);
            if best
                .as_ref()
                .is_none_or(|b| d.cmp_exact(b) == Ordering::Less)
            {
                best = Some(d);
            }
        }
    }
    best
}

/// Exact angular order of directions, counter-clockwise from the positive `x` axis.
pub fn angle_cmp(u: (i64, i64), v: (i64, i64)) -> Ordering {
    fn half(d: (i64, i64)) -> u8 {
        if d.1 > 0 || (d.1 == 0 && d.0 > 0) {
            0
        } else {
            1
        }
    }
    let (hu, hv) = (half(u), half(v));
    if hu != hv {
        return hu.cmp(&hv);
    }
    // Same half-plane: u before v when v is to the left of u.
    0.cmp(&cross(u, v))
}
