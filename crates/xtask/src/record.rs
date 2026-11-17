//! The record's checks: the ledger (`docs/ledger/`) and the layers page (`docs/layers.md`).
//!
//! Each is held by a script under `tools/` that reads the repository with Python's standard
//! library alone, run through uv as the import check is ([`crate::pyimports`]):
//!
//! - `tools/ledger_check.py`: every row of the ledger has an archetype and marks from their
//!   lists, every pointer of a `Now` names something that exists, no term is written twice, and
//!   the counts are current (`docs/ledger/README.md`, "The row").
//! - `tools/layers_check.py`: every crate, chapter and library has a place, a crate depends only
//!   inward, a library names only the kinds of those above it, the engine's form of each
//!   archetype points at things that exist, and section 25 of the ledger equals `lib/`
//!   (`docs/layers.md`, "Checks").
//!
//! A script prints one finding to a line, `path:line: message` or `path: message`, then a last
//! line that counts them, and exits 1 when there is any. This side runs it and turns what it
//! printed into findings.

use std::path::Path;
use std::process::Command;

use crate::report::{CheckError, Outcome};

pub const LEDGER: &str = "tools/ledger_check.py";
pub const LAYERS: &str = "tools/layers_check.py";
/// The interpreter uv runs, as the import check's; overridden by `VIA_CHECK_PYTHON`.
const DEFAULT_PYTHON: &str = "3.12";

pub fn check(root: &Path, script: &str) -> Result<Outcome, CheckError> {
    if !root.join(script).is_file() {
        return Err(CheckError(format!("{script} is missing")));
    }
    let python = std::env::var("VIA_CHECK_PYTHON").unwrap_or_else(|_| DEFAULT_PYTHON.to_string());
    let output = Command::new("uv")
        .args([
            "run",
            "--no-project",
            "--quiet",
            "--python",
            &python,
            "python",
            "-I",
        ])
        .arg(root.join(script))
        .current_dir(root)
        .output()
        .map_err(|e| CheckError(format!("cannot run uv (needed for {script}): {e}")))?;
    // 0: clean. 1: findings, each on a line. Anything else: the script itself failed.
    if !matches!(output.status.code(), Some(0) | Some(1)) {
        return Err(CheckError(format!(
            "{script} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    let Some(summary) = lines.pop() else {
        return Err(CheckError(format!("{script} printed nothing")));
    };
    let mut o = Outcome::default();
    for line in lines {
        let (path, at, message) = located(script, line);
        o.finding(path, at, message);
    }
    if output.status.code() == Some(1) && o.findings.is_empty() {
        o.finding(
            script,
            0,
            format!("exited 1 and named no finding: {summary}"),
        );
    }
    o.note(summary.to_string());
    Ok(o)
}

/// `path:line: message` or `path: message` as (path, line or 0, message). A line that starts
/// with no path is a finding about the script's subject as a whole.
fn located(script: &str, line: &str) -> (String, usize, String) {
    let Some((place, message)) = line.split_once(": ") else {
        return (script.to_string(), 0, line.to_string());
    };
    if place.contains(' ') {
        return (script.to_string(), 0, line.to_string());
    }
    match place.rsplit_once(':') {
        Some((path, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => (
            path.to_string(),
            n.parse().unwrap_or(0),
            message.to_string(),
        ),
        _ => (place.to_string(), 0, message.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::located;

    #[test]
    fn a_finding_line_is_split_at_its_place() {
        assert_eq!(
            located(
                "tools/ledger_check.py",
                "docs/ledger/built.md:15: 'kerb, curb': Now is none"
            ),
            (
                "docs/ledger/built.md".to_string(),
                15,
                "'kerb, curb': Now is none".to_string()
            )
        );
        assert_eq!(
            located(
                "tools/layers_check.py",
                "docs/layers.md: lib/water has no row"
            ),
            (
                "docs/layers.md".to_string(),
                0,
                "lib/water has no row".to_string()
            )
        );
        assert_eq!(
            located(
                "tools/layers_check.py",
                "crates/via-model (model) depends on via-proto (door): outward or sideways"
            ),
            (
                "tools/layers_check.py".to_string(),
                0,
                "crates/via-model (model) depends on via-proto (door): outward or sideways"
                    .to_string()
            )
        );
    }
}
