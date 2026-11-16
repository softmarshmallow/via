//! The build's tiling (`build.md` 5): square chunks over the posts, neighbours sharing their edge
//! posts, and the chunk of a plan point in integer arithmetic.

use via_num::Grid;

/// The default chunk side in millimetres, when the world writes no `chunk`: the largest multiple
/// of `resolution` not above 64 m, and at least `resolution` (`build.md` 5).
pub const DEFAULT_SIDE_MM: i64 = 64_000;

/// One chunk: its index and the posts it holds, ends included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub cx: u64,
    pub cy: u64,
    pub i0: u64,
    pub j0: u64,
    pub i1: u64,
    pub j1: u64,
}

impl Chunk {
    /// Posts per row and rows.
    pub fn columns(&self) -> u64 {
        self.i1 - self.i0 + 1
    }

    pub fn rows(&self) -> u64 {
        self.j1 - self.j0 + 1
    }
}

/// Every chunk of a grid, south row first, west to east.
#[derive(Clone, Debug)]
pub struct Layout {
    pub side_mm: i64,
    pub ncx: u64,
    pub ncy: u64,
    pub chunks: Vec<Chunk>,
    origin: (i64, i64),
}

impl Layout {
    /// The default tiling (`build.md` 5).
    #[cfg(test)]
    pub fn new(grid: &Grid) -> Layout {
        Layout::with_side(grid, None)
    }

    /// The tiling with the world's `chunk` (M6): a side that is not a positive whole multiple of
    /// `resolution` (`build.chunk_not_multiple`, the model's) falls back to the default.
    pub fn with_side(grid: &Grid, side: Option<i64>) -> Layout {
        let r = grid.resolution;
        let side_mm = match side {
            Some(s) if s >= r && s % r == 0 => s,
            _ => ((DEFAULT_SIDE_MM / r) * r).max(r),
        };
        let k = u64::try_from(side_mm / r).unwrap_or(1).max(1);
        let count = |n: u64| (n.saturating_sub(1)).div_ceil(k).max(1);
        let (ncx, ncy) = (count(grid.nx), count(grid.ny));
        let mut chunks = Vec::new();
        for cy in 0..ncy {
            for cx in 0..ncx {
                chunks.push(Chunk {
                    cx,
                    cy,
                    i0: cx * k,
                    j0: cy * k,
                    i1: ((cx + 1) * k).min(grid.nx - 1),
                    j1: ((cy + 1) * k).min(grid.ny - 1),
                });
            }
        }
        Layout {
            side_mm,
            ncx,
            ncy,
            chunks,
            origin: (grid.extent[0], grid.extent[1]),
        }
    }

    /// The index in [`Layout::chunks`] of the chunk holding a plan point, clamped to the grid.
    pub fn chunk_of(&self, x: i64, y: i64) -> usize {
        let idx = |v: i64, o: i64, n: u64| -> u64 {
            let c = (v - o).div_euclid(self.side_mm).max(0);
            u64::try_from(c).unwrap_or(0).min(n - 1)
        };
        let cx = idx(x, self.origin.0, self.ncx);
        let cy = idx(y, self.origin.1, self.ncy);
        usize::try_from(cy * self.ncx + cx).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worked_example_two_chunks_share_a_column() {
        let g = Grid::new([0, 0, 128_000, 64_000], 2_000).unwrap();
        let l = Layout::new(&g);
        assert_eq!((l.side_mm, l.ncx, l.ncy), (64_000, 2, 1));
        assert_eq!((l.chunks[0].i0, l.chunks[0].i1), (0, 32));
        assert_eq!((l.chunks[1].i0, l.chunks[1].i1), (32, 64));
        assert_eq!(l.chunk_of(100_000, 30_000), 1);
        assert_eq!(l.chunk_of(128_000, 64_000), 1);
    }

    #[test]
    fn tiny_grid_has_one_chunk() {
        let g = Grid::new([0, 0, 2_000, 2_000], 1_000).unwrap();
        let l = Layout::new(&g);
        assert_eq!(l.chunks.len(), 1);
        assert_eq!(l.chunks[0].i1, 2);
    }
}
