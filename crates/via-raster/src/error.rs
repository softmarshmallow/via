//! The errors of this crate, as values.
//!
//! `via-raster` knows the post, the sample or the window at fault, but not the field, the key or the
//! file that asked: those belong to the caller. So each error carries its registry code and the
//! facts its message needs, and [`RasterError::diag`] turns it into a located [`Diag`] once the
//! caller gives the subject and the site. Nothing here panics on bad input.

use std::fmt;
use via_diag::{codes, Code, Diag, Position, Site, Subject, Text};
use via_num::NumError;

/// An error of `via-raster`. [`RasterError::code`] gives its registry code.
#[derive(Clone, Debug, PartialEq)]
pub enum RasterError {
    /// A value on its way into a stored field is not finite, or overflows the 32-bit float's range
    /// on narrowing (`num.not_finite_result`, `fields.md` 1.1). `at` is the post in millimetres.
    NotFinite { at: (i64, i64), bits: u64 },
    /// A mask outside 0 to 1, or not a number (`formula.range`, `fields.md` 5).
    MaskRange { at: (i64, i64), mask: f64 },
    /// A `route` cost at or below 0, or not finite, at a post (`formula.range`, `derived.md` 3):
    /// a course over free ground has no least.
    CostRange { at: (i64, i64), cost: f64 },
    /// A cap or a width outside its range (`formula.range`).
    ParameterRange { what: &'static str, value: f64 },
    /// A read beyond the extent, or with no neighbour on either side (`formula.outside_field`).
    OutsideField { at: (i64, i64) },
    /// A distance over no shape (`formula.empty_selection`).
    EmptySelection,
    /// A selection mixing shapes a signed distance cannot fold (`formula.shape`).
    Shape { reason: &'static str },
    /// A signed distance over several paths (`formula.selector_count`).
    SelectorCount { paths: usize },
    /// An array file outside what a field `file` takes (`field.file_format`).
    FileFormat { reason: String },
    /// An array file whose header asks for more samples than the post-count limit
    /// (`field.file_too_large`), found before any sample is read.
    FileTooLarge { rows: u64, columns: u64, limit: u64 },
    /// A non-finite sample in an array file, by row (from the south) and column
    /// (`field.file_nonfinite`).
    FileNonFinite { row: u64, column: u64 },
    /// A weight below 0 or not finite reaching the writer (`build.internal`, `build.md` 6.2).
    WeightInvalid { at: (i64, i64), material: String },
    /// A checked integer operation overflowed (`num.overflow`).
    Overflow { op: &'static str },
    /// A computed point outside the lattice (`geom.range`).
    LatticeRange,
    /// A caller asked for posts its inputs do not hold: a window not covered, rasters of different
    /// windows. A defect of the caller, never of the world (`diag.internal`).
    Internal { reason: String },
}

impl RasterError {
    /// The code of `spec/diagnostics.md`'s registry for this error.
    pub fn code(&self) -> Code {
        match self {
            RasterError::NotFinite { .. } => codes::num::NOT_FINITE_RESULT,
            RasterError::MaskRange { .. }
            | RasterError::CostRange { .. }
            | RasterError::ParameterRange { .. } => codes::formula::RANGE,
            RasterError::OutsideField { .. } => codes::formula::OUTSIDE_FIELD,
            RasterError::EmptySelection => codes::formula::EMPTY_SELECTION,
            RasterError::Shape { .. } => codes::formula::SHAPE,
            RasterError::SelectorCount { .. } => codes::formula::SELECTOR_COUNT,
            RasterError::FileFormat { .. } => codes::field::FILE_FORMAT,
            RasterError::FileTooLarge { .. } => codes::field::FILE_TOO_LARGE,
            RasterError::FileNonFinite { .. } => codes::field::FILE_NONFINITE,
            RasterError::WeightInvalid { .. } => codes::build::INTERNAL,
            RasterError::Overflow { .. } => codes::num::OVERFLOW,
            RasterError::LatticeRange => codes::geom::RANGE,
            RasterError::Internal { .. } => codes::diag::INTERNAL,
        }
    }

    /// The plan position the error names, if it has one.
    pub fn position(&self) -> Option<Position> {
        match self {
            RasterError::NotFinite { at, .. }
            | RasterError::MaskRange { at, .. }
            | RasterError::CostRange { at, .. }
            | RasterError::OutsideField { at }
            | RasterError::WeightInvalid { at, .. } => Some(Position::plan(at.0, at.1)),
            _ => None,
        }
    }

    fn hint(&self) -> Text {
        let s = match self {
            RasterError::NotFinite { .. } => "keep the value within the 32-bit float range",
            RasterError::MaskRange { .. } => "wrap the mask in `clamp(..., 0, 1)`",
            RasterError::CostRange { .. } => {
                "keep the cost above 0 everywhere, for example `= 1 + ...`"
            }
            RasterError::ParameterRange { .. } => "give a finite value above 0",
            RasterError::OutsideField { .. } => "read inside the extent",
            RasterError::EmptySelection => "select at least one feature",
            RasterError::Shape { .. } => "select regions only, one path, or points only",
            RasterError::SelectorCount { .. } => "narrow the selector to one path",
            RasterError::FileFormat { .. } => {
                "save a 2-D array of `<f4` with at least 2 rows and 2 columns"
            }
            RasterError::FileTooLarge { .. } => "crop or downsample the raster",
            RasterError::FileNonFinite { .. } => "repair the file",
            RasterError::WeightInvalid { .. } | RasterError::Internal { .. } => {
                "a defect of the engine; report it with the world"
            }
            RasterError::Overflow { .. } | RasterError::LatticeRange => {
                "move the shape nearer the origin or reduce the distance"
            }
        };
        Text::new(s).unwrap_or_else(|_| via_diag::text!("see the message"))
    }

    /// A located diagnostic: the caller gives the subject (the field, feature or step) and the
    /// site (the key, or [`file_site`] for a sample of a file); this adds the code, message, hint
    /// and position.
    pub fn diag(&self, subject: Subject, site: Site) -> Diag {
        let message = Text::new(self.to_string())
            .unwrap_or_else(|_| via_diag::text!("a raster operation failed"));
        let d = Diag::new(self.code(), subject, site, message, self.hint());
        match self.position() {
            Some(p) => d.at(p),
            None => d,
        }
    }

    /// Maps a `via-num` error at a post: a non-finite value keeps its post, an overflow its
    /// operation.
    pub fn from_num(e: NumError, at: (i64, i64)) -> RasterError {
        match e {
            NumError::NotFiniteResult { bits } => RasterError::NotFinite { at, bits },
            NumError::Overflow { op } => RasterError::Overflow { op },
            NumError::LatticeRange => RasterError::LatticeRange,
            other => RasterError::Internal {
                reason: format!("unexpected {} at a post", other.code()),
            },
        }
    }
}

/// The site of a sample inside a file that is not text (`diagnostics.md` 1.2): the file as a
/// record writes it, located by row (from the south) and column.
pub fn file_site(file: impl Into<String>, row: u64, column: u64) -> Site {
    let mut locator = std::collections::BTreeMap::new();
    locator.insert("row".to_string(), via_diag::Value::Int(row as i64));
    locator.insert("column".to_string(), via_diag::Value::Int(column as i64));
    Site::File {
        file: file.into(),
        locator,
    }
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mm = |v: i64| via_num::write_length(v);
        match self {
            RasterError::NotFinite { at, bits } => write!(
                f,
                "the value {} at ({}, {}) is not a finite 32-bit float",
                f64::from_bits(*bits),
                mm(at.0),
                mm(at.1)
            ),
            RasterError::MaskRange { at, mask } => write!(
                f,
                "mask {mask} at ({}, {}) is outside 0 to 1",
                mm(at.0),
                mm(at.1)
            ),
            RasterError::CostRange { at, cost } => write!(
                f,
                "route cost {cost} at ({}, {}) is not above 0; a course over free ground has no least",
                mm(at.0),
                mm(at.1)
            ),
            RasterError::ParameterRange { what, value } => {
                write!(f, "{what} {value} is not a finite value above 0")
            }
            RasterError::OutsideField { at } => write!(
                f,
                "no field value at ({}, {}): beyond the extent",
                mm(at.0),
                mm(at.1)
            ),
            RasterError::EmptySelection => f.write_str("a distance over no shape"),
            RasterError::Shape { reason } => write!(f, "{reason}"),
            RasterError::SelectorCount { paths } => {
                write!(f, "a signed distance over {paths} paths; it takes one")
            }
            RasterError::FileFormat { reason } => write!(f, "array file: {reason}"),
            RasterError::FileTooLarge {
                rows,
                columns,
                limit,
            } => write!(
                f,
                "array file of {rows} by {columns} samples is more than the limit of {limit}"
            ),
            RasterError::FileNonFinite { row, column } => {
                write!(f, "non-finite sample at row {row}, column {column}")
            }
            RasterError::WeightInvalid { at, material } => write!(
                f,
                "weight of `{material}` at ({}, {}) is below 0 or not finite",
                mm(at.0),
                mm(at.1)
            ),
            RasterError::Overflow { op } => write!(f, "integer overflow in {op}"),
            RasterError::LatticeRange => f.write_str("a computed point is outside the lattice"),
            RasterError::Internal { reason } => write!(f, "internal: {reason}"),
        }
    }
}

impl std::error::Error for RasterError {}
