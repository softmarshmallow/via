//! The function table (`formulas.md` 8; machine copy `spec/functions.toml`, R33).
//!
//! One row per function, in the order of section 8. Each row holds the columns of
//! `spec/functions.toml` as written (signatures, rate, halo, where, milestone), which this
//! crate's test `tests/table.rs` holds equal to that file, and the structured parameters the
//! checker and the evaluator dispatch on. A function added here without a row there, or the
//! reverse, fails that test.

/// How often a value can change (`formulas.md` 4.2), in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rate {
    Constant,
    PerFeature,
    PerPost,
}

impl Rate {
    /// The word of section 4.2.
    pub fn word(self) -> &'static str {
        match self {
            Rate::Constant => "constant",
            Rate::PerFeature => "per feature",
            Rate::PerPost => "per post",
        }
    }
}

/// What class of thing a selector parameter accepts (`formulas.md` 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    Field,
    Shape,
    Seed,
    Statistic,
}

impl Class {
    pub fn word(self) -> &'static str {
        match self {
            Class::Field => "field",
            Class::Shape => "shape",
            Class::Seed => "seed",
            Class::Statistic => "statistic",
        }
    }
}

/// A parameter's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamType {
    Number,
    YesNo,
    /// `if`'s branches: two of one type, number or yes/no.
    Same,
    Selector(Class),
}

/// A parameter's lower bound, where its range is stated (`formulas.md` 8.2, 8.4, 8.5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bound {
    /// No range of its own.
    None,
    /// Above 0.
    Positive,
    /// A whole number from 1 to 12.
    Octaves,
    /// `0 < gain <= 1`.
    Gain,
    /// Above 1.
    AboveOne,
}

/// What a parameter takes when not given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Default {
    /// Required.
    Required,
    /// A number.
    Number(f64),
    /// The formula owner's stream (`formulas.md` 8.2, "Default seed").
    OwnerSeed,
    /// The effect's or kind's `radius` where there is one, else `formula.cap` (8.4).
    Radius,
}

/// One parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub ty: ParamType,
    pub default: Default,
    /// The highest rate it may have.
    pub max_rate: Rate,
    pub bound: Bound,
}

/// The function a row dispatches to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fun {
    Abs,
    Sqrt,
    Pow,
    Exp,
    Floor,
    Sin,
    Cos,
    Tan,
    Min,
    Max,
    Clamp,
    Mix,
    Smoothstep,
    If,
    Fbm,
    Ridged,
    F,
    Slope,
    Dist,
    Sdist,
    Near,
    Inside,
    Along,
    Under,
    SmoothedAlong,
}

/// The result type of a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returns {
    Number,
    YesNo,
    /// `if`: the type of its branches.
    Branches,
}

/// What a function reads beyond the point (`formulas.md` 8.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Halo {
    None,
    /// `f`: none at a post, one post at a point that is not one (fields.md 1.2).
    PostAtPoint,
    /// One post around (`slope`).
    OnePost,
    /// Geometry within a cap or radius argument.
    Cap,
    /// Geometry within the window.
    Window,
    /// The whole feature.
    Feature,
    /// The feature's footprint.
    Footprint,
}

/// Where a function may appear, beyond what its rate allows (`formulas.md` 8, **Where**).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Any,
    /// A kind effect or a kind `let`.
    KindEffectOrLet,
    /// A kind effect on a scalar field or a kind `let`.
    ScalarKindEffectOrLet,
}

/// One row of section 8.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Function {
    pub fun: Fun,
    pub name: &'static str,
    /// Its place in section 8.
    pub order: u32,
    pub section: &'static str,
    /// `signatures`, as written in `spec/functions.toml`.
    pub signatures: &'static [&'static str],
    pub params: &'static [Param],
    /// Two or more of its one parameter (`min`, `max`).
    pub variadic: bool,
    pub result: Returns,
    /// Its own rate; `None` is the table's "none".
    pub rate: Option<Rate>,
    /// `rate`, as written.
    pub rate_text: &'static str,
    pub halo: Halo,
    /// `halo`, as written.
    pub halo_text: &'static str,
    pub where_: Where,
    /// `where`, as written.
    pub where_text: &'static str,
    /// The milestone that needs it; a later one is `doc.not_yet` now.
    pub milestone: u32,
}

const fn num(name: &'static str) -> Param {
    Param {
        name,
        ty: ParamType::Number,
        default: Default::Required,
        max_rate: Rate::PerPost,
        bound: Bound::None,
    }
}

