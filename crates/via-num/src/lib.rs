//! The one maths module (R26).
//!
//! Responsibility: every floating-point operation beyond `+ - * / sqrt` and the exact ones of
//! `spec/numbers.md` section 8, the defined `min` and `max`, fixed-order reductions, checked
//! lattice integers and the named seed streams. It exists so that the only place where systems
//! could disagree about a bit is one crate whose every function is pinned to `libm` with soft
//! floats; every other crate is denied the platform's float methods by `clippy.toml`.
//!
//! Governed by `spec/numbers.md`: section 1 to 3 for units and the exact reading of decimal text,
//! 4 for the grid of posts, 5.1 and 5.4 for frames, 7 for the seeds, 8 for the maths and 10 for
//! the vectors. The gradient noise is `spec/formulas.md` 8.2's.
//!
//! What is not here: the measures from a feature and path samples (`numbers.md` 5.5 and 6) need
//! the exact predicates of `geometry.md` 1 and live with them in `via-geom`, which calls this
//! crate for every double it computes.

mod decimal;
mod error;
mod frame;
mod lattice;
mod math;
mod noise;
mod reduce;
mod seed;
pub mod vectors;
mod write;

pub use decimal::{
    normalise_facing, parse_double, parse_facing, parse_length, parse_millionths, FULL_TURN,
    LATTICE_MAX,
};
pub use error::{ExtentHint, NumError, Quantity};
pub use frame::Frame;
pub use lattice::{
    checked_add, checked_mul, checked_sub, in_lattice, metres, round_to_lattice, write_facing,
    write_length, write_seed, Grid, DEFAULT_RESOLUTION_MM, POST_LIMIT,
};
pub use math::{
    acos, asin, atan, atan2, bearing, cbrt, clamp, cos, cos_deg, cosh, exp, exp2, exp_m1,
    facing_sin_cos, finite, fmod, hypot, ln, ln_1p, log10, log2, max, min, mix, positive_zero, pow,
    sin, sin_deg, sinh, store, store_f32, tan, tan_deg, tanh, DEGREES_PER_RAD,
    MILLIDEGREES_PER_RAD, PI, RAD_PER_DEGREE, RAD_PER_MILLIDEGREE,
};
pub use noise::{gradient_index, gradient_noise};
pub use reduce::{maximum, mean, minimum, product, sum, CHUNK};
pub use seed::{
    absorb, draw, mix as mix_word, seed_name, splitmix64, stream, unit, world_seed, ROOT,
};
pub use write::write_double;
