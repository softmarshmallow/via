//! Fixed-order reductions (`numbers.md` 8, rule 5).
//!
//! Floating-point addition is not associative, so a sum is defined over a fixed sequence and a
//! fixed grouping: chunks of 1024 in sequence order, each reduced left to right from its first
//! element, then the chunk results left to right the same way. A parallel version computes the
//! chunks anywhere and combines them in order, and gives these bits at any thread count. The chunk
//! rule is ours.

use crate::error::NumError;
use crate::math::{max, min};

/// The chunk length of rule 5.
pub const CHUNK: usize = 1024;

/// Reduces a non-empty sequence by the chunk rule; `None` when empty.
fn chunked(values: &[f64], op: impl Fn(f64, f64) -> f64) -> Option<f64> {
    values
        .chunks(CHUNK)
        .filter_map(|chunk| {
            chunk
                .split_first()
                .map(|(&first, rest)| rest.iter().fold(first, |acc, &v| op(acc, v)))
        })
        .reduce(&op)
}

/// The sum by rule 5. An empty sum is `+0`.
pub fn sum(values: &[f64]) -> f64 {
    chunked(values, |a, b| a + b).unwrap_or(0.0)
}

/// The product by rule 5. An empty product is `1`.
pub fn product(values: &[f64]) -> f64 {
    chunked(values, |a, b| a * b).unwrap_or(1.0)
}

/// The mean: the rule-5 sum divided by the count as a double. Empty is `num.empty_reduction`.
pub fn mean(values: &[f64]) -> Result<f64, NumError> {
    if values.is_empty() {
        return Err(NumError::EmptyReduction { op: "mean" });
    }
    Ok(sum(values) / (values.len() as f64))
}

/// The minimum by rule 5 with the `min` of rule 4. Empty is `num.empty_reduction`.
pub fn minimum(values: &[f64]) -> Result<f64, NumError> {
    chunked(values, min).ok_or(NumError::EmptyReduction { op: "minimum" })
}

/// The maximum by rule 5 with the `max` of rule 4. Empty is `num.empty_reduction`.
pub fn maximum(values: &[f64]) -> Result<f64, NumError> {
    chunked(values, max).ok_or(NumError::EmptyReduction { op: "maximum" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_change_the_sum_and_are_part_of_its_definition() {
        // 1e16 + 1 rounds back to 1e16, so left to right the ones vanish. By chunks, the second
        // chunk of ones sums to 1024 first, and 1e16 + 1024 is exact.
        let mut v = vec![1e16];
        v.extend(std::iter::repeat_n(1.0, 2047));
        let left_to_right = v.iter().fold(0.0, |a, &b| a + b);
        assert_eq!(left_to_right, 1e16);
        assert_eq!(sum(&v), 1e16 + 1024.0);
    }

    #[test]
    fn empty_reductions() {
        assert_eq!(sum(&[]).to_bits(), 0);
        assert_eq!(product(&[]), 1.0);
        assert_eq!(mean(&[]).map_err(|e| e.code()), Err("num.empty_reduction"));
        assert_eq!(
            minimum(&[]).map_err(|e| e.code()),
            Err("num.empty_reduction")
        );
        assert_eq!(
            maximum(&[]).map_err(|e| e.code()),
            Err("num.empty_reduction")
        );
    }

    #[test]
    fn minimum_and_maximum_keep_signed_zero_rules() {
        assert_eq!(
            minimum(&[0.0, -0.0, 1.0]).map(f64::to_bits),
            Ok(0x8000_0000_0000_0000)
        );
        assert_eq!(maximum(&[-0.0, 0.0, -1.0]).map(f64::to_bits), Ok(0));
        assert_eq!(mean(&[1.0, 2.0]), Ok(1.5));
    }
}
