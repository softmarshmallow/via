//! Marking and syntax (`formulas.md` 1 and 3).
//!
//! A recursive-descent parser of section 3's EBNF, one function per rule. Nesting (parentheses,
//! calls, prefix operators) is counted and bounded at [`MAX_DEPTH`], so the parser's stack is
//! bounded for any text an agent writes.

use std::ops::Range;

use via_diag::codes::formula;

use crate::error::{At, ExprError, FormulaId};
use crate::lex::{lex, Op, Tok, Token, MAX_LEN};

/// The deepest nesting allowed (`formula.too_deep`).
pub const MAX_DEPTH: usize = 64;

/// A binary arithmetic operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
}

impl Arith {
    pub fn text(self) -> &'static str {
        match self {
            Arith::Add => "+",
            Arith::Sub => "-",
            Arith::Mul => "*",
            Arith::Div => "/",
        }
    }
}

/// A comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl Cmp {
    pub fn text(self) -> &'static str {
        match self {
            Cmp::Lt => "<",
            Cmp::Le => "<=",
            Cmp::Gt => ">",
            Cmp::Ge => ">=",
            Cmp::Eq => "==",
            Cmp::Ne => "!=",
        }
    }

    fn from_op(op: Op) -> Option<Cmp> {
        Some(match op {
            Op::Lt => Cmp::Lt,
            Op::Le => Cmp::Le,
            Op::Gt => Cmp::Gt,
            Op::Ge => Cmp::Ge,
            Op::EqEq => Cmp::Eq,
            Op::Ne => Cmp::Ne,
            _ => return None,
        })
    }
}

/// An argument of a call: positional or `name = expr`.
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<(String, Range<u32>)>,
    pub value: Expr,
}

/// A parsed expression and its bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: Kind,
    pub span: Range<u32>,
}

/// The forms of section 3.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Num(f64),
    Sel(String),
    Name(String),
    /// `<id>.<attribute>`.
    Member(String, String),
    Call {
        name: String,
        name_span: Range<u32>,
        args: Vec<Arg>,
    },
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Arith(Arith, Box<Expr>, Box<Expr>),
    Cmp(Cmp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// Where a string is read (`formulas.md` 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// A key whose Takes column says `formula`: the string must start with `=`.
    Key,
    /// A feature attribute or a kind's `defaults`: `=` makes it a formula, else it is text.
    Attribute,
}

/// What a string in an attribute is (`formulas.md` 1): a formula, text, or text that looks like
/// a formula with something before its `=` (`formula.unmarked`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marked {
    Formula,
    Text,
    Unmarked,
}

/// Classifies a string by its mark.
pub fn marked(value: &str, mark: Mark) -> Marked {
    if value.starts_with('=') {
        return Marked::Formula;
    }
    let first = value.trim_start_matches([' ', '\t']).chars().next();
    if first == Some('=') || mark == Mark::Key {
        Marked::Unmarked
    } else {
        Marked::Text
    }
}

/// Parses a marked formula string, checking its mark, size and syntax.
pub fn parse(formula: FormulaId, value: &str) -> Result<Expr, ExprError> {
    let whole = || At::Expr {
        formula,
        span: 0..value.len() as u32,
    };
    if !value.starts_with('=') {
        let first = value.trim_start_matches([' ', '\t']).chars().next();
        let hint = if first == Some('=') {
            "remove what comes before `=`"
        } else {
            "start the string with `=`"
        };
        return Err(ExprError::new(
            formula::UNMARKED,
            whole(),
            "a formula string must start with `=`",
            hint,
        ));
    }
    if value.len() - 1 > MAX_LEN {
        return Err(ExprError::new(
            formula::TOO_LONG,
            whole(),
            format!(
                "the formula is {} bytes long, over the limit of {MAX_LEN}",
                value.len() - 1
            ),
            "name parts of it with `let`, or move the logic to a step",
        ));
    }
    let tokens = lex(formula, value, 1)?;
    if tokens.is_empty() {
        return Err(ExprError::new(
            formula::SYNTAX,
            whole(),
            "expected an expression after `=`",
            "write an expression after `=`",
        ));
    }
    let mut p = Parser {
        formula,
        value,
        tokens,
        pos: 0,
        depth: 0,
    };
    let e = p.expr()?;
    if p.pos < p.tokens.len() {
        return Err(p.unexpected("an operator or the end of the formula"));
    }
    Ok(e)
}

