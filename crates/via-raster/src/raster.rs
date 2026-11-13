//! Values at the posts of a window, and reading them between posts (`fields.md` 1.1, 1.2).
//!
//! A [`Raster<f64>`] holds a field's working values (`v` between rows, a formula's values); a
//! [`Raster<f32>`] holds a resolved field, narrowed once after its last entry; a [`Raster<u8>`]
//! is a cover (1 inside, 0 outside).

use crate::error::RasterError;
use crate::window::{post_mm, Window};
use via_num::Grid;

/// Values at every post of a window, in post order.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster<T> {
    window: Window,
    values: Vec<T>,
}

impl<T: Copy> Raster<T> {
    /// Every post holds `v`.
    pub fn filled(window: Window, v: T) -> Raster<T> {
        Raster {
            window,
            values: vec![v; window.len()],
        }
    }

    /// Values given in post order; `None` when their count is not the window's.
    pub fn from_values(window: Window, values: Vec<T>) -> Option<Raster<T>> {
        (values.len() == window.len()).then_some(Raster { window, values })
    }

    /// Each post's value from `f(i, j)`, in post order. The first error stops it.
    pub fn from_fn<E>(
        window: Window,
        mut f: impl FnMut(u64, u64) -> Result<T, E>,
    ) -> Result<Raster<T>, E> {
        let mut values = Vec::with_capacity(window.len());
        for (i, j) in window.posts() {
            values.push(f(i, j)?);
        }
        Ok(Raster { window, values })
    }

    /// The window.
    pub fn window(&self) -> Window {
        self.window
    }

    /// The values in post order.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The values in post order, to change in place.
    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    /// The value at post `(i, j)` of the grid, if this raster holds it.
    pub fn get(&self, i: u64, j: u64) -> Option<T> {
        self.window.offset(i, j).map(|k| self.values[k])
    }

    /// Sets the value at post `(i, j)`; false when this raster does not hold it.
    pub fn set(&mut self, i: u64, j: u64, v: T) -> bool {
        match self.window.offset(i, j) {
            Some(k) => {
                self.values[k] = v;
                true
            }
            None => false,
        }
    }

    /// The same values over a smaller window (a cut), or `None` when `window` is not inside.
    pub fn cut(&self, window: Window) -> Option<Raster<T>> {
        if !self.window.covers(&window) {
            return None;
        }
        let mut values = Vec::with_capacity(window.len());
        for j in window.j0..window.j1 {
            let a = self.window.offset(window.i0, j)?;
            values.extend_from_slice(&self.values[a..a + window.width() as usize]);
        }
        Some(Raster { window, values })
    }

    /// One raster over `window` from parts that together cover it. A post several parts hold is
    /// taken from the first in the order given (`build.md` 5: a shared post from the first chunk
    /// in chunk order). `None` when some post is in no part.
    pub fn assemble(window: Window, parts: &[&Raster<T>]) -> Option<Raster<T>> {
        let mut values: Vec<Option<T>> = vec![None; window.len()];
        for part in parts {
            let Some(w) = part.window.intersect(&window) else {
                continue;
            };
            for (i, j) in w.posts() {
                let k = window.offset(i, j)?;
                if values[k].is_none() {
                    values[k] = part.get(i, j);
                }
            }
        }
        let values: Option<Vec<T>> = values.into_iter().collect();
        values.map(|values| Raster { window, values })
    }

    /// Each value mapped by `f`.
    pub fn map<U: Copy>(&self, f: impl Fn(T) -> U) -> Raster<U> {
        Raster {
            window: self.window,
            values: self.values.iter().map(|&v| f(v)).collect(),
        }
    }
}

/// Narrows a field's working values to 32-bit floats, once, round to nearest, ties to even, after
/// the store guard (`fields.md` 1.1, `numbers.md` 8 rules 7 to 9). A value that is not finite,
/// or beyond the float's range, is `num.not_finite_result` at the first such post in post order.
pub fn narrow(grid: &Grid, values: &Raster<f64>) -> Result<Raster<f32>, RasterError> {
    let w = values.window();
    Raster::from_fn(w, |i, j| {
        let v = values.values[w.offset(i, j).unwrap_or(0)];
        via_num::store_f32(v).map_err(|e| RasterError::from_num(e, post_mm(grid, i, j)))
    })
}

/// A resolved field widened to doubles, which is exact (`fields.md` 1.1).
pub fn widen(values: &Raster<f32>) -> Raster<f64> {
    values.map(f64::from)
}

/// The grid cell of 1.2 along one axis, and the fraction in it. `a` is the point's offset from the
/// extent's origin in millimetres, `0 <= a <= (n - 1) * r`, `n >= 2`. The cell is
/// `min(floor(a / r), n - 2)` in integers; the fraction `(a - i * r) as double / r as double`.
pub fn grid_cell(a: i64, r: i64, n: u64) -> (u64, f64) {
    let i = (a.div_euclid(r) as u64).min(n.saturating_sub(2));
    let rest = a - i as i64 * r;
    (i, (rest as f64) / (r as f64))
}

