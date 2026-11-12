//! The maths every other crate is denied (`numbers.md` 8).
//!
//! `+ - * / sqrt` are correctly rounded on every target and are used directly everywhere. All
//! else is here: transcendentals from the `libm` crate with soft floats (never the platform's
//! library, which differs between systems), `min` and `max` with a defined signed zero, and the
//! exact angle reductions of `numbers.md` 2 and `formulas.md` 8.1. No function here calls
//! `mul_add`, and Rust never contracts `a * b + c` on its own, so every expression is evaluated in
//! the order written.
//!
//! These functions return the raw double, NaN or infinite included: a formula decides per
//! operation whether a non-finite value is live (`formulas.md` 9), so the check is the caller's,
//! through [`finite`] and [`store`].

use crate::error::NumError;

/// `PI` as `numbers.md` 10 fixes its bits.
pub const PI: f64 = f64::from_bits(0x4009_21FB_5444_2D18);

/// `PI / 180000.0`, one correctly rounded division: radians per millidegree.
pub const RAD_PER_MILLIDEGREE: f64 = f64::from_bits(0x3EF2_4D15_2119_3474);

/// `K = 180000.0 / PI`, one correctly rounded division: millidegrees per radian.
pub const MILLIDEGREES_PER_RAD: f64 = f64::from_bits(0x40EB_F9F8_F1C5_6B6C);

/// `PI / 180.0`, one correctly rounded division: radians per degree (`formulas.md` 8.1).
pub const RAD_PER_DEGREE: f64 = PI / 180.0;

/// `180.0 / PI`, one correctly rounded division: degrees per radian (`formulas.md` 8.3).
pub const DEGREES_PER_RAD: f64 = 180.0 / PI;

/// `a` to the power `b` (musl's `pow`, through `libm`).
pub fn pow(a: f64, b: f64) -> f64 {
    libm::pow(a, b)
}

/// `e` to the power `a`.
pub fn exp(a: f64) -> f64 {
    libm::exp(a)
}

/// `2` to the power `a`.
pub fn exp2(a: f64) -> f64 {
    libm::exp2(a)
}

/// `exp(a) - 1`, accurate near 0.
pub fn exp_m1(a: f64) -> f64 {
    libm::expm1(a)
}

/// Natural logarithm.
pub fn ln(a: f64) -> f64 {
    libm::log(a)
}

/// `ln(1 + a)`, accurate near 0.
pub fn ln_1p(a: f64) -> f64 {
    libm::log1p(a)
}

/// Base-2 logarithm.
pub fn log2(a: f64) -> f64 {
    libm::log2(a)
}

/// Base-10 logarithm.
pub fn log10(a: f64) -> f64 {
    libm::log10(a)
}

/// Sine of an angle in radians. Formulas take degrees through [`sin_deg`].
pub fn sin(a: f64) -> f64 {
    libm::sin(a)
}

/// Cosine of an angle in radians.
pub fn cos(a: f64) -> f64 {
    libm::cos(a)
}

/// Tangent of an angle in radians.
pub fn tan(a: f64) -> f64 {
    libm::tan(a)
}

/// Arc sine, in radians.
pub fn asin(a: f64) -> f64 {
    libm::asin(a)
}

/// Arc cosine, in radians.
pub fn acos(a: f64) -> f64 {
    libm::acos(a)
}

/// Arc tangent, in radians.
pub fn atan(a: f64) -> f64 {
    libm::atan(a)
}

/// The angle of `(x, y)` from the `x` axis toward `y`, in radians, as C's `atan2(y, x)`.
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// Hyperbolic sine.
pub fn sinh(a: f64) -> f64 {
    libm::sinh(a)
}

/// Hyperbolic cosine.
pub fn cosh(a: f64) -> f64 {
    libm::cosh(a)
}

/// Hyperbolic tangent.
pub fn tanh(a: f64) -> f64 {
    libm::tanh(a)
}

/// `sqrt(a * a + b * b)` without intermediate overflow.
pub fn hypot(a: f64, b: f64) -> f64 {
    libm::hypot(a, b)
}

/// Cube root.
pub fn cbrt(a: f64) -> f64 {
    libm::cbrt(a)
}

