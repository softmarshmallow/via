//! Formulas.
//!
//! Responsibility: the formula language: parsing, the name, type, rate and halo checks made
//! before anything runs, and evaluation through `via-num`, with dispatch from the function table
//! (`spec/functions.toml`, held equal to [`table::TABLE`] by this crate's tests) so the
//! evaluator, the reference page and the SDK's list cannot drift apart.
//!
//! Governed by `spec/formulas.md`.
//!
//! # How a formula travels
//!
//! 1. **Text phase.** The document model registers every formula string in an [`Env`]
//!    ([`Env::text`], which parses it: marking, size, characters, syntax) and declares the names
//!    of the world: `let`s by scope, kind attributes, features and fields. [`Env::check`] checks
//!    one formula in its [`Context`] (its key, scope and default noise stream) and gives a
//!    [`Program`]: names resolved by context, types and rates checked, arguments bound, selectors
//!    checked for form and class, caps defaulted, constant parameter ranges checked.
//!    [`Env::shadows`], [`Env::check_lets`] and [`seed_shared`] are the checks across formulas.
//! 2. **Selection phase.** [`bind`] resolves the program's selectors through [`Select`] at the
//!    reading node's stratum: counts, classes, shapes, empty selections.
//! 3. **Value phase.** [`evaluate`] runs the program over a [`Domain`], a window of posts or a list
//!    of points, reading fields and geometry through [`Reads`]; a window's values are the same cut
//!    of the whole. A failure is the first live failing operation in post order ([`Failure`]),
//!    found by the explain pass.
//!
//! Every error is an [`ExprError`] located in formula terms (a byte range of the formula's string
//! value); [`ExprError::to_diag`] makes the diagnostic once the caller supplies subjects and sites.

mod check;
mod context;
mod env;
mod error;
mod eval;
mod lex;
mod noise;
mod parse;
mod program;
pub mod table;

pub use context::{Builtin, Context, Key, KeyRange, NoisePart, Results, RESERVED};
pub use env::{seed_shared, AttrType, Env, FeatureDecl, KindDecl};
pub use error::{At, Detail, ExprError, FormulaId, Scope};
pub use eval::{
    bind, evaluate, narrow, Binding, Domain, Failure, FieldInfo, ReadError, Reads, Scalar, Select,
    Setup, Shape, ShapeInfo, Target, Window,
};
pub use lex::MAX_LEN;
pub use noise::fbm;
pub use parse::{marked, Mark, Marked, MAX_DEPTH};
pub use program::{Measure, Node, Op, Program, ReadSet, Seed, SelectorUse, Stat, Type};
pub use table::{Class, Fun, Rate};
