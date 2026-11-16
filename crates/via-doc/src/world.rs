//! The merged world: what the build graph is made from, with the spans every diagnostic points
//! back to.

use crate::change::{Base, Removal};
use crate::check::Rank;
use crate::report::chain_entries;
use crate::schema;
use crate::source::{Sources, Span};
use crate::tree::{Entry, Str, Table, Value};
use std::collections::BTreeMap;
use via_diag::{KeyPart, Location, Resolve, Resolved, Site, Subject};

/// The merged world (`documents.md` 8): the base with the requested changes applied, checked.
#[derive(Clone, Debug)]
pub struct World {
    pub(crate) sources: Sources,
    pub(crate) world_file: String,
    pub(crate) base: Base,
    pub(crate) ranks: BTreeMap<String, Rank>,
    pub(crate) changes: Vec<String>,
    pub(crate) removed: Vec<Removal>,
}

/// A formula string in the merged world, for `formulas.md`: where it is, and its text with the
/// map back to the file (`documents.md` 7.1).
#[derive(Clone, Debug)]
pub struct FormulaAt<'w> {
    /// The entity and key path that hold it.
    pub subject: Subject,
    /// The string, `=` included, with its span map.
    pub text: &'w Str,
}

impl World {
    /// Every file read, in reading order.
    pub fn sources(&self) -> &Sources {
        &self.sources
    }

    /// The world file as a record writes it.
    pub fn file(&self) -> &str {
        &self.world_file
    }

    /// The merged root: the world's top-level keys and every top table (`layer`, `kind`, `let`,
    /// `feature`, ...). Change tables are not in it.
    pub fn root(&self) -> &Table {
        &self.base.root
    }

    /// The requested changes, sorted bytewise: names, and change files as source paths.
    pub fn changes(&self) -> &[String] {
        &self.changes
    }

    /// The entities requested changes removed: `(top table, id, change)`, for `doc.dangling`
    /// on names a later node resolves (a formula's reads).
    pub fn removals(&self) -> Vec<(&str, &str, &str)> {
        self.removed
            .iter()
            .map(|r| (r.top.as_str(), r.id.as_str(), r.change.as_str()))
            .collect()
    }

    /// The table of an entity, `None` for a `let` name or an absent one.
    pub fn entity(&self, top: &str, id: &str) -> Option<&Table> {
        self.base.root.table(top).and_then(|t| t.table(id))
    }

    /// The entry of an entity (for a `let` name, its value).
    pub fn entity_entry(&self, top: &str, id: &str) -> Option<&Entry> {
        self.base.root.table(top).and_then(|t| t.get(id))
    }

    /// The ids of a top table, bytewise.
    pub fn ids(&self, top: &str) -> Vec<&str> {
        self.base
            .root
            .table(top)
            .map_or(Vec::new(), |t| t.keys().collect())
    }

    /// The layer an entity is declared in; for a kind, its library.
    pub fn home(&self, top: &str, id: &str) -> Option<&str> {
        self.base
            .homes
            .get(&(top.to_string(), id.to_string()))
            .map(String::as_str)
    }

    /// A layer's rank (`world` included).
    pub fn layer_rank(&self, layer: &str) -> Option<i64> {
        self.ranks.get(layer).map(|r| r.rank)
    }

    /// A layer's owner.
    pub fn layer_owner(&self, layer: &str) -> Option<&str> {
        self.ranks.get(layer).and_then(|r| r.owner.as_deref())
    }

    /// The rank of an entity: its layer's. Kinds have none.
    pub fn rank(&self, top: &str, id: &str) -> Option<i64> {
        if top == "kind" {
            return None;
        }
        self.home(top, id).and_then(|h| self.layer_rank(h))
    }

    /// The top table and rank of an id of the world namespace, for the direction check of a name
    /// a formula reads (5.1).
    pub fn find(&self, id: &str) -> Option<(String, i64)> {
        let (top, home) = crate::check::find_id(&self.base.root, &self.base.homes, id)?;
        Some((top, self.layer_rank(home)?))
    }

