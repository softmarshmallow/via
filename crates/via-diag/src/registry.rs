//! The registry of core codes (`spec/diagnostics.md` section 2).
//!
//! The rows live in `generated.rs`, generated from `spec/diagnostics.toml`; this module gives the
//! types and the lookups. A [`Code`] can only be had from a generated constant or from
//! [`lookup`], so the engine cannot raise an unregistered core code.

use crate::generated::ROWS;
use crate::Severity;

/// How a code is shown to be raised (section 2, `covered_by`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoveredBy {
    /// Some conformance case raises it.
    Conformance,
    /// A loader raises it; the loader's tests cover it.
    Reader,
    /// A named workspace test covers it (`internal` codes and a few run conditions).
    Workspace,
}

impl CoveredBy {
    /// The word written in the registry.
    pub const fn as_str(self) -> &'static str {
        match self {
            CoveredBy::Conformance => "conformance",
            CoveredBy::Reader => "reader",
            CoveredBy::Workspace => "workspace",
        }
    }
}

/// One registry row, as `spec/diagnostics.toml` holds it.
#[derive(Debug)]
pub struct Row {
    /// `group.name`.
    pub code: &'static str,
    /// The code's one severity.
    pub severity: Severity,
    /// The chapter whose Diagnostics table holds the row (file stem under `spec/`).
    pub chapter: &'static str,
    /// The milestone that first raises it.
    pub milestone: u32,
    /// One line: the chapter table's text.
    pub summary: &'static str,
    /// The names of the code's `data` fields.
    pub data: &'static [&'static str],
    /// The `data` field that ranks occurrences, largest first.
    pub worst: Option<&'static str>,
    /// How the code is shown to be raised.
    pub covered_by: CoveredBy,
    /// What it means, what to change, examples; what `via explain` prints.
    pub explain: &'static str,
}

impl Row {
    /// The group, the part before the dot.
    pub fn group(&self) -> &'static str {
        self.code.split_once('.').map_or(self.code, |(g, _)| g)
    }
}

/// A registered core code. Copyable and cheap; its row is static.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(u16);

impl Code {
    /// Only the generated constants call this, with an index into the sorted table.
    pub(crate) const fn from_index(i: usize) -> Code {
        Code(i as u16)
    }

    /// The registry row.
    pub fn row(self) -> &'static Row {
        &ROWS[usize::from(self.0)]
    }

    /// `group.name`.
    pub fn as_str(self) -> &'static str {
        self.row().code
    }

    /// The code's one severity.
    pub fn severity(self) -> Severity {
        self.row().severity
    }

    /// True for a defect of ours (a code whose name is `internal`; section 2 and 7: exit 3).
    pub fn is_internal(self) -> bool {
        is_internal(self.as_str())
    }
}

impl std::fmt::Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code({})", self.as_str())
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every row, sorted bytewise by code.
pub fn rows() -> &'static [Row] {
    &ROWS
}

/// The registered core code of that name, if there is one.
pub fn lookup(code: &str) -> Option<Code> {
    ROWS.binary_search_by(|r| r.code.cmp(code))
        .ok()
        .map(Code::from_index)
}

/// True for a code whose name part is `internal`, core or step.
pub fn is_internal(code: &str) -> bool {
    code.rsplit('.').next() == Some("internal")
}

/// Run diagnostics (`spec/build.md` 11.1, the one list): about this run of the engine, not the
/// world, so never written to `diagnostics.json`.
pub fn is_run_code(code: &str) -> bool {
    const RUN: &[&str] = &[
        "graph.build_file_restored",
        "graph.busy",
        "graph.step_unstable",
        "graph.store_corrupt",
        "graph.store_unavailable",
        "identity.no_previous",
        "identity.reselected",
        "protocol.slow_exit",
    ];
    if RUN.contains(&code) {
        return true;
    }
    // The chunk side and the budget are the world's (M6), as is a defect of ours.
    const WORLD_BUILD: &[&str] = &[
        "build.chunk_not_multiple",
        "build.internal",
        "build.over_budget",
        "build.waiver_unused",
    ];
    code.starts_with("build.") && !WORLD_BUILD.contains(&code)
}

/// True when `code` matches the form of section 2: `group.name`, or `step.<id>.<name>`, each part
/// `[a-z][a-z0-9_]*`. Registration is a separate question.
pub fn well_formed(code: &str) -> bool {
    let parts: Vec<&str> = code.split('.').collect();
    let ok = parts.iter().all(|p| ident(p));
    match parts.as_slice() {
        [g, _] => ok && *g != "step",
        ["step", _, _] => ok,
        _ => false,
    }
}

/// `[a-z][a-z0-9_]*`: the form of a code part, an id and a step's code name.
pub fn ident(p: &str) -> bool {
    let mut cs = p.chars();
    matches!(cs.next(), Some('a'..='z')) && cs.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}
