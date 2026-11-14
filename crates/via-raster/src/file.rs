//! A base from a file (`fields.md` 4.3): sampling a raster's own grid onto the world's posts.

use crate::error::RasterError;
use crate::npy::Samples;
use crate::raster::{bilinear, Raster};
use crate::window::{post_mm, Window};
use via_num::Grid;

/// The filters of M1. `cubic` is M2 (`doc.not_yet`, the caller's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Linear,
    /// Keys cubic convolution, `a = -0.5` (Keys, 1981), over the 4 by 4 samples around (M2).
    Cubic,
}

/// Keys' weights for the four samples at offsets -1, 0, 1, 2 from the cell, at fraction `t`
/// (`a = -0.5`): 1, 0, 0, 0 at `t = 0`.
fn keys_weights(t: f64) -> [f64; 4] {
    let a = -0.5;
    let k = |x: f64| {
        let x = x.abs();
        if x <= 1.0 {
            ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
        } else if x < 2.0 {
            ((a * x - 5.0 * a) * x + 8.0 * a) * x - 4.0 * a
        } else {
            0.0
        }
    };
    [k(1.0 + t), k(t), k(1.0 - t), k(2.0 - t)]
}

/// Where the raster's corner samples lie, in millimetres, and the value beyond it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// `[x0, y0, x1, y1]`: the raster's corners, `x1 > x0`, `y1 > y0` (`num.extent_empty` is the
    /// caller's, from `numbers.md` 4).
    pub extent: [i64; 4],
    /// The value at posts beyond `extent`.
    pub outside: f64,
    pub filter: Filter,
}

/// Whether the raster's extent lies wholly outside the world's (`field.file_offworld`): every post
/// then takes `outside`.
pub fn offworld(grid: &Grid, extent: [i64; 4]) -> bool {
    let [a0, b0, a1, b1] = grid.extent;
    let [x0, y0, x1, y1] = extent;
    x1 < a0 || x0 > a1 || y1 < b0 || y0 > b1
}

/// One axis of 4.3: `N = (X - x0) * (W - 1)` exact in 128 bits, `D = x1 - x0`.
struct Axis {
    n: i128,
    d: i128,
    w: i128,
}

impl Axis {
    /// `linear`: grid cell `c = min(floor(N / D), W - 2)`, fraction `(N - c * D) / D` in doubles.
    fn linear(&self) -> (u64, f64) {
        let c = (self.n / self.d).min(self.w - 2);
        let f = ((self.n - c * self.d) as f64) / (self.d as f64);
        (c as u64, f)
    }

    /// `nearest`: `floor((2 * N + D) / (2 * D))`, a tie going to the higher index.
    fn nearest(&self) -> u64 {
        ((2 * self.n + self.d) / (2 * self.d)) as u64
    }
}

