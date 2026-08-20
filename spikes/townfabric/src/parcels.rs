//! Plots and buildings, on the mechanism the literature actually
//! describes.
//!
//! The first version subdivided each block by recursive bounding-box
//! halving and put a building on every resulting cell. Measured against
//! real towns that is wrong in a specific, quantified way (see
//! VALIDATION.md): it produced 864 buildings where Alnwick has 303, tiled
//! block interiors that are in reality gardens and yards, and made every
//! footprint the same size.
//!
//! What the sources describe instead:
//!
//! - **Plots run back from the street.** Conzen's (1960) plot pattern is a
//!   series of strips, each with a frontage on a street and a tail running
//!   into the block. Vanegas et al. (2012) put the same thing
//!   algorithmically: the split axis is chosen *against street frontage*,
//!   and the stop criterion is an era-dependent target width drawn from a
//!   seeded distribution. Binary halving around a module is what
//!   reproduces the half- and quarter-width structure Slater (1981)
//!   measured in burgage series.
//! - **Buildings fill the plot in phases** — Conzen's burgage cycle:
//!   *institutive* (a building on the frontage), *repletive* (outbuildings
//!   accreting down the tail), *climax* (near-maximal coverage), then
//!   recession and urban fallow. The driver is demand per unit frontage.
//!   So coverage is a per-plot state driven by demand, not a constant, and
//!   the rear of a plot is empty until demand pushes building into it.
//!
//! Consequences that fall out rather than being arranged: the number of
//! buildings is set by frontage rather than by block area; block interiors
//! stay open at low demand; and footprint sizes acquire a tail, because a
//! plot carries one frontage building and several smaller rear ones.

use serde::{Deserialize, Serialize};

use crate::geom::*;
use crate::grow::Rng;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ParcelConfig {
    /// Plot-width module (m) — the era's metrology. Widths are drawn as
    /// multiples of it (Slater's half/quarter structure).
    pub frontage_module_m: f64,
    /// How deep a plot runs back from its frontage (m).
    pub plot_depth_m: f64,
    /// Street reservation taken off the block before plots start (m):
    /// half the carriageway plus verge. Used only where the fronting
    /// street's class does not supply its own width.
    pub street_reserve_m: f64,
    /// Multiplier on the fronting street's own half-width. A town does not
    /// have one street width: lanes are narrow and main streets are wide,
    /// and that mix is what produces a partial street wall rather than
    /// all-or-nothing (measured: ~50% at Alnwick and Lavenham).
    pub reserve_from_class: f64,
    /// Blocks outside this area range are not town fabric.
    pub min_block_area_m2: f64,
    pub max_block_area_m2: f64,
    /// Setback of the frontage building from the plot's frontage line (m).
    pub front_setback_m: f64,
    /// Gap to each side boundary (m). 0 builds party walls — a continuous
    /// street wall, which is what pre-modern fabric has.
    pub side_setback_m: f64,
    /// Depth of the frontage building itself (m), and how much that varies.
    pub building_depth_m: f64,
    pub building_depth_var: f64,
    /// Coverage the burgage cycle drives toward at full demand, and at the
    /// edge of town. Coverage above `climax_coverage` is not reached.
    pub climax_coverage: f64,
    pub edge_coverage: f64,
    /// Rear accretions are at most this share of the frontage building's
    /// footprint, and stop when the plot tail runs out.
    pub rear_max_frac: f64,
    /// Storeys at the most and least accessible plots, and the era's cap.
    pub storeys_core: f64,
    pub storeys_edge: f64,
    pub storeys_cap: f64,
}

impl Default for ParcelConfig {
    fn default() -> Self {
        Self {
            frontage_module_m: 9.0,
            plot_depth_m: 40.0,
            street_reserve_m: 3.0,
            reserve_from_class: 1.0,
            min_block_area_m2: 300.0,
            max_block_area_m2: 200_000.0,
            front_setback_m: 0.0,
            side_setback_m: 0.0,
            building_depth_m: 10.0,
            building_depth_var: 0.35,
            climax_coverage: 0.6,
            edge_coverage: 0.25,
            rear_max_frac: 0.5,
            storeys_core: 3.0,
            storeys_edge: 1.5,
            storeys_cap: 5.0,
        }
    }
}

