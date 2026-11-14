//! Hollows: the level a hollow holds to, and the ground with every hollow filled
//! (`derived.md` 4.1).
//!
//! Mechanism: Priority-Flood (Barnes, Lehman and Mulla, 2014, "Priority-Flood: An optimal
//! depression-filling and watershed-labeling algorithm for digital elevation models", Computers &
//! Geosciences 62). The outlets are put in a queue at their own values; the lowest is taken, and
//! each neighbour not yet reached is given the greater of its own value and the taken post's
//! level, then queued. Each post is taken once, so the level a post is given is the least, over all
//! paths of eight-neighbour posts from it to an outlet, of the greatest value on the path, both
//! ends included: the level water stands at before it runs out.
//!
//! The queue is ordered by level, then post order, so equal levels are taken the same way on every
//! system. Values are compared exactly, as doubles widened from the field's narrowed values
//! (`fields.md` 1.1); nothing is computed but the greater of two values.

use crate::error::RasterError;
use crate::raster::{grid_cell, Raster};
use crate::window::{post_mm, Window};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use via_num::Grid;

/// The eight neighbours of a post, as `(di, dj)`, in post order of the neighbour.
pub(crate) const EIGHT: [(i64, i64); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// A queue entry: a level and a post index, taken least level first, then least index.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Low(pub f64, pub usize);

impl PartialEq for Low {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Low {}
impl PartialOrd for Low {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Low {
    // Reversed, so `BinaryHeap`, a max-heap, gives the least first.
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
    }
}

/// The values of a field over the whole grid, widened to doubles, with the grid's shape. The raster
/// must cover every post (a derived result is resolved over the whole extent, `derived.md` 1).
pub(crate) struct Whole {
    pub nx: usize,
    pub ny: usize,
    pub v: Vec<f64>,
}

impl Whole {
    pub fn read<T: Copy + Into<f64>>(
        grid: &Grid,
        values: &Raster<T>,
    ) -> Result<Whole, RasterError> {
        let w = Window::whole(grid);
        if values.window() != w {
            return Err(RasterError::Internal {
                reason: format!(
                    "a derived result reads the whole extent {}, not {}",
                    w.key(),
                    values.window().key()
                ),
            });
        }
        Ok(Whole {
            nx: grid.nx as usize,
            ny: grid.ny as usize,
            v: values.values().iter().map(|&x| x.into()).collect(),
        })
    }

    pub fn ij(&self, k: usize) -> (usize, usize) {
        (k % self.nx, k / self.nx)
    }

    pub fn on_edge(&self, k: usize) -> bool {
        let (i, j) = self.ij(k);
        i == 0 || j == 0 || i + 1 == self.nx || j + 1 == self.ny
    }

    /// The neighbours of post `k` that are in the grid, in post order.
    pub fn neighbours(&self, k: usize) -> impl Iterator<Item = usize> + '_ {
        let (i, j) = self.ij(k);
        EIGHT.iter().filter_map(move |&(di, dj)| {
            let a = i as i64 + di;
            let b = j as i64 + dj;
            (a >= 0 && b >= 0 && (a as usize) < self.nx && (b as usize) < self.ny)
                .then(|| b as usize * self.nx + a as usize)
        })
    }
}

/// The start post of a point (`derived.md` 4): the lowest of the four posts of the grid cell
/// holding it (`fields.md` 1.2), ties to post order. A point beyond the extent is
/// `formula.outside_field`.
pub fn start_post<T: Copy + Into<f64>>(
    grid: &Grid,
    values: &Raster<T>,
    x: i64,
    y: i64,
) -> Result<(u64, u64), RasterError> {
    let [x0, y0, x1, y1] = grid.extent;
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return Err(RasterError::OutsideField { at: (x, y) });
    }
    let (i, _) = grid_cell(x - x0, grid.resolution, grid.nx);
    let (j, _) = grid_cell(y - y0, grid.resolution, grid.ny);
    let i1 = (i + 1).min(grid.nx - 1);
    let j1 = (j + 1).min(grid.ny - 1);
    let mut best: Option<((u64, u64), f64)> = None;
    for (a, b) in [(i, j), (i1, j), (i, j1), (i1, j1)] {
        let v: f64 = values
            .get(a, b)
            .map(Into::into)
            .ok_or_else(|| RasterError::Internal {
                reason: format!("post ({a}, {b}) is not in {}", values.window().key()),
            })?;
        if best.is_none_or(|(_, bv)| v < bv) {
            best = Some(((a, b), v));
        }
    }
    best.map(|(p, _)| p).ok_or(RasterError::Internal {
        reason: "a grid cell with no post".into(),
    })
}

