//! Workspace checks on the conformance cases (`spec/conformance.md` 7), the parts that need only
//! the case manifests and the code registry:
//!
//! - every `spec/cases/<chapter>/<case>/case.toml` reads as TOML with `format = "via-case-1"`, an
//!   integer `milestone` 0 to 7, a folder name in the case-id grammar and a chapter that exists
//!   (the runner's `list` validates the whole manifest; this is the floor `cargo test` holds);
//! - no case copies from `spike/`, and no `[copy.*]` path leaves the repository (absolute, or
//!   through `..`), and no case file is a symbolic link;
//! - `diagnostics.md`'s coverage check: every registered code up to the milestone with
//!   `covered_by = "conformance"` (the default) is expected by some case at or below that
//!   milestone, and every code a case expects is registered.
//!
//! Not here yet, and recorded as owed in `docs/plan.md`: that every Conformance row of every
//! chapter has a case folder and that the folder's `milestone` equals the row's. The chapters
//! write rows in three forms with milestones in prose, and the row-to-id mapping
//! (`conformance.md` 2) needs each chapter's row names settled first.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::report::{read_text, rel, CheckError, Outcome};

pub const CASES: &str = "spec/cases";
pub const REGISTRY: &str = "spec/diagnostics.toml";
const FORMAT: &str = "via-case-1";

/// The case-id grammar of `conformance.md` 2: `[a-z][a-z0-9_]*`, at most 48 bytes.
pub fn is_case_id(s: &str) -> bool {
    s.len() <= 48
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A copy path that stays inside the repository and out of `spike/`; the reason when it does not.
pub fn bad_copy_path(p: &str, is_from: bool) -> Option<&'static str> {
    let path = Path::new(p);
    if path.is_absolute() || p.starts_with('/') || p.starts_with('\\') || p.contains(':') {
        return Some("is absolute; a case names repository paths only");
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Some("goes through `..`; a case names repository paths only");
    }
    let first = p
        .trim_start_matches("./")
        .split(['/', '\\'])
        .next()
        .unwrap_or("");
    if is_from && first == "spike" {
        return Some("copies from spike/; a case never does (conformance.md 3.2)");
    }
    None
}

/// One case as this check reads it.
#[derive(Debug)]
struct Case {
    path: String,
    milestone: u32,
    codes: BTreeSet<String>,
}

fn case_dirs(root: &Path) -> Result<Vec<(String, PathBuf)>, CheckError> {
    let base = root.join(CASES);
    let mut out = Vec::new();
    let mut chapters: Vec<PathBuf> = std::fs::read_dir(&base)
        .map_err(|e| CheckError(format!("{CASES}: {e}")))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    chapters.sort();
    for ch in chapters {
        let name = ch
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut cases: Vec<PathBuf> = std::fs::read_dir(&ch)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        cases.sort();
        for c in cases {
            out.push((name.clone(), c));
        }
    }
    Ok(out)
}

/// The codes a manifest's `[action.<a>.diag.<id>]` tables expect.
fn expected_codes(doc: &toml::Table) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let actions = doc.get("action").and_then(|v| v.as_table());
    for a in actions.into_iter().flat_map(|t| t.values()) {
        let diags = a.get("diag").and_then(|v| v.as_table());
        for d in diags.into_iter().flat_map(|t| t.values()) {
            if let Some(c) = d.get("code").and_then(|v| v.as_str()) {
                out.insert(c.to_string());
            }
        }
    }
    out
}

/// Every file under `dir` that is a symbolic link, sorted.
fn links(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            match std::fs::symlink_metadata(&p) {
                Ok(m) if m.file_type().is_symlink() => out.push(p),
                Ok(m) if m.is_dir() => stack.push(p),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

fn read_case(root: &Path, chapter: &str, dir: &Path, o: &mut Outcome) -> Option<Case> {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let manifest = dir.join("case.toml");
    let path = rel(root, &manifest);
    if !is_case_id(&id) {
        o.finding(
            rel(root, dir),
            0,
            format!("`{id}` is not a case id ([a-z][a-z0-9_]*, at most 48 bytes)"),
        );
    }
    if !root.join("spec").join(format!("{chapter}.md")).is_file() {
        o.finding(
            rel(root, dir),
            0,
            format!("no chapter spec/{chapter}.md for this case"),
        );
    }
    for l in links(dir) {
        o.finding(
            rel(root, &l),
            0,
            "a case file is a symbolic link; a case holds its inputs or copies them",
        );
    }
    let text = match read_text(&manifest) {
        Ok(t) => t,
        Err(e) => {
            o.finding(rel(root, dir), 0, format!("no readable case.toml: {e}"));
            return None;
        }
    };
    let doc: toml::Table = match toml::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            o.finding(path, 0, format!("not TOML: {e}"));
            return None;
        }
    };
    if doc.get("format").and_then(|v| v.as_str()) != Some(FORMAT) {
        o.finding(path.clone(), 0, format!("`format` is not \"{FORMAT}\""));
    }
    let milestone = match doc.get("milestone").and_then(|v| v.as_integer()) {
        Some(m @ 0..=7) => m as u32,
        _ => {
            o.finding(path, 0, "`milestone` is not an integer 0 to 7");
            return None;
        }
    };
    let copies = doc.get("copy").and_then(|v| v.as_table());
    for (cid, c) in copies.into_iter().flat_map(|t| t.iter()) {
        for (key, is_from) in [("from", true), ("to", false)] {
            if let Some(p) = c.get(key).and_then(|v| v.as_str()) {
                if let Some(why) = bad_copy_path(p, is_from) {
                    o.finding(
                        path.clone(),
                        0,
                        format!("[copy.{cid}].{key} = \"{p}\" {why}"),
                    );
                }
            }
        }
    }
    Some(Case {
        path,
        milestone,
        codes: expected_codes(&doc),
    })
}

