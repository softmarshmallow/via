//! The selection and value phases (`formulas.md` 1, 7 and 9): binding selectors, and evaluating
//! a program over a window of posts or a list of points.
//!
//! **Windows.** A value at a post depends only on the post's position (`numbers.md` 6): `x` and
//! `y` are the post's world coordinates, noise is hashed at them, and the one function that reads
//! neighbours, `slope`, reads its one-post halo from the field over the window grown by 1,
//! clipped to the grid, with the same edge rule whole or windowed. So a window's values are the
//! same cut of the whole, bit for bit.
//!
//! **Reads** of fields, of a feature's geometry and of attributes are callbacks into [`Reads`],
//! implemented by the model and the raster crates, so this crate stays free of the graph.
//!
//! **Explain mode** (`formulas.md` 9). The fast pass computes every operation's column and notes
//! which operations failed anywhere, live or not. Only when one did does the explain pass run:
//! it works out, top-down, at which posts each operation is live (the branch of an `if` not taken
//! is not), and reports the first post in post order at which a live operation fails, and at
//! that post the first such operation in evaluation order. That is the record whatever the
//! window, so the least record over all windows is the whole domain's.

use via_diag::codes::{diag, doc, formula};
use via_diag::Code;
use via_num::{clamp, cos_deg, max, metres, min, mix, sin_deg, tan_deg, Grid, NumError};

use crate::context::{KeyRange, NoisePart};
use crate::error::{num_text, At, ExprError};
use crate::parse::{Arith, Cmp};
use crate::program::{Measure, Node, Op, Program, Seed, Stat};
use crate::table::{Bound, Class};

/// A rectangle of posts, `i0 <= i < i1`, `j0 <= j < j1` (`numbers.md` 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub i0: u64,
    pub j0: u64,
    pub i1: u64,
    pub j1: u64,
}

impl Window {
    /// The whole grid.
    pub fn whole(grid: &Grid) -> Window {
        Window {
            i0: 0,
            j0: 0,
            i1: grid.nx,
            j1: grid.ny,
        }
    }

    fn width(&self) -> u64 {
        self.i1 - self.i0
    }

    fn grown(&self, grid: &Grid, by: u64) -> Window {
        Window {
            i0: self.i0.saturating_sub(by),
            j0: self.j0.saturating_sub(by),
            i1: (self.i1 + by).min(grid.nx),
            j1: (self.j1 + by).min(grid.ny),
        }
    }
}

/// Where a formula is evaluated: a window of the world's posts, or points (`formulas.md` 6.9).
#[derive(Clone, Copy, Debug)]
pub enum Domain<'a> {
    Window {
        grid: Grid,
        window: Window,
    },
    /// Lattice points in world millimetres.
    Points(&'a [(i64, i64)]),
}

impl Domain<'_> {
    /// How many posts or points.
    pub fn len(&self) -> usize {
        match self {
            Domain::Window { window, .. } => (window.width() * (window.j1 - window.j0)) as usize,
            Domain::Points(p) => p.len(),
        }
    }

    /// Whether there is none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `k`-th post (row by row from the south) or point, in world millimetres.
    pub fn point(&self, k: usize) -> (i64, i64) {
        match self {
            Domain::Window { grid, window } => {
                let (i, j) = self.ij(k, window);
                grid.post(i, j).unwrap_or((0, 0))
            }
            Domain::Points(p) => p[k],
        }
    }

    fn ij(&self, k: usize, window: &Window) -> (u64, u64) {
        let w = window.width();
        (window.i0 + k as u64 % w, window.j0 + k as u64 / w)
    }

    /// The `k`-th post's index in the whole grid's post order, or the point's index: the order in
    /// which failures are reported.
    pub fn order(&self, k: usize) -> u64 {
        match self {
            Domain::Window { grid, window } => {
                let (i, j) = self.ij(k, window);
                j * grid.nx + i
            }
            Domain::Points(_) => k as u64,
        }
    }

    /// Every post or point, in order.
    pub fn points(&self) -> Vec<(i64, i64)> {
        (0..self.len()).map(|k| self.point(k)).collect()
    }
}

/// A read that failed: the reader's code and words; the evaluator adds the place.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub code: Code,
    pub message: String,
    pub hint: String,
}

impl ReadError {
    pub fn new(code: Code, message: impl Into<String>, hint: impl Into<String>) -> ReadError {
        ReadError {
            code,
            message: message.into(),
            hint: hint.into(),
        }
    }

    fn missing(what: &str) -> ReadError {
        ReadError::new(
            diag::INTERNAL,
            format!("the reader cannot give {what}"),
            "report it: this is a defect of the engine",
        )
    }
}

/// An attribute's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scalar {
    Number(f64),
    YesNo(bool),
}

