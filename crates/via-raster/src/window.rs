//! Windows of posts, halos, effect windows and chunks (`numbers.md` 6, `fields.md` 5.3,
//! `build.md` 5).
//!
//! A window is a rectangle of posts `i0 <= i < i1`, `j0 <= j < j1` of the world's grid
//! (`via_num::Grid`). Every operation of this crate takes the window it fills and gives, at each
//! post, the bits the whole extent gives there (the window rule).

use crate::error::RasterError;
use via_num::Grid;

/// A rectangle of posts, ends exclusive: `i0 <= i < i1`, `j0 <= j < j1`. Never empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Window {
    pub i0: u64,
    pub j0: u64,
    pub i1: u64,
    pub j1: u64,
}

impl Window {
    /// A window, or `None` when it would hold no post.
    pub fn new(i0: u64, j0: u64, i1: u64, j1: u64) -> Option<Window> {
        (i0 < i1 && j0 < j1).then_some(Window { i0, j0, i1, j1 })
    }

    /// Every post of the grid.
    pub fn whole(grid: &Grid) -> Window {
        Window {
            i0: 0,
            j0: 0,
            i1: grid.nx,
            j1: grid.ny,
        }
    }

    /// Posts along `i`.
    pub fn width(&self) -> u64 {
        self.i1 - self.i0
    }

    /// Posts along `j`.
    pub fn height(&self) -> u64 {
        self.j1 - self.j0
    }

    /// The number of posts.
    pub fn len(&self) -> usize {
        (self.width() * self.height()) as usize
    }

    /// A window always holds a post.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Whether post `(i, j)` is in the window.
    pub fn contains(&self, i: u64, j: u64) -> bool {
        (self.i0..self.i1).contains(&i) && (self.j0..self.j1).contains(&j)
    }

    /// Whether every post of `other` is in this window.
    pub fn covers(&self, other: &Window) -> bool {
        self.i0 <= other.i0 && self.j0 <= other.j0 && other.i1 <= self.i1 && other.j1 <= self.j1
    }

    /// Whether the window lies inside the grid.
    pub fn within(&self, grid: &Grid) -> bool {
        self.i1 <= grid.nx && self.j1 <= grid.ny
    }

    /// The window grown by `halo` posts on every side, clipped to the grid (`numbers.md` 6).
    #[must_use]
    pub fn grow(&self, halo: u64, grid: &Grid) -> Window {
        Window {
            i0: self.i0.saturating_sub(halo),
            j0: self.j0.saturating_sub(halo),
            i1: self.i1.saturating_add(halo).min(grid.nx),
            j1: self.j1.saturating_add(halo).min(grid.ny),
        }
    }

    /// The posts both windows hold, if any.
    pub fn intersect(&self, other: &Window) -> Option<Window> {
        Window::new(
            self.i0.max(other.i0),
            self.j0.max(other.j0),
            self.i1.min(other.i1),
            self.j1.min(other.j1),
        )
    }

    /// The offset of post `(i, j)` in a window's values, in post order (rows from the south, each
    /// west to east), or `None` outside it.
    pub fn offset(&self, i: u64, j: u64) -> Option<usize> {
        self.contains(i, j)
            .then(|| ((j - self.j0) * self.width() + (i - self.i0)) as usize)
    }

    /// The posts in post order: `j` outer, `i` inner (`numbers.md` 4).
    pub fn posts(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        (self.j0..self.j1).flat_map(move |j| (self.i0..self.i1).map(move |i| (i, j)))
    }

    /// The window's part of a node key, `w:<i0>.<j0>.<i1>.<j1>` (`build.md` 5, `graph.md` 3.1).
    pub fn key(&self) -> String {
        format!("w:{}.{}.{}.{}", self.i0, self.j0, self.i1, self.j1)
    }
}

/// Post `(i, j)` in millimetres. The caller has the post from a window inside the grid, so this
/// is exact and cannot overflow (`numbers.md` 4).
pub fn post_mm(grid: &Grid, i: u64, j: u64) -> (i64, i64) {
    let r = grid.resolution;
    (grid.extent[0] + i as i64 * r, grid.extent[1] + j as i64 * r)
}

