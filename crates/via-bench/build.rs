//! Embed the git revision so every emitted number can carry it (ADR 0008
//! D11: protocol identifier, code revision, snapshot date, seed set).
//!
//! The rerun triggers watch the files that actually change on a commit:
//! `.git/HEAD` only moves on checkout, so watching it alone replays a
//! stale revision after an ordinary same-branch commit (the review
//! reproduced provenance JSONs carrying the previous commit's hash plus a
//! stale `+dirty` flag). We resolve the git dir (worktree-safe) and watch
//! HEAD, the current branch ref, packed-refs, and the index.

use std::path::PathBuf;
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

    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        let git_dir = PathBuf::from(git_dir);
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join("packed-refs").display()
        );
        if let Some(head) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(head.trim_start_matches('/')).display()
            );
        }
    }
}
