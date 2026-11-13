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
    let changed = changed_statements(run, baseline);
    let mut prev: BTreeMap<&NodeId, &NodeId> = BTreeMap::new();
    let mut seen: BTreeSet<&NodeId> = BTreeSet::new();
    let mut queue: VecDeque<&NodeId> = VecDeque::new();
    let mut found: BTreeMap<String, Vec<NodeId>> = BTreeMap::new();
    seen.insert(target);
    queue.push_back(target);
    while let Some(n) = queue.pop_front() {
        for s in run.made_from.get(n).into_iter().flatten() {
            if changed.contains(s) && !found.contains_key(s) {
                let mut path = vec![n.clone()];
                let mut at = n;
                while let Some(p) = prev.get(at) {
                    path.push((*p).clone());
                    at = p;
                }
                found.insert(s.clone(), path);
            }
        }
        let mut inputs: Vec<&NodeId> = run
            .edges
            .get(n)
            .into_iter()
            .flatten()
            .map(|(_, t)| t)
            .collect();
        inputs.sort();
        inputs.dedup();
        for t in inputs {
            if seen.insert(t) {
                prev.insert(t, n);
                queue.push_back(t);
            }
        }
    }
    found
        .into_iter()
        .map(|(statement, path)| ChangeCause { statement, path })
        .collect()
}
