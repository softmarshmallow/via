//! The merged world read into declarations: fields and their rows (`fields.md` 1 to 5), kinds
//! and effects (`authority.md` 1 and 2), features (`geometry.md` 2), instances, scatters and
//! hand edits (`rules.md` 2, 3, 7; `identity.md` 4). Formulas are registered and checked here,
//! once, against one [`via_expr::Env`] (`formulas.md`, the text phase).
//!
//! Everything a later stage reads is in these tables; nothing later reads the document again.
//! Faults found here are diagnostics on the declaration; a declaration that cannot be used is
//! left out, and what depends on it is not built (it says so through its own diagnostic).

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Diag, KeyPath, Site, Subject};
use via_doc::{Table, Value, World};
use via_expr::{AttrType, Context, Env, FormulaId, Key, NoisePart, Program};
use via_num::Grid;

use crate::text::{self, at_header, at_key, at_path, FormulaText};

/// The three tiers (`authority.md` 3), ordered as entries are applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Derived,
    Shaped,
    Anchored,
}

impl Tier {
    pub fn parse(s: &str) -> Option<Tier> {
        match s {
            "derived" => Some(Tier::Derived),
            "shaped" => Some(Tier::Shaped),
            "anchored" => Some(Tier::Anchored),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Tier::Derived => "derived",
            Tier::Shaped => "shaped",
            Tier::Anchored => "anchored",
        }
    }
}

/// A field's role (`fields.md` 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Surface,
    Material,
    KeepOut,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "surface" => Some(Role::Surface),
            "material" => Some(Role::Material),
            "keep_out" => Some(Role::KeepOut),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Role::Surface => "surface",
            Role::Material => "material",
            Role::KeepOut => "keep_out",
        }
    }
}

/// A `then` row (`fields.md` 5).
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub value: Option<FormulaId>,
    pub mask: Option<FormulaId>,
    pub paint: Option<String>,
    pub order: i64,
    pub rank: i64,
}

/// A field (`fields.md` 1).
#[derive(Clone, Debug)]
pub struct FieldDecl {
    pub name: String,
    pub role: Option<Role>,
    pub weights: bool,
    pub unit: Option<String>,
    pub tags: Vec<String>,
    pub rank: i64,
    pub base: Option<FormulaId>,
    /// A base from an array file (`fields.md` 4), in place of `base`.
    pub file: Option<FileDecl>,
    pub rows: Vec<Row>,
}

/// A field's `file` and its keys (`fields.md` 4.2).
#[derive(Clone, Debug)]
pub struct FileDecl {
    /// The path as written, relative to the document that names it.
    pub written: String,
    /// That document as a record names it (`documents.md` 2.1): a path under the world's
    /// folder, or a tagged form (`lib:<id>/...`).
    pub source: String,
    /// Where the raster's corner samples lie; `None` for the world's extent.
    pub extent: Option<[i64; 4]>,
    pub outside: f64,
    pub filter: via_raster::file::Filter,
    /// A PNG's `channel` (`grey`, `r`, `g`, `b`, `a`), as written (`fields.md` 4.2).
    pub channel: Option<String>,
    /// A PNG's `range`, `[lo, hi]`: what level 0 and the top level stand for.
    pub range: Option<(f64, f64)>,
}

/// An effect's field selector (`authority.md` 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    Role(Role),
    Tag(String),
}

/// One effect of a kind.
#[derive(Clone, Debug)]
pub struct Effect {
    pub id: String,
    pub selector: Selector,
    pub value: Option<FormulaId>,
    pub mask: Option<FormulaId>,
    pub paint: Option<String>,
    /// Millimetres; the effect's own, else the kind's.
    pub radius: Option<i64>,
    pub max_change: Option<FormulaId>,
    pub order: i64,
}

/// An attribute value (`authority.md` 1.1).
#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Number(f64),
    YesNo(bool),
    Text(String),
}

/// A kind (`authority.md` 1).
#[derive(Clone, Debug, Default)]
pub struct Kind {
    pub id: String,
    pub authority: Option<Tier>,
    pub defaults: BTreeMap<String, Attr>,
    pub requires: Vec<String>,
    pub names: Vec<String>,
    pub radius: Option<i64>,
    pub effects: BTreeMap<String, Effect>,
    pub output: Option<String>,
    /// `exit` (`composition.md` 7): the attributes an answer must match; empty for `true`;
    /// `None` when the kind marks no exit.
    pub exit: Option<Vec<String>>,
}

/// A place written as `on = { ref, t, offset }`.
#[derive(Clone, Debug, PartialEq)]
pub struct On {
    pub reference: String,
    pub t: Option<f64>,
    pub offset: Option<i64>,
}

/// A written facing (`rules.md` 3.3, `geometry.md` 2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Bearing(i64),
    Along,
    Toward,
    Away,
}

/// A selection by place, `on = { of, class, near, t, offset }` (`identity.md` 3).
#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    /// The maker's id.
    pub of: String,
    /// The class asked for; `None` when the maker has one class.
    pub class: Option<String>,
    /// The point near the wanted thing, millimetres in the world frame.
    pub near: (i64, i64),
    pub t: Option<f64>,
    pub offset: Option<i64>,
}

/// A placement: a coordinate, a place on a named thing, or a produced thing selected by place.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    At((i64, i64)),
    On(On),
    Of(Select),
}

/// A path along another feature, `follows = { ref, span, offset }` (`geometry.md` 4).
#[derive(Clone, Debug, PartialEq)]
pub struct Follows {
    pub reference: String,
    pub span: (f64, f64),
    /// Millimetres to the left of the reference's travel.
    pub offset: i64,
}

/// One way point of a written path (`derived.md` 3): a point, or a named place.
#[derive(Clone, Debug, PartialEq)]
pub enum WayDecl {
    /// A point, millimetres.
    At((i64, i64)),
    /// A named place, as `starts_at` takes one (`geometry.md` 2.7).
    Name(String),
}

/// A feature's written shape.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeDecl {
    Point(Place),
    Path(Vec<WayDecl>),
    Region(Vec<(i64, i64)>),
    /// A region written as a condition (`geometry.md` 3.7): the formula.
    Where(FormulaId),
    Follows(Follows),
}

/// A feature (`geometry.md` 2.1).
#[derive(Clone, Debug)]
pub struct FeatureDecl {
    pub id: String,
    pub kind: Option<String>,
    pub tier: Option<Tier>,
    pub rank: i64,
    pub shape: ShapeDecl,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub face: Option<Face>,
    pub attrs: BTreeMap<String, Attr>,
    /// `smooth = true` on a path (`geometry.md` 2.4).
    pub smooth: bool,
    /// `holds` beside `where` (`derived.md` 2): the point whose piece is kept, millimetres.
    pub holds: Option<(i64, i64)>,
    /// `route` on a path (`derived.md` 3): the cost per metre, a formula over posts.
    pub route: Option<FormulaId>,
}

