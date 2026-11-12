//! Skipped dependents (`spec/diagnostics.md` section 4, "Skipped nodes" and `diag.skipped`).
//!
//! Every node that depends on a failed node, directly or through others, is `skipped`: it does
//! not run and raises nothing of its own. The engine says one thing about them, one `diag.skipped`
//! per failed node (or per cycle). This module is generic over the graph's node type, so the graph
//! crate keeps its own nodes and asks here only for the closure and the record.

use crate::codes;
use crate::collect::text;
use crate::diag::Diag;
use crate::site::Site;
use crate::subject::Subject;
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Every node reached from `failed` through `readers` (node to the nodes that read it), the
/// failed nodes themselves excluded. Pass one failed node, or every member of one cycle as a
/// group, to get what that failure skips.
pub fn dependents<N: Ord + Clone>(
    readers: &BTreeMap<N, BTreeSet<N>>,
    failed: &BTreeSet<N>,
) -> BTreeSet<N> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<&N> = failed.iter().collect();
    while let Some(n) = stack.pop() {
        if let Some(rs) = readers.get(n) {
            for r in rs {
                if !failed.contains(r) && out.insert(r.clone()) {
                    stack.push(r);
                }
            }
        }
    }
    out
}

/// The `diag.skipped` for one failure, or `None` when it skipped nothing with a subject.
///
/// - `failed`: the subject of the failed node's first error in sort order, or of the
///   `graph.cycle` for a cycle; its key path is dropped. The location is the span that creates
///   that entity, or the world file's start for `world`.
/// - `skipped`: the subjects of the skipped nodes (`None` for a node made from no statement).
///   `count` is the distinct subjects, the failed node's own excluded; `data.first` the first ten
///   of them, bytewise.
pub fn skipped<'a>(
    failed: &Subject,
    skipped: impl IntoIterator<Item = Option<&'a Subject>>,
) -> Option<Diag> {
    let own = failed.address();
    let names: BTreeSet<String> = skipped
        .into_iter()
        .flatten()
        .filter(|s| **s != own)
        .map(|s| s.to_string())
        .collect();
    if names.is_empty() {
        return None;
    }
    let count = u32::try_from(names.len()).unwrap_or(u32::MAX);
    let first: Vec<&String> = names.iter().take(10).collect();
    let site = if own == Subject::World {
        Site::World
    } else {
        Site::Header
    };
    let listed: Vec<String> = first.iter().map(|n| format!("`{n}`")).collect();
    Some(
        Diag::new(
            codes::diag::SKIPPED,
            own.clone(),
            site,
            text(format!(
                "{count} {} not built because `{own}` failed: {}{}",
                if count == 1 {
                    "thing was"
                } else {
                    "things were"
                },
                listed.join(", "),
                if names.len() > 10 { ", ..." } else { "" }
            )),
            text(format!(
                "fix the error on `{own}`; what depends on it is built again with it"
            )),
        )
        .counted(count)
        .with("first", Value::texts(first.into_iter().cloned())),
    )
}

/// The `diag.skipped` for export steps that did not run because the build had an error or a
/// `failed_on` record (section 4, "Writing"): subject `world`, `data.stage = "export"`.
pub fn exports_skipped(count: u32) -> Option<Diag> {
    (count > 0).then(|| {
        Diag::new(
            codes::diag::SKIPPED,
            Subject::World,
            Site::World,
            text(format!(
                "{count} export {} did not run because the build has errors",
                if count == 1 { "step" } else { "steps" }
            )),
            crate::text!("fix the errors; export steps run on a build without any"),
        )
        .counted(count)
        .with("stage", "export")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_skips_transitively_and_not_the_group() {
        let mut readers: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        readers.insert(1, [2].into());
        readers.insert(2, [3, 1].into());
        readers.insert(3, [4].into());
        readers.insert(5, [6].into());
        let d = dependents(&readers, &[1, 2].into());
        assert_eq!(d, [3, 4].into());
        assert_eq!(dependents(&readers, &[5].into()), [6].into());
        assert!(dependents(&readers, &[4].into()).is_empty());
    }

    #[test]
    fn skipped_counts_distinct_subjects() {
        let failed = Subject::parse("scatter.shrubs.density").unwrap();
        let a = Subject::parse("step.checks").unwrap();
        let own = Subject::parse("scatter.shrubs").unwrap();
        let d = skipped(&failed, [Some(&a), Some(&a), None, Some(&own)]).unwrap();
        assert_eq!(d.subject().to_string(), "scatter.shrubs");
        assert_eq!(d.data()["first"], Value::texts(["step.checks"]));
        assert!(skipped(&failed, [None, Some(&own)]).is_none());
    }
}
