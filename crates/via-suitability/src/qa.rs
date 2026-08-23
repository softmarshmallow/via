//! QA diagnostics — ADR 0008 D10 visual-inspection channel: never CI,
//! never gates (ADR 0011 D6 demotes the col cross-check explicitly),
//! and deliberately absent from the stage summary. Consumed by the CLI
//! for renders and stdout only.

use std::collections::BTreeMap;

use crate::for_neighbors8;

/// Basin-boundary col cross-check (0013 §Passes, a project
/// observation): for each pair of adjacent land basins, the
/// minimum-height cell on their shared boundary — the optimal vertical
/// crossing of that D8 divide. Correspondence with saddle sites is
/// expected but not invariant: basins are labeled by ocean outlet
/// (coastal adjacencies bottom out at the shoreline) and divides derive
/// from the epsilon-filled routed surface, so they can wander off
/// heights ridges across filled flats. Returns the distinct col cells
/// in ascending order.
pub fn basin_boundary_cols(
    w: u32,
    h: u32,
    heights_m: &[f64],
    basin: &[u32],
    land: &[bool],
) -> Vec<u32> {
    // Lower in the total order = lower height, ties by ascending index.
    let lower = |a: u32, b: u32| (heights_m[a as usize], a) < (heights_m[b as usize], b);
    let mut best: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for i in 0..(w * h) {
        if !land[i as usize] {
            continue;
        }
        let bi = basin[i as usize];
        for_neighbors8(w, h, i, |nb, _| {
            let nbi = nb as usize;
            if land[nbi] && basin[nbi] != bi {
                let key = (bi.min(basin[nbi]), bi.max(basin[nbi]));
                best.entry(key)
                    .and_modify(|c| {
                        if lower(i, *c) {
                            *c = i;
                        }
                    })
                    .or_insert(i);
            }
        });
    }
    let mut cells: Vec<u32> = best.into_values().collect();
    cells.sort_unstable();
    cells.dedup();
    cells
}

/// Via's instantiation of the Filet et al. (2025) change-point
/// instrument (adopted by ADR 0011 D4 as a consistency cross-check, not
/// a second authority): for each mainstem — walked upstream from a
/// river mouth along the largest-drainage river donor — the single
/// least-squares change-point of the per-step water-surface drop
/// sequence splits the profile into a lower "plain" section and an
/// upper section. Returns the boundary cell per stem of at least
/// `min_len` cells, in ascending cell order. The single-change-point
/// least-squares form is via's minimal reading of the paper's method,
/// stated as such; it lives only in this QA channel.
/// The artifact slices the change-point walk reads.
pub struct StemInputs<'a> {
    pub receivers: &'a [u32],
    pub strahler: &'a [u32],
    pub area_cells: &'a [u64],
    pub land: &'a [bool],
    pub surface_m: &'a [f64],
}

pub fn filet_change_points(w: u32, h: u32, inp: &StemInputs, min_len: usize) -> Vec<u32> {
    let StemInputs {
        receivers,
        strahler,
        area_cells,
        land,
        surface_m,
    } = *inp;
    let n = (w * h) as usize;
    let mut points = Vec::new();
    for mouth in 0..n as u32 {
        let mi = mouth as usize;
        // A mouth: river cell whose receiver is ocean.
        if strahler[mi] == 0 || land[receivers[mi] as usize] {
            continue;
        }
        // Walk upstream along the largest-drainage river donor.
        let mut stem = vec![mouth];
        let mut c = mouth;
        loop {
            let mut next: Option<u32> = None;
            for_neighbors8(w, h, c, |nb, _| {
                if receivers[nb as usize] == c && nb != c && strahler[nb as usize] > 0 {
                    let better = match next {
                        None => true,
                        Some(cur) => {
                            (area_cells[nb as usize], std::cmp::Reverse(nb))
                                > (area_cells[cur as usize], std::cmp::Reverse(cur))
                        }
                    };
                    if better {
                        next = Some(nb);
                    }
                }
            });
            match next {
                Some(d) => {
                    stem.push(d);
                    c = d;
                }
                None => break,
            }
            if stem.len() > n {
                break; // corrupt receivers; QA gives up on this stem
            }
        }
        if stem.len() < min_len {
            continue;
        }
        // Per-step drops along the stem (downstream cell to upstream cell).
        let drops: Vec<f64> = stem
            .windows(2)
            .map(|p| surface_m[p[1] as usize] - surface_m[p[0] as usize])
            .collect();
        if let Some(k) = least_squares_change_point(&drops) {
            points.push(stem[k]);
        }
    }
    points.sort_unstable();
    points.dedup();
    points
}

/// Index k (1..len-1) minimizing the two-segment constant-fit SSE, via
/// prefix sums; None if the sequence is too short or flat.
fn least_squares_change_point(xs: &[f64]) -> Option<usize> {
    let l = xs.len();
    if l < 4 {
        return None;
    }
    let mut prefix = vec![0.0f64; l + 1];
    let mut prefix2 = vec![0.0f64; l + 1];
    for (i, &x) in xs.iter().enumerate() {
        prefix[i + 1] = prefix[i] + x;
        prefix2[i + 1] = prefix2[i] + x * x;
    }
    let sse = |a: usize, b: usize| {
        // SSE of xs[a..b] against its mean.
        let s = prefix[b] - prefix[a];
        let s2 = prefix2[b] - prefix2[a];
        s2 - s * s / (b - a) as f64
    };
    let mut best: Option<(f64, usize)> = None;
    for k in 1..l {
        let cost = sse(0, k) + sse(k, l);
        if best.is_none() || cost < best.unwrap().0 {
            best = Some((cost, k));
        }
    }
    best.map(|(_, k)| k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_between_two_basins_is_their_boundary_minimum() {
        // 5×3, two land basins split at x=2; heights dip at (2,1).
        let (w, h) = (5u32, 3u32);
        let n = (w * h) as usize;
        let mut heights = vec![10.0f64; n];
        let mut basin = vec![0u32; n];
        let land = vec![true; n];
        for y in 0..3 {
            for x in 3..5 {
                basin[(y * w + x) as usize] = 1;
            }
        }
        heights[(w + 2) as usize] = 4.0; // the low crossing at (2,1)
        let cols = basin_boundary_cols(w, h, &heights, &basin, &land);
        assert_eq!(cols, vec![w + 2]);
    }

    #[test]
    fn change_point_lands_at_the_slope_break() {
        // 12 gentle drops then 8 steep ones: the break is at index 12.
        let mut xs = vec![0.1f64; 12];
        xs.extend(vec![2.0f64; 8]);
        assert_eq!(least_squares_change_point(&xs), Some(12));
        assert_eq!(least_squares_change_point(&[0.1, 0.1]), None);
    }
}