    /// Every formula string of the merged world: values of keys that take a formula, and
    /// attributes whose kind default makes them formulas (7.1).
    pub fn formulas(&self) -> Vec<FormulaAt<'_>> {
        let mut out = Vec::new();
        let root = &self.base.root;
        for (top, e) in root.iter() {
            let Value::Table(t) = &e.item.value else {
                continue;
            };
            for (id, ent) in t.iter() {
                let subject = Subject::entity(top.as_str(), id.as_str());
                match (top.as_str(), &ent.item.value) {
                    ("let", Value::Str(s)) => out.push(FormulaAt { subject, text: s }),
                    (_, Value::Table(et)) => {
                        let shape = if top == "kind" {
                            Some(schema::Shape::Kind)
                        } else {
                            schema::entity_shape(top)
                        };
                        let attrs = if top == "feature" {
                            self.formula_attributes(et)
                        } else {
                            Vec::new()
                        };
                        collect(et, shape, &subject, &attrs, &mut out);
                    }
                    _ => {}
                }
            }
        }
        out
    }

    fn formula_attributes(&self, feature: &Table) -> Vec<String> {
        let Some(kind) = feature
            .str("kind")
            .and_then(|k| self.base.root.table("kind").and_then(|ks| ks.table(k)))
        else {
            return Vec::new();
        };
        match kind.table("defaults") {
            Some(d) => d
                .iter()
                .filter(|(_, e)| match &e.item.value {
                    Value::Num(_) => true,
                    Value::Str(s) => s.text.starts_with('='),
                    _ => false,
                })
                .map(|(k, _)| k.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    /// `via merge`: the canonical JSON, one line feed at the end.
    pub fn to_json(&self) -> String {
        crate::json::canonical(self)
    }

    /// `via merge --origins`: the same shape, each value that is not a table replaced by the
    /// file, line, column and chain of its key span.
    pub fn origins_json(&self) -> String {
        crate::json::origins(self)
    }

    /// The entry at a key path below an entity, and the deepest chain on the way.
    fn walk(&self, subject: &Subject, extra: &[KeyPart]) -> Option<(&Entry, Option<&Str>)> {
        let Subject::Entity {
            collection,
            id,
            keys,
        } = subject
        else {
            return None;
        };
        let mut e = self.entity_entry(collection, id)?;
        let mut item = &e.item;
        // A `let` is a value, not a table: its formula's path is its own name, which names the
        // entry itself (`let.slope` with `Expr(slope, ..)`).
        if !matches!(item.value, Value::Table(_)) {
            let own = [KeyPart::Key(id.clone())];
            let rest = |p: &'_ [KeyPart]| -> bool { p.is_empty() || p == own.as_slice() };
            if rest(keys) && rest(extra) {
                let s = match &item.value {
                    Value::Str(s) => Some(s),
                    _ => None,
                };
                return Some((e, s));
            }
        }
        // A site's path is below the subject's keys, or, when it already starts with them, below
        // the entity: `scatter.trees.density` with `Expr(density, ..)` is the one key.
        let keys: &[KeyPart] = if !keys.is_empty() && extra.starts_with(keys) {
            &[]
        } else {
            keys
        };
        for part in keys.iter().chain(extra) {
            match (part, &item.value) {
                (KeyPart::Key(k), Value::Table(t)) => {
                    e = t.get(k)?;
                    item = &e.item;
                }
                (KeyPart::Index(i), Value::List(xs)) => {
                    item = xs.get(*i as usize)?;
                }
                _ => return None,
            }
        }
        let s = match &item.value {
            Value::Str(s) => Some(s),
            _ => None,
        };
        Some((e, s))
    }

    /// A feature's key it does not write, taken from its kind (`diagnostics.md` 1.2, `defaults`
    /// in the chain table): an attribute from the kind's `defaults`, any other key (`authority`)
    /// from the kind itself. The chain ends with the `defaults` entry at the feature's header.
    fn inherited(
        &self,
        subject: &Subject,
        extra: &[KeyPart],
    ) -> Option<(&Entry, Option<&Str>, via_diag::ChainEntry)> {
        let Subject::Entity {
            collection,
            id,
            keys,
        } = subject
        else {
            return None;
        };
        // A feature takes from its `kind`; a hand instance's footprint from its `ground` kind.
        let via = match collection.as_str() {
            "feature" => "kind",
            "instance" => "ground",
            _ => return None,
        };
        let feature = self.entity_entry(collection, id)?;
        let Value::Table(ft) = &feature.item.value else {
            return None;
        };
        let kind = ft.str(via)?;
        let kind_subject = Subject::entity("kind", kind);
        let path: Vec<KeyPart> = if !keys.is_empty() && extra.starts_with(keys) {
            extra.to_vec()
        } else {
            keys.iter().chain(extra).cloned().collect()
        };
        let first = path.first()?;
        // Only a key the feature does not write is inherited.
        if let KeyPart::Key(k) = first {
            if ft.get(k).is_some() {
                return None;
            }
        }
        let mut under_defaults = vec![KeyPart::Key("defaults".to_string())];
        under_defaults.extend(path.iter().cloned());
        let (e, s) = self
            .walk(&kind_subject, &under_defaults)
            .or_else(|| self.walk(&kind_subject, &path))?;
        let header = ft.header();
        let link = via_diag::ChainEntry {
            through: via_diag::Through::Defaults,
            name: format!("{collection}.{id}"),
            file: self.sources.name(header.src).to_string(),
            line: self.sources.line(header),
            owner: None,
        };
        Some((e, s, link))
    }

    fn resolved(&self, span: Span, entry: &Entry) -> Resolved {
        Resolved {
            location: self.sources.location(span),
            chain: chain_entries(&self.sources, &entry.chain),
        }
    }
}

fn collect<'w>(
    t: &'w Table,
    shape: Option<schema::Shape>,
    subject: &Subject,
    attrs: &[String],
    out: &mut Vec<FormulaAt<'w>>,
) {
    use schema::{Rule, Takes};
    for (k, e) in t.iter() {
        let takes = match shape {
            Some(schema::Shape::Let) => Some(Takes::Formula),
            Some(s) => match schema::rule(s, k) {
                Some(Rule::Ok(t)) => Some(t),
                _ if attrs.contains(k) => Some(Takes::Formula),
                _ => None,
            },
            None => None,
        };
        let sub = subject
            .clone()
            .with_keys(&via_diag::KeyPath::key(k.clone()));
        match (takes, &e.item.value) {
            (Some(Takes::Formula), Value::Str(s)) => out.push(FormulaAt {
                subject: sub,
                text: s,
            }),
            (Some(Takes::Table(s)), Value::Table(st)) => collect(st, Some(s), &sub, &[], out),
            (Some(Takes::Ids(s)), Value::Table(st)) => {
                for (id, ie) in st.iter() {
                    if let Value::Table(it) = &ie.item.value {
                        let isub = sub.clone().with_keys(&via_diag::KeyPath::key(id.clone()));
                        collect(it, Some(s), &isub, &[], out);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Resolve for World {
    fn resolve(&self, subject: &Subject, site: &Site) -> Option<Resolved> {
        match site {
            Site::Header => {
                let Subject::Entity { collection, id, .. } = subject else {
                    return None;
                };
                let e = self.entity_entry(collection, id)?;
                let span = match &e.item.value {
                    Value::Table(t) => t.header(),
                    _ => e.key,
                };
                Some(self.resolved(span, e))
            }
            Site::Key(path) | Site::Selector(path) => {
                if let Subject::World = subject {
                    let mut parts = path.parts().iter();
                    let Some(KeyPart::Key(k)) = parts.next() else {
                        return None;
                    };
                    // A world key's table (`[budget]`, `build.md` 5.1) is walked as far as the
                    // path holds, so a record names the deepest key written.
                    let mut e = self.base.root.get(k)?;
                    for p in parts {
                        let (KeyPart::Key(k), Value::Table(t)) = (p, &e.item.value) else {
                            break;
                        };
                        match t.get(k) {
                            Some(below) => e = below,
                            None => break,
                        }
                    }
                    return Some(self.resolved(e.key, e));
                }
                if let Some((e, _)) = self.walk(subject, path.parts()) {
                    return Some(self.resolved(e.key, e));
                }
                let (e, _, link) = self.inherited(subject, path.parts())?;
                let mut r = self.resolved(e.key, e);
                r.chain.push(link);
                Some(r)
            }
            Site::Expr(path, range) => {
                let (e, s, link) = match self.walk(subject, path.parts()) {
                    Some((e, s)) => (e, s, None),
                    None => {
                        let (e, s, l) = self.inherited(subject, path.parts())?;
                        (e, s, Some(l))
                    }
                };
                let s = s?;
                let (start, end) = s.file_range(range.start as usize, range.end as usize)?;
                let span = Span {
                    src: e.key.src,
                    start,
                    end,
                };
                let mut r = self.resolved(span, e);
                r.chain.extend(link);
                Some(r)
            }
            Site::File { .. } | Site::Text { .. } | Site::Argument(_) | Site::World => None,
        }
    }

    fn world_file(&self) -> String {
        self.world_file.clone()
    }
}

impl World {
    /// The location of a key below an entity, for tests and the CLI's `--origins` checks.
    pub fn key_location(&self, subject: &Subject) -> Option<Location> {
        let (e, _) = self.walk(subject, &[])?;
        Some(self.sources.location(e.key))
    }
}