struct Parser<'a> {
    formula: FormulaId,
    value: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn at(&self, span: Range<u32>) -> At {
        At::Expr {
            formula: self.formula,
            span,
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn peek_op(&self) -> Option<Op> {
        match self.peek() {
            Some(Tok::Op(op)) => Some(*op),
            _ => None,
        }
    }

    fn span_here(&self) -> Range<u32> {
        match self.tokens.get(self.pos) {
            Some(t) => t.span.clone(),
            None => {
                let end = self.value.len() as u32;
                end..end
            }
        }
    }

    fn text(&self, span: &Range<u32>) -> &str {
        self.value
            .get(span.start as usize..span.end as usize)
            .unwrap_or("")
    }

    /// The error for a token that cannot stand here: `=` for `==` is `formula.operator`.
    fn unexpected(&self, expected: &str) -> ExprError {
        let span = self.span_here();
        match self.peek() {
            Some(Tok::Op(Op::Assign)) => ExprError::new(
                formula::OPERATOR,
                self.at(span),
                "`=` appears only after a parameter name in a call",
                "write `==`",
            ),
            None => ExprError::new(
                formula::SYNTAX,
                self.at(span),
                format!("the formula ends where {expected} was expected"),
                format!("write {expected}"),
            ),
            Some(_) => {
                let t = self.text(&span).to_string();
                ExprError::new(
                    formula::SYNTAX,
                    self.at(span),
                    format!("expected {expected}, found `{t}`"),
                    format!("write {expected} here"),
                )
            }
        }
    }

    fn enter(&mut self, span: Range<u32>) -> Result<(), ExprError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ExprError::new(
                formula::TOO_DEEP,
                self.at(span),
                format!("the formula nests deeper than {MAX_DEPTH}"),
                "name inner parts with `let`",
            ));
        }
        Ok(())
    }

    fn expect(&mut self, op: Op, what: &str) -> Result<Range<u32>, ExprError> {
        if self.peek_op() == Some(op) {
            let s = self.span_here();
            self.pos += 1;
            Ok(s)
        } else {
            Err(self.unexpected(what))
        }
    }

    fn expr(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.and_expr()?;
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            let right = self.and_expr()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::Or(Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.not_expr()?;
        while self.peek() == Some(&Tok::And) {
            self.pos += 1;
            let right = self.not_expr()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::And(Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn not_expr(&mut self) -> Result<Expr, ExprError> {
        if self.peek() == Some(&Tok::Not) {
            let s = self.span_here();
            self.enter(s.clone())?;
            self.pos += 1;
            let inner = self.not_expr()?;
            self.depth -= 1;
            let span = s.start..inner.span.end;
            return Ok(Expr {
                kind: Kind::Not(Box::new(inner)),
                span,
            });
        }
        self.compare()
    }

    fn compare(&mut self) -> Result<Expr, ExprError> {
        let left = self.sum()?;
        let Some(cmp) = self.peek_op().and_then(Cmp::from_op) else {
            return Ok(left);
        };
        self.pos += 1;
        let right = self.sum()?;
        if let Some(cmp2) = self.peek_op().and_then(Cmp::from_op) {
            let op_span = self.span_here();
            self.pos += 1;
            let third = self.sum()?;
            let (a, b, c) = (
                self.text(&left.span),
                self.text(&right.span),
                self.text(&third.span),
            );
            return Err(ExprError::new(
                formula::SYNTAX,
                self.at(op_span),
                "comparisons do not chain",
                format!("write `{a} {} {b} and {b} {} {c}`", cmp.text(), cmp2.text()),
            ));
        }
        let span = left.span.start..right.span.end;
        Ok(Expr {
            kind: Kind::Cmp(cmp, Box::new(left), Box::new(right)),
            span,
        })
    }

    fn sum(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.product()?;
        loop {
            let op = match self.peek_op() {
                Some(Op::Plus) => Arith::Add,
                Some(Op::Minus) => Arith::Sub,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.product()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::Arith(op, Box::new(left), Box::new(right)),
                span,
            };
        }
    }

    fn product(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.negation()?;
        loop {
            let op = match self.peek_op() {
                Some(Op::Star) => Arith::Mul,
                Some(Op::Slash) => Arith::Div,
                _ => return Ok(left),
            };
            self.pos += 1;
            let right = self.negation()?;
            let span = left.span.start..right.span.end;
            left = Expr {
                kind: Kind::Arith(op, Box::new(left), Box::new(right)),
                span,
            };
        }
    }

    fn negation(&mut self) -> Result<Expr, ExprError> {
        match self.peek_op() {
            Some(Op::Minus) => {
                let s = self.span_here();
                self.enter(s.clone())?;
                self.pos += 1;
                let inner = self.negation()?;
                self.depth -= 1;
                let span = s.start..inner.span.end;
                Ok(Expr {
                    kind: Kind::Neg(Box::new(inner)),
                    span,
                })
            }
            Some(Op::Plus) => Err(ExprError::new(
                formula::SYNTAX,
                self.at(self.span_here()),
                "there is no unary `+`",
                "remove the `+`",
            )),
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, ExprError> {
        let Some(t) = self.tokens.get(self.pos).cloned() else {
            return Err(self.unexpected("an expression"));
        };
        match t.tok {
            Tok::Num(v) => {
                self.pos += 1;
                Ok(Expr {
                    kind: Kind::Num(v),
                    span: t.span,
                })
            }
            Tok::Sel(s) => {
                self.pos += 1;
                Ok(Expr {
                    kind: Kind::Sel(s),
                    span: t.span,
                })
            }
            Tok::Op(Op::LParen) => {
                self.enter(t.span.clone())?;
                self.pos += 1;
                let mut inner = self.expr()?;
                let close = self.expect(Op::RParen, "`)`")?;
                self.depth -= 1;
                // The operand is the whole `( ... )`, so a fault at an operation on it is located
                // and quoted with both parentheses (formulas.md, "Every failure is located").
                inner.span = t.span.start..close.end;
                Ok(inner)
            }
            Tok::Ident(name) => {
                self.pos += 1;
                match self.peek_op() {
                    Some(Op::LParen) => self.call(name, t.span),
                    Some(Op::Dot) => {
                        self.pos += 1;
                        match self.tokens.get(self.pos).cloned() {
                            Some(Token {
                                tok: Tok::Ident(attr),
                                span,
                            }) => {
                                self.pos += 1;
                                if self.peek_op() == Some(Op::LParen) {
                                    return Err(self.unexpected("an operator"));
                                }
                                Ok(Expr {
                                    kind: Kind::Member(name, attr),
                                    span: t.span.start..span.end,
                                })
                            }
                            _ => Err(self.unexpected("an attribute name after `.`")),
                        }
                    }
                    _ => Ok(Expr {
                        kind: Kind::Name(name),
                        span: t.span,
                    }),
                }
            }
            _ => Err(self.unexpected("an expression")),
        }
    }

    fn call(&mut self, name: String, name_span: Range<u32>) -> Result<Expr, ExprError> {
        self.enter(name_span.clone())?;
        self.pos += 1; // `(`
        let mut args = Vec::new();
        if self.peek_op() == Some(Op::RParen) {
            let end = self.span_here().end;
            self.pos += 1;
            self.depth -= 1;
            return Ok(Expr {
                kind: Kind::Call {
                    name,
                    name_span: name_span.clone(),
                    args,
                },
                span: name_span.start..end,
            });
        }
        loop {
            let named = match (
                self.tokens.get(self.pos),
                self.tokens.get(self.pos + 1).map(|t| &t.tok),
            ) {
                (
                    Some(Token {
                        tok: Tok::Ident(n),
                        span,
                    }),
                    Some(Tok::Op(Op::Assign)),
                ) => Some((n.clone(), span.clone())),
                _ => None,
            };
            if named.is_some() {
                self.pos += 2;
            }
            let value = self.expr()?;
            args.push(Arg { name: named, value });
            match self.peek_op() {
                Some(Op::Comma) => self.pos += 1,
                Some(Op::RParen) => {
                    let end = self.span_here().end;
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Expr {
                        kind: Kind::Call {
                            name,
                            name_span: name_span.clone(),
                            args,
                        },
                        span: name_span.start..end,
                    });
                }
                _ => return Err(self.unexpected("`,` or `)`")),
            }
        }
    }
}
