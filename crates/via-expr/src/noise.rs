//! `fbm` (`formulas.md` 8.2) over `via-num`'s gradient noise.

use via_num::{gradient_noise, pow, NumError};

/// `fbm` at world coordinates `(x, y)` in metres: octaves `0 .. octaves`, each the gradient noise
/// of `stream` at `(x / w_i, y / w_i)` with `w_i = wavelength / pow(lacunarity, i)` and weight
/// `g_i = pow(gain, i)`; the weighted sum over the sum of weights, each summed from `i = 0`
/// upward. A point beyond the hashable lattice is `NumError::NoiseRange`.
/// `ridged` (`formulas.md` 8.2): the octaves of `fbm`, each folded at its zero and squared,
/// `r_i = (1 - abs(n_i)) * (1 - abs(n_i))`, in 0 to 1.
pub fn ridged(
    stream: u64,
    x: f64,
    y: f64,
    wavelength: f64,
    octaves: u32,
    gain: f64,
    lacunarity: f64,
) -> Result<f64, NumError> {
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..octaves {
        let w = wavelength / pow(lacunarity, f64::from(i));
        let g = pow(gain, f64::from(i));
        let n = gradient_noise(stream, i64::from(i), x / w, y / w)?;
        let f = 1.0 - n.abs();
        let term = g * (f * f);
        if i == 0 {
            num = term;
            den = g;
        } else {
            num += term;
            den += g;
        }
    }
    Ok(num / den)
}

pub fn fbm(
    stream: u64,
    x: f64,
    y: f64,
    wavelength: f64,
    octaves: u32,
    gain: f64,
    lacunarity: f64,
) -> Result<f64, NumError> {
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..octaves {
        let w = wavelength / pow(lacunarity, f64::from(i));
        let g = pow(gain, f64::from(i));
        let n = gradient_noise(stream, i64::from(i), x / w, y / w)?;
        let term = g * n;
        // The sums start at their first term, not at 0: `0 + -0` would be `+0`.
        if i == 0 {
            num = term;
            den = g;
        } else {
            num += term;
            den += g;
        }
    }
    Ok(num / den)
}
