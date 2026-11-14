//! A found course: the least-cost way over the posts through written points (`derived.md` 3).
//!
//! The cost is a formula's value per metre of travel at each post, evaluated by the caller. Each
//! post joins its eight neighbours; a step's weight is `0.5 * (c(a) + c(b)) * len`, `len` the
//! resolution in metres or that times `sqrt(2)`, in doubles, in that order. Between each two
//! written points in turn the course is found by Dijkstra's search (Dijkstra 1959), with a queue
//! ordered by distance then post order, each post settled once, and a distance changed only on a
//! strictly smaller value, so equal courses are told apart the same way on every system.
//!
//! The posts of each leg are then simplified (Douglas and Peucker 1973) to within half a post
//! spacing, so the eight-neighbour staircase becomes the few posts where the course turns; each
//! written point takes the place of its nearest post, and a post that would make the course
//! double back, or stand closer than half a spacing to its neighbour, is dropped. Every test is
//! in integer millimetres. Smoothing is the kernel's (`geometry.md` 2.4) and is not done here.

use crate::error::RasterError;
use crate::flood::{Low, Whole};
use crate::raster::Raster;
use crate::window::post_mm;
use std::collections::BinaryHeap;
use via_num::Grid;

/// A course before smoothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    /// The first written point, the posts of each leg, each later written point after its leg;
    /// consecutive repeats removed. Millimetres.
    pub points: Vec<(i64, i64)>,
    /// The sum of the weights of every step taken.
    pub cost: f64,
}

/// The post nearest a point: least squared distance in integer millimetres, ties to the lower post
/// in post order (`derived.md` 3, step 2). A point beyond the extent takes the nearest post on it.
pub fn nearest_post(grid: &Grid, x: i64, y: i64) -> (u64, u64) {
    let axis = |a: i64, a0: i64, n: u64| -> u64 {
        let r = grid.resolution;
        let last = (n - 1) as i64;
        let off = a - a0;
        let lo = off.div_euclid(r).clamp(0, last);
        let hi = (lo + 1).min(last);
        let d = |k: i64| (off - k * r).unsigned_abs();
        // Separable: the least squared sum is the least along each axis; a tie keeps the lower
        // index, which is the lower post in post order.
        if d(hi) < d(lo) {
            hi as u64
        } else {
            lo as u64
        }
    };
    (
        axis(x, grid.extent[0], grid.nx),
        axis(y, grid.extent[1], grid.ny),
    )
}

/// The course through `waypoints` (millimetres, at least one) over `cost` (per metre, every post
/// of the grid). A cost at or below 0, or not finite, is `formula.range` at the first such post in
/// post order.
pub fn route(
    grid: &Grid,
    cost: &Raster<f64>,
    waypoints: &[(i64, i64)],
) -> Result<Course, RasterError> {
    let w = Whole::read(grid, cost)?;
    for (k, &c) in w.v.iter().enumerate() {
        if !(c.is_finite() && c > 0.0) {
            let (i, j) = w.ij(k);
            return Err(RasterError::CostRange {
                at: post_mm(grid, i as u64, j as u64),
                cost: c,
            });
        }
    }
    let Some(&first) = waypoints.first() else {
        return Err(RasterError::Internal {
            reason: "a course with no written point".into(),
        });
    };
    let r = (grid.resolution as f64) / 1000.0;
    let diag = r * 2f64.sqrt();
    let half = grid.resolution / 2;
    // Each point and whether it is written (a written point is never dropped).
    let mut points: Vec<((i64, i64), bool)> = vec![(first, true)];
    let mut total = 0.0;
    let index = |p: (u64, u64)| p.1 as usize * w.nx + p.0 as usize;
    for leg in waypoints.windows(2) {
        let a = index(nearest_post(grid, leg[0].0, leg[0].1));
        let b = index(nearest_post(grid, leg[1].0, leg[1].1));
        let (posts, d) = dijkstra(&w, a, b, r, diag);
        total += d;
        let mm: Vec<(i64, i64)> = posts
            .iter()
            .map(|&k| {
                let (i, j) = w.ij(k);
                post_mm(grid, i as u64, j as u64)
            })
            .collect();
        let kept = simplify(&mm, half);
        // The leg's end posts give way to the written points they are nearest.
        if kept.len() > 2 {
            points.extend(kept[1..kept.len() - 1].iter().map(|&p| (p, false)));
        }
        points.push((leg[1], true));
    }
    points.dedup_by(|b, a| {
        if a.0 == b.0 {
            a.1 |= b.1;
            true
        } else {
            false
        }
    });
    let points = unfold(points, half);
    Ok(Course {
        points,
        cost: total,
    })
}