/// What [`spill`] finds (`derived.md` 4.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spill {
    /// The level the hollow holding the start post fills to before it runs out.
    pub level: f64,
    /// True when the start post's own value is the level: the point is in no hollow.
    pub dry: bool,
    /// The start post.
    pub start: (u64, u64),
}

/// The spill level at a point (`derived.md` 4.1). The outlets are the posts on the extent's edge
/// and, when `radius` is above 0, every post farther than `radius` millimetres from the point
/// (`dx² + dy² > R²`, in integers). The level is found by Priority-Flood from the outlets, stopped
/// when the start post is taken.
pub fn spill<T: Copy + Into<f64>>(
    grid: &Grid,
    values: &Raster<T>,
    x: i64,
    y: i64,
    radius: i64,
) -> Result<Spill, RasterError> {
    if radius < 0 {
        return Err(RasterError::ParameterRange {
            what: "spill radius",
            value: radius as f64,
        });
    }
    let w = Whole::read(grid, values)?;
    let start = start_post(grid, values, x, y)?;
    let s = start.1 as usize * w.nx + start.0 as usize;
    let r2 = i128::from(radius) * i128::from(radius);
    let outlet = |k: usize| -> bool {
        if w.on_edge(k) {
            return true;
        }
        if radius == 0 {
            return false;
        }
        let (i, j) = w.ij(k);
        let (px, py) = post_mm(grid, i as u64, j as u64);
        let dx = i128::from(px) - i128::from(x);
        let dy = i128::from(py) - i128::from(y);
        dx * dx + dy * dy > r2
    };
    let mut seen = vec![false; w.v.len()];
    let mut q = BinaryHeap::new();
    for (k, s) in seen.iter_mut().enumerate() {
        if outlet(k) {
            *s = true;
            q.push(Low(w.v[k], k));
        }
    }
    while let Some(Low(level, k)) = q.pop() {
        if k == s {
            return Ok(Spill {
                level,
                dry: w.v[s] == level,
                start,
            });
        }
        for n in w.neighbours(k) {
            if !seen[n] {
                seen[n] = true;
                q.push(Low(via_num::max(w.v[n], level), n));
            }
        }
    }
    Err(RasterError::Internal {
        reason: "the start post was never reached from an outlet".into(),
    })
}

/// The ground with every hollow filled to its spill level, and the order Priority-Flood took the
/// posts in. Outlets are the posts on the extent's edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Filled {
    /// The filled level at every post, in post order: at least the post's own value.
    pub level: Raster<f64>,
    /// The post indices (post order index `j * nx + i`) in the order they were taken. Levels never
    /// fall along it, so a post's lower neighbours, and the post that reached it, come earlier.
    pub order: Vec<usize>,
    /// The post that reached each post, `None` for an outlet. On a filled flat this is the way
    /// out (Barnes 2014, section 2.3).
    pub from: Vec<Option<usize>>,
}

impl Filled {
    /// How deep each post stands under its filled level, `level - value`, 0 on dry ground: the
    /// lakes that fill the ground. A lake is the mask of `depth > 0` (`geometry.md` 3.7).
    pub fn depth<T: Copy + Into<f64>>(&self, values: &Raster<T>) -> Raster<f64> {
        let v = values.values();
        let l = self.level.values();
        let out: Vec<f64> = (0..l.len()).map(|k| l[k] - v[k].into()).collect();
        Raster::from_values(self.level.window(), out).unwrap_or_else(|| self.level.clone())
    }
}

