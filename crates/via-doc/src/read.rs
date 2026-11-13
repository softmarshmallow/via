//! The reader: TOML 1.0.0 text to the tree of `spec/documents.md` 2, with spans.
//!
//! Ours, not a crate (the chapter's open question, settled in M0 step 5 by its three cases): the
//! maintained readers available (`toml_edit` 0.25, `toml_parser` 1.1, `toml` 1.2) all read TOML
//! 1.1, so `syntax_v11` would pass where it must fail, and none hands back a map from each
//! character of a string to the file. The reader counts depth before it descends, so no input
//! overflows the stack (section 1), and it never builds a binary float.

use crate::report::{At, Fault};
use crate::source::{Span, SrcId};
use crate::tree::{Entry, Form, Item, Num, Str, Table, Value};
use std::collections::BTreeMap;
use via_diag::{codes, Subject, Value as DValue};

/// Nesting limit of section 1.
pub const MAX_DEPTH: usize = 64;

/// Bytes to text: a UTF-16 or UTF-32 byte-order mark, or bytes that are not UTF-8, are
/// `doc.syntax`, located by file and byte offset (section 1).
pub(crate) fn decode(bytes: Vec<u8>, file: &str) -> Result<String, Box<Fault>> {
    let wide = [
        (&[0x00u8, 0x00, 0xFE, 0xFF][..], "UTF-32"),
        (&[0xFF, 0xFE, 0x00, 0x00][..], "UTF-32"),
        (&[0xFE, 0xFF][..], "UTF-16"),
        (&[0xFF, 0xFE][..], "UTF-16"),
    ];
    for (mark, name) in wide {
        if bytes.starts_with(mark) {
            return Err(Box::new(Fault::new(
                codes::doc::SYNTAX,
                Subject::World,
                At::File(file.to_string(), vec![("offset".into(), DValue::Int(0))]),
                format!("`{file}` starts with a {name} byte-order mark; documents are UTF-8"),
                "save the file as UTF-8",
            )));
        }
    }
    String::from_utf8(bytes).map_err(|e| {
        let offset = e.utf8_error().valid_up_to();
        Box::new(Fault::new(
            codes::doc::SYNTAX,
            Subject::World,
            At::File(
                file.to_string(),
                vec![(
                    "offset".into(),
                    DValue::Int(i64::try_from(offset).unwrap_or(i64::MAX)),
                )],
            ),
            format!("`{file}` is not UTF-8: the byte at offset {offset} starts no character"),
            "save the file as UTF-8",
        ))
    })
}

/// Reads one file's text to its root table. On a fault the file is read no further and every
/// fault found is returned (a `[[x]]` header does not stop reading, so the rewrite hint can name
/// the items).
pub(crate) fn read(src: SrcId, text: &str) -> Result<Table, Vec<Fault>> {
    let start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let mut p = Parser {
        b: text.as_bytes(),
        s: text,
        pos: start,
        src,
        faults: Vec::new(),
    };
    let mut root = Table::new(Span::new(src, start, start), Form::Root);
    let r = p.document(&mut root);
    if r.is_err() || !p.faults.is_empty() {
        return Err(p.faults);
    }
    Ok(root)
}

/// One syntax fault of a document read outside a world (a pack, `assets.md` 1): section 1's
/// rules, messages and hints, with the byte range in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxFault {
    /// What is wrong, with no file, line or column.
    pub message: String,
    /// What to change.
    pub hint: String,
    /// The UTF-8 byte range `[start, end)` in the file.
    pub bytes: [u32; 2],
    /// The record's `data`, when the fault carries any.
    pub data: Vec<(String, DValue)>,
}

/// Reads `bytes` as a document under section 1's rules and gives its syntax faults; none when it
/// reads. `file` names the file in a message about its encoding.
pub fn syntax_faults(bytes: Vec<u8>, file: &str) -> Vec<SyntaxFault> {
    let of = |f: &Fault| {
        let bytes = match &f.at {
            At::Span(s) => [s.start, s.end],
            At::File(_, facts) => {
                let at = facts
                    .iter()
                    .find(|(k, _)| k == "offset")
                    .and_then(|(_, v)| match v {
                        DValue::Int(i) => u32::try_from(*i).ok(),
                        _ => None,
                    })
                    .unwrap_or(0);
                [at, at]
            }
            At::Argument(_) | At::World => [0, 0],
        };
        SyntaxFault {
            message: f.message.clone(),
            hint: f.hint.clone(),
            bytes,
            data: f.data.clone(),
        }
    };
    let text = match decode(bytes, file) {
        Ok(t) => t,
        Err(f) => return vec![of(&f)],
    };
    match read(SrcId(0), &text) {
        Ok(_) => Vec::new(),
        Err(faults) => faults.iter().map(of).collect(),
    }
}

struct Stop;
type R<T> = Result<T, Stop>;

struct Parser<'a> {
    b: &'a [u8],
    s: &'a str,
    pos: usize,
    src: SrcId,
    faults: Vec<Fault>,
}

/// One `[[x]]` path met in the file, for the rewrite hint of section 1.
struct ArrayInfo {
    first: Span,
    every_item_has_id: bool,
    first_id: Option<String>,
}

enum Target {
    Path(Vec<String>),
    Scratch(String, Table),
}

