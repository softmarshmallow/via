//! Pass sites — Morse saddles of the height field with persistence
//! (ADR 0011 D4, research 0013 §Passes).
//!
//! Detection is the superlevel-set sweep: land cells sorted by height
//! descending, one union-find pass; a cell that merges two components is a
//! saddle, paired with the peak of the component it merges away, and
//! persistence = z(paired peak) − z(saddle) (Edelsbrunner, Letscher &
//! Zomorodian 2002; instantiated at DEM scale by Kirmse & de Ferranti
//! 2017, whose ~30 m minimum-prominence floor is the config default).
//! The Peucker–Douglas (1975) 8-ring operator is recorded per site as a
//! diagnostic — Takahashi et al. (1995) showed the raw operator is
//! topologically inconsistent on grids, which is why the sweep, not the
//! ring, decides the site set.
//!
//! Declared adaptations: the domain is land cells (passes are land-
//! movement affordances; ocean bathymetry is excluded); out-of-grid
//! neighbours count as lower; equal integer-cm heights are totally
//! ordered by ascending cell index (the deterministic tie-break ADR 0011
//! D4 requires); a cell merging more than two components records the
//! largest persistence among the components it kills.

use serde::Serialize;

/// One pass site. Sites are emitted in ascending cell order — the fixed,
/// documented order ADR 0011 D3 requires.
#[derive(Clone, Debug, Serialize)]
pub struct SaddleSite {
    /// Flat cell index of the saddle (row-major, y·w + x).
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    pub elevation_m: f64,
    /// Persistence: z(paired peak) − z(saddle), metres.
    pub persistence_m: f64,
    /// The merged-away (paired) peak.
    pub peak_cell: u32,
    pub peak_elevation_m: f64,
    /// Peucker–Douglas 8-ring operator agreement (diagnostic only).
    pub ring_saddle: bool,
}

/// Total order on cells: higher height first; equal heights break ties by
/// ascending cell index ("earlier index is higher"). Every comparison in
/// this module goes through this order, so the sweep and the ring
/// diagnostic agree on what "higher" means.
#[inline]
fn higher(heights_cm: &[i32], a: u32, b: u32) -> bool {
    (heights_cm[a as usize], std::cmp::Reverse(a)) > (heights_cm[b as usize], std::cmp::Reverse(b))
}

/// Peucker–Douglas 3×3 operator: cyclic signs of (neighbour − centre)
/// around the 8-ring; four or more sign changes is a saddle candidate.
/// Out-of-grid neighbours count as lower.
fn ring_test(w: u32, h: u32, heights_cm: &[i32], c: u32) -> bool {
    // Cyclic ring order (not scan order): clockwise from the NW corner.
    const RING: [(i32, i32); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
    ];
    let (x, y) = ((c % w) as i64, (c / w) as i64);
    let mut signs = [false; 8]; // true = neighbour higher than centre
    for (k, &(dx, dy)) in RING.iter().enumerate() {
        let (nx, ny) = (x + dx as i64, y + dy as i64);
        if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
            let nb = (ny * w as i64 + nx) as u32;
            signs[k] = higher(heights_cm, nb, c);
        }
    }
    let changes = (0..8).filter(|&k| signs[k] != signs[(k + 1) % 8]).count();
    changes >= 4
}

struct UnionFind {
    parent: Vec<u32>,
    /// Peak cell of each component root — the first (highest) cell the
    /// component saw in sweep order.
    peak: Vec<u32>,
}

impl UnionFind {
    fn find(&mut self, mut i: u32) -> u32 {
        while self.parent[i as usize] != i {
            self.parent[i as usize] = self.parent[self.parent[i as usize] as usize];
            i = self.parent[i as usize];
        }
        i
    }
}

