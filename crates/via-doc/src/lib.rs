//! Documents.
//!
//! Responsibility: reading every file of a world as TOML with the source span of each value,
//! checking closed keys against the vocabulary, and merging the world file, its layers, its
//! includes and the requested changes into the one merged world the graph is made from. Spans
//! are kept because every diagnostic points back to a file and line.
//!
//! Governed by `spec/documents.md`, with `spec/vocabulary.md` for the words.
//!
//! # Use
//!
//! [`load`] reads a world (its path, and the changes requested) into a [`Loaded`]: the merged
//! [`World`] when no reading step ended with an error, and every diagnostic found, already
//! located. The world implements [`via_diag::Resolve`], so a diagnostic a later node raises on
//! an entity, a key or a sub-expression of a formula is located through it. [`World::to_json`]
//! is `via merge`'s canonical JSON; [`World::origins_json`] is `--origins`.
//!
//! The reader is ours ([`read`]'s module documentation says why): TOML 1.0.0 exactly, with key,
//! value and header spans, every number as its literal and normalised text, and a map from each
//! byte of a string back to the file.

mod change;
mod check;
mod ids;
mod join;
mod json;
mod load;
mod path;
pub mod read;
mod report;
pub mod schema;
mod source;
mod tree;
mod world;

pub use change::free_name;
pub use ids::{is_id, repair as repair_id};
pub use join::{document_file, Returned};
pub use json::{attribute_json, FORMAT};
pub use load::{load, ChangeRequest, LoadOptions, Loaded};
pub use path::open_file;
pub use source::{Role, Source, Sources, Span, SrcId};
pub use tree::{Entry, Form, Item, Link, Num, Str, Table, Value};
pub use world::{FormulaAt, World};

/// The document-language version (`documents.md` 2.3): what the text means.
pub const LANGUAGE: &str = "via-doc-1";

#[cfg(test)]
mod tests;
