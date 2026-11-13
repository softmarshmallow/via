//! The integer-millimetre lattice and the world's grid of posts (`numbers.md` 3.1, 3.4 and 4).
//!
//! Lengths are integers so two systems agree on every coordinate and every comparison is exact
//! (R24). Arithmetic on them is checked: an overflow is an error naming the operation, never a
//! wrap (`numbers.md` 8, rule 6).

use crate::decimal::{fixed_text, LATTICE_MAX};
use crate::error::{ExtentHint, NumError};

/// The resolution used when a world writes none: 500 mm (`numbers.md` 4). It is held to the same
/// rules as a written one.
pub const DEFAULT_RESOLUTION_MM: i64 = 500;

/// The most posts a grid may hold, `2^26` (`numbers.md` 4). Provisional until the M0 probe.
pub const POST_LIMIT: u64 = 1 << 26;

/// Writes a length as exact metres from its millimetres (`numbers.md` 3.4): the point three digits
/// from the right, trailing zeros and a trailing point dropped, never through a double.
pub fn write_length(mm: i64) -> String {
    fixed_text(mm < 0, mm.unsigned_abs().to_string().as_bytes(), 3)
}

/// Writes a facing as exact degrees from its millidegrees, the same way (`numbers.md` 3.4).
pub fn write_facing(millidegrees: i64) -> String {
    write_length(millidegrees)
}

/// Writes a 64-bit word, a world seed included, as 16 lower-case hexadecimal digits
/// (`numbers.md` 3.4).
pub fn write_seed(seed: u64) -> String {
    format!("{seed:016x}")
}

/// A lattice coordinate as a double in metres: one correctly rounded division, so 12,345 mm is
/// the double nearest 12.345 (`numbers.md` 4).
pub fn metres(mm: i64) -> f64 {
    (mm as f64) / 1000.0
}

/// Whether a value lies on the lattice's range, `±(2^31 - 1)` mm.
pub fn in_lattice(mm: i64) -> bool {
    (-LATTICE_MAX..=LATTICE_MAX).contains(&mm)
}

/// Rounds a computed position in millimetres to the lattice: nearest, ties to even, once, at the
/// end of its expression (`numbers.md` 3.1, "Computed positions"). Outside the lattice, or not
/// finite, is `geom.range`.
pub fn round_to_lattice(mm: f64) -> Result<i64, NumError> {
    let r = mm.round_ties_even();
    // Compare as doubles before converting: `as` saturates silently, which would hide the error.
    if r.is_finite() && r.abs() <= LATTICE_MAX as f64 {
        Ok(r as i64)
    } else {
        Err(NumError::LatticeRange)
    }
}

/// Checked lattice addition (`numbers.md` 8, rule 6). `op` names the operation for the
/// diagnostic; the caller adds subject and position.
pub fn checked_add(a: i64, b: i64, op: &'static str) -> Result<i64, NumError> {
    a.checked_add(b).ok_or(NumError::Overflow { op })
}

/// Checked lattice subtraction.
pub fn checked_sub(a: i64, b: i64, op: &'static str) -> Result<i64, NumError> {
    a.checked_sub(b).ok_or(NumError::Overflow { op })
}

/// Checked lattice multiplication.
pub fn checked_mul(a: i64, b: i64, op: &'static str) -> Result<i64, NumError> {
    a.checked_mul(b).ok_or(NumError::Overflow { op })
}

