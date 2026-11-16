//! The closed keys of every table, and what each key takes (`spec/vocabulary.md` 3 and 5,
//! `spec/documents.md` 7).
//!
//! The lists are the vocabulary's, for the tables a world, a layer, a kinds file and a change
//! file hold. A pack's keys are `assets.md`'s and are checked by the pack reader.

/// What a key accepts (`vocabulary.md` 2.4), with the extra forms its rows name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    /// Free text, or a word from a list.
    Str,
    /// An id or a place name.
    Name,
    /// A TOML integer.
    Integer,
    /// A finite double.
    Number,
    /// Metres on the lattice.
    Length,
    /// A `face`: a bearing in degrees, or a word.
    Facing,
    /// `true` or `false`.
    Bool,
    /// A formula string or a number.
    Formula,
    /// `[x, y]`.
    Point,
    /// A list of points.
    Points,
    /// A path's way points: each a point or a name (`derived.md` 3).
    Ways,
    /// So many lengths.
    Lengths(usize),
    /// So many numbers.
    Numbers(usize),
    /// A list of strings.
    Strings,
    /// A list of names.
    Names,
    /// A number, or two numbers (`scale`, `tint` on a scatter).
    NumberOrPair,
    /// A name or `{ of, class, near }` (`within`).
    NameOrSelect,
    /// A scatter's `assets`: each an asset id, `@<taxon path>` or `{ id, weight }`.
    Choices,
    /// The world's seed: an integer.
    WorldSeed,
    /// A rule's seed: a name.
    RuleSeed,
    /// A table of a known shape.
    Table(Shape),
    /// A table of entries keyed by id, each of a shape (`then`, `effect`).
    Ids(Shape),
    /// `params`: any keys, handed on unread.
    Free,
    /// A kind's `defaults`: any keys, each declaring an attribute.
    Defaults,
    /// An import's `footprint`: a name, or a list of points (`composition.md` 2).
    Footprint,
    /// A kind's `exit`: `true`, or a list of attribute names (`composition.md` 7).
    Exit,
    /// A given's `default`: a formula, a number or a selector (`composition.md` 8).
    Given,
}

impl Takes {
    /// The words `doc.type` uses for what the key takes.
    pub fn words(self) -> &'static str {
        match self {
            Takes::Str => "a string",
            Takes::Name => "a name",
            Takes::Integer => "an integer",
            Takes::Number => "a number",
            Takes::Length => "a length in metres",
            Takes::Facing => "a bearing in degrees or a word",
            Takes::Bool => "`true` or `false`",
            Takes::Formula => "a formula string or a number",
            Takes::Point => "a point `[x, y]`",
            Takes::Points => "a list of points",
            Takes::Ways => "a list of points and names",
            Takes::Lengths(4) => "4 lengths",
            Takes::Lengths(3) => "3 lengths",
            Takes::Lengths(_) => "a list of lengths",
            Takes::Numbers(2) => "2 numbers",
            Takes::Numbers(3) => "3 numbers",
            Takes::Numbers(_) => "a list of numbers",
            Takes::Strings => "a list of strings",
            Takes::Names => "a list of names",
            Takes::NumberOrPair => "a number, or 2 numbers",
            Takes::NameOrSelect => "a name or `{ of, class, near }`",
            Takes::Choices => "a list of asset ids or `{ id, weight }` tables",
            Takes::WorldSeed => "an integer",
            Takes::Footprint => "a name or a list of points",
            Takes::Exit => "`true` or a list of attribute names",
            Takes::Given => "a formula, a number or a selector",
            Takes::RuleSeed => "a name",
            Takes::Table(_) | Takes::Ids(_) | Takes::Free | Takes::Defaults => "a table",
        }
    }
}

