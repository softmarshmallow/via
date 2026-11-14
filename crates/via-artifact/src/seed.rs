//! Sub-seeds of the erosion model: one per `(stage, salt)` from the run's seed, no global random
//! state anywhere.

use via_num::splitmix64;

/// The sub-seed for `(stage, salt)` from the run's seed. Its values are the old workspace's,
/// bit for bit: only the home of SplitMix64 moved to `via-num`.
pub fn derive(global_seed: u64, stage: &str, salt: u64) -> u64 {
    let mut h = splitmix64(global_seed ^ 0x5851_F42D_4C95_7F2D);
    for &b in stage.as_bytes() {
        h = splitmix64(h ^ (b as u64));
    }
    splitmix64(h ^ salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_stages_distinct_seeds() {
        assert_ne!(derive(1, "terrain", 0), derive(1, "hydrology", 0));
        assert_ne!(derive(1, "terrain", 0), derive(1, "terrain", 1));
        assert_ne!(derive(1, "terrain", 0), derive(2, "terrain", 0));
    }

    /// Locked: changing the derivation moves every simulated landscape.
    #[test]
    fn stable_values() {
        // Computed independently from the definition (SplitMix64, Steele, Lea and Flood, 2014).
        assert_eq!(derive(42, "terrain", 0), 0xF07E_DA44_EA28_EFC2);
    }
}
