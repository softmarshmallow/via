//! Deterministic Dijkstra over the multimodal node-split graph.
//! Heap ordering is (cost, node index); relaxation is strict-less-than
//! only, so distance and predecessor arrays are bit-reproducible
//! regardless of thread scheduling around them (ADR 0012 D4/D6).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::cost::MoveModel;

pub const NO_PRED: u32 = u32::MAX;

pub struct Solution {
    pub dist: Vec<f64>,
    pub pred: Vec<u32>,
    /// Nodes in the order they were settled. A node's predecessor is
    /// always settled strictly earlier (predecessors are assigned only
    /// while expanding an already-settled node), so the reverse of this
    /// order is a children-before-parents traversal of the predecessor
    /// tree — valid even where rounding makes a child's distance equal
    /// its parent's, which a distance sort would get wrong.
    pub settle_order: Vec<u32>,
}

/// Total-order key for f64 costs (all costs are finite and >= 0).
#[derive(PartialEq)]
struct Key(f64);
impl Eq for Key {}
impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Multi-source Dijkstra. `sources` are (node, initial cost) pairs.
/// `discount` optionally rescales a move's cost (the trunk reuse
/// discount); it must be pure and deterministic.
pub fn dijkstra(
    model: &MoveModel,
    sources: &[(u32, f64)],
    mut discount: Option<&mut dyn FnMut(u32, u32, f64) -> f64>,
    early_exit: Option<u32>,
) -> Solution {
    let n = model.n_nodes();
    let mut dist = vec![f64::INFINITY; n];
    let mut pred = vec![NO_PRED; n];
    let mut settled = vec![false; n];
    let mut settle_order: Vec<u32> = Vec::new();
    let mut heap: BinaryHeap<Reverse<(Key, u32)>> = BinaryHeap::new();
    for &(s, d0) in sources {
        if d0 < dist[s as usize] {
            dist[s as usize] = d0;
            heap.push(Reverse((Key(d0), s)));
        }
    }
    while let Some(Reverse((Key(d), u))) = heap.pop() {
        let ui = u as usize;
        if settled[ui] || d > dist[ui] {
            continue;
        }
        settled[ui] = true;
        settle_order.push(u);
        if early_exit == Some(u) {
            break;
        }
        model.for_moves(u, |m| {
            let cost = match discount.as_mut() {
                Some(f) => f(u, m.to, m.hours),
                None => m.hours,
            };
            let nd = d + cost;
            let vi = m.to as usize;
            if nd < dist[vi] {
                dist[vi] = nd;
                pred[vi] = u;
                heap.push(Reverse((Key(nd), m.to)));
            }
        });
    }
    Solution {
        dist,
        pred,
        settle_order,
    }
}