/// The world's grid of posts (`numbers.md` 4): an extent in millimetres and a post spacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    /// `[x0, y0, x1, y1]` in millimetres.
    pub extent: [i64; 4],
    /// The post spacing `r` in millimetres, at least 1.
    pub resolution: i64,
    /// Posts along `x`, both edges included.
    pub nx: u64,
    /// Posts along `y`, both edges included.
    pub ny: u64,
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The divisors of `g` nearest `r`, below and above (`num.extent_not_multiple`'s hint).
fn nearest_divisors(g: i64, r: i64) -> (Option<i64>, Option<i64>) {
    let mut below: Option<i64> = None;
    let mut above: Option<i64> = None;
    let mut consider = |d: i64| {
        if d < r && below.is_none_or(|b| d > b) {
            below = Some(d);
        }
        if d > r && above.is_none_or(|a| d < a) {
            above = Some(d);
        }
    };
    let mut i: i64 = 1;
    while i <= g / i {
        if g % i == 0 {
            consider(i);
            consider(g / i);
        }
        i += 1;
    }
    (below, above)
}

impl Grid {
    /// Checks an extent and a resolution and counts the posts, in the order the chapter gives:
    /// `num.extent_empty`, `num.resolution`, `num.extent_not_multiple`, `num.too_many_posts`.
    /// The post count is checked before anything is allocated.
    pub fn new(extent: [i64; 4], resolution: i64) -> Result<Grid, NumError> {
        let [x0, y0, x1, y1] = extent;
        if x1 <= x0 || y1 <= y0 {
            return Err(NumError::ExtentEmpty { extent });
        }
        if resolution < 1 {
            return Err(NumError::Resolution { resolution });
        }
        // Lattice values are within ±(2^31 - 1), so widths fit in i64 with room to spare.
        let w = checked_sub(x1, x0, "extent width")?;
        let h = checked_sub(y1, y0, "extent height")?;
        let r = resolution;
        if w % r != 0 || h % r != 0 {
            // Rounded down lies between `lo` and the written corner, so on the lattice; rounded
            // up may not, and is then left out rather than suggested (`num.out_of_range`).
            let round = |lo: i64, width: i64| {
                let down = lo + (width / r) * r;
                let mut out = vec![down];
                if width % r != 0 {
                    if let Some(up) = down.checked_add(r).filter(|&v| in_lattice(v)) {
                        out.push(up);
                    }
                }
                out
            };
            let (resolution_below, resolution_above) = nearest_divisors(gcd(w, h), r);
            return Err(NumError::ExtentNotMultiple {
                extent,
                resolution,
                hint: ExtentHint {
                    x1: round(x0, w),
                    y1: round(y0, h),
                    resolution_below,
                    resolution_above,
                },
            });
        }
        let nx = (w / r) as u64 + 1;
        let ny = (h / r) as u64 + 1;
        if u128::from(nx) * u128::from(ny) > u128::from(POST_LIMIT) {
            return Err(NumError::TooManyPosts {
                nx,
                ny,
                limit: POST_LIMIT,
            });
        }
        Ok(Grid {
            extent,
            resolution,
            nx,
            ny,
        })
    }

    /// The number of posts, `nx * ny`, at most [`POST_LIMIT`].
    pub fn len(&self) -> u64 {
        self.nx * self.ny
    }

    /// A grid always has at least one post in each direction.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Post `(i, j)` in millimetres, exact: `(x0 + i * r, y0 + j * r)`. `None` outside the grid.
    pub fn post(&self, i: u64, j: u64) -> Option<(i64, i64)> {
        if i >= self.nx || j >= self.ny {
            return None;
        }
        let r = self.resolution;
        Some((self.extent[0] + i as i64 * r, self.extent[1] + j as i64 * r))
    }

    /// The index of post `(i, j)` in post order: row by row from the south, `k = j * nx + i`.
    pub fn index(&self, i: u64, j: u64) -> Option<u64> {
        (i < self.nx && j < self.ny).then(|| j * self.nx + i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `numbers.md` 8 rule 6: a checked lattice operation that overflows is `num.overflow`, never a
    /// wrap. No text reaches it (lengths are bounded), so this is its named workspace test.
    #[test]
    fn lattice_overflow_is_num_overflow() {
        let code = |r: Result<i64, NumError>| r.map_err(|e| e.code());
        assert_eq!(code(checked_add(i64::MAX, 1, "add")), Err("num.overflow"));
        assert_eq!(code(checked_sub(i64::MIN, 1, "sub")), Err("num.overflow"));
        assert_eq!(code(checked_mul(i64::MAX, 2, "mul")), Err("num.overflow"));
        assert_eq!(code(checked_add(2, 3, "add")), Ok(5));
    }

    #[test]
    fn worked_example_grid() {
        let g = Grid::new([0, 0, 10_000, 6_000], 2_000).unwrap();
        assert_eq!((g.nx, g.ny), (6, 4));
        assert_eq!(g.post(5, 3), Some((10_000, 6_000)));
        assert_eq!(g.index(5, 3), Some(23));
        assert_eq!(metres(10_000).to_bits(), 10.0f64.to_bits());
    }

    #[test]
    fn default_resolution_on_the_worked_extent() {
        let g = Grid::new([0, 0, 10_000, 6_000], DEFAULT_RESOLUTION_MM).unwrap();
        assert_eq!((g.nx, g.ny), (21, 13));
    }

    #[test]
    fn not_multiple_hint_matches_the_chapter() {
        match Grid::new([0, 0, 10_000, 6_000], 3_000) {
            Err(NumError::ExtentNotMultiple { hint, .. }) => {
                assert_eq!(hint.x1, [9_000, 12_000]);
                assert_eq!(hint.y1, [6_000]);
                assert_eq!(hint.resolution_below, Some(2_000));
                assert_eq!(hint.resolution_above, None);
            }
            other => panic!("{other:?}"),
        }
        let text = Grid::new([0, 0, 10_000, 6_000], 3_000)
            .unwrap_err()
            .to_string();
        assert_eq!(
            text,
            "extent [0, 0, 10, 6] is not a whole multiple of resolution 3; keep x0 and y0 and \
             try x1 9 or 12, y1 6, or resolution 2"
        );
    }

    /// The hint is user text in metres, never a Rust rendering, and never suggests a corner the
    /// lattice cannot hold.
    #[test]
    fn not_multiple_hint_stays_on_the_lattice() {
        // Widths 4,294,966,500 mm: rounded up, x1 would be 2,147,485,000 mm, above 2^31 - 1.
        let (lo, hi) = (-2_147_483_000, 2_147_483_500);
        let err = Grid::new([lo, lo, hi, hi], 2_000).unwrap_err();
        match &err {
            NumError::ExtentNotMultiple { hint, .. } => {
                assert_eq!(hint.x1, [2_147_483_000]);
                assert_eq!(hint.y1, [2_147_483_000]);
                assert!(hint.x1.iter().chain(&hint.y1).all(|&v| in_lattice(v)));
            }
            other => panic!("{other:?}"),
        }
        let text = err.to_string();
        assert!(!text.contains("Some") && !text.contains("None"), "{text}");
        assert!(
            text.contains("[-2147483, -2147483, 2147483.5, 2147483.5]"),
            "{text}"
        );
        assert!(text.contains("x1 2147483, y1 2147483,"), "{text}");
        assert!(!text.contains("2147485"), "{text}");
        // An absent resolution above is left out, not written.
        let text = Grid::new([0, 0, 10_000, 6_000], 3_000)
            .unwrap_err()
            .to_string();
        assert!(text.ends_with("or resolution 2"), "{text}");
    }

    #[test]
    fn grid_errors_in_order() {
        let code = |e: [i64; 4], r| Grid::new(e, r).map(|_| ()).map_err(|e| e.code());
        assert_eq!(code([0, 0, 0, 1], 0), Err("num.extent_empty"));
        assert_eq!(code([0, 0, 1, 1], 0), Err("num.resolution"));
        assert_eq!(
            code([0, 0, 2_000_000_000, 2_000_000_000], 1),
            Err("num.too_many_posts")
        );
    }

    #[test]
    fn lengths_write_as_exact_metres() {
        assert_eq!(write_length(1005), "1.005");
        assert_eq!(write_length(3000), "3");
        assert_eq!(write_length(-250), "-0.25");
        assert_eq!(write_length(0), "0");
        assert_eq!(write_length(i64::MIN), "-9223372036854775.808");
        assert_eq!(write_seed(0), "0000000000000000");
    }

    #[test]
    fn rounding_to_the_lattice() {
        assert_eq!(round_to_lattice(500.5), Ok(500));
        assert_eq!(round_to_lattice(-731.8728027636117), Ok(-732));
        assert_eq!(round_to_lattice(2_147_483_647.4), Ok(2_147_483_647));
        assert_eq!(
            round_to_lattice(2_147_483_647.6),
            Err(NumError::LatticeRange)
        );
        assert_eq!(round_to_lattice(f64::NAN), Err(NumError::LatticeRange));
    }
}
