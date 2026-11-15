//! Formula keys and the names each context sees (`formulas.md` 6).

use crate::table::Rate;

/// A formula key of `formulas.md` 6.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// A world `[let]` value (6.2).
    WorldLet,
    /// A `[field.<n>.let]` value (6.3).
    FieldLet,
    /// A field's `base` (6.3).
    FieldBase,
    /// A `then` row's `value` (6.4).
    ThenValue,
    /// A `then` row's `mask` (6.4).
    ThenMask,
    /// A kind effect's `value` (6.5).
    EffectValue,
    /// A kind effect's `mask` (6.5).
    EffectMask,
    /// A kind effect's `max_change` (6.5, without `v`, `d`, `sd`, `t`).
    MaxChange,
    /// A `[kind.<k>.let]` value (6.5).
    KindLet,
    /// A feature's `where` (6.6).
    Where,
    /// A scatter's `density` (6.8).
    Density,
    /// An attribute, or a kind's `defaults` (6.7).
    Attribute,
    /// A kind surface's `base` (M3).
    SurfaceBase,
    /// A kind surface's `bound` (M3).
    SurfaceBound,
    /// An import's `blend` (M4).
    Blend,
    /// A volume `base` or an effect on a volume (M5).
    Volume,
}

/// What a key's result may be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Results {
    Number,
    /// Number or yes/no (`let` values).
    Either,
    /// Yes/no, or a number read as "above 0" (`where`).
    WhereRegion,
}

/// A key's range (6.1), never clamped silently.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyRange {
    None,
    /// 0 to 1.
    Unit,
    /// 0 or more.
    NonNegative,
}

impl Key {
    /// The milestone the key arrives in; a later one is `doc.not_yet` now.
    pub fn milestone(self) -> u32 {
        match self {
            Key::SurfaceBase | Key::SurfaceBound => 3,
            Key::Blend => 4,
            Key::Volume => 5,
            _ => 1,
        }
    }

    /// The highest rate (6.1).
    pub fn max_rate(self) -> Rate {
        match self {
            Key::MaxChange | Key::Attribute | Key::SurfaceBound => Rate::PerFeature,
            _ => Rate::PerPost,
        }
    }

    /// The result types it takes.
    pub fn results(self) -> Results {
        match self {
            Key::WorldLet | Key::FieldLet | Key::KindLet => Results::Either,
            Key::Where => Results::WhereRegion,
            _ => Results::Number,
        }
    }

    /// Its range.
    pub fn range(self) -> KeyRange {
        match self {
            Key::ThenMask | Key::EffectMask | Key::Density | Key::Blend => KeyRange::Unit,
            Key::MaxChange => KeyRange::NonNegative,
            _ => KeyRange::None,
        }
    }

    /// The words a message uses for it.
    pub fn words(self) -> &'static str {
        match self {
            Key::WorldLet => "a world `let`",
            Key::FieldLet => "a field `let`",
            Key::FieldBase => "a field base",
            Key::ThenValue => "a `then` row's value",
            Key::ThenMask => "a `then` row's mask",
            Key::EffectValue => "a kind effect's value",
            Key::EffectMask => "a kind effect's mask",
            Key::MaxChange => "a kind effect's `max_change`",
            Key::KindLet => "a kind `let`",
            Key::Where => "a feature's `where`",
            Key::Density => "a scatter's density",
            Key::Attribute => "an attribute",
            Key::SurfaceBase => "a kind surface's base",
            Key::SurfaceBound => "a kind surface's bound",
            Key::Blend => "an import's blend",
            Key::Volume => "a volume formula",
        }
    }

    /// Whether the key is one of a kind's (6.5): it sees the kind's attributes and `let` names.
    pub fn in_kind(self) -> bool {
        matches!(
            self,
            Key::EffectValue | Key::EffectMask | Key::MaxChange | Key::KindLet
        )
    }

    /// Whether the key is one of a field's (6.3, 6.4): it sees the field's `let` names.
    pub fn in_field(self) -> bool {
        matches!(
            self,
            Key::FieldLet | Key::FieldBase | Key::ThenValue | Key::ThenMask
        )
    }
}

/// A built-in name (`formulas.md` 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Pi,
    X,
    Y,
    D,
    Sd,
    T,
    V,
    Z,
}

/// The built-in names and the keywords: a `let` or attribute named so is `formula.shadow`.
pub const RESERVED: &[&str] = &["pi", "x", "y", "d", "sd", "t", "v", "z", "and", "or", "not"];

