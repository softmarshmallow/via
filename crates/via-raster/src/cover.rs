//! Rasterising shapes exactly: which posts lie inside a region or within a stroke of a path
//! (`numbers.md` 6, "Samples of a feature"; `geometry.md` 1, Predicates). Input is integer
//! millimetres; every decision is an integer predicate, never a tolerance (R24).
//!
//! A cover is a [`Raster<u8>`]: 1 at a post inside, 0 outside.

use crate::error::RasterError;
use crate::raster::Raster;
use crate::window::{post_mm, Window};
use via_num::Grid;

/// A lattice point in millimetres.
pub type Pt = (i64, i64);

/// The bounding box `[xmin, ymin, xmax, ymax]` of some points, or `None` for none.
pub fn bounds(points: impl IntoIterator<Item = Pt>) -> Option<[i64; 4]> {
    points.into_iter().fold(None, |acc, (x, y)| {
        Some(match acc {
            None => [x, y, x, y],
            Some([a, b, c, d]) => [a.min(x), b.min(y), c.max(x), d.max(y)],
        })
    })
}

/// `a * b` for unsigned 128-bit values, as a 256-bit `(high, low)` pair.
pub(crate) fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    let mask = u128::from(u64::MAX);
    let (a1, a0) = (a >> 64, a & mask);
    let (b1, b0) = (b >> 64, b & mask);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & mask) + (p10 & mask);
    let low = (p00 & mask) | (mid << 64);
    let high = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (high, low)
}

/// Twice the signed area of a ring (shoelace), exact in 128 bits; positive counter-clockwise.
pub fn twice_area(ring: &[Pt]) -> i128 {
    let n = ring.len();
    let mut s: i128 = 0;
    for k in 0..n {
        let (x0, y0) = ring[k];
        let (x1, y1) = ring[(k + 1) % n];
        s += i128::from(x0) * i128::from(y1) - i128::from(x1) * i128::from(y0);
    }
    s
}

/// Whether `p` lies on the closed segment `a`-`b`, exactly.
pub fn on_segment(a: Pt, b: Pt, p: Pt) -> bool {
    let (ux, uy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (wx, wy) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
    if ux * wy - uy * wx != 0 {
        return false;
    }
    let dot = ux * wx + uy * wy;
    (0..=ux * ux + uy * uy).contains(&dot)
}

/// Exact point in region (`numbers.md` 5.5, "The side of a region"): on any ring's outline is
/// inside; otherwise the non-zero winding number over every ring (shells counter-clockwise, holes
/// clockwise), so a hole is outside.
pub fn inside(rings: &[Vec<Pt>], p: Pt) -> bool {
    let mut wn: i64 = 0;
    for ring in rings {
        let n = ring.len();
        for k in 0..n {
            let a = ring[k];
            let b = ring[(k + 1) % n];
            if on_segment(a, b, p) {
                return true;
            }
            let side = i128::from(b.0 - a.0) * i128::from(p.1 - a.1)
                - i128::from(p.0 - a.0) * i128::from(b.1 - a.1);
            if a.1 <= p.1 {
                if b.1 > p.1 && side > 0 {
                    wn += 1;
                }
            } else if b.1 <= p.1 && side < 0 {
                wn -= 1;
            }
        }
    }
    wn != 0
}

/// A crossing of a row by an edge, as the exact rational `num / den`, `den > 0`, with the edge's
/// winding contribution to posts west of it.
#[derive(Clone, Copy)]
struct Crossing {
    num: i128,
    den: i128,
    wind: i64,
}

/// The cover of a region at every post of `window`: 1 inside, outline included, by the exact
/// winding test; holes outside. A scanline over each post row: the half-open rule decides which
/// edges cross a row (`ya <= y < yb`), each crossing is an exact rational, and a post exactly on an
/// edge is inside. Equal to [`inside`] at every post.
pub fn region_cover(grid: &Grid, window: Window, rings: &[Vec<Pt>]) -> Raster<u8> {
    let mut out = Raster::filled(window, 0u8);
    let Some([bx0, by0, bx1, by1]) = bounds(rings.iter().flatten().copied()) else {
        return out;
    };
    let edges: Vec<(Pt, Pt)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |k| (r[k], r[(k + 1) % r.len()])))
        .filter(|(a, b)| a != b)
        .collect();
    for j in window.j0..window.j1 {
        let (_, y) = post_mm(grid, window.i0, j);
        if y < by0 || y > by1 {
            continue;
        }
        let yy = i128::from(y);
        let mut crossings: Vec<Crossing> = Vec::new();
        // Exact boundary: horizontal edges on the row, and every edge's exact crossing point.
        let mut on_edge: Vec<(i128, i128, i128)> = Vec::new(); // (num, den) or a span
        for &(a, b) in &edges {
            let (ax, ay, bx, by) = (
                i128::from(a.0),
                i128::from(a.1),
                i128::from(b.0),
                i128::from(b.1),
            );
            if ay == by {
                if ay == yy {
                    // A span [min, max] of x on the row: stored as num = min, den = 0, third = max.
                    on_edge.push((ax.min(bx), 0, ax.max(bx)));
                }
                continue;
            }
            let (lo, hi) = if ay < by { (ay, by) } else { (by, ay) };
            if yy < lo || yy > hi {
                continue;
            }
            // x at the row: ax + (y - ay) * (bx - ax) / (by - ay), as num / den with den > 0.
            let mut num = ax * (by - ay) + (yy - ay) * (bx - ax);
            let mut den = by - ay;
            if den < 0 {
                num = -num;
                den = -den;
            }
            on_edge.push((num, den, 0));
            let half_open = if ay < by {
                ay <= yy && yy < by
            } else {
                by <= yy && yy < ay
            };
            if half_open {
                crossings.push(Crossing {
                    num,
                    den,
                    wind: if ay < by { 1 } else { -1 },
                });
            }
        }
        // Posts west of a crossing get its winding: sort crossings by x, sweep posts west to east.
        crossings.sort_by(|p, q| (p.num * q.den).cmp(&(q.num * p.den)));
        let total: i64 = crossings.iter().map(|c| c.wind).sum();
        let mut passed = 0usize;
        let mut wn_east = total;
        for i in window.i0..window.i1 {
            let (x, _) = post_mm(grid, i, j);
            if x < bx0 || x > bx1 {
                continue;
            }
            let xx = i128::from(x);
            // Crossings at or west of x no longer count.
            while passed < crossings.len() && crossings[passed].num <= xx * crossings[passed].den {
                wn_east -= crossings[passed].wind;
                passed += 1;
            }
            let boundary = on_edge.iter().any(|&(num, den, max)| {
                if den == 0 {
                    num <= xx && xx <= max
                } else {
                    num == xx * den
                }
            });
            if boundary || wn_east != 0 {
                out.set(i, j, 1);
            }
        }
    }
    out
}