impl FeatureDecl {
    /// The named way points of a written path, in order (`derived.md` 3).
    pub fn way_names(&self) -> Vec<&str> {
        match &self.shape {
            ShapeDecl::Path(ws) => ws
                .iter()
                .filter_map(|w| match w {
                    WayDecl::Name(n) => Some(n.as_str()),
                    WayDecl::At(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// A hand instance (`rules.md` 2).
#[derive(Clone, Debug)]
pub struct InstanceDecl {
    #[allow(dead_code)] // the id the table is keyed by; kept beside its facts
    pub id: String,
    pub asset: String,
    pub place: Place,
    pub face: Option<Face>,
    pub scale: f64,
    pub tint: f64,
    pub ground: Option<String>,
    pub margin: i64,
    pub rank: i64,
    /// The set it places (`rules.md` 4); then `asset` is empty.
    pub set: Option<String>,
}

/// One choice of a scatter's `assets` (`rules.md` 7.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// An asset id, or `@<taxon>`.
    pub what: String,
    /// Millionths.
    pub weight: i64,
}

/// A number or a drawn range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ranged {
    Fixed(f64),
    Range(f64, f64),
}

/// A scatter's `clump` (`rules.md` 7.3).
#[derive(Clone, Debug, PartialEq)]
pub enum Clump {
    /// Each kept candidate a parent of `lo..=hi` members within `radius` millimetres.
    Members {
        lo: i64,
        hi: i64,
        radius: i64,
        shrink: f64,
    },
    /// The keep chance times `max(0, 1 - d / radius)`, `d` to the nearest of `of`'s output.
    Around { of: String, radius: i64 },
}

/// A scatter (`rules.md` 7).
#[derive(Clone, Debug)]
pub struct ScatterDecl {
    pub id: String,
    pub choices: Vec<Choice>,
    pub spacing: i64,
    pub jitter: i64,
    pub density: Option<FormulaId>,
    pub within: Option<String>,
    /// `within = { of, class, near }`: a piece of a `where` region (`identity.md` 3).
    pub within_of: Option<Select>,
    pub scale: Ranged,
    pub tint: Ranged,
    pub seed: String,
    /// Layer rank: giving way reads down only (`rules.md` 3.4).
    pub rank: i64,
    pub clump: Option<Clump>,
}

/// A pin (`identity.md` 4.2).
#[derive(Clone, Debug)]
pub struct PinDecl {
    pub id: String,
    pub asset: String,
    pub place: Place,
    pub face: Option<Face>,
    pub by: Option<Vec<String>>,
    pub rank: i64,
}

/// A clear (`identity.md` 4.3).
#[derive(Clone, Debug)]
pub struct ClearDecl {
    pub id: String,
    pub place: Option<Place>,
    pub region: Option<Vec<(i64, i64)>>,
    pub radius: Option<i64>,
    pub by: Option<Vec<String>>,
    #[allow(dead_code)] // layer rank: giving way and hand edits read down only (`rules.md` 3.4)
    pub rank: i64,
}

/// A step (`protocol.md`): M1 runs none in the model; it is listed for the caller.
#[derive(Clone, Debug)]
pub struct StepDecl {
    #[allow(dead_code)] // the id the table is keyed by; kept beside its facts
    pub id: String,
    pub when: String,
    pub command: Vec<String>,
}

/// Every formula, with where it was written.
#[derive(Debug, Default)]
pub struct Formulas {
    pub env: Env,
    /// Per formula id: the entity and key path holding it.
    pub at: Vec<(Subject, KeyPath)>,
    pub programs: BTreeMap<FormulaId, Program>,
    /// Per kind: the attributes its `defaults` and its `requires` declare, to locate a
    /// declaration a record points at.
    pub kind_attrs: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>,
}

impl Formulas {
    fn add(&mut self, ft: &FormulaText, subject: Subject, path: KeyPath) -> FormulaId {
        let id = match ft {
            FormulaText::Text(t) => self.env.text(t),
            FormulaText::Number(n) => self.env.number(*n),
        };
        debug_assert_eq!(id.0 as usize, self.at.len());
        // A formula written in a step's returned document reads the document's own keys first
        // (`protocol.md` 5, Names): its entity, or the row under a field, has a produced name.
        let address = subject.clone().with_keys(&path).to_string();
        if let Some(step) = address
            .split('.')
            .find_map(|part| part.trim_matches('"').split_once('#').map(|(s, _)| s))
            .filter(|s| !s.is_empty())
        {
            self.env.scope(id, step);
        }
        self.at.push((subject, path));
        id
    }

    /// The diagnostic of a formula fault, located at the formula's key.
    pub fn diag(&self, e: &via_expr::ExprError) -> Diag {
        let subject_of = |f: Option<FormulaId>, at: &via_expr::At| -> Subject {
            // A fault inside a `let` is the `let`'s: its address, its span, and the reading
            // formula in `related` (`formulas.md` 9), so two readers give one record.
            match at.formula().or(f) {
                // One key of an entity: the address and the key path (`diagnostics.md` 1.1).
                Some(f) => self
                    .at
                    .get(f.0 as usize)
                    .map(|(s, p)| match s.key_path() {
                        // A `let`'s address is the whole statement (`let.slope`).
                        None if matches!(s, Subject::Entity { collection, .. } if collection == "let") => {
                            s.clone()
                        }
                        None => s.clone().with_keys(p),
                        Some(_) => s.clone(),
                    })
                    .unwrap_or(Subject::World),
                None => match at {
                    via_expr::At::Declaration { scope, name } => self.declaration(scope, name).0,
                    _ => Subject::World,
                },
            }
        };
        let site_of = |at: &via_expr::At| -> Site {
            match at {
                via_expr::At::Expr { formula, span } => match self.at.get(formula.0 as usize) {
                    Some((_, p)) if self.env.value(*formula).is_some() => {
                        Site::Expr(p.clone(), span.clone())
                    }
                    Some((_, p)) => Site::Key(p.clone()),
                    None => Site::World,
                },
                via_expr::At::Key { formula } => match self.at.get(formula.0 as usize) {
                    Some((_, p)) => Site::Key(p.clone()),
                    None => Site::World,
                },
                via_expr::At::Declaration { scope, name } => self.declaration(scope, name).1,
            }
        };
        e.to_diag(&subject_of, &site_of)
    }

    /// The statement and site that declare a `let` or attribute (`via_expr::At::Declaration`):
    /// a world `let` is its own statement; a field's or kind's `let` is the key under its `let`
    /// table; a kind's attribute is its `defaults` key, else its `requires`; a kindless feature's
    /// attribute is its own key.
    fn declaration(&self, scope: &via_expr::Scope, name: &str) -> (Subject, Site) {
        let key = |parts: &[&str]| {
            let mut p = KeyPath::key(parts[0]);
            for x in &parts[1..] {
                p = p.then(*x);
            }
            Site::Key(p)
        };
        match scope {
            via_expr::Scope::World => (Subject::entity("let", name), Site::Header),
            via_expr::Scope::Field(f) => {
                (Subject::entity("field", f.as_str()), key(&["let", name]))
            }
            via_expr::Scope::Kind(k) => match self.kind_attrs.get(k) {
                Some((defaults, _)) if defaults.contains(name) => (
                    Subject::entity("kind", k.as_str()),
                    key(&["defaults", name]),
                ),
                Some((_, requires)) if requires.contains(name) => {
                    (Subject::entity("kind", k.as_str()), key(&["requires"]))
                }
                Some(_) => (Subject::entity("kind", k.as_str()), Site::Header),
                None => (Subject::entity("feature", k.as_str()), key(&[name])),
            },
        }
    }
}

/// The `requires` line of a kind, for a record that names it (`authority.md` 1).
pub(crate) fn requires_line(kind: &str) -> via_diag::Related {
    via_diag::Related {
        subject: Subject::entity("kind", kind),
        site: Site::Key(KeyPath::key("requires")),
        position: None,
        note: via_diag::text!("the kind's `requires`"),
    }
}

/// The world read.
#[derive(Debug, Default)]
pub struct Decls {
    pub grid: Option<Grid>,
    pub seed: u64,
    pub pack: Option<String>,
    pub fields: BTreeMap<String, FieldDecl>,
    pub kinds: BTreeMap<String, Kind>,
    pub features: BTreeMap<String, FeatureDecl>,
    /// Every place name the world declares (`identity.md` 1): a feature, instance or pin id to
    /// its table, whether or not its own checks passed. A name whose declaration failed stays
    /// visible: a reference to it reads the failed entity, it is never `names nothing`.
    pub declared: BTreeMap<String, &'static str>,
    pub instances: BTreeMap<String, InstanceDecl>,
    /// Produced instances that gave way (`authority.md` 2.2), taken out of `instances`: what
    /// stands on one is not placed either, and says nothing.
    pub gave_way: BTreeSet<String>,
    pub scatters: BTreeMap<String, ScatterDecl>,
    /// `[set.<id>]` tables (`rules.md` 4), those that passed their checks.
    pub sets: BTreeMap<String, crate::sets::SetDecl>,
    /// Sets that failed their own checks (a cycle, no items): placing one says nothing more.
    pub failed_sets: BTreeSet<String>,
    /// `[run.<id>]` tables (`rules.md` 5).
    pub runs: BTreeMap<String, crate::sets::RunDecl>,
    pub pins: BTreeMap<String, PinDecl>,
    pub clears: BTreeMap<String, ClearDecl>,
    pub steps: BTreeMap<String, StepDecl>,
    /// `[network.<id>]` tables (`geometry.md` 8.1).
    pub networks: BTreeMap<String, crate::makers::NetworkDecl>,
    /// `[cells.<id>]` tables (`geometry.md` 8.2).
    pub cells: BTreeMap<String, crate::makers::CellsDecl>,
    pub formulas: Formulas,
    /// Attribute formulas (`formulas.md` 6.7), worked out once per feature after the text phase.
    pub attr_formulas: Vec<AttrFormula>,
    /// Attributes whose formula failed or could not be worked out: a reader says nothing more
    /// (`graph.md` 3.4), the formula's own record says why.
    pub failed_attrs: BTreeSet<(String, String)>,
    /// The layer rank each formula reads at; a kind's formulas read at the served feature's.
    pub formula_rank: BTreeMap<FormulaId, i64>,
    pub diags: Vec<Diag>,
    /// Fields whose base is read from an array file (`fields.md` 4): the samples, where they lie,
    /// and the digest of the file's bytes (a declared outside input, `graph.md`). Filled by the
    /// build, which knows the folders.
    pub files: BTreeMap<String, FileBase>,
    /// `datum` (`composition.md` 5).
    pub datum: Option<String>,
    /// `[import.<id>]` tables as written (`composition.md` 2).
    pub imports: BTreeMap<String, crate::import::ImportDecl>,
    /// Each import built alone and placed; filled by the build, which knows the folders.
    pub imported: BTreeMap<String, std::sync::Arc<crate::import::Imported>>,
    /// The scoped names formulas read as fields (`<import>.<field>`, section 4).
    pub scoped_fields: BTreeSet<String>,
    /// `[given.<name>]` (`composition.md` 8): what this world accepts from a parent.
    pub givens: BTreeMap<String, crate::import::GivenDecl>,
    /// The scoped place names the text writes, formula place arguments included
    /// (`composition.md` 4).
    pub scoped_places: BTreeSet<String>,
    /// Each given field's values from a parent, on this world's posts.
    pub given_fields: BTreeMap<String, FileBase>,
}

/// One attribute written as a formula: the feature's own text, or its kind's `defaults`.
#[derive(Clone, Debug)]
pub struct AttrFormula {
    pub feature: String,
    pub name: String,
    pub formula: FormulaId,
    /// True when the formula is the kind's default, not the feature's own text.
    pub from_kind: bool,
}

/// An attribute formula's reads: the attributes worked out so far.
struct AttrReads<'a> {
    features: &'a BTreeMap<String, FeatureDecl>,
    failed: &'a BTreeSet<(String, String)>,
    own: &'a str,
}

impl via_expr::Reads for AttrReads<'_> {
    fn attribute(
        &self,
        feature: Option<&str>,
        name: &str,
    ) -> Result<via_expr::Scalar, via_expr::ReadError> {
        let id = feature.unwrap_or(self.own);
        match self.features.get(id).and_then(|f| f.attrs.get(name)) {
            Some(Attr::Number(x)) => Ok(via_expr::Scalar::Number(*x)),
            Some(Attr::YesNo(b)) => Ok(via_expr::Scalar::YesNo(*b)),
            Some(Attr::Text(_)) => Err(via_expr::ReadError::new(
                codes::formula::TYPE,
                format!("`{name}` is text, not a number"),
                "read a number or bool attribute",
            )),
            None if self.failed.contains(&(id.to_string(), name.to_string())) => {
                Err(via_expr::ReadError::new(
                    codes::diag::SKIPPED,
                    format!("`{id}.{name}` could not be worked out"),
                    "see the attribute's own diagnostic",
                ))
            }
            None => Err(via_expr::ReadError::new(
                codes::diag::INTERNAL,
                format!("the attribute `{id}.{name}` is not known"),
                "report it: this is a defect of the engine",
            )),
        }
    }
}

/// An attribute formula selects nothing (`formulas.md` 6.7).
struct NoSelect;

impl via_expr::Select for NoSelect {
    fn fields(&self, _selector: &str) -> Vec<via_expr::FieldInfo> {
        Vec::new()
    }
    fn shapes(&self, _selector: &str) -> Vec<via_expr::ShapeInfo> {
        Vec::new()
    }
}

/// A field's base file, read.
#[derive(Clone, Debug)]
pub struct FileBase {
    pub samples: via_raster::npy::Samples,
    pub placement: via_raster::file::Placement,
    pub digest: String,
}

pub(crate) fn table_of<'w>(root: &'w Table, top: &str) -> Vec<(&'w str, &'w Table)> {
    match root.table(top) {
        Some(t) => t
            .iter()
            .filter_map(|(id, e)| match &e.item.value {
                Value::Table(t) => Some((id.as_str(), t)),
                _ => None,
            })
            .collect(),
        None => Vec::new(),
    }
}

fn kind_noise(feature_scope: &str, effect: &str, key: &str) -> Vec<NoisePart> {
    vec![
        NoisePart::Text("noise".into()),
        NoisePart::Text(feature_scope.into()),
        NoisePart::Feature,
        NoisePart::Text("effect".into()),
        NoisePart::Text(effect.into()),
        NoisePart::Text(key.into()),
    ]
}

impl Decls {
    /// Whether a name is something a generate step returned, `<step>#<key>` or a part of one
    /// (`<step>#<key>.ground`): what `authority.md` 4.4 leaves out of the views of every step at
    /// its layer and above. A `#` name another maker made (a scatter's instance) is not. The step
    /// view's `view_rank` (via-cli) holds the same rule; the two change together.
    pub fn step_returned(&self, name: &str) -> bool {
        name.split_once('#')
            .is_some_and(|(maker, _)| self.steps.contains_key(maker))
    }

    /// Whether a name is a step's return or a thing a `[network]` or `[cells]` table made: both
    /// stand outside the views of every step at their layer (`geometry.md` 8.1,
    /// Effects of a made region; `authority.md` 4.4, What counts as returned).
    pub fn stands_returned(&self, name: &str) -> bool {
        self.step_returned(name)
            || name
                .split_once('#')
                .is_some_and(|(maker, _)| self.is_maker_table(maker))
    }

    /// Whether an instance id is produced: a step returned it or a producer placed it
    /// (`<producer>#<key>`), not written by hand (`authority.md` 2.2, A produced instance gives
    /// way).
    pub fn produced_instance(&self, id: &str) -> bool {
        id.split_once('#')
            .is_some_and(|(m, _)| self.steps.contains_key(m) || self.scatters.contains_key(m))
    }

    /// The layer rank of a network or cells table.
    pub(crate) fn maker_table_rank(&self, id: &str) -> Option<i64> {
        self.networks
            .get(id)
            .map(|n| n.rank)
            .or_else(|| self.cells.get(id).map(|c| c.rank))
    }

    /// What a `where` reads (`authority.md` 2.3: "reads" is transitive): the fields, through the
    /// bases and rows of the fields it reads, and the shape selectors (feature ids, place names,
    /// `kind:<k>`). A shape node of a `where` region has an edge to each field's head view and
    /// to each shape (`graph.md`).
    pub fn where_reads(&self, formula: FormulaId) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut shapes = BTreeSet::new();
        let mut fields: BTreeSet<String> = BTreeSet::new();
        let mut stack = vec![formula];
        while let Some(fid) = stack.pop() {
            let Some(p) = self.formulas.programs.get(&fid) else {
                continue;
            };
            for (class, sel) in &p.reads.selectors {
                match class {
                    via_expr::Class::Field => {
                        for fd in self.fields.values() {
                            let hit = match sel.strip_prefix("tag:") {
                                Some(t) => fd.tags.iter().any(|x| x == t),
                                None => fd.name == *sel,
                            };
                            if hit && fields.insert(fd.name.clone()) {
                                stack.extend(fd.base);
                                for r in &fd.rows {
                                    stack.extend(r.value);
                                    stack.extend(r.mask);
                                }
                            }
                        }
                    }
                    via_expr::Class::Shape => {
                        shapes.insert(sel.clone());
                    }
                    _ => {}
                }
            }
        }
        (fields, shapes)
    }

    /// Reads the merged world.
    pub fn read(world: &World) -> Decls {
        Decls::read_given(world, None)
    }

    /// The world read, with what a parent gave it (`composition.md` 8); `None` builds every
    /// given from its default.
    pub fn read_given(world: &World, given: Option<&crate::import::GivenIn>) -> Decls {
        let mut d = Decls::default();
        let root = world.root();
        d.read_world_keys(root);
        d.read_kinds(root);
        d.read_fields(world, root);
        d.read_features(world, root);
        crate::import::read_givens(&mut d, root, given);
        d.read_sets(root);
        d.read_instances(world, root);
        d.read_runs(world, root);
        d.sets_unused();
        d.read_scatters(world, root);
        d.read_edits(world, root);
        d.read_steps(root);
        d.read_makers(world, root);
        crate::import::read_imports(&mut d, world, root);
        d.datum = root.str("datum").map(str::to_string);
        let ids: BTreeSet<String> = d.imports.keys().cloned().collect();
        d.scoped_fields = crate::import::scoped_field_names(root, &ids);
        d.scoped_places = crate::import::scoped_refs(root, &ids);
        d.declare_env();
        d.check_formulas(root);
        let upward = d.upward_reads(world);
        d.diags.extend(upward);
        d.eval_attributes();
        d
    }