const fn feature_num(name: &'static str, default: Default, bound: Bound) -> Param {
    Param {
        name,
        ty: ParamType::Number,
        default,
        max_rate: Rate::PerFeature,
        bound,
    }
}

const fn sel(name: &'static str, class: Class) -> Param {
    Param {
        name,
        ty: ParamType::Selector(class),
        default: Default::Required,
        max_rate: Rate::Constant,
        bound: Bound::None,
    }
}

const A: &[Param] = &[num("a")];
const AB: &[Param] = &[num("a"), num("b")];
const NOISE: &[Param] = &[
    feature_num("wavelength", Default::Required, Bound::Positive),
    feature_num("octaves", Default::Number(4.0), Bound::Octaves),
    Param {
        name: "seed",
        ty: ParamType::Selector(Class::Seed),
        default: Default::OwnerSeed,
        max_rate: Rate::Constant,
        bound: Bound::None,
    },
    feature_num("gain", Default::Number(0.5), Bound::Gain),
    feature_num("lacunarity", Default::Number(2.0), Bound::AboveOne),
];
const FIELD: &[Param] = &[sel("field", Class::Field)];
const SHAPE: &[Param] = &[sel("shape", Class::Shape)];
const SHAPE_CAP: &[Param] = &[
    sel("shape", Class::Shape),
    feature_num("cap", Default::Radius, Bound::Positive),
];
const SHAPE_RADIUS: &[Param] = &[
    sel("shape", Class::Shape),
    feature_num("radius", Default::Required, Bound::Positive),
];
const UNDER: &[Param] = &[
    Param {
        name: "field",
        ty: ParamType::Selector(Class::Field),
        default: Default::Required,
        max_rate: Rate::Constant,
        bound: Bound::None,
    },
    sel("stat", Class::Statistic),
];
const LENGTH: &[Param] = &[feature_num("length", Default::Required, Bound::Positive)];

#[allow(clippy::too_many_arguments)]
const fn row(
    fun: Fun,
    name: &'static str,
    order: u32,
    section: &'static str,
    signatures: &'static [&'static str],
    params: &'static [Param],
    result: Returns,
    rate: Option<Rate>,
    rate_text: &'static str,
    halo: Halo,
    halo_text: &'static str,
    milestone: u32,
) -> Function {
    Function {
        fun,
        name,
        order,
        section,
        signatures,
        params,
        variadic: false,
        result,
        rate,
        rate_text,
        halo,
        halo_text,
        where_: Where::Any,
        where_text: "any",
        milestone,
    }
}

const fn arith(
    fun: Fun,
    name: &'static str,
    order: u32,
    sig: &'static [&'static str],
    params: &'static [Param],
) -> Function {
    row(
        fun,
        name,
        order,
        "8.1",
        sig,
        params,
        Returns::Number,
        None,
        "none",
        Halo::None,
        "none",
        1,
    )
}

const fn variadic(mut f: Function) -> Function {
    f.variadic = true;
    f
}

const fn only(mut f: Function, where_: Where, text: &'static str) -> Function {
    f.where_ = where_;
    f.where_text = text;
    f
}