/// Detect all pass sites with persistence ≥ `min_persistence_m`, in
/// ascending cell order.
pub fn detect(
    w: u32,
    h: u32,
    heights_cm: &[i32],
    land: &[bool],
    min_persistence_m: f64,
) -> Vec<SaddleSite> {
    let n = w as usize * h as usize;
    // Land cells in sweep order: height descending, index ascending on ties.
    let mut order: Vec<u32> = (0..n as u32).filter(|&i| land[i as usize]).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(heights_cm[i as usize]), i));

    let mut uf = UnionFind {
        parent: (0..n as u32).collect(),
        peak: (0..n as u32).collect(),
    };
    let mut processed = vec![false; n];
    let mut sites = Vec::new();

    for &c in &order {
        processed[c as usize] = true;
        // Distinct components among already-processed (higher) neighbours.
        let mut roots: Vec<u32> = Vec::with_capacity(8);
        crate::for_neighbors8(w, h, c, |nb, _| {
            if processed[nb as usize] && land[nb as usize] {
                let r = uf.find(nb);
                if !roots.contains(&r) {
                    roots.push(r);
                }
            }
        });
        match roots.len() {
            0 => {} // a peak: c starts its own component (already its own root)
            _ => {
                // Attach c to the component with the highest peak; every
                // further distinct component merges here and dies.
                roots.sort_by(|&a, &b| {
                    if higher(heights_cm, uf.peak[a as usize], uf.peak[b as usize]) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                });
                let survivor = roots[0];
                uf.parent[c as usize] = survivor;
                let mut best: Option<(u32, i32)> = None; // (dying peak, its height)
                for &r in &roots[1..] {
                    let dying_peak = uf.peak[r as usize];
                    uf.parent[r as usize] = survivor;
                    let p = heights_cm[dying_peak as usize];
                    if best.is_none() || p > best.unwrap().1 {
                        best = Some((dying_peak, p));
                    }
                }
                if let Some((peak_cell, peak_cm)) = best {
                    let persistence_m = (peak_cm - heights_cm[c as usize]) as f64 / 100.0;
                    if persistence_m >= min_persistence_m {
                        sites.push(SaddleSite {
                            cell: c,
                            x: c % w,
                            y: c / w,
                            elevation_m: heights_cm[c as usize] as f64 / 100.0,
                            persistence_m,
                            peak_cell,
                            peak_elevation_m: peak_cm as f64 / 100.0,
                            ring_saddle: ring_test(w, h, heights_cm, c),
                        });
                    }
                }
            }
        }
    }
    sites.sort_by_key(|s| s.cell);
    sites
}

#[cfg(test)]
mod tests {
    use super::*;

    // 7×5, all land: two peaks on row 2 (A = 100 m at x=1, B = 80 m at
    // x=5) joined over a 50 m col at x=3; everything else 1 cm.
    fn fixture() -> (u32, u32, Vec<i32>, Vec<bool>) {
        let (w, h) = (7u32, 5u32);
        let n = (w * h) as usize;
        let mut heights = vec![1i32; n];
        let row = |x: u32| (2 * w + x) as usize;
        heights[row(1)] = 10_000;
        heights[row(2)] = 6_000;
        heights[row(3)] = 5_000;
        heights[row(4)] = 6_000;
        heights[row(5)] = 8_000;
        (w, h, heights, vec![true; n])
    }

    #[test]
    fn col_between_two_peaks_gets_exact_persistence() {
        let (w, h, heights, land) = fixture();
        let sites = detect(w, h, &heights, &land, 0.0);
        // Exactly one site above the trivial floor: the col at (3,2),
        // pairing away peak B (80 m): persistence = 80 − 50 = 30 m.
        let major: Vec<_> = sites.iter().filter(|s| s.persistence_m > 1.0).collect();
        assert_eq!(major.len(), 1);
        let s = major[0];
        assert_eq!((s.x, s.y), (3, 2));
        assert!((s.persistence_m - 30.0).abs() < 1e-9);
        assert_eq!(s.peak_cell, 2 * w + 5);
        assert!(s.ring_saddle, "ideal col should satisfy the ring operator");
    }

    #[test]
    fn persistence_floor_prunes() {
        let (w, h, heights, land) = fixture();
        assert!(!detect(w, h, &heights, &land, 29.0).is_empty());
        assert!(detect(w, h, &heights, &land, 31.0)
            .iter()
            .all(|s| s.persistence_m >= 31.0));
    }

    #[test]
    fn equal_height_plateau_col_yields_exactly_one_major_site() {
        // Widen the col to three equal-height cells; the tie-break must
        // pick exactly one merge cell, deterministically.
        let (w, h, mut heights, land) = fixture();
        heights[(2 * w + 2) as usize] = 5_000;
        heights[(2 * w + 4) as usize] = 5_000;
        let a = detect(w, h, &heights, &land, 10.0);
        let b = detect(w, h, &heights, &land, 10.0);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].cell, b[0].cell);
        assert!((a[0].persistence_m - 30.0).abs() < 1e-9);
    }

    #[test]
    fn ocean_cells_are_outside_the_domain() {
        let (w, h, heights, mut land) = fixture();
        // Sink everything except the two peaks and the col into ocean.
        for (i, l) in land.iter_mut().enumerate() {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            *l = y == 2 && (1..=5).contains(&x);
        }
        let sites = detect(w, h, &heights, &land, 0.0);
        assert_eq!(sites.len(), 1);
        assert_eq!((sites[0].x, sites[0].y), (3, 2));
    }
}