    /// `documents.md` 5.1 and `formulas.md` 5: a formula reads downward only. A `let`, a field
    /// named outright, or `<id>.<attribute>` of a higher layer is `doc.upward_ref`, at the reading
    /// formula, naming both layers and owners. A kind has no rank: its formula is checked at the
    /// rank of each feature of the kind, one record with those features in `related`. Read
    /// selectors (`kind:`, `tag:`, shapes) are not named references: they leave out what lies
    /// above the reader when bound (`resolve::Sel`).
    pub fn upward_reads(&mut self, world: &World) -> Vec<Diag> {
        for (k, (subject, path)) in self.formulas.at.iter().enumerate() {
            if let Subject::Entity { collection, id, .. } = subject {
                if collection != "kind" {
                    let mut r = world.rank(collection, id).unwrap_or(0);
                    // A row a step returned under a lower layer's field has the step's rank
                    // (`protocol.md` 5, Rank): its row id is `<step>#<row>`.
                    if collection == "field" {
                        let row = path.to_string();
                        if let Some(step) = row
                            .split('.')
                            .find_map(|p| p.trim_matches('"').split_once('#').map(|(s, _)| s))
                        {
                            if let Some(sr) = world.rank("step", step) {
                                r = r.max(sr);
                            }
                        }
                    }
                    self.formula_rank.insert(FormulaId(k as u32), r);
                }
            }
        }
        let layer_words = |top: &str, id: &str| -> (i64, String) {
            let home = world.home(top, id).unwrap_or("world");
            let rank = world.rank(top, id).unwrap_or(0);
            let owner = world
                .layer_owner(home)
                .map_or("no owner".to_string(), |o| format!("owner `{o}`"));
            (rank, format!("layer `{home}` (rank {rank}, {owner})"))
        };
        let mut out = Vec::new();
        for (fid, p) in &self.formulas.programs {
            let Some((subject, path)) = self.formulas.at.get(fid.0 as usize) else {
                continue;
            };
            let Subject::Entity { collection, id, .. } = subject else {
                continue;
            };
            // The readers: the formula's own entity, or each feature of the kind.
            let readers: Vec<(i64, Option<String>)> = if collection == "kind" {
                self.features
                    .values()
                    .filter(|f| f.kind.as_deref() == Some(id.as_str()))
                    .map(|f| (f.rank, Some(f.id.clone())))
                    .collect()
            } else {
                vec![(self.formula_rank.get(fid).copied().unwrap_or(0), None)]
            };
            let mut targets: Vec<(String, String)> = Vec::new();
            for (scope, name) in &p.reads.lets {
                if scope == &via_expr::Scope::World.to_string() {
                    targets.push(("let".into(), name.clone()));
                }
            }
            for (fid2, _) in &p.reads.members {
                targets.push(("feature".into(), fid2.clone()));
            }
            for (class, sel) in &p.reads.selectors {
                if *class == via_expr::Class::Field
                    && !sel.starts_with("tag:")
                    && self.fields.contains_key(sel)
                {
                    targets.push(("field".into(), sel.clone()));
                }
            }
            targets.sort();
            targets.dedup();
            for (ttop, tid) in targets {
                let (trank, twords) = layer_words(&ttop, &tid);
                let low: Vec<&(i64, Option<String>)> =
                    readers.iter().filter(|(r, _)| *r < trank).collect();
                if low.is_empty() {
                    continue;
                }
                let (mine, reader) = if collection == "kind" {
                    let names: Vec<String> = low
                        .iter()
                        .filter_map(|(_, f)| f.as_ref().map(|f| format!("`feature.{f}`")))
                        .collect();
                    (
                        low.iter().map(|(r, _)| *r).max().unwrap_or(0),
                        format!(
                            "the kind `{id}`'s formula, serving {} at rank {}",
                            names.join(", "),
                            low.iter()
                                .map(|(r, _)| r.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                } else {
                    let (r, w) = layer_words(collection, id);
                    (r, format!("`{collection}.{id}` in {w}"))
                };
                let at = if collection == "let" {
                    subject.clone()
                } else {
                    subject.clone().with_keys(path)
                };
                let site = match self.formulas.env.value(*fid) {
                    Some(_) => Site::Expr(path.clone(), p.span.clone()),
                    None => Site::Key(path.clone()),
                };
                let mut d = text::diag(
                    codes::doc::UPWARD_REF,
                    at,
                    site,
                    &format!(
                        "{reader} reads `{ttop}.{tid}`, declared in {twords}; a layer refers downward only"
                    ),
                    &format!(
                        "move `{ttop}.{tid}` to rank {mine} or below, or move the reader to rank {trank} or above"
                    ),
                )
                .related(via_diag::Related {
                    subject: Subject::entity(&ttop, &tid),
                    site: if ttop == "let" {
                        Site::Key(KeyPath::key(&tid))
                    } else {
                        Site::Header
                    },
                    position: None,
                    note: via_diag::text!("the thing read, in a higher layer"),
                });
                for (_, f) in &low {
                    if let Some(f) = f {
                        d = d.related(via_diag::Related {
                            subject: Subject::entity("feature", f),
                            site: Site::Header,
                            position: None,
                            note: via_diag::text!("a feature of the kind, in a lower layer"),
                        });
                    }
                }
                out.push(d);
            }
        }
        out
    }

    pub(crate) fn push(&mut self, diag: Diag) {
        self.diags.push(diag);
    }

    fn world_key_fault(&mut self, key: &str, e: &via_num::NumError) {
        self.push(text::diag(
            text::num_code(e),
            Subject::World,
            Site::Key(KeyPath::key(key)),
            &e.to_string(),
            "write the world's extent and resolution on the lattice",
        ));
    }

    fn read_world_keys(&mut self, root: &Table) {
        let extent = match root.value("extent") {
            Some(Value::List(xs)) if xs.len() == 4 => {
                let mut v = [0i64; 4];
                let mut ok = true;
                for (k, x) in xs.iter().enumerate() {
                    match text::length(&x.value) {
                        Some(Ok(mm)) => v[k] = mm,
                        Some(Err(e)) => {
                            self.world_key_fault("extent", &e);
                            ok = false;
                        }
                        None => ok = false,
                    }
                }
                ok.then_some(v)
            }
            _ => None,
        };
        let resolution = match root.value("resolution") {
            Some(v) => match text::length(v) {
                Some(Ok(mm)) => Some(mm),
                Some(Err(e)) => {
                    self.world_key_fault("resolution", &e);
                    None
                }
                None => None,
            },
            None => Some(via_num::DEFAULT_RESOLUTION_MM),
        };
        if let (Some(e), Some(r)) = (extent, resolution) {
            match Grid::new(e, r) {
                Ok(g) => self.grid = Some(g),
                Err(err) => self.world_key_fault("extent", &err),
            }
        }
        if let Some(Value::Num(n)) = root.value("seed") {
            match n
                .norm
                .parse::<i64>()
                .map_err(|_| via_num::NumError::SeedType {
                    text: n.raw.clone(),
                    hint: None,
                }) {
                Ok(s) => match via_num::world_seed(s) {
                    Ok(s) => self.seed = s,
                    Err(e) => self.world_key_fault("seed", &e),
                },
                Err(e) => self.world_key_fault("seed", &e),
            }
        }
        self.pack = root.str("pack").map(str::to_string);
    }

    fn read_kinds(&mut self, root: &Table) {
        for (id, t) in table_of(root, "kind") {
            let mut k = Kind {
                id: id.to_string(),
                ..Kind::default()
            };
            if let Some(a) = t.str("authority") {
                match Tier::parse(a) {
                    Some(tier) => k.authority = Some(tier),
                    None => self.push(text::with_nearest(
                        at_key(
                            codes::authority::UNKNOWN,
                            "kind",
                            id,
                            "authority",
                            &format!("`{a}` is not a tier"),
                            &text::near_hint(
                                a,
                                ["derived", "shaped", "anchored"],
                                "write `derived`, `shaped` or `anchored`",
                            ),
                        ),
                        a,
                        ["derived", "shaped", "anchored"],
                        text::undeclared,
                    )),
                }
            }
            if let Some(defaults) = t.table("defaults") {
                for (name, e) in defaults.iter() {
                    let v = match &e.item.value {
                        Value::Num(n) => text::num(n).ok().map(Attr::Number),
                        Value::Bool(b) => Some(Attr::YesNo(*b)),
                        Value::Str(s) => Some(Attr::Text(s.text.clone())),
                        _ => None,
                    };
                    if let Some(v) = v {
                        k.defaults.insert(name.clone(), v);
                    }
                }
            }
            if let Some(req) = t.value("requires").and_then(text::strings) {
                for r in &req {
                    if k.defaults.contains_key(r) {
                        self.push(at_key(
                            codes::authority::ATTRIBUTE_TWICE,
                            "kind",
                            id,
                            "requires",
                            &format!("`{r}` is declared in both `defaults` and `requires`"),
                            "declare it once: with a value in `defaults`, or without in `requires`",
                        ));
                    }
                }
                k.requires = req;
            }
            if let Some(names) = t.value("names").and_then(text::strings) {
                for n in &names {
                    if !k.defaults.contains_key(n) && !k.requires.contains(n) {
                        let declared: Vec<&str> = k
                            .defaults
                            .keys()
                            .map(String::as_str)
                            .chain(k.requires.iter().map(String::as_str))
                            .collect();
                        self.push(text::with_nearest(
                            at_key(
                                codes::authority::NAMES_UNDECLARED,
                                "kind",
                                id,
                                "names",
                                &format!(
                                    "`{n}` in `names` is in neither `defaults` nor `requires`"
                                ),
                                &text::near_hint(
                                    n,
                                    declared.iter().copied(),
                                    "declare it in `requires`",
                                ),
                            ),
                            n,
                            declared.iter().copied(),
                            |n: &str| {
                                Some((
                                    Subject::entity("kind", id),
                                    Site::Key(if k.defaults.contains_key(n) {
                                        KeyPath::key("defaults").then(n)
                                    } else {
                                        KeyPath::key("requires")
                                    }),
                                ))
                            },
                        ));
                    }
                }
                k.names = names;
            }
            if let Some(v) = t.value("radius") {
                match text::length(v) {
                    Some(Ok(mm)) if mm > 0 => k.radius = Some(mm),
                    Some(Ok(mm)) => self.push(at_key(
                        codes::authority::RANGE,
                        "kind",
                        id,
                        "radius",
                        &format!(
                            "`radius` is {}; it must be above 0",
                            via_num::write_length(mm)
                        ),
                        "write a length above 0",
                    )),
                    Some(Err(e)) => self.push(text::num_diag(&e, "kind", id, &["radius"])),
                    None => {}
                }
            }
            k.output = t.str("output").map(str::to_string);
            // `exit` (`composition.md` 7): the doc checks its form.
            k.exit = match t.value("exit") {
                Some(Value::Bool(true)) => Some(Vec::new()),
                Some(v @ Value::List(_)) => text::strings(v),
                _ => None,
            };
            for sub in ["surface", "mount"] {
                if t.get(sub).is_some() {
                    self.push(at_key(
                        codes::doc::NOT_YET,
                        "kind",
                        id,
                        sub,
                        &format!("a kind `{sub}` arrives in M3"),
                        "in M1 every surface is the terrain",
                    ));
                }
            }
            d_effects(self, id, t, &mut k);
            self.formulas.kind_attrs.insert(
                id.to_string(),
                (
                    k.defaults.keys().cloned().collect(),
                    k.requires.iter().cloned().collect(),
                ),
            );
            self.kinds.insert(id.to_string(), k);
        }
    }

    fn read_fields(&mut self, world: &World, root: &Table) {
        for (name, t) in table_of(root, "field") {
            let rank = world.rank("field", name).unwrap_or(0);
            let weights = match t.str("type") {
                None | Some("scalar") => false,
                Some("weights") => true,
                Some("volume") => {
                    self.push(at_key(
                        codes::doc::NOT_YET,
                        "field",
                        name,
                        "type",
                        "a `volume` field arrives in M5",
                        "use a scalar field until M5",
                    ));
                    continue;
                }
                Some(other) => {
                    let hint = if other == "material" {
                        "write `type = \"weights\"` with `role = \"material\"`".to_string()
                    } else {
                        text::near_hint(
                            other,
                            ["scalar", "weights", "volume"],
                            "write `scalar` or `weights`",
                        )
                    };
                    let d = at_key(
                        codes::field::TYPE_UNKNOWN,
                        "field",
                        name,
                        "type",
                        &format!("`{other}` is not a field type"),
                        &hint,
                    );
                    self.push(if other == "material" {
                        d
                    } else {
                        text::with_nearest(
                            d,
                            other,
                            ["scalar", "weights", "volume"],
                            text::undeclared,
                        )
                    });
                    continue;
                }
            };
            let role = match t.str("role") {
                None => None,
                Some(r) => match Role::parse(r) {
                    Some(role) => {
                        let want_weights = role == Role::Material;
                        if want_weights != weights {
                            self.push(at_key(
                                codes::field::ROLE_TYPE,
                                "field",
                                name,
                                "role",
                                &format!(
                                    "the role `{r}` takes a {} field",
                                    if want_weights {
                                        "`weights`"
                                    } else {
                                        "`scalar`"
                                    }
                                ),
                                if want_weights {
                                    "write `type = \"weights\"`"
                                } else {
                                    "remove `type`"
                                },
                            ));
                        }
                        Some(role)
                    }
                    None => {
                        // `diagnostics.md` 1.1: one key's fault has the key in its subject.
                        let role = KeyPath::key("role");
                        self.push(text::with_nearest(
                            text::diag(
                                codes::field::ROLE_UNKNOWN,
                                Subject::entity("field", name).with_keys(&role),
                                Site::Key(role.clone()),
                                &format!("`{r}` is not a role"),
                                &text::near_hint(
                                    r,
                                    ["surface", "material", "keep_out"],
                                    "a library word belongs in `tags`",
                                ),
                            ),
                            r,
                            ["surface", "material", "keep_out"],
                            text::undeclared,
                        ));
                        None
                    }
                },
            };
            let unit = t.str("unit").map(str::to_string);
            if let Some(u) = &unit {
                if !["m", "deg", "ratio"].contains(&u.as_str()) {
                    self.push(text::with_nearest(
                        at_key(
                            codes::field::UNIT_UNKNOWN,
                            "field",
                            name,
                            "unit",
                            &format!("`{u}` is not a unit"),
                            &text::near_hint(
                                u,
                                ["m", "deg", "ratio"],
                                "write `m`, `deg` or `ratio`",
                            ),
                        ),
                        u,
                        ["m", "deg", "ratio"],
                        text::undeclared,
                    ));
                } else if weights && u != "ratio" {
                    self.push(at_key(
                        codes::field::WEIGHTS_UNIT,
                        "field",
                        name,
                        "unit",
                        "a `weights` field's unit is `ratio`",
                        "remove `unit`",
                    ));
                } else if let Some(role) = role {
                    let want = if role == Role::Surface { "m" } else { "ratio" };
                    if u != want {
                        self.push(at_key(
                            codes::field::ROLE_UNIT,
                            "field",
                            name,
                            "unit",
                            &format!("the role `{}` is in `{want}`", role.word()),
                            "remove `unit`, or write the role's",
                        ));
                    }
                }
            }
            let tags = t.value("tags").and_then(text::strings).unwrap_or_default();
            let file = self.read_file(world, name, t);
            if weights && t.get("base").is_some() {
                self.push(at_key(
                    codes::field::WEIGHTS_BASE,
                    "field",
                    name,
                    "base",
                    "a `weights` field has no `base`",
                    "read the raster as a scalar field and use it as a row's `mask`",
                ));
            }
            let base = if weights {
                None
            } else {
                t.value("base")
                    .and_then(text::formula_text)
                    .and_then(|r| r.ok())
                    .map(|ft| {
                        self.formulas
                            .add(&ft, Subject::entity("field", name), KeyPath::key("base"))
                    })
            };
            let mut rows = Vec::new();
            for (rid, rt) in table_of(t, "then") {
                let get = |key: &str, me: &mut Decls| {
                    rt.value(key)
                        .and_then(text::formula_text)
                        .and_then(|r| r.ok())
                        .map(|ft| {
                            me.formulas.add(
                                &ft,
                                Subject::entity("field", name),
                                text::path(&["then", rid, key]),
                            )
                        })
                };
                let value = get("value", self);
                let mask = get("mask", self);
                let paint = rt.str("paint").map(str::to_string);
                let order = match rt.value("order") {
                    Some(Value::Num(n)) if n.integer => n.norm.parse().unwrap_or(0),
                    _ => 0,
                };
                if weights {
                    if paint.is_none() {
                        self.push(at_path(
                            codes::field::ROW_MISSING,
                            "field",
                            name,
                            &["then", rid],
                            &format!("the weights row `{rid}` has no `paint`"),
                            "add `paint`",
                        ));
                        continue;
                    }
                    if value.is_some() {
                        self.push(at_path(
                            codes::field::ROW_KEY,
                            "field",
                            name,
                            &["then", rid, "value"],
                            &format!("`value` on the weights row `{rid}`"),
                            "use `paint` on a weights field",
                        ));
                    }
                } else {
                    if value.is_none() {
                        self.push(at_path(
                            codes::field::ROW_MISSING,
                            "field",
                            name,
                            &["then", rid],
                            &format!("the row `{rid}` has no `value`"),
                            "add `value`",
                        ));
                        continue;
                    }
                    if paint.is_some() {
                        self.push(at_path(
                            codes::field::ROW_KEY,
                            "field",
                            name,
                            &["then", rid, "paint"],
                            &format!("`paint` on the scalar row `{rid}`"),
                            "use `value` on a scalar field",
                        ));
                    }
                }
                // A returned row has its step's layer rank (`protocol.md` 5, Rank and Order).
                let row_rank = rid
                    .split_once('#')
                    .and_then(|(step, _)| world.rank("step", step))
                    .map_or(rank, |sr| sr.max(rank));
                rows.push(Row {
                    id: rid.to_string(),
                    value,
                    mask,
                    paint,
                    order,
                    rank: row_rank,
                });
            }
            self.fields.insert(
                name.to_string(),
                FieldDecl {
                    name: name.to_string(),
                    role,
                    weights,
                    unit,
                    tags,
                    rank,
                    base,
                    file,
                    rows,
                },
            );
        }
        // One terrain, one material (fields.md 2.1, surfaces.md 1).
        for (role, code, what) in [
            (
                Role::Surface,
                codes::surface::TWO_TERRAINS,
                "a second `surface` field",
            ),
            (
                Role::Material,
                codes::field::ROLE_TWICE,
                "a second `material` field",
            ),
        ] {
            let with: Vec<String> = self
                .fields
                .values()
                .filter(|f| f.role == Some(role))
                .map(|f| f.name.clone())
                .collect();
            for extra in with.iter().skip(1) {
                self.push(at_key(
                    code,
                    "field",
                    extra,
                    "role",
                    &format!("{what}: `{}` already has the role", with[0]),
                    "remove `role` from one; every other surface is a kind surface",
                ));
            }
        }
    }

    /// A field's `file` and its keys (`fields.md` 4.2); faults are said here, the file is read
    /// by the build, which knows the folders.
    fn read_file(&mut self, world: &World, name: &str, t: &Table) -> Option<FileDecl> {
        let keys = ["channel", "extent", "filter", "outside", "range"];
        let Some(e) = t.get("file") else {
            for k in keys {
                if t.get(k).is_some() {
                    self.push(text::at_own_key(
                        codes::field::FILE_KEY,
                        "field",
                        name,
                        k,
                        &format!("`{k}` without `file`: it says how a file is read"),
                        "add `file`, or remove the key",
                    ));
                }
            }
            return None;
        };
        let Value::Str(written) = &e.item.value else {
            return None;
        };
        // `channel` and `range` are a PNG's (M2): on an array file they do not apply.
        if written.text.to_ascii_lowercase().ends_with(".npy") {
            for k in ["channel", "range"] {
                if t.get(k).is_some() {
                    self.push(text::at_own_key(
                        codes::field::FILE_KEY,
                        "field",
                        name,
                        k,
                        &format!("`{k}` does not apply to an array file"),
                        "remove the key: an array file holds the values themselves",
                    ));
                }
            }
        }
        if t.get("base").is_some() {
            self.push(at_key(
                codes::field::BASE_AND_FILE,
                "field",
                name,
                "file",
                "a field has both `base` and `file`",
                "keep one",
            ));
            return None;
        }
        let outside = match t.value("outside").map(text::number) {
            Some(Some(Ok(x))) => x,
            Some(Some(Err(e))) => {
                self.push(text::num_diag(&e, "field", name, &["outside"]));
                return None;
            }
            _ => {
                self.push(at_key(
                    codes::field::FILE_OUTSIDE,
                    "field",
                    name,
                    "file",
                    "a field read from a `file` names no `outside`",
                    "write `outside = <value>`: the value at posts beyond the raster's extent",
                ));
                return None;
            }
        };
        let filter = match t.str("filter") {
            None | Some("linear") => via_raster::file::Filter::Linear,
            Some("nearest") => via_raster::file::Filter::Nearest,
            Some("cubic") => via_raster::file::Filter::Cubic,
            Some(f) => {
                self.push(text::at_own_key(
                    codes::field::FILE_KEY,
                    "field",
                    name,
                    "filter",
                    &format!("`{f}` is not a filter"),
                    &text::near_hint(
                        f,
                        ["nearest", "linear", "cubic"],
                        "write `nearest`, `linear` or `cubic`",
                    ),
                ));
                return None;
            }
        };
        let extent = match t.value("extent") {
            Some(Value::List(xs)) if xs.len() == 4 => {
                let mut v = [0i64; 4];
                for (k, x) in xs.iter().enumerate() {
                    match text::length(&x.value) {
                        Some(Ok(mm)) => v[k] = mm,
                        Some(Err(e)) => {
                            self.push(text::num_diag(&e, "field", name, &["extent"]));
                            return None;
                        }
                        None => return None,
                    }
                }
                if v[2] <= v[0] || v[3] <= v[1] {
                    self.push(at_key(
                        codes::num::EXTENT_EMPTY,
                        "field",
                        name,
                        "extent",
                        "the raster's extent is empty: `x1` must exceed `x0` and `y1` `y0`",
                        "write `[x0, y0, x1, y1]` with x0 < x1 and y0 < y1",
                    ));
                    return None;
                }
                Some(v)
            }
            Some(_) => return None,
            None => None,
        };
        let channel = t.str("channel").map(str::to_string);
        if let Some(c) = &channel {
            if !["grey", "r", "g", "b", "a"].contains(&c.as_str()) {
                self.push(text::at_own_key(
                    codes::field::FILE_KEY,
                    "field",
                    name,
                    "channel",
                    &format!("`{c}` is not a channel"),
                    "write `grey`, `r`, `g`, `b` or `a`",
                ));
                return None;
            }
        }
        let range = match t.value("range") {
            Some(Value::List(xs)) if xs.len() == 2 => {
                match (text::number(&xs[0].value), text::number(&xs[1].value)) {
                    (Some(Ok(lo)), Some(Ok(hi))) => {
                        if lo == hi {
                            self.push(at_key(
                                codes::field::FILE_RANGE,
                                "field",
                                name,
                                "range",
                                "`range` with `lo` equal to `hi`: every level reads one value",
                                "a constant is a base: write `base` instead of `file`",
                            ));
                            return None;
                        }
                        Some((lo, hi))
                    }
                    _ => return None,
                }
            }
            Some(_) => {
                self.push(text::at_own_key(
                    codes::field::FILE_KEY,
                    "field",
                    name,
                    "range",
                    "`range` is `[lo, hi]`",
                    "write `range = [0, 1]`",
                ));
                return None;
            }
            None => None,
        };
        Some(FileDecl {
            written: written.text.clone(),
            source: world.sources().name(e.key.src).to_string(),
            extent,
            outside,
            filter,
            channel,
            range,
        })
    }

    /// An import's `at` or `on` (`composition.md` 2), read as an instance's is.
    pub(crate) fn read_import_place(&mut self, id: &str, t: &Table) -> Option<Place> {
        self.read_place("import", id, t)
    }

    /// Adds a formula written outside the tables this module reads (an import's `blend`).
    pub(crate) fn add_formula(
        &mut self,
        ft: &FormulaText,
        subject: Subject,
        path: KeyPath,
    ) -> FormulaId {
        self.formulas.add(ft, subject, path)
    }

    fn read_place(&mut self, top: &str, id: &str, t: &Table) -> Option<Place> {
        if let Some(v) = t.value("at") {
            return match text::point(v) {
                Some(Ok(p)) => Some(Place::At(p)),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, top, id, &["at"]));
                    None
                }
                None => None,
            };
        }
        let on = t.table("on")?;
        if on.get("of").is_some() && on.get("ref").is_some() {
            self.push(at_key(
                codes::identity::ON_BOTH,
                top,
                id,
                "on",
                "`on` has both `ref` and `of`",
                "keep `ref` for an authored thing, `of` for a produced one",
            ));
            return None;
        }
        if let Some(of) = on.str("of") {
            return self.read_select(top, id, on, of, &["on"]).map(Place::Of);
        }
        let reference = on.str("ref")?.to_string();
        if reference.contains('#') {
            self.push(at_path(
                codes::identity::KEY_IN_TEXT,
                top,
                id,
                &["on", "ref"],
                &format!("`{reference}` is a produced name, which text never writes"),
                &format!(
                    "select it by place: `on = {{ of = \"{}\", near = [x, y] }}`",
                    reference.split('#').next().unwrap_or("")
                ),
            ));
            return None;
        }
        if on.get("span").is_some() {
            self.push(at_path(
                codes::place::ON_FORM,
                top,
                id,
                &["on", "span"],
                "`span` is a run's; a placement at a point takes `t`",
                "write `t`",
            ));
            return None;
        }
        let (t_, offset) = self.read_t_offset(top, id, on, "on")?;
        Some(Place::On(On {
            reference,
            t: t_,
            offset,
        }))
    }

