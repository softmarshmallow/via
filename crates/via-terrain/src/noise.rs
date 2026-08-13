//! Deterministic value noise. Hand-rolled on splitmix64 so that every bit of
//! simulation input is a pure function of (seed, coordinates) — no external
//! noise crate whose internals could drift under us.

use via_artifact::seed::splitmix64;

#[inline]
fn hash2(seed: u64, x: i64, y: i64) -> u64 {
    let mut h = splitmix64(seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    h = splitmix64(h ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    h
}

/// Uniform lattice value in [-1, 1).
#[inline]
fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    const SCALE: f64 = 2.0 / 9_007_199_254_740_992.0; // 2 / 2^53
    ((hash2(seed, x, y) >> 11) as f64) * SCALE - 1.0
}

#[inline]
fn quintic(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Smooth value noise in roughly [-1, 1].
pub fn value(seed: u64, x: f64, y: f64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let (xi, yi) = (x0 as i64, y0 as i64);
    let (tx, ty) = (quintic(x - x0), quintic(y - y0));
    let v00 = lattice(seed, xi, yi);
    let v10 = lattice(seed, xi + 1, yi);
    let v01 = lattice(seed, xi, yi + 1);
    let v11 = lattice(seed, xi + 1, yi + 1);
    let a = v00 + (v10 - v00) * tx;
    let b = v01 + (v11 - v01) * tx;
    a + (b - a) * ty
}

/// Fractional Brownian motion over `value` noise, normalized to ~[-1, 1].
pub fn fbm(seed: u64, x: f64, y: f64, octaves: u32) -> f64 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for k in 0..octaves {
        let oct_seed = splitmix64(seed ^ (k as u64).wrapping_mul(0xA24B_AED4_963E_E407));
        sum += amp * value(oct_seed, x * freq, y * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Per-cell white jitter in [-1, 1), for symmetry breaking.
pub fn cell_jitter(seed: u64, cell_index: u32) -> f64 {
    const SCALE: f64 = 2.0 / 9_007_199_254_740_992.0;
    ((splitmix64(seed ^ (cell_index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)) >> 11) as f64)
        * SCALE
        - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_noise_is_bounded_and_stable() {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for i in 0..10_000 {
            let v = fbm(7, (i % 100) as f64 * 0.13, (i / 100) as f64 * 0.17, 5);
            min = min.min(v);
            max = max.max(v);
        }
        assert!(
            min >= -1.0 && max <= 1.0,
            "fbm out of range: [{min}, {max}]"
        );
        assert_eq!(value(3, 1.5, 2.5), value(3, 1.5, 2.5));
    }
}
