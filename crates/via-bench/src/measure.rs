//! One battery, run over real and synthetic fabric by the same code.
//!
//! Ported from spikes/townfabric (07a95b8) with the ADR 0008 D2 character
//! replacements applied:
//!
//! - the invented Lorenz-style "backbone concentration" is replaced by
//!   Boeing's published `bc_gini` and `bc_max` (Gini coefficient and
//!   maximum of normalized node betweenness centrality; Boeing 2021,
//!   dataset indicator table quoted in research 0012 §5.1);
//! - the invented length-weighted mod-180° bearing entropy is replaced by
//!   Boeing's orientation entropy (Boeing 2019; osmnx convention: one
//!   bearing per street chain endpoint pair plus its reciprocal, 36 bins
//!   of 10° with bin 1 spanning [-5°, 5°), unweighted, natural log) and
//!   the derived orientation order φ = 1 − ((H − ln 4)/(ln 36 − ln 4))²;
//! - `circuity_avg`, `k_avg` and `self_loop_proportion` are added from the
//!   same indicator table;
//! - block elongation (minor/major axis of the minimum bounding
//!   rectangle) is reported by its momepy name; the raw axis medians stay
//!   as its components;
//! - storey statistics are computed over tagged buildings only, with the
//!   tagged share reported, so one untagged building no longer poisons the
//!   mean (a spike defect);
//! - `street_wall_share` (≤ 6 m) and `street_fronting_share` (≤ 25 m)
//!   remain **labelled inventions** under D2: no published character
//!   covers "the building line sits on the street"; they exist to catch
//!   fabric whose buildings ignore the street, and their thresholds are
//!   declared here, not sourced.
//!
//! Parcels exist only in synthetic fabric (OSM has no cadastre), so parcel
//! statistics are optional.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::Serialize;

use crate::geom::*;
use crate::graph::{Graph, Simple};
use crate::{Building, Parcel};

#[derive(Debug, Serialize)]
pub struct Fabric {
    pub label: String,
    // --- street topology (simplified graph, interior tally) ---
    pub nodes: usize,
    pub edges: usize,
    pub street_km: f64,
    /// (e − v + 1) / (2v − 5): 0 for a tree (Cardillo 2006; Buhl 2006).
    /// Bands are context, never targets (ADR 0008 D3).
    pub meshedness: f64,
    pub edge_node_ratio: f64,
    /// Average node degree 2e/v (Boeing 2021 `k_avg`).
    pub k_avg: f64,
    pub dead_end_share: f64,
    pub deg3_share: f64,
    pub deg4_share: f64,
    /// Interior chains whose two endpoints are the same junction.
    pub self_loop_proportion: f64,
    /// Shannon entropy of street bearings (Boeing 2019; nats).
    pub orientation_entropy: f64,
    /// φ = 1 − ((H − ln 4)/(ln 36 − ln 4))²: 1 for a perfect grid, → 0 as
    /// bearings approach uniformity. A spectrum, not a classifier
    /// (research 0012 §5.1).
    pub orientation_order: f64,
    /// Σ street length / Σ endpoint straight-line distance over interior
    /// chains, self-loops excluded (Boeing 2021 `circuity`).
    pub circuity_avg: f64,
    pub segment_len_median_m: f64,
    pub segment_len_p90_m: f64,
    /// Gini coefficient of normalized node betweenness centrality,
    /// computed on the whole buffered graph, tallied over interior nodes
    /// (Boeing 2021 `bc_gini`; replaces the spike's invented statistic per
    /// ADR 0008 D2).
    pub bc_gini: f64,
    pub bc_max: f64,
    // --- blocks ---
    pub blocks: usize,
    pub block_area_median_m2: f64,
    pub block_area_p90_m2: f64,
    /// Median corner count, counting vertices that turn by more than 25°.
    /// Added under ADR 0008 D10 when the eye caught wedge blocks the
    /// battery could not see.
    pub block_corners_median: f64,
    /// Median compactness 4πA/P²: 1.0 a circle, ~0.785 a square.
    pub block_compactness_median: f64,
    /// Median minor/major axis ratio of the minimum bounding rectangle
    /// (momepy `Elongation`, Fleischmann et al. 2022).
    pub block_elongation_median: f64,
    pub block_minor_axis_median_m: f64,
    pub block_major_axis_median_m: f64,
    // --- buildings ---
    pub buildings: usize,
    pub footprint_area_median_m2: f64,
    pub footprint_area_p90_m2: f64,
    /// Distance from a building's nearest corner to the nearest street
    /// centreline.
    pub bldg_street_dist_median_m: f64,
    /// Share of buildings within 6 m of a street centreline. Labelled
    /// invention (ADR 0008 D2): catches fabric whose buildings ignore the
    /// street; threshold declared, not sourced.
    pub street_wall_share: f64,
    /// Share of buildings within 25 m of a street. Labelled invention,
    /// same motivation: beyond this there is no plausible frontage.
    pub street_fronting_share: f64,
    /// Footprint area / block area.
    pub gsi: f64,
    /// Mean storeys over buildings that carry a storey tag, and the share
    /// that do. The spike's mean was NaN-poisoned by one untagged
    /// building; splitting the statistic removes the poisoning without
    /// hiding the coverage problem (0012 §9.1).
    pub storeys_mean_tagged: f64,
    pub storeys_tagged_share: f64,
    // --- parcels (synthetic only) ---
    pub parcels: Option<usize>,
    pub parcel_area_median_m2: Option<f64>,
    pub frontage_median_m: Option<f64>,
}