    /// `t` and `offset` of an `on` table (`geometry.md` 2.6), `None` after a fault.
    #[allow(clippy::type_complexity)]
    pub(crate) fn read_t_offset(
        &mut self,
        top: &str,
        id: &str,
        on: &Table,
        key: &str,
    ) -> Option<(Option<f64>, Option<i64>)> {
        let t_ = match on.value("t") {
            Some(v) => match text::number(v) {
                Some(Ok(x)) => Some(x),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, top, id, &[key, "t"]));
                    return None;
                }
                None => None,
            },
            None => None,
        };
        let offset = match on.value("offset") {
            Some(v) => match text::length(v) {
                Some(Ok(mm)) => Some(mm),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, top, id, &[key, "offset"]));
                    return None;
                }
                None => None,
            },
            None => None,
        };
        Some((t_, offset))
    }

    /// A selection by place (`identity.md` 3): `{ of, class, near }`, with `t` and `offset` where
    /// the table takes them. `path` is the key path of the table (`on`, or `within`).
    pub(crate) fn read_select(
        &mut self,
        top: &str,
        id: &str,
        on: &Table,
        of: &str,
        path: &[&str],
    ) -> Option<Select> {
        fn join<'p>(path: &[&'p str], k: &'p str) -> Vec<&'p str> {
            let mut v = path.to_vec();
            v.push(k);
            v
        }
        let at = |k: &'static str| join(path, k);
        if of.contains('#') {
            self.push(at_path(
                codes::identity::KEY_IN_TEXT,
                top,
                id,
                &at("of"),
                &format!("`{of}` is a produced name; `of` names its maker"),
                &format!(
                    "select it by place: `{{ of = \"{}\", near = [x, y] }}`",
                    of.split('#').next().unwrap_or("")
                ),
            ));
            return None;
        }
        let class = on.str("class").map(str::to_string);
        if let Some(c) = &class {
            if c == "field" || c == "row" {
                self.push(at_path(
                    codes::identity::NO_PLACE,
                    top,
                    id,
                    &at("class"),
                    &format!("a `{c}` has no place, so it cannot be selected by one"),
                    "select a class with a place, or read the field through a tag",
                ));
                return None;
            }
        }
        let near = match on.value("near") {
            Some(v) => match text::point(v) {
                Some(Ok(p)) => p,
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, top, id, &at("near")));
                    return None;
                }
                None => return None,
            },
            None => {
                self.push(at_path(
                    codes::doc::MISSING_KEY,
                    top,
                    id,
                    path,
                    "a selection by place needs `near`",
                    "write `near = [x, y]`, a point near the thing you mean",
                ));
                return None;
            }
        };
        let (t_, offset) = if path == ["on"] {
            self.read_t_offset(top, id, on, "on")?
        } else {
            (None, None)
        };
        Some(Select {
            of: of.to_string(),
            class,
            near,
            t: t_,
            offset,
        })
    }

    pub(crate) fn read_face(&mut self, top: &str, id: &str, t: &Table) -> Option<Face> {
        match t.value("face")? {
            Value::Num(n) => match via_num::parse_facing(&n.norm) {
                Ok(m) => Some(Face::Bearing(m)),
                Err(e) => {
                    self.push(text::num_diag(&e, top, id, &["face"]));
                    None
                }
            },
            Value::Str(s) => match s.text.as_str() {
                "along" => Some(Face::Along),
                "toward" => Some(Face::Toward),
                "away" => Some(Face::Away),
                w => {
                    let words = ["along", "toward", "away"];
                    self.push(text::with_nearest(
                        at_key(
                            codes::geom::FACE_UNKNOWN,
                            top,
                            id,
                            "face",
                            &format!("`{w}` is not a facing word"),
                            &text::near_hint(
                                w,
                                words,
                                "write a bearing in degrees, or `along`, `toward`, `away`",
                            ),
                        ),
                        w,
                        words,
                        text::undeclared,
                    ));
                    None
                }
            },
            _ => None,
        }
    }

    fn read_features(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "feature") {
            self.declared.insert(id.to_string(), "feature");
            let kind = t.str("kind").map(str::to_string);
            let mut tier = None;
            if let Some(a) = t.str("authority") {
                if kind.is_none() {
                    self.push(at_key(
                        codes::authority::NO_KIND,
                        "feature",
                        id,
                        "authority",
                        &format!("`{id}` has no kind, so nothing to hold"),
                        "give it a `kind`, or remove `authority`",
                    ));
                } else {
                    match Tier::parse(a) {
                        Some(x) => tier = Some(x),
                        None => self.push(text::with_nearest(
                            at_key(
                                codes::authority::UNKNOWN,
                                "feature",
                                id,
                                "authority",
                                &format!("`{a}` is not a tier"),
                                &text::near_hint(
                                    a,
                                    ["derived", "shaped", "anchored"],
                                    "write `derived`, `shaped` or `anchored`",
                                ),
                            ),
                            a,
                            ["derived", "shaped", "anchored"],
                            text::undeclared,
                        )),
                    }
                }
            }
            if let Some(k) = &kind {
                if !self.kinds.contains_key(k) {
                    self.push(text::with_nearest(
                        at_key(
                            codes::authority::UNKNOWN_KIND,
                            "feature",
                            id,
                            "kind",
                            &format!("`{k}` names no kind of this world"),
                            &text::near_hint(
                                k,
                                self.kinds.keys().map(String::as_str),
                                "include the library that declares it",
                            ),
                        ),
                        k,
                        self.kinds.keys().map(String::as_str),
                        text::header_of("kind"),
                    ));
                    continue;
                }
            }
            let starts_at = t.str("starts_at").map(str::to_string);
            let ends_at = t.str("ends_at").map(str::to_string);
            for (k, v) in [("starts_at", &starts_at), ("ends_at", &ends_at)] {
                if v.as_deref().is_some_and(|s| s.contains('#')) {
                    self.push(at_key(
                        codes::identity::KEY_IN_TEXT,
                        "feature",
                        id,
                        k,
                        "a produced name is written where a name is taken",
                        "select by place with `{ of, near }`",
                    ));
                }
            }
            let shape = if let Some(v) = t.value("path") {
                match text::ways(v) {
                    Some(Ok(p)) => {
                        for w in &p {
                            if let WayDecl::Name(n) = w {
                                if n.contains('#') {
                                    self.push(at_key(
                                        codes::identity::KEY_IN_TEXT,
                                        "feature",
                                        id,
                                        "path",
                                        "a produced name is written where a name is taken",
                                        "select by place with `{ of, near }` on a point feature, and name that",
                                    ));
                                }
                            }
                        }
                        ShapeDecl::Path(p)
                    }
                    Some(Err(e)) => {
                        self.push(text::num_diag(&e, "feature", id, &["path"]));
                        continue;
                    }
                    None => continue,
                }
            } else if let Some(v) = t.value("region") {
                match text::points(v) {
                    Some(Ok(p)) => ShapeDecl::Region(p),
                    Some(Err(e)) => {
                        self.push(text::num_diag(&e, "feature", id, &["region"]));
                        continue;
                    }
                    None => continue,
                }
            } else if let Some(v) = t.value("where") {
                match text::formula_text(v) {
                    Some(Ok(ft)) => ShapeDecl::Where(self.formulas.add(
                        &ft,
                        Subject::entity("feature", id),
                        KeyPath::key("where"),
                    )),
                    Some(Err(e)) => {
                        self.push(text::num_diag(&e, "feature", id, &["where"]));
                        continue;
                    }
                    None => continue,
                }
            } else if let Some(ft) = t.table("follows") {
                match self.read_follows(id, ft) {
                    Some(f) => ShapeDecl::Follows(f),
                    None => continue,
                }
            } else {
                match self.read_place("feature", id, t) {
                    Some(p) => ShapeDecl::Point(p),
                    None => {
                        if t.get("at").is_none()
                            && t.get("on").is_none()
                            && t.get("where").is_none()
                            && t.get("follows").is_none()
                        {
                            self.push(at_header(
                                codes::doc::MISSING_KEY,
                                "feature",
                                id,
                                &format!("`{id}` has no shape"),
                                "write `at`, `on`, `path` or `region`",
                            ));
                        }
                        continue;
                    }
                }
            };
            let face = self.read_face("feature", id, t);
            // `holds` and `route` (`derived.md` 2 and 3), each where it stands alone said first.
            let has_where = matches!(shape, ShapeDecl::Where(_));
            let is_path = matches!(shape, ShapeDecl::Path(_));
            let alone = crate::derived::shape_key_faults(
                id,
                has_where,
                t.get("holds").is_some(),
                t.get("route").is_some(),
                is_path,
            );
            let alone_any = !alone.is_empty();
            for d in alone {
                self.push(d);
            }
            if alone_any {
                continue;
            }
            let holds = match t.value("holds") {
                Some(v) => match text::point(v) {
                    Some(Ok(p)) => Some(p),
                    Some(Err(e)) => {
                        self.push(text::num_diag(&e, "feature", id, &["holds"]));
                        continue;
                    }
                    None => continue,
                },
                None => None,
            };
            let route = match t.value("route") {
                Some(v) => match text::formula_text(v) {
                    Some(Ok(ft)) => Some(self.formulas.add(
                        &ft,
                        Subject::entity("feature", id),
                        KeyPath::key("route"),
                    )),
                    Some(Err(e)) => {
                        self.push(text::num_diag(&e, "feature", id, &["route"]));
                        continue;
                    }
                    None => continue,
                },
                None => None,
            };
            let smooth = match t.value("smooth") {
                Some(Value::Bool(b)) if matches!(shape, ShapeDecl::Path(_)) => *b,
                Some(_) => {
                    self.push(at_key(
                        codes::geom::SMOOTH_NOT_PATH,
                        "feature",
                        id,
                        "smooth",
                        "`smooth` is taken only with `path`",
                        "remove `smooth`: it smooths a written `path`, nothing else",
                    ));
                    false
                }
                None => false,
            };
            if matches!(shape, ShapeDecl::Follows(_)) {
                for k in ["starts_at", "ends_at"] {
                    if t.get(k).is_some() {
                        self.push(at_key(
                            codes::doc::NOT_YET,
                            "feature",
                            id,
                            k,
                            &format!("`{k}` with `follows` is decided in M3 (`geometry.md` 4, the row on smoothing `follows`)"),
                            "remove it: a path along another feature runs over its `span`",
                        ));
                    }
                }
            }
            let mut attrs = BTreeMap::new();
            if let Some(k) = kind.as_ref().and_then(|k| self.kinds.get(k)).cloned() {
                // Attribute formulas (`formulas.md` 1 and 6.7): a string that starts with `=`.
                // Each is lowered to a formula at its own text, the feature's or the kind's
                // `defaults`, and worked out after the text phase (`eval_attributes`).
                let mut from_default: BTreeSet<String> = BTreeSet::new();
                let mut formulas: BTreeSet<String> = BTreeSet::new();
                for (n, v) in &k.defaults {
                    match v {
                        Attr::Text(s) if s.starts_with('=') => {
                            from_default.insert(n.clone());
                        }
                        _ => {
                            attrs.insert(n.clone(), v.clone());
                        }
                    }
                }
                let declared: BTreeSet<&String> =
                    k.defaults.keys().chain(k.requires.iter()).collect();
                for (n, e) in t.iter() {
                    if declared.contains(n) {
                        if let Value::Str(s) = &e.item.value {
                            if s.text.starts_with('=') {
                                // A formula where the kind's default is a bool or text.
                                let bad = match k.defaults.get(n) {
                                    Some(Attr::YesNo(_)) => Some("a bool"),
                                    Some(Attr::Text(d)) if !d.starts_with('=') => Some("text"),
                                    None if k.names.contains(n) => Some("text (a place name)"),
                                    _ => None,
                                };
                                if let Some(what) = bad {
                                    self.push(text::at_own_key(
                                        codes::formula::TYPE,
                                        "feature",
                                        id,
                                        n,
                                        &format!("`{n}` is written as a formula, but its kind `{}` gives it {what}", k.id),
                                        &format!("write {what} for `{n}`, as the kind's default is"),
                                    ));
                                    attrs.remove(n);
                                    from_default.remove(n);
                                    continue;
                                }
                                attrs.remove(n);
                                from_default.remove(n);
                                formulas.insert(n.clone());
                                let fid = self.formulas.add(
                                    &FormulaText::Text(s.text.clone()),
                                    Subject::entity("feature", id),
                                    KeyPath::key(n),
                                );
                                self.attr_formulas.push(AttrFormula {
                                    feature: id.to_string(),
                                    name: n.clone(),
                                    formula: fid,
                                    from_kind: false,
                                });
                                continue;
                            }
                            let rest = s.text.trim_start_matches([' ', '\t']);
                            if rest.starts_with('=') {
                                self.push(text::at_own_key(
                                    codes::formula::UNMARKED,
                                    "feature",
                                    id,
                                    n,
                                    &format!("the attribute `{n}` has text before its `=`"),
                                    &format!("remove what comes before `=`: write `\"{rest}\"`"),
                                ));
                                attrs.remove(n);
                                from_default.remove(n);
                                continue;
                            }
                        }
                        let v = match &e.item.value {
                            Value::Num(x) => text::num(x).ok().map(Attr::Number),
                            Value::Bool(b) => Some(Attr::YesNo(*b)),
                            Value::Str(s) => Some(Attr::Text(s.text.clone())),
                            _ => None,
                        };
                        if let Some(v) = v {
                            from_default.remove(n);
                            attrs.insert(n.clone(), v);
                        }
                    }
                }
                for n in &from_default {
                    let Some(Attr::Text(s)) = k.defaults.get(n) else {
                        continue;
                    };
                    formulas.insert(n.clone());
                    let fid = self.formulas.add(
                        &FormulaText::Text(s.clone()),
                        Subject::entity("kind", &k.id),
                        text::path(&["defaults", n]),
                    );
                    self.attr_formulas.push(AttrFormula {
                        feature: id.to_string(),
                        name: n.clone(),
                        formula: fid,
                        from_kind: true,
                    });
                }
                for r in &k.requires {
                    if !attrs.contains_key(r) && !formulas.contains(r) {
                        // `authority.md` 1: both lines, the feature's and the kind's `requires`.
                        self.push(
                            at_header(
                                codes::authority::REQUIRES,
                                "feature",
                                id,
                                &format!("`{id}` lacks `{r}`, which its kind `{}` requires", k.id),
                                &format!("write `{r} = ...` on `{id}`"),
                            )
                            .related(requires_line(&k.id)),
                        );
                    }
                }
            }
            self.features.insert(
                id.to_string(),
                FeatureDecl {
                    id: id.to_string(),
                    kind,
                    tier,
                    rank: world.rank("feature", id).unwrap_or(0),
                    shape,
                    starts_at,
                    ends_at,
                    face,
                    attrs,
                    smooth,
                    holds,
                    route,
                },
            );
        }
    }

    /// `follows = { ref, span, offset }` (`geometry.md` 4).
    fn read_follows(&mut self, id: &str, t: &Table) -> Option<Follows> {
        let Some(reference) = t.str("ref").map(str::to_string) else {
            self.push(at_path(
                codes::doc::MISSING_KEY,
                "feature",
                id,
                &["follows"],
                "`follows` names no `ref`",
                "write `follows = { ref = \"<path or region>\", span = [from, to], offset = 0 }`",
            ));
            return None;
        };
        if reference.contains('#') {
            self.push(at_path(
                codes::identity::KEY_IN_TEXT,
                "feature",
                id,
                &["follows", "ref"],
                &format!("`{reference}` is a produced name, which text never writes"),
                "name an authored path or region",
            ));
            return None;
        }
        let span = match t.value("span") {
            None => (0.0, 1.0),
            Some(Value::List(xs)) if xs.len() == 2 => {
                let mut v = [0.0; 2];
                for (k, x) in xs.iter().enumerate() {
                    match text::number(&x.value) {
                        Some(Ok(n)) => v[k] = n,
                        Some(Err(e)) => {
                            self.push(text::num_diag(&e, "feature", id, &["follows", "span"]));
                            return None;
                        }
                        None => return None,
                    }
                }
                for n in v {
                    if !(0.0..=1.0).contains(&n) {
                        self.push(at_path(
                            codes::geom::T_RANGE,
                            "feature",
                            id,
                            &["follows", "span"],
                            &format!("a `span` end is {}, outside 0 to 1", text::dbl(n)),
                            "write fractions from 0 to 1",
                        ));
                        return None;
                    }
                }
                (v[0], v[1])
            }
            Some(_) => return None,
        };
        let offset = match t.value("offset") {
            None => 0,
            Some(v) => match text::length(v) {
                Some(Ok(mm)) => mm,
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, "feature", id, &["follows", "offset"]));
                    return None;
                }
                None => return None,
            },
        };
        Some(Follows {
            reference,
            span,
            offset,
        })
    }

    pub(crate) fn read_scale_tint(
        &mut self,
        top: &str,
        id: &str,
        t: &Table,
        key: &str,
        drawn: bool,
    ) -> Option<Ranged> {
        let v = match t.value(key) {
            None => return Some(Ranged::Fixed(1.0)),
            Some(v) => v,
        };
        let r = match v {
            Value::Num(n) => Ranged::Fixed(text::num(n).ok()?),
            Value::List(xs) if xs.len() == 2 => {
                let lo = text::number(&xs[0].value)?.ok()?;
                let hi = text::number(&xs[1].value)?.ok()?;
                if !drawn {
                    self.push(at_key(
                        codes::place::RANGE,
                        top,
                        id,
                        key,
                        &format!("`{key}` takes one number here; only a scatter draws"),
                        "write one number",
                    ));
                    return None;
                }
                Ranged::Range(lo, hi)
            }
            _ => return None,
        };
        let (lo, hi) = match r {
            Ranged::Fixed(v) => (v, v),
            Ranged::Range(a, b) => (a, b),
        };
        let bad = lo > hi || (key == "scale" && lo <= 0.0) || (key == "tint" && lo < 0.0);
        if bad {
            let code = if drawn {
                codes::scatter::RANGE
            } else {
                codes::place::RANGE
            };
            self.push(at_key(
                code,
                top,
                id,
                key,
                &format!("`{key}` is out of range"),
                if key == "scale" {
                    "`scale` above 0, `lo <= hi`"
                } else {
                    "`tint` 0 or more, `lo <= hi`"
                },
            ));
            return None;
        }
        Some(r)
    }

    fn read_instances(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "instance") {
            self.declared.insert(id.to_string(), "instance");
            // `rules.md` 2: exactly one of `asset` and `set`; a set placed is a frame (4).
            let Some((asset, set)) = self.read_what("instance", id, t) else {
                continue;
            };
            if t.get("at").is_none() && t.get("on").is_none() {
                self.push(at_header(
                    codes::place::NO_POSITION,
                    "instance",
                    id,
                    "the instance has neither `at` nor `on`",
                    "write `at` or `on`",
                ));
                continue;
            }
            let Some(place) = self.read_place("instance", id, t) else {
                continue;
            };
            let face = self.read_face("instance", id, t);
            let Some(Ranged::Fixed(scale)) =
                self.read_scale_tint("instance", id, t, "scale", false)
            else {
                continue;
            };
            let Some(Ranged::Fixed(tint)) = self.read_scale_tint("instance", id, t, "tint", false)
            else {
                continue;
            };
            let ground = t.str("ground").map(str::to_string);
            let margin = match t.value("margin") {
                None => 0,
                Some(v) => match text::length(v) {
                    Some(Ok(mm)) if mm >= 0 => mm,
                    Some(Ok(_)) => {
                        self.push(at_key(
                            codes::place::RANGE,
                            "instance",
                            id,
                            "margin",
                            "`margin` is below 0",
                            "write a length, 0 or more",
                        ));
                        continue;
                    }
                    _ => 0,
                },
            };
            if t.get("margin").is_some() && ground.is_none() {
                self.push(at_key(
                    codes::place::MARGIN_ALONE,
                    "instance",
                    id,
                    "margin",
                    "`margin` without `ground`",
                    "add `ground = \"<kind>\"`, or remove `margin`",
                ));
                continue;
            }
            if let Some(g) = &ground {
                match self.kinds.get(g) {
                    None => {
                        self.push(text::with_nearest(
                            at_key(
                                codes::authority::UNKNOWN_KIND,
                                "instance",
                                id,
                                "ground",
                                &format!("`{g}` names no kind of this world"),
                                &text::near_hint(
                                    g,
                                    self.kinds.keys().map(String::as_str),
                                    "include the library that declares it",
                                ),
                            ),
                            g,
                            self.kinds.keys().map(String::as_str),
                            text::header_of("kind"),
                        ));
                        continue;
                    }
                    Some(k) if !k.requires.is_empty() => {
                        self.push(
                            at_key(
                                codes::authority::GROUND_REQUIRES,
                                "instance",
                                id,
                                "ground",
                                &format!(
                                    "the kind `{g}` requires `{}`, which a footprint cannot write",
                                    k.requires.join("`, `")
                                ),
                                "give the attribute a default in the kind, or ground with another kind",
                            )
                            .related(requires_line(g)),
                        );
                        continue;
                    }
                    _ => {}
                }
            }
            self.instances.insert(
                id.to_string(),
                InstanceDecl {
                    id: id.to_string(),
                    asset,
                    place,
                    face,
                    scale,
                    tint,
                    ground,
                    margin,
                    rank: world.rank("instance", id).unwrap_or(0),
                    set,
                },
            );
        }
        for top in ["lock"] {
            for (id, _) in table_of(root, top) {
                self.push(at_header(
                    codes::doc::NOT_YET,
                    top,
                    id,
                    &format!("`[{top}]` arrives after M1"),
                    "see the vocabulary's milestones",
                ));
            }
        }
    }

    fn read_scatters(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "scatter") {
            let mut choices = Vec::new();
            if let Some(Value::List(xs)) = t.value("assets") {
                for x in xs {
                    match &x.value {
                        Value::Str(s) => choices.push(Choice {
                            what: s.text.clone(),
                            weight: 1_000_000,
                        }),
                        Value::Table(c) => {
                            let what = c.str("id").unwrap_or("").to_string();
                            let weight = match c.value("weight") {
                                Some(Value::Num(n)) => {
                                    via_num::parse_millionths(&n.norm).unwrap_or(0)
                                }
                                _ => 1_000_000,
                            };
                            if weight <= 0 {
                                self.push(at_key(
                                    codes::scatter::RANGE,
                                    "scatter",
                                    id,
                                    "assets",
                                    "a `weight` must be above 0",
                                    "write a weight above 0",
                                ));
                                continue;
                            }
                            choices.push(Choice { what, weight });
                        }
                        _ => {}
                    }
                }
            }
            if choices.is_empty() {
                self.push(at_header(
                    codes::doc::MISSING_KEY,
                    "scatter",
                    id,
                    "the scatter has no `assets`",
                    "write `assets = [\"<id>\"]`",
                ));
                continue;
            }
            let spacing = match t.value("spacing").and_then(text::length) {
                Some(Ok(mm)) if mm > 0 => mm,
                Some(Ok(_)) => {
                    self.push(at_key(
                        codes::scatter::RANGE,
                        "scatter",
                        id,
                        "spacing",
                        "`spacing` must be above 0",
                        "write a length above 0",
                    ));
                    continue;
                }
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, "scatter", id, &["spacing"]));
                    continue;
                }
                None => {
                    self.push(at_header(
                        codes::doc::MISSING_KEY,
                        "scatter",
                        id,
                        "the scatter has no `spacing`",
                        "write `spacing` in metres",
                    ));
                    continue;
                }
            };
            let jitter = match t.value("jitter") {
                None => 800_000,
                Some(Value::Num(n)) => match via_num::parse_millionths(&n.norm) {
                    Ok(j) if (0..=1_000_000).contains(&j) => j,
                    Ok(_) => {
                        self.push(at_key(
                            codes::scatter::RANGE,
                            "scatter",
                            id,
                            "jitter",
                            "`jitter` lies in 0 to 1",
                            "write a share from 0 to 1",
                        ));
                        continue;
                    }
                    Err(e) => {
                        self.push(text::num_diag(&e, "scatter", id, &["jitter"]));
                        continue;
                    }
                },
                _ => 800_000,
            };
            let density = t
                .value("density")
                .and_then(text::formula_text)
                .and_then(|r| r.ok())
                .map(|ft| {
                    self.formulas
                        .add(&ft, Subject::entity("scatter", id), KeyPath::key("density"))
                });
            let within = match t.value("within") {
                Some(Value::Str(s)) => {
                    if s.text.contains('#') {
                        self.push(at_key(
                            codes::identity::KEY_IN_TEXT,
                            "scatter",
                            id,
                            "within",
                            "a produced name is written in `within`",
                            "select by place with `{ of, class, near }`",
                        ));
                        continue;
                    }
                    Some(s.text.clone())
                }
                Some(_) => None,
                None => None,
            };
            let within_of = match t.table("within") {
                Some(w) => match w.str("of") {
                    Some(of) => match self.read_select("scatter", id, w, of, &["within"]) {
                        Some(sel) => Some(sel),
                        None => continue,
                    },
                    None => {
                        self.push(at_path(
                            codes::doc::MISSING_KEY,
                            "scatter",
                            id,
                            &["within"],
                            "a selection in `within` needs `of`",
                            "write `within = { of = \"<maker>\", class = \"piece\", near = [x, y] }`",
                        ));
                        continue;
                    }
                },
                None => None,
            };
            let clump = match t.table("clump") {
                Some(c) => match self.read_clump(id, c) {
                    Some(c) => Some(c),
                    None => continue,
                },
                None => None,
            };
            let Some(scale) = self.read_scale_tint("scatter", id, t, "scale", true) else {
                continue;
            };
            let Some(tint) = self.read_scale_tint("scatter", id, t, "tint", true) else {
                continue;
            };
            let seed = t.str("seed").unwrap_or(id).to_string();
            // `numbers.md` 7: a scatter seed is a name, by the id grammar.
            if !via_doc::is_id(&seed) {
                let key = KeyPath::key("seed");
                self.push(text::diag(
                    codes::num::SEED_TYPE,
                    Subject::entity("scatter", id).with_keys(&key),
                    Site::Key(key),
                    &format!("the seed `{seed}` is not a name: a name is `[a-z][a-z0-9_]*`"),
                    &format!("write `{}`", via_doc::repair_id(&seed)),
                ));
                continue;
            }
            self.scatters.insert(
                id.to_string(),
                ScatterDecl {
                    id: id.to_string(),
                    choices,
                    spacing,
                    jitter,
                    density,
                    within,
                    within_of,
                    scale,
                    tint,
                    seed,
                    rank: world.rank("scatter", id).unwrap_or(0),
                    clump,
                },
            );
        }
    }

    /// `clump` (`rules.md` 7.3): one of its two forms, checked; a fault is `scatter.clump`.
    fn read_clump(&mut self, id: &str, c: &Table) -> Option<Clump> {
        let fault = |s: &mut Self, msg: &str, hint: &str| {
            s.push(text::at_own_key(
                codes::scatter::CLUMP,
                "scatter",
                id,
                "clump",
                msg,
                hint,
            ));
        };
        let members = c.get("members").is_some() || c.get("shrink").is_some();
        let around = c.get("around").is_some();
        if members && around {
            fault(
                self,
                "`clump` holds keys of both forms",
                "write `members` (with `shrink`) or `around`, as two scatters if both are meant",
            );
            return None;
        }
        if !members && !around {
            fault(
                self,
                "`clump` has neither `members` nor `around`",
                "write `members = [lo, hi]` or `around = \"<producer>\"`",
            );
            return None;
        }
        let radius = match c.value("radius").map(text::length) {
            Some(Some(Ok(r))) if r > 0 => r,
            Some(Some(Ok(_))) => {
                fault(
                    self,
                    "`radius` must be above 0",
                    "give `radius` in metres, above 0",
                );
                return None;
            }
            _ => {
                fault(self, "`clump` has no `radius`", "write `radius = <metres>`");
                return None;
            }
        };
        if around {
            let Some(of) = c.str("around") else {
                fault(
                    self,
                    "`around` names a producer by its id",
                    "write `around = \"<producer>\"`",
                );
                return None;
            };
            return Some(Clump::Around {
                of: of.to_string(),
                radius,
            });
        }
        let int = |v: &Value| match text::number(v) {
            Some(Ok(x)) if (x as i64) as f64 == x => Some(x as i64),
            _ => None,
        };
        let (lo, hi) = match c.value("members") {
            Some(Value::List(xs)) if xs.len() == 2 => {
                match (int(&xs[0].value), int(&xs[1].value)) {
                    (Some(a), Some(b)) => (a, b),
                    _ => (-1, -1),
                }
            }
            Some(v) => int(v).map_or((-1, -1), |n| (n, n)),
            None => (0, 0),
        };
        if lo < 0 || hi < lo {
            fault(
                self,
                "`members` is a whole number or `[lo, hi]`, 0 or more, lo at most hi",
                "write `members = [2, 5]`",
            );
            return None;
        }
        let shrink = match c.value("shrink").map(text::number) {
            None => 0.0,
            Some(Some(Ok(x))) if (0.0..1.0).contains(&x) => x,
            _ => {
                fault(
                    self,
                    "`shrink` lies in 0 to below 1",
                    "write `shrink = 0.4`",
                );
                return None;
            }
        };
        Some(Clump::Members {
            lo,
            hi,
            radius,
            shrink,
        })
    }

    fn read_by(&mut self, top: &str, id: &str, t: &Table) -> Option<Vec<String>> {
        let by = t.value("by").and_then(text::strings)?;
        for b in &by {
            if b.contains('#') {
                self.push(at_key(
                    codes::identity::KEY_IN_TEXT,
                    top,
                    id,
                    "by",
                    "a produced name is written in `by`",
                    "name the producer by its id",
                ));
            }
        }
        Some(by)
    }

    fn read_edits(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "pin") {
            self.declared.insert(id.to_string(), "pin");
            let Some(asset) = t.str("asset").map(str::to_string) else {
                self.push(at_header(
                    codes::place::ASSET_OR_SET,
                    "pin",
                    id,
                    "a pin writes no `asset`",
                    "write `asset`",
                ));
                continue;
            };
            if t.get("at").is_none() && t.get("on").is_none() {
                self.push(at_header(
                    codes::place::NO_POSITION,
                    "pin",
                    id,
                    "the pin has neither `at` nor `on`",
                    "write `at` or `on`",
                ));
                continue;
            }
            let Some(place) = self.read_place("pin", id, t) else {
                continue;
            };
            let face = self.read_face("pin", id, t);
            let by = self.read_by("pin", id, t);
            self.pins.insert(
                id.to_string(),
                PinDecl {
                    id: id.to_string(),
                    asset,
                    place,
                    face,
                    by,
                    rank: world.rank("pin", id).unwrap_or(0),
                },
            );
        }
        for (id, t) in table_of(root, "clear") {
            let place = if t.get("at").is_some() || t.get("on").is_some() {
                match self.read_place("clear", id, t) {
                    Some(p) => Some(p),
                    None => continue,
                }
            } else {
                None
            };
            let region = match t.value("region").and_then(text::points) {
                Some(Ok(p)) => Some(p),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, "clear", id, &["region"]));
                    continue;
                }
                None => None,
            };
            if place.is_none() && region.is_none() {
                self.push(at_header(
                    codes::place::NO_POSITION,
                    "clear",
                    id,
                    "the clear has no place",
                    "write `at`, `on` or `region`",
                ));
                continue;
            }
            let radius = match t.value("radius").and_then(text::length) {
                Some(Ok(mm)) => Some(mm),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, "clear", id, &["radius"]));
                    continue;
                }
                None => None,
            };
            let by = self.read_by("clear", id, t);
            self.clears.insert(
                id.to_string(),
                ClearDecl {
                    id: id.to_string(),
                    place,
                    region,
                    radius,
                    by,
                    rank: world.rank("clear", id).unwrap_or(0),
                },
            );
        }
    }

    fn read_steps(&mut self, root: &Table) {
        for (id, t) in table_of(root, "step") {
            let when = t.str("when").unwrap_or("").to_string();
            let command = t
                .value("command")
                .and_then(text::strings)
                .unwrap_or_default();
            if when == "export" {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "step",
                    id,
                    "when",
                    "an `export` step arrives in M6",
                    "write `when = \"lint\"` or `\"generate\"`",
                ));
            }
            self.steps.insert(
                id.to_string(),
                StepDecl {
                    id: id.to_string(),
                    when,
                    command,
                },
            );
        }
    }

    fn declare_env(&mut self) {
        let env = &mut self.formulas.env;
        for name in self.fields.keys() {
            env.field(name);
        }
        for name in &self.scoped_fields {
            env.field(name);
        }
        for (name, g) in &self.givens {
            if g.accepts == "field" {
                env.given_field(name);
            }
        }
        for (id, k) in &self.kinds {
            for (a, v) in &k.defaults {
                let ty = match v {
                    Attr::Number(_) => AttrType::Number,
                    Attr::YesNo(_) => AttrType::YesNo,
                    // A default formula gives a number (`formulas.md` 6.7).
                    Attr::Text(s) if s.starts_with('=') => AttrType::Number,
                    Attr::Text(_) => AttrType::Text,
                };
                env.kind_attribute(id, a, ty);
            }
            for a in &k.requires {
                if !k.names.contains(a) {
                    env.kind_attribute(id, a, AttrType::Number);
                } else {
                    env.kind_attribute(id, a, AttrType::Text);
                }
            }
            if let Some(r) = k.radius {
                env.kind_radius(id, via_num::metres(r));
            }
        }
        for (id, f) in &self.features {
            env.feature(id, f.kind.as_deref());
            for (a, v) in &f.attrs {
                let ty = match v {
                    Attr::Number(_) => AttrType::Number,
                    Attr::YesNo(_) => AttrType::YesNo,
                    Attr::Text(_) => AttrType::Text,
                };
                env.feature_attribute(id, a, ty);
            }
        }
        for af in &self.attr_formulas {
            env.feature_attribute(&af.feature, &af.name, AttrType::Number);
        }
    }

    /// `formulas.md` 6.7: every attribute formula, once per feature, in dependency order, before
    /// any shape or field. A loop between them is `graph.cycle` with the chain (case 39). A fault
    /// is the formula's own, at its text; what read it says nothing more.
    fn eval_attributes(&mut self) {
        let n = self.attr_formulas.len();
        if n == 0 {
            return;
        }
        let index: BTreeMap<(String, String), usize> = self
            .attr_formulas
            .iter()
            .enumerate()
            .map(|(i, a)| ((a.feature.clone(), a.name.clone()), i))
            .collect();
        let deps: Vec<Vec<usize>> = self
            .attr_formulas
            .iter()
            .map(|a| {
                let Some(p) = self.formulas.programs.get(&a.formula) else {
                    return Vec::new();
                };
                let own = p
                    .reads
                    .attributes
                    .iter()
                    .map(|x| (a.feature.clone(), x.clone()));
                let mut v: Vec<usize> = own
                    .chain(p.reads.members.iter().cloned())
                    .filter_map(|k| index.get(&k).copied())
                    .collect();
                v.sort_unstable();
                v.dedup();
                v
            })
            .collect();
        // Depth first, in declaration order; a node met again on the stack closes a loop.
        let mut state = vec![0u8; n];
        let mut order: Vec<usize> = Vec::with_capacity(n);
        let mut looped: BTreeSet<usize> = BTreeSet::new();
        let mut loops: Vec<Vec<usize>> = Vec::new();
        fn visit(
            i: usize,
            deps: &[Vec<usize>],
            state: &mut [u8],
            stack: &mut Vec<usize>,
            order: &mut Vec<usize>,
            loops: &mut Vec<Vec<usize>>,
        ) {
            state[i] = 1;
            stack.push(i);
            for &j in &deps[i] {
                match state[j] {
                    0 => visit(j, deps, state, stack, order, loops),
                    1 => {
                        let at = stack.iter().position(|x| *x == j).unwrap_or(0);
                        loops.push(stack[at..].to_vec());
                    }
                    _ => {}
                }
            }
            stack.pop();
            state[i] = 2;
            order.push(i);
        }
        for i in 0..n {
            if state[i] == 0 {
                let mut stack = Vec::new();
                visit(i, &deps, &mut state, &mut stack, &mut order, &mut loops);
            }
        }
        let label = |a: &AttrFormula| format!("{}.{}", a.feature, a.name);
        let mut said: BTreeSet<Vec<usize>> = BTreeSet::new();
        for members in loops {
            looped.extend(members.iter().copied());
            let mut key = members.clone();
            key.sort_unstable();
            if !said.insert(key) {
                continue;
            }
            // One loop, one record (`graph.md` 2), written from its least member.
            let names: Vec<String> = members
                .iter()
                .map(|i| label(&self.attr_formulas[*i]))
                .collect();
            let least = (0..names.len())
                .min_by(|x, y| names[*x].cmp(&names[*y]))
                .unwrap_or(0);
            let mut chain: Vec<String> = names[least..].to_vec();
            chain.extend(names[..least].iter().cloned());
            let first = &self.attr_formulas[members[least]];
            let (subject, site) = if first.from_kind {
                (Subject::entity("feature", &first.feature), Site::Header)
            } else {
                let p = KeyPath::key(&first.name);
                (
                    Subject::entity("feature", &first.feature).with_keys(&p),
                    Site::Key(p),
                )
            };
            let d = text::diag(
                codes::graph::CYCLE,
                subject,
                site,
                &format!(
                    "attribute formulas read each other in a loop: {} -> {}",
                    chain.join(" -> "),
                    chain[0]
                ),
                "break the loop: one of these attributes must not read the next",
            );
            self.diags.push(d);
        }
        for i in order {
            let a = self.attr_formulas[i].clone();
            let key = (a.feature.clone(), a.name.clone());
            let blocked = looped.contains(&i)
                || deps[i].iter().any(|j| {
                    let o = &self.attr_formulas[*j];
                    self.failed_attrs
                        .contains(&(o.feature.clone(), o.name.clone()))
                });
            let Some(p) = self.formulas.programs.get(&a.formula).filter(|_| !blocked) else {
                self.failed_attrs.insert(key);
                continue;
            };
            let result = via_expr::bind(p, &NoSelect)
                .map_err(|e| self.formulas.diag(&e))
                .and_then(|b| {
                    let reads = AttrReads {
                        features: &self.features,
                        failed: &self.failed_attrs,
                        own: &a.feature,
                    };
                    let setup = via_expr::Setup {
                        world_seed: self.seed,
                        feature: Some(&a.feature),
                    };
                    let one = [(0i64, 0i64)];
                    match via_expr::evaluate(p, &b, &reads, &setup, &via_expr::Domain::Points(&one))
                    {
                        Ok(v) => Ok(v.first().copied()),
                        Err(f) if f.error.code == codes::diag::SKIPPED => Ok(None),
                        Err(f) => Err(self
                            .formulas
                            .diag(&f.error)
                            .with("feature", via_diag::Value::text(a.feature.clone()))),
                    }
                });
            match result {
                Ok(Some(v)) => {
                    if let Some(f) = self.features.get_mut(&a.feature) {
                        f.attrs.insert(a.name.clone(), Attr::Number(v));
                    }
                }
                Ok(None) => {
                    self.failed_attrs.insert(key);
                }
                Err(d) => {
                    self.diags.push(d);
                    self.failed_attrs.insert(key);
                }
            }
        }
    }

    fn check_formulas(&mut self, root: &Table) {
        // Lets: world and field scopes, and kinds' (formulas.md 5).
        let mut lets: Vec<(FormulaId, Context)> = Vec::new();
        for (name, e) in root
            .table("let")
            .map(|t| t.iter().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            if let Some(Ok(ft)) = text::formula_text(&e.item.value) {
                let id = self
                    .formulas
                    .add(&ft, Subject::entity("let", name), KeyPath::key(name));
                self.formulas.env.world_let(name, id);
            }
        }
        for (k, t) in table_of(root, "kind") {
            if let Some(l) = t.table("let") {
                for (name, e) in l.iter() {
                    if let Some(Ok(ft)) = text::formula_text(&e.item.value) {
                        let id = self.formulas.add(
                            &ft,
                            Subject::entity("kind", k),
                            text::path(&["let", name]),
                        );
                        self.formulas.env.kind_let(k, name, id);
                    }
                }
            }
        }
        for (f, t) in table_of(root, "field") {
            if let Some(l) = t.table("let") {
                for (name, e) in l.iter() {
                    if let Some(Ok(ft)) = text::formula_text(&e.item.value) {
                        let id = self.formulas.add(
                            &ft,
                            Subject::entity("field", f),
                            text::path(&["let", name]),
                        );
                        self.formulas.env.field_let(f, name, id);
                    }
                }
            }
        }
        for e in self.formulas.env.check_lets() {
            let d = self.formulas.diag(&e);
            self.diags.push(d);
        }
        for e in self.formulas.env.shadows() {
            let d = self.formulas.diag(&e);
            self.diags.push(d);
        }
        for f in self.fields.values() {
            if let Some(b) = f.base {
                lets.push((
                    b,
                    Context::new(Key::FieldBase)
                        .field(&f.name)
                        .noise(&["noise", "field", &f.name, "base"]),
                ));
            }
            for r in &f.rows {
                if let Some(v) = r.value {
                    lets.push((
                        v,
                        Context::new(Key::ThenValue)
                            .field(&f.name)
                            .weights(f.weights)
                            .noise(&["noise", "field", &f.name, "then", &r.id, "value"]),
                    ));
                }
                if let Some(m) = r.mask {
                    lets.push((
                        m,
                        Context::new(Key::ThenMask)
                            .field(&f.name)
                            .weights(f.weights)
                            .noise(&["noise", "field", &f.name, "then", &r.id, "mask"]),
                    ));
                }
            }
        }
        for k in self.kinds.values() {
            for e in k.effects.values() {
                let radius = e.radius.or(k.radius).map(via_num::metres);
                let base = |key: Key| {
                    let c = Context::new(key).kind(&k.id);
                    match radius {
                        Some(r) => c.radius(r),
                        None => c,
                    }
                };
                if let Some(v) = e.value {
                    lets.push((
                        v,
                        base(Key::EffectValue).noise_parts(kind_noise("feature", &e.id, "value")),
                    ));
                }
                if let Some(m) = e.mask {
                    lets.push((
                        m,
                        base(Key::EffectMask)
                            .weights(e.paint.is_some())
                            .noise_parts(kind_noise("feature", &e.id, "mask")),
                    ));
                }
                if let Some(m) = e.max_change {
                    lets.push((
                        m,
                        base(Key::MaxChange).noise_parts(kind_noise(
                            "feature",
                            &e.id,
                            "max_change",
                        )),
                    ));
                }
            }
        }
        for f in self.features.values() {
            if let ShapeDecl::Where(w) = f.shape {
                lets.push((
                    w,
                    Context::new(Key::Where)
                        .feature(&f.id)
                        .noise(&["noise", "feature", &f.id, "where"]),
                ));
            }
            // A cost per metre at every post, read in the head view as `where` is (`derived.md`
            // 3; `formulas.md` 6.1).
            if let Some(r) = f.route {
                lets.push((
                    r,
                    Context::new(Key::Where)
                        .feature(&f.id)
                        .noise(&["noise", "feature", &f.id, "route"]),
                ));
            }
        }
        for i in self.imports.values() {
            if let Some(b) = i.blend {
                lets.push((
                    b,
                    Context::new(Key::Blend).noise(&["noise", "import", &i.id, "blend"]),
                ));
            }
        }
        for s in self.scatters.values() {
            if let Some(d) = s.density {
                lets.push((
                    d,
                    Context::new(Key::Density).noise(&["noise", "scatter", &s.id, "density"]),
                ));
            }
        }
        // Attribute formulas (6.7): the feature's own attributes, `<id>.<attribute>`, constant
        // world `let`s.
        for a in &self.attr_formulas {
            lets.push((
                a.formula,
                Context::new(Key::Attribute)
                    .feature(&a.feature)
                    .noise(&["noise", "feature", &a.feature, &a.name]),
            ));
        }
        for (id, ctx) in lets {
            match self.formulas.env.check(id, &ctx) {
                Ok(p) => {
                    self.formulas.programs.insert(id, p);
                }
                Err(e) => {
                    let d = self.formulas.diag(&e);
                    // A kind's default formula is checked once per feature: one record.
                    if !self.diags.contains(&d) {
                        self.diags.push(d);
                    }
                }
            }
        }
        // `formulas.md` 8.2: two noise calls of one owner (a field, a feature, a scatter, a world
        // `let`; a kind's formulas count as the kind's) on one stream.
        let mut by_owner: BTreeMap<String, Vec<&via_expr::Program>> = BTreeMap::new();
        for (id, p) in &self.formulas.programs {
            if let Some((subject, _)) = self.formulas.at.get(id.0 as usize) {
                by_owner
                    .entry(subject.address().to_string())
                    .or_default()
                    .push(p);
            }
        }
        let mut shared = Vec::new();
        for programs in by_owner.values() {
            shared.extend(via_expr::seed_shared(programs));
        }
        for e in shared {
            let d = self.formulas.diag(&e);
            self.diags.push(d);
        }
    }
}

