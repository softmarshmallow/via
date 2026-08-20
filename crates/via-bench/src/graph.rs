//! A planar street graph that stays planar: every inserted segment is
//! snapped to nearby nodes and split against every edge it crosses, so
//! junctions are real junctions and the faces between streets are real
//! blocks.
//!
//! Ported from spikes/townfabric (07a95b8). The growth-rule callers are
//! gone; what remains is the structure both sides of a benchmark
//! comparison are built into, and the degree-2 simplification every
//! published street statistic assumes.

use std::collections::BTreeMap;

use crate::geom::*;

/// Coarse street class carried through import and rendering. The
/// benchmark's street *sets* (carriageway vs all-ways, 0012 P4) are decided
/// at import; this class only styles renders and never changes a number.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum Class {
    Corridor,
    Street,
    Lane,
}

pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub class: Class,
}

#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<P2>,
    pub edges: Vec<Edge>,
    /// node → incident edge indices.
    pub incident: Vec<Vec<usize>>,
    cell: f64,
    bins: BTreeMap<(i64, i64), Vec<usize>>,
}

impl Graph {
    pub fn new(bin_m: f64) -> Self {
        Self {
            cell: bin_m,
            ..Default::default()
        }
    }

    #[inline]
    fn key(&self, p: P2) -> (i64, i64) {
        (
            (p[0] / self.cell).floor() as i64,
            (p[1] / self.cell).floor() as i64,
        )
    }

