//! No crate but `via-num` depends on a maths library (`spec/numbers.md` 8, R26).
//!
//! clippy's `disallowed_methods` holds the float methods; it cannot see a crate that calls a
//! maths library's functions directly (`libm::sin`), which would bypass the one pinned, soft-float
//! copy just as surely. So this check reads every workspace member's `Cargo.toml` and fails when a
//! crate other than `via-num` declares a known maths crate, under any dependency table, target
//! table or `package =` rename. It sees direct dependencies only: a maths crate pulled in by some
//! other dependency is review's to catch, since it is not callable without being declared.

use std::path::Path;

use crate::report::{read_text, rel, CheckError, Outcome};

/// Crates that compute transcendentals or wrap them. A new one is added when a review meets it.
pub const MATHS_CRATES: [&str; 9] = [
    "libm",
    "num-traits",
    "num",
    "micromath",
    "fastapprox",
    "statrs",
    "nalgebra",
    "glam",
    "sleef",
];

/// The one crate allowed to name them.
pub const OWNER: &str = "via-num";

const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// The crate a dependency entry resolves to: its `package` when renamed, else its key. A
/// `workspace = true` entry takes the root's `package` rename, if any.
fn resolved(key: &str, value: &toml::Value, root_deps: Option<&toml::Table>) -> String {
    let table = value.as_table();
    if let Some(p) = table
        .and_then(|t| t.get("package"))
        .and_then(|v| v.as_str())
    {
        return p.to_string();
    }
    let from_workspace = table
        .and_then(|t| t.get("workspace"))
        .and_then(|v| v.as_bool())
        == Some(true);
    if from_workspace {
        if let Some(p) = root_deps
            .and_then(|d| d.get(key))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("package"))
            .and_then(|v| v.as_str())
        {
            return p.to_string();
        }
    }
    key.to_string()
}

/// Every dependency table of a manifest, plain and per target, with its dotted name for reports.
fn dep_tables(doc: &toml::Table) -> Vec<(String, &toml::Table)> {
    let mut out = Vec::new();
    for name in TABLES {
        if let Some(t) = doc.get(name).and_then(|v| v.as_table()) {
            out.push((name.to_string(), t));
        }
    }
    if let Some(targets) = doc.get("target").and_then(|v| v.as_table()) {
        for (cfg, body) in targets {
            for name in TABLES {
                if let Some(t) = body
                    .as_table()
                    .and_then(|b| b.get(name))
                    .and_then(|v| v.as_table())
                {
                    out.push((format!("target.{cfg}.{name}"), t));
                }
            }
        }
    }
    out
}

/// The 1-based line of the first line of `text` that starts with `key` followed by space or `=`;
/// 0 when not found, so the finding names the whole file.
fn line_of(text: &str, key: &str) -> usize {
    text.lines()
        .position(|l| {
            l.trim_start()
                .strip_prefix(key)
                .is_some_and(|rest| rest.starts_with([' ', '=', '.']))
        })
        .map_or(0, |i| i + 1)
}

/// Checks the manifest at `path` (crate `name`) and adds a finding per maths dependency.
pub fn check_manifest(
    root: &Path,
    path: &Path,
    root_deps: Option<&toml::Table>,
    out: &mut Outcome,
) -> Result<(), CheckError> {
    let text = read_text(path)?;
    let doc: toml::Table = text
        .parse()
        .map_err(|e| CheckError(format!("{}: {e}", rel(root, path))))?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if name == OWNER {
        return Ok(());
    }
    for (table, deps) in dep_tables(&doc) {
        for (key, value) in deps {
            let krate = resolved(key, value, root_deps);
            if MATHS_CRATES.contains(&krate.as_str()) {
                out.finding(
                    rel(root, path),
                    line_of(&text, key),
                    format!(
                        "`{name}` depends on the maths crate `{krate}` ([{table}]); only \
                         `{OWNER}` may (numbers.md 8)"
                    ),
                );
            }
        }
    }
    Ok(())
}

/// Scans `Cargo.toml` (its own dependency tables and `[workspace.dependencies]` are not crates,
/// so only the members are judged) and every `crates/*/Cargo.toml`, in sorted order.
pub fn check(root: &Path) -> Result<Outcome, CheckError> {
    let root_text = read_text(&root.join("Cargo.toml"))?;
    let root_doc: toml::Table = root_text
        .parse()
        .map_err(|e| CheckError(format!("Cargo.toml: {e}")))?;
    let root_deps = root_doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|v| v.as_table());

    let mut manifests = Vec::new();
    for entry in std::fs::read_dir(root.join("crates"))? {
        let p = entry?.path().join("Cargo.toml");
        if p.is_file() {
            manifests.push(p);
        }
    }
    manifests.sort();

    let mut out = Outcome::default();
    for p in &manifests {
        check_manifest(root, p, root_deps, &mut out)?;
    }
    out.note(format!(
        "{} crate manifests read; maths crates: {}",
        manifests.len(),
        MATHS_CRATES.join(", ")
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(manifest: &str, root_deps: Option<&toml::Table>) -> Vec<String> {
        // Tests run in parallel threads, so each manifest gets its own file.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("xtask-mathdeps-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join(format!("{n}.toml"));
        std::fs::write(&p, manifest).unwrap();
        let mut out = Outcome::default();
        check_manifest(&dir, &p, root_deps, &mut out).unwrap();
        let _ = std::fs::remove_file(&p);
        out.findings.iter().map(|f| f.to_string()).collect()
    }

    #[test]
    fn flags_libm_outside_via_num() {
        let f = findings(
            "[package]\nname = \"via-geom\"\n[dependencies]\nlibm = \"0.2\"\n",
            None,
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains(":4:") && f[0].contains("`libm`"), "{f:?}");
    }

    #[test]
    fn allows_via_num() {
        let f = findings(
            "[package]\nname = \"via-num\"\n[dependencies]\nlibm = \"0.2\"\n",
            None,
        );
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn sees_renames_and_targets() {
        let f = findings(
            "[package]\nname = \"via-raster\"\n[target.'cfg(unix)'.dev-dependencies]\n\
             m = { package = \"libm\", version = \"0.2\" }\n",
            None,
        );
        assert_eq!(f.len(), 1, "{f:?}");
        let root: toml::Table = "nt = { package = \"num-traits\", version = \"0.2\" }"
            .parse()
            .unwrap();
        let f = findings(
            "[package]\nname = \"via-expr\"\n[dependencies]\nnt = { workspace = true }\n",
            Some(&root),
        );
        assert_eq!(f.len(), 1, "{f:?}");
    }
}
