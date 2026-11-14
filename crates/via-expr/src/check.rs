//! The text phase (`formulas.md` 1): names by context, types, rates, arguments, selector form
//! and class, seeds, caps and constant parameter ranges, compiling a parsed formula into a
//! [`Program`].

use std::collections::BTreeMap;
use std::ops::Range;

use via_diag::codes::{doc, formula, graph, identity};
use via_diag::nearest::{nearest, Nearest};

use crate::context::{Builtin, Context, Key, NoisePart, Results};
use crate::env::{AttrType, Env, Source};
use crate::error::{At, ExprError, FormulaId, Scope};
use crate::parse::{Arg, Expr, Kind};
use crate::program::{Measure, Node, Op, Program, ReadSet, Seed, SelectorUse, Stat, Type};
use crate::table::{self, Bound, Class, Default, Fun, Function, ParamType, Rate, Where};

/// The `let`s and formula being compiled, innermost last.
#[derive(Clone)]
struct Frame {
    formula: FormulaId,
    scope: Scope,
    /// The formula's own frame sees the feature's own attributes (6.6, 6.7); a `let` does not.
    own: bool,
    noise: Vec<NoisePart>,
    via_let: Option<(String, At)>,
}

/// A compiled sub-expression: a node, or a selector literal waiting for its parameter.
enum Val {
    Node(usize),
    Sel(String, Range<u32>),
}

struct Compiler<'a> {
    env: &'a Env,
    ctx: &'a Context,
    nodes: Vec<Node>,
    selectors: Vec<SelectorUse>,
    noise: Vec<(Seed, At)>,
    reads: ReadSet,
    halo: u32,
    lets: BTreeMap<(Scope, String), usize>,
    stack: Vec<(Scope, String)>,
}

pub(crate) fn compile(env: &Env, id: FormulaId, ctx: &Context) -> Result<Program, ExprError> {
    if ctx.key.milestone() > 1 {
        return Err(ExprError::new(
            doc::NOT_YET,
            At::Key { formula: id },
            format!(
                "{} arrives in milestone {}",
                ctx.key.words(),
                ctx.key.milestone()
            ),
            "remove it for now",
        ));
    }
    let scope = if let Some(k) = &ctx.kind {
        Scope::Kind(k.clone())
    } else if let Some(f) = &ctx.field {
        Scope::Field(f.clone())
    } else {
        Scope::World
    };
    let mut c = Compiler {
        env,
        ctx,
        nodes: Vec::new(),
        selectors: Vec::new(),
        noise: Vec::new(),
        reads: ReadSet::default(),
        halo: 0,
        lets: BTreeMap::new(),
        stack: Vec::new(),
    };
    let frame = Frame {
        formula: id,
        scope,
        own: true,
        noise: ctx.noise.clone(),
        via_let: None,
    };
    let (root, span) = c.body(&frame)?;
    let node = &c.nodes[root];
    let at = node.at.clone();
    if (ctx.key.results(), node.ty) == (Results::Number, Type::YesNo) {
        return Err(ExprError::new(
            formula::TYPE,
            at,
            format!(
                "{} gives a number; this formula gives a yes/no",
                ctx.key.words()
            ),
            "write a number, for example `if(c, 1, 0)`",
        ));
    }
    if node.rate > ctx.key.max_rate() {
        let from = &c.nodes[node.rate_from];
        return Err(ExprError::new(
            formula::RATE,
            from.at.clone(),
            format!(
                "`{}` is {}; {} is worked out once {}",
                from.text,
                from.rate.word(),
                ctx.key.words(),
                match ctx.key.max_rate() {
                    Rate::PerFeature => "per feature",
                    Rate::Constant => "per build",
                    Rate::PerPost => "per post",
                }
            ),
            "read it where the formula is per post, or use a per-feature value",
        ));
    }
    Ok(Program {
        formula: id,
        key: ctx.key,
        nodes: c.nodes,
        root,
        selectors: c.selectors,
        noise: c.noise,
        reads: c.reads,
        halo_posts: c.halo,
        span,
    })
}

fn sel_err(code: via_diag::Code, at: At, msg: String, hint: String) -> ExprError {
    ExprError::new(code, at, msg, hint)
}