/// The values a formula reads, given by the model and raster crates. Each column has one value
/// per post or point of the domain, in its order.
pub trait Reads {
    /// An attribute of the feature served (`feature = None`) or of the feature `feature`.
    fn attribute(&self, feature: Option<&str>, name: &str) -> Result<Scalar, ReadError> {
        let _ = (feature, name);
        Err(ReadError::missing("attributes"))
    }
    /// A field's values, in the view the formula's key reads (`formulas.md` 6.6). `None` beyond
    /// the extent (`formula.outside_field`); at a point that is not a post, by `fields.md` 1.2.
    fn field(&self, field: &str, domain: &Domain) -> Result<Vec<Option<f64>>, ReadError> {
        let _ = (field, domain);
        Err(ReadError::missing("fields"))
    }
    /// `slope` at points that are not posts (`formulas.md` 6.9). At posts this crate computes
    /// it from [`Reads::field`].
    fn slope(&self, field: &str, points: &[(i64, i64)]) -> Result<Vec<Option<f64>>, ReadError> {
        let _ = (field, points);
        Err(ReadError::missing("slopes at points"))
    }
    /// `v`: the written field before this entry.
    fn v(&self, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let _ = domain;
        Err(ReadError::missing("`v`"))
    }
    /// `d`, `sd` or `t` of the feature served.
    fn local(&self, measure: Measure, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let _ = (measure, domain);
        Err(ReadError::missing("the feature's own measures"))
    }
    /// A measure of feature `feature` (`numbers.md` 5.5); `Measure::Inside` gives 1 or 0. `cap`
    /// bounds the geometry a distance needs: values beyond it may be any number at least `cap`.
    fn measure(
        &self,
        feature: &str,
        measure: Measure,
        domain: &Domain,
        cap: Option<f64>,
    ) -> Result<Vec<f64>, ReadError> {
        let _ = (feature, measure, domain, cap);
        Err(ReadError::missing("a feature's measures"))
    }
    /// `under` over the feature served (`formulas.md` 8.5): the entry's field (`None`) or a named
    /// field.
    fn under(&self, field: Option<&str>, stat: Stat) -> Result<f64, ReadError> {
        let _ = (field, stat);
        Err(ReadError::missing("`under`"))
    }
    /// `smoothed_along` of the feature served, at the domain's points.
    fn smoothed_along(&self, length: f64, domain: &Domain) -> Result<Vec<f64>, ReadError> {
        let _ = (length, domain);
        Err(ReadError::missing("`smoothed_along`"))
    }
}

/// A feature's shape, for the selection phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Path,
    Region { rings: u32 },
    Point,
}

/// A field a selector matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub weights: bool,
}

/// A feature a selector matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeInfo {
    pub id: String,
    pub shape: Shape,
}

/// What selectors resolve to at the reading node's stratum (`formulas.md` 7).
pub trait Select {
    /// The fields a field selector matches.
    fn fields(&self, selector: &str) -> Vec<FieldInfo>;
    /// The features a shape selector matches.
    fn shapes(&self, selector: &str) -> Vec<ShapeInfo>;
    /// The shape of the feature the formula serves, if any.
    fn own_shape(&self) -> Option<Shape> {
        None
    }
    /// Names of the class visible here, for the nearest-name hint.
    fn names(&self, class: Class) -> Vec<String> {
        let _ = class;
        Vec::new()
    }
}

/// What one selector resolved to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Field(String),
    /// Features, in id order.
    Shapes(Vec<String>),
    /// A seed or statistic: nothing to resolve.
    Literal,
}

/// A program's selectors, resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub targets: Vec<Target>,
    /// `formula.empty_selection` warnings, once per selector text.
    pub warnings: Vec<ExprError>,
}

fn sel_error(program: &Program, at: &At, code: Code, msg: String, hint: String) -> ExprError {
    let mut e = ExprError::new(code, at.clone(), msg, hint);
    e.subject = Some(program.formula);
    e
}

