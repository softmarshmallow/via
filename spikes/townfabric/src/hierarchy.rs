//! Streets earn their class.
//!
//! Until now a street's class — and therefore its width — was decided when
//! it was inserted: corridors were corridors because they were drawn
//! first, everything grown was a lane. Real fabric works the other way
//! round: a street is a main street because movement uses it, and it is
//! wide because it is a main street. Measured against real towns, having
//! one effective width put every building 2.5 m from a centreline where
//! Alnwick and Lavenham put half of theirs beyond 6 m.
//!
//! The instrument is **angular segment choice** (Hillier & Iida 2005):
//! betweenness over the segment graph where the cost of a route is the
//! total *turning* it requires, not its length. People navigate by
//! straightness, so the least-angular path predicts movement better than
//! the shortest one.
//!
//! Only choice is used. docs/research/humanity/0008 records why:
//! the 2018 meta-analysis puts the pooled correlation for choice at
//! ~0.48 but integration at 0.206, so integration is not something to
//! build on.
//!
//! What class boundaries to draw is era forcing — a town declares how
//! much of its network is main street — and the *placement* of those
//! streets is what emerges.

use serde::{Deserialize, Serialize};

use crate::geom::*;
use crate::graph::{Class, Graph, Simple};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HierarchyConfig {
    /// Share of street length that carries the town's main streets, and
    /// the share below that which is ordinary street; the rest are lanes.
    pub main_share: f64,
    pub street_share: f64,
    /// Carriageway-plus-verge width (m) of each class.
    pub width_main_m: f64,
    pub width_street_m: f64,
    pub width_lane_m: f64,
}

impl Default for HierarchyConfig {
    fn default() -> Self {
        Self {
            main_share: 0.12,
            street_share: 0.3,
            width_main_m: 16.0,
            width_street_m: 8.0,
            width_lane_m: 4.0,
        }
    }
}

impl HierarchyConfig {
    pub fn width_of(&self, c: Class) -> f64 {
        match c {
            Class::Corridor => self.width_main_m,
            Class::Street => self.width_street_m,
            Class::Lane => self.width_lane_m,
        }
    }
}

/// Angular choice per chain: betweenness on the segment graph with turn
/// cost as the metric. Brandes (2001) over a graph whose nodes are
/// streets and whose edges are the turns between them.
pub fn angular_choice(s: &Simple) -> Vec<f64> {
    let n = s.chains.len();
    let mut cb = vec![0.0f64; n];
    if n < 3 {
        return cb;
    }

    // Direction of a chain as seen leaving junction `j`.
    let heading = |ci: usize, j: usize| -> P2 {
        let ch = &s.chains[ci];
        if ch.a == j {
            norm(sub(ch.pts[1.min(ch.pts.len() - 1)], ch.pts[0]))
        } else {
            let l = ch.pts.len();
            norm(sub(ch.pts[l.saturating_sub(2)], ch.pts[l - 1]))
        }
    };

    // Segment graph: chains adjacent at a shared junction, weighted by the
    // turn between them (0 = straight on, 1 = a right-angle turn, 2 = a
    // reversal).
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for j in 0..s.nodes.len() {
        let inc = &s.incident[j];
        for (x, &ci) in inc.iter().enumerate() {
            for &cj in inc.iter().skip(x + 1) {
                if ci == cj {
                    continue;
                }
                let hi = heading(ci, j);
                let hj = heading(cj, j);
                // Continuing straight through the junction means the
                // outgoing heading reverses the incoming one.
                let cos = dot(mul(hi, -1.0), hj).clamp(-1.0, 1.0);
                let turn = (1.0 - cos) * 0.5 * 2.0;
                adj[ci].push((cj, turn));
                adj[cj].push((ci, turn));
            }
        }
    }

    for src in 0..n {
        let mut dist = vec![f64::INFINITY; n];
        let mut sigma = vec![0.0f64; n];
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::new();
        let mut heap = std::collections::BinaryHeap::new();
        dist[src] = 0.0;
        sigma[src] = 1.0;
        let key = |x: f64| -> u64 { (x * 4096.0) as u64 };
        heap.push(std::cmp::Reverse((0u64, src)));
        while let Some(std::cmp::Reverse((k, v))) = heap.pop() {
            if k > key(dist[v]) {
                continue;
            }
            order.push(v);
            for &(w, t) in &adj[v] {
                let nd = dist[v] + t;
                if nd < dist[w] - 1.0e-9 {
                    dist[w] = nd;
                    sigma[w] = sigma[v];
                    preds[w].clear();
                    preds[w].push(v);
                    heap.push(std::cmp::Reverse((key(nd), w)));
                } else if (nd - dist[w]).abs() <= 1.0e-9 {
                    sigma[w] += sigma[v];
                    preds[w].push(v);
                }
            }
        }
        let mut delta = vec![0.0f64; n];
        for &w in order.iter().rev() {
            for &v in &preds[w] {
                if sigma[w] > 0.0 {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != src {
                cb[w] += delta[w];
            }
        }
    }
    cb
}

/// Assign classes by choice and write them back onto the graph's edges.
/// Returns the per-chain choice, normalized to [0, 1], and the choice
/// field sampled per underlying edge.
pub fn classify(g: &mut Graph, cfg: &HierarchyConfig) -> Vec<f64> {
    let simple = g.simplify();
    let choice = angular_choice(&simple);
    let mut edge_choice = vec![0.0f64; g.edges.len()];
    if simple.chains.is_empty() {
        return edge_choice;
    }
    let max_c = choice.iter().cloned().fold(0.0f64, f64::max).max(1.0);

    // Rank streets by choice, then walk down assigning classes until each
    // class has taken its declared share of the town's street length.
    let total_len: f64 = simple.chains.iter().map(|c| c.len).sum();
    let mut idx: Vec<usize> = (0..simple.chains.len()).collect();
    idx.sort_by(|&a, &b| {
        choice[b]
            .partial_cmp(&choice[a])
            .unwrap()
            .then(a.cmp(&b))
    });
    let mut acc = 0.0;
    for &ci in &idx {
        let frac = acc / total_len.max(1.0e-9);
        let class = if frac < cfg.main_share {
            Class::Corridor
        } else if frac < cfg.main_share + cfg.street_share {
            Class::Street
        } else {
            Class::Lane
        };
        for &e in &simple.chains[ci].edges {
            g.set_class(e, class);
            edge_choice[e] = choice[ci] / max_c;
        }
        acc += simple.chains[ci].len;
    }
    edge_choice
}
