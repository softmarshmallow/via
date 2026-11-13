//! The names a world declares (`formulas.md` 5): `let` names by scope, kind attributes, features
//! and their attributes, and field names. The document model fills one [`Env`] after merge, and
//! every formula is checked against it.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::codes::formula;

use crate::context::{Context, Key, RESERVED};
use crate::error::{At, ExprError, FormulaId, Scope};
use crate::parse::{parse, Expr};
use crate::program::Program;

/// The type of a declared attribute (`formulas.md` 6.7): its kind default's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrType {
    /// A number or a formula giving one.
    Number,
    /// A bool.
    YesNo,
    Text,
    Array,
    Table,
}

impl AttrType {
    pub fn word(self) -> &'static str {
        match self {
            AttrType::Number => "a number",
            AttrType::YesNo => "a bool",
            AttrType::Text => "text",
            AttrType::Array => "an array",
            AttrType::Table => "a table",
        }
    }
}

/// A registered formula string.
#[derive(Clone, Debug)]
pub(crate) enum Source {
    /// A string value and its parse.
    Text {
        value: String,
        parsed: Result<Expr, ExprError>,
    },
    /// A TOML number in a formula key: a constant formula.
    Number(f64),
}

/// A kind's declarations.
#[derive(Clone, Debug, Default)]
pub struct KindDecl {
    pub attributes: BTreeMap<String, AttrType>,
    pub lets: BTreeMap<String, FormulaId>,
    /// The kind's `radius` in metres, the default cap of its `let`s (8.4).
    pub radius: Option<f64>,
}

/// A feature's declarations: its kind and its attributes after its kind's defaults.
#[derive(Clone, Debug, Default)]
pub struct FeatureDecl {
    pub kind: Option<String>,
    pub attributes: BTreeMap<String, AttrType>,
}

/// Everything formulas can name in one world.
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub(crate) sources: Vec<Source>,
    pub(crate) world_lets: BTreeMap<String, FormulaId>,
    pub(crate) field_lets: BTreeMap<String, BTreeMap<String, FormulaId>>,
    pub(crate) kinds: BTreeMap<String, KindDecl>,
    pub(crate) features: BTreeMap<String, FeatureDecl>,
    pub(crate) fields: BTreeSet<String>,
    /// Kind `let` names also given as attributes, in declaration order, for `formula.shadow`.
    duplicates: Vec<(String, String)>,
}

impl Env {
    pub fn new() -> Env {
        Env::default()
    }

    /// Registers a formula string (its leading `=` included) and parses it. Marking, size and
    /// syntax errors come back from [`Env::check`].
    pub fn text(&mut self, value: &str) -> FormulaId {
        let id = FormulaId(self.sources.len() as u32);
        let parsed = parse(id, value);
        self.sources.push(Source::Text {
            value: value.to_string(),
            parsed,
        });
        id
    }

    /// Registers a TOML number written in a formula key: a constant formula.
    pub fn number(&mut self, value: f64) -> FormulaId {
        let id = FormulaId(self.sources.len() as u32);
        self.sources.push(Source::Number(value));
        id
    }

