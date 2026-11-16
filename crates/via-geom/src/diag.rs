//! Kernel diagnostics (`spec/geometry.md`, Diagnostics).
//!
//! A kernel diagnostic carries a code, the source id of the input it is about and a position.
//! The caller maps the source id to subject, file and line (section 3); [`GeomDiag::to_diag`]
//! does it for a caller that has the subject and site, and `via kernel` uses the `cli` form.

use crate::shape::P;
use std::collections::BTreeMap;
use via_diag::{codes, Diag, Position, Site, Subject, Text, Value};

/// The kernel's codes. Each maps to one generated constant of `via-diag`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Code {
    Range,
    RepeatedPoint,
    Degenerate,
    SelfCrossing,
    NoLength,
    TRange,
    SpanOrder,
    ManyRings,
    SmoothNotPath,
    DuplicateSource,
    BadParameter,
    Overlap,
    DeadEnd,
    NoArea,
    OffsetEmpty,
    OffsetSplit,
    FollowBroken,
    CellNone,
    CellMerged,
    CellSmall,
    MaskTiny,
    MaskDropped,
    MaskEmpty,
    Internal,
    ClearEmpty,
    PinRemoved,
    PinOverlap,
    PinsOverlap,
    NearNone,
    NumOutOfRange,
    DeriveDry,
    DeriveOpen,
}

impl Code {
    /// The registry code.
    pub fn code(self) -> via_diag::Code {
        use codes::geom as g;
        use codes::identity as id;
        match self {
            Code::Range => g::RANGE,
            Code::RepeatedPoint => g::REPEATED_POINT,
            Code::Degenerate => g::DEGENERATE,
            Code::SelfCrossing => g::SELF_CROSSING,
            Code::NoLength => g::NO_LENGTH,
            Code::TRange => g::T_RANGE,
            Code::SpanOrder => g::SPAN_ORDER,
            Code::ManyRings => g::MANY_RINGS,
            Code::SmoothNotPath => g::SMOOTH_NOT_PATH,
            Code::DuplicateSource => g::DUPLICATE_SOURCE,
            Code::BadParameter => g::BAD_PARAMETER,
            Code::Overlap => g::OVERLAP,
            Code::DeadEnd => g::DEAD_END,
            Code::NoArea => g::NO_AREA,
            Code::OffsetEmpty => g::OFFSET_EMPTY,
            Code::OffsetSplit => g::OFFSET_SPLIT,
            Code::FollowBroken => g::FOLLOW_BROKEN,
            Code::CellNone => g::CELL_NONE,
            Code::CellMerged => g::CELL_MERGED,
            Code::CellSmall => g::CELL_SMALL,
            Code::MaskTiny => g::MASK_TINY,
            Code::MaskDropped => g::MASK_DROPPED,
            Code::MaskEmpty => g::MASK_EMPTY,
            Code::Internal => g::INTERNAL,
            Code::ClearEmpty => id::CLEAR_EMPTY,
            Code::PinRemoved => id::PIN_REMOVED,
            Code::PinOverlap => id::PIN_OVERLAP,
            Code::PinsOverlap => id::PINS_OVERLAP,
            Code::NearNone => id::NEAR_NONE,
            Code::NumOutOfRange => codes::num::OUT_OF_RANGE,
            Code::DeriveDry => codes::derive::DRY,
            Code::DeriveOpen => codes::derive::OPEN,
        }
    }