/// The selection phase: resolves every selector and checks counts, classes and shapes.
pub fn bind(program: &Program, select: &dyn Select) -> Result<Binding, ExprError> {
    let mut targets = Vec::with_capacity(program.selectors.len());
    let mut warnings = Vec::new();
    let mut warned: Vec<String> = Vec::new();
    for u in &program.selectors {
        let f = u.function;
        match u.class {
            Class::Seed | Class::Statistic => targets.push(Target::Literal),
            Class::Field => {
                let found = select.fields(&u.text);
                match found.as_slice() {
                    [] => {
                        let (code, msg) = if u.text.starts_with("tag:") {
                            (
                                formula::SELECTOR_COUNT,
                                format!("`'{}'` matches no field", u.text),
                            )
                        } else {
                            (formula::SELECTOR, format!("`'{}'` names no field", u.text))
                        };
                        let names = select.names(Class::Field);
                        let near =
                            via_diag::nearest::nearest(&u.text, names.iter().map(String::as_str));
                        return Err(sel_error(
                            program,
                            &u.at,
                            code,
                            msg,
                            near.hint("name a field of this world"),
                        ));
                    }
                    [one] => {
                        if one.weights {
                            return Err(sel_error(
                                program,
                                &u.at,
                                formula::TYPE,
                                format!(
                                    "`{f}` reads a scalar field; `{}` is a `weights` field",
                                    one.name
                                ),
                                "read a scalar field".into(),
                            ));
                        }
                        targets.push(Target::Field(one.name.clone()));
                    }
                    many => {
                        let names: Vec<String> =
                            many.iter().map(|x| format!("`{}`", x.name)).collect();
                        return Err(sel_error(
                            program,
                            &u.at,
                            formula::SELECTOR_COUNT,
                            format!(
                                "`'{}'` matches {} fields: {}; `{f}` reads one",
                                u.text,
                                many.len(),
                                names.join(", ")
                            ),
                            "name the field, or give the tag to one field only".into(),
                        ));
                    }
                }
            }
            Class::Shape => {
                let mut found = select.shapes(&u.text);
                found.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
                found.dedup_by(|a, b| a.id == b.id);
                if found.is_empty() {
                    if !u.text.starts_with("kind:") {
                        let names = select.names(Class::Shape);
                        let near =
                            via_diag::nearest::nearest(&u.text, names.iter().map(String::as_str));
                        return Err(sel_error(
                            program,
                            &u.at,
                            formula::SELECTOR,
                            format!("`'{}'` names nothing here", u.text),
                            near.hint("name a feature below this node"),
                        ));
                    }
                    if f == "along" {
                        return Err(sel_error(
                            program,
                            &u.at,
                            formula::SELECTOR_COUNT,
                            format!("`'{}'` holds no feature; `along` needs one", u.text),
                            "name one feature".into(),
                        ));
                    }
                    if !warned.contains(&u.text) {
                        warned.push(u.text.clone());
                        let used = match f {
                            "dist" => "its cap",
                            "sdist" => "minus its cap",
                            "near" => "0",
                            _ => "no",
                        };
                        warnings.push(sel_error(
                            program,
                            &u.at,
                            formula::EMPTY_SELECTION,
                            format!("`'{}'` holds no feature here; `{f}` gives {used}", u.text),
                            "add a feature of that kind below this node, or drop the term".into(),
                        ));
                    }
                }
                let shape_err = |msg: String| {
                    sel_error(
                        program,
                        &u.at,
                        formula::SHAPE,
                        msg,
                        "select features of a shape the function takes".into(),
                    )
                };
                let paths = found.iter().filter(|s| s.shape == Shape::Path).count();
                let points = found.iter().filter(|s| s.shape == Shape::Point).count();
                let regions = found.len() - paths - points;
                match f {
                    "inside" if paths + points > 0 => {
                        return Err(shape_err(format!(
                            "`inside` takes regions; `'{}'` holds a path or point",
                            u.text
                        )));
                    }
                    "sdist" => {
                        if [paths > 0, points > 0, regions > 0]
                            .iter()
                            .filter(|b| **b)
                            .count()
                            > 1
                        {
                            return Err(shape_err(format!(
                                "`sdist` over `'{}'` mixes regions, paths and points",
                                u.text
                            )));
                        }
                        if paths > 1 {
                            return Err(sel_error(
                                program,
                                &u.at,
                                formula::SELECTOR_COUNT,
                                format!("`sdist` takes one path; `'{}'` holds {paths}", u.text),
                                "name one path".into(),
                            ));
                        }
                    }
                    "along" => {
                        if found.len() > 1 {
                            let ids: Vec<String> =
                                found.iter().map(|s| format!("`{}`", s.id)).collect();
                            return Err(sel_error(
                                program,
                                &u.at,
                                formula::SELECTOR_COUNT,
                                format!(
                                    "`along` takes one feature; `'{}'` holds {}",
                                    u.text,
                                    ids.join(", ")
                                ),
                                "name one feature".into(),
                            ));
                        }
                        if let Some(ShapeInfo {
                            shape: Shape::Region { rings },
                            ..
                        }) = found.first()
                        {
                            if *rings > 1 {
                                return Err(shape_err(format!(
                                    "`along` is undefined on a region of {rings} rings"
                                )));
                            }
                        }
                    }
                    _ => {}
                }
                targets.push(Target::Shapes(found.into_iter().map(|s| s.id).collect()));
            }
        }
    }
    // `'edge_min'` (8.5): regions only, else `formula.shape`; on a region, milestone 2.
    if let Some(u) = program
        .selectors
        .iter()
        .find(|u| u.class == Class::Statistic && u.text == "edge_min")
    {
        return Err(match select.own_shape() {
            Some(Shape::Region { .. }) => sel_error(
                program,
                &u.at,
                doc::NOT_YET,
                "`'edge_min'` arrives in milestone 2".into(),
                "use `'min'` for now".into(),
            ),
            _ => sel_error(
                program,
                &u.at,
                formula::SHAPE,
                "`'edge_min'` takes a region; this feature is a path or a point".into(),
                "use `'min'` on a path or a point".into(),
            ),
        });
    }
    let smoothed = program
        .nodes
        .iter()
        .find(|n| matches!(n.op, Op::SmoothedAlong(_)));
    if let Some(n) = smoothed {
        if !matches!(select.own_shape(), Some(Shape::Path)) {
            return Err(sel_error(
                program,
                &n.at,
                formula::SHAPE,
                "`smoothed_along` takes a path; this feature is a region or a point".into(),
                "use it in a kind of paths".into(),
            ));
        }
    }
    Ok(Binding { targets, warnings })
}

/// What an evaluation needs besides the program and its reads.
#[derive(Clone, Copy, Debug)]
pub struct Setup<'a> {
    /// The world seed (`numbers.md` 7.1).
    pub world_seed: u64,
    /// The feature served, for default noise streams of a kind's formulas.
    pub feature: Option<&'a str>,
}

/// A failed evaluation: the error, and its order (post order, then evaluation order) so that the
/// least over many windows is the whole domain's record. `None` for a fault with no post.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub order: Option<(u64, usize)>,
    pub error: ExprError,
}

#[derive(Clone, Debug)]
enum Col {
    U(f64),
    V(Vec<f64>),
}

impl Col {
    fn get(&self, k: usize) -> f64 {
        match self {
            Col::U(v) => *v,
            Col::V(v) => v[k],
        }
    }

