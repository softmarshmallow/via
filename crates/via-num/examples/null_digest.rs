//! Prints the null digest: one BLAKE3 hex line over every vector of `spec/vectors/numbers.toml`
//! that via-num evaluates, in the form `via vectors` prints them (`build.md` 14). Two systems that
//! print the same line computed every vector to the same bits; this is what the cross-system check
//! compares. With `--lines`, prints the hashed lines first.
//!
//! cargo run -p via-num --example null_digest [-- [--lines] [FILE]]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut lines = false;
    let mut file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/numbers.toml");
    for a in std::env::args().skip(1) {
        if a == "--lines" {
            lines = true;
        } else {
            file = PathBuf::from(a);
        }
    }
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", file.display());
            return ExitCode::from(2);
        }
    };
    let outcomes = match via_num::vectors::evaluate_file(&text) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    if lines {
        for line in outcomes.iter().filter_map(via_num::vectors::Outcome::line) {
            println!("{line}");
        }
    }
    println!("{}", via_num::vectors::null_digest(&outcomes));
    ExitCode::SUCCESS
}