/// The table, in the order of section 8.
pub const TABLE: &[Function] = &[
    arith(Fun::Abs, "abs", 1, &["abs(a)"], A),
    arith(Fun::Sqrt, "sqrt", 2, &["sqrt(a)"], A),
    arith(Fun::Pow, "pow", 3, &["pow(a, b)"], AB),
    arith(Fun::Exp, "exp", 4, &["exp(a)"], A),
    arith(Fun::Floor, "floor", 5, &["floor(a)"], A),
    arith(Fun::Sin, "sin", 6, &["sin(a)"], A),
    arith(Fun::Cos, "cos", 7, &["cos(a)"], A),
    arith(Fun::Tan, "tan", 8, &["tan(a)"], A),
    variadic(arith(Fun::Min, "min", 9, &["min(a, b, ...)"], A)),
    variadic(arith(Fun::Max, "max", 10, &["max(a, b, ...)"], A)),
    arith(
        Fun::Clamp,
        "clamp",
        11,
        &["clamp(a, lo, hi)"],
        &[num("a"), num("lo"), num("hi")],
    ),
    arith(
        Fun::Mix,
        "mix",
        12,
        &["mix(a, b, s)"],
        &[num("a"), num("b"), num("s")],
    ),
    arith(
        Fun::Smoothstep,
        "smoothstep",
        13,
        &["smoothstep(e0, e1, a)"],
        &[num("e0"), num("e1"), num("a")],
    ),
    row(
        Fun::If,
        "if",
        14,
        "8.1",
        &["if(c, a, b)"],
        &[
            Param {
                name: "c",
                ty: ParamType::YesNo,
                default: Default::Required,
                max_rate: Rate::PerPost,
                bound: Bound::None,
            },
            Param {
                name: "a",
                ty: ParamType::Same,
                default: Default::Required,
                max_rate: Rate::PerPost,
                bound: Bound::None,
            },
            Param {
                name: "b",
                ty: ParamType::Same,
                default: Default::Required,
                max_rate: Rate::PerPost,
                bound: Bound::None,
            },
        ],
        Returns::Branches,
        None,
        "none",
        Halo::None,
        "none",
        1,
    ),
    row(
        Fun::Fbm,
        "fbm",
        15,
        "8.2",
        &["fbm(wavelength, octaves = 4, seed = <owner>, gain = 0.5, lacunarity = 2)"],
        NOISE,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::None,
        "none",
        1,
    ),
    row(
        Fun::Ridged,
        "ridged",
        16,
        "8.2",
        &["ridged(...)"],
        NOISE,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::None,
        "none",
        2,
    ),
    row(
        Fun::F,
        "f",
        17,
        "8.3",
        &["f(field)"],
        FIELD,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::PostAtPoint,
        "none at a post; 1 post at a point (6.9)",
        1,
    ),
    row(
        Fun::Slope,
        "slope",
        18,
        "8.3",
        &["slope(field)"],
        FIELD,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::OnePost,
        "1 post",
        1,
    ),
    row(
        Fun::Dist,
        "dist",
        19,
        "8.4",
        &["dist(shape, cap)"],
        SHAPE_CAP,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::Cap,
        "geometry within `cap`",
        1,
    ),
    row(
        Fun::Sdist,
        "sdist",
        20,
        "8.4",
        &["sdist(shape, cap)"],
        SHAPE_CAP,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::Cap,
        "geometry within `cap`",
        1,
    ),
    row(
        Fun::Near,
        "near",
        21,
        "8.4",
        &["near(shape, radius)"],
        SHAPE_RADIUS,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::Cap,
        "geometry within `radius`",
        1,
    ),
    row(
        Fun::Inside,
        "inside",
        22,
        "8.4",
        &["inside(shape)"],
        SHAPE,
        Returns::YesNo,
        Some(Rate::PerPost),
        "per post",
        Halo::Window,
        "geometry within the window",
        1,
    ),
    row(
        Fun::Along,
        "along",
        23,
        "8.4",
        &["along(shape)"],
        SHAPE,
        Returns::Number,
        Some(Rate::PerPost),
        "per post",
        Halo::Feature,
        "the whole feature",
        1,
    ),
    only(
        row(
            Fun::Under,
            "under",
            24,
            "8.5",
            &["under(stat)", "under(field, stat)"],
            UNDER,
            Returns::Number,
            Some(Rate::PerFeature),
            "per feature",
            Halo::Footprint,
            "the feature's footprint",
            1,
        ),
        Where::KindEffectOrLet,
        "kind effect, kind `let`",
    ),
    only(
        row(
            Fun::SmoothedAlong,
            "smoothed_along",
            25,
            "8.5",
            &["smoothed_along(length)"],
            LENGTH,
            Returns::Number,
            Some(Rate::PerPost),
            "per post",
            Halo::Feature,
            "the whole feature",
            1,
        ),
        Where::ScalarKindEffectOrLet,
        "kind effect on a scalar field, kind `let`",
    ),
];

/// The row of a function by name.
pub fn lookup(name: &str) -> Option<&'static Function> {
    TABLE.iter().find(|f| f.name == name)
}

/// The fixed hints of `formula.unknown_function` (`formulas.md`, Diagnostics), tried before the
/// nearest name.
pub fn alias(name: &str) -> Option<&'static str> {
    Some(match name {
        "clip" => "write `clamp`",
        "maximum" => "write `max`",
        "minimum" => "write `min`",
        "where" => "write `if`; `where` is a feature key",
        "lerp" => "write `mix`",
        "radians" | "degrees" => "angles are degrees; there is no converter",
        "ln" | "log" | "log10" => "none in the table; use `exp` and `pow`, or a step",
        "atan2" | "atan" | "asin" | "acos" => "none in the table; use a step",
        "mod" | "fmod" => "write `a - b * floor(a / b)`",
        "round" => "write `floor(a + 0.5)`",
        "ceil" => "write `-floor(-a)`",
        "noise" | "perlin" | "simplex" => "write `fbm(wavelength, 1)`",
        _ => return None,
    })
}
