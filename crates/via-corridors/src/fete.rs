//! FETE corridor density (White & Barber 2012, via adaptations per
//! ADR 0012 D4): least-cost paths between all directed pairs of a
//! uniform dry-land lattice, accumulated as per-cell traversal counts.
//! Accumulation per source is O(N) over the predecessor DAG (subtree
//! counts), never per-target path walks; sources run in parallel and
//! integer sums are order-independent, so the result is deterministic.

use rayon::prelude::*;

use crate::cost::MoveModel;
use crate::solve::{dijkstra, NO_PRED};

/// The lattice sources: dry-ground cells (no channel, no standing
/// water, land node present) at the configured spacing, centered.
pub fn lattice_sources(model: &MoveModel, spacing: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let off = spacing / 2;
    let mut y = off;
    while y < model.h {
        let mut x = off;
        while x < model.w {
            let i = (y * model.w + x) as usize;
            if model.land_node[i] && model.strahler[i] == 0 && model.water_depth[i] == 0.0 {
                out.push(i as u32);
            }
            x += spacing;
        }
        y += spacing;
    }
    out
}

/// Per-cell directed traversal counts (endpoints included), summed
/// over every ordered source pair.
pub fn density(model: &MoveModel, sources: &[u32]) -> Vec<u32> {
    let n_cells = model.n_cells();
    let source_set: Vec<bool> = {
        let mut v = vec![false; n_cells];
        for &s in sources {
            v[s as usize] = true;
        }
        v
    };
    sources
        .par_iter()
        .map(|&s| {
            let sol = dijkstra(model, &[(s, 0.0)], None, None);
            let n_nodes = model.n_nodes();
            // Subtree counts: seed 1 at each reachable target land node
            // (the other lattice sources), then push counts down the
            // predecessor chain in reverse settle order — children
            // before parents by construction, so no distance
            // comparison, and no assumption that a child's distance
            // strictly exceeds its parent's, is needed.
            let mut cnt = vec![0u32; n_nodes];
            for &v in &sol.settle_order {
                let vi = v as usize;
                let cell = model.cell_of(v) as usize;
                if !model.is_water_node(v) && source_set[cell] && v != s {
                    cnt[vi] += 1;
                }
            }
            let mut local = vec![0u32; n_cells];
            for &v in sol.settle_order.iter().rev() {
                let vi = v as usize;
                if cnt[vi] == 0 {
                    continue;
                }
                // A switch step visits both nodes of one cell; count
                // the cell once (the predecessor shares the cell).
                // Non-consecutive same-cell revisits cannot occur on
                // shortest paths: any indirect land↔water return to a
                // cell contains a switch plus positive extra cost, so
                // the direct same-cell switch strictly wins.
                let p = sol.pred[vi];
                if p == NO_PRED || model.cell_of(p) != model.cell_of(v) {
                    local[model.cell_of(v) as usize] += cnt[vi];
                }
                if p != NO_PRED {
                    cnt[p as usize] += cnt[vi];
                }
            }
            // The source cell itself carries every path it originates.
            // (Already counted via cnt at the source node, whose pred
            // is NO_PRED.)
            local
        })
        .reduce(
            || vec![0u32; n_cells],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(b) {
                    *x += y;
                }
                a
            },
        )
}
