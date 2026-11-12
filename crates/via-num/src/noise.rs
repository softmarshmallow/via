//! Gradient noise (`formulas.md` 8.2).
//!
//! Perlin's improved noise (Perlin, "Improving Noise", SIGGRAPH 2002) in two dimensions: his
//! quintic fade and his sixteen-entry gradient list at `z = 0`. His permutation table is replaced
//! by our lattice hash, [`crate::draw`] keyed by the corner and the octave (ours). Only `+ - *`,
//! `floor` and comparisons are used, each correctly rounded, so the value is the same bits on
//! every system. `fbm` and `ridged`, which sum octaves of this, are the formula evaluator's.

use crate::error::NumError;
use crate::math::{clamp, mix};
use crate::seed::draw;

/// Perlin's gradient list at `z = 0`, indexed by the top four bits of a corner's draw.
const GRADIENTS: [(f64, f64); 16] = [
    (1.0, 1.0),
    (-1.0, 1.0),
    (1.0, -1.0),
    (-1.0, -1.0),
    (1.0, 0.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (1.0, 1.0),
    (1.0, -1.0),
    (-1.0, 1.0),
    (-1.0, -1.0),
];

/// Coordinates at or beyond `2^52` have no fraction left to interpolate, and their corners would
/// near the edge of the 64-bit keys; `formulas.md` 8.2 makes them `formula.range`.
const LIMIT: f64 = 4_503_599_627_370_496.0;

/// Perlin's quintic fade, `t * t * t * (t * (t * 6 - 15) + 10)`, in that order.
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// The gradient index of a corner's draw: its top four bits.
pub fn gradient_index(word: u64) -> usize {
    (word >> 60) as usize
}

/// The gradient noise `n_i` of octave `octave` of `stream` at `(px, py)`, in `[-1, 1]`.
///
/// `px` and `py` are the octave's coordinates, `x / w_i` and `y / w_i`, divided by the caller.
/// A point that is not finite or has a magnitude of `2^52` or more is `formula.range`.
pub fn gradient_noise(stream: u64, octave: i64, px: f64, py: f64) -> Result<f64, NumError> {
    // Written so a NaN fails the test too.
    if !(px.abs() < LIMIT && py.abs() < LIMIT) {
        return Err(NumError::NoiseRange);
    }
    let fx0 = px.floor();
    let fy0 = py.floor();
    // Exact: |floor| < 2^52.
    let ix = fx0 as i64;
    let iy = fy0 as i64;
    let fx = px - fx0;
    let fy = py - fy0;
    let corner = |a: i64, b: i64| -> Result<f64, NumError> {
        let kx = ix.checked_add(a).ok_or(NumError::NoiseRange)?;
        let ky = iy.checked_add(b).ok_or(NumError::NoiseRange)?;
        let (gx, gy) = GRADIENTS[gradient_index(draw(stream, &[kx, ky, octave]))];
        Ok(gx * (fx - a as f64) + gy * (fy - b as f64))
    };
    let u = fade(fx);
    let v = fade(fy);
    let c00 = corner(0, 0)?;
    let c10 = corner(1, 0)?;
    let c01 = corner(0, 1)?;
    let c11 = corner(1, 1)?;
    let n = mix(mix(c00, c10, u), mix(c01, c11, u), v);
    Ok(clamp(n, -1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_lattice_corners_and_bounded() {
        for s in [0u64, 1, 0xDEAD_BEEF] {
            assert_eq!(gradient_noise(s, 0, 3.0, -2.0).map(f64::to_bits), Ok(0));
            let mut x = -4.0;
            while x < 4.0 {
                let n = gradient_noise(s, 2, x, x * 0.37).unwrap();
                assert!((-1.0..=1.0).contains(&n));
                x += 0.0625;
            }
        }
    }

    #[test]
    fn beyond_the_lattice_is_an_error() {
        assert_eq!(gradient_noise(0, 0, LIMIT, 0.0), Err(NumError::NoiseRange));
        assert_eq!(
            gradient_noise(0, 0, 0.0, f64::NAN),
            Err(NumError::NoiseRange)
        );
        assert!(gradient_noise(0, 0, LIMIT - 1.0, 0.5).is_ok());
    }
}