/// The remainder of `a / b` with the sign of `a`. Exact (the result is representable), so it is
/// the same everywhere; it comes from `libm` all the same so there is one definition.
pub fn fmod(a: f64, b: f64) -> f64 {
    libm::fmod(a, b)
}

/// `min` by `numbers.md` 8, rule 4: `a` if `a < b`, `b` if `b < a`; when equal, `-0` if either is
/// `-0`, else `a`. `std`'s `f64::min` leaves the sign of zero unspecified.
///
/// No NaN should reach it (rule 7); if one does, the NaN is returned (the first if both are), so
/// the caller's [`finite`] check sees it rather than having it silently dropped as IEEE `minNum`
/// would.
pub fn min(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else if b < a {
        b
    } else if a == b {
        if a.is_sign_negative() {
            a
        } else {
            b
        }
    } else if a.is_nan() {
        a
    } else {
        b
    }
}

/// `max` by `numbers.md` 8, rule 4, mirroring [`min`] with `+0`; NaN as in [`min`].
pub fn max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else if b > a {
        b
    } else if a == b {
        if a.is_sign_negative() {
            b
        } else {
            a
        }
    } else if a.is_nan() {
        a
    } else {
        b
    }
}

/// `clamp(a, lo, hi) = min(max(a, lo), hi)` (`formulas.md` 8.1). `lo > hi` is the caller's
/// `formula.range`.
pub fn clamp(a: f64, lo: f64, hi: f64) -> f64 {
    min(max(a, lo), hi)
}

/// `mix(a, b, s) = (1 - s) * a + s * b`, in that order (`formulas.md` 8.1, GLSL `mix`).
pub fn mix(a: f64, b: f64, s: f64) -> f64 {
    (1.0 - s) * a + s * b
}

/// Turns `-0` into `+0`; every other value is unchanged.
pub fn positive_zero(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else {
        x
    }
}

/// The non-finite guard (`numbers.md` 8, rule 7): the value if finite, else
/// `num.not_finite_result` carrying its bits. A formula maps the error to `formula.not_finite`.
pub fn finite(x: f64) -> Result<f64, NumError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(NumError::NotFiniteResult { bits: x.to_bits() })
    }
}

/// A value on its way into a stored field, attribute, position or file: finite (rule 7) and with
/// no negative zero (rule 8).
pub fn store(x: f64) -> Result<f64, NumError> {
    finite(x).map(positive_zero)
}

/// Rounds a double to `f32` on store, nearest, ties to even, once (`numbers.md` 8, rule 9), after
/// the guard of [`store`].
pub fn store_f32(x: f64) -> Result<f32, NumError> {
    let v = store(x)? as f32;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(NumError::NotFiniteResult { bits: x.to_bits() })
    }
}

