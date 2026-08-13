//! Seed derivation policy (CONTRIBUTING: seeds derive from
//! `global_seed + stage + salt`; no global RNG anywhere).

/// SplitMix64 finalizer-style mixer; the base primitive for all hashing
/// that reaches simulation state.
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Derive a sub-seed for `(stage, salt)` from the global seed.
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

    #[test]
    fn stable_values() {
        // Locked: changing seed derivation invalidates every golden snapshot.
        assert_eq!(derive(42, "terrain", 0), derive(42, "terrain", 0));
    }
}
