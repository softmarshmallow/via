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
    pub rows: Vec<Row>,
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

/// A placement: a coordinate or a place on a named thing.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    At((i64, i64)),
    On(On),
}

/// A feature's written shape.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeDecl {
    Point(Place),
    Path(Vec<(i64, i64)>),
    Region(Vec<(i64, i64)>),
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

/// A scatter (`rules.md` 7).
#[derive(Clone, Debug)]
pub struct ScatterDecl {
    pub id: String,
    pub choices: Vec<Choice>,
    pub spacing: i64,
    pub jitter: i64,
    pub density: Option<FormulaId>,
    pub within: Option<String>,
    pub scale: Ranged,
    pub tint: Ranged,
    pub seed: String,
    #[allow(dead_code)] // layer rank: giving way and hand edits read down only (`rules.md` 3.4)
    pub rank: i64,
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
}

impl Formulas {
    fn add(&mut self, ft: &FormulaText, subject: Subject, path: KeyPath) -> FormulaId {
        let id = match ft {
            FormulaText::Text(t) => self.env.text(t),
            FormulaText::Number(n) => self.env.number(*n),
        };
        debug_assert_eq!(id.0 as usize, self.at.len());
        self.at.push((subject, path));
        id
    }

    /// The diagnostic of a formula fault, located at the formula's key.
    pub fn diag(&self, e: &via_expr::ExprError) -> Diag {
        let subject_of = |f: Option<FormulaId>, at: &via_expr::At| -> Subject {
            match f.or_else(|| at.formula()) {
                // One key of an entity: the address and the key path (`diagnostics.md` 1.1).
                Some(f) => self
                    .at
                    .get(f.0 as usize)
                    .map(|(s, p)| match s.key_path() {
                        None => s.clone().with_keys(p),
                        Some(_) => s.clone(),
                    })
                    .unwrap_or(Subject::World),
                None => Subject::World,
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
                via_expr::At::Declaration { .. } => Site::World,
            }
        };
        e.to_diag(&subject_of, &site_of)
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
    pub instances: BTreeMap<String, InstanceDecl>,
    pub scatters: BTreeMap<String, ScatterDecl>,
    pub pins: BTreeMap<String, PinDecl>,
    pub clears: BTreeMap<String, ClearDecl>,
    pub steps: BTreeMap<String, StepDecl>,
    pub formulas: Formulas,
    pub diags: Vec<Diag>,
}

fn table_of<'w>(root: &'w Table, top: &str) -> Vec<(&'w str, &'w Table)> {
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
    /// Reads the merged world.
    pub fn read(world: &World) -> Decls {
        let mut d = Decls::default();
        let root = world.root();
        d.read_world_keys(root);
        d.read_kinds(root);
        d.read_fields(world, root);
        d.read_features(world, root);
        d.read_instances(world, root);
        d.read_scatters(world, root);
        d.read_edits(world, root);
        d.read_steps(root);
        d.declare_env();
        d.check_formulas(root);
        d
    }

    fn push(&mut self, diag: Diag) {
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
                    None => self.push(at_key(
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
                        self.push(at_key(
                            codes::authority::NAMES_UNDECLARED,
                            "kind",
                            id,
                            "names",
                            &format!("`{n}` in `names` is in neither `defaults` nor `requires`"),
                            &text::near_hint(n, declared, "declare it in `requires`"),
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
            if t.get("exit").is_some() {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "kind",
                    id,
                    "exit",
                    "`exit` on a kind arrives in M4",
                    "remove `exit` until M4",
                ));
            }
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
                    self.push(at_key(
                        codes::field::TYPE_UNKNOWN,
                        "field",
                        name,
                        "type",
                        &format!("`{other}` is not a field type"),
                        &hint,
                    ));
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
                        self.push(at_key(
                            codes::field::ROLE_UNKNOWN,
                            "field",
                            name,
                            "role",
                            &format!("`{r}` is not a role"),
                            &text::near_hint(
                                r,
                                ["surface", "material", "keep_out"],
                                "a library word belongs in `tags`",
                            ),
                        ));
                        None
                    }
                },
            };
            let unit = t.str("unit").map(str::to_string);
            if let Some(u) = &unit {
                if !["m", "deg", "ratio"].contains(&u.as_str()) {
                    self.push(at_key(
                        codes::field::UNIT_UNKNOWN,
                        "field",
                        name,
                        "unit",
                        &format!("`{u}` is not a unit"),
                        &text::near_hint(u, ["m", "deg", "ratio"], "write `m`, `deg` or `ratio`"),
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
            if t.get("file").is_some() {
                if t.get("base").is_some() {
                    self.push(at_key(
                        codes::field::BASE_AND_FILE,
                        "field",
                        name,
                        "file",
                        "a field has both `base` and `file`",
                        "keep one",
                    ));
                }
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "field",
                    name,
                    "file",
                    "a field read from a `file` is not built by this model yet",
                    "write a `base` formula",
                ));
            }
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
                rows.push(Row {
                    id: rid.to_string(),
                    value,
                    mask,
                    paint,
                    order,
                    rank,
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
        if on.get("of").is_some() {
            if on.get("ref").is_some() {
                self.push(at_key(
                    codes::identity::ON_BOTH,
                    top,
                    id,
                    "on",
                    "`on` has both `ref` and `of`",
                    "keep `ref` for an authored thing, `of` for a produced one",
                ));
            } else {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    top,
                    id,
                    "on",
                    "a selection by place `{ of, near }` is not built by this model yet",
                    "name an authored thing with `ref`",
                ));
            }
            return None;
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
        let t_ = match on.value("t") {
            Some(v) => match text::number(v) {
                Some(Ok(x)) => Some(x),
                Some(Err(e)) => {
                    self.push(text::num_diag(&e, top, id, &["on", "t"]));
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
                    self.push(text::num_diag(&e, top, id, &["on", "offset"]));
                    return None;
                }
                None => None,
            },
            None => None,
        };
        Some(Place::On(On {
            reference,
            t: t_,
            offset,
        }))
    }

    fn read_face(&mut self, top: &str, id: &str, t: &Table) -> Option<Face> {
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
                    self.push(at_key(
                        codes::geom::FACE_UNKNOWN,
                        top,
                        id,
                        "face",
                        &format!("`{w}` is not a facing word"),
                        &text::near_hint(
                            w,
                            ["along", "toward", "away"],
                            "write a bearing in degrees, or `along`, `toward`, `away`",
                        ),
                    ));
                    None
                }
            },
            _ => None,
        }
    }

    fn read_features(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "feature") {
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
                        None => self.push(at_key(
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
                        )),
                    }
                }
            }
            if let Some(k) = &kind {
                if !self.kinds.contains_key(k) {
                    self.push(at_key(
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
                    ));
                    continue;
                }
            }
            for key in ["where", "follows", "smooth"] {
                if t.get(key).is_some() {
                    self.push(at_key(
                        codes::doc::NOT_YET,
                        "feature",
                        id,
                        key,
                        &format!("`{key}` is not built by this model yet"),
                        "write the shape with `path`, `region`, `at` or `on`",
                    ));
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
                match text::points(v) {
                    Some(Ok(p)) => ShapeDecl::Path(p),
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
            let mut attrs = BTreeMap::new();
            if let Some(k) = kind.as_ref().and_then(|k| self.kinds.get(k)).cloned() {
                for (n, v) in &k.defaults {
                    attrs.insert(n.clone(), v.clone());
                }
                let declared: BTreeSet<&String> =
                    k.defaults.keys().chain(k.requires.iter()).collect();
                for (n, e) in t.iter() {
                    if declared.contains(n) {
                        let v = match &e.item.value {
                            Value::Num(x) => text::num(x).ok().map(Attr::Number),
                            Value::Bool(b) => Some(Attr::YesNo(*b)),
                            Value::Str(s) => Some(Attr::Text(s.text.clone())),
                            _ => None,
                        };
                        if let Some(v) = v {
                            attrs.insert(n.clone(), v);
                        }
                    }
                }
                for r in &k.requires {
                    if !attrs.contains_key(r) {
                        self.push(at_header(
                            codes::authority::REQUIRES,
                            "feature",
                            id,
                            &format!("`{id}` lacks `{r}`, which its kind `{}` requires", k.id),
                            &format!("write `{r} = ...` on `{id}`"),
                        ));
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
                },
            );
        }
    }

    fn read_scale_tint(
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
            if t.get("set").is_some() {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "instance",
                    id,
                    "set",
                    "a `set` arrives in M3",
                    "place an `asset`",
                ));
                continue;
            }
            let Some(asset) = t.str("asset").map(str::to_string) else {
                self.push(at_header(
                    codes::place::ASSET_OR_SET,
                    "instance",
                    id,
                    "an instance writes neither `asset` nor `set`",
                    "keep one of `asset`, `set`",
                ));
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
                        self.push(at_key(
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
                        ));
                        continue;
                    }
                    Some(k) if !k.requires.is_empty() => {
                        self.push(at_key(
                            codes::authority::GROUND_REQUIRES,
                            "instance",
                            id,
                            "ground",
                            &format!(
                                "the kind `{g}` requires `{}`, which a footprint cannot write",
                                k.requires.join("`, `")
                            ),
                            "give the attribute a default in the kind, or ground with another kind",
                        ));
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
                },
            );
        }
        for top in ["set", "run", "lock", "import", "given"] {
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
                Some(_) => {
                    self.push(at_key(
                        codes::doc::NOT_YET,
                        "scatter",
                        id,
                        "within",
                        "`within` by selection is not built by this model yet",
                        "name a region feature",
                    ));
                    continue;
                }
                None => None,
            };
            if t.get("clump").is_some() {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "scatter",
                    id,
                    "clump",
                    "`clump` arrives in M2",
                    "remove `clump` until M2",
                ));
            }
            let Some(scale) = self.read_scale_tint("scatter", id, t, "scale", true) else {
                continue;
            };
            let Some(tint) = self.read_scale_tint("scatter", id, t, "tint", true) else {
                continue;
            };
            let seed = t.str("seed").unwrap_or(id).to_string();
            self.scatters.insert(
                id.to_string(),
                ScatterDecl {
                    id: id.to_string(),
                    choices,
                    spacing,
                    jitter,
                    density,
                    within,
                    scale,
                    tint,
                    seed,
                    rank: world.rank("scatter", id).unwrap_or(0),
                },
            );
        }
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
            if when != "lint" {
                self.push(at_key(
                    codes::doc::NOT_YET,
                    "step",
                    id,
                    "when",
                    &format!("a `{when}` step arrives in M2; M1 runs one lint"),
                    "write `when = \"lint\"`",
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
        for (id, k) in &self.kinds {
            for (a, v) in &k.defaults {
                let ty = match v {
                    Attr::Number(_) => AttrType::Number,
                    Attr::YesNo(_) => AttrType::YesNo,
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
        for s in self.scatters.values() {
            if let Some(d) = s.density {
                lets.push((
                    d,
                    Context::new(Key::Density).noise(&["noise", "scatter", &s.id, "density"]),
                ));
            }
        }
        for (id, ctx) in lets {
            match self.formulas.env.check(id, &ctx) {
                Ok(p) => {
                    self.formulas.programs.insert(id, p);
                }
                Err(e) => {
                    let d = self.formulas.diag(&e);
                    self.diags.push(d);
                }
            }
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
                    d.push(at_path(
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
