//! A line water would follow (`derived.md` 4.2): steepest descent over the field's surface.
//!
//! The surface is the posts joined into triangles: each grid cell split by its diagonal from post
//! `(i, j)` to `(i + 1, j + 1)`, so it is a plane in each triangle and passes through every post
//! exactly. The line goes the way the surface falls fastest: across a triangle along its gradient,
//! down a groove where two triangles fall towards their common side, or out of a post along the
//! steepest of its six sides and six triangles. This is the continuous direction of the D-infinity
//! method (Tarboton 1997, "A new method for the determination of flow directions and upslope areas
//! in grid digital elevation models", Water Resources Research 33(2)) traced as a line, so a line
//! runs at any angle: it is not the eight-way post-to-post walk (O'Callaghan and Mark 1984) whose
//! rectilinear drainage the spike recorded as a failure.
//!
//! Every point of the line is on the millimetre lattice: a crossing of a cell side is rounded to the
//! nearest millimetre along that side (`numbers.md` 5), so it stays on the side exactly, and a
//! crossing that rounds to a post is that post. The value at each new point must be strictly below
//! the last, so the walk ends.
//!
//! The walk stops at a point with nowhere lower to go (`pit`), or on the extent's outline where the
//! surface falls out of the extent (`edge`).

use crate::error::RasterError;
use crate::flood::Whole;
use crate::raster::Raster;
use via_num::Grid;

/// How a line ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Nowhere lower to go inside the extent.
    Pit,
    /// The surface falls out of the extent.
    Edge,
}

impl End {
    /// As the `end` column writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            End::Pit => "pit",
            End::Edge => "edge",
        }
    }
}

/// A line water would follow.
#[derive(Clone, Debug, PartialEq)]
pub struct Descent {
    /// The point, then each point the line passes, millimetres, no consecutive repeat. One point
    /// when the line does not move: a point feature, `end = "pit"` (`derived.md` 4.2).
    pub points: Vec<(i64, i64)>,
    /// How it ends.
    pub end: End,
}

/// A side of a grid cell: from post `a` east, north, or north-east (the cell's diagonal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dir {
    E,
    N,
    NE,
}

impl Dir {
    fn step(self) -> (usize, usize) {
        match self {
            Dir::E => (1, 0),
            Dir::N => (0, 1),
            Dir::NE => (1, 1),
        }
    }
}

/// Where the walk stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum At {
    /// On a post.
    Post(usize, usize),
    /// Inside a side: post `a` plus `off` millimetres along `dir` on each axis it moves in,
    /// `0 < off < r`.
    Side {
        a: (usize, usize),
        dir: Dir,
        off: i64,
    },
    /// Inside triangle `tri` (0: below the diagonal, 1: above) of cell `c`, at `(x, y)` millimetres
    /// from the cell's least corner. Only a starting point is here.
    Inside {
        c: (usize, usize),
        tri: u8,
        x: i64,
        y: i64,
    },
}

/// A triangle: its cell and which half (0 below the diagonal, 1 above).
type Tri = ((usize, usize), u8);

/// A way out of a post: along a side to a post, or across a triangle in a direction.
type Way = Result<At, (Tri, (f64, f64))>;

struct Surface<'a> {
    w: &'a Whole,
    r: i64,
    x0: i64,
    y0: i64,
}

