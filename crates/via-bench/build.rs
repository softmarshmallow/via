//! Embed the git revision so every emitted number can carry it (ADR 0008
//! D11: protocol identifier, code revision, snapshot date, seed set).
//!
//! The script re-runs on EVERY build (the nonexistent rerun-if-changed
//! path below). Two staleness bugs forced this: watching `.git/HEAD`
//! alone replayed a stale revision after a same-branch commit, and
//! watching HEAD/index/packed-refs still replayed a stale `+dirty` flag
//! when untracked files appeared or disappeared — nothing in .git moves
//! on that transition (the corpus run reproduced it). A `git status`
//! per build costs ~10 ms, and rustc only recompiles when the stamp's
//! value actually changes.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn main() {
    let rev = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git(&["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let rev = if dirty { format!("{rev}+dirty") } else { rev };
    println!("cargo:rustc-env=VIA_BENCH_GIT_REV={rev}");

    // A path that never exists: Cargo treats it as always-changed, so
    // this script re-runs on every build and the stamp can never go
    // stale, whatever mutated the working tree.
    println!("cargo:rerun-if-changed=.force-git-rev-rerun");
}
