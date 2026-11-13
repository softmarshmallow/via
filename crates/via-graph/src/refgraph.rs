//! The reference graph and its one cycle report (`spec/graph.md` 2.1 and 2.2).
//!
//! The reference graph is known once `merge` has run. Its vertices are node ids with view,
//! window and item removed (a `head` view a vertex of its own, and one vertex per `let`), each
//! belonging to one statement, its **member**. Cycles are found by Tarjan's algorithm before any
//! other node runs; each strongly connected component with more than one vertex, or a vertex
//! with an edge to itself, is one `graph.cycle`, naming one loop in source order.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use via_diag::{codes, Diag, KeyPath, Related, Site, Subject, Text};

/// Where a statement stands in source order (2.2, item 1): by layer rank, then file path
/// bytewise, then line. A statement with no rank (a kind, the pack) comes after every ranked one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOrder {
    /// The layer rank; `None` for kinds and the pack.
    pub rank: Option<i64>,
    /// The file, as a record writes it (`world.toml`, `lib:<library>/kinds.toml`).
    pub file: String,
    /// The 1-based line of the statement's header.
    pub line: u32,
}

impl Ord for SourceOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        let rank = match (self.rank, other.rank) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        rank.then_with(|| self.file.as_bytes().cmp(other.file.as_bytes()))
            .then(self.line.cmp(&other.line))
    }
}

impl PartialOrd for SourceOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A statement as a member of the reference graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    /// The statement's entity address: `feature.a`, `field.height`, `let.k`.
    pub subject: Subject,
    /// Its place in source order.
    pub order: SourceOrder,
}

/// One reference: from the reader's vertex to the vertex it reads.
#[derive(Clone, Debug, PartialEq)]
pub struct RefEdge {
    /// The reader's vertex.
    pub from: String,
    /// The vertex read.
    pub to: String,
    /// The edge's label (2): `on.ref`, `base reads b`.
    pub label: String,
    /// The key under the reader's statement that makes the edge (`on.ref`, `ends_at`).
    pub key: KeyPath,
}

/// The reference graph: vertices with their members, and the references between them.
#[derive(Clone, Debug, Default)]
pub struct RefGraph {
    vertices: BTreeMap<String, String>,
    members: BTreeMap<String, Member>,
    edges: Vec<RefEdge>,
}

/// One `graph.cycle`: the diagnostic and what fails under it as one group.
#[derive(Clone, Debug)]
pub struct Cycle {
    /// The `graph.cycle` error.
    pub diag: Diag,
    /// The member ids of the component, the loop first, then the rest in source order.
    pub members: Vec<String>,
    /// Every vertex of those members: their nodes fail as one group.
    pub vertices: BTreeSet<String>,
}

impl RefGraph {
    /// An empty graph.
    pub fn new() -> RefGraph {
        RefGraph::default()
    }

    /// Adds a statement. `id` is its member id, normally its subject as text.
    pub fn member(&mut self, id: impl Into<String>, member: Member) {
        self.members.insert(id.into(), member);
    }

    /// Adds a vertex belonging to a member.
    pub fn vertex(&mut self, vertex: impl Into<String>, member: impl Into<String>) {
        self.vertices.insert(vertex.into(), member.into());
    }

    /// Adds a reference.
    pub fn edge(&mut self, edge: RefEdge) {
        self.edges.push(edge);
    }

    /// The member of a vertex.
    pub fn member_of(&self, vertex: &str) -> Option<&str> {
        self.vertices.get(vertex).map(String::as_str)
    }

