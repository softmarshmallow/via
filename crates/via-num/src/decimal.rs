//! Decimal text to exact numbers (`numbers.md` 3).
//!
//! A length, a facing, a `jitter` and a `weight` are read from the literal's digits, never through
//! a binary double: `1.005` read through a double and scaled by 1000 is `1004.9999999999999`, and
//! `2.0000000000000001` would pass as `2.0`. Other numbers are the nearest double to the exact
//! decimal value.

use crate::error::{NumError, Quantity};

/// The largest magnitude on the lattice, in millimetres: `2^31 - 1` (`numbers.md` 3.1, step 5).
/// The bound leaves room for exact predicates in 128-bit integers.
pub const LATTICE_MAX: i64 = (1 << 31) - 1;

/// Millidegrees in a full turn; facings are held in `[0, FULL_TURN)`.
pub const FULL_TURN: i64 = 360_000;

/// Exponents are held saturated at this magnitude. Any exponent this large already decides the
/// outcome (zero, `too_precise` or `out_of_range`), so saturating changes no result and keeps
/// every later sum inside 64 bits.
const EXP_CAP: i64 = 1 << 40;

/// A literal's exact value, `(-1)^negative * N * 10^exp`, with `N`'s digits held as text so no
/// power of ten is ever formed before the range is known.
#[derive(Debug)]
struct Decimal {
    negative: bool,
    /// `N` without leading or trailing zeros; empty when the value is zero.
    digits: Vec<u8>,
    exp: i64,
}

/// Reads the normalised number text `documents.md` hands over: an optional sign, digits, an
/// optional fraction and an optional exponent. `inf` and `nan` in any sign are `num.not_finite`.
fn parse_decimal(text: &str) -> Result<Decimal, NumError> {
    let malformed = || NumError::Malformed {
        text: text.to_string(),
    };
    let bytes = text.as_bytes();
    let (negative, body) = match bytes.first() {
        Some(b'-') => (true, &bytes[1..]),
        Some(b'+') => (false, &bytes[1..]),
        _ => (false, bytes),
    };
    if body == b"inf" || body == b"nan" {
        return Err(NumError::NotFinite {
            text: text.to_string(),
        });
    }
    let mut pos = 0;
    let int_start = pos;
    while pos < body.len() && body[pos].is_ascii_digit() {
        pos += 1;
    }
    let int_digits = &body[int_start..pos];
    if int_digits.is_empty() {
        return Err(malformed());
    }
    let mut frac_digits: &[u8] = &[];
    if pos < body.len() && body[pos] == b'.' {
        pos += 1;
        let start = pos;
        while pos < body.len() && body[pos].is_ascii_digit() {
            pos += 1;
        }
        frac_digits = &body[start..pos];
        if frac_digits.is_empty() {
            return Err(malformed());
        }
    }
    let mut exp_lit: i64 = 0;
    if pos < body.len() && (body[pos] == b'e' || body[pos] == b'E') {
        pos += 1;
        let mut exp_negative = false;
        if pos < body.len() && (body[pos] == b'+' || body[pos] == b'-') {
            exp_negative = body[pos] == b'-';
            pos += 1;
        }
        let start = pos;
        while pos < body.len() && body[pos].is_ascii_digit() {
            exp_lit = (exp_lit * 10 + i64::from(body[pos] - b'0')).min(EXP_CAP);
            pos += 1;
        }
        if pos == start {
            return Err(malformed());
        }
        if exp_negative {
            exp_lit = -exp_lit;
        }
    }
    if pos != body.len() {
        return Err(malformed());
    }

    let mut digits: Vec<u8> = int_digits.iter().chain(frac_digits).copied().collect();
    let first_nonzero = digits.iter().position(|&d| d != b'0');
    let Some(first) = first_nonzero else {
        // N = 0: the value is 0 whatever the exponent (step 2), and -0 is 0 (step 6).
        return Ok(Decimal {
            negative: false,
            digits: Vec::new(),
            exp: 0,
        });
    };
    digits.drain(..first);
    // Step 3: trailing zeros of N move into the exponent.
    let mut exp = exp_lit - frac_digits.len() as i64;
    while digits.last() == Some(&b'0') {
        digits.pop();
        exp += 1;
    }
    Ok(Decimal {
        negative,
        digits,
        exp: exp.clamp(-EXP_CAP, EXP_CAP),
    })
}