fn quantile(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() as f64 - 1.0) * q).round() as usize]
}

/// Gini coefficient over non-negative values. 0 = perfectly even,
/// → 1 as one node carries everything.
pub fn gini(vals: &[f64]) -> f64 {
    let mut v: Vec<f64> = vals.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len() as f64;
    let sum: f64 = v.iter().sum();
    if sum <= 0.0 {
        return 0.0;
    }
    let mut acc = 0.0;
    for (i, x) in v.iter().enumerate() {
        acc += (2.0 * (i as f64 + 1.0) - n - 1.0) * x;
    }
    acc / (n * sum)
}

/// Length-weighted node betweenness centrality (Brandes 2001) on the
/// simplified graph, normalized the way networkx does for undirected
/// graphs (× 2 / ((n−1)(n−2))) so the sidecar's cross-validation compares
/// like with like. Self-loop chains contribute no shortest paths and are
/// skipped.
fn node_betweenness(s: &Simple) -> Vec<f64> {
    let n = s.nodes.len();
    let mut bc = vec![0.0f64; n];
    if n < 3 {
        return bc;
    }
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for ch in &s.chains {
        if ch.a == ch.b {
            continue;
        }
        let w = ch.len.max(1.0e-6);
        adj[ch.a].push((ch.b, w));
        adj[ch.b].push((ch.a, w));
    }
    for src in 0..n {
        // Dijkstra with predecessor lists.
        let mut dist_v = vec![f64::INFINITY; n];
        let mut sigma = vec![0.0f64; n];
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::new();
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        dist_v[src] = 0.0;
        sigma[src] = 1.0;
        heap.push(Reverse((0u64, src)));
        let key = |x: f64| -> u64 { (x * 1024.0) as u64 };
        while let Some(Reverse((k, v))) = heap.pop() {
            if k > key(dist_v[v]) {
                continue;
            }
            order.push(v);
            for &(w, len) in &adj[v] {
                let nd = dist_v[v] + len;
                if nd < dist_v[w] - 1.0e-9 {
                    dist_v[w] = nd;
                    sigma[w] = sigma[v];
                    preds[w].clear();
                    preds[w].push(v);
                    heap.push(Reverse((key(nd), w)));
                } else if (nd - dist_v[w]).abs() <= 1.0e-9 && dist_v[w].is_finite() {
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
                bc[w] += delta[w];
            }
        }
    }
    // Each undirected pair was counted from both endpoints; halve, then
    // apply the networkx normalization for undirected graphs.
    let scale = 1.0 / ((n as f64 - 1.0) * (n as f64 - 2.0));
    for b in bc.iter_mut() {
        *b *= scale; // 0.5 * 2/((n-1)(n-2))
    }
    bc
}

/// Minimum distance from any vertex of `poly` to any street centreline.
fn dist_to_streets(g: &Graph, poly: &[P2], grid: &EdgeGrid) -> f64 {
    poly.iter()
        .map(|&p| grid.nearest_edge_dist(g, p))
        .fold(f64::INFINITY, f64::min)
}

/// Uniform grid over street edges, so building-to-street distance does not
/// cost O(buildings × edges). Ring expansion makes the answer exact, not
/// merely local.
pub struct EdgeGrid {
    cell: f64,
    minx: f64,
    miny: f64,
    nx: i64,
    ny: i64,
    bins: Vec<Vec<usize>>,
}

impl EdgeGrid {
    pub fn build(g: &Graph, cell: f64) -> Self {
        let (mut minx, mut miny) = (f64::INFINITY, f64::INFINITY);
        let (mut maxx, mut maxy) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &g.nodes {
            minx = minx.min(p[0]);
            miny = miny.min(p[1]);
            maxx = maxx.max(p[0]);
            maxy = maxy.max(p[1]);
        }
        if !minx.is_finite() {
            return Self {
                cell,
                minx: 0.0,
                miny: 0.0,
                nx: 1,
                ny: 1,
                bins: vec![Vec::new()],
            };
        }
        let nx = ((maxx - minx) / cell).ceil() as i64 + 1;
        let ny = ((maxy - miny) / cell).ceil() as i64 + 1;
        let mut bins = vec![Vec::new(); (nx * ny) as usize];
        for (ei, ed) in g.edges.iter().enumerate() {
            let (a, b) = (g.nodes[ed.a], g.nodes[ed.b]);
            let steps = (dist(a, b) / cell).ceil().max(1.0) as i64;
            for k in 0..=steps {
                let t = k as f64 / steps as f64;
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                let gx = ((p[0] - minx) / cell).floor() as i64;
                let gy = ((p[1] - miny) / cell).floor() as i64;
                if gx >= 0 && gy >= 0 && gx < nx && gy < ny {
                    let bin = &mut bins[(gy * nx + gx) as usize];
                    if !bin.contains(&ei) {
                        bin.push(ei);
                    }
                }
            }
        }
        Self {
            cell,
            minx,
            miny,
            nx,
            ny,
            bins,
        }
    }

    pub fn nearest_edge_dist(&self, g: &Graph, p: P2) -> f64 {
        let gx = ((p[0] - self.minx) / self.cell).floor() as i64;
        let gy = ((p[1] - self.miny) / self.cell).floor() as i64;
        let mut best = f64::INFINITY;
        let mut ring = 0i64;
        while ring <= self.nx.max(self.ny) {
            let mut any = false;
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if ring > 0 && dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }
                    let (cx, cy) = (gx + dx, gy + dy);
                    if cx < 0 || cy < 0 || cx >= self.nx || cy >= self.ny {
                        continue;
                    }
                    any = true;
                    for &ei in &self.bins[(cy * self.nx + cx) as usize] {
                        let ed = &g.edges[ei];
                        let (c, _) = closest_on_segment(p, g.nodes[ed.a], g.nodes[ed.b]);
                        best = best.min(dist(c, p));
                    }
                }
            }
            if best.is_finite() && best <= (ring as f64) * self.cell {
                break;
            }
            if !any && ring > 0 && best.is_finite() {
                break;
            }
            ring += 1;
        }
        best
    }
}

