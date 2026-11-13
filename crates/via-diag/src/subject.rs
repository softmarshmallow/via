//! Subjects (`spec/diagnostics.md` 1.1): the thing at fault, in one canonical form.

use crate::registry::ident;
use std::fmt;

/// One step of a key path: a key, or a 0-based list index.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyPart {
    /// A key.
    Key(String),
    /// A 0-based index into a list.
    Index(u32),
}

/// A key path below an entity: never empty, so a site that names a key always names one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyPath(Vec<KeyPart>);

impl KeyPath {
    /// A path of one key.
    pub fn key(k: impl Into<String>) -> KeyPath {
        KeyPath(vec![KeyPart::Key(k.into())])
    }

    /// A path from its parts; `None` when there are none.
    pub fn from_parts(parts: Vec<KeyPart>) -> Option<KeyPath> {
        (!parts.is_empty()).then_some(KeyPath(parts))
    }

    /// The path with one more key below.
    #[must_use]
    pub fn then(mut self, k: impl Into<String>) -> KeyPath {
        self.0.push(KeyPart::Key(k.into()));
        self
    }

    /// The path with a list index below.
    #[must_use]
    pub fn index(mut self, i: u32) -> KeyPath {
        self.0.push(KeyPart::Index(i));
        self
    }

    /// The parts, first to last.
    pub fn parts(&self) -> &[KeyPart] {
        &self.0
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_parts(&self.0, true, f)
    }
}

fn write_parts(parts: &[KeyPart], first: bool, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, p) in parts.iter().enumerate() {
        match p {
            KeyPart::Key(k) => {
                if !(first && i == 0) {
                    f.write_str(".")?;
                }
                write_name(k, f)?;
            }
            KeyPart::Index(n) => write!(f, "[{n}]")?,
        }
    }
    Ok(())
}

