//! Files read and the spans into them (`spec/documents.md` 2.1).
//!
//! Every file read is one [`Source`], held by index in [`Sources`]. A [`Span`] names a byte range
//! of one source; line and column are computed from it only when a record or an origin is written,
//! so a span is three integers wherever it is stored.

use via_diag::Location;

/// The index of a source in [`Sources`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SrcId(pub(crate) u32);

/// A byte range `[start, end)` of one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// The source.
    pub src: SrcId,
    /// First byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

impl Span {
    pub(crate) fn new(src: SrcId, start: usize, end: usize) -> Span {
        Span {
            src,
            start: clamp_u32(start),
            end: clamp_u32(end),
        }
    }

    /// The smallest span holding both; `other` must be in the same source.
    #[must_use]
    pub(crate) fn to(self, other: Span) -> Span {
        Span {
            src: self.src,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

fn clamp_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What a file is to the world, which decides the chain entries its values carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    /// The world file: layer `world`, which has no chain entry.
    World,
    /// A layer file.
    Layer {
        /// The layer's id.
        id: String,
        /// The layer's `owner`.
        owner: Option<String>,
    },
    /// A library's kinds file.
    Library {
        /// The library's id.
        id: String,
    },
    /// A change file.
    ChangeFile {
        /// Its name, the file stem.
        name: String,
    },
    /// Anything else read through this crate (a pack, a step's returned document).
    Other,
}

/// One file read.
#[derive(Clone, Debug)]
pub struct Source {
    /// The file as a record writes it (`documents.md` 2.1): never an absolute path.
    pub name: String,
    /// What the file is to the world.
    pub role: Role,
    /// The text, byte-order mark included, so byte offsets are the file's.
    pub(crate) text: String,
    /// Byte offset of the start of each line.
    line_starts: Vec<u32>,
    /// Bytes of a skipped UTF-8 byte-order mark at the start (0 or 3).
    bom: u32,
}

impl Source {
    pub(crate) fn new(name: String, role: Role, text: String) -> Source {
        let bom = if text.starts_with('\u{feff}') { 3 } else { 0 };
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(clamp_u32(i + 1));
            }
        }
        Source {
            name,
            role,
            text,
            line_starts,
            bom,
        }
    }

    /// The text, byte-order mark included.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 1-based line and column of a byte offset. A column counts Unicode scalar values; a skipped
    /// byte-order mark is not a column; CRLF is one line end.
    pub fn line_column(&self, offset: u32) -> (u32, u32) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let mut start = self.line_starts.get(line).copied().unwrap_or(0) as usize;
        if line == 0 {
            start = start.max(self.bom as usize);
        }
        let end = (offset as usize).min(self.text.len()).max(start);
        let column = self.text.get(start..end).map_or(0, |s| s.chars().count());
        (clamp_u32(line + 1), clamp_u32(column + 1))
    }
}

/// Every file read for one world, in reading order.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    files: Vec<Source>,
}

impl Sources {
    pub(crate) fn add(&mut self, source: Source) -> SrcId {
        self.files.push(source);
        SrcId(clamp_u32(self.files.len() - 1))
    }

    /// The source of an id.
    pub fn get(&self, id: SrcId) -> Option<&Source> {
        self.files.get(id.0 as usize)
    }

    /// Every source, in reading order.
    pub fn iter(&self) -> impl Iterator<Item = &Source> {
        self.files.iter()
    }

    /// The file name of a span's source, or the empty string for an unknown id.
    pub fn name(&self, id: SrcId) -> &str {
        self.get(id).map_or("", |s| s.name.as_str())
    }

    /// The line of a span's start.
    pub fn line(&self, span: Span) -> u32 {
        self.get(span.src)
            .map_or(1, |s| s.line_column(span.start).0)
    }

    /// A span as a record's location.
    pub fn location(&self, span: Span) -> Location {
        match self.get(span.src) {
            Some(s) => {
                let (line, column) = s.line_column(span.start);
                Location::Span {
                    file: s.name.clone(),
                    line,
                    column,
                    bytes: [span.start, span.end],
                }
            }
            None => Location::world_start(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_count_scalars_and_skip_the_mark() {
        let s = Source::new(
            "a.toml".into(),
            Role::World,
            "\u{feff}ab = 1\r\né = 2\n".into(),
        );
        assert_eq!(s.line_column(3), (1, 1));
        assert_eq!(s.line_column(8), (1, 6));
        // Line 2 starts after the CRLF; `=` is the third scalar.
        let eq = s.text.find("é =").unwrap() as u32 + 3;
        assert_eq!(s.line_column(eq), (2, 3));
    }
}
