//! Doubles written as text (`numbers.md` 3.4).
//!
//! A double is written exactly as ECMAScript's `Number::toString` with radix 10 (ECMA-262): the
//! shortest decimal that reads back to the same bits, no exponent for magnitudes in
//! `[1e-6, 1e21)`, otherwise `d.ddde+N` or `d.ddde-N`. Rust's own formatting differs (`3.0` is
//! `3`, `1e21` is `1e+21`), so only its digits are used: `{:e}` without a precision gives the
//! shortest round-trip digits and their exponent, and the layout here is ECMA-262's.

use crate::error::NumError;

/// Writes a finite double as ECMAScript does. A non-finite value is never written
/// (`numbers.md` 8, rule 7): `num.not_finite_result`. Both zeros are `0`.
pub fn write_double(x: f64) -> Result<String, NumError> {
    let x = crate::math::finite(x)?;
    if x == 0.0 {
        return Ok("0".to_string());
    }
    let sci = format!("{:e}", x.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let e: i64 = exponent.parse().unwrap_or(0);
    // ECMA-262's n: the value is 0.d1d2...dk times 10^n.
    let k = digits.len() as i64;
    let n = e + 1;
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
    } else {
        let exp = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exp < 0 { '-' } else { '+' });
        out.push_str(&exp.abs().to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::write_double;

    #[test]
    fn ecmascript_layout() {
        let w = |x: f64| write_double(x).unwrap();
        assert_eq!(w(123.456), "123.456");
        assert_eq!(w(-0.0), "0");
        assert_eq!(w(1e20), "100000000000000000000");
        assert_eq!(w(1.5e21), "1.5e+21");
        assert_eq!(w(0.000001), "0.000001");
        assert_eq!(w(-1.5e-7), "-1.5e-7");
        assert_eq!(w(f64::MAX), "1.7976931348623157e+308");
        assert!(write_double(f64::NAN).is_err());
    }
}
