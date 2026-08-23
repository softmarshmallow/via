//! One battery, run over real and synthetic fabric by the same code.
//!
//! The first version of this spike measured three statistics, found one of
//! them inside a literature band, and called that partial validation. That
//! was wrong twice over: a single aggregate statistic is weakly diagnostic
//! (percolation and DLA models match urban statistics with no human
//! mechanism in them at all — docs/research/humanity/0007), and the
//! statistics I chose could not see the failures that mattered.
//!
//! So the battery below is chosen to be *discriminating*. Each entry names
//! the modelling failure it would catch:
//!
//! - `bldg_street_dist_*`, `street_fronting_share` — buildings that ignore
//!   the street, or parcels with no frontage at all.
//! - `betweenness_top_decile_share` — whether the fabric has a through
//!   route at all, or is an undifferentiated mesh with no high street.
//! - `footprint_area_*`, `segment_len_*`, `block_area_*` — whether the
//!   geometry has real variety or is a lattice of clones.
//! - `meshedness`, degree shares, `bearing_entropy_norm` — topology and
//!   alignment, the classical measures.
//!
//! Parcels exist only in synthetic fabric (OSM has no cadastre), so parcel
//! statistics are optional and are compared against the literature rather
//! than against the reference towns.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::Serialize;

use crate::geom::*;
use crate::graph::{Graph, Simple};
use crate::parcels::{Building, Parcel};

