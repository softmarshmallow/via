//! The data model behind the syntax (`spec/documents.md` 2).
//!
//! Every file becomes a plain tree of tables, lists, strings, numbers and bools, each item with
//! its span. Everything after the reader works on this tree, never on TOML.

use crate::source::Span;
use std::collections::BTreeMap;
use via_diag::Through;

/// How a table came to be, which TOML's rules on redefinition need and the header span records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// The file's root table.
    Root,
    /// A `[x]` header defined it.
    Header,
    /// A header below it created it without defining it (`[a.b]` creates `a`).
    Implicit,
    /// A dotted key created it.
    Dotted,
    /// An inline table, or a table a dotted key created inside one: closed once written.
    Inline,
}

/// A table: a map from key to entry, ordered by key bytewise (`vocabulary.md` 2.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub(crate) entries: BTreeMap<String, Entry>,
    /// The span that created the table first in its file (2.1, header span).
    pub(crate) header: Span,
    pub(crate) form: Form,
}

impl Table {
    pub(crate) fn new(header: Span, form: Form) -> Table {
        Table {
            entries: BTreeMap::new(),
            header,
            form,
        }
    }

    /// The entry of a key.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.entries.get_mut(key)
    }

    /// Every entry, ordered by key bytewise.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Entry)> {
        self.entries.iter()
    }

    /// The keys, ordered bytewise.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the table holds nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The header span: what created the table first in its file.
    pub fn header(&self) -> Span {
        self.header
    }

    /// How the table was written.
    pub fn form(&self) -> Form {
        self.form
    }

    /// The value of a key, if any.
    pub fn value(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|e| &e.item.value)
    }

    /// The string value of a key, if it is one.
    pub fn str(&self, key: &str) -> Option<&str> {
        match self.value(key) {
            Some(Value::Str(s)) => Some(&s.text),
            _ => None,
        }
    }

    /// The table value of a key, if it is one.
    pub fn table(&self, key: &str) -> Option<&Table> {
        match self.value(key) {
            Some(Value::Table(t)) => Some(t),
            _ => None,
        }
    }

    pub(crate) fn table_mut(&mut self, key: &str) -> Option<&mut Table> {
        match self.entries.get_mut(key).map(|e| &mut e.item.value) {
            Some(Value::Table(t)) => Some(t),
            _ => None,
        }
    }

    pub(crate) fn insert(&mut self, key: String, entry: Entry) {
        self.entries.insert(key, entry);
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<Entry> {
        self.entries.remove(key)
    }
}

/// One key of a table, with the span of the key as written and how its value got there.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The key as written, dotted parts included (2.1, key span). For a table a header created,
    /// the header.
    pub key: Span,
    /// The value.
    pub item: Item,
    /// How the value came through, reading order, the last writer last (`diagnostics.md` 1.2).
    pub chain: Vec<Link>,
}

/// One entry of a chain: a span stands for `diagnostics.md`'s file and line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// How the value came through.
    pub through: Through,
    /// The layer id, library id, change name or feature address.
    pub name: String,
    /// The key that did it.
    pub span: Span,
    /// For a layer, its `owner`.
    pub owner: Option<String>,
}

/// A value with its span: the value as written, quotes and brackets included (2.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The value span.
    pub span: Span,
    /// The value.
    pub value: Value,
}

/// The five kinds of tree item, and the one TOML form that has none.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A table.
    Table(Table),
    /// A list, in written order.
    List(Vec<Item>),
    /// A string.
    Str(Str),
    /// A number.
    Num(Num),
    /// `true` or `false`.
    Bool(bool),
    /// A TOML date or time: no tree item, kept only so it can be reported as `doc.type`.
    DateTime,
}

impl Value {
    /// The word for the value's form, used in `doc.type` messages.
    pub fn form_word(&self) -> &'static str {
        match self {
            Value::Table(_) => "a table",
            Value::List(_) => "a list",
            Value::Str(_) => "a string",
            Value::Num(n) if n.integer => "an integer",
            Value::Num(_) => "a number",
            Value::Bool(_) => "a bool",
            Value::DateTime => "a date or time",
        }
    }
}

/// A string after TOML's escapes, with a map back to the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Str {
    /// The UTF-8 text.
    pub text: String,
    /// For each byte of `text`, the file byte offset of the character or escape it came from;
    /// one more entry, the offset of the closing quote. So a byte range of the value maps to a
    /// byte range of the file (2.1, through escapes and across lines).
    pub map: Vec<u32>,
}

impl Str {
    /// The file byte range of a byte range of the value.
    pub fn file_range(&self, start: usize, end: usize) -> Option<(u32, u32)> {
        let s = *self.map.get(start)?;
        let e = if end > start {
            // The end of the last byte's source character: the next mapped offset that differs.
            let last = *self.map.get(end - 1)?;
            let mut i = end;
            loop {
                match self.map.get(i) {
                    Some(&o) if o == last => i += 1,
                    Some(&o) => break o,
                    None => break last,
                }
            }
        } else {
            s
        };
        Some((s, e))
    }
}

/// A number: its literal, its normalised text, and whether it was a TOML integer (2). Never a
/// binary float: `via-num` turns the normalised text into a value as `numbers.md` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Num {
    /// The literal as written.
    pub raw: String,
    /// `numbers.md` 3.1 step 1: `_` dropped, a leading `+` dropped, a hexadecimal, octal or
    /// binary integer as decimal digits.
    pub norm: String,
    /// Written as a TOML integer.
    pub integer: bool,
}