    /// Existing node within `r`, preferring the closest; ties on index.
    pub fn find_node(&self, p: P2, r: f64) -> Option<usize> {
        let (kx, ky) = self.key(p);
        let mut best: Option<(f64, usize)> = None;
        let span = (r / self.cell).ceil() as i64;
        for dy in -span..=span {
            for dx in -span..=span {
                if let Some(v) = self.bins.get(&(kx + dx, ky + dy)) {
                    for &i in v {
                        let d = dist(p, self.nodes[i]);
                        if d <= r
                            && best
                                .map(|(bd, bi)| d < bd || (d == bd && i < bi))
                                .unwrap_or(true)
                        {
                            best = Some((d, i));
                        }
                    }
                }
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn add_node(&mut self, p: P2, snap_r: f64) -> usize {
        if let Some(i) = self.find_node(p, snap_r) {
            return i;
        }
        let i = self.nodes.len();
        self.nodes.push(p);
        self.incident.push(Vec::new());
        self.bins.entry(self.key(p)).or_default().push(i);
        i
    }

    fn add_edge_raw(&mut self, a: usize, b: usize, class: Class) {
        if a == b {
            return;
        }
        if self.incident[a].iter().any(|&e| {
            let ed = &self.edges[e];
            (ed.a == a && ed.b == b) || (ed.a == b && ed.b == a)
        }) {
            return;
        }
        let i = self.edges.len();
        self.edges.push(Edge { a, b, class });
        self.incident[a].push(i);
        self.incident[b].push(i);
    }

    /// Split edge `e` at point `p`, returning the node there.
    fn split_edge(&mut self, e: usize, p: P2, snap_r: f64) -> usize {
        let (a, b, class) = (self.edges[e].a, self.edges[e].b, self.edges[e].class);
        let n = self.add_node(p, snap_r);
        if n == a || n == b {
            return n;
        }
        // Rewrite the edge as a→n and add n→b.
        self.edges[e].b = n;
        self.incident[b].retain(|&x| x != e);
        self.incident[n].push(e);
        let i = self.edges.len();
        self.edges.push(Edge { a: n, b, class });
        self.incident[n].push(i);
        self.incident[b].push(i);
        let _ = a;
        n
    }

    /// Insert a segment, splitting it and every edge it crosses. Returns
    /// the node indices of the endpoints.
    ///
    /// Beyond the spike's proper-crossing splits, this handles the two
    /// T-touch cases within the snap radius: a segment *ending* on an
    /// edge's interior splits that edge, and a segment *passing over* an
    /// existing node routes through it. The spike left both as
    /// disconnected ghosts — invisible on OSM input, where junctions
    /// share nodes, but wrong against its own declared intent that
    /// junction counts be comparable, and immediately wrong for null
    /// models built from whole lattice lines.
    pub fn insert_segment(&mut self, p: P2, q: P2, class: Class, snap_r: f64) -> (usize, usize) {
        // Endpoint-on-edge: split the closest edge whose interior lies
        // within the snap radius of p or q, so the endpoint snap below
        // lands on a real junction node.
        for end in [p, q] {
            let mut best: Option<(f64, usize, P2)> = None;
            for e in 0..self.edges.len() {
                let (a, b) = (self.nodes[self.edges[e].a], self.nodes[self.edges[e].b]);
                let (c, t) = closest_on_segment(end, a, b);
                let d = dist(c, end);
                if d <= snap_r
                    && t > 1.0e-6
                    && t < 1.0 - 1.0e-6
                    && best.map(|(bd, _, _)| d < bd).unwrap_or(true)
                {
                    best = Some((d, e, c));
                }
            }
            if let Some((_, e, c)) = best {
                // Only split when no node is already close enough to snap
                // to — otherwise add_node would take that node anyway.
                if self.find_node(end, snap_r).is_none() {
                    self.split_edge(e, c, snap_r);
                }
            }
        }

        // Collect crossings against current edges, and existing nodes the
        // segment passes over, before any edit: splitting mutates the
        // edge list, so the scan must finish first. kind 0 = edge
        // crossing, kind 1 = node touch; sort is deterministic on
        // (parameter, kind, id).
        let mut hits: Vec<(f64, u8, usize, P2)> = Vec::new();
        for e in 0..self.edges.len() {
            let (a, b) = (self.nodes[self.edges[e].a], self.nodes[self.edges[e].b]);
            if let Some((x, t, _)) = segment_intersection(p, q, a, b) {
                hits.push((t, 0, e, x));
            }
        }
        for i in 0..self.nodes.len() {
            let (c, t) = closest_on_segment(self.nodes[i], p, q);
            if t > 1.0e-6 && t < 1.0 - 1.0e-6 && dist(c, self.nodes[i]) <= snap_r {
                hits.push((t, 1, i, self.nodes[i]));
            }
        }
        hits.sort_by(|l, r| {
            l.0.partial_cmp(&r.0)
                .unwrap()
                .then(l.1.cmp(&r.1))
                .then(l.2.cmp(&r.2))
        });

        let start = self.add_node(p, snap_r);
        let mut prev = start;
        let mut prev_p = self.nodes[start];
        for (_, kind, id, x) in hits {
            let n = if kind == 0 {
                // The edge may have been split already; re-find which edge
                // now carries this point.
                let e_now = self.edge_containing(x, id);
                self.split_edge(e_now, x, snap_r)
            } else {
                id
            };
            if n != prev && dist(prev_p, self.nodes[n]) > 1.0e-6 {
                self.add_edge_raw(prev, n, class);
                prev = n;
                prev_p = self.nodes[n];
            }
        }
        let end = self.add_node(q, snap_r);
        if end != prev {
            self.add_edge_raw(prev, end, class);
        }
        (start, end)
    }

    /// After earlier splits, find the edge that actually contains `x`,
    /// starting from the hint.
    fn edge_containing(&self, x: P2, hint: usize) -> usize {
        let on = |e: usize| -> f64 {
            let (a, b) = (self.nodes[self.edges[e].a], self.nodes[self.edges[e].b]);
            let (c, _) = closest_on_segment(x, a, b);
            dist(c, x)
        };
        if on(hint) < 1.0e-6 {
            return hint;
        }
        let mut best = (f64::INFINITY, hint);
        for e in 0..self.edges.len() {
            let d = on(e);
            if d < best.0 {
                best = (d, e);
            }
        }
        best.1
    }

    pub fn degree(&self, n: usize) -> usize {
        self.incident[n].len()
    }

    /// Bounded faces of the planar subdivision, as polygons wound
    /// counter-clockwise. Half-edge traversal: at each node the outgoing
    /// edges are ordered by bearing, and the successor of a half-edge is
    /// the neighbour clockwise from its reverse.
    pub fn faces(&self) -> Vec<Vec<P2>> {
        let mut sorted: Vec<Vec<(f64, usize, usize)>> = Vec::with_capacity(self.nodes.len());
        for n in 0..self.nodes.len() {
            let mut v: Vec<(f64, usize, usize)> = self.incident[n]
                .iter()
                .map(|&e| {
                    let other = if self.edges[e].a == n {
                        self.edges[e].b
                    } else {
                        self.edges[e].a
                    };
                    let d = sub(self.nodes[other], self.nodes[n]);
                    (d[1].atan2(d[0]), e, other)
                })
                .collect();
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
            sorted.push(v);
        }
        // Half-edges are (from, to) pairs keyed by edge index and direction.
        let mut visited = vec![[false; 2]; self.edges.len()];
        let mut faces = Vec::new();
        for e0 in 0..self.edges.len() {
            for dir in 0..2 {
                if visited[e0][dir] {
                    continue;
                }
                let mut poly = Vec::new();
                let (mut e, mut d) = (e0, dir);
                let mut guard = 0;
                loop {
                    if visited[e][d] {
                        break;
                    }
                    visited[e][d] = true;
                    let (from, to) = if d == 0 {
                        (self.edges[e].a, self.edges[e].b)
                    } else {
                        (self.edges[e].b, self.edges[e].a)
                    };
                    poly.push(self.nodes[from]);
                    // Next half-edge: at `to`, take the edge before the
                    // reverse of this one in bearing order.
                    let ring = &sorted[to];
                    let pos = ring.iter().position(|&(_, ee, _)| ee == e).unwrap();
                    let nxt = ring[(pos + ring.len() - 1) % ring.len()];
                    let (ne, nother) = (nxt.1, nxt.2);
                    let nd = if self.edges[ne].a == to && self.edges[ne].b == nother {
                        0
                    } else {
                        1
                    };
                    e = ne;
                    d = nd;
                    guard += 1;
                    if (e, d) == (e0, dir) || guard > 100_000 {
                        break;
                    }
                }
                if poly.len() >= 3 && polygon_area(&poly) > 0.0 {
                    faces.push(poly);
                }
            }
        }
        faces
    }
}

/// A street chain between two junctions, carrying its polyline geometry.
pub struct Chain {
    pub a: usize,
    pub b: usize,
    pub pts: Vec<P2>,
    pub len: f64,
}

/// The graph with degree-2 vertices dissolved: nodes are junctions and
/// dead ends, edges are whole streets between them.
///
/// This matters more than it sounds. A polyline drawn with many geometry
/// vertices — every OSM way, and every segment the insertion splits —
/// inflates the node count with degree-2 points, which drives meshedness
/// toward zero and makes betweenness meaningless. Every published figure
/// (Cardillo 2006; Boeing 2019) is computed on the simplified graph, so
/// any comparison must be too. Computing on the unsimplified graph was
/// failure #2 in ADR 0008's context list.
pub struct Simple {
    pub nodes: Vec<P2>,
    pub chains: Vec<Chain>,
    pub incident: Vec<Vec<usize>>,
}

impl Simple {
    pub fn degree(&self, n: usize) -> usize {
        self.incident[n].len()
    }
}

impl Graph {
    pub fn simplify(&self) -> Simple {
        let n = self.nodes.len();
        let is_junction: Vec<bool> = (0..n).map(|i| self.incident[i].len() != 2).collect();
        // Map original junction indices to compact ones, in index order.
        let mut map = vec![usize::MAX; n];
        let mut nodes = Vec::new();
        for i in 0..n {
            if is_junction[i] {
                map[i] = nodes.len();
                nodes.push(self.nodes[i]);
            }
        }
        let mut incident: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut chains: Vec<Chain> = Vec::new();
        let mut used = vec![false; self.edges.len()];

        let other = |e: usize, from: usize| -> usize {
            if self.edges[e].a == from {
                self.edges[e].b
            } else {
                self.edges[e].a
            }
        };

        for start in 0..n {
            if !is_junction[start] {
                continue;
            }
            for &e0 in &self.incident[start] {
                if used[e0] {
                    continue;
                }
                let mut pts = vec![self.nodes[start]];
                let mut e = e0;
                let mut cur = start;
                let mut len = 0.0;
                loop {
                    used[e] = true;
                    let nxt = other(e, cur);
                    len += dist(self.nodes[cur], self.nodes[nxt]);
                    pts.push(self.nodes[nxt]);
                    if is_junction[nxt] {
                        cur = nxt;
                        break;
                    }
                    // Degree-2: continue through it.
                    let Some(&ne) = self.incident[nxt].iter().find(|&&x| x != e) else {
                        cur = nxt;
                        break;
                    };
                    if used[ne] {
                        cur = nxt;
                        break;
                    }
                    e = ne;
                    cur = nxt;
                }
                if !is_junction[cur] {
                    continue; // dangling chain of degree-2 nodes; no endpoint
                }
                let (ca, cb) = (map[start], map[cur]);
                if ca == usize::MAX || cb == usize::MAX {
                    continue;
                }
                let ci = chains.len();
                incident[ca].push(ci);
                if cb != ca {
                    incident[cb].push(ci);
                }
                chains.push(Chain {
                    a: ca,
                    b: cb,
                    pts,
                    len,
                });
            }
        }
        Simple {
            nodes,
            chains,
            incident,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_segments_split_into_planar_junction() {
        let mut g = Graph::new(40.0);
        g.insert_segment([-10.0, 0.0], [10.0, 0.0], Class::Street, 0.5);
        g.insert_segment([0.0, -10.0], [0.0, 10.0], Class::Street, 0.5);
        // The crossing becomes a node of degree 4; each street splits in two.
        assert_eq!(g.nodes.len(), 5);
        assert_eq!(g.edges.len(), 4);
        let centre = g.find_node([0.0, 0.0], 0.1).expect("crossing node");
        assert_eq!(g.degree(centre), 4);
    }

    #[test]
    fn snap_merges_nearby_endpoints() {
        let mut g = Graph::new(40.0);
        g.insert_segment([0.0, 0.0], [10.0, 0.0], Class::Street, 2.0);
        // Endpoint within the snap radius reuses the node.
        g.insert_segment([10.5, 0.5], [10.0, 10.0], Class::Street, 2.0);
        assert_eq!(g.nodes.len(), 3);
    }

    #[test]
    fn simplify_dissolves_degree_two_vertices() {
        let mut g = Graph::new(40.0);
        // A polyline a-b-c-d: b and c are geometry vertices, not junctions.
        g.insert_segment([0.0, 0.0], [10.0, 0.0], Class::Street, 0.5);
        g.insert_segment([10.0, 0.0], [20.0, 5.0], Class::Street, 0.5);
        g.insert_segment([20.0, 5.0], [30.0, 5.0], Class::Street, 0.5);
        let s = g.simplify();
        assert_eq!(s.nodes.len(), 2, "only the two endpoints survive");
        assert_eq!(s.chains.len(), 1);
        let expected = 10.0 + (100.0f64 + 25.0).sqrt() + 10.0;
        assert!((s.chains[0].len - expected).abs() < 1e-9);
        assert_eq!(s.chains[0].pts.len(), 4);
    }

    #[test]
    fn unit_grid_faces() {
        let mut g = Graph::new(40.0);
        // 3x3 lattice of 100 m spacing: 4 bounded faces.
        for k in 0..3 {
            let y = k as f64 * 100.0;
            g.insert_segment([0.0, y], [200.0, y], Class::Street, 0.5);
            g.insert_segment([y, 0.0], [y, 200.0], Class::Street, 0.5);
        }
        let faces: Vec<_> = g
            .faces()
            .into_iter()
            .filter(|f| {
                let a = polygon_area(f).abs();
                (300.0..=90_000.0).contains(&a)
            })
            .collect();
        assert_eq!(faces.len(), 4);
        for f in &faces {
            assert!((polygon_area(f).abs() - 10_000.0).abs() < 1.0);
        }
    }
}