/// A name part: bare when TOML would write it bare (`A-Z a-z 0-9 _ -`), so a bad id reads as
/// written (`feature.Mill`, `identity.md` case 1); quoted otherwise (a taxon path, `assets.md`).
fn write_name(s: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bare = !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if bare {
        f.write_str(s)
    } else {
        write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// The thing at fault. Every other way of naming a thing (a bare id, a place name, a produced
/// name) is mapped to one of these before it reaches a record; that mapping needs the world, so
/// it is the caller's (`step::StepWorld` for a step's subjects).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Subject {
    /// The world as a whole: `world`.
    World,
    /// A command-line fault: `cli`.
    Cli,
    /// An entity address `<collection>.<id>`, with an optional key path below it.
    Entity {
        /// The top table: `feature`, `scatter`, `step`, ...
        collection: String,
        /// The id within it.
        id: String,
        /// The key path below the entity; empty for the entity itself.
        keys: Vec<KeyPart>,
    },
    /// A selection, as its selector is written: `kind:quarter`.
    Selector(String),
}

/// Why a subject string is not in the canonical form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectError(pub String);

impl fmt::Display for SubjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Subject {
    /// The entity `<collection>.<id>`.
    pub fn entity(collection: impl Into<String>, id: impl Into<String>) -> Subject {
        Subject::Entity {
            collection: collection.into(),
            id: id.into(),
            keys: Vec::new(),
        }
    }

    /// This subject with a key path appended below it. On `world`, `cli` or a selector, which
    /// have no keys, the subject is returned unchanged.
    #[must_use]
    pub fn with_keys(self, path: &KeyPath) -> Subject {
        match self {
            Subject::Entity {
                collection,
                id,
                mut keys,
            } => {
                keys.extend(path.parts().iter().cloned());
                Subject::Entity {
                    collection,
                    id,
                    keys,
                }
            }
            other => other,
        }
    }

    /// The entity address alone, key path dropped.
    #[must_use]
    pub fn address(&self) -> Subject {
        match self {
            Subject::Entity { collection, id, .. } => Subject::entity(collection, id),
            other => other.clone(),
        }
    }

    /// The key path below the entity, if any.
    pub fn key_path(&self) -> Option<KeyPath> {
        match self {
            Subject::Entity { keys, .. } => KeyPath::from_parts(keys.clone()),
            _ => None,
        }
    }

    /// A producer named with no key path: a `run`, a `scatter` or a step (`vocabulary.md`,
    /// "producer"). Only these records fold together (section 5).
    pub fn is_producer(&self) -> bool {
        matches!(self, Subject::Entity { collection, keys, .. }
            if keys.is_empty() && matches!(collection.as_str(), "run" | "scatter" | "step"))
    }

    /// Reads the canonical form: `world`, `cli`, a selector (`<class>:<rest>`), or
    /// `<collection>.<id>` with keys (`.key`) and indices (`[n]`) below. A part that is not an id
    /// may be quoted (`taxon."a/b"`).
    pub fn parse(s: &str) -> Result<Subject, SubjectError> {
        let bad = |why: &str| Err(SubjectError(format!("`{s}` is not a subject: {why}")));
        match s {
            "world" => return Ok(Subject::World),
            "cli" => return Ok(Subject::Cli),
            "" => return bad("it is empty"),
            _ => {}
        }
        if let Some((class, rest)) = s.split_once(':') {
            if ident(class) && !rest.is_empty() && !s.starts_with('"') {
                return Ok(Subject::Selector(s.to_string()));
            }
        }
        let parts = match split_parts(s) {
            Some(p) => p,
            None => return bad("a quote or bracket is not closed"),
        };
        let mut it = parts.into_iter();
        let collection = match it.next() {
            Some(KeyPart::Key(c)) if ident(&c) => c,
            _ => return bad("it does not start with a collection"),
        };
        let id = match it.next() {
            Some(KeyPart::Key(id)) if !id.is_empty() => id,
            _ => return bad("it has no id after the collection"),
        };
        Ok(Subject::Entity {
            collection,
            id,
            keys: it.collect(),
        })
    }
}

/// Splits `a.b."c d"[2].e` into parts; `None` on an unclosed quote or bracket or an empty part.
fn split_parts(s: &str) -> Option<Vec<KeyPart>> {
    let mut parts = Vec::new();
    let mut chars = s.chars().peekable();
    let mut expect_name = true;
    while chars.peek().is_some() {
        if expect_name {
            let mut name = String::new();
            if chars.peek() == Some(&'"') {
                chars.next();
                loop {
                    match chars.next()? {
                        '\\' => name.push(chars.next()?),
                        '"' => break,
                        c => name.push(c),
                    }
                }
            } else {
                while let Some(&c) = chars.peek() {
                    if c == '.' || c == '[' {
                        break;
                    }
                    name.push(c);
                    chars.next();
                }
                if name.is_empty() {
                    return None;
                }
            }
            parts.push(KeyPart::Key(name));
            expect_name = false;
        } else {
            match chars.next()? {
                '.' => expect_name = true,
                '[' => {
                    let mut n = String::new();
                    loop {
                        match chars.next()? {
                            ']' => break,
                            c if c.is_ascii_digit() => n.push(c),
                            _ => return None,
                        }
                    }
                    parts.push(KeyPart::Index(n.parse().ok()?));
                }
                _ => return None,
            }
        }
    }
    if expect_name {
        return None;
    }
    Some(parts)
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Subject::World => f.write_str("world"),
            Subject::Cli => f.write_str("cli"),
            Subject::Selector(s) => f.write_str(s),
            Subject::Entity {
                collection,
                id,
                keys,
            } => {
                f.write_str(collection)?;
                f.write_str(".")?;
                write_name(id, f)?;
                write_parts(keys, false, f)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for s in [
            "world",
            "cli",
            "feature.mill",
            "scatter.shrubs.density",
            "kind.path_a.effect.bed.mask",
            "feature.stream_b.path[2]",
            "kind:quarter",
            "taxon.\"flora_x/oak\"",
            "step.checks",
        ] {
            assert_eq!(Subject::parse(s).unwrap().to_string(), s, "{s}");
        }
    }

    #[test]
    fn rejects_non_subjects() {
        for s in ["", "feature", "feature.", ".x", "a.b[", "a.b[x]", "a.\"b"] {
            assert!(Subject::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn producers_have_no_key_path() {
        assert!(Subject::parse("step.infill").unwrap().is_producer());
        assert!(Subject::parse("scatter.shrubs").unwrap().is_producer());
        assert!(!Subject::parse("scatter.shrubs.density")
            .unwrap()
            .is_producer());
        assert!(!Subject::parse("instance.a").unwrap().is_producer());
    }

    #[test]
    fn keys_append_and_drop() {
        let s = Subject::entity("feature", "stream_b").with_keys(&KeyPath::key("path").index(2));
        assert_eq!(s.to_string(), "feature.stream_b.path[2]");
        assert_eq!(s.address().to_string(), "feature.stream_b");
        assert_eq!(s.key_path().unwrap().to_string(), "path[2]");
    }
}