impl Surface<'_> {
    fn v(&self, i: usize, j: usize) -> f64 {
        self.w.v[j * self.w.nx + i]
    }

    fn cells(&self) -> (usize, usize) {
        (self.w.nx - 1, self.w.ny - 1)
    }

    /// The gradient of a triangle, value per millimetre.
    fn grad(&self, c: (usize, usize), tri: u8) -> (f64, f64) {
        let (i, j) = c;
        let r = self.r as f64;
        let v00 = self.v(i, j);
        let v10 = self.v(i + 1, j);
        let v01 = self.v(i, j + 1);
        let v11 = self.v(i + 1, j + 1);
        if tri == 0 {
            ((v10 - v00) / r, (v11 - v10) / r)
        } else {
            ((v11 - v01) / r, (v01 - v00) / r)
        }
    }

    fn mm(&self, at: At) -> (i64, i64) {
        let r = self.r;
        match at {
            At::Post(i, j) => (self.x0 + i as i64 * r, self.y0 + j as i64 * r),
            At::Side { a, dir, off } => {
                let (di, dj) = dir.step();
                (
                    self.x0 + a.0 as i64 * r + di as i64 * off,
                    self.y0 + a.1 as i64 * r + dj as i64 * off,
                )
            }
            At::Inside { c, x, y, .. } => {
                (self.x0 + c.0 as i64 * r + x, self.y0 + c.1 as i64 * r + y)
            }
        }
    }

    fn z(&self, at: At) -> f64 {
        match at {
            At::Post(i, j) => self.v(i, j),
            At::Side { a, dir, off } => {
                let (di, dj) = dir.step();
                let va = self.v(a.0, a.1);
                let vb = self.v(a.0 + di, a.1 + dj);
                va + (vb - va) * ((off as f64) / (self.r as f64))
            }
            At::Inside { c, tri, x, y } => {
                let (gx, gy) = self.grad(c, tri);
                self.v(c.0, c.1) + gx * (x as f64) + gy * (y as f64)
            }
        }
    }

    /// A side, normalised: an end at 0 or `r` is that post.
    fn side(&self, a: (usize, usize), dir: Dir, off: i64) -> At {
        let (di, dj) = dir.step();
        if off <= 0 {
            At::Post(a.0, a.1)
        } else if off >= self.r {
            At::Post(a.0 + di, a.1 + dj)
        } else {
            At::Side { a, dir, off }
        }
    }

    /// Whether a direction points strictly out of the extent from a point on its outline.
    fn outward(&self, (x, y): (i64, i64), d: (f64, f64)) -> bool {
        let r = self.r;
        let x1 = self.x0 + (self.w.nx as i64 - 1) * r;
        let y1 = self.y0 + (self.w.ny as i64 - 1) * r;
        (x == self.x0 && d.0 < 0.0)
            || (x == x1 && d.0 > 0.0)
            || (y == self.y0 && d.1 < 0.0)
            || (y == y1 && d.1 > 0.0)
    }

    /// The triangles that hold a point on a post or a side, each as (cell, tri).
    fn around(&self, at: At) -> Vec<Tri> {
        let (cx, cy) = self.cells();
        let cell = |i: i64, j: i64| -> Option<(usize, usize)> {
            (i >= 0 && j >= 0 && (i as usize) < cx && (j as usize) < cy)
                .then_some((i as usize, j as usize))
        };
        let mut out = Vec::new();
        match at {
            At::Post(i, j) => {
                let (i, j) = (i as i64, j as i64);
                // Wedges counter-clockwise from east: E-NE, NE-N, N-W, W-SW, SW-S, S-E.
                for (c, t) in [
                    (cell(i, j), 0),
                    (cell(i, j), 1),
                    (cell(i - 1, j), 0),
                    (cell(i - 1, j - 1), 1),
                    (cell(i - 1, j - 1), 0),
                    (cell(i, j - 1), 1),
                ] {
                    if let Some(c) = c {
                        out.push((c, t));
                    }
                }
            }
            At::Side { a, dir, .. } => {
                let (i, j) = (a.0 as i64, a.1 as i64);
                let pair = match dir {
                    Dir::E => [(cell(i, j - 1), 1), (cell(i, j), 0)],
                    Dir::N => [(cell(i - 1, j), 0), (cell(i, j), 1)],
                    Dir::NE => [(cell(i, j), 0), (cell(i, j), 1)],
                };
                for (c, t) in pair {
                    if let Some(c) = c {
                        out.push((c, t));
                    }
                }
            }
            At::Inside { c, tri, .. } => out.push((c, tri)),
        }
        out
    }

    /// The corner offsets of a triangle, in its cell, counter-clockwise.
    fn corners(tri: u8) -> [(i64, i64); 3] {
        if tri == 0 {
            [(0, 0), (1, 0), (1, 1)]
        } else {
            [(0, 0), (1, 1), (0, 1)]
        }
    }

    /// Whether `d` from point `p` (millimetres from the cell's corner) points strictly into the
    /// triangle: strictly inside the half-plane of every side `p` lies on.
    fn enters(&self, tri: u8, p: (i64, i64), d: (f64, f64)) -> bool {
        let r = self.r;
        let k = Self::corners(tri);
        let mut ok = true;
        for e in 0..3 {
            let a = (k[e].0 * r, k[e].1 * r);
            let b = (k[(e + 1) % 3].0 * r, k[(e + 1) % 3].1 * r);
            let ex = (b.0 - a.0) as f64;
            let ey = (b.1 - a.1) as f64;
            // p on this side's line (exact, integers): then d must turn left of it, strictly.
            let on = i128::from(b.0 - a.0) * i128::from(p.1 - a.1)
                - i128::from(b.1 - a.1) * i128::from(p.0 - a.0)
                == 0;
            if on && ex * d.1 - ey * d.0 <= 0.0 {
                ok = false;
            }
        }
        ok
    }

    /// From point `p` (millimetres from cell `c`'s corner) along `d` across triangle `tri`: the
    /// first side the ray leaves by, as a lattice point on it.
    fn cross(&self, c: (usize, usize), tri: u8, p: (i64, i64), d: (f64, f64)) -> Option<At> {
        let r = self.r as f64;
        let (px, py) = (p.0 as f64, p.1 as f64);
        let (dx, dy) = d;
        // Each candidate: (t, side's post a, dir, offset along the side in millimetres).
        let mut best: Option<(f64, (usize, usize), Dir, f64)> = None;
        let mut take = |t: f64, a: (usize, usize), dir: Dir, s: f64| {
            if t > 0.0 && t.is_finite() && best.is_none_or(|b| t < b.0) {
                best = Some((t, a, dir, s));
            }
        };
        let (i, j) = c;
        if tri == 0 {
            if dy < 0.0 {
                let t = -py / dy;
                take(t, (i, j), Dir::E, px + t * dx);
            }
            if dx > 0.0 {
                let t = (r - px) / dx;
                take(t, (i + 1, j), Dir::N, py + t * dy);
            }
            if dx - dy < 0.0 {
                let t = (py - px) / (dx - dy);
                take(t, (i, j), Dir::NE, px + t * dx);
            }
        } else {
            if dx < 0.0 {
                let t = -px / dx;
                take(t, (i, j), Dir::N, py + t * dy);
            }
            if dy > 0.0 {
                let t = (r - py) / dy;
                take(t, (i, j + 1), Dir::E, px + t * dx);
            }
            if dx - dy > 0.0 {
                let t = (py - px) / (dx - dy);
                take(t, (i, j), Dir::NE, px + t * dx);
            }
        }
        let (_, a, dir, s) = best?;
        let s = via_num::clamp(s, 0.0, r);
        let off = via_num::round_to_lattice(s).ok()?;
        Some(self.side(a, dir, off))
    }

    /// A point's millimetres from the least corner of cell `c`.
    fn local(&self, c: (usize, usize), at: At) -> (i64, i64) {
        let (x, y) = self.mm(at);
        (
            x - self.x0 - c.0 as i64 * self.r,
            y - self.y0 - c.1 as i64 * self.r,
        )
    }

    /// The next point from `at`, or how the walk ends there.
    fn next(&self, at: At) -> Result<At, End> {
        let here = self.mm(at);
        let tris = self.around(at);
        // On the outline, a surface that falls out of the extent ends the line.
        for &(c, t) in &tris {
            let (gx, gy) = self.grad(c, t);
            let d = (-gx, -gy);
            if (d.0 != 0.0 || d.1 != 0.0) && self.outward(here, d) {
                return Err(End::Edge);
            }
        }
        match at {
            At::Inside { c, tri, x, y } => {
                let (gx, gy) = self.grad(c, tri);
                if gx == 0.0 && gy == 0.0 {
                    return Err(End::Pit);
                }
                self.cross(c, tri, (x, y), (-gx, -gy)).ok_or(End::Pit)
            }
            At::Side { a, dir, .. } => {
                // Into the triangle that falls fastest, of those the descent points into.
                let mut best: Option<(f64, Tri, (f64, f64))> = None;
                for &(c, t) in &tris {
                    let (gx, gy) = self.grad(c, t);
                    let d = (-gx, -gy);
                    let s = gx * gx + gy * gy;
                    if s > 0.0
                        && self.enters(t, self.local(c, at), d)
                        && best.is_none_or(|b| s > b.0)
                    {
                        best = Some((s, (c, t), d));
                    }
                }
                if let Some((_, (c, t), d)) = best {
                    return self.cross(c, t, self.local(c, at), d).ok_or(End::Pit);
                }
                // A groove: down the side to its lower end.
                let (di, dj) = dir.step();
                let b = (a.0 + di, a.1 + dj);
                let (va, vb) = (self.v(a.0, a.1), self.v(b.0, b.1));
                if va < vb {
                    Ok(At::Post(a.0, a.1))
                } else if vb < va {
                    Ok(At::Post(b.0, b.1))
                } else {
                    Err(End::Pit)
                }
            }
            At::Post(i, j) => {
                let v0 = self.v(i, j);
                let r = self.r as f64;
                let diag = r * 2f64.sqrt();
                // Candidates: the six sides in post order of their far post, then the triangles.
                let mut best: Option<(f64, Way)> = None;
                let (ii, jj) = (i as i64, j as i64);
                for (di, dj) in [(-1i64, -1i64), (0, -1), (-1, 0), (1, 0), (0, 1), (1, 1)] {
                    let (a, b) = (ii + di, jj + dj);
                    if a < 0 || b < 0 || a as usize >= self.w.nx || b as usize >= self.w.ny {
                        continue;
                    }
                    let vq = self.v(a as usize, b as usize);
                    if vq < v0 {
                        let len = if di != 0 && dj != 0 { diag } else { r };
                        let s = (v0 - vq) / len;
                        if best.as_ref().is_none_or(|b| s > b.0) {
                            best = Some((s, Ok(At::Post(a as usize, b as usize))));
                        }
                    }
                }
                for &(c, t) in &tris {
                    let (gx, gy) = self.grad(c, t);
                    let d = (-gx, -gy);
                    let s = (gx * gx + gy * gy).sqrt();
                    if s > 0.0
                        && self.enters(t, self.local(c, at), d)
                        && best.as_ref().is_none_or(|b| s > b.0)
                    {
                        best = Some((s, Err(((c, t), d))));
                    }
                }
                match best {
                    None => Err(End::Pit),
                    Some((_, Ok(next))) => Ok(next),
                    Some((_, Err(((c, t), d)))) => {
                        self.cross(c, t, self.local(c, at), d).ok_or(End::Pit)
                    }
                }
            }
        }
    }
}