pub struct Parcel {
    pub poly: Vec<P2>,
    pub area_m2: f64,
    /// Width of the plot's street frontage (m).
    pub frontage_m: f64,
    pub centroid: P2,
    /// Distance from the plot's frontage line to the street centreline.
    pub street_dist_m: f64,
    pub block: usize,
    /// Built coverage the burgage cycle drove this plot to.
    pub coverage: f64,
}

pub struct Building {
    pub poly: Vec<P2>,
    pub storeys: f64,
    pub floor_area_m2: f64,
    pub parcel: usize,
}

/// Clip a convex rectangle to a convex plot: buildings stay on their own
/// land.
fn clip_to(rect: &[P2], plot: &[P2]) -> Vec<P2> {
    let mut out = rect.to_vec();
    let n = plot.len();
    for i in 0..n {
        let (a, b) = (plot[i], plot[(i + 1) % n]);
        if dist(a, b) < 1.0e-9 {
            continue;
        }
        let nrm = perp(norm(sub(b, a)));
        // Plot rings are counter-clockwise here, so the interior is inward.
        out = clip_halfplane(&out, a, mul(nrm, -1.0));
        if out.len() < 3 {
            return Vec::new();
        }
    }
    out
}

/// Inward normal of edge (a → b) for a counter-clockwise polygon.
#[inline]
fn inward(a: P2, b: P2) -> P2 {
    perp(norm(sub(b, a)))
}

/// Clip `poly` to the side of the mitre line at `corner` that belongs to
/// the edge whose midpoint is `keep`: where two frontage strips meet at a
/// block corner, each takes the half nearer its own street.
fn clip_mitre(poly: &[P2], corner: P2, n_prev: P2, n_next: P2, keep: P2) -> Vec<P2> {
    let m = norm(add(n_prev, n_next));
    if len(m) < 1.0e-9 {
        return poly.to_vec();
    }
    let mut nrm = perp(m);
    if dot(sub(keep, corner), nrm) > 0.0 {
        nrm = mul(nrm, -1.0);
    }
    clip_halfplane(poly, corner, nrm)
}

/// Inset a block by the street reservation. Half-plane clipping is a
/// convex operation, so on a concave block it is checked against the
/// original area and replaced by a centroid-ward scaling if it ate too
/// much.
fn inset_block(poly: &[P2], by: f64) -> Vec<P2> {
    let a0 = polygon_area(poly).abs();
    let mut out = poly.to_vec();
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if dist(a, b) < 1.0e-9 {
            continue;
        }
        let nrm = inward(a, b);
        out = clip_halfplane(&out, add(a, mul(nrm, by)), mul(nrm, -1.0));
        if out.len() < 3 {
            out.clear();
            break;
        }
    }
    if out.len() < 3 || polygon_area(&out).abs() < 0.45 * a0 {
        let c = polygon_centroid(poly);
        let r_eq = (a0 / std::f64::consts::PI).sqrt();
        let k = (1.0 - by / r_eq.max(1.0)).clamp(0.05, 1.0);
        return poly.iter().map(|&q| add(c, mul(sub(q, c), k))).collect();
    }
    out
}

/// Plot widths along one frontage: multiples of the era's module, drawn
/// deterministically. The 1/2 and 3/2 draws are what put half- and
/// three-half-width plots into the series, as Slater found.
fn plot_widths(span: f64, module: f64, rng: &mut Rng) -> Vec<f64> {
    let mut out = Vec::new();
    let mut left = span;
    while left > module * 0.6 {
        let m = match rng.next_u64() % 8 {
            0 => 0.5,
            1 => 1.5,
            2 => 2.0,
            _ => 1.0,
        };
        let w = (module * m).min(left);
        out.push(w);
        left -= w;
    }
    // Whatever is left joins the last plot rather than becoming a sliver.
    if left > 0.0 {
        if let Some(l) = out.last_mut() {
            *l += left;
        } else {
            out.push(span);
        }
    }
    out
}