/// Every hollow filled (Priority-Flood, Barnes et al. 2014, algorithm 1), over the whole extent.
pub fn fill<T: Copy + Into<f64>>(grid: &Grid, values: &Raster<T>) -> Result<Filled, RasterError> {
    let w = Whole::read(grid, values)?;
    let n = w.v.len();
    let mut level = vec![0.0; n];
    let mut seen = vec![false; n];
    let mut from = vec![None; n];
    let mut order = Vec::with_capacity(n);
    let mut q = BinaryHeap::new();
    for (k, s) in seen.iter_mut().enumerate() {
        if w.on_edge(k) {
            *s = true;
            q.push(Low(w.v[k], k));
        }
    }
    while let Some(Low(l, k)) = q.pop() {
        level[k] = l;
        order.push(k);
        for m in w.neighbours(k) {
            if !seen[m] {
                seen[m] = true;
                from[m] = Some(k);
                q.push(Low(via_num::max(w.v[m], l), m));
            }
        }
    }
    let level = Raster::from_values(Window::whole(grid), level).ok_or(RasterError::Internal {
        reason: "filled levels do not match the grid".into(),
    })?;
    Ok(Filled { level, order, from })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn field(f: impl Fn(i64, i64) -> f64) -> Raster<f32> {
        let g = g8();
        Raster::from_fn(Window::whole(&g), |i, j| {
            Ok::<_, ()>(f(i as i64, j as i64) as f32)
        })
        .unwrap()
    }

    fn bowl() -> Raster<f32> {
        field(|x, y| ((x - 4) * (x - 4) + (y - 4) * (y - 4)) as f64)
    }

    fn twin() -> Raster<f32> {
        field(|x, y| ((x - 2).abs() + (y - 4).abs()).min((x - 6).abs() + (y - 4).abs()) as f64)
    }

    #[test]
    fn d11_bowl_spills_at_its_rim_or_its_radius() {
        let g = g8();
        let s = spill(&g, &bowl(), 4_000, 4_000, 0).unwrap();
        assert_eq!((s.level, s.dry), (16.0, false));
        let s = spill(&g, &bowl(), 4_000, 4_000, 2_000).unwrap();
        assert_eq!((s.level, s.dry), (5.0, false));
    }

    #[test]
    fn d12_twin_spills_over_its_low_side() {
        let s = spill(&g8(), &twin(), 2_000, 4_000, 0).unwrap();
        assert_eq!((s.level, s.dry), (2.0, false));
    }

    #[test]
    fn a_slope_is_dry() {
        let h = field(|x, _| x as f64);
        let s = spill(&g8(), &h, 4_000, 4_000, 0).unwrap();
        assert_eq!((s.level, s.dry), (4.0, true));
    }

    #[test]
    fn the_start_post_is_the_lowest_of_the_cell() {
        let g = g8();
        assert_eq!(start_post(&g, &bowl(), 4_500, 4_500).unwrap(), (4, 4));
        assert_eq!(start_post(&g, &bowl(), 3_500, 3_500).unwrap(), (4, 4));
        assert!(start_post(&g, &bowl(), 8_001, 0).is_err());
    }

    #[test]
    fn fill_levels_hollows_and_keeps_order() {
        let g = g8();
        let f = fill(&g, &twin()).unwrap();
        // Each hollow fills to 2, the level at which it reaches the edge.
        assert_eq!(f.level.get(2, 4), Some(2.0));
        assert_eq!(f.level.get(6, 4), Some(2.0));
        assert_eq!(f.level.get(4, 4), Some(2.0));
        let d = f.depth(&twin());
        assert_eq!(d.get(2, 4), Some(2.0));
        assert_eq!(d.get(0, 0), Some(0.0));
        // Levels never fall along the order.
        let l = f.level.values();
        assert!(f.order.windows(2).all(|p| l[p[0]] <= l[p[1]]));
        assert_eq!(f.order.len(), l.len());
    }

    #[test]
    fn spill_agrees_with_fill() {
        // The same minimax, by its two searches, at every inner post of a rough field.
        let g = g8();
        let h =
            field(|x, y| ((x * 7 + y * 13) % 5) as f64 + ((x - 4).abs() + (y - 4).abs()) as f64);
        let f = fill(&g, &h).unwrap();
        for j in 0..9 {
            for i in 0..9 {
                let s = spill(&g, &h, i * 1000, j * 1000, 0).unwrap();
                let sp = s.start;
                assert_eq!(Some(s.level), f.level.get(sp.0, sp.1), "({i}, {j})");
            }
        }
    }
}