/// How a stroke ends at the path's two ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    /// A half disc beyond each end.
    Round,
    /// Square at each end, no further.
    Butt,
}

/// Whether `p` is within `w` of the segment `a`-`b`, exactly, with the segment's own reach only
/// (its projection between the ends) when `ends` is false.
fn near_segment(a: Pt, b: Pt, p: Pt, w2: u128, ends: bool) -> bool {
    let (ux, uy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (wx, wy) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
    let uu = ux * ux + uy * uy;
    let dot = ux * wx + uy * wy;
    if dot < 0 || dot > uu {
        if !ends {
            return false;
        }
        let (vx, vy) = if dot < 0 {
            (wx, wy)
        } else {
            (i128::from(p.0 - b.0), i128::from(p.1 - b.1))
        };
        return ((vx * vx + vy * vy) as u128) <= w2;
    }
    // Interior: cross^2 / uu <= w^2, compared as cross^2 <= w^2 * uu in 256 bits.
    let cross = (ux * wy - uy * wx).unsigned_abs();
    mul_wide(cross, cross) <= mul_wide(w2, uu as u128)
}

/// The cover of a stroke of half-width `half_width` mm (`>= 0`) about a path: every post whose
/// exact distance to the path is at most `half_width` for `Cap::Round` (round joins and caps);
/// for `Cap::Butt`, within `half_width` of some segment's own span, or of an interior vertex
/// (round joins). Exact: distances are compared squared, in 256-bit integers where they need it.
///
/// This is the exact swept set, not `geometry.md` 3.4's polygon, whose arcs are chords within
/// 5 mm; a reader that must agree with a stroked region rasterises that region with
/// [`region_cover`].
pub fn stroke_cover(
    grid: &Grid,
    window: Window,
    path: &[Pt],
    half_width: i64,
    cap: Cap,
) -> Result<Raster<u8>, RasterError> {
    let mut out = Raster::filled(window, 0u8);
    if half_width < 0 {
        return Err(RasterError::ParameterRange {
            what: "stroke half-width",
            value: half_width as f64,
        });
    }
    let Some([x0, y0, x1, y1]) = bounds(path.iter().copied()) else {
        return Ok(out);
    };
    let ov = || RasterError::Overflow {
        op: "stroke bounds",
    };
    let grown = [
        x0.checked_sub(half_width).ok_or_else(ov)?,
        y0.checked_sub(half_width).ok_or_else(ov)?,
        x1.checked_add(half_width).ok_or_else(ov)?,
        y1.checked_add(half_width).ok_or_else(ov)?,
    ];
    let w2 = (i128::from(half_width) * i128::from(half_width)) as u128;
    let n = path.len();
    for (i, j) in window.posts() {
        let p = post_mm(grid, i, j);
        if p.0 < grown[0] || p.0 > grown[2] || p.1 < grown[1] || p.1 > grown[3] {
            continue;
        }
        let hit = if n == 1 {
            cap == Cap::Round && {
                let (dx, dy) = (i128::from(p.0 - path[0].0), i128::from(p.1 - path[0].1));
                ((dx * dx + dy * dy) as u128) <= w2
            }
        } else {
            (0..n - 1).any(|k| near_segment(path[k], path[k + 1], p, w2, cap == Cap::Round))
                || (cap == Cap::Butt
                    && (1..n - 1).any(|k| {
                        let (dx, dy) = (i128::from(p.0 - path[k].0), i128::from(p.1 - path[k].1));
                        ((dx * dx + dy * dy) as u128) <= w2
                    }))
        };
        if hit {
            out.set(i, j, 1);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(extent: [i64; 4], r: i64) -> Grid {
        Grid::new(extent, r).unwrap()
    }

    #[test]
    fn wide_multiply_is_exact() {
        assert_eq!(mul_wide(u128::MAX, u128::MAX), (u128::MAX - 1, 1));
        assert_eq!(mul_wide(1 << 70, 1 << 70), (1 << 12, 0));
        assert_eq!(mul_wide(12345, 678), (0, 12345 * 678));
    }

    #[test]
    fn region_cover_equals_the_winding_test() {
        let grid = g([0, 0, 20_000, 20_000], 500);
        // A concave shell with a hole and an outline through posts and between them.
        let shell = vec![
            (1_000, 1_000),
            (15_250, 2_000),
            (18_000, 17_000),
            (9_000, 9_000),
            (2_000, 18_000),
        ];
        let hole = vec![(5_000, 5_000), (5_000, 8_000), (8_000, 5_000)];
        let rings = vec![shell, hole];
        let w = Window::whole(&grid);
        let c = region_cover(&grid, w, &rings);
        let mut n = 0;
        for (i, j) in w.posts() {
            let want = inside(&rings, post_mm(&grid, i, j));
            assert_eq!(c.get(i, j) == Some(1), want, "post ({i}, {j})");
            n += usize::from(want);
        }
        assert!(n > 100);
        // The hole's outline is inside; its interior is not.
        assert_eq!(c.get(10, 10), Some(1));
        assert_eq!(c.get(12, 12), Some(0));
        // Windowed equals the cut.
        let part = Window::new(7, 3, 30, 22).unwrap();
        assert_eq!(region_cover(&grid, part, &rings), c.cut(part).unwrap());
    }

    #[test]
    fn region_samples_include_the_outline() {
        let grid = g([0, 0, 8_000, 8_000], 1_000);
        let square = vec![vec![
            (2_000, 2_000),
            (5_000, 2_000),
            (5_000, 5_000),
            (2_000, 5_000),
        ]];
        let c = region_cover(&grid, Window::whole(&grid), &square);
        assert_eq!(c.values().iter().filter(|&&v| v == 1).count(), 16);
    }

    #[test]
    fn stroke_is_the_exact_distance() {
        let grid = g([0, 0, 20_000, 20_000], 1_000);
        let path = vec![(2_000, 2_000), (12_000, 7_000), (12_000, 15_000)];
        let w = Window::whole(&grid);
        let round = stroke_cover(&grid, w, &path, 3_000, Cap::Round).unwrap();
        let butt = stroke_cover(&grid, w, &path, 3_000, Cap::Butt).unwrap();
        // Round: within 3 m of the path; check against doubles where they are far from the edge.
        for (i, j) in w.posts() {
            let (x, y) = post_mm(&grid, i, j);
            let d = (0..2)
                .map(|k| seg_dist(path[k], path[k + 1], (x, y)))
                .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
            if (d - 3_000.0).abs() > 1.0 {
                assert_eq!(round.get(i, j) == Some(1), d < 3_000.0, "({i}, {j}) {d}");
            }
        }
        // Exactly on the edge counts: (12, 15) + 3 m north.
        assert_eq!(round.get(12, 18), Some(1));
        assert_eq!(butt.get(12, 18), Some(0));
        assert_eq!(butt.get(12, 15), Some(1));
        // Butt keeps the round join at the interior vertex.
        assert_eq!(butt.get(14, 6), Some(1));
        let part = Window::new(5, 5, 14, 19).unwrap();
        assert_eq!(
            stroke_cover(&grid, part, &path, 3_000, Cap::Round).unwrap(),
            round.cut(part).unwrap()
        );
    }

    fn seg_dist(a: Pt, b: Pt, p: Pt) -> f64 {
        let (ux, uy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
        let (wx, wy) = ((p.0 - a.0) as f64, (p.1 - a.1) as f64);
        let t = via_num::clamp((ux * wx + uy * wy) / (ux * ux + uy * uy), 0.0, 1.0);
        let (dx, dy) = (wx - t * ux, wy - t * uy);
        (dx * dx + dy * dy).sqrt()
    }
}