    /// Every cycle, ordered by its first member's source order. An edge to a vertex that was
    /// never added is ignored here: the reader of a name that does not exist is the document
    /// checks' to report.
    pub fn cycles(&self) -> Vec<Cycle> {
        let ids: Vec<&String> = self.vertices.keys().collect();
        let index: BTreeMap<&str, usize> = ids
            .iter()
            .enumerate()
            .map(|(i, v)| (v.as_str(), i))
            .collect();
        let mut adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); ids.len()];
        for e in &self.edges {
            if let (Some(&a), Some(&b)) = (index.get(e.from.as_str()), index.get(e.to.as_str())) {
                adj[a].insert(b);
            }
        }
        let mut out = Vec::new();
        for comp in tarjan(&adj) {
            let looped = comp.len() > 1 || adj[comp[0]].contains(&comp[0]);
            if !looped {
                continue;
            }
            let vset: BTreeSet<&str> = comp.iter().map(|&i| ids[i].as_str()).collect();
            if let Some(c) = self.report(&vset) {
                out.push(c);
            }
        }
        out.sort_by(|a, b| {
            let oa = self.members.get(&a.members[0]).map(|m| &m.order);
            let ob = self.members.get(&b.members[0]).map(|m| &m.order);
            oa.cmp(&ob).then_with(|| a.members[0].cmp(&b.members[0]))
        });
        out
    }

    fn order_of<'a>(&'a self, member: &'a str) -> (Option<&'a SourceOrder>, &'a str) {
        (self.members.get(member).map(|m| &m.order), member)
    }

    fn subject_of(&self, member: &str) -> Subject {
        self.members
            .get(member)
            .map(|m| m.subject.clone())
            .unwrap_or_else(|| Subject::Selector(member.to_string()))
    }

    fn report(&self, vset: &BTreeSet<&str>) -> Option<Cycle> {
        // Members, and the member graph inside the component with the lowest label per pair.
        let mut members: BTreeSet<&str> = BTreeSet::new();
        for v in vset {
            members.insert(self.vertices.get(*v)?.as_str());
        }
        let mut medges: BTreeMap<(&str, &str), &RefEdge> = BTreeMap::new();
        for e in &self.edges {
            if !(vset.contains(e.from.as_str()) && vset.contains(e.to.as_str())) {
                continue;
            }
            let a = self.vertices.get(&e.from)?.as_str();
            let b = self.vertices.get(&e.to)?.as_str();
            let slot = medges.entry((a, b)).or_insert(e);
            if e.label.as_bytes() < slot.label.as_bytes() {
                *slot = e;
            }
        }
        let mut sorted: Vec<&str> = members.iter().copied().collect();
        sorted.sort_by(|a, b| self.order_of(a).cmp(&self.order_of(b)));
        let first = sorted[0];
        let rank: BTreeMap<&str, usize> = sorted.iter().enumerate().map(|(i, m)| (*m, i)).collect();
        let mut next: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (a, b) in medges.keys() {
            next.entry(a).or_default().push(b);
        }
        for list in next.values_mut() {
            list.sort_by_key(|m| rank.get(m).copied().unwrap_or(usize::MAX));
        }
        // Breadth first from the first member back to itself; neighbours in source order.
        let lp = shortest_loop(first, &next)?;
        let mut lines: Vec<(&str, &RefEdge)> = Vec::new();
        for w in 0..lp.len() {
            let a = lp[w];
            let b = lp[(w + 1) % lp.len()];
            lines.push((a, medges.get(&(a, b))?));
        }
        let in_loop: BTreeSet<&str> = lp.iter().copied().collect();
        let rest: Vec<&str> = sorted
            .iter()
            .copied()
            .filter(|m| !in_loop.contains(m))
            .collect();

        let subject = self.subject_of(first);
        let n = lp.len();
        let message = format!(
            "references form a loop of {n} statement{}",
            if n == 1 { "" } else { "s" }
        );
        let keys: Vec<String> = lines
            .iter()
            .map(|(m, e)| format!("{} {}", self.subject_of(m), e.key))
            .collect();
        let mut hint = match keys.len() {
            1 => format!("remove {}; it reads its own statement", keys[0]),
            2 => format!("remove {} or {}; either breaks this loop", keys[0], keys[1]),
            _ => format!(
                "remove {} or {}; any one breaks this loop",
                keys[..keys.len() - 1].join(", "),
                keys[keys.len() - 1]
            ),
        };
        if !rest.is_empty() {
            let names: Vec<String> = rest
                .iter()
                .map(|m| self.subject_of(m).to_string())
                .collect();
            hint.push_str(&format!(
                "; {} {} other loops",
                names.join(", "),
                if names.len() == 1 { "holds" } else { "hold" }
            ));
        }
        let mut diag = Diag::new(
            codes::graph::CYCLE,
            subject,
            Site::Key(lines[0].1.key.clone()),
            text(message),
            text(hint),
        );
        for (m, e) in &lines {
            let to = self.vertices.get(&e.to).map_or("", String::as_str);
            diag = diag.related(Related {
                subject: self.subject_of(m),
                site: Site::Key(e.key.clone()),
                position: None,
                note: text(format!(
                    "{} = {} -> {}",
                    e.key,
                    e.label,
                    self.subject_of(to)
                )),
            });
        }
        for m in &rest {
            diag = diag.related(Related {
                subject: self.subject_of(m),
                site: Site::Header,
                position: None,
                note: text("in the same group of loops".to_string()),
            });
        }
        let vertices = self
            .vertices
            .iter()
            .filter(|(_, m)| members.contains(m.as_str()))
            .map(|(v, _)| v.clone())
            .collect();
        let mut ordered: Vec<String> = lp.iter().map(|m| m.to_string()).collect();
        ordered.extend(rest.iter().map(|m| m.to_string()));
        Some(Cycle {
            diag,
            members: ordered,
            vertices,
        })
    }
}

