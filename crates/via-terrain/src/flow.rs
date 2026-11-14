//! Flow routing: priority-flood depression handling (Barnes et al. 2014),
//! D8 receivers, the Braun–Willett stack, and drainage accumulation.
//!
//! Everything order-sensitive in here is sequential by design; determinism
//! comes from fixed iteration order and explicit tie-breaking, not luck.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use via_num as num;

use rayon::prelude::*;

use crate::grid::Grid;

/// Monotone map from finite f64 to u64 preserving total order; lets elevations
/// serve as heap keys without float-Ord unpleasantness.
#[inline]
fn f64_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 {
        !b
    } else {
        b ^ (1u64 << 63)
    }
}

/// Priority-flood + ε (Barnes et al. 2014, alg. 3): raises depression floors
/// so every non-seed cell gains a strictly descending path (≥ ε per step) to
/// some seed. Mutates `h` in place. Heap ties break on cell index.
pub fn priority_flood_eps(grid: &Grid, h: &mut [f64], is_seed: &[bool], eps: f64) {
    let n = grid.n();
    let mut visited = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    for i in 0..n {
        if is_seed[i] {
            visited[i] = true;
            heap.push(Reverse((f64_key(h[i]), i as u32)));
        }
    }
    assert!(
        !heap.is_empty(),
        "priority_flood_eps: no seed cells (the border ring must be base level)"
    );
    while let Some(Reverse((_, c))) = heap.pop() {
        let hc = h[c as usize];
        grid.for_neighbors(c, |nb, _| {
            let nbu = nb as usize;
            if !visited[nbu] {
                visited[nbu] = true;
                let floor = hc + eps;
                if h[nbu] < floor {
                    h[nbu] = floor;
                }
                heap.push(Reverse((f64_key(h[nbu]), nb)));
            }
        });
    }
}

/// Fold shallow depressions back into the true surface (ADR 0004).
///
/// After priority-flood builds the routing surface, every connected ponded
/// component whose maximum depth is below `min_lake_depth_m` is treated as
/// sub-grid noise — transient pits recut each step by erosion and
/// diffusion, which would otherwise stand as millimetre "lakes" that stop
/// incision and trap sediment (observed: the landscape smears). Within one
/// multi-century step such ponding sediments instantly in reality, so the
/// true surface adopts the routed surface there, exactly as the ε-fill did
/// before M3. Deep components survive as real lakes.
///
/// Returns the total raised depth (m summed over merged cells): the merge
/// is a declared mass source (sub-grid fill), and the run meters it so the
/// fake stays quantified rather than silent.
pub fn merge_shallow_depressions(
    grid: &Grid,
    h_true: &mut [f64],
    h_route: &[f64],
    min_lake_depth_m: f64,
) -> f64 {
    let n = grid.n();
    let mut merged_depth_m = 0.0f64;
    let mut seen = vec![false; n];
    let mut component: Vec<u32> = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    for start in 0..n as u32 {
        let su = start as usize;
        if seen[su] || h_route[su] - h_true[su] <= 0.0 {
            continue;
        }
        component.clear();
        let mut max_depth = 0.0f64;
        seen[su] = true;
        queue.push_back(start);
        while let Some(c) = queue.pop_front() {
            let cu = c as usize;
            component.push(c);
            max_depth = num::max(max_depth, h_route[cu] - h_true[cu]);
            grid.for_neighbors(c, |nb, _| {
                let nbu = nb as usize;
                if !seen[nbu] && h_route[nbu] - h_true[nbu] > 0.0 {
                    seen[nbu] = true;
                    queue.push_back(nb);
                }
            });
        }
        if max_depth < min_lake_depth_m {
            for &c in &component {
                merged_depth_m += h_route[c as usize] - h_true[c as usize];
                h_true[c as usize] = h_route[c as usize];
            }
        }
    }
    merged_depth_m
}

/// Weighted multiple-flow-direction graph (Freeman 1991; ADR 0005): every
/// cell sends flow to each strictly lower D8 neighbour of the routing
/// surface, weighted ∝ slope^p. CSR edges, donor lists, a topological
/// order by routed height, and the max-weight channel tree, in one place.
pub struct MfdGraph {
    off: Vec<u32>,
    to: Vec<u32>,
    weight: Vec<f64>,
    /// Along-edge distance in metres (parallel to `to`).
    dist_m: Vec<f64>,
    donor_off: Vec<u32>,
    donor_list: Vec<u32>,
    /// Cell indices sorted by (routed height, index) ascending: every
    /// edge target appears before its source (receivers-first).
    pub order: Vec<u32>,
    /// Max-weight (= steepest; ties to lower index) receiver per cell,
    /// self for base cells: the channel tree for statistics, extraction,
    /// and the `receivers` artifact contract.
    pub tree_receivers: Vec<u32>,
}