/// Writes a signed decimal magnitude given as digits with `decimals` of them after the point,
/// trailing zeros and a trailing point dropped (`numbers.md` 3.4). Shared by the hint of
/// `num.too_precise` and by [`crate::write_length`].
pub(crate) fn fixed_text(negative: bool, magnitude: &[u8], decimals: usize) -> String {
    let mut digits: Vec<u8> = magnitude.to_vec();
    while digits.len() > 1 && digits[0] == b'0' {
        digits.remove(0);
    }
    if digits.iter().all(|&d| d == b'0') {
        return "0".to_string();
    }
    if digits.len() <= decimals {
        let mut padded = vec![b'0'; decimals + 1 - digits.len()];
        padded.extend_from_slice(&digits);
        digits = padded;
    }
    let split = digits.len() - decimals;
    let (int_part, frac_part) = digits.split_at(split);
    let mut frac_end = frac_part.len();
    while frac_end > 0 && frac_part[frac_end - 1] == b'0' {
        frac_end -= 1;
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(int_part).unwrap_or("0"));
    if frac_end > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&frac_part[..frac_end]).unwrap_or("0"));
    }
    out
}

/// Adds one to a decimal digit string.
fn increment(digits: &[u8]) -> Vec<u8> {
    let mut out = digits.to_vec();
    for d in out.iter_mut().rev() {
        if *d == b'9' {
            *d = b'0';
        } else {
            *d += 1;
            return out;
        }
    }
    out.insert(0, b'1');
    out
}

/// Reads the exact value as an integer count of `10^-decimals` steps (`numbers.md` 3.1, steps 3
/// and 4). `digit_limit` is the step-4 bound: 10 for lengths, 18 for facings and millionths.
fn to_steps(text: &str, quantity: Quantity, digit_limit: i64) -> Result<i64, NumError> {
    let d = parse_decimal(text)?;
    if d.digits.is_empty() {
        return Ok(0);
    }
    let decimals = i64::from(quantity.decimals());
    let e = d.exp + decimals;
    if e < 0 {
        // Not a whole step. The two nearest steps are the truncation and the one beyond it; the
        // dropped digits are never all zero, since N has no trailing zeros.
        let drop = (-e) as usize;
        let kept: Vec<u8> = if d.digits.len() > drop {
            d.digits[..d.digits.len() - drop].to_vec()
        } else {
            vec![b'0']
        };
        let next = increment(&kept);
        let places = decimals as usize;
        let nearest = if d.negative {
            [
                fixed_text(true, &next, places),
                fixed_text(true, &kept, places),
            ]
        } else {
            [
                fixed_text(false, &kept, places),
                fixed_text(false, &next, places),
            ]
        };
        return Err(NumError::TooPrecise {
            text: text.to_string(),
            quantity,
            nearest,
        });
    }
    if d.digits.len() as i64 + e > digit_limit {
        return Err(NumError::OutOfRange {
            text: text.to_string(),
        });
    }
    // At most `digit_limit` (<= 18) digits: fits in i64 without overflow.
    let mut value: i64 = 0;
    for &digit in &d.digits {
        value = value * 10 + i64::from(digit - b'0');
    }
    for _ in 0..e {
        value *= 10;
    }
    Ok(if d.negative { -value } else { value })
}

/// Reads a length's text to integer millimetres on the lattice (`numbers.md` 3.1).
///
/// Errors: `num.not_finite`, `num.too_precise` (never rounded), `num.out_of_range` (beyond
/// `±(2^31 - 1)` mm, found before any power of ten is formed).
pub fn parse_length(text: &str) -> Result<i64, NumError> {
    let mm = to_steps(text, Quantity::Length, 10)?;
    if !(-LATTICE_MAX..=LATTICE_MAX).contains(&mm) {
        return Err(NumError::OutOfRange {
            text: text.to_string(),
        });
    }
    Ok(mm)
}

/// Reads a `face` number to millidegrees normalised to `[0, 360000)` (`numbers.md` 2 and 3.2).
/// `-90` and `270` give the same facing.
pub fn parse_facing(text: &str) -> Result<i64, NumError> {
    Ok(normalise_facing(to_steps(text, Quantity::Facing, 18)?))
}

