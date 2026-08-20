//! Embed the git revision so every emitted number can carry it (ADR 0008
//! D11: protocol identifier, code revision, snapshot date, seed set).

use std::process::Command;

fn main() {
    let rev = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    let rev = if dirty { format!("{rev}+dirty") } else { rev };
    println!("cargo:rustc-env=VIA_BENCH_GIT_REV={rev}");
    // Re-run when HEAD moves; the +dirty flag is best-effort.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