    /// The string value of a registered formula, if it is text.
    pub fn value(&self, id: FormulaId) -> Option<&str> {
        match self.sources.get(id.0 as usize) {
            Some(Source::Text { value, .. }) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn source(&self, id: FormulaId) -> Option<&Source> {
        self.sources.get(id.0 as usize)
    }

    /// Declares a field.
    pub fn field(&mut self, name: &str) {
        self.fields.insert(name.to_string());
    }

    /// Declares a world `let`.
    pub fn world_let(&mut self, name: &str, formula: FormulaId) {
        self.world_lets.insert(name.to_string(), formula);
    }

    /// Declares a field's `let`.
    pub fn field_let(&mut self, field: &str, name: &str, formula: FormulaId) {
        self.field_lets
            .entry(field.to_string())
            .or_default()
            .insert(name.to_string(), formula);
    }

    /// Declares a kind attribute (from `defaults` or `requires`).
    pub fn kind_attribute(&mut self, kind: &str, name: &str, ty: AttrType) {
        let k = self.kinds.entry(kind.to_string()).or_default();
        if k.lets.contains_key(name) {
            self.duplicates.push((kind.to_string(), name.to_string()));
        }
        k.attributes.insert(name.to_string(), ty);
    }

    /// Declares a kind `let`.
    pub fn kind_let(&mut self, kind: &str, name: &str, formula: FormulaId) {
        let k = self.kinds.entry(kind.to_string()).or_default();
        if k.attributes.contains_key(name) {
            self.duplicates.push((kind.to_string(), name.to_string()));
        }
        k.lets.insert(name.to_string(), formula);
    }

    /// Declares a kind's `radius`, in metres.
    pub fn kind_radius(&mut self, kind: &str, radius: f64) {
        self.kinds.entry(kind.to_string()).or_default().radius = Some(radius);
    }

    /// Declares a feature of a kind, or of none.
    pub fn feature(&mut self, id: &str, kind: Option<&str>) {
        let f = self.features.entry(id.to_string()).or_default();
        f.kind = kind.map(str::to_string);
    }

    /// Declares an attribute of a feature (its kind's defaults included, as the caller merged).
    pub fn feature_attribute(&mut self, id: &str, name: &str, ty: AttrType) {
        self.features
            .entry(id.to_string())
            .or_default()
            .attributes
            .insert(name.to_string(), ty);
    }

    /// Checks a formula in a context: marking, syntax, names, types, rates, arguments, selector
    /// form and class, seed form, missing cap, ranges of constant parameters (`formulas.md` 1,
    /// the text phase).
    pub fn check(&self, formula: FormulaId, ctx: &Context) -> Result<Program, ExprError> {
        crate::check::compile(self, formula, ctx)
    }

    /// `formula.shadow` (`formulas.md` 5): a `let` or attribute named like a built-in name or a
    /// keyword, a kind attribute that is also a kind `let`, and a `let` named like a field. In
    /// a fixed order: scopes world, fields, kinds, features, names in byte order.
    pub fn shadows(&self) -> Vec<ExprError> {
        let mut out = Vec::new();
        let reserved = |scope: Scope, name: &str, what: &str, out: &mut Vec<ExprError>| {
            if RESERVED.contains(&name) {
                out.push(ExprError::new(
                    formula::SHADOW,
                    At::Declaration {
                        scope: scope.clone(),
                        name: name.to_string(),
                    },
                    format!(
                        "{what} `{name}` in {scope} has the name of a built-in name or keyword"
                    ),
                    format!("rename `{name}`; built-in names and keywords are reserved"),
                ));
            }
        };
        let field_named = |scope: Scope, name: &str, out: &mut Vec<ExprError>| {
            if self.fields.contains(name) {
                out.push(ExprError::new(
                    formula::SHADOW,
                    At::Declaration {
                        scope,
                        name: name.to_string(),
                    },
                    format!("the `let` `{name}` has the name of a field"),
                    format!("rename the `let`; read the field with `f('{name}')`"),
                ));
            }
        };
        for name in self.world_lets.keys() {
            reserved(Scope::World, name, "the `let`", &mut out);
            field_named(Scope::World, name, &mut out);
        }
        for (field, lets) in &self.field_lets {
            for name in lets.keys() {
                reserved(Scope::Field(field.clone()), name, "the `let`", &mut out);
                field_named(Scope::Field(field.clone()), name, &mut out);
            }
        }
        for (kind, k) in &self.kinds {
            for name in k.attributes.keys() {
                reserved(Scope::Kind(kind.clone()), name, "the attribute", &mut out);
            }
            for name in k.lets.keys() {
                reserved(Scope::Kind(kind.clone()), name, "the `let`", &mut out);
                field_named(Scope::Kind(kind.clone()), name, &mut out);
            }
        }
        for (id, f) in &self.features {
            if f.kind.is_none() {
                for name in f.attributes.keys() {
                    reserved(Scope::Kind(id.clone()), name, "the attribute", &mut out);
                }
            }
        }
        let mut dup: Vec<&(String, String)> = self.duplicates.iter().collect();
        dup.sort();
        dup.dedup();
        for (kind, name) in dup {
            out.push(ExprError::new(
                formula::SHADOW,
                At::Declaration {
                    scope: Scope::Kind(kind.clone()),
                    name: name.clone(),
                },
                format!("kind `{kind}` declares `{name}` both as an attribute and as a `let`"),
                format!("rename one of the two `{name}`s"),
            ));
        }
        out
    }

    /// Checks every `let`, read or not, against its own table's widest context (`formulas.md` 5):
    /// 6.2 for the world's, 6.3 for a field's, 6.5 for a kind's. The first error of each.
    pub fn check_lets(&self) -> Vec<ExprError> {
        let mut out = Vec::new();
        for (name, id) in &self.world_lets {
            let ctx = Context::new(Key::WorldLet).noise(&["noise", "let", name]);
            if let Err(e) = self.check(*id, &ctx) {
                out.push(e);
            }
        }
        for (field, lets) in &self.field_lets {
            for (name, id) in lets {
                let ctx = Context::new(Key::FieldLet)
                    .field(field)
                    .noise(&["noise", "field", field, "let", name]);
                if let Err(e) = self.check(*id, &ctx) {
                    out.push(e);
                }
            }
        }
        for (kind, k) in &self.kinds {
            for (name, id) in &k.lets {
                let mut ctx = Context::new(Key::KindLet).kind(kind).noise_parts(vec![
                    crate::NoisePart::Text("noise".into()),
                    crate::NoisePart::Text("feature".into()),
                    crate::NoisePart::Feature,
                    crate::NoisePart::Text("let".into()),
                    crate::NoisePart::Text(name.clone()),
                ]);
                ctx.radius = k.radius;
                if let Err(e) = self.check(*id, &ctx) {
                    out.push(e);
                }
            }
        }
        out
    }
}

/// `formula.seed_shared` (`formulas.md` 8.2): among all the formulas of one owner, two distinct
/// noise calls on one stream. A `let` read twice is one call. Warnings, in call order.
pub fn seed_shared(programs: &[&Program]) -> Vec<ExprError> {
    let mut first: BTreeMap<&crate::program::Seed, &At> = BTreeMap::new();
    let mut seen: BTreeSet<&At> = BTreeSet::new();
    let mut out = Vec::new();
    for p in programs {
        for (seed, at) in &p.noise {
            if !seen.insert(at) {
                continue;
            }
            match first.get(seed) {
                None => {
                    first.insert(seed, at);
                }
                Some(earlier) => {
                    let mut e = ExprError::new(
                        formula::SEED_SHARED,
                        at.clone(),
                        "two noise calls of one owner use one stream: they are scaled copies of one pattern",
                        "give one of them its own `seed = '<name>'`",
                    );
                    e.related.push((
                        (*earlier).clone(),
                        "the other call on the stream".to_string(),
                    ));
                    out.push(e);
                }
            }
        }
    }
    out
}