impl<'a> Parser<'a> {
    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.src, start, end)
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn peek_at(&self, i: usize) -> Option<u8> {
        self.b.get(self.pos + i).copied()
    }

    fn char_len(&self, at: usize) -> usize {
        self.s
            .get(at..)
            .and_then(|r| r.chars().next())
            .map_or(1, char::len_utf8)
    }

    fn fail<T>(&mut self, at: usize, message: impl Into<String>, hint: &str) -> R<T> {
        let end = if at < self.b.len() {
            at + self.char_len(at)
        } else {
            at
        };
        self.fail_span(self.span(at, end), message, hint)
    }

    fn fail_span<T>(&mut self, span: Span, message: impl Into<String>, hint: &str) -> R<T> {
        self.faults.push(Fault::new(
            codes::doc::SYNTAX,
            Subject::World,
            At::Span(span),
            message,
            hint,
        ));
        Err(Stop)
    }

    fn found(&self) -> String {
        match self.s.get(self.pos..).and_then(|r| r.chars().next()) {
            None => "the end of the file".to_string(),
            Some('\n') => "a line end".to_string(),
            Some('\r') => "a carriage return".to_string(),
            Some(c) if c.is_control() => format!("the control character U+{:04X}", c as u32),
            Some(c) => format!("`{c}`"),
        }
    }

    /// Section 1: an item deeper than 64 is `doc.too_deep`, and the file is read no further.
    fn depth(&mut self, depth: usize, span: Span) -> R<()> {
        if depth > MAX_DEPTH {
            self.faults.push(Fault::new(
                codes::doc::TOO_DEEP,
                Subject::World,
                At::Span(span),
                format!("tables and lists nest {depth} deep here; the limit is {MAX_DEPTH}"),
                "flatten the structure: write entities keyed by id rather than nested lists",
            ));
            return Err(Stop);
        }
        Ok(())
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn comment(&mut self) -> R<()> {
        if self.peek() != Some(b'#') {
            return Ok(());
        }
        self.pos += 1;
        while let Some(c) = self.peek() {
            match c {
                b'\n' => break,
                b'\r' if self.peek_at(1) == Some(b'\n') => break,
                b'\t' => self.pos += 1,
                0x00..=0x1F | 0x7F => {
                    let f = self.found();
                    return self.fail(self.pos, format!("{f} in a comment"), "remove it");
                }
                _ => self.pos += 1,
            }
        }
        Ok(())
    }

    /// A line end, or the end of the file.
    fn newline(&mut self) -> R<bool> {
        match self.peek() {
            Some(b'\n') => {
                self.pos += 1;
                Ok(true)
            }
            Some(b'\r') if self.peek_at(1) == Some(b'\n') => {
                self.pos += 2;
                Ok(true)
            }
            Some(b'\r') => self.fail(
                self.pos,
                "a carriage return not followed by a line feed",
                "end lines with LF or CRLF",
            ),
            _ => Ok(false),
        }
    }

    fn skip_blank(&mut self) -> R<()> {
        loop {
            self.skip_ws();
            self.comment()?;
            if !self.newline()? {
                return Ok(());
            }
        }
    }

    fn end_of_line(&mut self) -> R<()> {
        self.skip_ws();
        self.comment()?;
        if self.pos >= self.b.len() || self.newline()? {
            return Ok(());
        }
        let f = self.found();
        self.fail(
            self.pos,
            format!("expected the end of the line, found {f}"),
            "put each key on its own line",
        )
    }

    fn document(&mut self, root: &mut Table) -> R<()> {
        let mut target = Target::Path(Vec::new());
        let mut arrays: BTreeMap<String, ArrayInfo> = BTreeMap::new();
        let result = self.lines(root, &mut target, &mut arrays);
        finish_scratch(&mut target, &mut arrays);
        for (path, info) in arrays {
            let mut f = Fault::new(
                codes::doc::SYNTAX,
                Subject::World,
                At::Span(info.first),
                format!("`[[{path}]]` is an array of tables; documents are keyed by id"),
                String::new(),
            );
            match (info.every_item_has_id, info.first_id) {
                (true, Some(id)) => {
                    let rewrite = format!("[{path}.{id}]");
                    f.hint = format!("write `{rewrite}`, one table per item, keyed by its `id`");
                    f = f.with("rewrite", rewrite);
                }
                _ => {
                    f.hint = format!("write `[{path}.<id>]`, one table per item, keyed by id");
                }
            }
            self.faults.push(f);
        }
        result
    }

    fn lines(
        &mut self,
        root: &mut Table,
        target: &mut Target,
        arrays: &mut BTreeMap<String, ArrayInfo>,
    ) -> R<()> {
        loop {
            self.skip_blank()?;
            if self.pos >= self.b.len() {
                return Ok(());
            }
            if self.peek() == Some(b'[') {
                finish_scratch(target, arrays);
                let start = self.pos;
                let array = self.peek_at(1) == Some(b'[');
                self.pos += if array { 2 } else { 1 };
                self.skip_ws();
                let parts = self.key()?;
                self.skip_ws();
                let close: &[u8] = if array { b"]]" } else { b"]" };
                if !self.b[self.pos..].starts_with(close) {
                    let f = self.found();
                    return self.fail(
                        self.pos,
                        format!("expected `{}` to close the header, found {f}", ascii(close)),
                        "close the header on the same line",
                    );
                }
                self.pos += close.len();
                let header = self.span(start, self.pos);
                for (i, (_, ks)) in parts.iter().enumerate() {
                    self.depth(i + 1, *ks)?;
                }
                let names: Vec<String> = parts.iter().map(|(k, _)| k.clone()).collect();
                if array {
                    let path = dotted(&names);
                    arrays.entry(path.clone()).or_insert(ArrayInfo {
                        first: header,
                        every_item_has_id: true,
                        first_id: None,
                    });
                    *target = Target::Scratch(path, Table::new(header, Form::Header));
                } else {
                    self.define_header(root, &parts, header)?;
                    *target = Target::Path(names);
                }
            } else {
                let (table, base_depth) = match target {
                    Target::Path(path) => {
                        let depth = path.len();
                        match table_at(root, path) {
                            Some(t) => (t, depth),
                            None => {
                                return self.fail(
                                    self.pos,
                                    "a key outside any table",
                                    "put the key under a `[table]` header",
                                );
                            }
                        }
                    }
                    Target::Scratch(_, t) => (t, 1),
                };
                self.keyval(table, base_depth)?;
            }
            self.end_of_line()?;
        }
    }

    fn define_header(&mut self, root: &mut Table, parts: &[(String, Span)], header: Span) -> R<()> {
        let mut t = root;
        for (i, (k, ks)) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            if t.get(k).is_none() {
                let form = if last { Form::Header } else { Form::Implicit };
                t.insert(
                    k.clone(),
                    Entry {
                        key: header,
                        item: Item {
                            span: header,
                            value: Value::Table(Table::new(header, form)),
                        },
                        chain: Vec::new(),
                    },
                );
            } else {
                let form = match t.value(k) {
                    Some(Value::Table(sub)) => Some(sub.form),
                    _ => None,
                };
                match (form, last) {
                    (None, _) => {
                        return self.fail_span(
                            *ks,
                            format!("`{}` already holds a value, so it cannot be a table", k),
                            "rename the table or the key",
                        )
                    }
                    (Some(Form::Inline), _) => {
                        return self.fail_span(
                            *ks,
                            format!("`{k}` was written as an inline table, which is closed"),
                            "write every key of it inside its braces",
                        )
                    }
                    (Some(Form::Implicit), true) => {
                        if let Some(sub) = t.table_mut(k) {
                            sub.form = Form::Header;
                        }
                    }
                    (Some(_), true) => {
                        let path = dotted(
                            &parts[..=i]
                                .iter()
                                .map(|(k, _)| k.clone())
                                .collect::<Vec<_>>(),
                        );
                        return self.fail_span(
                            header,
                            format!("the table `{path}` is defined twice"),
                            "write each table once, with all its keys",
                        );
                    }
                    (Some(_), false) => {}
                }
            }
            t = match t.table_mut(k) {
                Some(sub) => sub,
                None => {
                    return self.fail_span(
                        *ks,
                        "the key does not name a table",
                        "rename the key, or write the table it names with `[name]`",
                    )
                }
            };
        }
        Ok(())
    }

    /// `key = value` into `table`, whose depth is `depth`.
    fn keyval(&mut self, table: &mut Table, depth: usize) -> R<()> {
        let parts = self.key()?;
        let key_span = match (parts.first(), parts.last()) {
            (Some((_, a)), Some((_, b))) => a.to(*b),
            _ => {
                return self.fail(
                    self.pos,
                    "expected a key",
                    "write `key = value`, the key bare, or in a basic or literal string",
                )
            }
        };
        for (i, (_, ks)) in parts.iter().enumerate().take(parts.len() - 1) {
            self.depth(depth + i + 1, *ks)?;
        }
        self.skip_ws();
        if self.peek() != Some(b'=') {
            let f = self.found();
            return self.fail(
                self.pos,
                format!("expected `=` after the key, found {f}"),
                "write `key = value`",
            );
        }
        self.pos += 1;
        self.skip_ws();
        let item = self.value(depth + parts.len() - 1)?;
        self.insert_dotted(table, &parts, key_span, item)
    }

    fn insert_dotted(
        &mut self,
        table: &mut Table,
        parts: &[(String, Span)],
        key_span: Span,
        item: Item,
    ) -> R<()> {
        let mut t = table;
        let (last, init) = match parts.split_last() {
            Some(x) => x,
            None => {
                return self.fail_span(
                    key_span,
                    "expected a key",
                    "write `key = value`, the key bare, or in a basic or literal string",
                )
            }
        };
        for (k, ks) in init {
            match t.value(k) {
                None => t.insert(
                    k.clone(),
                    Entry {
                        key: key_span,
                        item: Item {
                            span: key_span,
                            value: Value::Table(Table::new(key_span, Form::Dotted)),
                        },
                        chain: Vec::new(),
                    },
                ),
                Some(Value::Table(sub)) if sub.form == Form::Dotted => {}
                Some(Value::Table(_)) => {
                    return self.fail_span(
                        *ks,
                        format!(
                            "`{k}` is a table defined elsewhere; a dotted key cannot add to it"
                        ),
                        "write the key under that table's header",
                    )
                }
                Some(_) => {
                    return self.fail_span(
                        *ks,
                        format!("`{k}` already holds a value, so it cannot be a table"),
                        "rename the key",
                    )
                }
            }
            t = match t.table_mut(k) {
                Some(sub) => sub,
                None => {
                    return self.fail_span(
                        *ks,
                        "the key does not name a table",
                        "rename the key, or write the table it names with `[name]`",
                    )
                }
            };
        }
        let (k, _) = last;
        if t.get(k).is_some() {
            return self.fail_span(
                key_span,
                format!("the key `{k}` is written twice in one table"),
                "write each key once",
            );
        }
        t.insert(
            k.clone(),
            Entry {
                key: key_span,
                item,
                chain: Vec::new(),
            },
        );
        Ok(())
    }

    /// A key: one or more simple keys joined by dots.
    fn key(&mut self) -> R<Vec<(String, Span)>> {
        let mut parts = Vec::new();
        loop {
            let start = self.pos;
            let name = match self.peek() {
                Some(b'"') => {
                    if self.b[self.pos..].starts_with(b"\"\"\"") {
                        return self.fail(
                            start,
                            "a key cannot be a multi-line string",
                            "write the key on one line in a basic or literal string",
                        );
                    }
                    self.basic(false)?.text
                }
                Some(b'\'') => {
                    if self.b[self.pos..].starts_with(b"'''") {
                        return self.fail(
                            start,
                            "a key cannot be a multi-line string",
                            "write the key on one line in a basic or literal string",
                        );
                    }
                    self.literal(false)?.text
                }
                Some(c) if bare(c) => {
                    while self.peek().is_some_and(bare) {
                        self.pos += 1;
                    }
                    self.s[start..self.pos].to_string()
                }
                _ => {
                    let f = self.found();
                    return self.fail(
                        self.pos,
                        format!("expected a key, found {f}"),
                        "a bare key is letters, digits, `_` and `-`; quote any other",
                    );
                }
            };
            parts.push((name, self.span(start, self.pos)));
            let save = self.pos;
            self.skip_ws();
            if self.peek() == Some(b'.') {
                self.pos += 1;
                self.skip_ws();
            } else {
                self.pos = save;
                return Ok(parts);
            }
        }
    }

    /// A value whose own depth, if it is a table or list, is `parent + 1`.
    fn value(&mut self, parent: usize) -> R<Item> {
        let start = self.pos;
        match self.peek() {
            Some(b'"') => {
                let multi = self.b[self.pos..].starts_with(b"\"\"\"");
                let s = self.basic(multi)?;
                Ok(Item {
                    span: self.span(start, self.pos),
                    value: Value::Str(s),
                })
            }
            Some(b'\'') => {
                let multi = self.b[self.pos..].starts_with(b"'''");
                let s = self.literal(multi)?;
                Ok(Item {
                    span: self.span(start, self.pos),
                    value: Value::Str(s),
                })
            }
            Some(b'[') => {
                self.depth(parent + 1, self.span(start, start + 1))?;
                self.array(parent + 1)
            }
            Some(b'{') => {
                self.depth(parent + 1, self.span(start, start + 1))?;
                self.inline(parent + 1)
            }
            Some(_) => self.scalar(),
            None => self.fail(
                self.pos,
                "expected a value, found the end of the file",
                "write the value, and close any open array with `]` and inline table with `}`",
            ),
        }
    }

    fn skip_in_array(&mut self) -> R<()> {
        loop {
            self.skip_ws();
            self.comment()?;
            if !self.newline()? {
                return Ok(());
            }
        }
    }

    fn array(&mut self, depth: usize) -> R<Item> {
        let start = self.pos;
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_in_array()?;
            if self.peek() == Some(b']') {
                self.pos += 1;
                break;
            }
            items.push(self.value(depth)?);
            self.skip_in_array()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => {
                    let f = self.found();
                    return self.fail(
                        self.pos,
                        format!("expected `,` or `]` in a list, found {f}"),
                        "separate list items with commas",
                    );
                }
            }
        }
        Ok(Item {
            span: self.span(start, self.pos),
            value: Value::List(items),
        })
    }

    fn inline_ws(&mut self) -> R<()> {
        self.skip_ws();
        match self.peek() {
            Some(b'\n' | b'\r') => self.fail(
                self.pos,
                "a line end inside an inline table is TOML 1.1, not TOML 1.0",
                "write the inline table on one line, or use a `[header]` table",
            ),
            Some(b'#') => self.fail(
                self.pos,
                "a comment inside an inline table is not TOML 1.0",
                "write the inline table on one line, or use a `[header]` table",
            ),
            _ => Ok(()),
        }
    }

    fn inline(&mut self, depth: usize) -> R<Item> {
        let start = self.pos;
        self.pos += 1;
        let mut t = Table::new(self.span(start, start + 1), Form::Inline);
        self.inline_ws()?;
        if self.peek() == Some(b'}') {
            self.pos += 1;
        } else {
            loop {
                self.keyval(&mut t, depth)?;
                self.inline_ws()?;
                match self.peek() {
                    Some(b',') => {
                        let comma = self.pos;
                        self.pos += 1;
                        self.inline_ws()?;
                        if self.peek() == Some(b'}') {
                            return self.fail(
                                comma,
                                "a trailing comma in an inline table is TOML 1.1, not TOML 1.0",
                                "remove the comma",
                            );
                        }
                    }
                    Some(b'}') => {
                        self.pos += 1;
                        break;
                    }
                    _ => {
                        let f = self.found();
                        return self.fail(
                            self.pos,
                            format!("expected `,` or `}}` in an inline table, found {f}"),
                            "separate keys with commas and close with `}`",
                        );
                    }
                }
            }
        }
        close(&mut t);
        Ok(Item {
            span: self.span(start, self.pos),
            value: Value::Table(t),
        })
    }

    fn string_char_bad(&mut self, at: usize) -> R<()> {
        let f = self.found();
        self.fail(
            at,
            format!("{f} in a string"),
            "write it as an escape, such as `\\t` or `\\u0007`",
        )
    }

    /// A basic string, `"..."` or `"""..."""`.
    fn basic(&mut self, multi: bool) -> R<Str> {
        let open = self.pos;
        self.pos += if multi { 3 } else { 1 };
        if multi {
            self.newline()?;
        }
        let mut text = String::new();
        let mut map = Vec::new();
        loop {
            let at = self.pos;
            let Some(c) = self.peek() else {
                return self.fail(open, "a string is not closed", "close it with its quote");
            };
            match c {
                b'"' => {
                    if !multi {
                        self.pos += 1;
                        map.push(u32::try_from(at).unwrap_or(u32::MAX));
                        return Ok(Str { text, map });
                    }
                    let mut n = 0;
                    while self.peek_at(n) == Some(b'"') {
                        n += 1;
                    }
                    if n >= 3 {
                        if n > 5 {
                            return self.fail(
                                at,
                                "too many quotes closing a string",
                                "escape them",
                            );
                        }
                        for i in 0..n - 3 {
                            push(&mut text, &mut map, '"', at + i);
                        }
                        self.pos += n;
                        map.push(u32::try_from(at + n - 3).unwrap_or(u32::MAX));
                        return Ok(Str { text, map });
                    }
                    for i in 0..n {
                        push(&mut text, &mut map, '"', at + i);
                    }
                    self.pos += n;
                }
                b'\\' => {
                    if multi {
                        // A line-ending backslash trims white space and line ends after it.
                        let mut j = self.pos + 1;
                        while matches!(self.b.get(j), Some(b' ' | b'\t')) {
                            j += 1;
                        }
                        if matches!(self.b.get(j), Some(b'\n'))
                            || (self.b.get(j) == Some(&b'\r') && self.b.get(j + 1) == Some(&b'\n'))
                        {
                            self.pos = j;
                            loop {
                                self.skip_ws();
                                if !self.newline()? {
                                    break;
                                }
                            }
                            continue;
                        }
                    }
                    let ch = self.escape()?;
                    push(&mut text, &mut map, ch, at);
                }
                b'\n' if multi => {
                    self.pos += 1;
                    push(&mut text, &mut map, '\n', at);
                }
                b'\r' if multi && self.peek_at(1) == Some(b'\n') => {
                    // CRLF reads as LF, so a file with CRLF line ends reads the same (section 1).
                    self.pos += 2;
                    push(&mut text, &mut map, '\n', at);
                }
                b'\t' => {
                    self.pos += 1;
                    push(&mut text, &mut map, '\t', at);
                }
                0x00..=0x1F | 0x7F => return self.string_char_bad(at).map(|()| Str { text, map }),
                _ => {
                    let len = self.char_len(at);
                    let ch = self.s[at..at + len].chars().next().unwrap_or('\u{fffd}');
                    self.pos += len;
                    push(&mut text, &mut map, ch, at);
                }
            }
        }
    }

    fn escape(&mut self) -> R<char> {
        let at = self.pos;
        self.pos += 1;
        let Some(e) = self.peek() else {
            return self.fail(at, "a string is not closed", "close it with its quote");
        };
        self.pos += 1;
        let simple = match e {
            b'b' => Some('\u{8}'),
            b't' => Some('\t'),
            b'n' => Some('\n'),
            b'f' => Some('\u{c}'),
            b'r' => Some('\r'),
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            _ => None,
        };
        if let Some(c) = simple {
            return Ok(c);
        }
        let digits = match e {
            b'u' => 4,
            b'U' => 8,
            b'e' | b'x' => {
                let span = self.span(at, at + 2);
                return self.fail_span(
                    span,
                    format!("the escape `\\{}` is TOML 1.1, not TOML 1.0", e as char),
                    "write `\\u` with four hexadecimal digits",
                );
            }
            _ => {
                let span = self.span(at, at + 1 + self.char_len(at + 1));
                return self.fail_span(
                    span,
                    "an escape TOML 1.0 does not have",
                    "write `\\\\` for a backslash",
                );
            }
        };
        let hex = self.s.get(self.pos..self.pos + digits).unwrap_or("");
        let value = (hex.len() == digits && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten()
            .and_then(char::from_u32);
        match value {
            Some(c) => {
                self.pos += digits;
                Ok(c)
            }
            None => {
                let span = self.span(at, (self.pos + digits).min(self.b.len()));
                self.fail_span(
                    span,
                    "a unicode escape that names no Unicode scalar value",
                    "write a scalar value, `\\u0000` to `\\uD7FF` or `\\uE000` to `\\U0010FFFF`",
                )
            }
        }
    }

    /// A literal string, `'...'` or `'''...'''`.
    fn literal(&mut self, multi: bool) -> R<Str> {
        let open = self.pos;
        self.pos += if multi { 3 } else { 1 };
        if multi {
            self.newline()?;
        }
        let mut text = String::new();
        let mut map = Vec::new();
        loop {
            let at = self.pos;
            let Some(c) = self.peek() else {
                return self.fail(open, "a string is not closed", "close it with its quote");
            };
            match c {
                b'\'' => {
                    if !multi {
                        self.pos += 1;
                        map.push(u32::try_from(at).unwrap_or(u32::MAX));
                        return Ok(Str { text, map });
                    }
                    let mut n = 0;
                    while self.peek_at(n) == Some(b'\'') {
                        n += 1;
                    }
                    if n >= 3 {
                        if n > 5 {
                            return self.fail(at, "too many quotes closing a string", "close the literal string with three quotes, at most two more before them");
                        }
                        for i in 0..n - 3 {
                            push(&mut text, &mut map, '\'', at + i);
                        }
                        self.pos += n;
                        map.push(u32::try_from(at + n - 3).unwrap_or(u32::MAX));
                        return Ok(Str { text, map });
                    }
                    for i in 0..n {
                        push(&mut text, &mut map, '\'', at + i);
                    }
                    self.pos += n;
                }
                b'\n' if multi => {
                    self.pos += 1;
                    push(&mut text, &mut map, '\n', at);
                }
                b'\r' if multi && self.peek_at(1) == Some(b'\n') => {
                    self.pos += 2;
                    push(&mut text, &mut map, '\n', at);
                }
                b'\t' => {
                    self.pos += 1;
                    push(&mut text, &mut map, '\t', at);
                }
                0x00..=0x1F | 0x7F => return self.string_char_bad(at).map(|()| Str { text, map }),
                _ => {
                    let len = self.char_len(at);
                    let ch = self.s[at..at + len].chars().next().unwrap_or('\u{fffd}');
                    self.pos += len;
                    push(&mut text, &mut map, ch, at);
                }
            }
        }
    }

    /// A bool, number, date or time.
    fn scalar(&mut self) -> R<Item> {
        let start = self.pos;
        if let Some(end) = datetime(self.b, start) {
            self.pos = end;
            return Ok(Item {
                span: self.span(start, end),
                value: Value::DateTime,
            });
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'+' | b'-' | b'.'))
        {
            self.pos += 1;
        }
        let tok = &self.s[start..self.pos];
        let span = self.span(start, self.pos);
        if tok.is_empty() {
            let f = self.found();
            return self.fail(start, format!("expected a value, found {f}"), "write a value: a string in quotes, a number, `true` or `false`, a date, `[...]` or `{...}`");
        }
        let value = match tok {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "inf" | "+inf" | "-inf" | "nan" | "+nan" | "-nan" => {
                self.faults.push(Fault::new(
                    codes::num::NOT_FINITE,
                    Subject::World,
                    At::Span(span),
                    format!("`{tok}` is not a finite number"),
                    "write a finite number",
                ));
                Value::Num(Num {
                    raw: tok.to_string(),
                    norm: tok.trim_start_matches('+').to_string(),
                    integer: false,
                })
            }
            _ => match number(tok) {
                Ok(n) => Value::Num(n),
                Err(why) => {
                    return self.fail_span(
                        span,
                        format!("`{tok}` is not a TOML 1.0 value: {why}"),
                        "write a number, `true`, `false` or a quoted string",
                    )
                }
            },
        };
        Ok(Item { span, value })
    }
}

