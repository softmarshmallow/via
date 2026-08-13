/// Regular square grid. Cell (x, y) ↔ index `y * w + x`; y grows southward.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
}

/// D8 neighbourhood in fixed scan order (NW, N, NE, W, E, SW, S, SE) with the
/// distance factor (× dx) to each neighbour. The order is part of the
/// determinism contract.
pub const D8: [(i32, i32, f64); 8] = [
    (-1, -1, std::f64::consts::SQRT_2),
    (0, -1, 1.0),
    (1, -1, std::f64::consts::SQRT_2),
    (-1, 0, 1.0),
    (1, 0, 1.0),
    (-1, 1, std::f64::consts::SQRT_2),
    (0, 1, 1.0),
    (1, 1, std::f64::consts::SQRT_2),
];

impl Grid {
    pub fn new(size: u32, dx: f64) -> Self {
        Self {
            w: size,
            h: size,
            dx,
        }
    }

    #[inline]
    pub fn n(&self) -> usize {
        self.w as usize * self.h as usize
    }

    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> u32 {
        y * self.w + x
    }

    #[inline]
    pub fn xy(&self, i: u32) -> (u32, u32) {
        (i % self.w, i / self.w)
    }

    #[inline]
    pub fn is_border(&self, i: u32) -> bool {
        let (x, y) = self.xy(i);
        x == 0 || y == 0 || x == self.w - 1 || y == self.h - 1
    }

    /// Visit the in-bounds D8 neighbours of `i` in fixed order.
    /// `f(neighbor_index, distance_factor)`.
    #[inline]
    pub fn for_neighbors(&self, i: u32, mut f: impl FnMut(u32, f64)) {
        let (x, y) = self.xy(i);
        for &(dxi, dyi, fac) in &D8 {
            let nx = x as i64 + dxi as i64;
            let ny = y as i64 + dyi as i64;
            if nx >= 0 && ny >= 0 && nx < self.w as i64 && ny < self.h as i64 {
                f(self.idx(nx as u32, ny as u32), fac);
            }
        }
    }

    /// Distance in metres between two D8-adjacent cells.
    #[inline]
    pub fn step_dist_m(&self, a: u32, b: u32) -> f64 {
        let (ax, ay) = self.xy(a);
        let (bx, by) = self.xy(b);
        let diag = ax != bx && ay != by;
        if diag {
            self.dx * std::f64::consts::SQRT_2
        } else {
            self.dx
        }
    }
}
