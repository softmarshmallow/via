//! `via explain` (`spec/diagnostics.md` sections 2 and 7), as library functions the CLI calls.
//!
//! - `via explain` with no code: [`list_text`].
//! - `via explain CODE`: [`lookup`], then [`text`] or, with `--json`, [`json`].
//! - `via explain step.<id>.<name> WORLD`: [`lookup`] with the step codes recorded in the last
//!   build of `WORLD`; it runs nothing and does not merge the world.
//!
//! An unknown code is `cli.unknown_code` (exit 2) with the nearest code; the search covers the
//! core codes and the given step codes.

use crate::collect::unknown_code;
use crate::diag::Diag;
use crate::json::Json;
use crate::registry::{self, Row};
use crate::Severity;
use std::fmt::Write as _;

/// A step code recorded in a build: its severity and the `explain` text the step sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepCode {
    /// `step.<id>.<name>`.
    pub code: String,
    /// Its one severity in that build.
    pub severity: Severity,
    /// The text the step sent, if it sent one.
    pub explain: Option<String>,
}

/// What `via explain` found.
#[derive(Clone, Copy, Debug)]
pub enum Explained<'a> {
    /// A core code's registry row.
    Core(&'static Row),
    /// A step code from the last build.
    Step(&'a StepCode),
}

/// Finds `code` among the core codes and `steps`; otherwise `cli.unknown_code` at argument
/// `argument` (1-based, after the program name), with the nearest code.
pub fn lookup<'a>(code: &str, steps: &'a [StepCode], argument: u32) -> Result<Explained<'a>, Diag> {
    if let Some(c) = registry::lookup(code) {
        return Ok(Explained::Core(c.row()));
    }
    if let Some(s) = steps.iter().find(|s| s.code == code) {
        return Ok(Explained::Step(s));
    }
    let known = registry::rows()
        .iter()
        .map(|r| r.code)
        .chain(steps.iter().map(|s| s.code.as_str()));
    Err(unknown_code(code, argument, known))
}

/// The text `via explain CODE` prints: code, severity, chapter, summary and `explain`.
pub fn text(e: Explained<'_>) -> String {
    match e {
        Explained::Core(r) => {
            let mut s = format!(
                "{} ({}, spec/{}.md, milestone {})\n{}\n\n{}",
                r.code, r.severity, r.chapter, r.milestone, r.summary, r.explain
            );
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s
        }
        Explained::Step(sc) => {
            let body = sc
                .explain
                .as_deref()
                .unwrap_or("The step sent no explain text for this code.\n");
            let mut s = format!(
                "{} ({}, a step's own code)\n\n{}",
                sc.code, sc.severity, body
            );
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s
        }
    }
}

/// `via explain CODE --json`: the registry row as one canonical JSON object and a line feed. A
/// step code gives `code`, `severity` and `explain` (with `step: true`).
pub fn json(e: Explained<'_>) -> String {
    let mut o = Json::obj();
    match e {
        Explained::Core(r) => {
            o.insert("code".into(), Json::Str(r.code.into()));
            o.insert("severity".into(), Json::Str(r.severity.as_str().into()));
            o.insert("chapter".into(), Json::Str(r.chapter.into()));
            o.insert("milestone".into(), Json::Int(i64::from(r.milestone)));
            o.insert("summary".into(), Json::Str(r.summary.into()));
            o.insert(
                "data".into(),
                Json::Arr(r.data.iter().map(|d| Json::Str((*d).into())).collect()),
            );
            if let Some(w) = r.worst {
                o.insert("worst".into(), Json::Str(w.into()));
            }
            o.insert("covered_by".into(), Json::Str(r.covered_by.as_str().into()));
            o.insert("explain".into(), Json::Str(r.explain.into()));
        }
        Explained::Step(sc) => {
            o.insert("code".into(), Json::Str(sc.code.clone()));
            o.insert("severity".into(), Json::Str(sc.severity.as_str().into()));
            o.insert("step".into(), Json::Bool(true));
            if let Some(e) = &sc.explain {
                o.insert("explain".into(), Json::Str(e.clone()));
            }
        }
    }
    let mut s = Json::Obj(o).to_canonical();
    s.push('\n');
    s
}

/// `via explain` with no code: every core code by group, one line each.
pub fn list_text() -> String {
    let mut s = String::new();
    let mut group = "";
    for r in registry::rows() {
        if r.group() != group {
            group = r.group();
            let _ = writeln!(s, "\n{group} (spec/{}.md)", r.chapter);
        }
        let _ = writeln!(
            s,
            "  {:<34} {:<8} {}",
            r.code,
            r.severity.as_str(),
            first_sentence(r.summary)
        );
    }
    s.trim_start_matches('\n').to_string()
}

fn first_sentence(s: &str) -> &str {
    match s.find(". ") {
        Some(i) => &s[..=i],
        None => s,
    }
}
