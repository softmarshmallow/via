//! The cutover (TODO, M7): the seed derivation moved into `via-num` (`sub_seed`, `numbers.md`
//! 7.2), and the model's value noise now hashes through `via-num`'s mix and `unit`. Both must
//! give the old bits, since every settled landscape (basin, atlas and the derived cases) is drawn
//! from them. The old code is copied here verbatim as the reference, and a few values are pinned
//! as literals, so a change to both sides still fails.

use via_num::{splitmix64, sub_seed};
use via_terrain::noise;

/// The seed derivation `sub_seed` replaced, as it stood at 3611e0a.
fn old_derive(global_seed: u64, stage: &str, salt: u64) -> u64 {
    let mut h = splitmix64(global_seed ^ 0x5851_F42D_4C95_7F2D);
    for &b in stage.as_bytes() {
        h = splitmix64(h ^ (b as u64));
    }
    splitmix64(h ^ salt)
}

/// `via-terrain`'s `noise::value`, as it stood at 3611e0a.
fn old_value(seed: u64, x: f64, y: f64) -> f64 {
    fn hash2(seed: u64, x: i64, y: i64) -> u64 {
        let h = splitmix64(seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        splitmix64(h ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
    }
    fn lattice(seed: u64, x: i64, y: i64) -> f64 {
        const SCALE: f64 = 2.0 / 9_007_199_254_740_992.0;
        ((hash2(seed, x, y) >> 11) as f64) * SCALE - 1.0
    }
    fn quintic(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
    let x0 = x.floor();
    let y0 = y.floor();
    let (xi, yi) = (x0 as i64, y0 as i64);
    let (tx, ty) = (quintic(x - x0), quintic(y - y0));
    let v00 = lattice(seed, xi, yi);
    let v10 = lattice(seed, xi + 1, yi);
    let v01 = lattice(seed, xi, yi + 1);
    let v11 = lattice(seed, xi + 1, yi + 1);
    let a = v00 + (v10 - v00) * tx;
    let b = v01 + (v11 - v01) * tx;
    a + (b - a) * ty
}

/// `via-terrain`'s `noise::cell_jitter`, as it stood at 3611e0a.
fn old_jitter(seed: u64, cell_index: u32) -> f64 {
    const SCALE: f64 = 2.0 / 9_007_199_254_740_992.0;
    ((splitmix64(seed ^ (cell_index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)) >> 11) as f64)
        * SCALE
        - 1.0
}

/// The labels and salts the model draws from (`fields.rs`, `climate.rs`), and some others.
const LABELS: [&str; 5] = ["terrain", "climate", "", "a", "hydrology"];
const SEEDS: [u64; 6] = [0, 1, 7, 42, 0x5851_F42D_4C95_7F2D, u64::MAX];

#[test]
fn sub_seed_gives_the_old_seeds() {
    for seed in SEEDS {
        for label in LABELS {
            for salt in 0..8 {
                assert_eq!(
                    sub_seed(seed, label, salt),
                    old_derive(seed, label, salt),
                    "({seed}, {label:?}, {salt})"
                );
            }
        }
    }
    // Pinned by the old derivation's own test, computed from the definition.
    assert_eq!(sub_seed(42, "terrain", 0), 0xF07E_DA44_EA28_EFC2);
}

#[test]
fn value_noise_gives_the_old_bits() {
    for seed in SEEDS {
        for i in -40..40i32 {
            for j in -7..7i32 {
                let (x, y) = (f64::from(i) * 0.37 + 0.011, f64::from(j) * 1.13 - 0.5);
                assert_eq!(
                    noise::value(seed, x, y).to_bits(),
                    old_value(seed, x, y).to_bits(),
                    "value({seed}, {x}, {y})"
                );
            }
        }
        for k in [0u32, 1, 2, 1000, u32::MAX] {
            assert_eq!(
                noise::cell_jitter(seed, k).to_bits(),
                old_jitter(seed, k).to_bits()
            );
        }
    }
}
