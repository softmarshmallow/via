//! Named seed streams and draws keyed by place (`numbers.md` 7).
//!
//! There is no global random state and no generator whose output depends on how many draws came
//! before: a draw is a pure function of a stream and the keys that name where it is for, a
//! counter-based generator in the sense of Salmon, Moraes, Dror and Shaw (2011) with SplitMix64
//! (Steele, Lea and Flood, 2014) as its mixing function. So a draw at one place never moves when a
//! draw elsewhere is added or removed. All arithmetic here wraps, by definition, and only here
//! (`numbers.md` 8, rule 6).

use crate::error::NumError;

/// The root constant xored into a seed before the parts are absorbed (`numbers.md` 7.2).
pub const ROOT: u64 = 0x5851_F42D_4C95_7F2D;

/// SplitMix64's finaliser over `x` (`numbers.md` 7.2).
pub fn splitmix64(x: u64) -> u64 {
    let mut x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// One word into a hash state: `splitmix64(h ^ x)`.
pub fn mix(h: u64, x: u64) -> u64 {
    splitmix64(h ^ x)
}

/// A byte string into a hash state, length first, so `["ab", "c"]` and `["a", "bc"]` differ.
pub fn absorb(h: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(mix(h, bytes.len() as u64), |h, &b| mix(h, u64::from(b)))
}

/// The stream named by `parts` under `seed` (`numbers.md` 7.2). The first part says who draws
/// (`"rule"`, `"noise"`, `"name"`, `"step"`), so streams of different users never meet.
/// `identity.md`'s digest is this function with seed 0, not a second one.
pub fn stream<P: AsRef<[u8]>>(seed: u64, parts: &[P]) -> u64 {
    parts
        .iter()
        .fold(splitmix64(seed ^ ROOT), |h, p| absorb(h, p.as_ref()))
}

/// The draw of a stream at a place named by integer keys, each mixed as its two's-complement bits.
pub fn draw(stream: u64, keys: &[i64]) -> u64 {
    keys.iter()
        .fold(mix(stream, keys.len() as u64), |h, &k| mix(h, k as u64))
}

/// The earlier Rust code's sub-seed for `(label, salt)` under `seed` (`numbers.md` 7.2,
/// `sub_seed`): the label's bytes mixed one by one from `seed`'s root, then the salt. It absorbs no
/// length, so it keeps labels apart less well than [`stream`]; it is kept, bit for bit, for the one
/// library model drawn from it (`lib/terrain`'s program), and a new user takes [`stream`].
pub fn sub_seed(seed: u64, label: &str, salt: u64) -> u64 {
    let h = label
        .as_bytes()
        .iter()
        .fold(splitmix64(seed ^ ROOT), |h, &b| mix(h, u64::from(b)));
    mix(h, salt)
}

/// A word to a double in `[0, 1)`: the top 53 bits times `2^-53`, exact.
pub fn unit(u: u64) -> f64 {
    // 2^-53 is exact as a double, and a 53-bit integer converts exactly, so one correctly rounded
    // product is exact too.
    ((u >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
}

/// Checks a world seed written as a TOML integer and returns it as the unsigned word it is used
/// as (`numbers.md` 7.1). A negative one is `num.seed_range`; one above `2^63 - 1` cannot reach
/// here, as it is not a TOML integer.
pub fn world_seed(seed: i64) -> Result<u64, NumError> {
    u64::try_from(seed).map_err(|_| NumError::SeedRange { seed })
}

/// Whether `name` matches the id grammar of `vocabulary.md` 2.1: `[a-z][a-z0-9_]*`, at most 48
/// bytes.
fn is_id(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 48
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// Checks a seed name (a scatter's `seed`, `numbers.md` 7.1). One that breaks the id grammar is
/// `num.seed_type`, with its lower-cased form as the hint when that one is a valid name.
pub fn seed_name(name: &str) -> Result<&str, NumError> {
    if is_id(name) {
        return Ok(name);
    }
    let lower = name.to_ascii_lowercase();
    Err(NumError::SeedType {
        text: name.to_string(),
        hint: is_id(&lower).then_some(lower),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values published with the SplitMix64 reference (Vigna's splitmix64.c, seeded at 0 and 1):
    // independent of this crate, unlike a test that compares a value with itself. The named
    // streams are pinned in spec/vectors/numbers.toml, read by tests/vectors.rs.
    #[test]
    fn splitmix64_matches_the_reference() {
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(1), 0x910A_2DEC_8902_5CC1);
        // The reference generator's second output from state 0 is the finaliser at state
        // 0x9E3779B97F4A7C15.
        assert_eq!(splitmix64(0x9E37_79B9_7F4A_7C15), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn length_prefix_keeps_part_boundaries() {
        assert_ne!(stream(0, &["ab", "c"]), stream(0, &["a", "bc"]));
        assert_ne!(draw(7, &[3, 4]), draw(7, &[3, 4, 0]));
    }

    /// `sub_seed` as `numbers.md` 7.2 defines it, from the primitives; its values on the old
    /// workspace's vectors are pinned where it is used (`crates/via-terrain/tests/carried.rs`).
    #[test]
    fn sub_seed_is_its_definition() {
        let root = splitmix64(9 ^ ROOT);
        assert_eq!(sub_seed(9, "", 5), mix(root, 5));
        assert_eq!(sub_seed(9, "ab", 5), mix(mix(mix(root, 0x61), 0x62), 5));
        // No length absorbed: unlike `stream`, it is not `absorb` under another name.
        assert_ne!(sub_seed(9, "ab", 0), mix(absorb(root, b"ab"), 0));
        assert_ne!(sub_seed(9, "ab", 0), sub_seed(9, "ab", 1));
        assert_ne!(sub_seed(9, "ab", 0), sub_seed(10, "ab", 0));
    }

    #[test]
    fn unit_bounds() {
        assert_eq!(unit(0).to_bits(), 0);
        assert_eq!(unit(u64::MAX).to_bits(), 0x3FEF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn seeds_in_text() {
        assert_eq!(world_seed(0), Ok(0));
        assert_eq!(world_seed(-1).map_err(|e| e.code()), Err("num.seed_range"));
        assert_eq!(seed_name("oaks_2"), Ok("oaks_2"));
        match seed_name("Oaks") {
            Err(NumError::SeedType { hint, .. }) => assert_eq!(hint.as_deref(), Some("oaks")),
            other => panic!("{other:?}"),
        }
        match seed_name("1x") {
            Err(NumError::SeedType { hint, .. }) => assert_eq!(hint, None),
            other => panic!("{other:?}"),
        }
    }
}
