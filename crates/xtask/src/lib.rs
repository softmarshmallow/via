//! Repository checks.
//!
//! Responsibility: the workspace tests that are not conformance: no domain noun in a core crate
//! (`spec/vocabulary.md` 7, from `spec/vocabulary.toml`), the vocabulary's machine copy equal to
//! its chapter (sections 3 and 7), the limitations test of R33, generated files fresh, and the
//! case checks of `spec/conformance.md` 7 that need only the manifests and the code registry
//! ([`cases`]: manifests read, no copy from `spike/` or outside the repository, code coverage).
//! They read the repository as text, so this crate depends on no engine crate.
//!
//! Not here: that every Conformance row has a case folder with an equal milestone (owed, recorded
//! in `docs/plan.md`); the function table against the evaluator (`spec/formulas.md` 8), which
//! `via-expr`'s own tests hold.
//!
//! Governed by `spec/conformance.md` section 7 and the chapters named above.
//!
//! `cargo run -p xtask -- check [--milestone N]` runs every check below and exits 1 if any fails.
//! The checks that need only the repository's text (deny-list, maths dependencies, vocabulary
//! sync, cases at M0, limitations, Python imports) are also `cargo test` tests in `tests/workspace.rs`, so `cargo test --workspace` holds them; the
//! freshness hooks (they run cargo) and the private-name guard (it needs untracked patterns) run
//! from the command only. Maths through `via-num` is held in two parts: the float methods by
//! clippy's `disallowed_methods` (`clippy.toml`), and "no crate but `via-num` depends on a maths
//! library" by [`mathdeps`] here.

pub mod cases;
pub mod denylist;
pub mod fresh;
pub mod keys;
pub mod lex;
pub mod limitations;
pub mod mathdeps;
pub mod private;
pub mod pyimports;
pub mod report;

use std::path::{Path, PathBuf};

use report::{CheckError, Outcome};

/// The repository root: this crate lives at `crates/xtask`.
pub fn repo_root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| here.to_path_buf())
}

/// Options of `xtask check`.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Names of checks to run; empty runs all.
    pub only: Vec<String>,
    /// Do not read `spike/local/private-patterns.txt`.
    pub no_local_patterns: bool,
    /// The milestone being worked on, as the conformance runner's `--milestone N`
    /// (`conformance.md` 4): the case checks hold codes up to it covered.
    pub milestone: u32,
}

type CheckFn = fn(&Path, &Options) -> Result<Outcome, CheckError>;

/// Every check, by name, in the order they run.
pub const CHECKS: [(&str, CheckFn); 8] = [
    ("deny-list", |r, _| denylist::check(r)),
    ("maths-deps", |r, _| mathdeps::check(r)),
    ("cases", |r, o| cases::check(r, o.milestone)),
    ("vocabulary-sync", |r, _| denylist::check_sync(r)),
    ("python-imports", |r, _| pyimports::check(r)),
    ("limitations", |r, _| limitations::check(r)),
    ("fresh", |r, _| fresh::check(r)),
    ("private-names", |r, o| {
        private::check(r, !o.no_local_patterns)
    }),
];

/// Runs the selected checks, printing a report; returns true when none failed.
pub fn run(root: &Path, opts: &Options, out: &mut dyn std::io::Write) -> std::io::Result<bool> {
    let mut all_ok = true;
    for (name, f) in CHECKS {
        if !opts.only.is_empty() && !opts.only.iter().any(|o| o == name) {
            continue;
        }
        match f(root, opts) {
            Ok(mut o) => {
                o.findings.sort();
                let status = if !o.findings.is_empty() {
                    all_ok = false;
                    format!("FAIL ({} findings)", o.findings.len())
                } else if o.skipped {
                    "SKIPPED".to_string()
                } else {
                    "ok".to_string()
                };
                writeln!(out, "{name}: {status}")?;
                for n in &o.notes {
                    writeln!(out, "  note: {n}")?;
                }
                for fd in &o.findings {
                    writeln!(out, "  {fd}")?;
                }
            }
            Err(e) => {
                all_ok = false;
                writeln!(out, "{name}: FAIL (could not run)\n  {e}")?;
            }
        }
    }
    Ok(all_ok)
}