fn text(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| via_diag::text!("a reference loop"))
}

/// The shortest cycle through `start`, found breadth first; ties go to the neighbour first in
/// the lists' order (source order).
fn shortest_loop<'a>(
    start: &'a str,
    next: &BTreeMap<&'a str, Vec<&'a str>>,
) -> Option<Vec<&'a str>> {
    let mut prev: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    queue.push_back(start);
    while let Some(m) = queue.pop_front() {
        for &n in next.get(m).map(Vec::as_slice).unwrap_or(&[]) {
            if n == start {
                let mut path = vec![m];
                let mut at = m;
                while at != start {
                    at = prev.get(at)?;
                    path.push(at);
                }
                path.reverse();
                return Some(path);
            }
            if !prev.contains_key(n) {
                prev.insert(n, m);
                queue.push_back(n);
            }
        }
    }
    None
}

/// Tarjan's strongly connected components, iterative so a long chain cannot overflow the
/// stack. Components come out in reverse topological order; each lists its vertices.
fn tarjan(adj: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    let mut counter = 0usize;
    let lists: Vec<Vec<usize>> = adj.iter().map(|s| s.iter().copied().collect()).collect();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = counter;
        low[root] = counter;
        counter += 1;
        stack.push(root);
        on[root] = true;
        while let Some(&mut (v, ref mut i)) = call.last_mut() {
            if *i < lists[v].len() {
                let w = lists[v][*i];
                *i += 1;
                if index[w] == usize::MAX {
                    index[w] = counter;
                    low[w] = counter;
                    counter += 1;
                    stack.push(w);
                    on[w] = true;
                    call.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(u, _)) = call.last() {
                    low[u] = low[u].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    comp.sort_unstable();
                    out.push(comp);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(rank: Option<i64>, file: &str, line: u32) -> SourceOrder {
        SourceOrder {
            rank,
            file: file.into(),
            line,
        }
    }

    fn graph(entries: &[(&str, &str, u32)]) -> RefGraph {
        let mut g = RefGraph::new();
        for (coll, id, line) in entries {
            let m = format!("{coll}.{id}");
            g.member(
                m.clone(),
                Member {
                    subject: Subject::entity(*coll, *id),
                    order: order(Some(0), "world.toml", *line),
                },
            );
            g.vertex(format!("shape:{id}"), m.clone());
            g.vertex(format!("attr:{id}"), m);
        }
        g
    }

    fn edge(g: &mut RefGraph, from: &str, to: &str, key: &str) {
        g.edge(RefEdge {
            from: from.into(),
            to: to.into(),
            label: key.into(),
            key: key
                .split('.')
                .fold(None, |p: Option<KeyPath>, k| {
                    Some(match p {
                        None => KeyPath::key(k),
                        Some(p) => p.then(k),
                    })
                })
                .unwrap(),
        });
    }

    #[test]
    fn two_member_loop_in_source_order() {
        let mut g = graph(&[
            ("feature", "beta", 30),
            ("feature", "alpha", 25),
            ("feature", "gamma", 40),
        ]);
        edge(&mut g, "shape:alpha", "shape:beta", "on.ref");
        edge(&mut g, "shape:beta", "shape:alpha", "ends_at");
        edge(&mut g, "shape:gamma", "shape:alpha", "on.ref");
        let cs = g.cycles();
        assert_eq!(cs.len(), 1);
        let c = &cs[0];
        assert_eq!(c.members, vec!["feature.alpha", "feature.beta"]);
        assert_eq!(c.diag.subject().to_string(), "feature.alpha");
        assert_eq!(c.diag.site(), &Site::Key(KeyPath::key("on").then("ref")));
        assert_eq!(
            c.diag.message().as_str(),
            "references form a loop of 2 statements"
        );
        assert_eq!(
            c.diag.hint().as_str(),
            "remove feature.alpha on.ref or feature.beta ends_at; either breaks this loop"
        );
        assert!(c.vertices.contains("attr:beta") && !c.vertices.contains("shape:gamma"));
    }

    #[test]
    fn ring_declared_in_reverse_starts_at_first_in_source_order() {
        let mut g = graph(&[("field", "c", 10), ("field", "b", 20), ("field", "a", 30)]);
        edge(&mut g, "shape:a", "shape:b", "base");
        edge(&mut g, "shape:b", "shape:c", "base");
        edge(&mut g, "shape:c", "shape:a", "base");
        let cs = g.cycles();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].members, vec!["field.c", "field.a", "field.b"]);
    }

    #[test]
    fn self_loop_and_rest_of_component() {
        let mut g = graph(&[
            ("feature", "a", 1),
            ("feature", "b", 2),
            ("feature", "c", 3),
        ]);
        edge(&mut g, "shape:a", "shape:a", "where");
        assert_eq!(
            g.cycles()[0].diag.message().as_str(),
            "references form a loop of 1 statement"
        );
        let mut g = graph(&[
            ("feature", "a", 1),
            ("feature", "b", 2),
            ("feature", "c", 3),
        ]);
        edge(&mut g, "shape:a", "shape:b", "x");
        edge(&mut g, "shape:b", "shape:a", "y");
        edge(&mut g, "shape:b", "shape:c", "z");
        edge(&mut g, "shape:c", "shape:a", "w");
        let c = &g.cycles()[0];
        assert_eq!(c.members, vec!["feature.a", "feature.b", "feature.c"]);
        assert!(c
            .diag
            .hint()
            .as_str()
            .ends_with("; feature.c holds other loops"));
    }

    #[test]
    fn unranked_comes_after_ranked() {
        assert!(order(Some(5), "z", 9) < order(None, "a", 1));
        assert!(order(Some(0), "a.toml", 9) < order(Some(0), "b.toml", 1));
        assert!(order(Some(0), "a.toml", 2) < order(Some(1), "a.toml", 1));
    }

    #[test]
    fn long_chain_does_not_overflow() {
        let mut g = RefGraph::new();
        let n = 200_000;
        for i in 0..n {
            g.vertex(format!("v:{i}"), format!("m{i}"));
            if i > 0 {
                g.edge(RefEdge {
                    from: format!("v:{i}"),
                    to: format!("v:{}", i - 1),
                    label: "x".into(),
                    key: KeyPath::key("x"),
                });
            }
        }
        assert!(g.cycles().is_empty());
    }
}