/// Subdivide blocks into street-fronting plots, then run the burgage cycle
/// on each. `demand` returns the demand pressure at a point, in [0, 1].
pub fn subdivide(
    blocks: &[Vec<P2>],
    cfg: &ParcelConfig,
    seed: u64,
    demand: impl Fn(P2) -> f64,
    // reserve_at: half-width (m) of the street fronting a point on a block
    // boundary.
    reserve_at: impl Fn(P2) -> f64,
) -> (Vec<Parcel>, Vec<Building>) {
    let mut parcels: Vec<Parcel> = Vec::new();
    let mut buildings: Vec<Building> = Vec::new();
    let mut rng = Rng::new(seed);

    for (bi, block) in blocks.iter().enumerate() {
        let area = polygon_area(block).abs();
        if !(cfg.min_block_area_m2..=cfg.max_block_area_m2).contains(&area) {
            continue;
        }
        // Counter-clockwise, so inward normals point into the block.
        let mut ring = block.clone();
        if polygon_area(&ring) < 0.0 {
            ring.reverse();
        }
        let n = ring.len();
        // Each edge is a street centreline. Its frontage line is that edge
        // pushed inward by the street reservation — computed per edge, so
        // the distance from a frontage to its street is exact. (Insetting
        // the whole polygon first cannot promise that: on a concave block
        // the convex clip collapses and the fallback scaling gives a
        // distance that varies with position.)
        let inset_ok = polygon_area(&inset_block(&ring, cfg.street_reserve_m)).abs() > 1.0;
        if !inset_ok {
            continue;
        }

        for i in 0..n {
            let (e0, e1) = (ring[i], ring[(i + 1) % n]);
            let span = dist(e0, e1);
            if span < cfg.frontage_module_m * 0.6 {
                continue;
            }
            let d = norm(sub(e0, e1));
            let d = mul(d, -1.0);
            let nrm = inward(e0, e1);
            // Frontage line: this street's centreline pushed in by its own
            // half-width.
            let mid_edge = mul(add(e0, e1), 0.5);
            let reserve = (reserve_at(mid_edge) * cfg.reserve_from_class)
                .max(cfg.street_reserve_m);
            let a = add(e0, mul(nrm, reserve));
            let b = add(e1, mul(nrm, reserve));
            let depth = cfg.plot_depth_m;
            let strip = vec![a, b, add(b, mul(nrm, depth)), add(a, mul(nrm, depth))];
            // Trim where it would run past a neighbouring frontage, and
            // keep it inside the block minus the reserve on every side.
            let prev = ring[(i + n - 1) % n];
            let next = ring[(i + 2) % n];
            let mid = add(a, mul(d, span * 0.5));
            let strip = clip_mitre(&strip, a, inward(prev, e0), nrm, mid);
            if strip.len() < 3 {
                continue;
            }
            let strip = clip_mitre(&strip, b, nrm, inward(e1, next), mid);
            if strip.len() < 3 {
                continue;
            }
            let mut strip = strip;
            for k in 0..n {
                if k == i {
                    continue;
                }
                let (ka, kb) = (ring[k], ring[(k + 1) % n]);
                if dist(ka, kb) < 1.0e-9 {
                    continue;
                }
                let kn = inward(ka, kb);
                strip = clip_halfplane(&strip, add(ka, mul(kn, reserve)), mul(kn, -1.0));
                if strip.len() < 3 {
                    break;
                }
            }
            if strip.len() < 3 {
                continue;
            }

            // Cut the strip into plots across its frontage.
            let widths = plot_widths(span, cfg.frontage_module_m, &mut rng);
            let mut t = 0.0;
            for w in widths {
                let t0 = t;
                let t1 = (t + w).min(span);
                t = t1;
                if t1 - t0 < cfg.frontage_module_m * 0.4 {
                    continue;
                }
                let p0 = add(a, mul(d, t0));
                let p1 = add(a, mul(d, t1));
                let mut plot = clip_halfplane(&strip, p0, mul(d, -1.0));
                if plot.len() < 3 {
                    continue;
                }
                plot = clip_halfplane(&plot, p1, d);
                let plot_area = polygon_area(&plot).abs();
                if plot.len() < 3 || plot_area < 12.0 {
                    continue;
                }
                let frontage = t1 - t0;
                let centroid = polygon_centroid(&plot);

                // --- the burgage cycle on this plot ---
                let dem = demand(centroid).clamp(0.0, 1.0);
                let target = cfg.edge_coverage
                    + (cfg.climax_coverage - cfg.edge_coverage) * dem;
                let pi = parcels.len();
                let mut built = 0.0;

                // Institutive phase: a building on the frontage.
                let bw = frontage - 2.0 * cfg.side_setback_m;
                let bd = cfg.building_depth_m
                    * (1.0 + cfg.building_depth_var * (rng.f01() - 0.5) * 2.0);
                if bw > 1.5 && bd > 1.5 {
                    let front_mid = add(p0, mul(d, frontage * 0.5));
                    let c = add(
                        front_mid,
                        mul(nrm, cfg.front_setback_m + bd * 0.5),
                    );
                    let rect = vec![
                        add(c, add(mul(d, bw * 0.5), mul(nrm, bd * 0.5))),
                        add(c, add(mul(d, -bw * 0.5), mul(nrm, bd * 0.5))),
                        add(c, add(mul(d, -bw * 0.5), mul(nrm, -bd * 0.5))),
                        add(c, add(mul(d, bw * 0.5), mul(nrm, -bd * 0.5))),
                    ];
                    // A building may not spill over its own plot boundary.
                    let rect = clip_to(&rect, &plot);
                    let fa = polygon_area(&rect).abs();
                    if fa >= 4.0 {
                        built += fa;
                        buildings.push(Building {
                            poly: rect,
                            storeys: 1.0,
                            floor_area_m2: fa,
                            parcel: pi,
                        });
                    }
                }

                // Repletive phase: outbuildings accrete down the tail
                // while demand is not yet satisfied and tail remains.
                let mut back = cfg.front_setback_m + cfg.building_depth_m * 1.4;
                let tail_limit = cfg.plot_depth_m * 0.92;
                while built / plot_area < target && back + 3.0 < tail_limit {
                    let rw = (frontage - 2.0 * cfg.side_setback_m)
                        * (0.45 + 0.35 * rng.f01());
                    let rd = cfg.building_depth_m * cfg.rear_max_frac * (0.6 + 0.6 * rng.f01());
                    if rw < 1.5 || rd < 1.5 || back + rd > tail_limit {
                        break;
                    }
                    let c = add(add(p0, mul(d, frontage * 0.5)), mul(nrm, back + rd * 0.5));
                    let rect = vec![
                        add(c, add(mul(d, rw * 0.5), mul(nrm, rd * 0.5))),
                        add(c, add(mul(d, -rw * 0.5), mul(nrm, rd * 0.5))),
                        add(c, add(mul(d, -rw * 0.5), mul(nrm, -rd * 0.5))),
                        add(c, add(mul(d, rw * 0.5), mul(nrm, -rd * 0.5))),
                    ];
                    let rect = clip_to(&rect, &plot);
                    let fa = polygon_area(&rect).abs();
                    if fa >= 3.0 {
                        built += fa;
                        buildings.push(Building {
                            poly: rect,
                            storeys: 1.0,
                            floor_area_m2: fa,
                            parcel: pi,
                        });
                    }
                    back += rd + 2.5;
                }

                parcels.push(Parcel {
                    area_m2: plot_area,
                    frontage_m: frontage,
                    centroid,
                    street_dist_m: reserve,
                    poly: plot,
                    block: bi,
                    coverage: built / plot_area,
                });
            }
        }
    }

    // Storeys follow access, under the era's cap (Clark's gradient; the
    // monocentric model derives it). Step 3 replaces the distance proxy
    // with network centrality.
    for b in buildings.iter_mut() {
        let dem = demand(polygon_centroid(&b.poly)).clamp(0.0, 1.0);
        let s = (cfg.storeys_edge + (cfg.storeys_core - cfg.storeys_edge) * dem)
            .clamp(1.0, cfg.storeys_cap)
            .round();
        b.storeys = s;
        b.floor_area_m2 = polygon_area(&b.poly).abs() * s;
    }
    (parcels, buildings)
}
