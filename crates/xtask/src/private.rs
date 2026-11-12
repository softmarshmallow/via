//! The private-name guard (CONTRIBUTING.md, "Never commit private material"; plan, "the
//! private-name check before every commit").
//!
//! The words searched for are themselves private, so they are never in the repository: they come
//! from the untracked `spike/local/private-patterns.txt` (one per line, `#` starts a comment) and
//! from `VIA_PRIVATE_PATTERNS` (separated by commas or newlines). Every file git would commit is
//! searched: tracked files and untracked files not ignored, so a new file is caught before its
//! first `git add`. A match is reported by pattern number and place only; neither the word nor the
//! line is printed, so the report itself can be pasted anywhere.

use std::path::Path;
use std::process::Command;

use crate::report::{CheckError, Outcome};

pub const LOCAL_PATTERNS: &str = "spike/local/private-patterns.txt";
pub const ENV_PATTERNS: &str = "VIA_PRIVATE_PATTERNS";
/// Folders that hold private material by rule; never searched, even if something there were
/// tracked by mistake (then the commit itself is the error, which git status shows).
const PRIVATE_DIRS: [&str; 3] = ["spike/local/", "spike/out/", "spike/build/"];

/// A pattern with the label it is reported by (`local#2`, `env#1`), lowercased for an
/// ASCII-case-insensitive search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub label: String,
    pub needle: Vec<u8>,
}

pub fn parse_patterns(text: &str, source: &str, split_commas: bool) -> Vec<Pattern> {
    let mut out = Vec::new();
    let pieces: Vec<&str> = if split_commas {
        text.split([',', '\n']).collect()
    } else {
        text.lines().collect()
    };
    for p in pieces {
        let p = p.trim();
        if p.is_empty() || p.starts_with('#') {
            continue;
        }
        out.push(Pattern {
            label: format!("{source}#{}", out.len() + 1),
            needle: p.as_bytes().to_ascii_lowercase(),
        });
    }
    out
}

/// Lines (1-based) of `hay` holding `needle`, case-insensitively for ASCII.
pub fn match_lines(hay: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return Vec::new();
    }
    let lower = hay.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut line = 1;
    let mut last_line = 0;
    for i in 0..=lower.len() - needle.len() {
        if i > 0 && lower[i - 1] == b'\n' {
            line += 1;
        }
        if lower[i..].starts_with(needle) && line != last_line {
            out.push(line);
            last_line = line;
        }
    }
    out
}

fn files_git_would_commit(root: &Path) -> Result<Vec<String>, CheckError> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| CheckError(format!("cannot run git ls-files: {e}")))?;
    if !out.status.success() {
        return Err(CheckError(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let mut files: Vec<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// `use_local` is false when the caller must not read `spike/local/` (an agent's run); the
/// environment's patterns are still used.
pub fn check(root: &Path, use_local: bool) -> Result<Outcome, CheckError> {
    let mut o = Outcome::default();
    let mut patterns = Vec::new();
    let local = root.join(LOCAL_PATTERNS);
    if !use_local {
        o.note(format!("{LOCAL_PATTERNS} not read (--no-local-patterns)"));
    } else if local.is_file() {
        let text = std::fs::read_to_string(&local)
            .map_err(|e| CheckError(format!("{LOCAL_PATTERNS}: {e}")))?;
        patterns.extend(parse_patterns(&text, "local", false));
    } else {
        o.note(format!("{LOCAL_PATTERNS} does not exist"));
    }
    if let Ok(text) = std::env::var(ENV_PATTERNS) {
        patterns.extend(parse_patterns(&text, "env", true));
    }
    if patterns.is_empty() {
        o.skipped = true;
        o.note(format!(
            "no patterns: write them to {LOCAL_PATTERNS} (untracked) or set {ENV_PATTERNS}; \
             nothing was searched"
        ));
        return Ok(o);
    }
    let files = files_git_would_commit(root)?;
    let mut searched = 0;
    for f in &files {
        if PRIVATE_DIRS.iter().any(|d| f.starts_with(d)) {
            continue;
        }
        for p in &patterns {
            if !match_lines(f.as_bytes(), &p.needle).is_empty() {
                o.finding(
                    f.clone(),
                    0,
                    format!("private pattern {} in the file's path", p.label),
                );
            }
        }
        let Ok(bytes) = std::fs::read(root.join(f)) else {
            // Listed by git but gone from disk: a deletion not yet staged; nothing to commit.
            continue;
        };
        searched += 1;
        for p in &patterns {
            for line in match_lines(&bytes, &p.needle) {
                o.finding(f.clone(), line, format!("private pattern {}", p.label));
            }
        }
    }
    o.note(format!(
        "{} patterns over {searched} files git would commit",
        patterns.len()
    ));
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_matches_case_insensitively() {
        let p = parse_patterns("# comment\nAcme Ltd\n\n  zeta  \n", "local", false);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].label, "local#1");
        assert_eq!(p[0].needle, b"acme ltd");
        let e = parse_patterns("one, two\nthree", "env", true);
        assert_eq!(e.len(), 3);
        assert_eq!(
            match_lines(b"x\nan ACME LTD row acme ltd\n\nacme ltd", b"acme ltd"),
            vec![2, 4]
        );
        assert!(match_lines(b"acm", b"acme").is_empty());
    }
}
