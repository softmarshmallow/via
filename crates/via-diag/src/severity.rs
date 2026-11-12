//! Severity (`spec/diagnostics.md` section 3).

/// A code's one severity. The declaration order is the sort order of section 5 (error first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The text cannot be read, names nothing or contradicts itself; no outputs are written.
    Error,
    /// The build is made, but something gave way that the author may not expect.
    Warning,
    /// A fact; never asks for action.
    Info,
}

impl Severity {
    /// The word written in JSON and the registry.
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// Reads the word; anything else is `None`.
    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            _ => None,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