#[derive(Debug, Serialize)]
pub struct Fabric {
    pub label: String,
    // --- street topology ---
    pub nodes: usize,
    pub edges: usize,
    pub street_km: f64,
    /// (e − v + 1) / (2v − 5): 0 for a tree, ~0.15–0.26 organic, 0.26–0.35
    /// planned grids (Cardillo 2006; Buhl 2006).
    pub meshedness: f64,
    pub edge_node_ratio: f64,
    pub dead_end_share: f64,
    pub deg3_share: f64,
    pub deg4_share: f64,
    /// Normalized Shannon entropy of street bearings (mod 180°, 36 bins).
    pub bearing_entropy_norm: f64,
    pub segment_len_median_m: f64,
    pub segment_len_p90_m: f64,
    /// Share of all through-movement (length-weighted betweenness) carried
    /// by the busiest tenth of street length. 0.1 would mean movement is
    /// spread perfectly evenly — no hierarchy at all; real towns
    /// concentrate their movement on a few streets.
    pub backbone_concentration: f64,
    // --- blocks ---
    pub blocks: usize,
    pub block_area_median_m2: f64,
    pub block_area_p90_m2: f64,
    /// Median corner count of a block, counting only vertices that turn by
    /// more than 25°. Real street blocks are quadrilateral-ish; a growth
    /// model that connects nodes with straight chords produces triangles,
    /// and no statistic in the first battery could see the difference.
    pub block_corners_median: f64,
    /// Median compactness 4πA/P²: 1.0 is a circle, ~0.78 a square, and a
    /// long thin wedge tends to 0.
    pub block_compactness_median: f64,
    /// Median short side of a block's oriented bounding box. If towns grow
    /// by laying a street with a plot series along each side, this should
    /// cluster near twice the plot depth regardless of how long blocks are
    /// — a strong, falsifiable claim about how fabric is generated.
    pub block_minor_axis_median_m: f64,
    pub block_major_axis_median_m: f64,
    // --- buildings ---
    pub buildings: usize,
    pub footprint_area_median_m2: f64,
    pub footprint_area_p90_m2: f64,
    /// Distance from a building's nearest corner to the nearest street
    /// centreline. Medieval fabric sits on the street; suburban fabric
    /// stands well back.
    pub bldg_street_dist_median_m: f64,
    /// Share of buildings within 6 m of a street centreline — the street
    /// wall.
    pub street_wall_share: f64,
    /// Share of buildings within 25 m of a street: anything beyond that has
    /// no plausible frontage at all.
    pub street_fronting_share: f64,
    /// Footprint area / block area.
    pub gsi: f64,
    pub mean_storeys: f64,
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

/// Length-weighted edge betweenness (Brandes 2001) on the *simplified*
/// graph: on a subdivided polyline every geometry vertex would count as an
/// origin, which is not what betweenness means.
fn edge_betweenness(s: &Simple) -> Vec<f64> {
    let n = s.nodes.len();
    let mut eb = vec![0.0f64; s.chains.len()];
    if n < 3 {
        return eb;
    }
    let mut adj: Vec<Vec<(usize, usize, f64)>> = vec![Vec::new(); n];
    for (ei, ch) in s.chains.iter().enumerate() {
        let w = ch.len.max(1.0e-6);
        adj[ch.a].push((ch.b, ei, w));
        adj[ch.b].push((ch.a, ei, w));
    }
    for s in 0..n {
        // Dijkstra with predecessor lists.
        let mut dist_v = vec![f64::INFINITY; n];
        let mut sigma = vec![0.0f64; n];
        let mut preds: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
        let mut order: Vec<usize> = Vec::new();
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        dist_v[s] = 0.0;
        sigma[s] = 1.0;
        heap.push(Reverse((0u64, s)));
        let key = |x: f64| -> u64 { (x * 1024.0) as u64 };
        while let Some(Reverse((k, v))) = heap.pop() {
            if k > key(dist_v[v]) {
                continue;
            }
            order.push(v);
            for &(w, ei, len) in &adj[v] {
                let nd = dist_v[v] + len;
                if nd < dist_v[w] - 1.0e-9 {
                    dist_v[w] = nd;
                    sigma[w] = sigma[v];
                    preds[w].clear();
                    preds[w].push((v, ei));
                    heap.push(Reverse((key(nd), w)));
                } else if (nd - dist_v[w]).abs() <= 1.0e-9 {
                    sigma[w] += sigma[v];
                    preds[w].push((v, ei));
                }
            }
        }
        let mut delta = vec![0.0f64; n];
        for &w in order.iter().rev() {
            for &(v, ei) in &preds[w] {
                if sigma[w] > 0.0 {
                    let c = (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                    eb[ei] += c;
                    delta[v] += c;
                }
            }
        }
    }
    eb
}

/// Minimum distance from any vertex of `poly` to any street centreline.
fn dist_to_streets(g: &Graph, poly: &[P2], grid: &EdgeGrid) -> f64 {
    poly.iter()
        .map(|&p| grid.nearest_edge_dist(g, p))
        .fold(f64::INFINITY, f64::min)
}

/// Uniform grid over street edges, so building-to-street distance does not
/// cost O(buildings × edges).
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
        // Expand rings until the nearest hit cannot be beaten by a farther
        // ring, so the answer is exact, not merely local.
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

/// `study_radius` sets the area the statistics describe. Fabric outside it
/// still participates in the graph — it must, or every clipped street
/// becomes a false dead end and every route through the edge of the map
/// disappears (Ratti's edge-effect critique, recorded in
/// docs/research/humanity/0008 and then promptly ignored by the first
/// version of this spike).
pub fn measure(
    label: &str,
    g: &Graph,
    blocks: &[Vec<P2>],
    buildings: &[Building],
    parcels: Option<&[Parcel]>,
    study_radius: Option<f64>,
) -> Fabric {
    // Topology, street lengths and betweenness are all computed on the
    // simplified graph — junctions and whole streets — because that is what
    // every published figure refers to.
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

    // Bearings come from the underlying sub-segments, weighted by length:
    // a curving street is not one direction.
    const BINS: usize = 36;
    let mut hist = [0.0f64; BINS];
    for (ci, ch) in simple.chains.iter().enumerate() {
        if !chain_in[ci] {
            continue;
        }
        for pair in ch.pts.windows(2) {
            let d = sub(pair[1], pair[0]);
            let l = len(d);
            if l < 1.0e-9 {
                continue;
            }
            let a = d[1].atan2(d[0]).to_degrees().rem_euclid(180.0);
            let b = ((a / 180.0 * BINS as f64) as usize).min(BINS - 1);
            hist[b] += l;
        }
    }
    let tot: f64 = hist.iter().sum();
    let entropy = if tot > 0.0 {
        -hist
            .iter()
            .filter(|&&h| h > 0.0)
            .map(|&h| {
                let p = h / tot;
                p * p.ln()
            })
            .sum::<f64>()
    } else {
        f64::NAN
    };

    // Backbone concentration, Lorenz-style: rank interior streets by
    // through-movement, walk down until a tenth of the length is covered,
    // and report the share of all movement that tenth carries.
    let eb = edge_betweenness(&simple);
    let mut idx: Vec<usize> = (0..simple.chains.len()).filter(|&i| chain_in[i]).collect();
    idx.sort_by(|&a, &b| {
        (eb[b] / simple.chains[b].len.max(1.0))
            .partial_cmp(&(eb[a] / simple.chains[a].len.max(1.0)))
            .unwrap()
            .then(a.cmp(&b))
    });
    let total_bt: f64 = idx.iter().map(|&i| eb[i]).sum();
    let target = street_m * 0.1;
    let (mut acc_len, mut acc_bt) = (0.0, 0.0);
    for &i in &idx {
        if acc_len >= target {
            break;
        }
        acc_len += simple.chains[i].len;
        acc_bt += eb[i];
    }
    let bt_share = if total_bt > 0.0 {
        acc_bt / total_bt
    } else {
        f64::NAN
    };

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
    let storeys: f64 = buildings.iter().map(|b| b.storeys).sum();
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
        dead_end_share: deg[1] as f64 / vf,
        deg3_share: deg[3] as f64 / vf,
        deg4_share: deg[4] as f64 / vf,
        bearing_entropy_norm: entropy / (BINS as f64).ln(),
        segment_len_median_m: quantile(&mut seg_len.clone(), 0.5),
        segment_len_p90_m: quantile(&mut seg_len, 0.9),
        backbone_concentration: bt_share,
        blocks: blocks.len(),
        block_area_median_m2: quantile(&mut block_areas.clone(), 0.5),
        block_area_p90_m2: quantile(&mut block_areas, 0.9),
        block_corners_median: quantile(&mut corners, 0.5),
        block_compactness_median: quantile(&mut compact, 0.5),
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
        mean_storeys: if buildings.is_empty() {
            f64::NAN
        } else {
            storeys / nb
        },
        parcels: parcel_n,
        parcel_area_median_m2: parcel_area,
        frontage_median_m: frontage,
    }
}