/// The window of a bounded effect (`fields.md` 5.3): the smallest rectangle of posts that covers
/// the shape's plan bounding box `[xmin, ymin, xmax, ymax]` grown by `radius`, cut at the extent.
/// `None` when that rectangle holds no post of the grid.
///
/// `i0 = max(0, floor((xmin - radius - x0) / r))`, `i1 = min(nx, ceil((xmax + radius - x0) / r) + 1)`,
/// and `j0`, `j1` alike, in checked integers.
pub fn effect_window(
    grid: &Grid,
    bounds: [i64; 4],
    radius_mm: i64,
) -> Result<Option<Window>, RasterError> {
    let r = grid.resolution;
    let ov = |op| RasterError::Overflow { op };
    let lo = |v: i64, origin: i64| -> Result<i64, RasterError> {
        let a = v
            .checked_sub(radius_mm)
            .and_then(|a| a.checked_sub(origin))
            .ok_or(ov("effect window"))?;
        Ok(a.div_euclid(r).max(0))
    };
    let hi = |v: i64, origin: i64, n: u64| -> Result<i64, RasterError> {
        let a = v
            .checked_add(radius_mm)
            .and_then(|a| a.checked_sub(origin))
            .ok_or(ov("effect window"))?;
        let c = -((-a).div_euclid(r)); // ceil(a / r)
        Ok(c.saturating_add(1).min(n as i64))
    };
    let [xmin, ymin, xmax, ymax] = bounds;
    let i0 = lo(xmin, grid.extent[0])?;
    let j0 = lo(ymin, grid.extent[1])?;
    let i1 = hi(xmax, grid.extent[0], grid.nx)?;
    let j1 = hi(ymax, grid.extent[1], grid.ny)?;
    if i1 <= i0 || j1 <= j0 {
        return Ok(None);
    }
    Ok(Window::new(i0 as u64, j0 as u64, i1 as u64, j1 as u64))
}

/// Whether a post is on an effect's ring (`fields.md` 5.3): `radius - r < d <= radius`, `d` in
/// metres as `numbers.md` 5.5 gives it, and the post not on the extent's edge. The comparison is
/// in metres, each length one correctly rounded division, as `d` is.
pub fn on_ring(grid: &Grid, i: u64, j: u64, d: f64, radius_mm: i64) -> bool {
    if i == 0 || j == 0 || i + 1 >= grid.nx || j + 1 >= grid.ny {
        return false;
    }
    let outer = via_num::metres(radius_mm);
    let inner = via_num::metres(radius_mm - grid.resolution);
    inner < d && d <= outer
}

/// The longest chunk side by default: 64 m (`build.md` 5).
pub const CHUNK_DEFAULT_MM: i64 = 64_000;

/// The build's tiling (`build.md` 5): square chunks of `k` posts per side, neighbours sharing
/// their edge posts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunks {
    /// Posts per chunk side, `c / resolution`.
    pub k: u64,
    /// Chunks along `x`.
    pub cx: u64,
    /// Chunks along `y`.
    pub cy: u64,
    nx: u64,
    ny: u64,
    side_mm: i64,
    origin: (i64, i64),
}

impl Chunks {
    /// The default tiling: the side is the largest multiple of the resolution not above 64 m, and
    /// at least the resolution. A written `chunk` is M6 (`doc.not_yet`).
    pub fn new(grid: &Grid) -> Chunks {
        let r = grid.resolution;
        let side = if r >= CHUNK_DEFAULT_MM {
            r
        } else {
            (CHUNK_DEFAULT_MM / r) * r
        };
        let k = (side / r) as u64;
        let count = |n: u64| ((n - 1).div_ceil(k)).max(1);
        Chunks {
            k,
            cx: count(grid.nx),
            cy: count(grid.ny),
            nx: grid.nx,
            ny: grid.ny,
            side_mm: side,
            origin: (grid.extent[0], grid.extent[1]),
        }
    }

    /// The chunk side in millimetres.
    pub fn side_mm(&self) -> i64 {
        self.side_mm
    }

    /// The posts of chunk `(cx, cy)` as a window: `i` from `cx * k` to `min((cx + 1) * k, nx - 1)`,
    /// both ends included, and `j` alike. `None` beyond the last chunk.
    pub fn window(&self, cx: u64, cy: u64) -> Option<Window> {
        if cx >= self.cx || cy >= self.cy {
            return None;
        }
        let last = |c: u64, n: u64| ((c + 1) * self.k).min(n - 1);
        Window::new(
            cx * self.k,
            cy * self.k,
            last(cx, self.nx) + 1,
            last(cy, self.ny) + 1,
        )
    }

    /// Every chunk in order, south row first, west to east, with its window.
    pub fn list(&self) -> Vec<((u64, u64), Window)> {
        let mut out = Vec::with_capacity((self.cx * self.cy) as usize);
        for cy in 0..self.cy {
            for cx in 0..self.cx {
                if let Some(w) = self.window(cx, cy) {
                    out.push(((cx, cy), w));
                }
            }
        }
        out
    }