/// Squared distance from `p` to the segment's line, times the segment's squared length, and that
/// squared length: `cross²` and `len²`, exact in integers.
fn cross_sq(a: (i64, i64), b: (i64, i64), p: (i64, i64)) -> (i128, i128) {
    let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (px, py) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
    let c = dx * py - dy * px;
    (c * c, dx * dx + dy * dy)
}

/// Douglas and Peucker (1973) on a leg's posts: the first and last kept; of the posts between, the
/// one farthest from the line through the two kept around it (ties to the first) is kept when it
/// lies more than `tol` from it, and each side is simplified in turn.
pub fn simplify(pts: &[(i64, i64)], tol: i64) -> Vec<(i64, i64)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let tol2 = i128::from(tol) * i128::from(tol);
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        // The line a..b is fixed for the pass, so its squared length l2 is the same for every k
        // and the squared cross products compare directly: no product of two of them, which
        // would leave i128 on legs of a few kilometres (L^4 D^2 in millimetres).
        let mut best: Option<(usize, i128, i128)> = None;
        for k in a + 1..b {
            let (c2, l2) = cross_sq(pts[a], pts[b], pts[k]);
            if best.is_none_or(|(_, bc, _)| c2 > bc) {
                best = Some((k, c2, l2));
            }
        }
        if let Some((k, c2, l2)) = best {
            // Farther than `tol` from the line: c2 > tol² · l2 (a line of no length keeps it).
            if l2 == 0 || c2 > tol2 * l2 {
                keep[k] = true;
                stack.push((a, k));
                stack.push((k, b));
            }
        }
    }
    pts.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// Drops, first in order and again until none is left, each point not written that makes the
/// course double back (it turns by more than 90 degrees there) or that stands closer than `min`
/// to a neighbour.
fn unfold(mut pts: Vec<((i64, i64), bool)>, min: i64) -> Vec<(i64, i64)> {
    let min2 = i128::from(min) * i128::from(min);
    let d2 = |a: (i64, i64), b: (i64, i64)| {
        let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        dx * dx + dy * dy
    };
    loop {
        let n = pts.len();
        let bad = (1..n.saturating_sub(1)).find(|&k| {
            let (a, b, c) = (pts[k - 1].0, pts[k].0, pts[k + 1].0);
            let dot = i128::from(b.0 - a.0) * i128::from(c.0 - b.0)
                + i128::from(b.1 - a.1) * i128::from(c.1 - b.1);
            !pts[k].1 && (dot < 0 || d2(a, b) < min2 || d2(b, c) < min2)
        });
        match bad {
            Some(k) => {
                pts.remove(k);
            }
            None => return pts.into_iter().map(|(p, _)| p).collect(),
        }
    }
}

