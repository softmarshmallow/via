//! Derived structure on the final surface: Strahler orders, basin labels,
//! flow distances. All measurements, no authored labels.

use crate::flow::DonorGraph;
use crate::grid::Grid;

/// Strahler stream order for river cells (land cells whose flow metric —
/// discharge in equivalent cells — is at least `min_metric`); 0 elsewhere.
/// Processed in reverse stack order so every donor is ordered before its
/// receiver.
pub fn strahler_orders(
    receivers: &[u32],
    stack: &[u32],
    donors: &DonorGraph,
    land: &[bool],
    flow_metric: &[f64],
    min_metric: f64,
) -> Vec<u32> {
    let n = receivers.len();
    let mut order = vec![0u32; n];
    let is_river = |i: usize| land[i] && flow_metric[i] >= min_metric;
    for &c in stack.iter().rev() {
        let ci = c as usize;
        if !is_river(ci) {
            continue;
        }
        let mut max_o = 0u32;
        let mut n_max = 0u32;
        for &d in donors.donors(c) {
            let o = order[d as usize];
            if o == 0 {
                continue;
            }
            if o > max_o {
                max_o = o;
                n_max = 1;
            } else if o == max_o {
                n_max += 1;
            }
        }
        order[ci] = if max_o == 0 {
            1
        } else if n_max >= 2 {
            max_o + 1
        } else {
            max_o
        };
    }
    order
}

/// Basin label = index of the base-level cell each cell drains to.
pub fn label_basins(receivers: &[u32], stack: &[u32]) -> Vec<u32> {
    let mut label = vec![0u32; receivers.len()];
    for &c in stack {
        let r = receivers[c as usize];
        label[c as usize] = if r == c { c } else { label[r as usize] };
    }
    label
}

/// Along-flow distance to the basin outlet, in metres.
pub fn flow_distance_m(grid: &Grid, receivers: &[u32], stack: &[u32]) -> Vec<f64> {
    let mut dist = vec![0.0f64; receivers.len()];
    for &c in stack {
        let r = receivers[c as usize];
        if r != c {
            dist[c as usize] = dist[r as usize] + grid.step_dist_m(c, r);
        }
    }
    dist
}

/// For each cell, the along-flow distance from the cell up to the farthest
/// drainage divide in its own subbasin, in metres (0 at divides). Computed
/// as max-upstream outlet distance minus the cell's outlet distance.
pub fn mainstream_length_m(receivers: &[u32], stack: &[u32], dist_m: &[f64]) -> Vec<f64> {
    let mut max_up = dist_m.to_vec();
    for &c in stack.iter().rev() {
        let r = receivers[c as usize];
        if r != c && max_up[c as usize] > max_up[r as usize] {
            max_up[r as usize] = max_up[c as usize];
        }
    }
    max_up
        .iter()
        .zip(dist_m.iter())
        .map(|(&m, &d)| m - d)
        .collect()
}