    /// The chunk that holds a plan point in millimetres: `floor((x - x0) / C)` per axis, clamped
    /// to the chunks there are, so a point on the north or east edge is in the last chunk.
    pub fn of_point(&self, x: i64, y: i64) -> Result<(u64, u64), RasterError> {
        let ov = RasterError::Overflow {
            op: "chunk of a point",
        };
        let ax = x.checked_sub(self.origin.0).ok_or(ov.clone())?;
        let ay = y.checked_sub(self.origin.1).ok_or(ov)?;
        let c = |a: i64, n: u64| a.div_euclid(self.side_mm).clamp(0, n as i64 - 1) as u64;
        Ok((c(ax, self.cx), c(ay, self.cy)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(extent: [i64; 4], r: i64) -> Grid {
        Grid::new(extent, r).unwrap()
    }

    #[test]
    fn grow_clips_to_the_grid() {
        let g = grid([0, 0, 8_000, 8_000], 1_000);
        let w = Window::new(0, 3, 2, 9).unwrap();
        assert_eq!(w.grow(1, &g), Window::new(0, 2, 3, 9).unwrap());
        assert_eq!(Window::whole(&g).len(), 81);
        assert_eq!(w.key(), "w:0.3.2.9");
    }

    #[test]
    fn posts_run_in_post_order() {
        let w = Window::new(1, 1, 3, 3).unwrap();
        let p: Vec<_> = w.posts().collect();
        assert_eq!(p, vec![(1, 1), (2, 1), (1, 2), (2, 2)]);
        assert_eq!(w.offset(2, 2), Some(3));
        assert_eq!(w.offset(0, 2), None);
    }

    #[test]
    fn effect_window_matches_the_chapter() {
        // fields.md conformance "bounded": a point at (4, 4), radius 5, extent [0, 0, 16, 16].
        let g = grid([0, 0, 16_000, 16_000], 1_000);
        let w = effect_window(&g, [4_000, 4_000, 4_000, 4_000], 5_000)
            .unwrap()
            .unwrap();
        assert_eq!(w, Window::new(0, 0, 10, 10).unwrap());
        // A radius off the post spacing rounds outward.
        let w = effect_window(&g, [4_000, 4_000, 4_000, 4_000], 2_500)
            .unwrap()
            .unwrap();
        assert_eq!(w, Window::new(1, 1, 8, 8).unwrap());
        // Wholly beyond the extent: no window.
        assert_eq!(
            effect_window(&g, [40_000, 40_000, 41_000, 41_000], 1_000).unwrap(),
            None
        );
    }

    #[test]
    fn ring_leaves_out_the_extent_edge() {
        let g = grid([0, 0, 16_000, 16_000], 1_000);
        assert!(on_ring(&g, 1, 1, 4.5, 5_000));
        assert!(!on_ring(&g, 1, 1, 4.0, 5_000));
        assert!(on_ring(&g, 1, 1, 5.0, 5_000));
        assert!(!on_ring(&g, 0, 1, 4.5, 5_000));
    }

    #[test]
    fn chunks_share_edges_at_256_m_by_half_a_metre() {
        // ford: 256 m at 0.5 m, 513 posts, chunks of 64 m = 128 posts.
        let g = grid([0, 0, 256_000, 256_000], 500);
        let c = Chunks::new(&g);
        assert_eq!((c.k, c.cx, c.cy), (128, 4, 4));
        assert_eq!(c.window(0, 0), Window::new(0, 0, 129, 129));
        assert_eq!(c.window(3, 3), Window::new(384, 384, 513, 513));
        assert_eq!(c.window(4, 0), None);
        assert_eq!(c.of_point(256_000, 0).unwrap(), (3, 0));
        assert_eq!(c.of_point(64_000, 63_999).unwrap(), (1, 0));
        // A narrow last chunk, and a world smaller than a chunk.
        let g = grid([0, 0, 70_000, 10_000], 1_000);
        let c = Chunks::new(&g);
        assert_eq!((c.cx, c.cy), (2, 1));
        assert_eq!(c.window(1, 0), Window::new(64, 0, 71, 11));
        // A resolution above 64 m is one post per chunk side.
        let g = grid([0, 0, 200_000, 100_000], 100_000);
        let c = Chunks::new(&g);
        assert_eq!((c.k, c.cx, c.cy), (1, 2, 1));
    }
}