/// (code, milestone, covered_by) of every registered code, sorted by code.
fn registry(root: &Path) -> Result<Vec<(String, u32, String)>, CheckError> {
    let text = read_text(&root.join(REGISTRY))?;
    let doc: toml::Table =
        toml::from_str(&text).map_err(|e| CheckError(format!("{REGISTRY}: {e}")))?;
    let mut out = Vec::new();
    // Flat TOML keyed by code (R1): `[group.name]` is a table `group` holding `name`.
    for (group, t) in &doc {
        let Some(t) = t.as_table() else { continue };
        for (name, row) in t {
            let Some(row) = row.as_table() else { continue };
            let code = format!("{group}.{name}");
            let milestone = row
                .get("milestone")
                .and_then(|v| v.as_integer())
                .and_then(|m| u32::try_from(m).ok())
                .ok_or_else(|| {
                    CheckError(format!("{REGISTRY}: [{code}] has no integer milestone"))
                })?;
            let covered = row
                .get("covered_by")
                .and_then(|v| v.as_str())
                .unwrap_or("conformance")
                .to_string();
            out.push((code, milestone, covered));
        }
    }
    out.sort();
    Ok(out)
}

/// Section 7's checks at `milestone`, the milestone being worked on (the runner's `--milestone`).
pub fn check(root: &Path, milestone: u32) -> Result<Outcome, CheckError> {
    let mut o = Outcome::default();
    let mut cases = Vec::new();
    for (chapter, dir) in case_dirs(root)? {
        if let Some(c) = read_case(root, &chapter, &dir, &mut o) {
            cases.push(c);
        }
    }
    let reg = registry(root)?;
    let known: BTreeSet<&str> = reg.iter().map(|(c, _, _)| c.as_str()).collect();
    // A step's own codes (`step.<id>.<name>`) are declared by the step, not the registry.
    for c in &cases {
        for code in &c.codes {
            if !code.starts_with("step.") && !known.contains(code.as_str()) {
                o.finding(
                    c.path.clone(),
                    0,
                    format!("expects `{code}`, which {REGISTRY} does not register"),
                );
            }
        }
    }
    // The lowest case milestone at which each code is expected.
    let mut first: BTreeMap<&str, u32> = BTreeMap::new();
    for c in &cases {
        for code in &c.codes {
            let e = first.entry(code.as_str()).or_insert(c.milestone);
            *e = (*e).min(c.milestone);
        }
    }
    let mut owed = 0;
    for (code, m, covered) in &reg {
        if covered != "conformance" || *m > milestone {
            continue;
        }
        owed += 1;
        match first.get(code.as_str()) {
            Some(at) if *at <= milestone => {}
            _ => o.finding(
                REGISTRY,
                0,
                format!("`{code}` (M{m}, covered_by conformance) is expected by no case at or below M{milestone}"),
            ),
        }
    }
    o.note(format!(
        "{} cases read at milestone {milestone}; {owed} conformance codes due, {} codes expected by cases",
        cases.len(),
        first.len()
    ));
    o.note("owed (docs/plan.md): every Conformance row has a case folder with an equal milestone");
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_ids() {
        assert!(is_case_id("c01_null"));
        assert!(is_case_id("pack_swap"));
        assert!(!is_case_id("C01"));
        assert!(!is_case_id("1abc"));
        assert!(!is_case_id(&"a".repeat(49)));
    }

    #[test]
    fn copy_paths() {
        assert_eq!(bad_copy_path("lib/ways/kinds.toml", true), None);
        assert_eq!(bad_copy_path(".", false), None);
        assert!(bad_copy_path("spike/out/x", true).is_some());
        assert!(bad_copy_path("./spike/x", true).is_some());
        assert!(bad_copy_path("/etc/passwd", true).is_some());
        assert!(bad_copy_path("lib/../../x", true).is_some());
        assert!(bad_copy_path("C:/x", true).is_some());
        // `to` names the working folder, so a folder called spike there is not a copy from it.
        assert_eq!(bad_copy_path("spike/x", false), None);
    }

    #[test]
    fn reads_expected_codes() {
        let doc: toml::Table = toml::from_str(
            "[action.a.diag.x]\ncode = \"formula.unknown_name\"\n[action.b.diag.y]\ncode = \"diag.skipped\"\n[action.b.file.z]\npath = \"p\"\n",
        )
        .expect("toml");
        let got: Vec<String> = expected_codes(&doc).into_iter().collect();
        assert_eq!(got, vec!["diag.skipped", "formula.unknown_name"]);
    }
}
