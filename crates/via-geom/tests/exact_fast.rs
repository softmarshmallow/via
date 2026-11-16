//! The kernel's fast paths give the exact answers of the plain ones (`geometry.md` 1, Predicates:
//! decided exactly, no tolerance): the wide product over significant limbs, the nearest place with
//! its bound taken first, and the winding number from the edges that span a row.

use std::cmp::Ordering;
use via_geom::lineref::{nearest, nearest_among, window_segments, Line};
use via_geom::pred::{self, dist2_segment, winding_of_edges, Side, Wide};
use via_geom::{Piece, Region, P};

/// A fixed linear congruential sequence, so a failure is the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo) as u64)) as i64
    }
}

#[test]
fn wide_product_matches_u128_and_associates() {
    let mut r = Lcg(7);
    for _ in 0..2000 {
        let (a, b) = (r.next() >> (r.next() % 50), r.next() >> (r.next() % 50));
        let p = Wide::from_u128(u128::from(a)).mul_wide(Wide::from_u128(u128::from(b)));
        assert_eq!(p, Wide::from_u128(u128::from(a) * u128::from(b)));
        // Up to three limbs each; the products reach five, so a carry put at the wrong limb
        // breaks the identities.
        let w = |r: &mut Lcg| {
            let hi = u128::from(r.next()) << (r.next() % 64);
            Wide::from_u128(hi | u128::from(r.next()))
        };
        let (x, y, z) = (w(&mut r), w(&mut r), w(&mut r));
        assert_eq!(x.mul_wide(y).mul_wide(z), x.mul_wide(y.mul_wide(z)));
        assert_eq!(x.mul_wide(y), y.mul_wide(x));
        let y1 = Wide::from_u128(1).mul_wide(y);
        assert_eq!(y1, y);
        assert_eq!(x.mul_wide(Wide::from_u128(0)), Wide::from_u128(0));
    }
    // The largest square of two limbs: (2^128 - 1)^2 = 2^256 - 2^129 + 1 > 2^255.
    let m = Wide::from_u128(u128::MAX);
    let sq = m.mul_wide(m);
    assert!(sq > Wide::from_u128(u128::MAX).mul_wide(Wide::from_u128(1u128 << 127)));
}

#[test]
fn ceil_bounds_the_exact_distance() {
    let mut r = Lcg(11);
    for _ in 0..2000 {
        let p = P::new(r.range(-600_000, 600_000), r.range(-600_000, 600_000));
        let a = P::new(r.range(-600_000, 600_000), r.range(-600_000, 600_000));
        let b = P::new(r.range(-600_000, 600_000), r.range(-600_000, 600_000));
        let (d, _) = dist2_segment(p, a, b);
        let c = d.ceil().expect("lattice distances fit");
        let at = |v: i128| pred::Dist2 {
            num: Wide::from_u128(v as u128),
            den: Wide::from_u128(1),
        };
        assert_ne!(at(c).cmp_exact(&d), Ordering::Less, "ceil below");
        assert_eq!(at(c - 1).cmp_exact(&d), Ordering::Less, "ceil not least");
    }
}

#[test]
fn nearest_equals_the_plain_scan() {
    let mut r = Lcg(3);
    for case in 0..300 {
        let n = 2 + (r.next() % if case % 2 == 0 { 30 } else { 400 }) as usize;
        let mut pts = vec![P::new(r.range(0, 400_000), r.range(0, 400_000))];
        while pts.len() < n {
            let last = *pts.last().unwrap();
            let q = P::new(
                last.x + r.range(-9_000, 9_000),
                last.y + r.range(-9_000, 9_000),
            );
            if q != last {
                pts.push(q);
            }
        }
        let closed = case % 3 == 0 && n >= 3;
        let line = Line::new(pts, closed);
        for _ in 0..40 {
            let p = P::new(r.range(-50_000, 450_000), r.range(-50_000, 450_000));
            let fast = nearest(&line, p);
            // The plain scan: every segment tested exactly, ties to the lower index.
            let mut best: Option<(usize, pred::Foot, pred::Dist2)> = None;
            for k in 0..line.segments() {
                let (a, b) = line.seg(k);
                let (d, f) = dist2_segment(p, a, b);
                if best
                    .as_ref()
                    .is_none_or(|(_, _, bd)| d.cmp_exact(bd) == Ordering::Less)
                {
                    best = Some((k, f, d));
                }
            }
            let (k, f, d) = best.unwrap();
            assert_eq!((fast.k, fast.foot), (k, f), "case {case} at {p:?}");
            assert_eq!(fast.dist.cmp_exact(&d), Ordering::Equal);
        }
    }
}