/// The tables whose keys are closed lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// `[layer.<id>]`.
    Layer,
    /// `[field.<name>]`.
    Field,
    /// `[field.<name>.then.<id>]`.
    Then,
    /// `[feature.<id>]`: reserved keys, then its kind's attributes.
    Feature,
    /// `[kind.<name>]`.
    Kind,
    /// `[kind.<k>.effect.<id>]`.
    Effect,
    /// `[kind.<k>.surface.<name>]` (`surfaces.md` 1, 1.3).
    KindSurface,
    /// `[kind.<k>.mount.<name>]` (`surfaces.md` 3.1).
    KindMount,
    /// `[instance.<id>]`.
    Instance,
    /// `[scatter.<id>]`.
    Scatter,
    /// `[pin.<id>]`.
    Pin,
    /// `[clear.<id>]`.
    Clear,
    /// `[step.<id>]`.
    Step,
    /// `[network.<id>]` (`geometry.md` 8.1).
    Network,
    /// `[cells.<id>]` (`geometry.md` 8.2).
    Cells,
    /// `[set.<id>]` (`rules.md` 4).
    Set,
    /// `[set.<id>.item.<name>]`.
    Item,
    /// `[run.<id>]` (`rules.md` 5).
    Run,
    /// `on`.
    On,
    /// `follows`.
    Follows,
    /// `{ of, class, near }` in `within`.
    Select,
    /// `{ id, weight }` in `assets`.
    Choice,
    /// `[import.<id>]` (`composition.md` 2).
    Import,
    /// `[given.<name>]` (`composition.md` 8).
    Given,
    /// `[budget]` (`build.md` 5.1).
    Budget,
    /// `[budget.waive.<id>]`.
    Waive,
    /// `[let]`, `[field.<n>.let]`, `[kind.<k>.let]`: names to formulas.
    Let,
}

/// What a key is at its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A listed key.
    Ok(Takes),
    /// A word of a later milestone.
    NotYet(u32),
    /// A removed or forbidden word with its fixed hint (`documents.md` 7).
    Fixed(&'static str),
}

/// The placement keys, which exclude each other (`vocabulary.md` 2.5).
pub const PLACEMENT: [&str; 6] = ["at", "on", "path", "region", "where", "follows"];

/// Keys only a path takes, cleared when a change writes another placement key (6.1).
pub const PATH_ONLY: [&str; 3] = ["starts_at", "ends_at", "smooth"];

/// Reserved keys of a feature (`vocabulary.md` 2.3).
pub const FEATURE_RESERVED: [&str; 16] = [
    "kind",
    "authority",
    "at",
    "on",
    "path",
    "region",
    "where",
    "follows",
    "smooth",
    "starts_at",
    "ends_at",
    "face",
    "holds",
    "route",
    "remove",
    "unset",
];

/// Tables written onto by value, replaced whole by a change (6.2).
pub const VALUE_TABLES: [&str; 3] = ["on", "follows", "clump"];

/// Top-level keys of the world file that are not top tables.
pub const WORLD_KEYS: [&str; 9] = [
    "name",
    "datum",
    "extent",
    "resolution",
    "seed",
    "include",
    "pack",
    "chunk",
    "budget",
];

/// Top tables a world or layer may hold, with M1's rule for each.
pub fn top_table(name: &str) -> Option<Rule> {
    Some(match name {
        "let" => Rule::Ok(Takes::Table(Shape::Let)),
        "field" => Rule::Ok(Takes::Ids(Shape::Field)),
        "feature" => Rule::Ok(Takes::Ids(Shape::Feature)),
        "instance" => Rule::Ok(Takes::Ids(Shape::Instance)),
        "scatter" => Rule::Ok(Takes::Ids(Shape::Scatter)),
        "pin" => Rule::Ok(Takes::Ids(Shape::Pin)),
        "clear" => Rule::Ok(Takes::Ids(Shape::Clear)),
        "step" => Rule::Ok(Takes::Ids(Shape::Step)),
        "network" => Rule::Ok(Takes::Ids(Shape::Network)),
        "cells" => Rule::Ok(Takes::Ids(Shape::Cells)),
        "set" => Rule::Ok(Takes::Ids(Shape::Set)),
        "run" => Rule::Ok(Takes::Ids(Shape::Run)),
        "lock" => Rule::NotYet(3),
        "import" => Rule::Ok(Takes::Ids(Shape::Import)),
        "given" => Rule::Ok(Takes::Ids(Shape::Given)),
        "params" => Rule::Fixed("write `[let]`"),
        "variant" => Rule::Fixed("write `[change.<n>]`"),
        _ => return None,
    })
}

/// Every top table name a world lists, for nearest-word hints.
pub const TOP_TABLES: [&str; 16] = [
    "cells", "change", "clear", "feature", "field", "given", "import", "instance", "layer", "let",
    "lock", "network", "pin", "run", "scatter", "set",
];