impl MfdGraph {
    /// Pure Freeman MFD: every cell spreads.
    pub fn build(grid: &Grid, h_route: &[f64], is_base: &[bool], exponent: f64) -> Self {
        Self::build_inner(grid, h_route, is_base, exponent, None)
    }

    /// Hybrid routing (ADR 0005, amended): flow **converges once
    /// channelized** (Holmgren 1994's area-dependent convergence, taken to
    /// its binary limit at our existing fluvial threshold). Dry cells with
    /// `discharge ≥ channel_min` keep only their steepest edge; hillslopes
    /// and flooded cells spread. Pure MFD alone braids valley floors into
    /// flat multi-thread sheets — measured: ε-flat channel cells 21 → 1228,
    /// Horton Rb 3.2 → 5.9 at island8k.
    pub fn build_hybrid(
        grid: &Grid,
        h_route: &[f64],
        is_base: &[bool],
        exponent: f64,
        discharge: &[f64],
        channel_min: f64,
        flooded: &[bool],
    ) -> Self {
        Self::build_inner(
            grid,
            h_route,
            is_base,
            exponent,
            Some((discharge, channel_min, flooded)),
        )
    }

    fn build_inner(
        grid: &Grid,
        h_route: &[f64],
        is_base: &[bool],
        exponent: f64,
        channels: Option<(&[f64], f64, &[bool])>,
    ) -> Self {
        let n = grid.n();
        // Per-cell edge collection: pure function of the routed surface
        // (and, for the hybrid, the first-pass discharge), parallel-safe.
        let cell_edges: Vec<Vec<(u32, f64, f64)>> = (0..n as u32)
            .into_par_iter()
            .map(|i| {
                if is_base[i as usize] {
                    return Vec::new();
                }
                let hi = h_route[i as usize];
                let mut edges: Vec<(u32, f64, f64)> = Vec::new();
                grid.for_neighbors(i, |nb, fac| {
                    let drop = hi - h_route[nb as usize];
                    if drop > 0.0 {
                        let dist = fac * grid.dx;
                        edges.push((nb, num::pow(drop / dist, exponent), dist));
                    }
                });
                // A loud assert, not debug-only: a stranded cell would be a
                // silent flux sink in release (discharge and sediment
                // vanish, mass closure quietly holds because the sweep
                // never sees the loss). Reachable if epsilon_fill_m is so
                // small that hc + ε rounds back to hc on high terrain —
                // config validation bounds ε away from that, and this is
                // the backstop (CONTRIBUTING: fail loudly).
                assert!(
                    !edges.is_empty(),
                    "non-base cell {i} has no downslope neighbour on the routed surface \
                     (epsilon_fill_m too small for the terrain's height range?)"
                );
                let channelized = channels.is_some_and(|(q, q_min, flooded)| {
                    q[i as usize] >= q_min && !flooded[i as usize]
                });
                if channelized {
                    // Convergent, but not locked to the grid: the flow
                    // takes the surface's own downhill direction and is
                    // split between the two lower neighbours 45 degrees
                    // apart that bracket it, by how it decomposes onto
                    // them (Tarboton 1997's D-infinity, in vector form, so
                    // no trigonometry). A channel on a smooth slope then
                    // runs the slope's way, not along a row, a column or a
                    // diagonal, as a single steepest edge (D8) would make it.
                    if let Some(split) = bracketing_split(grid, h_route, i, &edges) {
                        return split;
                    }
                    // Convergent: single steepest edge (max weight; ties to
                    // the lower index, the project-wide tie policy).
                    let mut best = 0usize;
                    for k in 1..edges.len() {
                        if edges[k].1 > edges[best].1
                            || (edges[k].1 == edges[best].1 && edges[k].0 < edges[best].0)
                        {
                            best = k;
                        }
                    }
                    let mut e = edges[best];
                    e.1 = 1.0;
                    return vec![e];
                }
                let sum: f64 = edges.iter().map(|e| e.1).sum();
                debug_assert!(sum > 0.0, "cell {i} has zero total edge weight");
                for e in &mut edges {
                    e.1 /= sum;
                }
                edges
            })
            .collect();

        let mut off = Vec::with_capacity(n + 1);
        off.push(0u32);
        let mut to = Vec::new();
        let mut weight = Vec::new();
        let mut dist_m = Vec::new();
        for edges in &cell_edges {
            for &(t, w, d) in edges {
                to.push(t);
                weight.push(w);
                dist_m.push(d);
            }
            off.push(to.len() as u32);
        }

        // Donor CSR (reverse adjacency), ascending source order.
        let mut donor_counts = vec![0u32; n + 1];
        for &t in &to {
            donor_counts[t as usize + 1] += 1;
        }
        for i in 1..=n {
            donor_counts[i] += donor_counts[i - 1];
        }
        let donor_off = donor_counts;
        let mut cursor = donor_off.clone();
        let mut donor_list = vec![0u32; to.len()];
        for (src, edges) in cell_edges.iter().enumerate() {
            for &(t, _, _) in edges {
                donor_list[cursor[t as usize] as usize] = src as u32;
                cursor[t as usize] += 1;
            }
        }

        // Topological order: every edge points to strictly lower routed
        // height, so (height, index) ascending is receivers-first. The
        // sort is a total order, hence deterministic under par_sort.
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.par_sort_unstable_by_key(|&i| (f64_key(h_route[i as usize]), i));

        // Channel tree: max weight, ties to the lower index (same policy
        // as the M1 D8 receivers).
        let tree_receivers: Vec<u32> = (0..n as u32)
            .into_par_iter()
            .map(|i| {
                let iu = i as usize;
                let (s, e) = (off[iu] as usize, off[iu + 1] as usize);
                let mut best = i;
                let mut best_w = 0.0f64;
                for k in s..e {
                    if weight[k] > best_w || (weight[k] == best_w && best != i && to[k] < best) {
                        best_w = weight[k];
                        best = to[k];
                    }
                }
                best
            })
            .collect();

        Self {
            off,
            to,
            weight,
            dist_m,
            donor_off,
            donor_list,
            order,
            tree_receivers,
        }
    }