/// A window's segments (`geometry.md` 3.8): for every post of a window, near or far from the
/// line, inside its box or not, the nearest among them is the nearest of the whole line,
/// segment, foot and exact distance, ties to the lower index. Collinear and repeated runs make
/// ties.
#[test]
fn window_segments_keep_the_nearest() {
    let mut r = Lcg(11);
    let mut pruned = 0usize;
    for case in 0..200 {
        let n = 2 + (r.next() % if case % 2 == 0 { 30 } else { 400 }) as usize;
        let mut pts = vec![P::new(r.range(0, 400_000), r.range(0, 400_000))];
        while pts.len() < n {
            let last = *pts.last().unwrap();
            // Every fourth case on a 1 m lattice of short steps: equal distances are common.
            let q = if case % 4 == 1 {
                P::new(
                    last.x + 1_000 * r.range(-3, 3),
                    last.y + 1_000 * r.range(-3, 3),
                )
            } else {
                P::new(
                    last.x + r.range(-9_000, 9_000),
                    last.y + r.range(-9_000, 9_000),
                )
            };
            if q != last {
                pts.push(q);
            }
        }
        let closed = case % 3 == 0 && n >= 3;
        let line = Line::new(pts, closed);
        for _ in 0..6 {
            let (x0, y0) = (r.range(-100_000, 450_000), r.range(-100_000, 450_000));
            let side = r.range(0, 64) * 2_000;
            let (lo, hi) = (P::new(x0, y0), P::new(x0 + side, y0 + side));
            let ks = window_segments(&line, lo, hi);
            pruned += line.segments() - ks.len();
            let posts = [lo, hi, P::new(lo.x, hi.y), P::new(hi.x, lo.y)]
                .into_iter()
                .chain((0..12).map(|_| P::new(r.range(lo.x, hi.x + 1), r.range(lo.y, hi.y + 1))));
            for p in posts {
                let (want, got) = (nearest(&line, p), nearest_among(&line, &ks, p));
                assert_eq!(
                    (got.k, got.foot),
                    (want.k, want.foot),
                    "case {case} at {p:?}"
                );
                assert_eq!(got.dist.cmp_exact(&want.dist), Ordering::Equal);
            }
        }
    }
    assert!(pruned > 0, "the windows leave segments out");
}

#[test]
fn row_edges_give_the_regions_side() {
    // A ring with teeth and a hole, and posts on rows through vertices, edges and holes.
    let mut shell = vec![P::new(0, 0), P::new(60_000, 0), P::new(60_000, 40_000)];
    for k in (0..5).rev() {
        let x = 10_000 * k + 10_000;
        shell.extend([
            P::new(x + 4_000, 40_000),
            P::new(x + 4_000, 10_000),
            P::new(x, 10_000),
            P::new(x, 40_000),
        ]);
    }
    shell.push(P::new(0, 40_000));
    let hole = vec![
        P::new(2_000, 2_000),
        P::new(2_000, 6_000),
        P::new(8_000, 6_000),
        P::new(8_000, 2_000),
    ];
    let region = Region {
        pieces: vec![Piece {
            shell,
            holes: vec![hole],
        }],
    };
    let rings = region.rings();
    let segs: Vec<(P, P)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    for y in (-1_000..=41_000).step_by(500) {
        let row: Vec<(P, P)> = segs
            .iter()
            .filter(|(a, b)| a.y.min(b.y) <= y && y <= a.y.max(b.y))
            .copied()
            .collect();
        for x in (-1_000..=61_000).step_by(500) {
            let p = P::new(x, y);
            let side =
                match winding_of_edges((i128::from(x), i128::from(y)), 1, row.iter().copied()) {
                    None => Side::On,
                    Some(0) => Side::Out,
                    Some(_) => Side::In,
                };
            assert_eq!(side, region.side(p), "at {p:?}");
        }
    }
}