    fn uniform(&self) -> Option<f64> {
        match self {
            Col::U(v) => Some(*v),
            Col::V(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Fail {
    NotFinite,
    Range(String, String),
    Outside,
    Read(ReadError),
}

#[derive(Clone, Debug, Default)]
struct Fails {
    /// Fails at every post.
    all: Option<Fail>,
    /// Fails at these posts.
    at: Vec<(usize, Fail)>,
}

impl Fails {
    fn is_empty(&self) -> bool {
        self.all.is_none() && self.at.is_empty()
    }
}

fn map1(a: &Col, _n: usize, f: impl Fn(f64) -> f64) -> Col {
    match a {
        Col::U(x) => Col::U(f(*x)),
        Col::V(v) => Col::V(v.iter().map(|&x| f(x)).collect()),
    }
}

fn map2(a: &Col, b: &Col, n: usize, f: impl Fn(f64, f64) -> f64) -> Col {
    match (a, b) {
        (Col::U(x), Col::U(y)) => Col::U(f(*x, *y)),
        _ => Col::V((0..n).map(|k| f(a.get(k), b.get(k))).collect()),
    }
}

fn map3(a: &Col, b: &Col, c: &Col, n: usize, f: impl Fn(f64, f64, f64) -> f64) -> Col {
    match (a, b, c) {
        (Col::U(x), Col::U(y), Col::U(z)) => Col::U(f(*x, *y, *z)),
        _ => Col::V((0..n).map(|k| f(a.get(k), b.get(k), c.get(k))).collect()),
    }
}

fn yes(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// `+ - * /`, correctly rounded, never fused.
pub(crate) fn arith(op: Arith, a: f64, b: f64) -> f64 {
    match op {
        Arith::Add => a + b,
        Arith::Sub => a - b,
        Arith::Mul => a * b,
        Arith::Div => a / b,
    }
}

fn compare(op: Cmp, a: f64, b: f64) -> bool {
    match op {
        Cmp::Lt => a < b,
        Cmp::Le => a <= b,
        Cmp::Gt => a > b,
        Cmp::Ge => a >= b,
        Cmp::Eq => a == b,
        Cmp::Ne => a != b,
    }
}

/// GLSL `smoothstep`, `s * s * (3 - 2 * s)` with `s = clamp((a - e0) / (e1 - e0), 0, 1)`.
pub(crate) fn smoothstep(e0: f64, e1: f64, a: f64) -> f64 {
    let s = clamp((a - e0) / (e1 - e0), 0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

/// The rule a parameter value breaks, if any (`formulas.md` 8.2, 8.4, 8.5).
pub(crate) fn bound_breach(bound: Bound, v: f64) -> Option<&'static str> {
    let ok = match bound {
        Bound::None => true,
        Bound::Positive => v > 0.0,
        Bound::Octaves => (1.0..=12.0).contains(&v) && v.floor() == v,
        Bound::Gain => v > 0.0 && v <= 1.0,
        Bound::AboveOne => v > 1.0,
    };
    if ok {
        return None;
    }
    Some(match bound {
        Bound::None => "",
        Bound::Positive => "above 0",
        Bound::Octaves => "a whole number from 1 to 12",
        Bound::Gain => "above 0 and at most 1",
        Bound::AboveOne => "above 1",
    })
}

/// The value of a node that reads nothing (literals, `pi` and arithmetic), if it is one. One
/// pass over the nodes up to `i` (operands come first), so no recursion.
pub(crate) fn fold_nodes(nodes: &[Node], i: usize) -> Option<f64> {
    let mut vals: Vec<Option<f64>> = Vec::with_capacity(i + 1);
    for node in nodes.iter().take(i + 1) {
        let f = |j: usize| vals.get(j).copied().flatten();
        let v = (|| {
            Some(match &node.op {
                Op::Const(v) => *v,
                Op::Neg(a) => -f(*a)?,
                Op::Arith(op, a, b) => arith(*op, f(*a)?, f(*b)?),
                Op::Abs(a) => f(*a)?.abs(),
                Op::Sqrt(a) => f(*a)?.sqrt(),
                Op::Pow(a, b) => via_num::pow(f(*a)?, f(*b)?),
                Op::Exp(a) => via_num::exp(f(*a)?),
                Op::Floor(a) => f(*a)?.floor(),
                Op::Sin(a) => sin_deg(f(*a)?),
                Op::Cos(a) => cos_deg(f(*a)?),
                Op::Tan(a) => tan_deg(f(*a)?),
                Op::Min(a, b) => min(f(*a)?, f(*b)?),
                Op::Max(a, b) => max(f(*a)?, f(*b)?),
                Op::Clamp(a, b, c) => clamp(f(*a)?, f(*b)?, f(*c)?),
                Op::Mix(a, b, c) => mix(f(*a)?, f(*b)?, f(*c)?),
                Op::Smoothstep(a, b, c) => smoothstep(f(*a)?, f(*b)?, f(*c)?),
                _ => return None,
            })
        })();
        vals.push(v);
    }
    vals.get(i).copied().flatten()
}

pub(crate) fn fold(program: &Program, i: usize) -> Option<f64> {
    fold_nodes(&program.nodes, i)
}

fn internal(program: &Program, msg: String) -> Box<Failure> {
    let mut e = ExprError::new(
        diag::INTERNAL,
        At::Key {
            formula: program.formula,
        },
        msg,
        "report it: this is a defect of the engine, not of the text",
    );
    e.subject = Some(program.formula);
    Box::new(Failure {
        order: None,
        error: e,
    })
}

/// Evaluates a bound program over a domain, giving one double per post or point. A failure is
/// the first post in post order at which a live operation fails (`formulas.md` 9).
pub fn evaluate(
    program: &Program,
    binding: &Binding,
    reads: &dyn Reads,
    setup: &Setup,
    domain: &Domain,
) -> Result<Vec<f64>, Box<Failure>> {
    if let Domain::Window { grid, window } = domain {
        if !(window.i0 < window.i1
            && window.j0 < window.j1
            && window.i1 <= grid.nx
            && window.j1 <= grid.ny)
        {
            return Err(internal(
                program,
                format!("window {window:?} is empty or outside the grid"),
            ));
        }
    }
    let n = domain.len();
    let mut ev = Evaluator {
        program,
        binding,
        reads,
        setup,
        domain,
        n,
        cols: Vec::with_capacity(program.nodes.len()),
        fails: Vec::with_capacity(program.nodes.len()),
        points: None,
    };
    for i in 0..program.nodes.len() {
        let (col, fails) = ev.node(i)?;
        let fails = if fails.is_empty() {
            ev.not_finite(&col)
        } else {
            fails
        };
        ev.cols.push(col);
        ev.fails.push(fails);
    }
    let root = &ev.cols[program.root];
    let range = program.key.range();
    let mut key_fails = Vec::new();
    if range != KeyRange::None {
        for k in 0..n {
            let v = root.get(k);
            let ok = match range {
                KeyRange::Unit => (0.0..=1.0).contains(&v),
                KeyRange::NonNegative => v >= 0.0,
                KeyRange::None => true,
            };
            if !ok {
                key_fails.push(k);
                break;
            }
        }
    }
    if !key_fails.is_empty() || !ev.fails.iter().all(Fails::is_empty) {
        // Some operation failed somewhere; only a live failure is one (formulas.md 9).
        if let Some(f) = ev.explain(key_fails.first().copied()) {
            return Err(Box::new(f));
        }
    }
    Ok((0..n).map(|k| ev.cols[program.root].get(k)).collect())
}

/// Narrows a column for a field file (`numbers.md` 8, rule 9): finite, `-0` as `+0`, rounded
/// once to `f32`. The index of the first value that cannot be stored on failure.
pub fn narrow(values: &[f64]) -> Result<Vec<f32>, Box<(usize, NumError)>> {
    values
        .iter()
        .enumerate()
        .map(|(k, &v)| via_num::store_f32(v).map_err(|e| Box::new((k, e))))
        .collect()
}

struct Evaluator<'a> {
    program: &'a Program,
    binding: &'a Binding,
    reads: &'a dyn Reads,
    setup: &'a Setup<'a>,
    domain: &'a Domain<'a>,
    n: usize,
    cols: Vec<Col>,
    fails: Vec<Fails>,
    points: Option<Vec<(i64, i64)>>,
}

impl Evaluator<'_> {
    fn col(&self, i: usize) -> &Col {
        &self.cols[i]
    }

    fn not_finite(&self, col: &Col) -> Fails {
        match col {
            Col::U(v) if !v.is_finite() => Fails {
                all: Some(Fail::NotFinite),
                at: Vec::new(),
            },
            Col::U(_) => Fails::default(),
            Col::V(v) => Fails {
                all: None,
                at: v
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| !x.is_finite())
                    .map(|(k, _)| (k, Fail::NotFinite))
                    .collect(),
            },
        }
    }

    fn read_failed(&self, i: usize, e: ReadError) -> Box<Failure> {
        let node = &self.program.nodes[i];
        let mut err = ExprError::new(e.code, node.at.clone(), e.message, e.hint);
        err.subject = Some(self.program.formula);
        Box::new(Failure {
            order: None,
            error: err,
        })
    }

    fn pts(&mut self) -> Vec<(i64, i64)> {
        if self.points.is_none() {
            self.points = Some(self.domain.points());
        }
        self.points.clone().unwrap_or_default()
    }

    fn target(&self, sel: usize) -> &Target {
        self.binding.targets.get(sel).unwrap_or(&Target::Literal)
    }

    fn uniform_or_max(&self, c: &Col) -> f64 {
        match c {
            Col::U(v) => *v,
            Col::V(v) => v.iter().copied().fold(f64::NEG_INFINITY, max),
        }
    }

    /// Range failures of a parameter column at every post.
    fn bound_fails(&self, c: &Col, bound: Bound, what: &str) -> Fails {
        let mk = |v: f64| {
            bound_breach(bound, v).map(|rule| {
                Fail::Range(
                    format!("{what} is {}", num_text(v)),
                    format!("give it a value {rule}"),
                )
            })
        };
        match c {
            Col::U(v) => Fails {
                all: mk(*v),
                at: Vec::new(),
            },
            Col::V(v) => Fails {
                all: None,
                at: v
                    .iter()
                    .enumerate()
                    .filter_map(|(k, &x)| mk(x).map(|f| (k, f)))
                    .collect(),
            },
        }
    }

    fn node(&mut self, i: usize) -> Result<(Col, Fails), Box<Failure>> {
        let n = self.n;
        let op = self.program.nodes[i].op.clone();
        let ok = |c: Col| Ok((c, Fails::default()));
        match op {
            Op::Const(v) => ok(Col::U(v)),
            Op::X | Op::Y => {
                let pts = self.pts();
                let x = matches!(op, Op::X);
                ok(Col::V(
                    pts.iter()
                        .map(|&(px, py)| metres(if x { px } else { py }))
                        .collect(),
                ))
            }
            Op::V => {
                let v = self
                    .reads
                    .v(self.domain)
                    .map_err(|e| self.read_failed(i, e))?;
                self.column(i, v)
            }
            Op::Local(m) => {
                let v = self
                    .reads
                    .local(m, self.domain)
                    .map_err(|e| self.read_failed(i, e))?;
                self.column(i, v)
            }
            Op::Attr(ref name) | Op::Member(_, ref name) => {
                let feature = match &op {
                    Op::Member(id, _) => Some(id.as_str()),
                    _ => None,
                };
                match self.reads.attribute(feature, name) {
                    Ok(Scalar::Number(v)) => ok(Col::U(v)),
                    Ok(Scalar::YesNo(b)) => ok(Col::U(yes(b))),
                    Err(e) => Ok((
                        Col::U(0.0),
                        Fails {
                            all: Some(Fail::Read(e)),
                            at: Vec::new(),
                        },
                    )),
                }
            }
            Op::Neg(a) => ok(map1(self.col(a), n, |x| -x)),
            Op::Not(a) => ok(map1(self.col(a), n, |x| yes(x == 0.0))),
            Op::Arith(o, a, b) => ok(map2(self.col(a), self.col(b), n, |x, y| arith(o, x, y))),
            Op::Cmp(o, a, b) => ok(map2(self.col(a), self.col(b), n, |x, y| {
                yes(compare(o, x, y))
            })),
            Op::And(a, b) => ok(map2(self.col(a), self.col(b), n, |x, y| {
                yes(x != 0.0 && y != 0.0)
            })),
            Op::Or(a, b) => ok(map2(self.col(a), self.col(b), n, |x, y| {
                yes(x != 0.0 || y != 0.0)
            })),
            Op::Abs(a) => ok(map1(self.col(a), n, f64::abs)),
            Op::Sqrt(a) => ok(map1(self.col(a), n, f64::sqrt)),
            Op::Exp(a) => ok(map1(self.col(a), n, via_num::exp)),
            Op::Floor(a) => ok(map1(self.col(a), n, f64::floor)),
            Op::Sin(a) => ok(map1(self.col(a), n, sin_deg)),
            Op::Cos(a) => ok(map1(self.col(a), n, cos_deg)),
            Op::Tan(a) => ok(map1(self.col(a), n, tan_deg)),
            Op::Pow(a, b) => ok(map2(self.col(a), self.col(b), n, via_num::pow)),
            Op::Min(a, b) => ok(map2(self.col(a), self.col(b), n, min)),
            Op::Max(a, b) => ok(map2(self.col(a), self.col(b), n, max)),
            Op::Mix(a, b, c) => ok(map3(self.col(a), self.col(b), self.col(c), n, mix)),
            Op::Clamp(a, lo, hi) | Op::Smoothstep(lo, hi, a) => {
                let is_clamp = matches!(op, Op::Clamp(..));
                let col = if is_clamp {
                    map3(self.col(a), self.col(lo), self.col(hi), n, clamp)
                } else {
                    map3(self.col(lo), self.col(hi), self.col(a), n, smoothstep)
                };
                let (l, h) = (self.col(lo).clone(), self.col(hi).clone());
                let bad = |k: usize| {
                    let (x, y) = (l.get(k), h.get(k));
                    let breach = if is_clamp { x > y } else { x >= y };
                    breach.then(|| {
                        if is_clamp {
                            Fail::Range(
                                format!(
                                    "`clamp`'s `lo` ({}) is above its `hi` ({})",
                                    num_text(x),
                                    num_text(y)
                                ),
                                "make `lo` at most `hi`".into(),
                            )
                        } else {
                            Fail::Range(
                                format!(
                                    "`smoothstep`'s `e0` ({}) is not below its `e1` ({})",
                                    num_text(x),
                                    num_text(y)
                                ),
                                "make `e0` below `e1`".into(),
                            )
                        }
                    })
                };
                let fails = match (l.uniform(), h.uniform()) {
                    (Some(_), Some(_)) => Fails {
                        all: bad(0),
                        at: Vec::new(),
                    },
                    _ => Fails {
                        all: None,
                        at: (0..n).filter_map(|k| bad(k).map(|f| (k, f))).collect(),
                    },
                };
                let fails = if fails.is_empty() {
                    self.not_finite(&col)
                } else {
                    fails
                };
                Ok((col, fails))
            }
            Op::If(c, a, b) => ok(map3(self.col(c), self.col(a), self.col(b), n, |c, a, b| {
                if c != 0.0 {
                    a
                } else {
                    b
                }
            })),
            Op::Fbm {
                wavelength,
                octaves,
                gain,
                lacunarity,
                ref seed,
                ridged,
            } => {
                let noise = if ridged {
                    crate::noise::ridged
                } else {
                    crate::noise::fbm
                };
                let stream = self.stream(seed);
                let (w, o, g, l) = (
                    self.col(wavelength).clone(),
                    self.col(octaves).clone(),
                    self.col(gain).clone(),
                    self.col(lacunarity).clone(),
                );
                let pts = self.pts();
                let mut fails = Fails::default();
                let mut out = Vec::with_capacity(n);
                for (k, &(px, py)) in pts.iter().enumerate() {
                    let params = [
                        (&w, Bound::Positive, "`wavelength`"),
                        (&o, Bound::Octaves, "`octaves`"),
                        (&g, Bound::Gain, "`gain`"),
                        (&l, Bound::AboveOne, "`lacunarity`"),
                    ];
                    let breach = params.iter().find_map(|(c, b, what)| {
                        let v = c.get(k);
                        bound_breach(*b, v).map(|rule| {
                            Fail::Range(
                                format!("{what} of `fbm` is {}", num_text(v)),
                                format!("give it a value {rule}"),
                            )
                        })
                    });
                    if let Some(f) = breach {
                        fails.at.push((k, f));
                        out.push(0.0);
                        continue;
                    }
                    match noise(
                        stream,
                        metres(px),
                        metres(py),
                        w.get(k),
                        o.get(k) as u32,
                        g.get(k),
                        l.get(k),
                    ) {
                        Ok(v) => out.push(v),
                        Err(_) => {
                            fails.at.push((
                                k,
                                Fail::Range(
                                    "`wavelength` of `fbm` puts the point beyond the noise lattice"
                                        .into(),
                                    "use a longer wavelength".into(),
                                ),
                            ));
                            out.push(0.0);
                        }
                    }
                }
                Ok((Col::V(out), fails))
            }
            Op::Field(sel) => {
                let Target::Field(name) = self.target(sel).clone() else {
                    return Err(internal(
                        self.program,
                        "a field selector was not bound".into(),
                    ));
                };
                let v = self
                    .reads
                    .field(&name, self.domain)
                    .map_err(|e| self.read_failed(i, e))?;
                self.optional(i, v)
            }
            Op::Slope(sel) => {
                let Target::Field(name) = self.target(sel).clone() else {
                    return Err(internal(
                        self.program,
                        "a field selector was not bound".into(),
                    ));
                };
                let v = match self.domain {
                    Domain::Window { grid, window } => self.slope_window(i, &name, grid, window)?,
                    Domain::Points(p) => self
                        .reads
                        .slope(&name, p)
                        .map_err(|e| self.read_failed(i, e))?,
                };
                self.optional(i, v)
            }
            Op::Dist { shape, cap }
            | Op::Sdist { shape, cap }
            | Op::Near { shape, radius: cap } => {
                let capc = self.col(cap).clone();
                let what = if matches!(op, Op::Near { .. }) {
                    "`radius`"
                } else {
                    "`cap`"
                };
                let fails = self.bound_fails(&capc, Bound::Positive, what);
                if !fails.is_empty() {
                    return Ok((Col::U(0.0), fails));
                }
                let signed = matches!(op, Op::Sdist { .. });
                let m = if signed { Measure::Sd } else { Measure::D };
                let acc =
                    self.fold_measures(i, shape, m, Some(self.uniform_or_max(&capc)), signed)?;
                let near = matches!(op, Op::Near { .. });
                let col = match (&op, acc) {
                    (Op::Sdist { .. }, None) => map1(&capc, n, |c| -c),
                    (_, None) => capc.clone(),
                    (Op::Sdist { .. }, Some(a)) => map2(&a, &capc, n, |s, c| clamp(s, -c, c)),
                    (_, Some(a)) => map2(&a, &capc, n, min),
                };
                let col = if near {
                    map2(&col, &capc, n, |d, r| 1.0 - smoothstep(0.0, r, d))
                } else {
                    col
                };
                Ok((col, Fails::default()))
            }
            Op::Inside(shape) => {
                let acc = self.fold_measures(i, shape, Measure::Inside, None, true)?;
                ok(acc
                    .map(|a| map1(&a, n, |x| yes(x > 0.0)))
                    .unwrap_or(Col::U(0.0)))
            }
            Op::Along(shape) => {
                let acc = self.fold_measures(i, shape, Measure::T, None, false)?;
                ok(acc.unwrap_or(Col::U(0.0)))
            }
            Op::Under { field, stat } => {
                let name = match field {
                    Some(s) => match self.target(s) {
                        Target::Field(f) => Some(f.clone()),
                        _ => {
                            return Err(internal(
                                self.program,
                                "a field selector was not bound".into(),
                            ))
                        }
                    },
                    None => None,
                };
                match self.reads.under(name.as_deref(), stat) {
                    Ok(v) => ok(Col::U(v)),
                    Err(e) => Ok((
                        Col::U(0.0),
                        Fails {
                            all: Some(Fail::Read(e)),
                            at: Vec::new(),
                        },
                    )),
                }
            }
            Op::SmoothedAlong(len) => {
                let l = self.col(len).clone();
                let fails = self.bound_fails(&l, Bound::Positive, "`length`");
                if !fails.is_empty() {
                    return Ok((Col::U(0.0), fails));
                }
                let v = self
                    .reads
                    .smoothed_along(self.uniform_or_max(&l), self.domain)
                    .map_err(|e| self.read_failed(i, e))?;
                self.column(i, v)
            }
        }
    }

    fn stream(&self, seed: &Seed) -> u64 {
        match seed {
            Seed::Named(name) => {
                via_num::stream(self.setup.world_seed, &["noise", "seed", name.as_str()])
            }
            Seed::Default(parts) => {
                let parts: Vec<String> = parts
                    .iter()
                    .map(|p| match p {
                        NoisePart::Text(t) => t.clone(),
                        NoisePart::Feature => self.setup.feature.unwrap_or("").to_string(),
                    })
                    .collect();
                via_num::stream(self.setup.world_seed, &parts)
            }
        }
    }

    fn column(&self, i: usize, v: Vec<f64>) -> Result<(Col, Fails), Box<Failure>> {
        if v.len() != self.n {
            return Err(internal(
                self.program,
                format!(
                    "a read for `{}` gave {} values for {} points",
                    self.program.nodes[i].text,
                    v.len(),
                    self.n
                ),
            ));
        }
        Ok((Col::V(v), Fails::default()))
    }

    fn optional(&self, i: usize, v: Vec<Option<f64>>) -> Result<(Col, Fails), Box<Failure>> {
        if v.len() != self.n {
            return Err(internal(
                self.program,
                format!(
                    "a read for `{}` gave {} values for {} points",
                    self.program.nodes[i].text,
                    v.len(),
                    self.n
                ),
            ));
        }
        let mut fails = Fails::default();
        let col = v
            .into_iter()
            .enumerate()
            .map(|(k, x)| {
                x.unwrap_or_else(|| {
                    fails.at.push((k, Fail::Outside));
                    0.0
                })
            })
            .collect();
        Ok((Col::V(col), fails))
    }

    /// Folds a measure over the bound features in id order (`numbers.md` 8, rule 4): `min` for
    /// a distance, `max` for a signed distance or membership. `None` when there are none.
    fn fold_measures(
        &self,
        i: usize,
        sel: usize,
        m: Measure,
        cap: Option<f64>,
        greatest: bool,
    ) -> Result<Option<Col>, Box<Failure>> {
        let Target::Shapes(ids) = self.target(sel) else {
            return Err(internal(
                self.program,
                "a shape selector was not bound".into(),
            ));
        };
        let mut acc: Option<Col> = None;
        for id in ids {
            let v = self
                .reads
                .measure(id, m, self.domain, cap)
                .map_err(|e| self.read_failed(i, e))?;
            let (c, _) = self.column(i, v)?;
            acc = Some(match acc {
                None => c,
                Some(a) => map2(&a, &c, self.n, if greatest { max } else { min }),
            });
        }
        Ok(acc)
    }

    /// `slope` at the posts of a window (`formulas.md` 8.3): central differences, one-sided
    /// where a neighbour is beyond the extent, read from the window grown by its one-post halo.
    fn slope_window(
        &self,
        i: usize,
        name: &str,
        grid: &Grid,
        window: &Window,
    ) -> Result<Vec<Option<f64>>, Box<Failure>> {
        let g = window.grown(grid, 1);
        let gd = Domain::Window {
            grid: *grid,
            window: g,
        };
        let h = self
            .reads
            .field(name, &gd)
            .map_err(|e| self.read_failed(i, e))?;
        if h.len() != gd.len() {
            return Err(internal(
                self.program,
                format!(
                    "a field read for `slope` gave {} values for {} posts",
                    h.len(),
                    gd.len()
                ),
            ));
        }
        let gw = g.width();
        let at = |ii: u64, jj: u64| h[((jj - g.j0) * gw + (ii - g.i0)) as usize];
        let r = metres(grid.resolution);
        let diff = |lo: Option<f64>, mid: Option<f64>, hi: Option<f64>| -> Option<f64> {
            match (lo, hi) {
                (Some(a), Some(b)) => Some((b - a) / (2.0 * r)),
                (None, Some(b)) => Some((b - mid?) / r),
                (Some(a), None) => Some((mid? - a) / r),
                (None, None) => None,
            }
        };
        let mut out = Vec::with_capacity(self.n);
        for j in window.j0..window.j1 {
            for ii in window.i0..window.i1 {
                let mid = at(ii, j);
                let w = (ii > 0).then(|| at(ii - 1, j)).flatten();
                let e = (ii + 1 < grid.nx).then(|| at(ii + 1, j)).flatten();
                let s = (j > 0).then(|| at(ii, j - 1)).flatten();
                let nn = (j + 1 < grid.ny).then(|| at(ii, j + 1)).flatten();
                let v = match (diff(w, mid, e), diff(s, mid, nn)) {
                    (Some(gx), Some(gy)) => {
                        Some(via_num::atan((gx * gx + gy * gy).sqrt()) * via_num::DEGREES_PER_RAD)
                    }
                    _ => None,
                };
                out.push(v);
            }
        }
        Ok(out)
    }

    /// The explain pass: liveness top-down, then the least (post order, evaluation order).
    fn explain(&self, key_fail: Option<usize>) -> Option<Failure> {
        let nodes = &self.program.nodes;
        let n = self.n;
        let mut live: Vec<Option<Vec<bool>>> = vec![None; nodes.len()];
        live[self.program.root] = Some(vec![true; n]);
        for i in (0..nodes.len()).rev() {
            let Some(m) = live[i].clone() else { continue };
            let mut give = |j: usize, f: &dyn Fn(usize) -> bool| {
                let slot = live[j].get_or_insert_with(|| vec![false; n]);
                for (k, s) in slot.iter_mut().enumerate() {
                    if m[k] && f(k) {
                        *s = true;
                    }
                }
            };
            match nodes[i].op {
                Op::If(c, a, b) => {
                    let cc = self.cols[c].clone();
                    give(c, &|_| true);
                    give(a, &|k| cc.get(k) != 0.0);
                    give(b, &|k| cc.get(k) == 0.0);
                }
                ref op => {
                    for j in op.operands() {
                        give(j, &|_| true);
                    }
                }
            }
        }
        let mut best: Option<(u64, usize, usize, Fail)> = None;
        let mut consider = |order: u64, seq: usize, k: usize, f: &Fail| {
            if best
                .as_ref()
                .is_none_or(|(o, s, _, _)| (order, seq) < (*o, *s))
            {
                best = Some((order, seq, k, f.clone()));
            }
        };
        for (i, fails) in self.fails.iter().enumerate() {
            let Some(m) = &live[i] else { continue };
            if let Some(f) = &fails.all {
                if let Some(k) = m.iter().position(|&b| b) {
                    consider(self.domain.order(k), i, k, f);
                }
            }
            for (k, f) in &fails.at {
                if m[*k] {
                    consider(self.domain.order(*k), i, *k, f);
                }
            }
        }
        if let Some(k) = key_fail {
            let v = self.cols[self.program.root].get(k);
            let (rule, hint) = match self.program.key.range() {
                KeyRange::NonNegative => ("0 or more", "write `max(..., 0)`"),
                _ => ("0 to 1", "write `clamp(..., 0, 1)`"),
            };
            let f = Fail::Range(
                format!(
                    "{} is {}; it must be {rule}",
                    self.program.key.words(),
                    num_text(v)
                ),
                hint.into(),
            );
            consider(self.domain.order(k), nodes.len(), k, &f);
        }
        // None when every failure noted lies where nothing was live.
        let (order, seq, k, fail) = best?;
        let (x, y) = self.domain.point(k);
        let node = nodes.get(seq).unwrap_or(&nodes[self.program.root]);
        let at = node.at.clone();
        let place = format!(
            "at ({}, {})",
            via_num::write_length(x),
            via_num::write_length(y)
        );
        let mut err = match fail {
            Fail::NotFinite => {
                let operands: Vec<String> = node
                    .op
                    .operands()
                    .iter()
                    .map(|&j| num_text(self.cols[j].get(k)))
                    .collect();
                let ops = if operands.is_empty() {
                    String::new()
                } else {
                    format!(" of {}", operands.join(", "))
                };
                ExprError::new(
                    formula::NOT_FINITE,
                    at,
                    format!(
                        "`{}` gave {}{ops} in `{}` {place}",
                        node.name,
                        num_text(self.cols[seq].get(k)),
                        node.text
                    ),
                    guard(&node.name),
                )
            }
            Fail::Range(msg, hint) => {
                ExprError::new(formula::RANGE, at, format!("{msg} {place}"), hint)
            }
            Fail::Outside => ExprError::new(
                formula::OUTSIDE_FIELD,
                at,
                format!("`{}` reads a field {place}, beyond its extent", node.text),
                "keep the points inside the extent, or guard the read with `if`",
            ),
            Fail::Read(e) => ExprError::new(e.code, at, e.message, e.hint),
        };
        err.subject = Some(self.program.formula);
        err.position = Some((x, y));
        if seq < nodes.len() {
            if let Some((note, reader)) = &node.via_let {
                err.related.push((reader.clone(), note.clone()));
            }
        }
        Some(Failure {
            order: Some((order, seq)),
            error: err,
        })
    }
}

/// The hint of `formula.not_finite`: a guard for the operation.
fn guard(op: &str) -> String {
    match op {
        "sqrt" => "guard it: `sqrt(max(a, 0))`",
        "/" => "guard the divisor: `if(b != 0, a / b, 0)`",
        "exp" => "keep the argument below about 709: `exp(min(a, 700))`",
        "pow" => "keep the result finite: `pow(max(a, 0), b)` with a bounded `b`",
        "tan" => "keep the angle away from 90 + 180k degrees",
        _ => "guard the operation so its result stays finite",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_is_exact_at_its_ends() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
    }

    #[test]
    fn bounds() {
        assert!(bound_breach(Bound::Octaves, 3.5).is_some());
        assert!(bound_breach(Bound::Octaves, 13.0).is_some());
        assert!(bound_breach(Bound::Octaves, 12.0).is_none());
        assert!(bound_breach(Bound::Gain, 0.0).is_some());
        assert!(bound_breach(Bound::Gain, 1.0).is_none());
        assert!(bound_breach(Bound::AboveOne, 1.0).is_some());
        assert!(bound_breach(Bound::Positive, f64::NAN).is_some());
    }
}