/// The bilinear rule of 1.2, in doubles, in this order, with no fused multiply-add:
/// `s = (1 - fx) * p00 + fx * p10`, `n = (1 - fx) * p01 + fx * p11`, `(1 - fy) * s + fy * n`.
pub fn bilinear(p00: f64, p10: f64, p01: f64, p11: f64, fx: f64, fy: f64) -> f64 {
    let s = (1.0 - fx) * p00 + fx * p10;
    let n = (1.0 - fx) * p01 + fx * p11;
    (1.0 - fy) * s + fy * n
}

/// A field read at a point `(x, y)` in integer millimetres (`fields.md` 1.2). The raster must hold
/// the four posts around the point; a point beyond the extent is `formula.outside_field`.
pub fn read_at<T: Copy + Into<f64>>(
    grid: &Grid,
    values: &Raster<T>,
    x: i64,
    y: i64,
) -> Result<f64, RasterError> {
    let [x0, y0, x1, y1] = grid.extent;
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return Err(RasterError::OutsideField { at: (x, y) });
    }
    let r = grid.resolution;
    let (i, fx) = grid_cell(x - x0, r, grid.nx);
    let (j, fy) = grid_cell(y - y0, r, grid.ny);
    let at = |i: u64, j: u64| -> Result<f64, RasterError> {
        values
            .get(i, j)
            .map(Into::into)
            .ok_or_else(|| RasterError::Internal {
                reason: format!("post ({i}, {j}) is not in {}", values.window().key()),
            })
    };
    // A grid one post wide along an axis has no second post; its fraction is then 0.
    let i1 = (i + 1).min(grid.nx - 1);
    let j1 = (j + 1).min(grid.ny - 1);
    Ok(bilinear(
        at(i, j)?,
        at(i1, j)?,
        at(i, j1)?,
        at(i1, j1)?,
        fx,
        fy,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    #[test]
    fn cut_equals_the_same_posts() {
        let g = g8();
        let whole =
            Raster::from_fn(Window::whole(&g), |i, j| Ok::<_, ()>((i * 100 + j) as f64)).unwrap();
        let w = Window::new(2, 3, 5, 4).unwrap();
        let c = whole.cut(w).unwrap();
        for (i, j) in w.posts() {
            assert_eq!(c.get(i, j), whole.get(i, j));
        }
        assert!(c.cut(Window::new(0, 0, 1, 1).unwrap()).is_none());
    }

    #[test]
    fn assemble_takes_shared_posts_from_the_first() {
        let a = Raster::filled(Window::new(0, 0, 3, 2).unwrap(), 1.0);
        let b = Raster::filled(Window::new(2, 0, 5, 2).unwrap(), 2.0);
        let r = Raster::assemble(Window::new(0, 0, 5, 2).unwrap(), &[&a, &b]).unwrap();
        assert_eq!(r.get(2, 0), Some(1.0));
        assert_eq!(r.get(3, 1), Some(2.0));
        assert!(Raster::assemble(Window::new(0, 0, 6, 2).unwrap(), &[&a, &b]).is_none());
    }

    #[test]
    fn narrowing_rounds_once_and_refuses_overflow() {
        let g = g8();
        let w = Window::new(0, 0, 1, 1).unwrap();
        let third = Raster::filled(w, 1.0 / 3.0);
        assert_eq!(narrow(&g, &third).unwrap().get(0, 0), Some(1.0f32 / 3.0));
        // fields.md "narrowing overflow": 1e39 names the post.
        let big = Raster::filled(w, 1e39);
        match narrow(&g, &big) {
            Err(RasterError::NotFinite { at, .. }) => assert_eq!(at, (0, 0)),
            other => panic!("{other:?}"),
        }
        let neg_zero = Raster::filled(w, -0.0);
        assert!(!narrow(&g, &neg_zero).unwrap().values()[0].is_sign_negative());
    }

    #[test]
    fn between_posts_is_bilinear() {
        // fields.md "between posts": base x * x, at (2.5, 0) reads 6.5, not 6.25.
        let g = g8();
        let h = Raster::from_fn(Window::whole(&g), |i, _| Ok::<_, ()>((i * i) as f32)).unwrap();
        assert_eq!(read_at(&g, &h, 2_500, 0).unwrap(), 6.5);
        // On a post: the post's value bit for bit; on the last column: clamped to the west cell.
        assert_eq!(read_at(&g, &h, 3_000, 4_000).unwrap(), 9.0);
        assert_eq!(read_at(&g, &h, 8_000, 8_000).unwrap(), 64.0);
        assert_eq!(grid_cell(2_000, 1_000, 9), (2, 0.0));
        assert_eq!(grid_cell(8_000, 1_000, 9), (7, 1.0));
        assert!(matches!(
            read_at(&g, &h, 8_001, 0),
            Err(RasterError::OutsideField { .. })
        ));
    }
}
