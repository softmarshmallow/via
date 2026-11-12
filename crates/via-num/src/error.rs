//! The errors of this crate, as values.
//!
//! `via-num` sits below `via-diag`, so it cannot build a located diagnostic itself: it knows the
//! value at fault but not the file, the key or the subject. Each error carries its code (the
//! string of `spec/diagnostics.md`'s registry) and the facts its hint needs, and the caller, which
//! knows the site, turns it into a `Diag`. Nothing here panics on bad input.

use std::fmt;

/// What a number read from text measures, so a `num.too_precise` hint can be written in the unit
/// the author wrote (`numbers.md` 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    /// A length: metres in text, millimetres held (3.1).
    Length,
    /// A facing: degrees in text, millidegrees held (3.2).
    Facing,
    /// A `jitter` or a `weight`: held in millionths (3.3).
    Millionths,
}

impl Quantity {
    /// How many decimals of the text unit one held step is: 3 for millimetres and millidegrees,
    /// 6 for millionths.
    pub fn decimals(self) -> u32 {
        match self {
            Quantity::Length | Quantity::Facing => 3,
            Quantity::Millionths => 6,
        }
    }
}

/// The hint of `num.extent_not_multiple`: the extent's far corner rounded down and up to multiples
/// of the resolution, and the nearest resolutions that divide both widths (`numbers.md` 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtentHint {
    /// `x1` rounded down and up to a multiple of `r` from `x0`, in millimetres.
    pub x1: [i64; 2],
    /// `y1` rounded down and up to a multiple of `r` from `y0`, in millimetres.
    pub y1: [i64; 2],
    /// The largest resolution below `r` that divides both widths, in millimetres.
    pub resolution_below: Option<i64>,
    /// The smallest resolution above `r` that divides both widths, if there is one.
    pub resolution_above: Option<i64>,
}

/// An error of `via-num`. [`NumError::code`] gives its registry code.
#[derive(Clone, Debug, PartialEq)]
pub enum NumError {
    /// The text is not a number of the normalised form `documents.md` hands over. A reader that
    /// lets such text through is the defect; reported as `doc.syntax`.
    Malformed { text: String },
    /// `inf` or `nan` in text (`num.not_finite`).
    NotFinite { text: String },
    /// Not a whole number of the held step (`num.too_precise`). `nearest` holds the two nearest
    /// values that are, below and above, as exact text in the unit the author wrote.
    TooPrecise {
        text: String,
        quantity: Quantity,
        nearest: [String; 2],
    },
    /// Out of range for its quantity (`num.out_of_range`).
    OutOfRange { text: String },
    /// `extent` with `x1 <= x0` or `y1 <= y0` (`num.extent_empty`).
    ExtentEmpty { extent: [i64; 4] },
    /// `resolution` below 1 mm (`num.resolution`).
    Resolution { resolution: i64 },
    /// A width or height not a whole multiple of the resolution (`num.extent_not_multiple`).
    ExtentNotMultiple {
        extent: [i64; 4],
        resolution: i64,
        hint: ExtentHint,
    },
    /// More posts than the limit (`num.too_many_posts`).
    TooManyPosts { nx: u64, ny: u64, limit: u64 },
    /// A negative world seed (`num.seed_range`).
    SeedRange { seed: i64 },
    /// A seed of the wrong form (`num.seed_type`). `hint` is the lower-cased name when that one
    /// is valid.
    SeedType { text: String, hint: Option<String> },
    /// A checked integer operation overflowed (`num.overflow`). `op` names the operation; the
    /// caller adds the subject and the position.
    Overflow { op: &'static str },
    /// A mean, minimum or maximum over nothing (`num.empty_reduction`).
    EmptyReduction { op: &'static str },
    /// A value about to be stored is infinite or NaN (`num.not_finite_result`). A formula maps
    /// this to `formula.not_finite`.
    NotFiniteResult { bits: u64 },
    /// A computed position rounded outside the lattice (`geom.range`, `numbers.md` 3.1).
    LatticeRange,
    /// A noise point beyond the hashable lattice (`formula.range` on `wavelength`,
    /// `formulas.md` 8.2).
    NoiseRange,
}

impl NumError {
    /// The code of `spec/diagnostics.md`'s registry for this error.
    pub fn code(&self) -> &'static str {
        match self {
            NumError::Malformed { .. } => "doc.syntax",
            NumError::NotFinite { .. } => "num.not_finite",
            NumError::TooPrecise { .. } => "num.too_precise",
            NumError::OutOfRange { .. } => "num.out_of_range",
            NumError::ExtentEmpty { .. } => "num.extent_empty",
            NumError::Resolution { .. } => "num.resolution",
            NumError::ExtentNotMultiple { .. } => "num.extent_not_multiple",
            NumError::TooManyPosts { .. } => "num.too_many_posts",
            NumError::SeedRange { .. } => "num.seed_range",
            NumError::SeedType { .. } => "num.seed_type",
            NumError::Overflow { .. } => "num.overflow",
            NumError::EmptyReduction { .. } => "num.empty_reduction",
            NumError::NotFiniteResult { .. } => "num.not_finite_result",
            NumError::LatticeRange => "geom.range",
            NumError::NoiseRange => "formula.range",
        }
    }
}

impl fmt::Display for NumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumError::Malformed { text } => write!(f, "`{text}` is not a number"),
            NumError::NotFinite { text } => write!(f, "`{text}` is not a finite number"),
            NumError::TooPrecise {
                text,
                quantity,
                nearest,
            } => {
                let step = match quantity {
                    Quantity::Length => "millimetres",
                    Quantity::Facing => "millidegrees",
                    Quantity::Millionths => "millionths",
                };
                write!(
                    f,
                    "`{text}` is not a whole number of {step}; the nearest that are: {} and {}",
                    nearest[0], nearest[1]
                )
            }
            NumError::OutOfRange { text } => write!(f, "`{text}` is out of range"),
            NumError::ExtentEmpty { extent } => {
                write!(f, "extent {extent:?} mm has x1 <= x0 or y1 <= y0")
            }
            NumError::Resolution { resolution } => {
                write!(f, "resolution {resolution} mm is below 1 mm")
            }
            NumError::ExtentNotMultiple {
                extent,
                resolution,
                hint,
            } => write!(
                f,
                "extent {extent:?} mm is not a whole multiple of resolution {resolution} mm \
                 (x1 {:?}, y1 {:?}, resolutions {:?} and {:?})",
                hint.x1, hint.y1, hint.resolution_below, hint.resolution_above
            ),
            NumError::TooManyPosts { nx, ny, limit } => {
                write!(f, "{nx} by {ny} posts exceeds the limit of {limit}")
            }
            NumError::SeedRange { seed } => write!(f, "world seed {seed} is negative"),
            NumError::SeedType { text, hint } => match hint {
                Some(h) => write!(f, "seed `{text}` is not a name; try `{h}`"),
                None => write!(f, "seed `{text}` is not a name"),
            },
            NumError::Overflow { op } => write!(f, "integer overflow in {op}"),
            NumError::EmptyReduction { op } => write!(f, "{op} over no values"),
            NumError::NotFiniteResult { bits } => {
                write!(f, "a non-finite value (bits {bits:#018X}) would be stored")
            }
            NumError::LatticeRange => write!(f, "a computed position is outside the lattice"),
            NumError::NoiseRange => write!(f, "a noise point is beyond the hashable lattice"),
        }
    }
}

impl std::error::Error for NumError {}