impl Compiler<'_> {
    /// Compiles a registered formula in a frame: the root, or a `let`'s value.
    fn body(&mut self, frame: &Frame) -> Result<(usize, Range<u32>), ExprError> {
        let env = self.env;
        match env.source(frame.formula) {
            Some(Source::Number(v)) => {
                let i = self.push(
                    Op::Const(*v),
                    Type::Number,
                    Rate::Constant,
                    None,
                    At::Key {
                        formula: frame.formula,
                    },
                    crate::error::num_text(*v),
                    "number".into(),
                    frame,
                );
                Ok((i, 0..0))
            }
            Some(Source::Text { parsed, .. }) => {
                let expr = parsed.as_ref().map_err(Clone::clone)?;
                let span = expr.span.clone();
                match self.expr(expr, frame)? {
                    Val::Node(i) => Ok((i, span)),
                    Val::Sel(_, s) => Err(ExprError::new(
                        formula::TYPE,
                        self.at(frame, s),
                        "a selector is legal only as an argument whose parameter takes one",
                        "pass it to a function, for example `f('<field>')`",
                    )),
                }
            }
            None => Err(ExprError::new(
                via_diag::codes::diag::INTERNAL,
                At::Key {
                    formula: frame.formula,
                },
                "a formula was checked that was never registered",
                "report it: this is a defect of the engine, not of the text",
            )),
        }
    }

    fn at(&self, frame: &Frame, span: Range<u32>) -> At {
        At::Expr {
            formula: frame.formula,
            span,
        }
    }

    fn text(&self, frame: &Frame, span: &Range<u32>) -> String {
        self.env
            .value(frame.formula)
            .and_then(|v| v.get(span.start as usize..span.end as usize))
            .unwrap_or("")
            .to_string()
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        op: Op,
        ty: Type,
        own_rate: Rate,
        operands_from: Option<usize>,
        at: At,
        text: String,
        name: String,
        frame: &Frame,
    ) -> usize {
        let i = self.nodes.len();
        let (rate, rate_from) = match operands_from {
            Some(j) if self.nodes[j].rate > own_rate => {
                (self.nodes[j].rate, self.nodes[j].rate_from)
            }
            _ => (own_rate, i),
        };
        self.nodes.push(Node {
            op,
            ty,
            rate,
            rate_from,
            at,
            text,
            name,
            via_let: frame.via_let.clone(),
        });
        i
    }

    /// The operand with the highest rate, the first on a tie.
    fn highest(&self, operands: &[usize]) -> Option<usize> {
        let mut best: Option<usize> = None;
        for &o in operands {
            if best.is_none_or(|b| self.nodes[o].rate > self.nodes[b].rate) {
                best = Some(o);
            }
        }
        best
    }

    fn node(&mut self, e: &Expr, frame: &Frame) -> Result<usize, ExprError> {
        match self.expr(e, frame)? {
            Val::Node(i) => Ok(i),
            Val::Sel(_, s) => Err(ExprError::new(
                formula::TYPE,
                self.at(frame, s),
                "a selector is legal only as an argument whose parameter takes one",
                "pass it to a function that takes one, for example `f('<field>')`",
            )),
        }
    }

    fn typed(
        &mut self,
        e: &Expr,
        frame: &Frame,
        want: Type,
        what: &str,
    ) -> Result<usize, ExprError> {
        let i = self.node(e, frame)?;
        if self.nodes[i].ty != want {
            let t = self.text(frame, &e.span);
            return Err(ExprError::new(
                formula::TYPE,
                self.at(frame, e.span.clone()),
                format!(
                    "{what} needs {}; `{t}` is {}",
                    want.word(),
                    self.nodes[i].ty.word()
                ),
                match want {
                    Type::YesNo => "write a comparison, for example `a > 0`",
                    Type::Number => "write a number, for example `if(c, 1, 0)`",
                },
            ));
        }
        Ok(i)
    }

    fn expr(&mut self, e: &Expr, frame: &Frame) -> Result<Val, ExprError> {
        let at = self.at(frame, e.span.clone());
        let text = self.text(frame, &e.span);
        let v = match &e.kind {
            Kind::Num(v) => self.push(
                Op::Const(*v),
                Type::Number,
                Rate::Constant,
                None,
                at,
                text,
                "number".into(),
                frame,
            ),
            Kind::Sel(s) => return Ok(Val::Sel(s.clone(), e.span.clone())),
            Kind::Name(n) => self.name(n, e, frame)?,
            Kind::Member(id, attr) => self.member(id, attr, e, frame)?,
            Kind::Neg(a) => {
                let a = self.typed(a, frame, Type::Number, "`-`")?;
                self.push(
                    Op::Neg(a),
                    Type::Number,
                    Rate::Constant,
                    Some(a),
                    at,
                    text,
                    "-".into(),
                    frame,
                )
            }
            Kind::Not(a) => {
                let a = self.typed(a, frame, Type::YesNo, "`not`")?;
                self.push(
                    Op::Not(a),
                    Type::YesNo,
                    Rate::Constant,
                    Some(a),
                    at,
                    text,
                    "not".into(),
                    frame,
                )
            }
            Kind::Arith(..) => self.chain(e, frame)?,
            Kind::Cmp(op, a, b) => {
                let what = format!("`{}`", op.text());
                let a = self.typed(a, frame, Type::Number, &what)?;
                let b = self.typed(b, frame, Type::Number, &what)?;
                let h = self.highest(&[a, b]);
                self.push(
                    Op::Cmp(*op, a, b),
                    Type::YesNo,
                    Rate::Constant,
                    h,
                    at,
                    text,
                    op.text().into(),
                    frame,
                )
            }
            Kind::And(..) | Kind::Or(..) => self.chain(e, frame)?,
            Kind::Call {
                name,
                name_span,
                args,
            } => self.call(name, name_span, args, e, frame)?,
        };
        Ok(Val::Node(v))
    }

    /// A left-associative chain (`+ -`, `* /`, `and`, `or`), compiled along its left spine
    /// without recursion, so a 4096-byte chain needs no deep stack. The order is the recursive
    /// one: the leftmost operand, then each right operand and its operation.
    fn chain(&mut self, e: &Expr, frame: &Frame) -> Result<usize, ExprError> {
        let mut spine: Vec<&Expr> = Vec::new();
        let mut cur = e;
        while let Kind::Arith(_, l, _) | Kind::And(l, _) | Kind::Or(l, _) = &cur.kind {
            spine.push(cur);
            cur = l;
        }
        let want = |k: &Kind| match k {
            Kind::Arith(..) => Type::Number,
            _ => Type::YesNo,
        };
        let word = |k: &Kind| match k {
            Kind::Arith(op, _, _) => op.text(),
            Kind::And(..) => "and",
            _ => "or",
        };
        let Some(last) = spine.last() else {
            return self.node(e, frame);
        };
        let mut acc = self.typed(
            cur,
            frame,
            want(&last.kind),
            &format!("`{}`", word(&last.kind)),
        )?;
        for whole in spine.iter().rev() {
            let (Kind::Arith(_, _, r) | Kind::And(_, r) | Kind::Or(_, r)) = &whole.kind else {
                continue;
            };
            let w = word(&whole.kind);
            let ty = want(&whole.kind);
            let b = self.typed(r, frame, ty, &format!("`{w}`"))?;
            let h = self.highest(&[acc, b]);
            let op = match &whole.kind {
                Kind::Arith(o, _, _) => Op::Arith(*o, acc, b),
                Kind::And(..) => Op::And(acc, b),
                _ => Op::Or(acc, b),
            };
            let at = self.at(frame, whole.span.clone());
            let text = self.text(frame, &whole.span);
            acc = self.push(op, ty, Rate::Constant, h, at, text, w.into(), frame);
        }
        Ok(acc)
    }

    /// The names visible here, for the nearest-name hint.
    fn visible(&self, frame: &Frame) -> Vec<String> {
        let mut out: Vec<String> = ["pi", "x", "y", "d", "sd", "t", "v"]
            .iter()
            .filter(|n| Builtin::parse(n).is_some_and(|b| self.ctx.sees(b)))
            .map(|n| n.to_string())
            .collect();
        match &frame.scope {
            Scope::Kind(k) => {
                if let Some(kd) = self.env.kinds.get(k) {
                    out.extend(kd.attributes.keys().cloned());
                    out.extend(kd.lets.keys().cloned());
                }
            }
            Scope::Field(f) => {
                if let Some(l) = self.env.field_lets.get(f) {
                    out.extend(l.keys().cloned());
                }
            }
            Scope::World => {}
        }
        if frame.own {
            if let Some(fd) = self
                .ctx
                .feature
                .as_ref()
                .and_then(|f| self.env.features.get(f))
            {
                out.extend(fd.attributes.keys().cloned());
            }
        }
        out.extend(self.env.world_lets.keys().cloned());
        out
    }

    /// Where a name visible in `frame` is declared (`diagnostics.md` 1.3, `related` of a
    /// not-found record): a `let`'s formula key, else the attribute's declaration. `None` for a
    /// built-in name, which no text declares.
    fn declared_at(&self, n: &str, frame: &Frame) -> Option<At> {
        if Builtin::parse(n).is_some() {
            return None;
        }
        let decl = |scope: Scope| At::Declaration {
            scope,
            name: n.to_string(),
        };
        match &frame.scope {
            Scope::Kind(k) => {
                if let Some(kd) = self.env.kinds.get(k) {
                    if let Some(f) = kd.lets.get(n) {
                        return Some(At::Key { formula: *f });
                    }
                    if kd.attributes.contains_key(n) {
                        return Some(decl(Scope::Kind(k.clone())));
                    }
                }
            }
            Scope::Field(fld) => {
                if let Some(f) = self.env.field_lets.get(fld).and_then(|l| l.get(n)) {
                    return Some(At::Key { formula: *f });
                }
            }
            Scope::World => {}
        }
        if frame.own {
            if let Some(f) = self.ctx.feature.as_ref() {
                if self
                    .env
                    .features
                    .get(f)
                    .is_some_and(|fd| fd.attributes.contains_key(n))
                {
                    return Some(decl(Scope::Kind(f.clone())));
                }
            }
        }
        self.env.world_lets.get(n).map(|f| At::Key { formula: *f })
    }

    fn name(&mut self, n: &str, e: &Expr, frame: &Frame) -> Result<usize, ExprError> {
        let at = self.at(frame, e.span.clone());
        if let Some(b) = Builtin::parse(n) {
            if b == Builtin::Z {
                return Err(ExprError::new(
                    doc::NOT_YET,
                    at,
                    "`z` arrives with volume formulas in milestone 5",
                    "remove it for now",
                ));
            }
            if !self.ctx.sees(b) {
                let mut msg = format!(
                    "`{n}` exists in {}, not in {}",
                    b.exists_in(),
                    self.ctx.key.words()
                );
                if b == Builtin::V && self.ctx.weights {
                    msg = format!("`v` does not exist on a `weights` field; {}", msg);
                }
                return Err(ExprError::new(
                    formula::UNKNOWN_NAME,
                    at,
                    msg,
                    match b {
                        Builtin::D | Builtin::Sd | Builtin::T => {
                            "measure a feature with `dist`, `sdist` or `along` instead"
                        }
                        _ => "read the field with `f('<field>')` instead",
                    },
                )
                .with("name", n));
            }
            let (op, name) = match b {
                Builtin::Pi => (Op::Const(via_num::PI), "pi"),
                Builtin::X => (Op::X, "x"),
                Builtin::Y => (Op::Y, "y"),
                Builtin::D => (Op::Local(Measure::D), "d"),
                Builtin::Sd => (Op::Local(Measure::Sd), "sd"),
                Builtin::T => (Op::Local(Measure::T), "t"),
                Builtin::V => {
                    self.reads.entry_field = true;
                    (Op::V, "v")
                }
                Builtin::Z => unreachable_z(),
            };
            return Ok(self.push(
                op,
                Type::Number,
                b.rate(),
                None,
                at,
                n.to_string(),
                name.into(),
                frame,
            ));
        }
        // The formula's own feature's attributes (6.6, 6.7).
        if frame.own {
            if let Some(fd) = self
                .ctx
                .feature
                .as_ref()
                .and_then(|f| self.env.features.get(f))
            {
                if let Some(ty) = fd.attributes.get(n) {
                    return self.attribute(n, *ty, e, frame);
                }
            }
        }
        // The inner scope, then the world's (5).
        match &frame.scope {
            Scope::Kind(k) => {
                if let Some(kd) = self.env.kinds.get(k) {
                    if let Some(ty) = kd.attributes.get(n) {
                        return self.attribute(n, *ty, e, frame);
                    }
                    if let Some(id) = kd.lets.get(n) {
                        let parts = vec![
                            NoisePart::Text("noise".into()),
                            NoisePart::Text("feature".into()),
                            NoisePart::Feature,
                            NoisePart::Text("let".into()),
                            NoisePart::Text(n.to_string()),
                        ];
                        return self.read_let(Scope::Kind(k.clone()), n, *id, parts, e, frame);
                    }
                }
            }
            Scope::Field(f) => {
                if let Some(id) = self.env.field_lets.get(f).and_then(|l| l.get(n)) {
                    let parts = ["noise", "field", f, "let", n]
                        .iter()
                        .map(|p| NoisePart::Text(p.to_string()))
                        .collect();
                    return self.read_let(Scope::Field(f.clone()), n, *id, parts, e, frame);
                }
            }
            Scope::World => {}
        }
        if let Some(id) = self.env.world_lets.get(n) {
            let parts = ["noise", "let", n]
                .iter()
                .map(|p| NoisePart::Text(p.to_string()))
                .collect();
            return self.read_let(Scope::World, n, *id, parts, e, frame);
        }
        let visible = self.visible(frame);
        let near = nearest(n, visible.iter().map(String::as_str));
        let hint = if self.env.fields.contains(n) {
            format!("`{n}` is a field; read it with `f('{n}')`")
        } else if table::lookup(n).is_some() {
            format!("`{n}` is a function; call it, `{n}(...)`")
        } else {
            near.hint("declare it in `[let]`")
        };
        let mut err = ExprError::new(
            formula::UNKNOWN_NAME,
            at,
            format!("`{n}` is not a name visible in {}", self.ctx.key.words()),
            hint,
        )
        .with("name", n);
        if let Nearest::Found(f) = near {
            err = err.with("nearest", f);
            if let Some(at) = self.declared_at(f, frame) {
                err.related.push((at, "declared here".to_string()));
            }
        }
        Err(err)
    }

    fn attribute(
        &mut self,
        n: &str,
        ty: AttrType,
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let at = self.at(frame, e.span.clone());
        let t = match ty {
            AttrType::Number => Type::Number,
            AttrType::YesNo => Type::YesNo,
            other => {
                return Err(ExprError::new(
                    formula::TYPE,
                    at,
                    format!(
                        "the attribute `{n}` is {}; a formula reads numbers and bools",
                        other.word()
                    ),
                    "read a number or bool attribute",
                ))
            }
        };
        self.reads.attributes.insert(n.to_string());
        Ok(self.push(
            Op::Attr(n.to_string()),
            t,
            Rate::PerFeature,
            None,
            at,
            n.to_string(),
            n.into(),
            frame,
        ))
    }

    fn member(
        &mut self,
        id: &str,
        attr: &str,
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let at = self.at(frame, e.span.clone());
        let Some(fd) = self.env.features.get(id) else {
            let near = nearest(id, self.env.features.keys().map(String::as_str));
            let mut err = ExprError::new(
                formula::UNKNOWN_NAME,
                at,
                format!("`{id}` is not a feature of this world"),
                near.hint("`<id>.<attribute>` reads a feature's attribute"),
            )
            .with("name", id);
            if let Nearest::Found(f) = near {
                err = err.with("nearest", f);
            }
            return Err(err);
        };
        let Some(ty) = fd.attributes.get(attr) else {
            // The bytes of the attribute name: after `<id>.`.
            let start = e.span.end - attr.len() as u32;
            let near = nearest(attr, fd.attributes.keys().map(String::as_str));
            let of = match &fd.kind {
                Some(k) => format!("`{id}` (kind `{k}`)"),
                None => format!("`{id}`"),
            };
            let mut err = ExprError::new(
                formula::UNKNOWN_NAME,
                self.at(frame, start..e.span.end),
                format!("`{attr}` is not an attribute of {of}"),
                near.hint("declare the attribute on the feature or its kind"),
            )
            .with("name", attr);
            if let Nearest::Found(f) = near {
                err = err.with("nearest", f);
            }
            return Err(err);
        };
        let t = match ty {
            AttrType::Number => Type::Number,
            AttrType::YesNo => Type::YesNo,
            other => {
                return Err(ExprError::new(
                    formula::TYPE,
                    at,
                    format!(
                        "the attribute `{id}.{attr}` is {}; a formula reads numbers and bools",
                        other.word()
                    ),
                    "read a number or bool attribute",
                ))
            }
        };
        self.reads
            .members
            .insert((id.to_string(), attr.to_string()));
        let text = self.text(frame, &e.span);
        Ok(self.push(
            Op::Member(id.to_string(), attr.to_string()),
            t,
            Rate::Constant,
            None,
            at,
            text,
            format!("{id}.{attr}"),
            frame,
        ))
    }

    fn read_let(
        &mut self,
        scope: Scope,
        name: &str,
        id: FormulaId,
        noise: Vec<NoisePart>,
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let key = (scope.clone(), name.to_string());
        if let Some(&i) = self.lets.get(&key) {
            return Ok(i);
        }
        if let Some(pos) = self.stack.iter().position(|k| *k == key) {
            let chain: Vec<String> = self.stack[pos..]
                .iter()
                .chain(std::iter::once(&key))
                .map(|(_, n)| format!("`{n}`"))
                .collect();
            return Err(ExprError::new(
                graph::CYCLE,
                self.at(frame, e.span.clone()),
                format!(
                    "the `let` names read each other in a cycle: {}",
                    chain.join(" -> ")
                ),
                "break the cycle: one of these must not read the others",
            ));
        }
        self.reads
            .lets
            .insert((scope.to_string(), name.to_string()));
        self.stack.push(key.clone());
        let inner = Frame {
            formula: id,
            scope,
            own: false,
            noise,
            via_let: Some((
                format!("read as `{name}` here"),
                frame
                    .via_let
                    .as_ref()
                    .map(|(_, a)| a.clone())
                    .unwrap_or_else(|| self.at(frame, e.span.clone())),
            )),
        };
        let r = self.body(&inner);
        self.stack.pop();
        let (i, _) = r?;
        self.lets.insert(key, i);
        Ok(i)
    }

    fn call(
        &mut self,
        name: &str,
        name_span: &Range<u32>,
        args: &[Arg],
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let at = self.at(frame, e.span.clone());
        let name_at = self.at(frame, name_span.clone());
        let Some(func) = table::lookup(name) else {
            let near = nearest(name, table::TABLE.iter().map(|f| f.name));
            let hint = match table::alias(name) {
                Some(h) => h.to_string(),
                None => near.hint("see the function table, `via explain formula.unknown_function`"),
            };
            let mut err = ExprError::new(
                formula::UNKNOWN_FUNCTION,
                name_at,
                format!("`{name}` is not a function of the table"),
                hint,
            );
            if let (None, Nearest::Found(f)) = (table::alias(name), near) {
                err = err.with("nearest", f);
            }
            return Err(err);
        };
        if func.milestone > 2 {
            return Err(ExprError::new(
                doc::NOT_YET,
                name_at,
                format!("`{name}` arrives in milestone {}", func.milestone),
                "remove it for now",
            ));
        }
        let kind_key = matches!(
            self.ctx.key,
            Key::EffectValue | Key::EffectMask | Key::MaxChange | Key::KindLet
        );
        match func.where_ {
            Where::Any => {}
            Where::KindEffectOrLet | Where::ScalarKindEffectOrLet if !kind_key => {
                return Err(ExprError::new(
                    formula::UNKNOWN_FUNCTION,
                    name_at,
                    format!(
                        "`{name}` exists in {}, not in {}",
                        func.where_text,
                        self.ctx.key.words()
                    ),
                    "move the term to a kind's effect or `let`",
                ));
            }
            Where::ScalarKindEffectOrLet if self.ctx.weights => {
                return Err(ExprError::new(
                    formula::TYPE,
                    name_at,
                    format!("`{name}` reads the written field, which here is a `weights` field"),
                    "use it on an effect that writes a scalar field",
                ));
            }
            _ => {}
        }
        let bound = self.bind(func, args, e, frame)?;
        self.build(func, bound, at, e, frame)
    }

    /// Matches arguments to parameters (`formulas.md` 3: positional first, each at most once,
    /// every name a parameter).
    fn bind<'e>(
        &self,
        func: &Function,
        args: &'e [Arg],
        e: &Expr,
        frame: &Frame,
    ) -> Result<Vec<Option<&'e Expr>>, ExprError> {
        let name = func.name;
        let arg_err = |span: Range<u32>, msg: String| {
            let sig = func.signatures.join("` or `");
            ExprError::new(
                formula::ARGUMENTS,
                self.at(frame, span),
                msg,
                format!("write `{sig}`"),
            )
        };
        if func.variadic {
            if let Some(a) = args.iter().find(|a| a.name.is_some()) {
                let (n, s) = a.name.clone().unwrap_or_default();
                return Err(arg_err(
                    s,
                    format!("`{name}` takes no named argument, `{n}` was given"),
                ));
            }
            if args.len() < 2 {
                return Err(arg_err(
                    e.span.clone(),
                    format!("`{name}` takes two or more numbers, {} given", args.len()),
                ));
            }
            return Ok(args.iter().map(|a| Some(&a.value)).collect());
        }
        // `under(stat)` is `under(field, stat)` with the field left out.
        let params: Vec<&str> =
            if func.fun == Fun::Under && args.len() == 1 && args[0].name.is_none() {
                vec!["stat"]
            } else {
                func.params.iter().map(|p| p.name).collect()
            };
        let mut slots: Vec<Option<&Expr>> = vec![None; func.params.len()];
        let mut named_seen = false;
        let mut positional = 0usize;
        for a in args {
            match &a.name {
                None => {
                    if named_seen {
                        return Err(arg_err(
                            a.value.span.clone(),
                            format!("a positional argument of `{name}` follows a named one"),
                        ));
                    }
                    let Some(pname) = params.get(positional) else {
                        return Err(arg_err(
                            a.value.span.clone(),
                            format!("`{name}` takes at most {} arguments", params.len()),
                        ));
                    };
                    let idx = func
                        .params
                        .iter()
                        .position(|p| p.name == *pname)
                        .unwrap_or(0);
                    slots[idx] = Some(&a.value);
                    positional += 1;
                }
                Some((n, s)) => {
                    named_seen = true;
                    let Some(idx) = func.params.iter().position(|p| p.name == n) else {
                        let list: Vec<String> = func
                            .params
                            .iter()
                            .map(|p| format!("`{}`", p.name))
                            .collect();
                        return Err(arg_err(
                            s.clone(),
                            format!(
                                "`{name}` has no parameter `{n}`; its parameters are {}",
                                list.join(", ")
                            ),
                        ));
                    };
                    if slots[idx].is_some() {
                        return Err(arg_err(
                            s.clone(),
                            format!("`{n}` of `{name}` is given twice"),
                        ));
                    }
                    slots[idx] = Some(&a.value);
                }
            }
        }
        for (p, slot) in func.params.iter().zip(&slots) {
            if slot.is_none() && p.default == Default::Required {
                if func.fun == Fun::Under
                    && p.name == "field"
                    && slots.iter().filter(|s| s.is_some()).count() == 1
                    && slots[1].is_some()
                {
                    continue;
                }
                return Err(arg_err(
                    e.span.clone(),
                    format!("`{name}` needs `{}`", p.name),
                ));
            }
        }
        Ok(slots)
    }

    /// A number argument: type, rate and constant range checked.
    fn number_arg(
        &mut self,
        func: &Function,
        pi: usize,
        arg: Option<&Expr>,
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let p = &func.params[pi];
        let Some(a) = arg else {
            let at = self.at(frame, e.span.clone());
            let v = match p.default {
                Default::Number(v) => v,
                Default::Radius => match self.ctx.radius {
                    Some(r) => r,
                    None => {
                        return Err(ExprError::new(
                            formula::CAP,
                            at,
                            format!("`{}` has no cap, and there is no `radius` to default to", func.name),
                            format!(
                                "give the cap, `{}('<shape>', 50)`: no farther than the world's extent matters, and the work grows with it",
                                func.name
                            ),
                        ))
                    }
                },
                _ => 0.0,
            };
            return Ok(self.push(
                Op::Const(v),
                Type::Number,
                Rate::Constant,
                None,
                at,
                crate::error::num_text(v),
                p.name.into(),
                frame,
            ));
        };
        let what = format!("`{}` of `{}`", p.name, func.name);
        let i = match self.expr(a, frame)? {
            Val::Node(i) => i,
            Val::Sel(_, s) => {
                return Err(ExprError::new(
                    formula::TYPE,
                    self.at(frame, s),
                    format!("{what} is a number, not a selector"),
                    "write a number",
                ))
            }
        };
        if self.nodes[i].ty != Type::Number {
            return Err(ExprError::new(
                formula::TYPE,
                self.at(frame, a.span.clone()),
                format!(
                    "{what} needs a number; `{}` is a yes/no",
                    self.text(frame, &a.span)
                ),
                "write a number, for example `if(c, 1, 0)`",
            ));
        }
        if self.nodes[i].rate > p.max_rate {
            let from = &self.nodes[self.nodes[i].rate_from];
            return Err(ExprError::new(
                formula::RATE,
                self.at(frame, a.span.clone()),
                format!(
                    "{what} must be known {}; `{}` is {}",
                    p.max_rate.word(),
                    from.text,
                    from.rate.word()
                ),
                "write a constant or an attribute here",
            ));
        }
        if p.bound != Bound::None {
            if let Some(v) = crate::eval::fold_nodes(&self.nodes, i) {
                if let Some(rule) = crate::eval::bound_breach(p.bound, v) {
                    return Err(ExprError::new(
                        formula::RANGE,
                        self.at(frame, a.span.clone()),
                        format!("{what} is {}; it must be {rule}", crate::error::num_text(v)),
                        format!("give `{}` a value {rule}", p.name),
                    ));
                }
            }
        }
        Ok(i)
    }

    /// A selector argument: its form and class (`formulas.md` 7). Returns its index.
    fn selector_arg(
        &mut self,
        func: &Function,
        pi: usize,
        arg: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let p = &func.params[pi];
        let ParamType::Selector(class) = p.ty else {
            return Err(ExprError::new(
                via_diag::codes::diag::INTERNAL,
                self.at(frame, arg.span.clone()),
                "a selector parameter was expected",
                "report it: this is a defect of the engine",
            ));
        };
        let what = format!("`{}` of `{}`", p.name, func.name);
        let (s, span) = match self.expr(arg, frame)? {
            Val::Sel(s, span) => (s, span),
            Val::Node(_) => {
                let at = self.at(frame, arg.span.clone());
                return Err(if class == Class::Seed {
                    ExprError::new(
                        formula::SEED,
                        at,
                        "a seed is a name, never a number",
                        "write `seed = '<name>'`",
                    )
                } else {
                    ExprError::new(
                        formula::TYPE,
                        at,
                        format!("{what} takes a {} selector in single quotes", class.word()),
                        format!("write `{}('<name>')`", func.name),
                    )
                });
            }
        };
        let at = self.at(frame, span.clone());
        // In a returned document a name resolves first to the document's own keys (`protocol.md`
        // 5, Names): `'eroded'` there is `'<step>#eroded'`.
        let s = match self.env.scopes.get(&frame.formula) {
            Some(step) if !s.contains('#') && !s.contains(':') => {
                let own = format!("{step}#{s}");
                if self.env.fields.contains(&own) || self.env.features.contains_key(&own) {
                    own
                } else {
                    s
                }
            }
            _ => s,
        };
        let produced = self.env.scopes.contains_key(&frame.formula)
            && s.split_once('#')
                .is_some_and(|(p, _)| self.env.scopes.get(&frame.formula).is_some_and(|q| q == p));
        if s.contains('#') && !produced {
            return Err(ExprError::new(
                identity::KEY_IN_TEXT,
                at,
                format!("`{s}` is a produced name; produced names are never written in text"),
                "select produced things by what they are, `'kind:<name>'` or `'tag:<tag>'`",
            ));
        }
        let fields = || self.env.fields.iter().map(String::as_str);
        match class {
            Class::Field => {
                if let Some(rest) = s.strip_prefix("role:") {
                    return Err(sel_err(
                        formula::SELECTOR,
                        at,
                        format!("formulas never read a field by role (`role:{rest}`)"),
                        "tag the field and write `'tag:<tag>'`".into(),
                    ));
                }
                if s.starts_with("tag:") {
                    // Resolved in the selection phase: a step may return a tagged field.
                } else if s.contains(':') || s.contains('.') {
                    return Err(sel_err(
                        formula::SELECTOR,
                        at,
                        format!("`{s}` is not a field selector"),
                        "write a field's name or `'tag:<tag>'`".into(),
                    ));
                } else if !self.env.fields.contains(&s) {
                    let near = nearest(&s, fields());
                    let msg = if self.env.features.contains_key(&s) {
                        format!("`{s}` names a feature, not a field")
                    } else {
                        format!("`{s}` names no field")
                    };
                    let mut err = sel_err(
                        formula::SELECTOR,
                        at,
                        msg,
                        near.hint("declare the field, or write `'tag:<tag>'`"),
                    );
                    if let Nearest::Found(f) = near {
                        err = err.with("nearest", f);
                    }
                    return Err(err);
                }
            }
            Class::Shape => {
                let parts: Vec<&str> = s.split('.').collect();
                if s.starts_with("role:") || s.starts_with("tag:") {
                    return Err(sel_err(
                        formula::SELECTOR,
                        at,
                        format!("`{s}` selects fields; {what} takes a feature"),
                        "write a feature's id or `'kind:<name>'`".into(),
                    ));
                } else if s.starts_with("kind:") {
                } else if parts.len() > 2 {
                    return Err(ExprError::new(
                        doc::NOT_YET,
                        at,
                        format!(
                            "`{s}` names a place inside an import, which arrives in milestone 4"
                        ),
                        "remove it for now",
                    ));
                } else if parts.len() == 1 && self.env.fields.contains(&s) {
                    return Err(sel_err(
                        formula::SELECTOR,
                        at,
                        format!("`{s}` names a field; {what} takes a feature"),
                        format!("read the field with `f('{s}')`"),
                    ));
                } else if parts.len() == 1 && !self.env.features.contains_key(&s) {
                    let near = nearest(&s, self.env.features.keys().map(String::as_str));
                    let mut err = sel_err(
                        formula::SELECTOR,
                        at,
                        format!("`{s}` names no feature"),
                        near.hint("write a feature's id or `'kind:<name>'`"),
                    );
                    if let Nearest::Found(f) = near {
                        err = err.with("nearest", f);
                    }
                    return Err(err);
                }
            }
            Class::Seed => {
                if via_num::seed_name(&s).is_err() {
                    return Err(ExprError::new(
                        formula::SEED,
                        at,
                        format!("`{s}` is not a seed name"),
                        "a seed is a name: lower case, digits and `_`, starting with a letter"
                            .to_string(),
                    ));
                }
            }
            Class::Statistic => match s.as_str() {
                "mean" | "min" | "max" => {}
                // Regions only (8.5): a path or point is `formula.shape`, which the selection
                // phase says first, knowing the feature's shape; on a region it is `doc.not_yet`
                // until milestone 2 (`bind`).
                "edge_min" => {}
                _ => {
                    let near = nearest(&s, ["mean", "min", "max"]);
                    let mut err = sel_err(
                        formula::SELECTOR,
                        at,
                        format!("`{s}` is not a statistic"),
                        near.hint("write `'mean'`, `'min'` or `'max'`"),
                    );
                    if let Nearest::Found(f) = near {
                        err = err.with("nearest", f);
                    }
                    return Err(err);
                }
            },
        }
        if matches!(class, Class::Field | Class::Shape) {
            self.reads.selectors.insert((class, s.clone()));
        }
        self.selectors.push(SelectorUse {
            class,
            text: s,
            function: func.name,
            at,
        });
        Ok(self.selectors.len() - 1)
    }

    fn build(
        &mut self,
        func: &Function,
        slots: Vec<Option<&Expr>>,
        at: At,
        e: &Expr,
        frame: &Frame,
    ) -> Result<usize, ExprError> {
        let text = self.text(frame, &e.span);
        let name = func.name.to_string();
        let num = |c: &mut Self, i: usize| {
            c.number_arg(func, i.min(func.params.len() - 1), slots[i], e, frame)
        };
        let own = func.rate.unwrap_or(Rate::Constant);
        let unary = |c: &mut Self, f: fn(usize) -> Op| -> Result<usize, ExprError> {
            let a = num(c, 0)?;
            Ok(c.push(
                f(a),
                Type::Number,
                own,
                Some(a),
                at.clone(),
                text.clone(),
                name.clone(),
                frame,
            ))
        };
        match func.fun {
            Fun::Abs => unary(self, Op::Abs),
            Fun::Sqrt => unary(self, Op::Sqrt),
            Fun::Exp => unary(self, Op::Exp),
            Fun::Floor => unary(self, Op::Floor),
            Fun::Sin => unary(self, Op::Sin),
            Fun::Cos => unary(self, Op::Cos),
            Fun::Tan => unary(self, Op::Tan),
            Fun::Pow => {
                let a = num(self, 0)?;
                let b = num(self, 1)?;
                let h = self.highest(&[a, b]);
                Ok(self.push(Op::Pow(a, b), Type::Number, own, h, at, text, name, frame))
            }
            Fun::Min | Fun::Max => {
                let mut acc = num(self, 0)?;
                for k in 1..slots.len() {
                    let b = num(self, k)?;
                    let h = self.highest(&[acc, b]);
                    let op = if func.fun == Fun::Min {
                        Op::Min(acc, b)
                    } else {
                        Op::Max(acc, b)
                    };
                    acc = self.push(
                        op,
                        Type::Number,
                        own,
                        h,
                        at.clone(),
                        text.clone(),
                        name.clone(),
                        frame,
                    );
                }
                Ok(acc)
            }
            Fun::Clamp | Fun::Mix | Fun::Smoothstep => {
                let a = num(self, 0)?;
                let b = num(self, 1)?;
                let c = num(self, 2)?;
                let h = self.highest(&[a, b, c]);
                let op = match func.fun {
                    Fun::Clamp => Op::Clamp(a, b, c),
                    Fun::Mix => Op::Mix(a, b, c),
                    _ => Op::Smoothstep(a, b, c),
                };
                Ok(self.push(op, Type::Number, own, h, at, text, name, frame))
            }
            Fun::If => {
                let (Some(ce), Some(ae), Some(be)) = (slots[0], slots[1], slots[2]) else {
                    return Err(ExprError::new(
                        formula::ARGUMENTS,
                        at,
                        "`if` needs `c`, `a` and `b`",
                        "write `if(c, a, b)`",
                    ));
                };
                let c = self.typed(ce, frame, Type::YesNo, "the condition of `if`")?;
                let a = self.node(ae, frame)?;
                let b = self.node(be, frame)?;
                if self.nodes[a].ty != self.nodes[b].ty {
                    return Err(ExprError::new(
                        formula::TYPE,
                        self.at(frame, be.span.clone()),
                        format!(
                            "the branches of `if` must be of one type; `{}` is {} and `{}` is {}",
                            self.text(frame, &ae.span),
                            self.nodes[a].ty.word(),
                            self.text(frame, &be.span),
                            self.nodes[b].ty.word()
                        ),
                        "make both branches numbers, or both yes/no",
                    ));
                }
                let ty = self.nodes[a].ty;
                let h = self.highest(&[c, a, b]);
                Ok(self.push(Op::If(c, a, b), ty, own, h, at, text, name, frame))
            }
            Fun::Fbm | Fun::Ridged => {
                let wavelength = num(self, 0)?;
                let octaves = num(self, 1)?;
                let seed = match slots[2] {
                    Some(s) => {
                        let k = self.selector_arg(func, 2, s, frame)?;
                        Seed::Named(self.selectors[k].text.clone())
                    }
                    None => Seed::Default(frame.noise.clone()),
                };
                let gain = num(self, 3)?;
                let lacunarity = num(self, 4)?;
                self.noise.push((seed.clone(), at.clone()));
                let h = self.highest(&[wavelength, octaves, gain, lacunarity]);
                Ok(self.push(
                    Op::Fbm {
                        wavelength,
                        octaves,
                        gain,
                        lacunarity,
                        seed,
                        ridged: func.fun == Fun::Ridged,
                    },
                    Type::Number,
                    own,
                    h,
                    at,
                    text,
                    name,
                    frame,
                ))
            }
            Fun::F | Fun::Slope => {
                let Some(s) = slots[0] else {
                    return Err(ExprError::new(
                        formula::ARGUMENTS,
                        at,
                        format!("`{name}` needs `field`"),
                        format!("write `{name}('<field>')`"),
                    ));
                };
                let k = self.selector_arg(func, 0, s, frame)?;
                let op = if func.fun == Fun::F {
                    Op::Field(k)
                } else {
                    self.halo = self.halo.max(1);
                    Op::Slope(k)
                };
                Ok(self.push(op, Type::Number, own, None, at, text, name, frame))
            }
            Fun::Dist | Fun::Sdist | Fun::Near | Fun::Inside | Fun::Along => {
                let Some(s) = slots[0] else {
                    return Err(ExprError::new(
                        formula::ARGUMENTS,
                        at,
                        format!("`{name}` needs `shape`"),
                        format!("write `{}`", func.signatures[0]),
                    ));
                };
                let shape = self.selector_arg(func, 0, s, frame)?;
                let (op, h) = match func.fun {
                    Fun::Dist | Fun::Sdist | Fun::Near => {
                        let c = num(self, 1)?;
                        let op = match func.fun {
                            Fun::Dist => Op::Dist { shape, cap: c },
                            Fun::Sdist => Op::Sdist { shape, cap: c },
                            _ => Op::Near { shape, radius: c },
                        };
                        (op, Some(c))
                    }
                    Fun::Inside => (Op::Inside(shape), None),
                    _ => (Op::Along(shape), None),
                };
                let ty = if func.fun == Fun::Inside {
                    Type::YesNo
                } else {
                    Type::Number
                };
                Ok(self.push(op, ty, own, h, at, text, name, frame))
            }
            Fun::Under => {
                let field = match slots[0] {
                    Some(f) => Some(self.selector_arg(func, 0, f, frame)?),
                    None => {
                        self.reads.entry_field = true;
                        None
                    }
                };
                let Some(st) = slots[1] else {
                    return Err(ExprError::new(
                        formula::ARGUMENTS,
                        at,
                        "`under` needs `stat`",
                        "write `under('mean')`",
                    ));
                };
                let k = self.selector_arg(func, 1, st, frame)?;
                let stat = match self.selectors[k].text.as_str() {
                    "min" => Stat::Min,
                    "max" => Stat::Max,
                    _ => Stat::Mean,
                };
                Ok(self.push(
                    Op::Under { field, stat },
                    Type::Number,
                    own,
                    None,
                    at,
                    text,
                    name,
                    frame,
                ))
            }
            Fun::SmoothedAlong => {
                let l = num(self, 0)?;
                self.reads.entry_field = true;
                Ok(self.push(
                    Op::SmoothedAlong(l),
                    Type::Number,
                    own,
                    Some(l),
                    at,
                    text,
                    name,
                    frame,
                ))
            }
        }
    }
}

fn unreachable_z() -> (Op, &'static str) {
    // `z` returns `doc.not_yet` before this point; a constant keeps the function total.
    (Op::Const(0.0), "z")
}
