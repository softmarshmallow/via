//! What the change report gets from the graph (`spec/graph.md` 6.4): for each changed thing,
//! every statement whose digest differs from the baseline's and that it depends on, each with the
//! shortest edge path from that statement to it, ties by node id.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::engine::Run;
use crate::id::NodeId;
use crate::record::BuildRecord;

/// One cause of a change: the statement, and the path of nodes from the first node made from it
/// to the changed node, that node last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeCause {
    /// The statement id.
    pub statement: String,
    /// The edge path: first the node made from the statement, last the changed node.
    pub path: Vec<NodeId>,
}

/// The statements whose digest differs from the baseline's, or that only one of them has.
pub fn changed_statements(run: &Run, baseline: &BuildRecord) -> BTreeSet<String> {
    run.statements
        .iter()
        .filter(|(s, d)| baseline.statements.get(*s) != Some(d))
        .map(|(s, _)| s.clone())
        .chain(
            baseline
                .statements
                .keys()
                .filter(|s| !run.statements.contains_key(*s))
                .cloned(),
        )
        .collect()
}

/// The causes of a change to `target`, sorted by statement id. Breadth first from the target
/// along its inputs, taken in id order, so the first node found for a statement ends the
/// shortest path, ties going to the lowest ids.
pub fn causes(run: &Run, baseline: &BuildRecord, target: &NodeId) -> Vec<ChangeCause> {
    causes_from(
        &run.statements,
        &run.made_from,
        &run.edges,
        baseline,
        target,
    )
}

/// [`causes`] from a graph's parts rather than a finished run: each statement's digest, the
/// statements each node is made from, and each node's input edges. A node defined last (the
/// change report) has no run yet, and its plan knows these from its definitions.
pub fn causes_from(
    statements: &BTreeMap<String, crate::digest::Digest>,
    made_from: &BTreeMap<NodeId, Vec<String>>,
    edges: &BTreeMap<NodeId, Vec<(String, NodeId)>>,
    baseline: &BuildRecord,
    target: &NodeId,
) -> Vec<ChangeCause> {
    CauseGraph::new(statements, made_from, edges, baseline).causes(target)
}

/// [`causes_from`] for many targets over one graph. The changed statements, the inputs in id
/// order and the nodes that depend on a changed statement at all are found once; each target's
/// search then walks only those nodes. A node that reaches no changed statement is never the
/// parent of one that does on a breadth-first path (its child would make it reach one), so leaving
/// it out changes no path: the answers equal [`causes_from`]'s. The change report asked one full
/// search per changed thing, each sorting every node's inputs again: quay's distant edit spent
/// 3.7 s of its 17 s in it (M5 close).
pub struct CauseGraph<'a> {
    ids: Vec<&'a NodeId>,
    index: BTreeMap<&'a NodeId, u32>,
    /// Each node's inputs, by index (which is id order), deduplicated.
    inputs: Vec<Vec<u32>>,
    /// The changed statements each node is made from.
    made: Vec<Vec<&'a str>>,
    /// Whether a changed statement is reachable from the node along its inputs.
    relevant: Vec<bool>,
}

impl<'a> CauseGraph<'a> {
    /// The graph of `made_from` and `edges`, against `baseline`'s statements.
    pub fn new(
        statements: &BTreeMap<String, crate::digest::Digest>,
        made_from: &'a BTreeMap<NodeId, Vec<String>>,
        edges: &'a BTreeMap<NodeId, Vec<(String, NodeId)>>,
        baseline: &BuildRecord,
    ) -> CauseGraph<'a> {
        let changed: BTreeSet<&str> = statements
            .iter()
            .filter(|(s, d)| baseline.statements.get(*s) != Some(d))
            .map(|(s, _)| s.as_str())
            .chain(
                baseline
                    .statements
                    .keys()
                    .filter(|s| !statements.contains_key(*s))
                    .map(String::as_str),
            )
            .collect();
        let mut all: BTreeSet<&'a NodeId> = made_from.keys().collect();
        for (n, es) in edges {
            all.insert(n);
            all.extend(es.iter().map(|(_, t)| t));
        }
        let ids: Vec<&'a NodeId> = all.into_iter().collect();
        let index: BTreeMap<&'a NodeId, u32> = ids
            .iter()
            .enumerate()
            .map(|(i, n)| (*n, u32::try_from(i).unwrap_or(u32::MAX)))
            .collect();
        let mut inputs: Vec<Vec<u32>> = vec![Vec::new(); ids.len()];
        let mut readers: Vec<Vec<u32>> = vec![Vec::new(); ids.len()];
        for (n, es) in edges {
            let i = index[n];
            let mut v: Vec<u32> = es.iter().map(|(_, t)| index[t]).collect();
            v.sort_unstable();
            v.dedup();
            for t in &v {
                readers[*t as usize].push(i);
            }
            inputs[i as usize] = v;
        }
        let mut made: Vec<Vec<&'a str>> = vec![Vec::new(); ids.len()];
        let mut relevant = vec![false; ids.len()];
        let mut queue: VecDeque<u32> = VecDeque::new();
        for (n, ss) in made_from {
            let i = index[n] as usize;
            made[i] = ss
                .iter()
                .map(String::as_str)
                .filter(|s| changed.contains(s))
                .collect();
            if !made[i].is_empty() && !relevant[i] {
                relevant[i] = true;
                queue.push_back(index[n]);
            }
        }
        while let Some(n) = queue.pop_front() {
            for r in &readers[n as usize] {
                if !relevant[*r as usize] {
                    relevant[*r as usize] = true;
                    queue.push_back(*r);
                }
            }
        }
        CauseGraph {
            ids,
            index,
            inputs,
            made,
            relevant,
        }
    }

    /// The causes of a change to `target`, sorted by statement id, as [`causes_from`].
    pub fn causes(&self, target: &NodeId) -> Vec<ChangeCause> {
        let Some(&t) = self.index.get(target) else {
            return Vec::new();
        };
        if !self.relevant[t as usize] {
            return Vec::new();
        }
        let mut prev: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut queue: VecDeque<u32> = VecDeque::new();
        let mut found: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
        seen.insert(t);
        queue.push_back(t);
        while let Some(n) = queue.pop_front() {
            for s in &self.made[n as usize] {
                if !found.contains_key(s) {
                    let mut path = vec![self.ids[n as usize].clone()];
                    let mut at = n;
                    while let Some(p) = prev.get(&at) {
                        path.push(self.ids[*p as usize].clone());
                        at = *p;
                    }
                    found.insert(s, path);
                }
            }
            for i in &self.inputs[n as usize] {
                if self.relevant[*i as usize] && seen.insert(*i) {
                    prev.insert(*i, n);
                    queue.push_back(*i);
                }
            }
        }
        found
            .into_iter()
            .map(|(statement, path)| ChangeCause {
                statement: statement.to_string(),
                path,
            })
            .collect()
    }
}
