//! The model's value noise: the forcing texture of its starting surface, uplift and rain
//! (`fields.rs`, `climate.rs`). Every draw is a pure function of (seed, coordinates), hashed by
//! `via-num`'s SplitMix64 mix and mapped by its `unit` (`numbers.md` 7.2), so the library holds no
//! hash of its own. It is not the core's gradient noise (`formulas.md` 8.2): it is the earlier Rust
//! code's, kept bit for bit because every settled landscape is drawn from it.

use via_num as num;

#[inline]
fn hash2(seed: u64, x: i64, y: i64) -> u64 {
    let h = num::mix_word(seed, (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    num::mix_word(h, (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
}

/// A word to `[-1, 1)`: `2 * unit(u) - 1`, exact up to the subtraction.
#[inline]
fn signed_unit(u: u64) -> f64 {
    2.0 * num::unit(u) - 1.0
}

/// Uniform lattice value in [-1, 1).
#[inline]
fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    signed_unit(hash2(seed, x, y))
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
        let oct_seed = num::mix_word(seed, (k as u64).wrapping_mul(0xA24B_AED4_963E_E407));
        sum += amp * value(oct_seed, x * freq, y * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Per-cell white jitter in [-1, 1), for symmetry breaking.
pub fn cell_jitter(seed: u64, cell_index: u32) -> f64 {
    signed_unit(num::mix_word(
        seed,
        (cell_index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93),
    ))
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
            min = num::min(min, v);
            max = num::max(max, v);
        }
        assert!(
            min >= -1.0 && max <= 1.0,
            "fbm out of range: [{min}, {max}]"
        );
        assert_eq!(value(3, 1.5, 2.5), value(3, 1.5, 2.5));
    }
}
