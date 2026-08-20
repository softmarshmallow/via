//! Shared primitives. Ordering helpers exist so every heap and every sort
//! in the spike breaks ties on cell index — determinism is a hard
//! requirement here as everywhere (CONTRIBUTING).

/// Total order on f64 bits: monotone in value for all finite inputs, so a
/// binary heap keyed on it pops in numeric order without float comparison.
#[inline]
pub fn f64_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 {
        !b
    } else {
        b ^ (1u64 << 63)
    }
}

/// In-bounds D8 neighbours of `i` in fixed scan order, with the step's
/// length factor (1 or √2).
#[inline]
pub fn for_neighbors8(w: u32, h: u32, i: u32, mut f: impl FnMut(u32, f64)) {
    const D8: [(i32, i32, f64); 8] = [
        (-1, -1, std::f64::consts::SQRT_2),
        (0, -1, 1.0),
        (1, -1, std::f64::consts::SQRT_2),
        (-1, 0, 1.0),
        (1, 0, 1.0),
        (-1, 1, std::f64::consts::SQRT_2),
        (0, 1, 1.0),
        (1, 1, std::f64::consts::SQRT_2),
    ];
    let (x, y) = ((i % w) as i64, (i / w) as i64);
    for &(dx, dy, fac) in &D8 {
        let (nx, ny) = (x + dx as i64, y + dy as i64);
        if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
            f((ny * w as i64 + nx) as u32, fac);
        }
    }
}

/// Ordinary least squares on (x, y); returns (slope, intercept, r²).
pub fn ols(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    let n = x.len() as f64;
    if x.len() < 2 {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        sxy += (xi - mx) * (yi - my);
        sxx += (xi - mx) * (xi - mx);
        syy += (yi - my) * (yi - my);
    }
    if sxx == 0.0 {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let b = sxy / sxx;
    let r2 = if syy > 0.0 {
        sxy * sxy / (sxx * syy)
    } else {
        f64::NAN
    };
    (b, my - b * mx, r2)
}