    /// The code as written (`geom.range`).
    pub fn name(self) -> &'static str {
        self.code().as_str()
    }

    /// True for an error.
    pub fn is_error(self) -> bool {
        matches!(self.code().severity(), via_diag::Severity::Error)
    }

    fn hint(self) -> &'static str {
        match self {
            Code::Range => "Move the shape nearer the origin or reduce the distance.",
            Code::RepeatedPoint => "Delete the repeat.",
            Code::Degenerate => "Add points or remove the feature.",
            Code::SelfCrossing => "Reorder the points or split the region.",
            Code::NoLength => "Use `at`, or place on a path or region.",
            Code::TRange => "Write a fraction from 0 to 1.",
            Code::SpanOrder => "Swap the two, or reverse the path.",
            Code::ManyRings => "Place on one piece's outline, or use `at`.",
            Code::SmoothNotPath => "Remove `smooth`; it is taken only with `path`.",
            Code::DuplicateSource => "Give each input its own source.",
            Code::BadParameter => "Give the parameter a value in its range.",
            Code::NoArea => "Close the linework, or add a `clip`.",
            Code::OffsetEmpty => "Use a smaller distance.",
            Code::FollowBroken => "Use at most the widest offset that holds, or a shorter span.",
            Code::CellNone => "Lower `spacing` (or `min_length`) in the `[cells]` table, or lengthen the frontage.",
            Code::CellSmall => "Lower the minimum, or lengthen the frontage.",
            Code::MaskEmpty => "Check the condition against the range.",
            Code::Internal => "Report it with the reproduction folder `data.repro` names.",
            Code::ClearEmpty => "Delete the clear if it is no longer wanted.",
            Code::PinRemoved => "Narrow `by` to keep them.",
            Code::PinOverlap => "Add that producer to `by`, or move the pin.",
            Code::PinsOverlap => "Move one of the pins.",
            Code::NearNone => "Move `near` onto a thing the maker made.",
            Code::NumOutOfRange => "Use coordinates within plus or minus 2147483647 mm.",
            Code::DeriveDry => "Move `holds` into a piece, or change the condition.",
            Code::DeriveOpen => {
                "Change the condition so the piece closes inside; `derive.spill` gives a level that does."
            }
            Code::Overlap
            | Code::DeadEnd
            | Code::OffsetSplit
            | Code::CellMerged
            | Code::MaskTiny
            | Code::MaskDropped => "None needed.",
        }
    }
}

/// One kernel diagnostic, before its caller gives it a subject and a site.
#[derive(Clone, Debug, PartialEq)]
pub struct GeomDiag {
    /// The code.
    pub code: Code,
    /// The source id of the input it is about, if any.
    pub source: Option<String>,
    /// Where, in millimetres.
    pub position: Option<P>,
    /// The message.
    pub message: String,
    /// Facts, by name (`data`).
    pub data: BTreeMap<String, Value>,
}

impl GeomDiag {
    /// A diagnostic with a code and a message.
    pub fn new(code: Code, message: impl Into<String>) -> GeomDiag {
        GeomDiag {
            code,
            source: None,
            position: None,
            message: message.into(),
            data: BTreeMap::new(),
        }
    }

    /// The source id.
    #[must_use]
    pub fn source(mut self, s: &str) -> GeomDiag {
        self.source = Some(s.to_string());
        self
    }

    /// The position.
    #[must_use]
    pub fn at(mut self, p: P) -> GeomDiag {
        self.position = Some(p);
        self
    }

    /// A data fact.
    #[must_use]
    pub fn with(mut self, k: &str, v: impl Into<Value>) -> GeomDiag {
        self.data.insert(k.to_string(), v.into());
        self
    }

    /// The `via-diag` diagnostic for a subject and site the caller chose. The source id, if any,
    /// goes into `data.source` (geometry.md 7.1).
    pub fn to_diag(&self, subject: Subject, site: Site) -> Diag {
        let message = Text::new(self.message.clone())
            .unwrap_or_else(|_| Text::from_checked_static("a kernel diagnostic"));
        let hint = Text::from_checked_static(self.code.hint());
        let mut d = Diag::new(self.code.code(), subject, site, message, hint);
        if let Some(p) = self.position {
            d = d.at(Position::plan(p.x, p.y));
        }
        if let Some(s) = &self.source {
            d = d.with("source", Value::text(s.clone()));
        }
        for (k, v) in &self.data {
            d = d.with(k.clone(), v.clone());
        }
        d
    }
}

/// True when any diagnostic is an error.
pub fn has_error(diags: &[GeomDiag]) -> bool {
    diags.iter().any(|d| d.code.is_error())
}
