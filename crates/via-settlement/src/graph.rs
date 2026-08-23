//! Components and all-pairs times over the spliced trunk graph.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::cost::{CostField, UNREACHABLE};

#[derive(Clone, Copy, PartialEq)]
struct Visit(f64, u32);
impl Eq for Visit {}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by time, ties by vertex id so the order is total
        // and the result is deterministic.
        other
            .0
            .total_cmp(&self.0)
            .then_with(|| other.1.cmp(&self.1))
    }
}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Weakly-connected components of the arc graph.
///
/// Derived from the arcs themselves, not from the trunk nodes'
/// recorded `component`: a junction-only vertex is not a trunk node
/// and has no recorded component, so trusting that field would drop
/// every spliced vertex into "unanchored".
pub fn components(cf: &CostField) -> Vec<i32> {
    let n = cf.len();
    let mut undirected: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (u, arcs) in cf.arcs.iter().enumerate() {
        for &(v, _) in arcs {
            undirected[u].push(v);
            undirected[v as usize].push(u as u32);
        }
    }
    let mut comp = vec![-1i32; n];
    let mut next = 0i32;
    let mut stack: Vec<u32> = Vec::new();
    for s in 0..n {
        if comp[s] != -1 {
            continue;
        }
        comp[s] = next;
        stack.push(s as u32);
        while let Some(u) = stack.pop() {
            for &v in &undirected[u as usize] {
                if comp[v as usize] == -1 {
                    comp[v as usize] = next;
                    stack.push(v);
                }
            }
        }
        next += 1;
    }
    comp
}

/// Least-time from `src` to every vertex, over the whole field.
/// Unreachable vertices hold [`UNREACHABLE`].
pub fn dijkstra(cf: &CostField, src: u32) -> Vec<f64> {
    let mut dist = vec![UNREACHABLE; cf.len()];
    let mut heap = BinaryHeap::new();
    dist[src as usize] = 0.0;
    heap.push(Visit(0.0, src));
    while let Some(Visit(d, u)) = heap.pop() {
        if d > dist[u as usize] {
            continue;
        }
        for &(v, w) in &cf.arcs[u as usize] {
            let nd = d + w;
            if nd < dist[v as usize] {
                dist[v as usize] = nd;
                heap.push(Visit(nd, v));
            }
        }
    }
    dist
}

/// Row-major `exp(−β c_ij)` over the vertices of one component, in the
/// order given. `c_ii` follows the Decision 4 intrazonal convention:
/// `gamma × min_{k≠i} c_ik`, falling back to zero for a vertex with no
/// neighbour (which can only be a singleton, excluded upstream).
pub fn kernel_for(cf: &CostField, verts: &[u32], beta: f64, gamma: f64) -> (Vec<f64>, Vec<f64>) {
    let n = verts.len();
    let mut k = vec![0.0f64; n * n];
    let mut local = vec![u32::MAX; cf.len()];
    for (li, &v) in verts.iter().enumerate() {
        local[v as usize] = li as u32;
    }
    let mut c_ii = vec![0.0f64; n];
    for (li, &v) in verts.iter().enumerate() {
        let dist = dijkstra(cf, v);
        let mut nearest = f64::INFINITY;
        for (lj, &u) in verts.iter().enumerate() {
            if li == lj {
                continue;
            }
            let d = dist[u as usize];
            if d < UNREACHABLE {
                k[li * n + lj] = (-beta * d).exp();
                if d < nearest {
                    nearest = d;
                }
            }
        }
        let self_cost = if nearest.is_finite() {
            gamma * nearest
        } else {
            0.0
        };
        c_ii[li] = self_cost;
        k[li * n + li] = (-beta * self_cost).exp();
    }
    (k, c_ii)
}