/// Whether an import id is reserved: a top table's name, or another collection a diagnostic's
/// subject names (`identity.md` 1).
pub fn reserved_import(id: &str) -> bool {
    TOP_TABLES.contains(&id) || ["asset", "kind", "material", "step", "world"].contains(&id)
}

/// The world file's own top-level keys.
pub fn world_key(name: &str) -> Option<Rule> {
    Some(match name {
        "name" => Rule::Ok(Takes::Str),
        "extent" => Rule::Ok(Takes::Lengths(4)),
        "resolution" => Rule::Ok(Takes::Length),
        "seed" => Rule::Ok(Takes::WorldSeed),
        "include" => Rule::Ok(Takes::Strings),
        "pack" => Rule::Ok(Takes::Str),
        "datum" => Rule::Ok(Takes::Name),
        "chunk" => Rule::Ok(Takes::Length),
        "budget" => Rule::Ok(Takes::Table(Shape::Budget)),
        _ => return None,
    })
}

/// The keys of a shape, for the closed check and nearest-word hints.
pub fn keys(shape: Shape) -> &'static [(&'static str, Rule)] {
    use Rule::{Fixed, NotYet, Ok};
    use Takes as T;
    match shape {
        Shape::Layer => &[
            ("file", Ok(T::Str)),
            ("owner", Ok(T::Str)),
            ("rank", Ok(T::Integer)),
            (
                "after",
                Fixed("`after` was dropped: rank the layer above the layer that declares the import (`composition.md` 6.5)"),
            ),
        ],
        Shape::Field => &[
            ("role", Ok(T::Str)),
            ("type", Ok(T::Str)),
            ("base", Ok(T::Formula)),
            ("file", Ok(T::Str)),
            ("channel", Ok(T::Str)),
            ("range", Ok(T::Numbers(2))),
            ("extent", Ok(T::Lengths(4))),
            ("outside", Ok(T::Number)),
            ("filter", Ok(T::Str)),
            ("unit", Ok(T::Str)),
            ("then", Ok(T::Ids(Shape::Then))),
            ("taxon", Ok(T::Str)),
            ("tags", Ok(T::Strings)),
            ("let", Ok(T::Table(Shape::Let))),
        ],
        Shape::Then => &[
            ("value", Ok(T::Formula)),
            ("mask", Ok(T::Formula)),
            ("paint", Ok(T::Name)),
            ("order", Ok(T::Integer)),
        ],
        Shape::Feature => &[
            ("kind", Ok(T::Name)),
            ("authority", Ok(T::Str)),
            ("at", Ok(T::Point)),
            ("on", Ok(T::Table(Shape::On))),
            ("path", Ok(T::Ways)),
            ("region", Ok(T::Points)),
            ("where", Ok(T::Formula)),
            ("follows", Ok(T::Table(Shape::Follows))),
            ("smooth", Ok(T::Bool)),
            ("starts_at", Ok(T::Name)),
            ("ends_at", Ok(T::Name)),
            ("face", Ok(T::Facing)),
            ("surface", Ok(T::Name)),
            ("holds", Ok(T::Point)),
            ("route", Ok(T::Formula)),
            ("over", Fixed("`over` was dropped; change one decision, see `authority.md` 5")),
            ("joins", Fixed("write `starts_at` or `ends_at`")),
        ],
        Shape::Kind => &[
            ("authority", Ok(T::Str)),
            ("defaults", Ok(T::Defaults)),
            ("requires", Ok(T::Strings)),
            ("radius", Ok(T::Length)),
            ("exit", Ok(T::Exit)),
            ("output", Ok(T::Str)),
            ("names", Ok(T::Strings)),
            ("effect", Ok(T::Ids(Shape::Effect))),
            ("surface", Ok(T::Ids(Shape::KindSurface))),
            ("mount", Ok(T::Ids(Shape::KindMount))),
            ("connector", Ok(T::Str)),
            ("let", Ok(T::Table(Shape::Let))),
        ],
        Shape::KindSurface => &[
            ("base", Ok(T::Formula)),
            ("bound", Ok(T::Formula)),
            ("cut", Ok(T::Formula)),
            ("material", Ok(T::Str)),
        ],
        Shape::KindMount => &[
            ("t", Ok(T::Number)),
            ("offset", Ok(T::Length)),
            ("face", Ok(T::Facing)),
            ("surface", Ok(T::Name)),
            ("meets", Ok(T::Name)),
            ("connector", Ok(T::Str)),
        ],
        Shape::Effect => &[
            ("field", Ok(T::Str)),
            ("value", Ok(T::Formula)),
            ("mask", Ok(T::Formula)),
            ("paint", Ok(T::Name)),
            ("radius", Ok(T::Length)),
            ("max_change", Ok(T::Formula)),
            ("order", Ok(T::Integer)),
        ],
        Shape::Instance => &[
            ("asset", Ok(T::Name)),
            ("set", Ok(T::Name)),
            ("at", Ok(T::Point)),
            ("on", Ok(T::Table(Shape::On))),
            ("face", Ok(T::Facing)),
            ("surface", Ok(T::Name)),
            ("scale", Ok(T::Number)),
            ("tint", Ok(T::Number)),
            ("ground", Ok(T::Name)),
            ("margin", Ok(T::Length)),
            (
                "authority",
                Fixed(
                    "the footprint takes its `ground` kind's tier; ground with a kind of the tier you want",
                ),
            ),
        ],
        Shape::Scatter => &[
            ("assets", Ok(T::Choices)),
            ("spacing", Ok(T::Length)),
            ("jitter", Ok(T::Number)),
            ("density", Ok(T::Formula)),
            ("within", Ok(T::NameOrSelect)),
            ("surface", Ok(T::Name)),
            ("scale", Ok(T::NumberOrPair)),
            ("tint", Ok(T::NumberOrPair)),
            ("clump", Ok(T::Free)),
            ("seed", Ok(T::RuleSeed)),
        ],
        Shape::Pin => &[
            ("asset", Ok(T::Name)),
            ("set", NotYet(3)),
            ("at", Ok(T::Point)),
            ("on", Ok(T::Table(Shape::On))),
            ("face", Ok(T::Facing)),
            ("by", Ok(T::Names)),
        ],
        Shape::Clear => &[
            ("at", Ok(T::Point)),
            ("on", Ok(T::Table(Shape::On))),
            ("radius", Ok(T::Length)),
            ("region", Ok(T::Points)),
            ("by", Ok(T::Names)),
        ],
        Shape::Step => &[
            ("when", Ok(T::Str)),
            ("command", Ok(T::Strings)),
            ("worker", Ok(T::Bool)),
            ("params", Ok(T::Free)),
            ("inputs", Ok(T::Strings)),
            ("for_each", Ok(T::Str)),
            ("checks", Ok(T::Strings)),
        ],
        Shape::Network => &[
            ("from", Ok(T::Names)),
            ("within", Ok(T::Name)),
            ("kind", Ok(T::Name)),
        ],
        Shape::Cells => &[
            ("within", Ok(T::NameOrSelect)),
            ("frontage", Ok(T::Names)),
            ("spacing", Ok(T::Length)),
            ("depth", Ok(T::Length)),
            ("min_length", Ok(T::Length)),
            ("min_area", Ok(T::Number)),
            ("kind", Ok(T::Name)),
        ],
        Shape::Set => &[("item", Ok(T::Ids(Shape::Item)))],
        Shape::Item => &[
            ("asset", Ok(T::Name)),
            ("set", Ok(T::Name)),
            ("at", Ok(T::Point)),
            ("face", Ok(T::Facing)),
            ("scale", Ok(T::NumberOrPair)),
            ("tint", Ok(T::NumberOrPair)),
        ],
        // No `seed`: a run draws nothing (`rules.md` 5), so `seed` is `doc.unknown_key`.
        Shape::Run => &[
            ("asset", Ok(T::Name)),
            ("set", Ok(T::Name)),
            ("on", Ok(T::Table(Shape::On))),
            ("count", Ok(T::Integer)),
            ("spacing", Ok(T::Length)),
            ("ends", Ok(T::Str)),
            ("face", Ok(T::Facing)),
            ("scale", Ok(T::NumberOrPair)),
            ("tint", Ok(T::NumberOrPair)),
            ("ground", Ok(T::Name)),
            ("margin", Ok(T::Length)),
        ],
        Shape::On => &[
            ("ref", Ok(T::Name)),
            ("of", Ok(T::Name)),
            ("class", Ok(T::Str)),
            ("near", Ok(T::Point)),
            ("t", Ok(T::Number)),
            // A run's (`rules.md` 4); in an instance's `on` it is `place.on_form`, said by the
            // model, which knows what holds the `on`.
            ("span", Ok(T::Numbers(2))),
            ("offset", Ok(T::Length)),
            ("feature", Fixed("write `ref`")),
        ],
        Shape::Follows => &[
            ("ref", Ok(T::Name)),
            ("span", Ok(T::Numbers(2))),
            ("offset", Ok(T::Length)),
        ],
        Shape::Select => &[
            ("of", Ok(T::Name)),
            ("class", Ok(T::Str)),
            ("near", Ok(T::Point)),
        ],
        Shape::Choice => &[("id", Ok(T::Name)), ("weight", Ok(T::Number))],
        Shape::Import => &[
            ("file", Ok(T::Str)),
            ("at", Ok(T::Point)),
            ("on", Ok(T::Table(Shape::On))),
            ("face", Ok(T::Facing)),
            ("seed", Ok(T::WorldSeed)),
            ("changes", Ok(T::Strings)),
            ("footprint", Ok(T::Footprint)),
            ("blend", Ok(T::Formula)),
            ("authority", Ok(T::Str)),
            ("lift", Ok(T::Length)),
            ("max_grade", Ok(T::Number)),
            // `composition.md` 8: names to what is given, read by the model.
            ("give", Ok(T::Free)),
        ],
        Shape::Given => &[
            ("accepts", Ok(T::Str)),
            ("default", Ok(T::Given)),
            ("kind", Ok(T::Name)),
        ],
        Shape::Budget => &[
            ("triangles", Ok(T::Integer)),
            ("instances", Ok(T::Integer)),
            ("groups", Ok(T::Integer)),
            ("waive", Ok(T::Ids(Shape::Waive))),
        ],
        Shape::Waive => &[
            ("at", Ok(T::Numbers(2))),
            ("triangles", Ok(T::Integer)),
            ("instances", Ok(T::Integer)),
            ("groups", Ok(T::Integer)),
            ("reason", Ok(T::Str)),
        ],
        Shape::Let => &[],
    }
}