    /// Out-edges of `c`: (target, weight, distance in m).
    #[inline]
    pub fn edges(&self, c: u32) -> impl Iterator<Item = (u32, f64, f64)> + '_ {
        let (s, e) = (
            self.off[c as usize] as usize,
            self.off[c as usize + 1] as usize,
        );
        (s..e).map(move |k| (self.to[k], self.weight[k], self.dist_m[k]))
    }

    /// MFD donors of `c` (cells with an edge into `c`).
    #[inline]
    pub fn donors(&self, c: u32) -> &[u32] {
        &self.donor_list
            [self.donor_off[c as usize] as usize..self.donor_off[c as usize + 1] as usize]
    }
}

/// The downhill direction of `h` at cell `i` (central differences, one-sided at the border), split
/// onto the two lower neighbours among `edges` (target, weight, distance) 45 degrees apart whose
/// cone holds it: weights proportional to the direction's components along each, summing to 1.
/// `None` when the surface is flat there or no such pair holds it.
fn bracketing_split(
    grid: &Grid,
    h: &[f64],
    i: u32,
    edges: &[(u32, f64, f64)],
) -> Option<Vec<(u32, f64, f64)>> {
    let (x, y) = grid.xy(i);
    let at = |xx: u32, yy: u32| h[grid.idx(xx, yy) as usize];
    let (xl, xr) = (x.saturating_sub(1), (x + 1).min(grid.w - 1));
    let (yl, yr) = (y.saturating_sub(1), (y + 1).min(grid.h - 1));
    if xr == xl || yr == yl {
        return None;
    }
    let gx = (at(xr, y) - at(xl, y)) / (f64::from(xr - xl) * grid.dx);
    let gy = (at(x, yr) - at(x, yl)) / (f64::from(yr - yl) * grid.dx);
    let (fx, fy) = (-gx, -gy);
    if fx == 0.0 && fy == 0.0 {
        return None;
    }
    let offset = |nb: u32| {
        let (nx, ny) = grid.xy(nb);
        (i64::from(nx) - i64::from(x), i64::from(ny) - i64::from(y))
    };
    for (a, ea) in edges.iter().enumerate() {
        for eb in &edges[a + 1..] {
            let (oa, ob) = (offset(ea.0), offset(eb.0));
            // One along an axis, one diagonal, 45 degrees apart.
            let (la, lb) = (oa.0 * oa.0 + oa.1 * oa.1, ob.0 * ob.0 + ob.1 * ob.1);
            if la == lb || oa.0 * ob.0 + oa.1 * ob.1 <= 0 {
                continue;
            }
            let len = |l: i64| {
                if l == 1 {
                    1.0
                } else {
                    std::f64::consts::SQRT_2
                }
            };
            let ua = (oa.0 as f64 / len(la), oa.1 as f64 / len(la));
            let ub = (ob.0 as f64 / len(lb), ob.1 as f64 / len(lb));
            let det = ua.0 * ub.1 - ua.1 * ub.0;
            let ca = (fx * ub.1 - fy * ub.0) / det;
            let cb = (ua.0 * fy - ua.1 * fx) / det;
            if ca >= 0.0 && cb >= 0.0 && ca + cb > 0.0 {
                let s = ca + cb;
                let mut out = Vec::with_capacity(2);
                if ca > 0.0 {
                    out.push((ea.0, ca / s, ea.2));
                }
                if cb > 0.0 {
                    out.push((eb.0, cb / s, eb.2));
                }
                return Some(out);
            }
        }
    }
    None
}

