//! The gateway trunk network (ADR 0012 D4): nodes are the terrain's
//! own gateways (passes, heads of navigation, river mouths), the pair
//! set is the Gabriel graph in symmetrized cost-distance space with a
//! Kruskal-style patch inside each reachable component, insertion is
//! ascending symmetrized cost (uniform-mass Molinero ordering), and
//! routing applies Stahlberg's multiplicative reuse discount to built
//! land edges only.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::cost::MoveModel;
use crate::solve::{dijkstra, NO_PRED};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteClass {
    Pass,
    HeadOfNavigation,
    RiverMouth,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrunkNode {
    pub class: SiteClass,
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    /// Graph anchor: the cell's land node if present, else its water
    /// node if present; sites with neither are unanchored and sit in
    /// component -1 (recorded, exempt from everything downstream).
    pub anchored: bool,
    /// Reachable-component id (-1 = unanchored or isolated).
    pub component: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepMode {
    Land,
    Water,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrunkEdge {
    pub a: u32,
    pub b: u32,
    /// Undiscounted traversal time along the stored path, a→b.
    pub hours_ab: f64,
    /// Undiscounted traversal time along the reversed path, b→a;
    /// None where a reverse step is inadmissible (e.g. upstream at
    /// net speed ≤ 0).
    pub hours_ba: Option<f64>,
    /// Land-leg energy in J/kg along the path, per direction; water
    /// legs contribute nothing (no citable water energy form — 0014 —
    /// the absence is flagged by `water_legs`). The reverse total is
    /// None where the reverse path is inadmissible.
    pub energy_land_legs_ab: f64,
    pub energy_land_legs_ba: Option<f64>,
    /// True when the path contains water or switch legs, whose energy
    /// is absent by documented limitation, not zero.
    pub water_legs: bool,
    pub length_m: f64,
    /// The path as (cell, mode) steps; a mode change at the same or
    /// adjacent cell implies a transshipment switch.
    pub path: Vec<(u32, StepMode)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Junction {
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    pub degree: u32,
    pub edges: Vec<u32>,
}

pub struct TrunkResult {
    pub nodes: Vec<TrunkNode>,
    pub edges: Vec<TrunkEdge>,
    pub junctions: Vec<Junction>,
    /// Per-cell count of trunk edges traversing it.
    pub raster: Vec<u32>,
}

/// River mouths: river cells whose receiver is an ocean (self-
/// receiver) cell — recomputable from receivers + strahler alone.
pub fn river_mouths(w: u32, h: u32, receivers: &[u32], strahler: &[u32]) -> Vec<u32> {
    let n = (w * h) as usize;
    let mut out = Vec::new();
    for i in 0..n {
        if strahler[i] > 0 && receivers[i] != i as u32 {
            let r = receivers[i] as usize;
            if receivers[r] == r as u32 {
                out.push(i as u32);
            }
        }
    }
    out
}

/// Assemble the declared node set in fixed order: passes, then heads
/// of navigation, then river mouths, each ascending by cell.
pub fn node_set(
    model: &MoveModel,
    pass_cells: &[u32],
    head_cells: &[u32],
    mouth_cells: &[u32],
) -> Vec<TrunkNode> {
    let mut nodes = Vec::new();
    let mut push = |class: SiteClass, cells: &[u32]| {
        let mut sorted: Vec<u32> = cells.to_vec();
        sorted.sort_unstable();
        for cell in sorted {
            let c = cell as usize;
            nodes.push(TrunkNode {
                class,
                cell,
                x: cell % model.w,
                y: cell / model.w,
                anchored: model.land_node[c] || model.water_node[c],
                component: -1,
            });
        }
    };
    push(SiteClass::Pass, pass_cells);
    push(SiteClass::HeadOfNavigation, head_cells);
    push(SiteClass::RiverMouth, mouth_cells);
    nodes
}

fn anchor(model: &MoveModel, node: &TrunkNode) -> Option<u32> {
    let c = node.cell as usize;
    if model.land_node[c] {
        Some(node.cell)
    } else if model.water_node[c] {
        Some(model.w * model.h + node.cell)
    } else {
        None
    }
}

struct Dsu(Vec<u32>);
impl Dsu {
    fn new(n: usize) -> Self {
        Dsu((0..n as u32).collect())
    }
    fn find(&mut self, i: u32) -> u32 {
        if self.0[i as usize] != i {
            let r = self.find(self.0[i as usize]);
            self.0[i as usize] = r;
            r
        } else {
            i
        }
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[rb as usize] = ra;
        }
    }
}

pub fn build(model: &MoveModel, mut nodes: Vec<TrunkNode>, reuse_alpha: f64) -> TrunkResult {
    let s = nodes.len();
    // Symmetrized cost-distance matrix from per-node full solves.
    let mut d = vec![f64::INFINITY; s * s];
    let mut sols = Vec::with_capacity(s);
    for node in &nodes {
        let sol = anchor(model, node).map(|a| dijkstra(model, &[(a, 0.0)], None, None));
        sols.push(sol);
    }
    for (i, sol) in sols.iter().enumerate() {
        if let Some(sol) = sol {
            for (j, nj) in nodes.iter().enumerate() {
                if let Some(aj) = anchor(model, nj) {
                    d[i * s + j] = sol.dist[aj as usize];
                }
            }
        }
    }
    let sym = |d: &[f64], i: usize, j: usize| {
        let (ab, ba) = (d[i * s + j], d[j * s + i]);
        if ab.is_finite() && ba.is_finite() {
            (ab + ba) / 2.0
        } else {
            f64::INFINITY
        }
    };

    // Reachable components (finite symmetrized distance).
    let mut dsu = Dsu::new(s);
    for i in 0..s {
        for j in (i + 1)..s {
            if sym(&d, i, j).is_finite() {
                dsu.union(i as u32, j as u32);
            }
        }
    }
    let mut comp_ids: BTreeMap<u32, i32> = BTreeMap::new();
    for (i, node) in nodes.iter_mut().enumerate() {
        if !node.anchored {
            continue;
        }
        let root = dsu.find(i as u32);
        let next = comp_ids.len() as i32;
        let id = *comp_ids.entry(root).or_insert(next);
        node.component = id;
    }

    // Gabriel graph in symmetrized cost space, per component.
    // Co-located sites (distance 0 — e.g. a head of navigation that
    // is also a mouth) get no edge: it would be a zero-length loop.
    let mut k: Vec<(usize, usize)> = Vec::new();
    for i in 0..s {
        for j in (i + 1)..s {
            let dij = sym(&d, i, j);
            if !dij.is_finite() || dij == 0.0 {
                continue;
            }
            let d2 = dij * dij;
            let mut keep = true;
            for r in 0..s {
                if r == i || r == j {
                    continue;
                }
                let (dir, djr) = (sym(&d, i, r), sym(&d, j, r));
                if dir.is_finite() && djr.is_finite() && dir * dir + djr * djr < d2 {
                    keep = false;
                    break;
                }
            }
            if keep {
                k.push((i, j));
            }
        }
    }
    // Kruskal patch inside each reachable component: while the Gabriel
    // prune leaves sub-components, add the globally cheapest
    // cross-sub-component pair (ties by node-index pair).
    let mut gg_dsu = Dsu::new(s);
    for &(i, j) in &k {
        gg_dsu.union(i as u32, j as u32);
    }
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..s {
        for j in (i + 1)..s {
            let dij = sym(&d, i, j);
            if dij.is_finite() && dij > 0.0 {
                candidates.push((dij, i, j));
            } else if dij == 0.0 {
                // Co-located sites share a component without an edge.
                gg_dsu.union(i as u32, j as u32);
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    for &(_, i, j) in &candidates {
        if gg_dsu.find(i as u32) != gg_dsu.find(j as u32) {
            gg_dsu.union(i as u32, j as u32);
            k.push((i, j));
        }
    }

    // Insertion order: ascending symmetrized cost, ties by index pair.
    k.sort_by(|&(ai, aj), &(bi, bj)| {
        sym(&d, ai, aj)
            .total_cmp(&sym(&d, bi, bj))
            .then(ai.cmp(&bi))
            .then(aj.cmp(&bj))
    });

    // Sequential routing with the multiplicative reuse discount on
    // built land edges (canonical undirected node pairs).
    let mut built: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut edges: Vec<TrunkEdge> = Vec::new();
    let n_cells = model.n_cells();
    let mut raster = vec![0u32; n_cells];
    for (i, j) in k {
        let (ai, aj) = match (anchor(model, &nodes[i]), anchor(model, &nodes[j])) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        let sol = {
            let mut disc = |u: u32, v: u32, hours: f64| {
                let land_edge = !model.is_water_node(u) && !model.is_water_node(v);
                if land_edge && built.contains(&(u.min(v), u.max(v))) {
                    hours * reuse_alpha
                } else {
                    hours
                }
            };
            dijkstra(model, &[(ai, 0.0)], Some(&mut disc), Some(aj))
        };
        if !sol.dist[aj as usize].is_finite() {
            continue;
        }
        // Retrace the node path a→b.
        let mut node_path = vec![aj];
        let mut cur = aj;
        while cur != ai {
            cur = sol.pred[cur as usize];
            debug_assert!(cur != NO_PRED);
            node_path.push(cur);
        }
        node_path.reverse();
        // Undiscounted costs, energies, length; register built edges.
        let mut hours_ab = 0.0;
        let mut hours_ba: Option<f64> = Some(0.0);
        let mut e_ab = 0.0;
        let mut e_ba = 0.0;
        let mut water_legs = false;
        let mut length_m = 0.0;
        for w2 in node_path.windows(2) {
            let (u, v) = (w2[0], w2[1]);
            hours_ab += model.move_cost(u, v).expect("forward step admissible");
            hours_ba = match (hours_ba, model.move_cost(v, u)) {
                (Some(acc), Some(c)) => Some(acc + c),
                _ => None,
            };
            let (cu, cv) = (model.cell_of(u), model.cell_of(v));
            let land_edge = !model.is_water_node(u) && !model.is_water_node(v);
            if cu != cv {
                let (x1, y1) = ((cu % model.w) as f64, (cu / model.w) as f64);
                let (x2, y2) = ((cv % model.w) as f64, (cv / model.w) as f64);
                length_m += model.dx * ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                if land_edge {
                    e_ab += model.step_energy(cu, cv);
                    e_ba += model.step_energy(cv, cu);
                    built.insert((u.min(v), u.max(v)));
                }
            }
            if !land_edge {
                water_legs = true;
            }
        }
        let path: Vec<(u32, StepMode)> = node_path
            .iter()
            .map(|&p| {
                (
                    model.cell_of(p),
                    if model.is_water_node(p) {
                        StepMode::Water
                    } else {
                        StepMode::Land
                    },
                )
            })
            .collect();
        // Per-edge cell set for the raster.
        let mut cells: Vec<u32> = path.iter().map(|&(c, _)| c).collect();
        cells.sort_unstable();
        cells.dedup();
        for &c in &cells {
            raster[c as usize] += 1;
        }
        edges.push(TrunkEdge {
            a: i as u32,
            b: j as u32,
            hours_ab,
            hours_ba,
            energy_land_legs_ab: e_ab,
            energy_land_legs_ba: if hours_ba.is_some() { Some(e_ba) } else { None },
            water_legs,
            length_m,
            path,
        });
    }

    let junctions = derive_junctions(model, &edges, &nodes);
    TrunkResult {
        nodes,
        edges,
        junctions,
        raster,
    }
}

/// Union graph of trunk paths (cells as vertices, consecutive distinct
/// cells as edges); a junction is a vertex of degree ≥ 3 or a trunk
/// endpoint cell.
fn derive_junctions(model: &MoveModel, edges: &[TrunkEdge], nodes: &[TrunkNode]) -> Vec<Junction> {
    let mut nbrs: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut incident: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (eid, e) in edges.iter().enumerate() {
        let mut prev: Option<u32> = None;
        for &(cell, _) in &e.path {
            incident.entry(cell).or_default().insert(eid as u32);
            if let Some(p) = prev {
                if p != cell {
                    nbrs.entry(p).or_default().insert(cell);
                    nbrs.entry(cell).or_default().insert(p);
                }
            }
            prev = Some(cell);
        }
    }
    let endpoint_cells: BTreeSet<u32> = edges
        .iter()
        .flat_map(|e| [nodes[e.a as usize].cell, nodes[e.b as usize].cell])
        .collect();
    let mut out = Vec::new();
    for (&cell, set) in &nbrs {
        let degree = set.len() as u32;
        if degree >= 3 || endpoint_cells.contains(&cell) {
            out.push(Junction {
                cell,
                x: cell % model.w,
                y: cell / model.w,
                degree,
                edges: incident[&cell].iter().copied().collect(),
            });
        }
    }
    out
}