/// Where a point lies: on a post, inside a side, or inside a triangle.
fn locate(s: &Surface, x: i64, y: i64) -> At {
    let r = s.r;
    let (cx, cy) = s.cells();
    let ci = ((x - s.x0).div_euclid(r) as usize).min(cx - 1);
    let cj = ((y - s.y0).div_euclid(r) as usize).min(cy - 1);
    let lx = x - s.x0 - ci as i64 * r;
    let ly = y - s.y0 - cj as i64 * r;
    let xe = lx == 0 || lx == r;
    let ye = ly == 0 || ly == r;
    let pi = ci + usize::from(lx == r);
    let pj = cj + usize::from(ly == r);
    if xe && ye {
        At::Post(pi, pj)
    } else if ye {
        s.side((ci, pj), Dir::E, lx)
    } else if xe {
        s.side((pi, cj), Dir::N, ly)
    } else if lx == ly {
        s.side((ci, cj), Dir::NE, lx)
    } else {
        At::Inside {
            c: (ci, cj),
            tri: if lx > ly { 0 } else { 1 },
            x: lx,
            y: ly,
        }
    }
}

/// The line water would follow from `(x, y)` (millimetres) over `values`, every post of the grid.
/// A point beyond the extent is `formula.outside_field`.
pub fn descend<T: Copy + Into<f64>>(
    grid: &Grid,
    values: &Raster<T>,
    x: i64,
    y: i64,
) -> Result<Descent, RasterError> {
    let [x0, y0, x1, y1] = grid.extent;
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return Err(RasterError::OutsideField { at: (x, y) });
    }
    let w = Whole::read(grid, values)?;
    let s = Surface {
        w: &w,
        r: grid.resolution,
        x0,
        y0,
    };
    let mut at = locate(&s, x, y);
    let mut points = vec![(x, y)];
    let mut z = s.z(at);
    // Each step falls strictly, so the walk ends; the bound only guards against a defect.
    let limit = 16 * w.v.len() + 16;
    let mut end = End::Pit;
    for _ in 0..limit {
        match s.next(at) {
            Err(e) => {
                end = e;
                break;
            }
            Ok(n) => {
                let zn = s.z(n);
                if zn >= z {
                    break;
                }
                at = n;
                z = zn;
                points.push(s.mm(n));
            }
        }
    }
    points.dedup();
    if points.len() < 2 {
        end = End::Pit;
    }
    Ok(Descent { points, end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::Window;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn field(f: impl Fn(f64, f64) -> f64) -> Raster<f32> {
        Raster::from_fn(Window::whole(&g8()), |i, j| {
            Ok::<_, ()>(f(i as f64, j as f64) as f32)
        })
        .unwrap()
    }

    fn bowl() -> Raster<f32> {
        field(|x, y| (x - 4.0) * (x - 4.0) + (y - 4.0) * (y - 4.0))
    }

    #[test]
    fn d13_bowl_from_the_corner_and_from_the_bottom() {
        let d = descend(&g8(), &bowl(), 8_000, 8_000).unwrap();
        let want: Vec<(i64, i64)> = (4..=8).rev().map(|k| (k * 1000, k * 1000)).collect();
        assert_eq!(d.points, want);
        assert_eq!(d.end, End::Pit);
        let d = descend(&g8(), &bowl(), 4_000, 4_000).unwrap();
        assert_eq!(d.points, vec![(4_000, 4_000)]);
        assert_eq!(d.end, End::Pit);
    }

    #[test]
    fn d14_a_ramp_runs_off_the_edge() {
        let d = descend(&g8(), &field(|x, _| x), 4_000, 4_000).unwrap();
        let want: Vec<(i64, i64)> = (0..=4).rev().map(|k| (k * 1000, 4_000)).collect();
        assert_eq!(d.points, want);
        assert_eq!(d.end, End::Edge);
    }

    #[test]
    fn an_oblique_plane_gives_a_straight_line_not_a_staircase() {
        // A plane falling towards (-2, -1): the line keeps that bearing across cells, where an
        // eight-way walk would step east-west and diagonally in turn.
        let h = field(|x, y| 2.0 * x + y);
        let d = descend(&g8(), &h, 7_300, 6_100).unwrap();
        assert_eq!(d.end, End::Edge);
        let (sx, sy) = (7_300i64, 6_100i64);
        for &(x, y) in &d.points {
            // On the line through the start with direction (2, 1), within the rounding.
            let off = (x - sx) - 2 * (y - sy);
            assert!(off.abs() <= 10, "({x}, {y}) is {off} mm off the line");
        }
        let last = *d.points.last().unwrap();
        assert_eq!(last.0, 0, "it leaves by the west side: {last:?}");
    }

    #[test]
    fn a_groove_is_followed_down_its_floor() {
        // A V valley along y = 4, falling west: the line from the flank meets the floor and
        // runs down it.
        let h = field(|x, y| 3.0 * (y - 4.0).abs() + 0.5 * x);
        let d = descend(&g8(), &h, 6_500, 6_200).unwrap();
        assert_eq!(d.end, End::Edge);
        let last = *d.points.last().unwrap();
        assert_eq!(last, (0, 4_000));
        // Values fall at every point (the field is a plane in each triangle, so exact there).
        let hv: Vec<f64> = d
            .points
            .iter()
            .map(|&(x, y)| 3.0 * ((y as f64) / 1000.0 - 4.0).abs() + 0.5 * (x as f64) / 1000.0)
            .collect();
        assert!(hv.windows(2).all(|p| p[1] < p[0]), "{hv:?}");
    }

    #[test]
    fn a_flat_is_a_pit_and_outside_is_an_error() {
        let d = descend(&g8(), &field(|_, _| 1.0), 3_300, 2_100).unwrap();
        assert_eq!((d.points.len(), d.end), (1, End::Pit));
        assert!(matches!(
            descend(&g8(), &bowl(), -1, 0),
            Err(RasterError::OutsideField { .. })
        ));
    }
}