/// The rule of one key of a shape.
pub fn rule(shape: Shape, key: &str) -> Option<Rule> {
    keys(shape).iter().find(|(k, _)| *k == key).map(|(_, r)| *r)
}

/// The listed keys of a shape that a user may write now, for nearest-word hints.
pub fn listed(shape: Shape) -> impl Iterator<Item = &'static str> {
    keys(shape)
        .iter()
        .filter(|(_, r)| matches!(r, Rule::Ok(_)))
        .map(|(k, _)| *k)
}

/// Values of a key that the vocabulary puts in a later milestone (`vocabulary.md` 5).
pub fn value_not_yet(shape: Shape, key: &str, value: &str) -> Option<u32> {
    match (shape, key, value) {
        (Shape::Step, "when", "export") => Some(6),
        _ => None,
    }
}

/// The shape of an entity in a top table.
pub fn entity_shape(top: &str) -> Option<Shape> {
    match top_table(top) {
        Some(Rule::Ok(Takes::Ids(s))) => Some(s),
        _ => match top {
            "kind" => Some(Shape::Kind),
            "layer" => Some(Shape::Layer),
            _ => None,
        },
    }
}

/// True for a top table whose ids are in the world namespace (`documents.md` 3).
pub fn in_world_namespace(top: &str) -> bool {
    matches!(
        top,
        "field"
            | "feature"
            | "instance"
            | "set"
            | "run"
            | "scatter"
            | "pin"
            | "clear"
            | "lock"
            | "import"
            | "given"
            | "step"
            | "layer"
            | "network"
            | "cells"
    )
}

/// The named-reference keys of a shape (`documents.md` 5.1), each with whether it holds a list.
pub fn name_keys(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::Feature => &["starts_at", "ends_at", "path"],
        Shape::Pin | Shape::Clear => &["by"],
        Shape::Scatter => &["within"],
        Shape::On | Shape::Select => &["ref", "of"],
        Shape::Import => &["footprint"],
        Shape::Follows => &["ref"],
        Shape::Network => &["from", "within"],
        Shape::Cells => &["within", "frontage"],
        _ => &[],
    }
}
