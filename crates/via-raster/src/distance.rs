//! Capped distance to shapes at every post of a window (`formulas.md` 8.4, `numbers.md` 5.5).
//!
//! The measures themselves (the exact nearest point, `d` and `sd`) are `numbers.md` 5.5's and
//! belong to the geometry kernel: `via-geom` implements [`Measured`] for its points, paths and
//! regions, and this crate only fills windows with them, folds a selection and applies the cap.
//! The halo is the cap: a post farther than the cap from a shape's bounding box takes the cap
//! without measuring, which is exact (`d > cap` there), so the work per window is bounded by the
//! geometry within the cap, never the whole extent.

use crate::error::RasterError;
use crate::raster::Raster;
use crate::window::{post_mm, Window};
use via_num::Grid;

/// The three shapes (`geometry.md` 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeClass {
    Point,
    Path,
    Region,
}

/// A feature's measures at a lattice point, in metres (`numbers.md` 5.5): `d >= 0`, and `sd` with
/// its sign (`+` left of a path's travel and inside a region).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure {
    pub d: f64,
    pub sd: f64,
}

/// A shape that gives `numbers.md` 5.5's measures. Implemented by the geometry kernel.
pub trait Measured {
    /// Point, path or region.
    fn class(&self) -> ShapeClass;
    /// The plan bounding box `[xmin, ymin, xmax, ymax]` in millimetres.
    fn bounds(&self) -> [i64; 4];
    /// `d` and `sd` at a lattice point, exactly as 5.5 computes them.
    fn measure(&self, x: i64, y: i64) -> Measure;
}

/// The cap in whole millimetres, rounded up, or `None` when it is beyond the lattice (no
/// culling then).
fn cap_mm(cap: f64) -> Option<i64> {
    let c = (cap * 1000.0).ceil();
    (c <= 4.0 * via_num::LATTICE_MAX as f64).then_some(c as i64)
}

/// Whether a post is farther than `cap_mm` from a box along either axis, so `d > cap` there.
fn beyond(b: [i64; 4], cap_mm: Option<i64>, x: i64, y: i64) -> bool {
    let Some(c) = cap_mm else { return false };
    let gap = |v: i64, lo: i64, hi: i64| {
        i128::from(lo) - i128::from(v) > i128::from(c)
            || i128::from(v) - i128::from(hi) > i128::from(c)
    };
    gap(x, b[0], b[2]) || gap(y, b[1], b[3])
}

fn check_cap(cap: f64) -> Result<f64, RasterError> {
    if cap.is_finite() && cap > 0.0 {
        Ok(cap)
    } else {
        Err(RasterError::ParameterRange {
            what: "cap",
            value: cap,
        })
    }
}

/// How many posts a distance capped at `cap` metres reaches, rounded up: the halo a caller grows a
/// window by when it reads geometry within the cap (`numbers.md` 6).
pub fn halo_posts(grid: &Grid, cap: f64) -> Result<u64, RasterError> {
    let c = cap_mm(check_cap(cap)?).ok_or(RasterError::ParameterRange {
        what: "cap",
        value: cap,
    })?;
    Ok(c.div_euclid(grid.resolution) as u64 + 1)
}

/// `dist(shape, cap)` at every post of `window` (`formulas.md` 8.4): the least `d` over the
/// selection, folded in the order given (the caller's id order) by `numbers.md` 8 rule 4, then at
/// most `cap`.
pub fn distance(
    grid: &Grid,
    window: Window,
    shapes: &[&dyn Measured],
    cap: f64,
) -> Result<Raster<f64>, RasterError> {
    let cap = check_cap(cap)?;
    if shapes.is_empty() {
        return Err(RasterError::EmptySelection);
    }
    let c = cap_mm(cap);
    let boxes: Vec<[i64; 4]> = shapes.iter().map(|s| s.bounds()).collect();
    Raster::from_fn(window, |i, j| {
        let (x, y) = post_mm(grid, i, j);
        let mut acc = cap;
        for (s, b) in shapes.iter().zip(&boxes) {
            if !beyond(*b, c, x, y) {
                acc = via_num::min(acc, s.measure(x, y).d);
            }
        }
        via_num::store(acc).map_err(|e| RasterError::from_num(e, (x, y)))
    })
}