/// MFD discharge: each cell's weight plus all upstream flux, split along
/// the out-edges. Sequential over the descending topological order
/// (donors complete before their receivers); deterministic.
pub fn accumulate_discharge_mfd(g: &MfdGraph, weight: &[f64]) -> Vec<f64> {
    let mut q = weight.to_vec();
    for &c in g.order.iter().rev() {
        let qc = q[c as usize];
        for (t, w, _) in g.edges(c) {
            q[t as usize] += qc * w;
        }
    }
    q
}

/// D8 steepest-descent receivers on `h`. Base cells receive themselves.
/// Ties (bit-equal gradients) break toward the lower neighbour index.
pub fn compute_receivers(grid: &Grid, h: &[f64], is_base: &[bool]) -> Vec<u32> {
    (0..grid.n() as u32)
        .into_par_iter()
        .map(|i| {
            if is_base[i as usize] {
                return i;
            }
            let hi = h[i as usize];
            let mut best = i;
            let mut best_grad = 0.0_f64;
            grid.for_neighbors(i, |nb, fac| {
                let drop = hi - h[nb as usize];
                if drop > 0.0 {
                    let grad = drop / fac;
                    if grad > best_grad || (grad == best_grad && best != i && nb < best) {
                        best_grad = grad;
                        best = nb;
                    }
                }
            });
            best
        })
        .collect()
}

/// Donor adjacency in CSR form: `donors(c)` = cells whose receiver is `c`,
/// in ascending index order.
pub struct DonorGraph {
    off: Vec<u32>,
    list: Vec<u32>,
}

impl DonorGraph {
    pub fn build(receivers: &[u32]) -> Self {
        let n = receivers.len();
        let mut counts = vec![0u32; n + 1];
        for (c, &r) in receivers.iter().enumerate() {
            if r as usize != c {
                counts[r as usize + 1] += 1;
            }
        }
        for i in 1..=n {
            counts[i] += counts[i - 1];
        }
        let off = counts;
        let mut cursor = off.clone();
        let mut list = vec![0u32; off[n] as usize];
        for (c, &r) in receivers.iter().enumerate() {
            if r as usize != c {
                list[cursor[r as usize] as usize] = c as u32;
                cursor[r as usize] += 1;
            }
        }
        Self { off, list }
    }

    #[inline]
    pub fn donors(&self, c: u32) -> &[u32] {
        &self.list[self.off[c as usize] as usize..self.off[c as usize + 1] as usize]
    }
}

/// Braun & Willett (2013) stack: a topological order of the receiver forest
/// in which every cell appears after its receiver. Built by iterative DFS
/// from base cells in ascending index order.
pub fn build_stack(receivers: &[u32], donors: &DonorGraph) -> Vec<u32> {
    let n = receivers.len();
    let mut stack = Vec::with_capacity(n);
    let mut dfs: Vec<u32> = Vec::new();
    for b in 0..n as u32 {
        if receivers[b as usize] == b {
            dfs.push(b);
            while let Some(c) = dfs.pop() {
                stack.push(c);
                dfs.extend_from_slice(donors.donors(c));
            }
        }
    }
    debug_assert_eq!(
        stack.len(),
        n,
        "receiver graph must be a forest over all cells"
    );
    stack
}

/// Upslope cell counts (each cell contributes 1, including base cells).
pub fn accumulate_area(receivers: &[u32], stack: &[u32]) -> Vec<u64> {
    let mut area = vec![1u64; receivers.len()];
    for &c in stack.iter().rev() {
        let r = receivers[c as usize];
        if r != c {
            area[r as usize] += area[c as usize];
        }
    }
    area
}