/// Normalises millidegrees to `[0, 360000)` by Euclidean remainder (`numbers.md` 2).
pub fn normalise_facing(millidegrees: i64) -> i64 {
    millidegrees.rem_euclid(FULL_TURN)
}

/// Reads a scatter's `jitter` or an asset choice's `weight` to integer millionths
/// (`numbers.md` 3.3): `0.8` is `800000`; more than six decimals is `num.too_precise`.
pub fn parse_millionths(text: &str) -> Result<i64, NumError> {
    to_steps(text, Quantity::Millionths, 18)
}

/// Reads any other number to the nearest double, ties to even (`numbers.md` 3.3).
///
/// The text is first checked against the same grammar as a length, so `inf`, `nan` and stray
/// forms are refused before the conversion. A value whose nearest double is infinite is
/// `num.out_of_range`; one that rounds to zero is `+0`.
pub fn parse_double(text: &str) -> Result<f64, NumError> {
    parse_decimal(text)?;
    // std's decimal-to-float conversion is correctly rounded for inputs of any length
    // (Eisel-Lemire with an exact big-decimal fallback), which is the rule of 3.3.
    let value: f64 = text.parse().map_err(|_| NumError::Malformed {
        text: text.to_string(),
    })?;
    if !value.is_finite() {
        return Err(NumError::OutOfRange {
            text: text.to_string(),
        });
    }
    Ok(if value == 0.0 { 0.0 } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponents_too_large_to_hold_saturate_without_changing_the_outcome() {
        assert_eq!(parse_length("0e99999999999999999999999"), Ok(0));
        assert_eq!(
            parse_length("1e99999999999999999999999").map_err(|e| e.code()),
            Err("num.out_of_range")
        );
        assert_eq!(
            parse_length("1e-99999999999999999999999").map_err(|e| e.code()),
            Err("num.too_precise")
        );
    }

    #[test]
    fn too_precise_names_the_two_nearest_lengths() {
        match parse_length("2.0005") {
            Err(NumError::TooPrecise { nearest, .. }) => {
                assert_eq!(nearest, ["2".to_string(), "2.001".to_string()]);
            }
            other => panic!("{other:?}"),
        }
        match parse_length("-0.0005") {
            Err(NumError::TooPrecise { nearest, .. }) => {
                assert_eq!(nearest, ["-0.001".to_string(), "0".to_string()]);
            }
            other => panic!("{other:?}"),
        }
        match parse_facing("12.3456") {
            Err(NumError::TooPrecise { nearest, .. }) => {
                assert_eq!(nearest, ["12.345".to_string(), "12.346".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn facings_normalise_and_allow_any_size_within_64_bits() {
        assert_eq!(parse_facing("-90"), Ok(270_000));
        assert_eq!(parse_facing("270"), Ok(270_000));
        assert_eq!(parse_facing("630"), Ok(270_000));
        assert_eq!(parse_facing("1e14"), Ok(280_000));
        assert_eq!(
            parse_facing("1e15").map_err(|e| e.code()),
            Err("num.out_of_range")
        );
    }

    #[test]
    fn millionths_read_exactly() {
        assert_eq!(parse_millionths("0.8"), Ok(800_000));
        assert_eq!(
            parse_millionths("0.0000001").map_err(|e| e.code()),
            Err("num.too_precise")
        );
    }

    #[test]
    fn doubles_are_nearest_and_never_negative_zero() {
        assert_eq!(
            parse_double("0.1").map(f64::to_bits),
            Ok(0x3FB9_9999_9999_999A)
        );
        assert_eq!(parse_double("-0.0").map(f64::to_bits), Ok(0));
        assert_eq!(parse_double("-1e-400").map(f64::to_bits), Ok(0));
        assert_eq!(
            parse_double("1e400").map_err(|e| e.code()),
            Err("num.out_of_range")
        );
        assert_eq!(
            parse_double("-inf").map_err(|e| e.code()),
            Err("num.not_finite")
        );
        assert_eq!(
            parse_double("infinity").map_err(|e| e.code()),
            Err("doc.syntax")
        );
    }

    #[test]
    fn stray_forms_are_refused() {
        for text in ["", ".5", "5.", "1e", "1_000", "0x10", "1.2.3", " 1"] {
            assert_eq!(
                parse_length(text).map_err(|e| e.code()),
                Err("doc.syntax"),
                "{text}"
            );
        }
    }
}