fn push(text: &mut String, map: &mut Vec<u32>, c: char, at: usize) {
    text.push(c);
    let o = u32::try_from(at).unwrap_or(u32::MAX);
    for _ in 0..c.len_utf8() {
        map.push(o);
    }
}

fn bare(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

fn ascii(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Keys joined by dots, quoted where not bare (for messages and the rewrite hint).
pub(crate) fn dotted(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| {
            if !p.is_empty() && p.bytes().all(bare) {
                p.clone()
            } else {
                format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\""))
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn table_at<'t>(root: &'t mut Table, path: &[String]) -> Option<&'t mut Table> {
    let mut t = root;
    for k in path {
        t = t.table_mut(k)?;
    }
    Some(t)
}

/// Closes an inline table: every table a dotted key made inside it is closed too.
fn close(t: &mut Table) {
    t.form = Form::Inline;
    for e in t.entries.values_mut() {
        if let Value::Table(sub) = &mut e.item.value {
            if sub.form == Form::Dotted {
                close(sub);
            }
        }
    }
}

fn finish_scratch(target: &mut Target, arrays: &mut BTreeMap<String, ArrayInfo>) {
    if let Target::Scratch(path, t) = std::mem::replace(target, Target::Path(Vec::new())) {
        if let Some(info) = arrays.get_mut(&path) {
            match t.str("id") {
                Some(id) => {
                    if info.first_id.is_none() {
                        info.first_id = Some(id.to_string());
                    }
                }
                None => info.every_item_has_id = false,
            }
        }
    }
}

/// The end of a date or time starting at `i`, if one does.
fn datetime(b: &[u8], i: usize) -> Option<usize> {
    let d = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
    let digits = |k: usize, n: usize| (k..k + n).all(d);
    let time = |k: usize| -> Option<usize> {
        // TOML 1.0: seconds are required.
        if !(digits(k, 2) && b.get(k + 2) == Some(&b':') && digits(k + 3, 2)) {
            return None;
        }
        if !(b.get(k + 5) == Some(&b':') && digits(k + 6, 2)) {
            return None;
        }
        let mut e = k + 8;
        if b.get(e) == Some(&b'.') && d(e + 1) {
            e += 1;
            while d(e) {
                e += 1;
            }
        }
        Some(e)
    };
    if digits(i, 4) && b.get(i + 4) == Some(&b'-') && digits(i + 5, 2) {
        if !(b.get(i + 7) == Some(&b'-') && digits(i + 8, 2)) {
            return None;
        }
        let mut e = i + 10;
        let sep = b.get(e).copied();
        let after_sep = matches!(sep, Some(b'T' | b't'))
            || (sep == Some(b' ') && digits(e + 1, 2) && b.get(e + 3) == Some(&b':'));
        if after_sep {
            e = time(e + 1)?;
            match b.get(e) {
                Some(b'Z' | b'z') => e += 1,
                Some(b'+' | b'-')
                    if digits(e + 1, 2) && b.get(e + 3) == Some(&b':') && digits(e + 4, 2) =>
                {
                    e += 6;
                }
                _ => {}
            }
        }
        return Some(e);
    }
    if digits(i, 2) && b.get(i + 2) == Some(&b':') {
        return time(i);
    }
    None
}

/// `digit (_? digit)*` in the given radix from `i`; the end, or `None`.
fn digit_run(b: &[u8], i: usize, ok: fn(u8) -> bool) -> Option<usize> {
    if !b.get(i).is_some_and(|&c| ok(c)) {
        return None;
    }
    let mut e = i + 1;
    loop {
        match b.get(e) {
            Some(&c) if ok(c) => e += 1,
            Some(b'_') if b.get(e + 1).is_some_and(|&c| ok(c)) => e += 2,
            _ => return Some(e),
        }
    }
}

/// A TOML 1.0 integer or float, normalised (`numbers.md` 3.1 step 1).
fn number(tok: &str) -> Result<Num, &'static str> {
    let b = tok.as_bytes();
    let radix = match b {
        [b'0', b'x', ..] => Some((16, (|c: u8| c.is_ascii_hexdigit()) as fn(u8) -> bool)),
        [b'0', b'o', ..] => Some((8, (|c: u8| (b'0'..=b'7').contains(&c)) as fn(u8) -> bool)),
        [b'0', b'b', ..] => Some((2, (|c: u8| c == b'0' || c == b'1') as fn(u8) -> bool)),
        _ => None,
    };
    if let Some((r, ok)) = radix {
        if digit_run(b, 2, ok) != Some(b.len()) {
            return Err("a malformed integer");
        }
        let digits: String = tok[2..].chars().filter(|&c| c != '_').collect();
        let v =
            i64::from_str_radix(&digits, r).map_err(|_| "an integer out of the 64-bit range")?;
        return Ok(Num {
            raw: tok.to_string(),
            norm: v.to_string(),
            integer: true,
        });
    }
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i = 1;
    }
    let int_end = digit_run(b, i, |c| c.is_ascii_digit()).ok_or("a malformed number")?;
    if b[i] == b'0' && int_end > i + 1 {
        return Err("a leading zero");
    }
    let mut e = int_end;
    let mut float = false;
    if b.get(e) == Some(&b'.') {
        e = digit_run(b, e + 1, |c| c.is_ascii_digit()).ok_or("a malformed fraction")?;
        float = true;
    }
    if matches!(b.get(e), Some(b'e' | b'E')) {
        let mut k = e + 1;
        if matches!(b.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        e = digit_run(b, k, |c| c.is_ascii_digit()).ok_or("a malformed exponent")?;
        float = true;
    }
    if e != b.len() {
        return Err("a malformed number");
    }
    let mut norm: String = tok.chars().filter(|&c| c != '_').collect();
    if norm.starts_with('+') {
        norm.remove(0);
    }
    if !float && norm.parse::<i64>().is_err() {
        return Err("an integer out of the 64-bit range");
    }
    Ok(Num {
        raw: tok.to_string(),
        norm,
        integer: !float,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(text: &str) -> Table {
        match read(SrcId(0), text) {
            Ok(t) => t,
            Err(f) => panic!(
                "{text:?}: {:?}",
                f.iter().map(|f| &f.message).collect::<Vec<_>>()
            ),
        }
    }

    fn bad(text: &str) -> Vec<Fault> {
        match read(SrcId(0), text) {
            Ok(_) => panic!("{text:?} read"),
            Err(f) => f,
        }
    }

    #[test]
    fn numbers_normalise() {
        let t = ok("a = 0x10\nb = +1_000\nc = 1.5e+3\nd = -0.25\ne = 0o17\nf = 0b101\n");
        let n = |k: &str| match t.value(k) {
            Some(Value::Num(n)) => (n.norm.clone(), n.integer),
            _ => panic!(),
        };
        assert_eq!(n("a"), ("16".into(), true));
        assert_eq!(n("b"), ("1000".into(), true));
        assert_eq!(n("c"), ("1.5e+3".into(), false));
        assert_eq!(n("d"), ("-0.25".into(), false));
        assert_eq!(n("e"), ("15".into(), true));
        assert_eq!(n("f"), ("5".into(), true));
        for b in [
            "a = 01\n",
            "a = 1__0\n",
            "a = 1.\n",
            "a = .5\n",
            "a = 0x\n",
            "a = +0x1\n",
        ] {
            bad(b);
        }
    }

    #[test]
    fn toml_one_one_forms_are_syntax() {
        // Case syntax_v11, and the other TOML 1.1 forms of section 1.
        for t in [
            "a = { x = 1,\n y = 2 }\n",
            "a = { x = 1, }\n",
            "a = \"\\e\"\n",
            "a = \"\\x41\"\n",
            "a = 07:32\n",
        ] {
            let f = bad(t);
            assert_eq!(f[0].code_str(), "doc.syntax", "{t:?}");
        }
    }

    #[test]
    fn array_tables_give_the_rewrite() {
        // Case array_table.
        let f = bad("[[feature]]\nid = \"mill\"\nkind = \"x\"\n\n[[feature]]\nid = \"cart\"\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code_str(), "doc.syntax");
        assert!(f[0]
            .data
            .iter()
            .any(|(k, v)| k == "rewrite" && *v == DValue::text("[feature.mill]")));
        let f = bad("[[feature]]\nid = \"mill\"\n[[feature]]\nkind = \"x\"\n");
        assert!(f[0].data.is_empty());
        assert!(f[0].hint.contains("[feature.<id>]"));
    }

    #[test]
    fn depth_is_counted_before_descent() {
        // Case too_deep.
        let deep = |n: usize| format!("x = {}1{}\n", "[".repeat(n), "]".repeat(n));
        ok(&deep(64));
        let f = bad(&deep(65));
        assert_eq!(f[0].code_str(), "doc.too_deep");
        assert_eq!(f[0].at, At::Span(Span::new(SrcId(0), 4 + 64, 4 + 65)));
        let f = bad(&deep(100_000));
        assert_eq!(f[0].code_str(), "doc.too_deep");
        let header = |n: usize| format!("[{}]\n", vec!["a"; n].join("."));
        ok(&header(64));
        assert_eq!(bad(&header(65))[0].code_str(), "doc.too_deep");
    }

    #[test]
    fn strings_map_back_to_the_file() {
        let text = "f = \"= a\\t+ \\u00e9 + bb\"\n";
        let t = ok(text);
        let Some(Value::Str(s)) = t.value("f") else {
            panic!()
        };
        assert_eq!(s.text, "= a\t+ é + bb");
        let at = s.text.find("bb").unwrap();
        let (fs, fe) = s.file_range(at, at + 2).unwrap();
        assert_eq!(&text[fs as usize..fe as usize], "bb");
        let e = s.text.find('é').unwrap();
        let (fs, fe) = s.file_range(e, e + 2).unwrap();
        assert_eq!(&text[fs as usize..fe as usize], "\\u00e9");
    }

    #[test]
    fn multiline_strings() {
        let t = ok("a = \"\"\"\r\nx\r\ny \\\r\n   z\"\"\"\"\nb = '''\nq'''\n");
        assert_eq!(t.str("a"), Some("x\ny z\""));
        assert_eq!(t.str("b"), Some("q"));
    }

    #[test]
    fn redefinition_rules() {
        ok("[a.b]\n[a]\nx = 1\n");
        ok("a.b = 1\n[a.c]\n");
        ok("x = { a.b = 1, a.c = 2 }\n");
        bad("[a]\n[a]\n");
        bad("a = 1\na = 2\n");
        bad("a.b = 1\n[a]\n");
        bad("x = { a = 1 }\n[x.b]\n");
        bad("[a.b]\nc = 1\n[a]\nb.d = 1\n");
    }

    #[test]
    fn forms_and_spans() {
        let text = "[feature.mill]\nkind = \"k\"\non = { ref = \"m\", t = 0.5 }\n";
        let t = ok(text);
        let mill = t.table("feature").unwrap().table("mill").unwrap();
        assert_eq!(mill.form, Form::Header);
        let on = mill.get("on").unwrap();
        assert_eq!(&text[on.key.start as usize..on.key.end as usize], "on");
        assert_eq!(
            &text[on.item.span.start as usize..on.item.span.end as usize],
            "{ ref = \"m\", t = 0.5 }"
        );
        let d = ok("feature.mill.on.t = 1\n");
        let m = d.table("feature").unwrap().table("mill").unwrap();
        assert_eq!(m.form, Form::Dotted);
    }

    #[test]
    fn dates_and_non_finite() {
        let t = ok("seed = 1979-05-27\nb = 1979-05-27T07:32:00Z\nc = 07:32:00\n");
        assert_eq!(t.value("seed"), Some(&Value::DateTime));
        assert_eq!(t.value("b"), Some(&Value::DateTime));
        assert_eq!(bad("a = nan\n")[0].code_str(), "num.not_finite");
        assert_eq!(bad("a = -inf\n")[0].code_str(), "num.not_finite");
    }

    #[test]
    fn encodings() {
        let f = decode(vec![b'a', 0xFF], "layers/d.toml").unwrap_err();
        assert_eq!(f.code_str(), "doc.syntax");
        assert_eq!(
            f.at,
            At::File(
                "layers/d.toml".into(),
                vec![("offset".into(), DValue::Int(1))]
            )
        );
        assert!(decode(vec![0xFF, 0xFE, b'a', 0], "a.toml").is_err());
        let t = ok("\u{feff}a = 1\r\nb = 2\r\n");
        assert_eq!(t.get("a").unwrap().key.start, 3);
    }
}
