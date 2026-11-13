//! Located errors of the formula language (`formulas.md` 9, "Every failure is located").
//!
//! `via-expr` sits below the document reader, so it cannot name a file or a line. An
//! [`ExprError`] says where the fault is in terms of the formula it was found in: a byte range of
//! that formula's string value, its leading `=` included ([`At::Expr`]), the whole key
//! ([`At::Key`]), or a declaration ([`At::Declaration`]). The caller, which knows each formula's
//! subject and key path, turns it into a [`via_diag::Diag`] with [`ExprError::to_diag`]; the
//! document reader maps the bytes to the file through the string's span map (`documents.md` 2.1).

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use via_diag::{Code, Diag, Position, Related, Site, Subject, Text, Value};

/// A formula registered in an [`crate::Env`], numbered in registration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormulaId(pub u32);

/// A naming scope (`formulas.md` 5): the world's, a field's or a kind's.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    /// The world's `[let]`.
    World,
    /// `[field.<n>.let]`.
    Field(String),
    /// A kind's attributes and `[kind.<k>.let]`.
    Kind(String),
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scope::World => f.write_str("the world's [let]"),
            Scope::Field(n) => write!(f, "field `{n}`"),
            Scope::Kind(k) => write!(f, "kind `{k}`"),
        }
    }
}

/// Where a fault is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum At {
    /// Bytes `[start, end)` of a formula's string value, its leading `=` included.
    Expr {
        /// The formula.
        formula: FormulaId,
        /// The bytes of the sub-expression at fault.
        span: Range<u32>,
    },
    /// A formula's whole key (a constant number, or a value as a whole).
    Key {
        /// The formula.
        formula: FormulaId,
    },
    /// A declaration: a `let` or an attribute named `name` in `scope`.
    Declaration {
        /// Where it is declared.
        scope: Scope,
        /// The declared name.
        name: String,
    },
}

impl At {
    fn key(&self) -> (u8, u32, u32, u32, Option<(&Scope, &str)>) {
        match self {
            At::Expr { formula, span } => (0, formula.0, span.start, span.end, None),
            At::Key { formula } => (1, formula.0, 0, 0, None),
            At::Declaration { scope, name } => (2, 0, 0, 0, Some((scope, name.as_str()))),
        }
    }

    /// The formula this place is in, if any.
    pub fn formula(&self) -> Option<FormulaId> {
        match self {
            At::Expr { formula, .. } | At::Key { formula } => Some(*formula),
            At::Declaration { .. } => None,
        }
    }
}

impl Ord for At {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for At {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// An error of the formula language, located in formula terms. Its facts sit behind one box,
/// so a `Result<_, ExprError>` stays a pointer wide on its error side; they are read and set
/// through `Deref` as [`Detail`]'s fields.
#[derive(Clone, Debug, PartialEq)]
pub struct ExprError(Box<Detail>);

/// The facts of an [`ExprError`].
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    /// The registry code.
    pub code: Code,
    /// What is wrong.
    pub message: String,
    /// What to change.
    pub hint: String,
    /// Where.
    pub at: At,
    /// The formula whose key is the record's subject: the reading formula for a fault inside a
    /// `let` (`formulas.md` 9). `None` for a declaration.
    pub subject: Option<FormulaId>,
    /// Other places involved, with a note each: for a fault in a `let`, the reading formula.
    pub related: Vec<(At, String)>,
    /// The post or point, in world millimetres, for a value-phase fault.
    pub position: Option<(i64, i64)>,
    /// `data` facts the code's registry row lists (`nearest`).
    pub data: BTreeMap<String, Value>,
}

impl std::ops::Deref for ExprError {
    type Target = Detail;
    fn deref(&self) -> &Detail {
        &self.0
    }
}

impl std::ops::DerefMut for ExprError {
    fn deref_mut(&mut self) -> &mut Detail {
        &mut self.0
    }
}

impl ExprError {
    /// An error at `at` whose subject is the formula `at` is in.
    pub fn new(code: Code, at: At, message: impl Into<String>, hint: impl Into<String>) -> Self {
        ExprError(Box::new(Detail {
            code,
            message: message.into(),
            hint: hint.into(),
            subject: at.formula(),
            at,
            related: Vec::new(),
            position: None,
            data: BTreeMap::new(),
        }))
    }

    /// The registry code as text, `formula.syntax`.
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    /// The same error with one `data` fact.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.data.insert(key.to_string(), value.into());
        self
    }

    /// The byte range, when the place is a sub-expression.
    pub fn span(&self) -> Option<Range<u32>> {
        match &self.at {
            At::Expr { span, .. } => Some(span.clone()),
            _ => None,
        }
    }

    /// The diagnostic. `subject_of` gives a formula's key address; `site_of` gives the site of a
    /// place, relative to the record's subject (the document reader knows both).
    pub fn to_diag(
        &self,
        subject_of: &dyn Fn(Option<FormulaId>, &At) -> Subject,
        site_of: &dyn Fn(&At) -> Site,
    ) -> Diag {
        let message = Text::new(self.message.clone())
            .unwrap_or_else(|_| via_diag::text!("a formula is not valid"));
        let hint = Text::new(self.hint.clone())
            .unwrap_or_else(|_| via_diag::text!("see the formula language, spec/formulas.md"));
        let mut d = Diag::new(
            self.code,
            subject_of(self.subject, &self.at),
            site_of(&self.at),
            message,
            hint,
        );
        if let Some((x, y)) = self.position {
            d = d.at(Position::plan(x, y));
        }
        for (k, v) in &self.data {
            d = d.with(k.clone(), v.clone());
        }
        for (at, note) in &self.related {
            if let Ok(note) = Text::new(note.clone()) {
                d = d.related(Related {
                    subject: subject_of(at.formula(), at),
                    site: site_of(at),
                    position: None,
                    note,
                });
            }
        }
        d
    }
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.code.as_str(),
            self.message,
            self.hint
        )
    }
}

impl std::error::Error for ExprError {}

/// A number for a message: the shortest text that reads back (`numbers.md` 3.4), or the name of
/// a non-finite value.
pub(crate) fn num_text(x: f64) -> String {
    if x.is_nan() {
        "NaN".to_string()
    } else if x == f64::INFINITY {
        "infinity".to_string()
    } else if x == f64::NEG_INFINITY {
        "-infinity".to_string()
    } else if x == 0.0 && x.is_sign_negative() {
        "-0".to_string()
    } else {
        via_num::write_double(x).unwrap_or_else(|_| format!("{x}"))
    }
}
