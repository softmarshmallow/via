//! Null models (ADR 0008 D9; 0012 §8): run and reported beside any model,
//! on the same protocol, through the same battery. A character on which
//! the model and all three nulls are indistinguishable carries no
//! evidential weight and is reported as such.
//!
//! Three nulls, per the adopted specification:
//!
//! 1. a regular grid at matched intersection density;
//! 2. a random planar graph at matched node count and density;
//! 3. a diffusion-limited-aggregation growth — a tree grown by random
//!    attachment, standing in for the percolation/DLA family that
//!    reproduces urban statistics with no human mechanism (research
//!    0007's equifinality warning).
//!
//! Matching: with no generator yet, nulls are matched to the reference
//! class under comparison — the caller passes the target node count /
//! density taken from measured reference towns.
//!
//! Determinism: seeds derive from `via_artifact::seed::derive` per
//! CONTRIBUTING; the walk and scatter below consume their RNG in a fixed
//! order.

use via_artifact::seed;

use crate::geom::*;
use crate::graph::{Class, Graph};

/// SplitMix64 stream (the workspace's base primitive).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed_v: u64) -> Self {
        Self(seed_v)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        seed::splitmix64(self.0)
    }
    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Regular square grid: lines every `spacing_m`, spanning the disc of
/// radius `extent_m`. Intersection density is matched through the spacing.
pub fn grid(spacing_m: f64, extent_m: f64) -> Graph {
    let mut g = Graph::new(40.0);
    let k = (extent_m / spacing_m).floor() as i64;
    for i in -k..=k {
        let c = i as f64 * spacing_m;
        // Chord half-length of the disc at offset c.
        let h = (extent_m * extent_m - c * c).max(0.0).sqrt();
        if h < spacing_m * 0.5 {
            continue;
        }
        g.insert_segment([-h, c], [h, c], Class::Street, 0.5);
        g.insert_segment([c, -h], [c, h], Class::Street, 0.5);
    }
    g
}

/// Random planar graph: `n` nodes scattered uniformly in the disc, then
/// the shortest candidate edges (from each node's 6 nearest neighbours)
/// inserted — with planar splitting — until the edge/node ratio reaches
/// `target_edge_node_ratio` or candidates run out.
pub fn random_planar(n: usize, target_edge_node_ratio: f64, extent_m: f64, seed_v: u64) -> Graph {
    let mut rng = Rng::new(seed_v);
    let mut pts: Vec<P2> = Vec::with_capacity(n);
    while pts.len() < n {
        let x = (rng.next_f64() * 2.0 - 1.0) * extent_m;
        let y = (rng.next_f64() * 2.0 - 1.0) * extent_m;
        if x * x + y * y <= extent_m * extent_m {
            pts.push([x, y]);
        }
    }
    // Candidate edges: 6 nearest neighbours per node, deduplicated,
    // shortest first; ties broken on indices for determinism.
    let mut cands: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..n {
        let mut d: Vec<(f64, usize)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (dist(pts[i], pts[j]), j))
            .collect();
        d.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        for &(l, j) in d.iter().take(6) {
            let (a, b) = (i.min(j), i.max(j));
            cands.push((l, a, b));
        }
    }
    cands.sort_by(|x, y| {
        x.0.partial_cmp(&y.0)
            .unwrap()
            .then(x.1.cmp(&y.1))
            .then(x.2.cmp(&y.2))
    });
    cands.dedup_by(|a, b| a.1 == b.1 && a.2 == b.2);

    let mut g = Graph::new(40.0);
    for &(_, a, b) in &cands {
        if !g.nodes.is_empty() {
            let ratio = g.edges.len() as f64 / g.nodes.len() as f64;
            if ratio >= target_edge_node_ratio {
                break;
            }
        }
        g.insert_segment(pts[a], pts[b], Class::Street, 0.5);
    }
    g
}

/// DLA-style tree: particles walk in from the rim and attach to the
/// nearest network node when they come within `attach_m` of one. Produces
/// dead-end-heavy dendritic growth — the null that no meshedness should
/// survive.
pub fn dla(particles: usize, attach_m: f64, step_m: f64, extent_m: f64, seed_v: u64) -> Graph {
    let mut g = Graph::new(40.0);
    // Seed segment at the origin so the network is attachable.
    g.insert_segment([0.0, 0.0], [step_m.max(1.0), 0.0], Class::Street, 0.5);
    let mut rng = Rng::new(seed_v);
    for _ in 0..particles {
        let ang = rng.next_f64() * std::f64::consts::TAU;
        let mut p = [extent_m * ang.cos(), extent_m * ang.sin()];
        // Bounded walk; a particle that wanders too far is abandoned.
        for _ in 0..10_000 {
            if let Some(nearest) = g.find_node(p, attach_m) {
                let q = g.nodes[nearest];
                if dist(p, q) > 1.0 {
                    g.insert_segment(q, p, Class::Street, 0.5);
                }
                break;
            }
            let a = rng.next_f64() * std::f64::consts::TAU;
            p = [p[0] + step_m * a.cos(), p[1] + step_m * a.sin()];
            if len(p) > extent_m * 1.3 {
                // Re-enter from the rim.
                let a2 = rng.next_f64() * std::f64::consts::TAU;
                p = [extent_m * a2.cos(), extent_m * a2.sin()];
            }
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::measure;

    #[test]
    fn grid_is_meshed_dla_is_tree() {
        let g = grid(100.0, 435.0);
        let f = measure("grid", &g, &[], &[], None, Some(300.0));
        assert!(f.meshedness > 0.2, "grid meshedness {}", f.meshedness);
        assert!(f.dead_end_share < 0.05);

        let t = dla(
            150,
            30.0,
            20.0,
            435.0,
            seed::derive(42, "bench-null-dla", 0),
        );
        let ft = measure("dla", &t, &[], &[], None, Some(300.0));
        // A tree: meshedness ~0 (attachment can close the odd loop through
        // snapping, so allow a whisker), dead ends everywhere.
        assert!(ft.meshedness < 0.05, "dla meshedness {}", ft.meshedness);
        assert!(
            ft.dead_end_share > 0.3,
            "dla dead ends {}",
            ft.dead_end_share
        );
    }

    #[test]
    fn random_planar_hits_target_density() {
        let g = random_planar(300, 1.3, 435.0, seed::derive(42, "bench-null-random", 0));
        let ratio = g.edges.len() as f64 / g.nodes.len() as f64;
        assert!(ratio >= 1.25, "edge/node ratio {ratio}");
    }

    #[test]
    fn nulls_are_deterministic() {
        let s = seed::derive(42, "bench-null-random", 1);
        let a = random_planar(100, 1.2, 300.0, s);
        let b = random_planar(100, 1.2, 300.0, s);
        assert_eq!(a.nodes.len(), b.nodes.len());
        assert_eq!(a.edges.len(), b.edges.len());
        for (x, y) in a.nodes.iter().zip(b.nodes.iter()) {
            assert_eq!(x, y);
        }
    }
}