/// Precipitation-weighted accumulation ("discharge" in equivalent cells):
/// each cell contributes its precipitation relative to the land mean, so
/// values stay comparable to cell counts and K keeps its calibration.
/// Sequential in reverse stack order; deterministic.
pub fn accumulate_discharge(receivers: &[u32], stack: &[u32], weight: &[f64]) -> Vec<f64> {
    let mut q = weight.to_vec();
    for &c in stack.iter().rev() {
        let r = receivers[c as usize];
        if r != c {
            q[r as usize] += q[c as usize];
        }
    }
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_grid_with_pit() -> (Grid, Vec<f64>, Vec<bool>) {
        // 8×8 bowl: border at 0 (seeds), interior rim high, centre pit.
        let grid = Grid::new(8, 100.0);
        let n = grid.n();
        let mut h = vec![0.0f64; n];
        let mut seed = vec![false; n];
        for i in 0..n as u32 {
            let (x, y) = grid.xy(i);
            if grid.is_border(i) {
                seed[i as usize] = true;
            } else {
                let cx = (x as f64 - 3.5).abs();
                let cy = (y as f64 - 3.5).abs();
                h[i as usize] = 10.0 - num::max(cx, cy); // rim high, pit at centre
            }
        }
        (grid, h, seed)
    }

    #[test]
    fn flood_leaves_no_land_pits() {
        let (grid, mut h, seed) = tiny_grid_with_pit();
        priority_flood_eps(&grid, &mut h, &seed, 1e-6);
        let rcv = compute_receivers(&grid, &h, &seed);
        for i in 0..grid.n() as u32 {
            if !seed[i as usize] {
                assert_ne!(rcv[i as usize], i, "cell {i} is still a pit after flood");
            }
        }
    }

    #[test]
    fn accumulation_conserves_mass() {
        let (grid, mut h, seed) = tiny_grid_with_pit();
        priority_flood_eps(&grid, &mut h, &seed, 1e-6);
        let rcv = compute_receivers(&grid, &h, &seed);
        let donors = DonorGraph::build(&rcv);
        let stack = build_stack(&rcv, &donors);
        let area = accumulate_area(&rcv, &stack);
        let total_at_bases: u64 = (0..grid.n())
            .filter(|&i| rcv[i] == i as u32)
            .map(|i| area[i])
            .sum();
        assert_eq!(total_at_bases, grid.n() as u64);
    }

    #[test]
    fn a_channel_on_a_slope_off_the_grid_splits_its_water_the_slope_s_way() {
        // A plane falling toward +y and a little toward +x (about 20 degrees off the column):
        // a single steepest edge would send every channel straight down the column; the split
        // sends most down the column and the rest down the diagonal beside it.
        let grid = Grid::new(16, 10.0);
        let n = grid.n();
        let (ex, ey) = (0.342, 0.940);
        let h: Vec<f64> = (0..n as u32)
            .map(|i| {
                let (x, y) = grid.xy(i);
                100.0 - ex * f64::from(x) - ey * f64::from(y)
            })
            .collect();
        let base: Vec<bool> = (0..n as u32).map(|i| grid.is_border(i)).collect();
        let q = vec![1.0e9; n];
        let flooded = vec![false; n];
        let g = MfdGraph::build_hybrid(&grid, &h, &base, 1.1, &q, 1.0, &flooded);
        let c = grid.idx(8, 8);
        let edges: Vec<(u32, f64, f64)> = g.edges(c).collect();
        assert_eq!(edges.len(), 2, "{edges:?}");
        let down = edges
            .iter()
            .find(|e| e.0 == grid.idx(8, 9))
            .expect("down the column");
        let diag = edges
            .iter()
            .find(|e| e.0 == grid.idx(9, 9))
            .expect("down the diagonal");
        assert!(down.1 > diag.1 && diag.1 > 0.2, "{edges:?}");
        assert!((down.1 + diag.1 - 1.0).abs() < 1e-12);
        // Falling exactly down the column, all of it goes down the column.
        let col: Vec<f64> = (0..n as u32)
            .map(|i| 100.0 - f64::from(grid.xy(i).1))
            .collect();
        let g = MfdGraph::build_hybrid(&grid, &col, &base, 1.1, &q, 1.0, &flooded);
        let edges: Vec<(u32, f64, f64)> = g.edges(c).collect();
        assert_eq!(edges.len(), 1, "{edges:?}");
        assert_eq!(edges[0].0, grid.idx(8, 9));
    }
}