/// Compass bearing (0° = north, clockwise) of the vector a→b.
fn compass_bearing(a: P2, b: P2) -> f64 {
    let d = sub(b, a);
    d[0].atan2(d[1]).to_degrees().rem_euclid(360.0)
}

/// `study_radius` sets the disc the statistics describe (0012 P1). Fabric
/// outside it still participates in the graph (0012 P2) — it must, or
/// every clipped street becomes a false dead end and every route through
/// the edge of the map disappears.
pub fn measure(
    label: &str,
    g: &Graph,
    blocks: &[Vec<P2>],
    buildings: &[Building],
    parcels: Option<&[Parcel]>,
    study_radius: Option<f64>,
) -> Fabric {
    // Topology, street lengths, bearings and betweenness are all computed
    // on the simplified graph — junctions and whole streets — because that
    // is what every published figure refers to (0012 P3).
    let simple = g.simplify();
    let inside = |p: P2| -> bool { study_radius.map(|r| len(p) <= r).unwrap_or(true) };
    let node_in: Vec<bool> = simple.nodes.iter().map(|&p| inside(p)).collect();
    let chain_in: Vec<bool> = simple
        .chains
        .iter()
        .map(|c| node_in[c.a] && node_in[c.b])
        .collect();
    let v = node_in.iter().filter(|&&b| b).count();
    let e = chain_in.iter().filter(|&&b| b).count();
    let mut seg_len: Vec<f64> = simple
        .chains
        .iter()
        .zip(chain_in.iter())
        .filter(|&(_, &i)| i)
        .map(|(c, _)| c.len)
        .collect();
    let street_m: f64 = seg_len.iter().sum();
    let meshedness = if v > 2 {
        (e as f64 - v as f64 + 1.0) / (2.0 * v as f64 - 5.0)
    } else {
        f64::NAN
    };
    // Degrees come from the whole graph, counted only over interior nodes:
    // a street leaving the study area is not a dead end.
    let mut deg = [0usize; 12];
    for n in 0..simple.nodes.len() {
        if node_in[n] {
            deg[simple.degree(n).min(11)] += 1;
        }
    }
    let vf = v.max(1) as f64;

    // Orientation entropy, Boeing 2019 / osmnx convention: one bearing per
    // interior chain from endpoint to endpoint (geometry between the
    // endpoints is ignored, as osmnx does after simplification), plus its
    // reciprocal; 36 bins of 10° with bin 1 spanning [-5°, 5°); unweighted
    // counts; natural log.
    const BINS: usize = 36;
    let mut counts = [0u64; BINS];
    let mut self_loops = 0usize;
    let (mut circ_num, mut circ_den) = (0.0f64, 0.0f64);
    for (ci, ch) in simple.chains.iter().enumerate() {
        if !chain_in[ci] {
            continue;
        }
        if ch.a == ch.b {
            self_loops += 1;
            continue;
        }
        let (pa, pb) = (simple.nodes[ch.a], simple.nodes[ch.b]);
        let straight = dist(pa, pb);
        if straight < 1.0e-9 {
            continue;
        }
        circ_num += ch.len;
        circ_den += straight;
        let b = compass_bearing(pa, pb);
        for bearing in [b, (b + 180.0) % 360.0] {
            let bin = (((bearing + 5.0).rem_euclid(360.0)) / 10.0).floor() as usize % BINS;
            counts[bin] += 1;
        }
    }
    let tot: f64 = counts.iter().sum::<u64>() as f64;
    let entropy = if tot > 0.0 {
        -counts
            .iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / tot;
                p * p.ln()
            })
            .sum::<f64>()
    } else {
        f64::NAN
    };
    let (h_grid, h_max) = (4.0f64.ln(), (BINS as f64).ln());
    let orientation_order = 1.0 - ((entropy - h_grid) / (h_max - h_grid)).powi(2);

    // Betweenness on the whole buffered graph (0012 P2: routes through the
    // edge must exist), tallied over interior nodes.
    let bc = node_betweenness(&simple);
    let bc_interior: Vec<f64> = bc
        .iter()
        .zip(node_in.iter())
        .filter(|&(_, &i)| i)
        .map(|(&b, _)| b)
        .collect();
    let bc_gini = gini(&bc_interior);
    let bc_max = bc_interior.iter().copied().fold(f64::NAN, f64::max);

    let blocks: Vec<&Vec<P2>> = blocks
        .iter()
        .filter(|b| inside(polygon_centroid(b)))
        .collect();
    let buildings: Vec<&Building> = buildings
        .iter()
        .filter(|b| inside(polygon_centroid(&b.poly)))
        .collect();
    let mut block_areas: Vec<f64> = blocks.iter().map(|b| polygon_area(b).abs()).collect();
    let mut corners: Vec<f64> = Vec::with_capacity(blocks.len());
    let mut minor: Vec<f64> = Vec::with_capacity(blocks.len());
    let mut major: Vec<f64> = Vec::with_capacity(blocks.len());
    let mut elong: Vec<f64> = Vec::with_capacity(blocks.len());
    let mut compact: Vec<f64> = Vec::with_capacity(blocks.len());
    for b in &blocks {
        let n = b.len();
        let mut c = 0usize;
        for i in 0..n {
            let p = b[(i + n - 1) % n];
            let q = b[i];
            let r = b[(i + 1) % n];
            let (u, v) = (norm(sub(q, p)), norm(sub(r, q)));
            if len(u) < 1.0e-9 || len(v) < 1.0e-9 {
                continue;
            }
            let turn = dot(u, v).clamp(-1.0, 1.0).acos().to_degrees();
            if turn > 25.0 {
                c += 1;
            }
        }
        corners.push(c as f64);
        let (_, _, _, hu, hv) = obb(b);
        minor.push(2.0 * hv);
        major.push(2.0 * hu);
        if hu > 1.0e-9 {
            elong.push(hv / hu);
        }
        let a = polygon_area(b).abs();
        let per = polygon_perimeter(b);
        if per > 1.0e-6 {
            compact.push(4.0 * std::f64::consts::PI * a / (per * per));
        }
    }
    let block_total: f64 = block_areas.iter().sum();

    let grid = EdgeGrid::build(g, 40.0);
    let mut foot_areas: Vec<f64> = Vec::with_capacity(buildings.len());
    let mut sd: Vec<f64> = Vec::with_capacity(buildings.len());
    for b in &buildings {
        foot_areas.push(polygon_area(&b.poly).abs());
        sd.push(dist_to_streets(g, &b.poly, &grid));
    }
    let wall = sd.iter().filter(|&&d| d <= 6.0).count() as f64;
    let fronting = sd.iter().filter(|&&d| d <= 25.0).count() as f64;
    let nb = buildings.len().max(1) as f64;
    let foot_total: f64 = foot_areas.iter().sum();
    let tagged: Vec<f64> = buildings
        .iter()
        .map(|b| b.storeys)
        .filter(|s| s.is_finite())
        .collect();
    let storeys_mean_tagged = if tagged.is_empty() {
        f64::NAN
    } else {
        tagged.iter().sum::<f64>() / tagged.len() as f64
    };
    let storeys_tagged_share = if buildings.is_empty() {
        f64::NAN
    } else {
        tagged.len() as f64 / nb
    };
    let parcels: Option<Vec<&Parcel>> =
        parcels.map(|ps| ps.iter().filter(|p| inside(p.centroid)).collect());

    let (parcel_n, parcel_area, frontage) = match parcels.as_deref() {
        Some(p) if !p.is_empty() => {
            let mut a: Vec<f64> = p.iter().map(|x| x.area_m2).collect();
            let mut f: Vec<f64> = p.iter().map(|x| x.frontage_m).collect();
            (
                Some(p.len()),
                Some(quantile(&mut a, 0.5)),
                Some(quantile(&mut f, 0.5)),
            )
        }
        _ => (None, None, None),
    };

    Fabric {
        label: label.to_string(),
        nodes: v,
        edges: e,
        street_km: street_m / 1000.0,
        meshedness,
        edge_node_ratio: e as f64 / vf,
        k_avg: 2.0 * e as f64 / vf,
        dead_end_share: deg[1] as f64 / vf,
        deg3_share: deg[3] as f64 / vf,
        deg4_share: deg[4] as f64 / vf,
        self_loop_proportion: if e > 0 {
            self_loops as f64 / e as f64
        } else {
            f64::NAN
        },
        orientation_entropy: entropy,
        orientation_order,
        circuity_avg: if circ_den > 0.0 {
            circ_num / circ_den
        } else {
            f64::NAN
        },
        segment_len_median_m: quantile(&mut seg_len.clone(), 0.5),
        segment_len_p90_m: quantile(&mut seg_len, 0.9),
        bc_gini,
        bc_max,
        blocks: blocks.len(),
        block_area_median_m2: quantile(&mut block_areas.clone(), 0.5),
        block_area_p90_m2: quantile(&mut block_areas, 0.9),
        block_corners_median: quantile(&mut corners, 0.5),
        block_compactness_median: quantile(&mut compact, 0.5),
        block_elongation_median: quantile(&mut elong, 0.5),
        block_minor_axis_median_m: quantile(&mut minor, 0.5),
        block_major_axis_median_m: quantile(&mut major, 0.5),
        buildings: buildings.len(),
        footprint_area_median_m2: quantile(&mut foot_areas.clone(), 0.5),
        footprint_area_p90_m2: quantile(&mut foot_areas, 0.9),
        bldg_street_dist_median_m: quantile(&mut sd.clone(), 0.5),
        street_wall_share: wall / nb,
        street_fronting_share: fronting / nb,
        gsi: if block_total > 0.0 {
            foot_total / block_total
        } else {
            f64::NAN
        },
        storeys_mean_tagged,
        storeys_tagged_share,
        parcels: parcel_n,
        parcel_area_median_m2: parcel_area,
        frontage_median_m: frontage,
    }
}

