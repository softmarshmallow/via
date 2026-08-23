//! The settlement cost field `c_ij` (ADR 0013 Decision 4).
//!
//! Vertices are the distinct *cells* of the ADR 0012 trunk graph:
//! trunk nodes (passes, heads of navigation, river mouths) and the
//! junction cells spliced into edge paths. Arc weights are read from
//! the recorded directional times — `hours_ab` / `hours_ba` for a
//! whole edge, and the `cum_hours_*` arrays for the pieces either
//! side of a spliced junction. Nothing here re-derives the movement
//! model; that is the obligation Decision 2 imposes and the reason
//! ADR 0012 D5 was amended to carry cumulative times at all.
//!
//! Identity is by cell, not by record index: on the reference world
//! 1191 of 1465 junctions coincide with a trunk node, so keying by
//! record would double-count almost the whole junction set.

use std::collections::{BTreeMap, BTreeSet};

use via_corridors::trunk::{Junction, TrunkEdge, TrunkNode};

/// `f64::MAX` is reserved as "no admissible route", mirroring the
/// suitability/corridors sentinel discipline.
pub const UNREACHABLE: f64 = f64::MAX;

#[derive(Clone, Debug)]
pub struct CostField {
    /// Vertex id -> terrain cell, ascending by cell (deterministic).
    pub cells: Vec<u32>,
    /// Terrain cell -> vertex id.
    pub index_of_cell: BTreeMap<u32, u32>,
    /// Vertex id -> trunk-node record index, where the vertex is one.
    pub node_of_vertex: Vec<Option<u32>>,
    /// Reachable-component id per vertex; -1 where unanchored.
    pub component: Vec<i32>,
    /// Directed adjacency: `arcs[u] = [(v, hours), ...]`.
    pub arcs: Vec<Vec<(u32, f64)>>,
}

impl CostField {
    /// Build the spliced graph from the trunk records.
    pub fn build(nodes: &[TrunkNode], edges: &[TrunkEdge], junctions: &[Junction]) -> Self {
        // Vertex set: every trunk-node cell AND every junction cell.
        // The two overlap heavily — on the reference world 1191 of
        // 1465 junctions already sit on a trunk node — so the set is
        // keyed by cell and the union is taken once, deterministically
        // ascending because a BTreeSet is.
        let mut cellset: BTreeSet<u32> = BTreeSet::new();
        for n in nodes {
            cellset.insert(n.cell);
        }
        for j in junctions {
            cellset.insert(j.cell);
        }
        let cells: Vec<u32> = cellset.into_iter().collect();
        let mut index_of_cell: BTreeMap<u32, u32> = BTreeMap::new();
        for (i, &c) in cells.iter().enumerate() {
            index_of_cell.insert(c, i as u32);
        }

        let mut node_of_vertex = vec![None; cells.len()];
        let mut component = vec![-1i32; cells.len()];
        for (ni, n) in nodes.iter().enumerate() {
            let v = index_of_cell[&n.cell] as usize;
            // First writer wins, so a duplicated cell is deterministic.
            if node_of_vertex[v].is_none() {
                node_of_vertex[v] = Some(ni as u32);
                component[v] = n.component;
            }
        }

        let mut arcs: Vec<Vec<(u32, f64)>> = vec![Vec::new(); cells.len()];
        for e in edges {
            splice_edge(e, &index_of_cell, &mut arcs);
        }
        for a in arcs.iter_mut() {
            a.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
            a.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
        }
        Self {
            cells,
            index_of_cell,
            node_of_vertex,
            component,
            arcs,
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// Add the arcs an edge contributes, split at every vertex cell on its
/// path. Weights come from the cumulative arrays, so a split piece
/// costs exactly the recorded time of that stretch.
fn splice_edge(e: &TrunkEdge, index_of_cell: &BTreeMap<u32, u32>, arcs: &mut [Vec<(u32, f64)>]) {
    let mut marks: Vec<(usize, u32)> = Vec::new();
    for (k, &(cell, _)) in e.path.iter().enumerate() {
        if let Some(&v) = index_of_cell.get(&cell) {
            marks.push((k, v));
        }
    }
    for w in marks.windows(2) {
        let ((k1, v1), (k2, v2)) = (w[0], w[1]);
        if v1 == v2 {
            continue;
        }
        let fwd = e.cum_hours_ab[k2] - e.cum_hours_ab[k1];
        if fwd.is_finite() && fwd >= 0.0 {
            arcs[v1 as usize].push((v2, fwd));
        }
        if let Some(cba) = &e.cum_hours_ba {
            let rev = cba[k1] - cba[k2];
            if rev.is_finite() && rev >= 0.0 {
                arcs[v2 as usize].push((v1, rev));
            }
        }
    }
}