fn d_effects(d: &mut Decls, kind: &str, t: &Table, k: &mut Kind) {
    for (eid, et) in table_of(t, "effect") {
        let sel_text = et.str("field").unwrap_or("");
        let selector = if let Some(r) = sel_text.strip_prefix("role:") {
            match Role::parse(r) {
                Some(role) => Some(Selector::Role(role)),
                None => {
                    d.push(text::with_nearest(
                        at_path(
                            codes::authority::SELECTOR,
                            "kind",
                            kind,
                            &["effect", eid, "field"],
                            &format!("`{r}` is not a role"),
                            &text::near_hint(
                                r,
                                ["surface", "material", "keep_out"],
                                "write `role:<role>` or `tag:<tag>`",
                            ),
                        ),
                        r,
                        ["surface", "material", "keep_out"],
                        text::undeclared,
                    ));
                    None
                }
            }
        } else if let Some(tag) = sel_text.strip_prefix("tag:") {
            Some(Selector::Tag(tag.to_string()))
        } else {
            d.push(at_path(
                codes::authority::SELECTOR,
                "kind",
                kind,
                &["effect", eid, "field"],
                &format!("`{sel_text}` is not a field selector"),
                "write `role:<role>` or `tag:<tag>`, never a field's name",
            ));
            None
        };
        let Some(selector) = selector else { continue };
        let get = |key: &str, d: &mut Decls| {
            et.value(key)
                .and_then(text::formula_text)
                .and_then(|r| r.ok())
                .map(|ft| {
                    d.formulas.add(
                        &ft,
                        Subject::entity("kind", kind),
                        text::path(&["effect", eid, key]),
                    )
                })
        };
        let value = get("value", d);
        let mask = get("mask", d);
        let max_change = get("max_change", d);
        let radius = match et.value("radius").and_then(text::length) {
            Some(Ok(mm)) if mm > 0 => Some(mm),
            Some(Ok(mm)) => {
                d.push(at_path(
                    codes::authority::RANGE,
                    "kind",
                    kind,
                    &["effect", eid, "radius"],
                    &format!(
                        "`radius` is {}; it must be above 0",
                        via_num::write_length(mm)
                    ),
                    "write a length above 0",
                ));
                continue;
            }
            Some(Err(e)) => {
                d.push(text::num_diag(&e, "kind", kind, &["effect", eid, "radius"]));
                continue;
            }
            None => None,
        };
        let order = match et.value("order") {
            Some(Value::Num(n)) if n.integer => n.norm.parse().unwrap_or(0),
            _ => 0,
        };
        k.effects.insert(
            eid.to_string(),
            Effect {
                id: eid.to_string(),
                selector,
                value,
                mask,
                paint: et.str("paint").map(str::to_string),
                radius: radius.or(k.radius),
                max_change,
                order,
            },
        );
    }
}
