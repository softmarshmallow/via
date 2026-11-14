//! A checked formula: a flat graph of operations the evaluator runs.
//!
//! Nodes are numbered children first, and a `let` read twice is one node, so a node's index is
//! its place in evaluation order (`formulas.md` 9: operands left to right, then the operation; a
//! shared part at its first read) and every part is computed once per window whatever reads it
//! (4.2, "Sharing").

use std::collections::BTreeSet;
use std::ops::Range;

use crate::context::{Key, NoisePart};
use crate::error::{At, FormulaId};
use crate::parse::{Arith, Cmp};
use crate::table::{Class, Rate};

/// A value type (`formulas.md` 4.1). Selectors never reach a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Number,
    YesNo,
}

impl Type {
    pub fn word(self) -> &'static str {
        match self {
            Type::Number => "a number",
            Type::YesNo => "a yes/no",
        }
    }
}

/// A measure of a feature at a point (`numbers.md` 5.5), or membership (`formulas.md` 8.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Measure {
    D,
    Sd,
    T,
    /// 1 inside a region, outline included, else 0.
    Inside,
}

/// A statistic of `under` (`formulas.md` 8.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stat {
    Mean,
    Min,
    Max,
}

impl Stat {
    pub fn word(self) -> &'static str {
        match self {
            Stat::Mean => "mean",
            Stat::Min => "min",
            Stat::Max => "max",
        }
    }
}

/// A noise call's stream (`formulas.md` 8.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Seed {
    /// `stream(world seed, ["noise", "seed", <name>])`.
    Named(String),
    /// The owner's default parts.
    Default(Vec<NoisePart>),
}

/// An operation. Operands are node indices, always lower than the node's own.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Const(f64),
    X,
    Y,
    /// `v`, the written field before this entry.
    V,
    /// `d`, `sd` or `t` of the formula's own feature.
    Local(Measure),
    /// A bare attribute of the feature the formula serves.
    Attr(String),
    /// `<id>.<attribute>`.
    Member(String, String),
    Neg(usize),
    Not(usize),
    Arith(Arith, usize, usize),
    Cmp(Cmp, usize, usize),
    And(usize, usize),
    Or(usize, usize),
    Abs(usize),
    Sqrt(usize),
    Pow(usize, usize),
    Exp(usize),
    Floor(usize),
    Sin(usize),
    Cos(usize),
    Tan(usize),
    Min(usize, usize),
    Max(usize, usize),
    Clamp(usize, usize, usize),
    Mix(usize, usize, usize),
    Smoothstep(usize, usize, usize),
    If(usize, usize, usize),
    Fbm {
        wavelength: usize,
        octaves: usize,
        gain: usize,
        lacunarity: usize,
        seed: Seed,
        /// `ridged` rather than `fbm` (`formulas.md` 8.2).
        ridged: bool,
    },
    /// `f(field)`: selector index.
    Field(usize),
    /// `slope(field)`.
    Slope(usize),
    Dist {
        shape: usize,
        cap: usize,
    },
    Sdist {
        shape: usize,
        cap: usize,
    },
    Near {
        shape: usize,
        radius: usize,
    },
    Inside(usize),
    Along(usize),
    Under {
        field: Option<usize>,
        stat: Stat,
    },
    SmoothedAlong(usize),
}

impl Op {
    /// The operand nodes, in evaluation order.
    pub fn operands(&self) -> Vec<usize> {
        match self {
            Op::Const(_)
            | Op::X
            | Op::Y
            | Op::V
            | Op::Local(_)
            | Op::Attr(_)
            | Op::Member(..)
            | Op::Field(_)
            | Op::Slope(_)
            | Op::Inside(_)
            | Op::Along(_)
            | Op::Under { .. } => Vec::new(),
            Op::Neg(a)
            | Op::Not(a)
            | Op::Abs(a)
            | Op::Sqrt(a)
            | Op::Exp(a)
            | Op::Floor(a)
            | Op::Sin(a)
            | Op::Cos(a)
            | Op::Tan(a)
            | Op::SmoothedAlong(a) => vec![*a],
            Op::Arith(_, a, b)
            | Op::Cmp(_, a, b)
            | Op::And(a, b)
            | Op::Or(a, b)
            | Op::Pow(a, b)
            | Op::Min(a, b)
            | Op::Max(a, b) => vec![*a, *b],
            Op::Clamp(a, b, c) | Op::Mix(a, b, c) | Op::Smoothstep(a, b, c) | Op::If(a, b, c) => {
                vec![*a, *b, *c]
            }
            Op::Fbm {
                wavelength,
                octaves,
                gain,
                lacunarity,
                ..
            } => vec![*wavelength, *octaves, *gain, *lacunarity],
            Op::Dist { cap, .. } | Op::Sdist { cap, .. } => vec![*cap],
            Op::Near { radius, .. } => vec![*radius],
        }
    }
}

/// One node.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub op: Op,
    pub ty: Type,
    pub rate: Rate,
    /// The node whose rate set this one's (itself for a leaf or a call whose own rate rules).
    pub rate_from: usize,
    /// The sub-expression's bytes, in the formula it was written in.
    pub at: At,
    /// Its text, for messages.
    pub text: String,
    /// The operation's name, for `formula.not_finite` (`sqrt`, `/`).
    pub name: String,
    /// Inside a `let`: the reading formula's span of the read, for `related`.
    pub via_let: Option<(String, At)>,
}

/// A selector a call reads, resolved in the selection phase (`formulas.md` 7).
#[derive(Clone, Debug, PartialEq)]
pub struct SelectorUse {
    pub class: Class,
    /// The selector's contents, without quotes.
    pub text: String,
    /// The function reading it.
    pub function: &'static str,
    pub at: At,
}

/// What a formula reads, for the graph's edges (`graph.md`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadSet {
    /// `let` names, as `(scope words, name)`.
    pub lets: BTreeSet<(String, String)>,
    /// Bare attributes of the feature served.
    pub attributes: BTreeSet<String>,
    /// `<id>.<attribute>` reads.
    pub members: BTreeSet<(String, String)>,
    /// Field and shape selectors.
    pub selectors: BTreeSet<(Class, String)>,
    /// Whether `v`, one-argument `under` or `smoothed_along` is read: shared per (feature,
    /// effect, written field), not per feature (4.2).
    pub entry_field: bool,
}

/// A checked formula.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub formula: FormulaId,
    pub key: Key,
    pub nodes: Vec<Node>,
    pub root: usize,
    pub selectors: Vec<SelectorUse>,
    /// Noise calls: their stream and place (for `formula.seed_shared`).
    pub noise: Vec<(Seed, At)>,
    pub reads: ReadSet,
    /// The posts around a post the formula reads (`numbers.md` 6): 1 with `slope`, else 0.
    pub halo_posts: u32,
    /// The whole formula's bytes.
    pub span: Range<u32>,
}

impl Program {
    /// The result type.
    pub fn ty(&self) -> Type {
        self.nodes[self.root].ty
    }

    /// The rate.
    pub fn rate(&self) -> Rate {
        self.nodes[self.root].rate
    }

    /// How many operations: a `let` read many times counts once.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Never empty: a formula has at least one node.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The value when the formula is a constant that reads nothing, for checks made before any
    /// node runs.
    pub fn constant(&self) -> Option<f64> {
        crate::eval::fold(self, self.root)
    }
}
