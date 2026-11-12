//! Every vector of `spec/vectors/numbers.toml` through the code the build uses (`numbers.md` 10),
//! and the null digest recorded in `spec/vectors/num/null_digest.txt`.
//!
//! Expected values live only in the vector file, so this test embeds no stream names or bits:
//! changing a definition shows up as a failing vector id, not as an edited literal.

use std::path::PathBuf;

use via_num::vectors::{evaluate_file, null_digest, DELEGATED};

fn spec_vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors")
}

fn outcomes() -> Vec<via_num::vectors::Outcome> {
    let path = spec_vectors().join("numbers.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    evaluate_file(&text).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_vector_gives_its_bits() {
    let outcomes = outcomes();
    let mut failures = Vec::new();
    let mut evaluated = 0;
    for o in &outcomes {
        match &o.got {
            Some(got) => {
                evaluated += 1;
                if *got != o.expected {
                    failures.push(format!("{}: got {got}, expected {}", o.id, o.expected));
                }
            }
            None => assert!(
                DELEGATED.contains(&o.func.as_str()),
                "{}: unknown function `{}`",
                o.id,
                o.func
            ),
        }
    }
    assert!(
        failures.is_empty(),
        "{} failing vectors:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(evaluated >= 100, "only {evaluated} vectors evaluated");
}

#[test]
fn noise_gradient_indices_are_the_top_four_bits() {
    // formulas.md 8.2 tables each corner's draw with its gradient index; the index is checked from
    // the word, so a change to `>> 60` fails here.
    let want: [(&str, usize); 5] = [
        ("noise_draw_0_0_0", 4),
        ("noise_draw_1_0_0", 8),
        ("noise_draw_0_1_0", 15),
        ("noise_draw_minus_1_0_0", 3),
        ("noise_draw_0_0_1", 2),
    ];
    let outcomes = outcomes();
    for (id, index) in want {
        let o = outcomes
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("no vector {id}"));
        let hex = o.expected.trim_matches('"').trim_start_matches("0x");
        let word = u64::from_str_radix(hex, 16).unwrap();
        assert_eq!(via_num::gradient_index(word), index, "{id}");
    }
}

#[test]
fn null_digest_is_recorded() {
    let path = spec_vectors().join("num/null_digest.txt");
    let recorded =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(null_digest(&outcomes()), recorded.trim());
}