impl Builtin {
    pub fn parse(name: &str) -> Option<Builtin> {
        Some(match name {
            "pi" => Builtin::Pi,
            "x" => Builtin::X,
            "y" => Builtin::Y,
            "d" => Builtin::D,
            "sd" => Builtin::Sd,
            "t" => Builtin::T,
            "v" => Builtin::V,
            "z" => Builtin::Z,
            _ => return None,
        })
    }

    pub fn rate(self) -> Rate {
        match self {
            Builtin::Pi => Rate::Constant,
            _ => Rate::PerPost,
        }
    }

    /// Where the name exists, for the hint of `formula.unknown_name`.
    pub fn exists_in(self) -> &'static str {
        match self {
            Builtin::D | Builtin::Sd | Builtin::T => "a kind effect",
            Builtin::V => "a `then` row and a kind effect on a scalar field",
            Builtin::Z => "a volume formula (M5)",
            _ => "every formula",
        }
    }
}

/// One stream part of a default noise seed (`formulas.md` 8.2): text, or the id of the feature
/// the formula is evaluated for (a kind's formulas serve many features).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoisePart {
    Text(String),
    Feature,
}

/// Where a formula is checked: its key and the scope and facts that key's names come from.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub key: Key,
    /// The field whose `let` names are visible (6.3, 6.4).
    pub field: Option<String>,
    /// The kind whose attributes and `let` names are visible (6.5).
    pub kind: Option<String>,
    /// The feature whose own attributes are visible (`where`, an attribute formula).
    pub feature: Option<String>,
    /// The written field is a `weights` field: no `v` (6.4, 6.5).
    pub weights: bool,
    /// The effect's `radius`, else the kind's, in metres: the default `cap` (8.4).
    pub radius: Option<f64>,
    /// The parts of the formula's default noise stream (8.2).
    pub noise: Vec<NoisePart>,
}

impl Context {
    /// A context for `key` with nothing else set.
    pub fn new(key: Key) -> Context {
        Context {
            key,
            field: None,
            kind: None,
            feature: None,
            weights: false,
            radius: None,
            noise: Vec::new(),
        }
    }

    /// With the field scope.
    #[must_use]
    pub fn field(mut self, field: &str) -> Context {
        self.field = Some(field.to_string());
        self
    }

    /// With the kind scope.
    #[must_use]
    pub fn kind(mut self, kind: &str) -> Context {
        self.kind = Some(kind.to_string());
        self
    }

    /// With the feature whose attributes are visible.
    #[must_use]
    pub fn feature(mut self, feature: &str) -> Context {
        self.feature = Some(feature.to_string());
        self
    }

    /// With a `weights` field written.
    #[must_use]
    pub fn weights(mut self, weights: bool) -> Context {
        self.weights = weights;
        self
    }

    /// With a default cap.
    #[must_use]
    pub fn radius(mut self, radius: f64) -> Context {
        self.radius = Some(radius);
        self
    }

    /// With the default noise parts, every one text.
    #[must_use]
    pub fn noise<S: AsRef<str>>(mut self, parts: &[S]) -> Context {
        self.noise = parts
            .iter()
            .map(|p| NoisePart::Text(p.as_ref().to_string()))
            .collect();
        self
    }

    /// With default noise parts that may name the feature.
    #[must_use]
    pub fn noise_parts(mut self, parts: Vec<NoisePart>) -> Context {
        self.noise = parts;
        self
    }

    /// Whether a built-in name is visible here (6.2 to 6.8).
    pub fn sees(&self, b: Builtin) -> bool {
        match b {
            Builtin::Pi | Builtin::X | Builtin::Y => true,
            // An import's `blend` reads `d`, the distance inward from the footprint's outline
            // (`composition.md` 6.3), and neither `sd` nor `t`.
            Builtin::D => matches!(
                self.key,
                Key::EffectValue | Key::EffectMask | Key::KindLet | Key::Blend
            ),
            Builtin::Sd | Builtin::T => {
                matches!(self.key, Key::EffectValue | Key::EffectMask | Key::KindLet)
            }
            Builtin::V => {
                !self.weights
                    && matches!(
                        self.key,
                        Key::ThenValue
                            | Key::ThenMask
                            | Key::EffectValue
                            | Key::EffectMask
                            | Key::KindLet
                    )
            }
            Builtin::Z => false,
        }
    }
}