/// A field's base from samples, at every post of `window` (`fields.md` 4.3). The samples are posts
/// of their own grid, `W` columns from `x0` to `x1` and `H` rows from `y0` to `y1`, corners
/// included. A post inside the extent, edges included, reads the samples by the filter; one beyond
/// it takes `outside`. Each value depends on the post alone, so a window equals the same cut.
pub fn resample(
    grid: &Grid,
    window: Window,
    samples: &Samples,
    at: Placement,
) -> Result<Raster<f64>, RasterError> {
    let [x0, y0, x1, y1] = at.extent;
    if x1 <= x0 || y1 <= y0 || samples.columns < 2 || samples.rows < 2 {
        return Err(RasterError::Internal {
            reason: "resample needs a non-empty extent and at least 2 by 2 samples".into(),
        });
    }
    let outside = via_num::finite(at.outside).map_err(|e| RasterError::from_num(e, (x0, y0)))?;
    let w = i128::from(samples.columns as i64);
    let h = i128::from(samples.rows as i64);
    Raster::from_fn(window, |i, j| {
        let (x, y) = post_mm(grid, i, j);
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return Ok(outside);
        }
        let ax = Axis {
            n: i128::from(x - x0) * (w - 1),
            d: i128::from(x1 - x0),
            w,
        };
        let ay = Axis {
            n: i128::from(y - y0) * (h - 1),
            d: i128::from(y1 - y0),
            w: h,
        };
        let s = |c: u64, r: u64| f64::from(samples.at(c, r));
        Ok(match at.filter {
            Filter::Nearest => s(ax.nearest(), ay.nearest()),
            Filter::Linear => {
                let (c, fx) = ax.linear();
                let (r, fy) = ay.linear();
                bilinear(s(c, r), s(c + 1, r), s(c, r + 1), s(c + 1, r + 1), fx, fy)
            }
            Filter::Cubic => {
                // Rows west to east, then the four rows south to north; an index past the edge
                // reads the edge sample (`fields.md` 4.3, step 4).
                let (c, fx) = ax.linear();
                let (r, fy) = ay.linear();
                let (wx, wy) = (keys_weights(fx), keys_weights(fy));
                let col = |k: i64| (c as i64 + k).clamp(0, samples.columns as i64 - 1) as u64;
                let row = |k: i64| (r as i64 + k).clamp(0, samples.rows as i64 - 1) as u64;
                let mut total = 0.0;
                for (jy, wyj) in wy.iter().enumerate() {
                    let mut line = 0.0;
                    for (ix, wxi) in wx.iter().enumerate() {
                        line += wxi * s(col(ix as i64 - 1), row(jy as i64 - 1));
                    }
                    total += wyj * line;
                }
                total
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npy::{f32_file, field_file, read_samples};
    use crate::raster::narrow;

    fn samples(rows: u64, columns: u64, f: impl Fn(u64, u64) -> f32) -> Samples {
        let mut values = Vec::new();
        for r in 0..rows {
            for c in 0..columns {
                values.push(f(c, r));
            }
        }
        Samples {
            rows,
            columns,
            values,
        }
    }

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    #[test]
    fn a_nine_by_nine_file_reads_bit_for_bit() {
        // fields.md "array file base" and "corner alignment", both filters.
        let g = g8();
        let s = samples(9, 9, |c, r| (c as f32) * 0.1 + (r as f32) * 7.3);
        for filter in [Filter::Nearest, Filter::Linear] {
            let v = resample(
                &g,
                Window::whole(&g),
                &s,
                Placement {
                    extent: g.extent,
                    outside: 0.0,
                    filter,
                },
            )
            .unwrap();
            for (i, j) in Window::whole(&g).posts() {
                assert_eq!(
                    v.get(i, j).unwrap().to_bits(),
                    f64::from(s.at(i, j)).to_bits()
                );
            }
        }
    }

    #[test]
    fn written_field_reads_back() {
        let g = Grid::new([0, 0, 10_000, 6_000], 500).unwrap();
        let w = Window::whole(&g);
        let f = Raster::from_fn(w, |i, j| Ok::<_, ()>(via_num::sin(i as f64) * j as f64)).unwrap();
        let f = narrow(&g, &f).unwrap();
        let s = read_samples(&field_file(&f), 1 << 26).unwrap();
        let back = resample(
            &g,
            w,
            &s,
            Placement {
                extent: g.extent,
                outside: 0.0,
                filter: Filter::Linear,
            },
        )
        .unwrap();
        let back = narrow(&g, &back).unwrap();
        assert_eq!(back, f);
    }

    #[test]
    fn half_resolution_lies_on_every_other_post() {
        let g = g8();
        let s = samples(5, 5, |c, r| (c * 10 + r) as f32 + 0.25);
        let v = resample(
            &g,
            Window::whole(&g),
            &s,
            Placement {
                extent: g.extent,
                outside: 0.0,
                filter: Filter::Linear,
            },
        )
        .unwrap();
        for c in 0..5 {
            for r in 0..5 {
                assert_eq!(v.get(2 * c, 2 * r), Some(f64::from(s.at(c, r))));
            }
        }
        // Between two samples: their mean.
        assert_eq!(v.get(1, 0), Some(5.25));
    }

    #[test]
    fn orientation_first_row_is_south() {
        let g = g8();
        let s = read_samples(&f32_file(&[2, 2], &[1.0, 0.0, 0.0, 0.0]), 100).unwrap();
        let v = resample(
            &g,
            Window::whole(&g),
            &s,
            Placement {
                extent: g.extent,
                outside: 0.0,
                filter: Filter::Nearest,
            },
        )
        .unwrap();
        assert_eq!(v.get(0, 0), Some(1.0));
        assert_eq!(v.get(8, 8), Some(0.0));
    }

    #[test]
    fn nearest_tie_goes_east() {
        // fields.md "nearest tie": extent [0, 0, 1, 1] at 0.5; post (1, 0) takes column 1.
        let g = Grid::new([0, 0, 1_000, 1_000], 500).unwrap();
        let s = samples(2, 2, |c, _| c as f32);
        let v = resample(
            &g,
            Window::whole(&g),
            &s,
            Placement {
                extent: g.extent,
                outside: 0.0,
                filter: Filter::Nearest,
            },
        )
        .unwrap();
        assert_eq!(v.get(1, 0), Some(1.0));
    }

    #[test]
    fn beyond_the_extent_reads_outside() {
        let g = g8();
        let s = samples(3, 3, |_, _| 5.0);
        let at = Placement {
            extent: [0, 0, 4_000, 8_000],
            outside: -1.0,
            filter: Filter::Linear,
        };
        let v = resample(&g, Window::whole(&g), &s, at).unwrap();
        assert_eq!(v.get(4, 0), Some(5.0));
        assert_eq!(v.get(5, 0), Some(-1.0));
        assert!(offworld(&g, [100_000, 100_000, 110_000, 110_000]));
        assert!(!offworld(&g, [8_000, 8_000, 9_000, 9_000]));
    }
}
