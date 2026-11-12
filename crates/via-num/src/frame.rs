//! Placed and nested frames (`numbers.md` 5.1 and 5.4).
//!
//! A frame is an origin on the lattice, a facing in millidegrees and a scale. A point written in it
//! reaches the world by one fixed expression and one rounding, however deep the nesting, so the
//! result is the same bits everywhere. The quadrant reduction, the order and the rounding rule are
//! ours.

use crate::error::NumError;
use crate::lattice::{checked_add, in_lattice};
use crate::math::facing_sin_cos;

/// A placed frame in the world: its axes are `x` to its right, `y` forward, `z` up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The origin in world millimetres, a lattice point.
    pub origin: (i64, i64),
    /// The facing, millidegrees in `[0, 360000)`.
    pub facing: i64,
    /// The scale, a double greater than 0.
    pub scale: f64,
}

impl Frame {
    /// The world frame: origin `(0, 0)`, facing north, scale 1.
    pub const WORLD: Frame = Frame {
        origin: (0, 0),
        facing: 0,
        scale: 1.0,
    };

    /// A frame placed in the world; the facing is normalised. 5.1 asks `k > 0`: a scale that is
    /// zero, negative, NaN or infinite is `num.out_of_range`, refused here so no frame that would
    /// fold or mirror the points written in it can exist. An origin off the lattice is
    /// `geom.range`.
    pub fn new(origin: (i64, i64), facing: i64, scale: f64) -> Result<Frame, NumError> {
        check_scale(scale)?;
        if !in_lattice(origin.0) || !in_lattice(origin.1) {
            return Err(NumError::LatticeRange);
        }
        Ok(Frame {
            origin,
            facing: crate::decimal::normalise_facing(facing),
            scale,
        })
    }

    /// Takes a point written in this frame, in millimetres, to the world (5.1): scale first, then
    /// the turn, each product and sum one double operation in the order written, and one rounding.
    /// Only the world point `(px, py)` is held to the lattice: the rounded offset may itself exceed
    /// it (origin -2,000 km, offset +3,000 km, point +1,000 km is legitimate), so it is rounded to
    /// an `i64` with no lattice bound. A point off the lattice, or an offset no `i64` holds, is
    /// `geom.range`; an origin sum that overflows is `num.overflow`.
    pub fn to_world(&self, local: (i64, i64)) -> Result<(i64, i64), NumError> {
        let (s, c) = facing_sin_cos(self.facing);
        let a = (local.0 as f64) * self.scale;
        let b = (local.1 as f64) * self.scale;
        let dx = round_offset((a * c) + (b * s))?;
        let dy = round_offset((-(a * s)) + (b * c))?;
        let px = checked_add(self.origin.0, dx, "frame x")?;
        let py = checked_add(self.origin.1, dy, "frame y")?;
        if !in_lattice(px) || !in_lattice(py) {
            return Err(NumError::LatticeRange);
        }
        Ok((px, py))
    }

    /// The frame written in this one at local origin `origin`, local facing `facing` and local
    /// scale `scale` (5.4): its origin taken to the world by this frame and rounded once, its facing
    /// the sum modulo a turn, its scale one multiplication from the root outward. The local scale
    /// must be greater than 0, and so must the product: two tiny scales can underflow to 0 and two
    /// huge ones overflow to infinity, either is `num.out_of_range`.
    pub fn nest(&self, origin: (i64, i64), facing: i64, scale: f64) -> Result<Frame, NumError> {
        check_scale(scale)?;
        let composed = self.scale * scale;
        check_scale(composed)?;
        Ok(Frame {
            origin: self.to_world(origin)?,
            facing: crate::decimal::normalise_facing(
                self.facing + crate::decimal::normalise_facing(facing),
            ),
            scale: composed,
        })
    }

    /// A height written in this frame, in the world: multiplied by the scale, not rounded.
    pub fn height_to_world(&self, z: f64) -> f64 {
        z * self.scale
    }

    /// A facing written in this frame, in the world: the sum, modulo a turn.
    pub fn facing_to_world(&self, facing: i64) -> i64 {
        crate::decimal::normalise_facing(self.facing + crate::decimal::normalise_facing(facing))
    }
}

/// 5.1's `k > 0`, finite. The error carries the scale as its shortest round-trip text where it has
/// one, so the diagnostic names the value the author can find.
fn check_scale(scale: f64) -> Result<(), NumError> {
    if scale.is_finite() && scale > 0.0 {
        Ok(())
    } else {
        Err(NumError::OutOfRange {
            text: crate::write::write_double(scale).unwrap_or_else(|_| format!("{scale}")),
        })
    }
}

/// Rounds a frame offset to the nearest integer, ties to even, with no lattice bound (5.1 bounds
/// only the world point). Not finite, or beyond what an `i64` holds, is `geom.range`: no world
/// point on the lattice can come of it.
fn round_offset(mm: f64) -> Result<i64, NumError> {
    let r = mm.round_ties_even();
    // 2^63 as a double is exactly i64::MAX + 1; compare as doubles because `as` saturates silently.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if (-LIMIT..LIMIT).contains(&r) {
        Ok(r as i64)
    } else {
        Err(NumError::LatticeRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_beyond_lattice_reaches_a_lattice_point() {
        // Origin -2,000 km, offset +3,000 km, point +1,000 km: the offset alone is off the lattice.
        let f = Frame::new((-2_000_000_000, 0), 0, 1.0).unwrap();
        assert_eq!(f.to_world((3_000_000_000, 0)), Ok((1_000_000_000, 0)));
        let g = Frame::new((0, -2_000_000_000), 0, 1.5).unwrap();
        assert_eq!(g.to_world((0, 2_000_000_000)), Ok((0, 1_000_000_000)));
    }

    #[test]
    fn point_off_the_lattice_is_geom_range() {
        let f = Frame::new((2_000_000_000, 0), 0, 1.0).unwrap();
        assert_eq!(f.to_world((200_000_000, 0)), Err(NumError::LatticeRange));
        let big = Frame::new((0, 0), 0, 1e300).unwrap();
        assert_eq!(big.to_world((1, 0)), Err(NumError::LatticeRange));
    }

    #[test]
    fn scale_must_be_positive_and_finite() {
        for k in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY] {
            let e = Frame::new((0, 0), 0, k).unwrap_err();
            assert_eq!(e.code(), "num.out_of_range", "scale {k}");
            let e = Frame::WORLD.nest((0, 0), 0, k).unwrap_err();
            assert_eq!(e.code(), "num.out_of_range", "nested scale {k}");
        }
        let tiny = Frame::new((0, 0), 0, 1e-200).unwrap();
        assert_eq!(
            tiny.nest((0, 0), 0, 1e-200).unwrap_err().code(),
            "num.out_of_range"
        );
    }

    #[test]
    fn origin_off_the_lattice_is_refused() {
        assert_eq!(
            Frame::new((3_000_000_000, 0), 0, 1.0),
            Err(NumError::LatticeRange)
        );
    }
}
