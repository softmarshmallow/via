//! Node ids (`spec/graph.md` 1): `<family>:<subject>`, then `/v:<view>`, `/w:<window>` and
//! `/i:<item>` where the family has them. Ids sort bytewise.

use std::fmt;

/// A node id. Sorted bytewise; every order the chapter gives that no key settles is by id.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

/// Why a text is not a node id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadId(pub String);

impl NodeId {
    /// Checks the form: a family of `[a-z]+` alone (`merge`), or a family, a `:`, a non-empty subject, and then only the
    /// suffixes `/v:`, `/w:` and `/i:`, each at most once, in that order, none empty.
    pub fn parse(s: &str) -> Result<NodeId, BadId> {
        let bad = |why: &str| BadId(format!("`{s}` is not a node id: {why}"));
        // A family with one node is its own id (`merge`).
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase()) {
            return Ok(NodeId(s.to_string()));
        }
        let (family, rest) = s.split_once(':').ok_or_else(|| bad("no `:`"))?;
        if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
            return Err(bad("the family is not [a-z]+"));
        }
        let mut parts = rest.split("/v:");
        let head = parts.next().unwrap_or("");
        let after_view = parts.next();
        if parts.next().is_some() {
            return Err(bad("two views"));
        }
        let subject_end = head
            .find("/w:")
            .or_else(|| head.find("/i:"))
            .unwrap_or(head.len());
        if subject_end == 0 {
            return Err(bad("the subject is empty"));
        }
        if after_view.is_some() && subject_end != head.len() {
            return Err(bad(
                "suffixes are /v:, /w:, /i:, once each, in order, none empty",
            ));
        }
        let tail = match after_view {
            Some(v) => v,
            None => &head[subject_end..],
        };
        let mut last = 0u8;
        for seg in tail_segments(tail, after_view.is_some()) {
            let (tag, body) = seg;
            let rank = match tag {
                'v' => 1,
                'w' => 2,
                'i' => 3,
                _ => return Err(bad("an unknown suffix")),
            };
            if rank <= last || body.is_empty() {
                return Err(bad(
                    "suffixes are /v:, /w:, /i:, once each, in order, none empty",
                ));
            }
            last = rank;
        }
        Ok(NodeId(s.to_string()))
    }

    /// Builds an id from its parts. The subject is used as given.
    pub fn of(family: &str, subject: &str) -> NodeId {
        NodeId(format!("{family}:{subject}"))
    }

    /// This id with `/v:<view>` appended.
    #[must_use]
    pub fn view(self, view: &str) -> NodeId {
        NodeId(format!("{}/v:{view}", self.0))
    }

    /// This id with `/w:<i0>.<j0>.<i1>.<j1>` appended (posts, `i1` and `j1` excluded).
    #[must_use]
    pub fn window(self, i0: u64, j0: u64, i1: u64, j1: u64) -> NodeId {
        NodeId(format!("{}/w:{i0}.{j0}.{i1}.{j1}", self.0))
    }

    /// This id with `/i:<item>` appended.
    #[must_use]
    pub fn item(self, item: &str) -> NodeId {
        NodeId(format!("{}/i:{item}", self.0))
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The family: the part before the first `:`.
    pub fn family(&self) -> &str {
        self.0.split_once(':').map_or(&self.0[..], |(f, _)| f)
    }

    /// The view, if any.
    pub fn view_name(&self) -> Option<&str> {
        let (_, rest) = self.0.split_once("/v:")?;
        Some(cut_at_suffix(rest))
    }

    /// The vertex of the reference graph this node belongs to (2.1): the id with `/v:`, `/w:`
    /// and `/i:` removed, except that a `head` view (`head`, `<layer>.head`) is a vertex of its
    /// own, written `<family>:<subject>/v:head`.
    pub fn vertex(&self) -> String {
        let base = cut_at_suffix(&self.0);
        match self.view_name() {
            Some(v) if v == "head" || v.ends_with(".head") => format!("{base}/v:head"),
            _ => base.to_string(),
        }
    }
}

fn cut_at_suffix(s: &str) -> &str {
    let mut end = s.len();
    for tag in ["/v:", "/w:", "/i:"] {
        if let Some(p) = s.find(tag) {
            end = end.min(p);
        }
    }
    &s[..end]
}

fn tail_segments(tail: &str, has_view: bool) -> Vec<(char, &str)> {
    let mut out = Vec::new();
    let mut rest = tail;
    if has_view {
        let end = rest
            .find("/w:")
            .or_else(|| rest.find("/i:"))
            .unwrap_or(rest.len());
        out.push(('v', &rest[..end]));
        rest = &rest[end..];
    }
    while let Some(r) = rest.strip_prefix('/') {
        let mut chars = r.chars();
        let tag = chars.next().unwrap_or('?');
        let body = chars.as_str().strip_prefix(':').unwrap_or("");
        let end = body
            .find("/w:")
            .or_else(|| body.find("/i:"))
            .unwrap_or(body.len());
        out.push((tag, &body[..end]));
        rest = &body[end..];
    }
    if !rest.is_empty() {
        out.push(('?', rest));
    }
    out
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_and_vertices() {
        let id = NodeId::of("field", "height")
            .view("build")
            .window(0, 0, 65, 65);
        assert_eq!(id.as_str(), "field:height/v:build/w:0.0.65.65");
        assert_eq!(NodeId::parse(id.as_str()), Ok(id.clone()));
        assert_eq!(id.family(), "field");
        assert_eq!(id.view_name(), Some("build"));
        assert_eq!(id.vertex(), "field:height");
        let head = NodeId::of("field", "height").view("base.head");
        assert_eq!(head.vertex(), "field:height/v:head");
        let item = NodeId::of("step", "checks").item("north");
        assert_eq!(item.vertex(), "step:checks");
        let merge = NodeId::parse("merge").unwrap();
        assert_eq!(
            (merge.family(), merge.vertex().as_str()),
            ("merge", "merge")
        );
    }

    #[test]
    fn malformed_ids_are_refused() {
        for s in [
            "field x",
            ":x",
            "Field:x",
            "field:",
            "field:x/v:",
            "field:x/w:1/v:b",
            "field:x/v:a/v:b",
        ] {
            assert!(NodeId::parse(s).is_err(), "{s}");
        }
        // A subject may hold `/` and `:` (paths); only the three suffixes are read.
        assert!(NodeId::parse("source:lib:alpha/kinds.toml").is_ok());
        assert!(NodeId::parse("input:data/q:1").is_ok());
        assert!(NodeId::parse("write:fields/height.npy").is_ok());
    }
}