/// `sdist(shape, cap)` at every post of `window` (`formulas.md` 8.4): `sd` in `-cap` to `cap`.
/// Regions: the greatest `sd` among them. One path: its `sd`. Points: minus `dist`. A selection
/// mixing classes is `formula.shape`; several paths is `formula.selector_count`.
pub fn signed_distance(
    grid: &Grid,
    window: Window,
    shapes: &[&dyn Measured],
    cap: f64,
) -> Result<Raster<f64>, RasterError> {
    let cap = check_cap(cap)?;
    let Some(first) = shapes.first() else {
        return Err(RasterError::EmptySelection);
    };
    let class = first.class();
    if shapes.iter().any(|s| s.class() != class) {
        return Err(RasterError::Shape {
            reason: "a signed distance over a selection mixing regions, paths and points",
        });
    }
    if class == ShapeClass::Path && shapes.len() > 1 {
        return Err(RasterError::SelectorCount {
            paths: shapes.len(),
        });
    }
    let c = cap_mm(cap);
    let boxes: Vec<[i64; 4]> = shapes.iter().map(|s| s.bounds()).collect();
    Raster::from_fn(window, |i, j| {
        let (x, y) = post_mm(grid, i, j);
        let mut acc = -cap;
        for (s, b) in shapes.iter().zip(&boxes) {
            // Beyond the cap a region or a point is outside (sd < -cap); a path's side is not
            // known without measuring, so a path is always measured.
            if class != ShapeClass::Path && beyond(*b, c, x, y) {
                continue;
            }
            acc = via_num::max(acc, s.measure(x, y).sd);
        }
        via_num::store(via_num::clamp(acc, -cap, cap)).map_err(|e| RasterError::from_num(e, (x, y)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point, measured as 5.5 says: `e = sqrt(dot(w, w) as f64) / 1000`, `sd = -e`.
    struct P(i64, i64);

    impl Measured for P {
        fn class(&self) -> ShapeClass {
            ShapeClass::Point
        }
        fn bounds(&self) -> [i64; 4] {
            [self.0, self.1, self.0, self.1]
        }
        fn measure(&self, x: i64, y: i64) -> Measure {
            let (dx, dy) = (i128::from(x - self.0), i128::from(y - self.1));
            let e = ((dx * dx + dy * dy) as f64).sqrt() / 1000.0;
            Measure { d: e, sd: -e }
        }
    }

    /// An axis-aligned box region: 0 inside, distance outside (a stand-in for the kernel's).
    struct B([i64; 4]);

    impl Measured for B {
        fn class(&self) -> ShapeClass {
            ShapeClass::Region
        }
        fn bounds(&self) -> [i64; 4] {
            self.0
        }
        fn measure(&self, x: i64, y: i64) -> Measure {
            let [a, b, c, d] = self.0;
            let dx = (a - x).max(0).max(x - c);
            let dy = (b - y).max(0).max(y - d);
            let out = (((dx * dx + dy * dy) as i128) as f64).sqrt() / 1000.0;
            if out > 0.0 {
                Measure { d: out, sd: -out }
            } else {
                let depth = (x - a).min(c - x).min(y - b).min(d - y) as f64 / 1000.0;
                Measure { d: 0.0, sd: depth }
            }
        }
    }

    #[test]
    fn culling_by_the_cap_changes_no_bit() {
        let g = Grid::new([0, 0, 64_000, 48_000], 500).unwrap();
        let shapes = [P(10_000, 10_000), P(40_250, 31_000)];
        let refs: Vec<&dyn Measured> = shapes.iter().map(|s| s as &dyn Measured).collect();
        let w = Window::whole(&g);
        let d = distance(&g, w, &refs, 6.0).unwrap();
        for (i, j) in w.posts() {
            let (x, y) = post_mm(&g, i, j);
            let true_d = shapes
                .iter()
                .map(|s| s.measure(x, y).d)
                .fold(6.0, via_num::min);
            assert_eq!(d.get(i, j).unwrap().to_bits(), true_d.to_bits());
        }
        assert_eq!(d.get(20, 20), Some(0.0));
        assert_eq!(d.get(0, 0), Some(6.0));
    }

    #[test]
    fn window_equals_cut() {
        let g = Grid::new([0, 0, 64_000, 48_000], 500).unwrap();
        let shapes: Vec<Box<dyn Measured>> = vec![
            Box::new(B([20_000, 20_000, 30_000, 26_000])),
            Box::new(B([31_000, 10_000, 33_000, 40_000])),
        ];
        let refs: Vec<&dyn Measured> = shapes.iter().map(|s| s.as_ref()).collect();
        let whole = signed_distance(&g, Window::whole(&g), &refs, 4.0).unwrap();
        let dwhole = distance(&g, Window::whole(&g), &refs, 4.0).unwrap();
        for w in [
            Window::new(0, 0, 1, 1).unwrap(),
            Window::new(30, 30, 101, 47).unwrap(),
            Window::new(55, 15, 70, 90).unwrap(),
        ] {
            assert_eq!(
                signed_distance(&g, w, &refs, 4.0).unwrap(),
                whole.cut(w).unwrap()
            );
            assert_eq!(distance(&g, w, &refs, 4.0).unwrap(), dwhole.cut(w).unwrap());
        }
        // Inside the first box at its middle: depth 3 m; the cap bounds both signs.
        assert_eq!(whole.get(50, 46), Some(3.0));
        assert_eq!(whole.get(0, 0), Some(-4.0));
    }

    #[test]
    fn selections_checked() {
        let g = Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap();
        let w = Window::whole(&g);
        let p = P(0, 0);
        let b = B([1_000, 1_000, 2_000, 2_000]);
        let mixed: Vec<&dyn Measured> = vec![&p, &b];
        assert_eq!(
            signed_distance(&g, w, &mixed, 1.0).map_err(|e| e.code().as_str()),
            Err("formula.shape")
        );
        assert_eq!(
            distance(&g, w, &[], 1.0).map_err(|e| e.code().as_str()),
            Err("formula.empty_selection")
        );
        assert_eq!(
            distance(&g, w, &mixed, 0.0).map_err(|e| e.code().as_str()),
            Err("formula.range")
        );
        assert_eq!(halo_posts(&g, 2.5).unwrap(), 3);
    }
}