/// One leg: the posts from `a` to `b` inclusive, and the distance.
fn dijkstra(w: &Whole, a: usize, b: usize, r: f64, diag: f64) -> (Vec<usize>, f64) {
    let n = w.v.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut pred: Vec<Option<usize>> = vec![None; n];
    let mut done = vec![false; n];
    let mut q = BinaryHeap::new();
    dist[a] = 0.0;
    q.push(Low(0.0, a));
    while let Some(Low(d, k)) = q.pop() {
        if done[k] {
            continue;
        }
        done[k] = true;
        if k == b {
            break;
        }
        let (i, j) = w.ij(k);
        for m in w.neighbours(k) {
            if done[m] {
                continue;
            }
            let (mi, mj) = w.ij(m);
            let len = if mi != i && mj != j { diag } else { r };
            let nd = d + 0.5 * (w.v[k] + w.v[m]) * len;
            if nd < dist[m] {
                dist[m] = nd;
                pred[m] = Some(k);
                q.push(Low(nd, m));
            }
        }
    }
    let mut posts = vec![b];
    let mut k = b;
    while let Some(p) = pred[k] {
        posts.push(p);
        k = p;
    }
    posts.reverse();
    (posts, dist[b])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::Window;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn cost(f: impl Fn(i64, i64) -> f64) -> Raster<f64> {
        Raster::from_fn(Window::whole(&g8()), |i, j| {
            Ok::<_, ()>(f(i as i64, j as i64))
        })
        .unwrap()
    }

    #[test]
    fn simplify_holds_legs_of_tens_of_kilometres() {
        // A 40 km leg with a 30 km detour: L^4 D^2 in millimetres is far past i128; the
        // comparison must not form it.
        let pts = [
            (0, 0),
            (10_000_000, 1_000),
            (20_000_000, 30_000_000),
            (30_000_000, 2_000),
            (40_000_000, 0),
        ];
        assert_eq!(
            simplify(&pts, 500),
            vec![
                (0, 0),
                (10_000_000, 1_000),
                (20_000_000, 30_000_000),
                (30_000_000, 2_000),
                (40_000_000, 0)
            ]
        );
        // Within the tolerance of the line, only the ends stay.
        let flat = [(0, 0), (20_000_000, 400), (40_000_000, 0)];
        assert_eq!(simplify(&flat, 500), vec![(0, 0), (40_000_000, 0)]);
    }

    #[test]
    fn d06_flat_cost_is_the_straight_row() {
        let c = route(&g8(), &cost(|_, _| 1.0), &[(0, 0), (8_000, 0)]).unwrap();
        // The nine posts of the row, simplified to its ends.
        assert_eq!(c.points, vec![(0, 0), (8_000, 0)]);
        assert_eq!(c.cost, 8.0);
    }

    #[test]
    fn d07_ties_keep_the_first_settled() {
        let c = route(&g8(), &cost(|_, _| 1.0), &[(0, 0), (2_000, 1_000)]).unwrap();
        // (0, 0), (1, 0), (2, 1): (1, 0) lies 0.45 m from the line, within half a spacing.
        assert_eq!(c.points, vec![(0, 0), (2_000, 1_000)]);
    }

    #[test]
    fn d08_goes_round_a_ridge() {
        // A wall of cost 100 at x = 4 but for a gap at y = 8.
        let c = route(
            &g8(),
            &cost(|x, y| if x == 4 && y < 8 { 100.0 } else { 1.0 }),
            &[(0, 4_000), (8_000, 4_000)],
        )
        .unwrap();
        assert!(c.points.iter().all(|&(x, y)| !(x == 4_000 && y < 8_000)));
        assert_eq!(c.points, vec![(0, 4_000), (4_000, 8_000), (8_000, 4_000)]);
    }

    #[test]
    fn d09_a_free_post_is_formula_range_at_the_first() {
        match route(&g8(), &cost(|_, _| 0.0), &[(0, 0), (8_000, 0)]) {
            Err(RasterError::CostRange { at, .. }) => assert_eq!(at, (0, 0)),
            other => panic!("{other:?}"),
        }
        match route(
            &g8(),
            &cost(|x, y| if (x, y) == (3, 2) { -1.0 } else { 1.0 }),
            &[(0, 0)],
        ) {
            Err(e @ RasterError::CostRange { .. }) => {
                assert_eq!(e.code(), via_diag::codes::formula::RANGE);
                assert_eq!(e.position(), Some(via_diag::Position::plan(3_000, 2_000)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn waypoints_off_posts_stay_written() {
        let c = route(
            &g8(),
            &cost(|_, _| 1.0),
            &[(300, 200), (2_000, 0), (2_400, 2_600)],
        )
        .unwrap();
        assert_eq!(c.points.first(), Some(&(300, 200)));
        assert_eq!(c.points.last(), Some(&(2_400, 2_600)));
        // Through the middle written point.
        assert!(c.points.contains(&(2_000, 0)));
        assert_eq!(nearest_post(&g8(), 2_500, 2_600), (2, 3));
        assert_eq!(nearest_post(&g8(), -50, 9_000), (0, 8));
    }

    #[test]
    fn a_course_neither_doubles_back_nor_keeps_short_steps() {
        // An oblique leg over a cost that rises east: the staircase of posts, simplified.
        let g = Grid::new([0, 0, 40_000, 40_000], 1_000).unwrap();
        let c = Raster::from_fn(Window::whole(&g), |i, j| {
            Ok::<_, ()>(1.0 + 0.01 * (i as f64) + 0.003 * (j as f64))
        })
        .unwrap();
        let w = [(500, 300), (31_700, 12_400), (39_600, 39_800)];
        let found = route(&g, &c, &w).unwrap();
        let p = &found.points;
        assert_eq!(p.first(), Some(&w[0]));
        assert_eq!(p.last(), Some(&w[2]));
        for k in 1..p.len() {
            let (dx, dy) = (p[k].0 - p[k - 1].0, p[k].1 - p[k - 1].1);
            assert!(dx * dx + dy * dy >= 500 * 500, "a short step at {k}: {p:?}");
        }
        for k in 1..p.len() - 1 {
            let (a, b, c) = (p[k - 1], p[k], p[k + 1]);
            let dot = (b.0 - a.0) * (c.0 - b.0) + (b.1 - a.1) * (c.1 - b.1);
            assert!(dot >= 0, "it doubles back at {k}: {p:?}");
        }
        assert!(p.len() < 20, "{p:?}");
    }
}
