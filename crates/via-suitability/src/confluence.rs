//! Confluence sites — definitional on the inverted receivers tree
//! (ADR 0011 D4): a river cell (strahler > 0) with ≥ 2 river donors.
//! Zero parameters of its own; the channelization threshold that defines
//! river membership is the terrain stage's `river_min_area_km2`, restated
//! in the stage summary with any reported number.
//!
//! Importance is the Benda et al. (2004) symmetry ratio — smaller/larger
//! contributing drainage of the two largest river donors, exact from the
//! `area_cells` artifact. Taking the two largest when a D8 cell has more
//! than two river donors is a declared adaptation (the paper treats
//! two-branch junctions).

use serde::Serialize;

use crate::for_neighbors8;

/// One confluence site. Sites are emitted in ascending cell order — the
/// fixed, documented order ADR 0011 D3 requires.
#[derive(Clone, Debug, Serialize)]
pub struct ConfluenceSite {
    /// Flat cell index of the junction (row-major, y·w + x).
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    /// Strahler order of the junction cell (from the emitted artifact).
    pub strahler: u32,
    /// Number of river donors (strahler > 0 cells draining here).
    pub river_donors: u32,
    /// Exact upslope cell counts of the two largest river donors.
    pub donor_area_cells_larger: u64,
    pub donor_area_cells_smaller: u64,
    /// Benda et al. (2004) symmetry ratio: smaller / larger donor drainage.
    pub symmetry_ratio: f64,
}

/// River donors of `cell`: D8 neighbours that are river cells and whose
/// receiver is `cell`. Receivers always point to a D8 neighbour, so the
/// neighbour scan is exhaustive; fixed scan order keeps it deterministic.
fn river_donors_of(w: u32, h: u32, cell: u32, receivers: &[u32], strahler: &[u32]) -> Vec<u32> {
    let mut donors = Vec::new();
    for_neighbors8(w, h, cell, |nb, _| {
        if receivers[nb as usize] == cell && nb != cell && strahler[nb as usize] > 0 {
            donors.push(nb);
        }
    });
    donors
}

/// Detect all confluence sites, in ascending cell order.
pub fn detect(
    w: u32,
    h: u32,
    receivers: &[u32],
    strahler: &[u32],
    area_cells: &[u64],
) -> Vec<ConfluenceSite> {
    let n = w as usize * h as usize;
    let mut sites = Vec::new();
    for cell in 0..n as u32 {
        if strahler[cell as usize] == 0 {
            continue;
        }
        let donors = river_donors_of(w, h, cell, receivers, strahler);
        if donors.len() < 2 {
            continue;
        }
        let mut areas: Vec<u64> = donors.iter().map(|&d| area_cells[d as usize]).collect();
        areas.sort_unstable_by(|a, b| b.cmp(a));
        let (larger, smaller) = (areas[0], areas[1]);
        sites.push(ConfluenceSite {
            cell,
            x: cell % w,
            y: cell / w,
            strahler: strahler[cell as usize],
            river_donors: donors.len() as u32,
            donor_area_cells_larger: larger,
            donor_area_cells_smaller: smaller,
            symmetry_ratio: smaller as f64 / larger as f64,
        });
    }
    sites
}

/// ADR 0011 D6 confluence-definition gate: every emitted site has ≥ 2
/// river donors and no non-emitted river cell does. Donor counts are
/// recomputed here by a full receiver sweep — a different traversal than
/// `detect`'s neighbour scan — with river membership read from the emitted
/// strahler artifact. Binary; any mismatch fails.
pub fn verify_definition(
    w: u32,
    h: u32,
    receivers: &[u32],
    strahler: &[u32],
    sites: &[ConfluenceSite],
) -> bool {
    let n = w as usize * h as usize;
    let mut donor_count = vec![0u32; n];
    for i in 0..n {
        let r = receivers[i] as usize;
        if r != i && strahler[i] > 0 {
            donor_count[r] += 1;
        }
    }
    let mut expected: Vec<u32> = (0..n as u32)
        .filter(|&c| strahler[c as usize] > 0 && donor_count[c as usize] >= 2)
        .collect();
    expected.sort_unstable();
    let emitted: Vec<u32> = sites.iter().map(|s| s.cell).collect();
    if emitted != expected {
        return false;
    }
    sites
        .iter()
        .all(|s| s.river_donors == donor_count[s.cell as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    // 5×5 grid; a main stem flows west along row 2 and a tributary joins
    // from the north at (2,2). Receivers elsewhere self-point (ocean).
    fn fixture() -> (u32, u32, Vec<u32>, Vec<u32>, Vec<u64>) {
        let (w, h) = (5u32, 5u32);
        let n = (w * h) as usize;
        let idx = |x: u32, y: u32| y * w + x;
        let mut receivers: Vec<u32> = (0..n as u32).collect();
        let mut strahler = vec![0u32; n];
        let mut area = vec![1u64; n];
        // Main stem: (4,2) -> (3,2) -> (2,2) -> (1,2) -> (0,2).
        for x in 1..=4 {
            receivers[idx(x, 2) as usize] = idx(x - 1, 2);
            strahler[idx(x, 2) as usize] = 1;
        }
        // Outlet cell (0,2) stays strahler 0: not a river.
        // Tributary: (2,0) -> (2,1) -> (2,2).
        receivers[idx(2, 0) as usize] = idx(2, 1);
        receivers[idx(2, 1) as usize] = idx(2, 2);
        strahler[idx(2, 0) as usize] = 1;
        strahler[idx(2, 1) as usize] = 1;
        // Drainage: main stem donor (3,2) carries 10 cells, tributary
        // donor (2,1) carries 4.
        area[idx(3, 2) as usize] = 10;
        area[idx(2, 1) as usize] = 4;
        (w, h, receivers, strahler, area)
    }

    #[test]
    fn junction_is_detected_with_exact_symmetry_ratio() {
        let (w, h, receivers, strahler, area) = fixture();
        let sites = detect(w, h, &receivers, &strahler, &area);
        assert_eq!(sites.len(), 1);
        let s = &sites[0];
        assert_eq!((s.x, s.y), (2, 2));
        assert_eq!(s.river_donors, 2);
        assert_eq!(s.donor_area_cells_larger, 10);
        assert_eq!(s.donor_area_cells_smaller, 4);
        assert!((s.symmetry_ratio - 0.4).abs() < 1e-12);
        assert!(verify_definition(w, h, &receivers, &strahler, &sites));
    }

    #[test]
    fn verification_rejects_missing_and_spurious_sites() {
        let (w, h, receivers, strahler, area) = fixture();
        let sites = detect(w, h, &receivers, &strahler, &area);
        assert!(!verify_definition(w, h, &receivers, &strahler, &[]));
        let mut spurious = sites.clone();
        spurious.push(ConfluenceSite {
            cell: 0,
            x: 0,
            y: 0,
            strahler: 0,
            river_donors: 2,
            donor_area_cells_larger: 1,
            donor_area_cells_smaller: 1,
            symmetry_ratio: 1.0,
        });
        assert!(!verify_definition(w, h, &receivers, &strahler, &spurious));
    }
}
