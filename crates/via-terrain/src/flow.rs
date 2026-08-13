//! Flow routing: priority-flood depression handling (Barnes et al. 2014),
//! D8 receivers, the Braun–Willett stack, and drainage accumulation.
//!
//! Everything order-sensitive in here is sequential by design; determinism
//! comes from fixed iteration order and explicit tie-breaking, not luck.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

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
            max_depth = max_depth.max(h_route[cu] - h_true[cu]);
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
                h[i as usize] = 10.0 - cx.max(cy); // rim high, pit at centre
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
}
