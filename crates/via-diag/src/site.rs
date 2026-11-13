//! Sites and locations (`spec/diagnostics.md` 1 and 1.2).
//!
//! A [`Site`] is what a node stores: where the fault is, in terms that survive text moving. A
//! [`Location`] is what a record holds: the file, line, column and bytes in this merge. The
//! [`Resolve`] trait, implemented by the document reader, turns one into the other at emission.

use crate::subject::{KeyPath, Subject};
use crate::value::Value;
use std::collections::BTreeMap;
use std::ops::Range;

/// Where the fault is, relative to the diagnostic's subject. None of these is empty.
#[derive(Clone, Debug, PartialEq)]
pub enum Site {
    /// The span of that key under the subject's entity.
    Key(KeyPath),
    /// A sub-expression of the formula string at that key: a byte range of the string's value,
    /// which the resolver maps to the file (`documents.md` 2.1).
    Expr(KeyPath, Range<u32>),
    /// The span that creates the subject's entity (1.2).
    Header,
    /// The span of the key that wrote the selector.
    Selector(KeyPath),
    /// A file that is not text, already written in the form of `documents.md` 2.1
    /// (`input:dem.tif`), and the `data` that locates inside it (a sample, a byte offset, a
    /// pointer into a model). The locator is merged into the record's `data`.
    File {
        /// The file as written in a record.
        file: String,
        /// Facts locating the fault inside the file.
        locator: BTreeMap<String, Value>,
    },
    /// Text that is not a document of the merge (a pack, `assets.md` 2), located when it is
    /// read: the file as a record writes it, a 1-based line and column, and `[start, end)` bytes.
    Text {
        /// The file, in the form of `documents.md` 2.1.
        file: String,
        /// 1-based line.
        line: u32,
        /// 1-based column in Unicode scalar values.
        column: u32,
        /// `[start, end)` in bytes.
        bytes: [u32; 2],
    },
    /// A command-line fault: the 1-based index of the argument at fault, after the program name.
    Argument(u32),
    /// The world file at line 1, column 1, bytes `[0, 0]`.
    World,
}

impl Site {
    /// A short text naming the site, for `diag.internal` when it does not resolve.
    pub(crate) fn describe(&self) -> String {
        match self {
            Site::Key(p) => format!("key `{p}`"),
            Site::Expr(p, r) => format!("bytes {}..{} of the formula at `{p}`", r.start, r.end),
            Site::Header => "the header".to_string(),
            Site::Selector(p) => format!("the selector at `{p}`"),
            Site::File { file, .. } => format!("file `{file}`"),
            Site::Text { file, line, .. } => format!("`{file}` line {line}"),
            Site::Argument(n) => format!("argument {n}"),
            Site::World => "the world file".to_string(),
        }
    }
}

/// One of the three location forms of 1.2. Every record has exactly one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// A command-line fault, before the world file is known: the argument index.
    Argument(u32),
    /// A file that is not text; the inside position is in `data`.
    File {
        /// The file, in the form of `documents.md` 2.1.
        file: String,
    },
    /// Text: a file, a 1-based line and column (in Unicode scalar values), and the byte range
    /// `[start, end)` of the text an author would edit.
    Span {
        /// The file, in the form of `documents.md` 2.1 (never an absolute path).
        file: String,
        /// 1-based line.
        line: u32,
        /// 1-based column in Unicode scalar values.
        column: u32,
        /// `[start, end)` in bytes.
        bytes: [u32; 2],
    },
}

impl Location {
    /// The world file at 1:1, bytes `[0, 0]`.
    pub fn world_start(world_file: impl Into<String>) -> Location {
        Location::Span {
            file: world_file.into(),
            line: 1,
            column: 1,
            bytes: [0, 0],
        }
    }

    /// The rank of the form in the sort order of section 5: argument, file, span.
    pub(crate) fn form_rank(&self) -> u8 {
        match self {
            Location::Argument(_) => 0,
            Location::File { .. } => 1,
            Location::Span { .. } => 2,
        }
    }

    pub(crate) fn file(&self) -> Option<&str> {
        match self {
            Location::Argument(_) => None,
            Location::File { file } | Location::Span { file, .. } => Some(file),
        }
    }
}

/// How a value came through to the text at fault (1.2, `chain`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Through {
    /// An `include` of a library; `name` is the library's id.
    Include,
    /// A layer; `name` is the layer's id, and the entry carries the layer's `owner`.
    Layer,
    /// A change; `name` is the change's name.
    Change,
    /// A kind default; `name` is the address of the feature that took it.
    Defaults,
}

impl Through {
    /// The word written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Through::Include => "include",
            Through::Layer => "layer",
            Through::Change => "change",
            Through::Defaults => "defaults",
        }
    }
}

/// One entry of a record's `chain`, in reading order, the last writer last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainEntry {
    /// How the value came through.
    pub through: Through,
    /// The library id, layer id, change name or feature address.
    pub name: String,
    /// The file, in the form of `documents.md` 2.1.
    pub file: String,
    /// The line of the key that did it.
    pub line: u32,
    /// For a layer: the layer's `owner`, who answers for the text.
    pub owner: Option<String>,
}

/// A site resolved against the current merge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The one location.
    pub location: Location,
    /// How the value got there; empty when written once in the world file.
    pub chain: Vec<ChainEntry>,
}

/// Resolves sites against the current merge. Implemented by the document reader, which holds
/// the spans; `via-diag` resolves `Argument` and `File` sites itself and asks for the rest.
pub trait Resolve {
    /// The location and chain of a `Key`, `Expr`, `Header` or `Selector` site of `subject`.
    /// `None` when the site has no span in this merge: a defect of ours, which the record
    /// reports as `diag.internal`.
    fn resolve(&self, subject: &Subject, site: &Site) -> Option<Resolved>;

    /// The world file as a record writes it (`world.toml`), for `World` sites and for a
    /// diagnostic whose site did not resolve.
    fn world_file(&self) -> String;
}

/// A resolver for diagnostics raised before any world is read (`via explain` with no world, a
/// usage error): only `Argument` and `File` sites resolve; anything else is reported against an
/// empty world file name, which never happens for the `cli.*` codes it serves.
pub struct NoWorld;

impl Resolve for NoWorld {
    fn resolve(&self, _: &Subject, _: &Site) -> Option<Resolved> {
        None
    }

    fn world_file(&self) -> String {
        String::new()
    }
}
