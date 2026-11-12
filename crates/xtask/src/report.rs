//! What a check returns: findings with a location, and notes that explain a skip or a scope.

use std::fmt;

/// One thing a check found wrong, located in a repository file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// Repository-relative path with `/` separators, so the report reads the same on Windows.
    pub path: String,
    /// 1-based; 0 when the finding is about the whole file.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.path, self.message)
        } else {
            write!(f, "{}:{}: {}", self.path, self.line, self.message)
        }
    }
}

/// The result of a check that could run. Findings are sorted before printing so the report is
/// the same bytes on every run.
#[derive(Debug, Default)]
pub struct Outcome {
    pub findings: Vec<Finding>,
    /// Said whether or not the check passed: what was skipped and why, what was covered.
    pub notes: Vec<String>,
    /// True when the check had nothing to run against; it neither passes nor fails.
    pub skipped: bool,
}

impl Outcome {
    pub fn finding(&mut self, path: impl Into<String>, line: usize, message: impl Into<String>) {
        self.findings.push(Finding {
            path: path.into(),
            line,
            message: message.into(),
        });
    }

    pub fn note(&mut self, s: impl Into<String>) {
        self.notes.push(s.into());
    }
}

/// A check that could not run at all (a missing file, a tool that failed). Reported as a
/// failure: a check that cannot run has not passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckError(pub String);

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<std::io::Error> for CheckError {
    fn from(e: std::io::Error) -> Self {
        CheckError(e.to_string())
    }
}

/// Reads a repository file as text, naming the file in the error.
pub fn read_text(path: &std::path::Path) -> Result<String, CheckError> {
    std::fs::read_to_string(path).map_err(|e| CheckError(format!("{}: {e}", path.display())))
}

/// A repository-relative display path with `/` separators.
pub fn rel(root: &std::path::Path, path: &std::path::Path) -> String {
    let p = path.strip_prefix(root).unwrap_or(path);
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
