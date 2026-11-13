//! `slope` (`formulas.md` 8.3): a function with a halo of one post (R28, `fields.md` 6.3).

use crate::error::RasterError;
use crate::raster::{grid_cell, Raster};
use crate::window::{post_mm, Window};
use via_num::Grid;

/// The halo `slope` declares: one post.
pub const SLOPE_HALO: u64 = 1;

/// The slope in degrees, 0 to 90, at every post of `window`, from a resolved field `h` that holds
/// `window` grown by [`SLOPE_HALO`] (clipped to the grid).
///
/// At post `(i, j)`, with `r` the spacing in metres: `gx = (h[i+1, j] - h[i-1, j]) / (2 * r)`, and
/// `gy` likewise along `j`. Where a neighbour lies beyond the extent (never beyond the window: the
/// window's halo holds it) the difference is one-sided, `(h[i+1, j] - h[i, j]) / r` or
/// `(h[i, j] - h[i-1, j]) / r`. With no neighbour on either side there is no value
/// (`formula.outside_field`). Then `atan(sqrt(gx * gx + gy * gy)) * (180 / pi)`, `atan` from
/// `via-num`. The same edge rule whole or windowed, so a window equals the same cut.
pub fn slope(grid: &Grid, h: &Raster<f32>, window: Window) -> Result<Raster<f64>, RasterError> {
    let need = window.grow(SLOPE_HALO, grid);
    if !window.within(grid) || !h.window().covers(&need) {
        return Err(RasterError::Internal {
            reason: format!(
                "slope over {} needs {}, the field holds {}",
                window.key(),
                need.key(),
                h.window().key()
            ),
        });
    }
    let r = via_num::metres(grid.resolution);
    let two_r = 2.0 * r;
    let at = |i: u64, j: u64| f64::from(h.get(i, j).unwrap_or(0.0));
    let diff = |c: u64, n: u64, get: &dyn Fn(u64) -> f64| -> Option<f64> {
        let lo = c.checked_sub(1);
        let hi = (c + 1 < n).then_some(c + 1);
        match (lo, hi) {
            (Some(lo), Some(hi)) => Some((get(hi) - get(lo)) / two_r),
            (None, Some(hi)) => Some((get(hi) - get(c)) / r),
            (Some(lo), None) => Some((get(c) - get(lo)) / r),
            (None, None) => None,
        }
    };
    Raster::from_fn(window, |i, j| {
        let outside = || RasterError::OutsideField {
            at: post_mm(grid, i, j),
        };
        let gx = diff(i, grid.nx, &|ii| at(ii, j)).ok_or_else(outside)?;
        let gy = diff(j, grid.ny, &|jj| at(i, jj)).ok_or_else(outside)?;
        let s = via_num::atan((gx * gx + gy * gy).sqrt()) * via_num::DEGREES_PER_RAD;
        via_num::finite(s).map_err(|e| RasterError::from_num(e, post_mm(grid, i, j)))
    })
}

/// `slope` at a point that is not a post (`fields.md` 1.2, "A function whose halo is in posts"):
/// the bilinear interpolation of its values at the four posts around the point. `slopes` must hold
/// those posts.
pub fn slope_at(grid: &Grid, slopes: &Raster<f64>, x: i64, y: i64) -> Result<f64, RasterError> {
    crate::raster::read_at(grid, slopes, x, y)
}

/// The window of posts a read at a point needs: the grid cell 1.2 picks.
pub fn cell_window(grid: &Grid, x: i64, y: i64) -> Result<Window, RasterError> {
    let [x0, y0, x1, y1] = grid.extent;
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return Err(RasterError::OutsideField { at: (x, y) });
    }
    let (i, _) = grid_cell(x - x0, grid.resolution, grid.nx);
    let (j, _) = grid_cell(y - y0, grid.resolution, grid.ny);
    Window::new(i, j, (i + 2).min(grid.nx), (j + 2).min(grid.ny))
        .ok_or(RasterError::OutsideField { at: (x, y) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plane_has_one_slope_and_edges_are_one_sided() {
        // h = x (1 m per m): 45 degrees everywhere, edges included.
        let g = Grid::new([0, 0, 8_000, 4_000], 500).unwrap();
        let w = Window::whole(&g);
        let h = Raster::from_fn(w, |i, _| Ok::<_, ()>(i as f32 * 0.5)).unwrap();
        let s = slope(&g, &h, w).unwrap();
        for v in s.values() {
            assert!((v - 45.0).abs() < 1e-12, "{v}");
        }
    }

    #[test]
    fn needs_its_halo() {
        let g = Grid::new([0, 0, 8_000, 4_000], 500).unwrap();
        let h = Raster::filled(Window::new(2, 2, 5, 5).unwrap(), 0.0f32);
        assert!(matches!(
            slope(&g, &h, Window::new(2, 2, 5, 5).unwrap()),
            Err(RasterError::Internal { .. })
        ));
        assert!(slope(&g, &h, Window::new(3, 3, 4, 4).unwrap()).is_ok());
    }

    #[test]
    fn window_equals_cut_at_every_edge() {
        // numbers.md "halo_edge": windows touching each grid edge give the whole's bits.
        let g = Grid::new([0, 0, 12_000, 7_000], 500).unwrap();
        let all = Window::whole(&g);
        let h = Raster::from_fn(all, |i, j| {
            Ok::<_, ()>((via_num::sin(i as f64 * 0.3) * 4.0 + (j * j) as f64 * 0.02) as f32)
        })
        .unwrap();
        let whole = slope(&g, &h, all).unwrap();
        for w in [
            Window::new(0, 0, 1, 1).unwrap(),
            Window::new(0, 3, 4, 9).unwrap(),
            Window::new(20, 0, 25, 2).unwrap(),
            Window::new(10, 12, 25, 15).unwrap(),
            Window::new(24, 14, 25, 15).unwrap(),
            Window::new(7, 3, 14, 6).unwrap(),
        ] {
            let input = h.cut(w.grow(SLOPE_HALO, &g)).unwrap();
            let part = slope(&g, &input, w).unwrap();
            for (i, j) in w.posts() {
                assert_eq!(
                    part.get(i, j).map(f64::to_bits),
                    whole.get(i, j).map(f64::to_bits)
                );
            }
        }
    }

    #[test]
    fn at_a_point_interpolates_the_post_values() {
        let g = Grid::new([0, 0, 4_000, 4_000], 1_000).unwrap();
        let w = Window::whole(&g);
        let h = Raster::from_fn(w, |i, j| Ok::<_, ()>((i * i + j) as f32)).unwrap();
        let s = slope(&g, &h, w).unwrap();
        let cell = cell_window(&g, 1_500, 2_000).unwrap();
        assert_eq!(cell, Window::new(1, 2, 3, 4).unwrap());
        let v = slope_at(&g, &s, 1_500, 2_000).unwrap();
        let mid = 0.5 * s.get(1, 2).unwrap() + 0.5 * s.get(2, 2).unwrap();
        assert_eq!(v.to_bits(), mid.to_bits());
    }
}