/// The simplified graph as data, for the sidecar's formula-level checks:
/// with the identical graph in both implementations, a disagreement on a
/// character isolates the formula rather than the graph construction.
pub fn export_simple(g: &Graph, study_radius: Option<f64>) -> serde_json::Value {
    let simple = g.simplify();
    let inside = |p: P2| -> bool { study_radius.map(|r| len(p) <= r).unwrap_or(true) };
    serde_json::json!({
        "nodes": simple.nodes.iter().map(|p| serde_json::json!({
            "x": p[0], "y": p[1], "interior": inside(*p),
        })).collect::<Vec<_>>(),
        "chains": simple.chains.iter().map(|c| serde_json::json!({
            "a": c.a, "b": c.b, "len_m": c.len,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Class, Graph};

    fn lattice(n: usize, spacing: f64) -> Graph {
        let mut g = Graph::new(40.0);
        let extent = (n - 1) as f64 * spacing;
        let half = extent / 2.0;
        for k in 0..n {
            let c = k as f64 * spacing - half;
            g.insert_segment([-half, c], [half, c], Class::Street, 0.5);
            g.insert_segment([c, -half], [c, half], Class::Street, 0.5);
        }
        g
    }

    #[test]
    fn grid_topology_by_hand() {
        // 5x5 lattice, no study clip. The four outer corners are degree-2
        // and dissolve; v = 25 - 4 = 21. Chains: 40 lattice edges, minus
        // the 8 corner-adjacent pairs merged pairwise into 4 corner
        // chains: 40 - 8 + 4 = 36.
        let g = lattice(5, 100.0);
        let f = measure("grid", &g, &[], &[], None, None);
        assert_eq!(f.nodes, 21);
        assert_eq!(f.edges, 36);
        let expect_m = (36.0 - 21.0 + 1.0) / (2.0 * 21.0 - 5.0);
        assert!((f.meshedness - expect_m).abs() < 1e-12);
        assert!((f.k_avg - 2.0 * 36.0 / 21.0).abs() < 1e-12);
        assert!(f.dead_end_share.abs() < 1e-12);
        assert!((f.self_loop_proportion).abs() < 1e-12);
        // Note: the four dissolved corners leave L-shaped chains whose
        // endpoint bearings are diagonal, so this fixture is deliberately
        // NOT a perfect-grid entropy case; see interior_tally for that.
    }

    #[test]
    fn interior_tally_excludes_buffer() {
        // Same lattice, study radius 120: only the 5 nodes within 120 m of
        // the origin are interior (centre + 4 at distance 100); chains
        // count only with both endpoints interior — the 4 spokes.
        let g = lattice(5, 100.0);
        let f = measure("clip", &g, &[], &[], None, Some(120.0));
        assert_eq!(f.nodes, 5);
        assert_eq!(f.edges, 4);
        // Those interior nodes keep their full-graph degree 4: a street
        // leaving the study area is not a dead end.
        assert!(f.dead_end_share.abs() < 1e-12);
        assert!((f.deg4_share - 1.0).abs() < 1e-12);
        // The four interior chains are the axis-aligned spokes: a perfect
        // grid under the bearing convention — entropy ln 4, order 1,
        // circuity 1.
        assert!((f.orientation_entropy - 4.0f64.ln()).abs() < 1e-9);
        assert!((f.orientation_order - 1.0).abs() < 1e-9);
        assert!((f.circuity_avg - 1.0).abs() < 1e-12);
    }

    #[test]
    fn dead_end_detected() {
        let mut g = Graph::new(40.0);
        g.insert_segment([-100.0, 0.0], [100.0, 0.0], Class::Street, 0.5);
        g.insert_segment([0.0, 0.0], [0.0, 80.0], Class::Street, 0.5);
        // A T with one stub: nodes at (-100,0),(100,0),(0,0),(0,80);
        // degrees 1,1,3,1.
        let f = measure("tee", &g, &[], &[], None, None);
        assert_eq!(f.nodes, 4);
        assert_eq!(f.edges, 3);
        assert!((f.dead_end_share - 0.75).abs() < 1e-12);
        assert!((f.deg3_share - 0.25).abs() < 1e-12);
        // A tree has meshedness 0.
        assert!(f.meshedness.abs() < 1e-12);
    }

    #[test]
    fn betweenness_concentrates_on_a_bridge() {
        // Two 3x3 grids joined by one bridge chain: the bridge endpoints
        // carry the highest betweenness.
        let mut g = Graph::new(40.0);
        for k in 0..3 {
            let c = k as f64 * 100.0;
            // Left grid at x in [-400, -200].
            g.insert_segment([-400.0, c], [-200.0, c], Class::Street, 0.5);
            g.insert_segment([-400.0 + c, 0.0], [-400.0 + c, 200.0], Class::Street, 0.5);
            // Right grid at x in [200, 400].
            g.insert_segment([200.0, c], [400.0, c], Class::Street, 0.5);
            g.insert_segment([200.0 + c, 0.0], [200.0 + c, 200.0], Class::Street, 0.5);
        }
        g.insert_segment([-200.0, 100.0], [200.0, 100.0], Class::Street, 0.5);
        let simple = g.simplify();
        let bc = node_betweenness(&simple);
        // The two bridge endpoints are the nodes at (±200, 100).
        let mut ends: Vec<usize> = Vec::new();
        for (i, p) in simple.nodes.iter().enumerate() {
            if (p[1] - 100.0).abs() < 1.0 && (p[0].abs() - 200.0).abs() < 1.0 {
                ends.push(i);
            }
        }
        assert_eq!(ends.len(), 2);
        let max_other = bc
            .iter()
            .enumerate()
            .filter(|(i, _)| !ends.contains(i))
            .map(|(_, &b)| b)
            .fold(0.0f64, f64::max);
        for &e in &ends {
            assert!(bc[e] > max_other, "bridge endpoint must dominate");
        }
        // And the Gini over a bridged pair of grids exceeds the Gini of a
        // plain grid, where movement is spread evenly.
        let plain = lattice(5, 100.0);
        let f_bridge = measure("bridge", &g, &[], &[], None, None);
        let f_plain = measure("plain", &plain, &[], &[], None, None);
        assert!(f_bridge.bc_gini > f_plain.bc_gini);
    }

    #[test]
    fn storeys_over_tagged_only() {
        let mut g = Graph::new(40.0);
        g.insert_segment([-100.0, 0.0], [100.0, 0.0], Class::Street, 0.5);
        let sq = |cx: f64, cy: f64| -> Vec<P2> {
            vec![
                [cx - 5.0, cy - 5.0],
                [cx + 5.0, cy - 5.0],
                [cx + 5.0, cy + 5.0],
                [cx - 5.0, cy + 5.0],
            ]
        };
        let buildings = vec![
            Building {
                poly: sq(0.0, 10.0),
                storeys: 2.0,
                floor_area_m2: 100.0,
            },
            Building {
                poly: sq(20.0, 10.0),
                storeys: f64::NAN,
                floor_area_m2: 100.0,
            },
        ];
        let f = measure("st", &g, &[], &buildings, None, None);
        assert_eq!(f.buildings, 2);
        assert!((f.storeys_mean_tagged - 2.0).abs() < 1e-12);
        assert!((f.storeys_tagged_share - 0.5).abs() < 1e-12);
        // Both squares front the street: nearest corner is 5 m from the
        // centreline.
        assert!((f.bldg_street_dist_median_m - 5.0).abs() < 1e-9);
        assert!((f.street_wall_share - 1.0).abs() < 1e-12);
    }

    #[test]
    fn gini_extremes() {
        assert!(gini(&[1.0, 1.0, 1.0, 1.0]).abs() < 1e-12);
        // One node carries everything: G = (n-1)/n.
        let g = gini(&[0.0, 0.0, 0.0, 10.0]);
        assert!((g - 0.75).abs() < 1e-12);
    }

    #[test]
    fn measurement_is_deterministic() {
        let g = lattice(4, 80.0);
        let a = serde_json::to_string(&measure("d", &g, &[], &[], None, Some(150.0))).unwrap();
        let b = serde_json::to_string(&measure("d", &g, &[], &[], None, Some(150.0))).unwrap();
        assert_eq!(a, b);
    }
}