/// Sine and cosine of a facing in millidegrees, by quadrant, so the four axis directions are
/// exact (`numbers.md` 2). The facing is normalised first. Zero results are `+0`.
pub fn facing_sin_cos(millidegrees: i64) -> (f64, f64) {
    let m = crate::decimal::normalise_facing(millidegrees);
    let q = m / 90_000;
    let r = m % 90_000;
    let a = (r as f64) * RAD_PER_MILLIDEGREE;
    let s = sin(a);
    let c = cos(a);
    let (x, y) = match q {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    (positive_zero(x), positive_zero(y))
}

/// The bearing of a direction of integer millimetres, in millidegrees clockwise from north,
/// normalised (`numbers.md` 2): `round(atan2(dx, dy) * K)`, ties to even. `None` for the zero
/// direction, which has no bearing; callers report it (`geom.no_length`, `geom.face_side`) and
/// never default to north.
pub fn bearing(dx: i64, dy: i64) -> Option<i64> {
    if dx == 0 && dy == 0 {
        return None;
    }
    let m = (atan2(dx as f64, dy as f64) * MILLIDEGREES_PER_RAD).round_ties_even();
    // |atan2| <= PI, so |m| <= 180000: the conversion is exact.
    Some(crate::decimal::normalise_facing(m as i64))
}

/// The quadrant and the reduced angle of `a` degrees (`formulas.md` 8.1): `r = fmod(a, 360)`, plus
/// 360 when negative, `q` the quarter turn, `s = r - 90 * q` (exact, Sterbenz).
fn degree_quadrant(a: f64) -> (u8, f64) {
    let mut r = fmod(a, 360.0);
    if r < 0.0 {
        r += 360.0;
    }
    // A tiny negative `r` plus 360 rounds to 360 itself, which no quadrant holds; it is a whole
    // turn, so it is read as 0. (formulas.md 8.1 leaves this case unwritten.)
    if r >= 360.0 {
        r = 0.0;
    }
    let q: u8 = if r < 90.0 {
        0
    } else if r < 180.0 {
        1
    } else if r < 270.0 {
        2
    } else {
        3
    };
    (q, r - 90.0 * f64::from(q))
}

/// Sine of `a` degrees by the reduction of `formulas.md` 8.1: `sin(180)` and `sin(360)` are
/// exactly 0, `sin(90)` is 1. A zero result is `+0`.
pub fn sin_deg(a: f64) -> f64 {
    let (q, s) = degree_quadrant(a);
    let h = s * RAD_PER_DEGREE;
    positive_zero(match q {
        0 => sin(h),
        1 => cos(h),
        2 => -sin(h),
        _ => -cos(h),
    })
}

/// Cosine of `a` degrees by the reduction of `formulas.md` 8.1: `cos(90)` is exactly 0 and
/// `cos(180)` is -1. A zero result is `+0`.
pub fn cos_deg(a: f64) -> f64 {
    let (q, s) = degree_quadrant(a);
    let h = s * RAD_PER_DEGREE;
    positive_zero(match q {
        0 => cos(h),
        1 => -sin(h),
        2 => -cos(h),
        _ => sin(h),
    })
}

/// Tangent of `a` degrees, `sin_deg(a) / cos_deg(a)`: `tan(90)` is non-finite, never a large
/// finite number.
pub fn tan_deg(a: f64) -> f64 {
    sin_deg(a) / cos_deg(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_the_single_divisions_they_name() {
        assert_eq!(PI.to_bits(), std::f64::consts::PI.to_bits());
        assert_eq!((PI / 180_000.0).to_bits(), RAD_PER_MILLIDEGREE.to_bits());
        assert_eq!((180_000.0 / PI).to_bits(), MILLIDEGREES_PER_RAD.to_bits());
    }

    #[test]
    fn min_max_signed_zero_and_nan() {
        assert_eq!(min(-0.0, 0.0).to_bits(), 0x8000_0000_0000_0000);
        assert_eq!(min(0.0, -0.0).to_bits(), 0x8000_0000_0000_0000);
        assert_eq!(max(-0.0, 0.0).to_bits(), 0);
        assert_eq!(max(0.0, -0.0).to_bits(), 0);
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert_eq!(clamp(5.0, -1.0, 1.0), 1.0);
    }

    #[test]
    fn degree_trig_is_exact_at_the_axes() {
        assert_eq!(sin_deg(180.0).to_bits(), 0);
        assert_eq!(sin_deg(360.0).to_bits(), 0);
        assert_eq!(sin_deg(-180.0).to_bits(), 0);
        assert_eq!(cos_deg(90.0).to_bits(), 0);
        assert_eq!(sin_deg(90.0), 1.0);
        assert_eq!(cos_deg(180.0), -1.0);
        assert_eq!(sin_deg(-90.0), -1.0);
        assert!(!tan_deg(90.0).is_finite());
        assert_eq!(sin_deg(-1e-20).to_bits(), 0);
    }

    #[test]
    fn bearing_of_nothing_is_none() {
        assert_eq!(bearing(0, 0), None);
        assert_eq!(bearing(-1, 0), Some(270_000));
    }

    #[test]
    fn guard() {
        assert_eq!(store(-0.0).map(f64::to_bits), Ok(0));
        assert_eq!(
            finite(f64::INFINITY).map_err(|e| e.code()),
            Err("num.not_finite_result")
        );
        assert_eq!(
            store_f32(1e300).map_err(|e| e.code()),
            Err("num.not_finite_result")
        );
    }
}
