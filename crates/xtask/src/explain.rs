//! The registry's `explain` texts are finished (`spec/diagnostics.md` 2): every code up to the
//! milestone (M1 at least) has an `explain` holding a `To fix:` line, a `Bad:` line and a
//! `Fixed:` line, each at the start of a line. `via explain CODE` prints that text; it is the long
//! form of a failure's hint, so a code whose explain repeats its summary fails here.
//!
//! The generator (`crates/via-diag/tools/gen.rs`) seeds a new code's explain from its summary and
//! hint; this check is what stops such a stub from shipping.

use std::path::Path;

use crate::cases::REGISTRY;
use crate::report::{read_text, CheckError, Outcome};

/// The lines an explain text must hold, each at the start of a line.
pub const REQUIRED: [&str; 3] = ["To fix:", "Bad:", "Fixed:"];

/// The required line starts `explain` lacks, in [`REQUIRED`]'s order.
pub fn missing(explain: &str) -> Vec<&'static str> {
    REQUIRED
        .iter()
        .copied()
        .filter(|p| !explain.lines().any(|l| l.starts_with(p)))
        .collect()
}

/// Checks every code of `spec/diagnostics.toml` whose `milestone` is at or below
/// `max(milestone, 1)`.
pub fn check(root: &Path, milestone: u32) -> Result<Outcome, CheckError> {
    let text = read_text(&root.join(REGISTRY))?;
    check_text(&text, milestone)
}

/// [`check`] on the registry's text.
pub fn check_text(text: &str, milestone: u32) -> Result<Outcome, CheckError> {
    let upto = milestone.max(1);
    let table: toml::Table = text
        .parse()
        .map_err(|e| CheckError(format!("{REGISTRY}: {e}")))?;
    let mut out = Outcome::default();
    let mut checked = 0usize;
    for (group, codes) in &table {
        let Some(codes) = codes.as_table() else {
            out.finding(REGISTRY, 0, format!("`{group}` is not a table of codes"));
            continue;
        };
        for (name, row) in codes {
            let code = format!("{group}.{name}");
            let m = row
                .get("milestone")
                .and_then(|v| v.as_integer())
                .unwrap_or(1);
            if m < 0 || m > i64::from(upto) {
                continue;
            }
            checked += 1;
            let line = header_line(text, &code);
            let explain = row.get("explain").and_then(|v| v.as_str()).unwrap_or("");
            let lacks = missing(explain);
            if !lacks.is_empty() {
                out.finding(
                    REGISTRY,
                    line,
                    format!(
                        "`{code}` (M{m}): explain lacks a line starting {}; write what to change and one bad and one fixed example (diagnostics.md 2)",
                        lacks.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join(", ")
                    ),
                );
            }
        }
    }
    out.note(format!("{checked} codes up to M{upto} checked"));
    Ok(out)
}

/// The 1-based line of `[code]` in the registry, or 0 when it is not written that way.
fn header_line(text: &str, code: &str) -> usize {
    let header = format!("[{code}]");
    text.lines()
        .position(|l| l.trim() == header)
        .map_or(0, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stub_fails_and_a_finished_text_passes() {
        let reg = "[a.stub]\nseverity = \"error\"\nmilestone = 1\nexplain = \"\"\"\nA thing.\n\"\"\"\n\n\
                   [a.done]\nseverity = \"error\"\nmilestone = 1\nexplain = \"\"\"\nA thing.\nTo fix: Do b.\nBad:   x = 1\nFixed: x = 2\n\"\"\"\n\n\
                   [a.later]\nseverity = \"error\"\nmilestone = 3\nexplain = \"\"\"\nA thing.\n\"\"\"\n";
        let o = check_text(reg, 0).unwrap();
        assert_eq!(o.findings.len(), 1, "{:?}", o.findings);
        assert_eq!(o.findings[0].line, 1);
        assert!(o.findings[0].message.contains("`a.stub`"));
        let o3 = check_text(reg, 3).unwrap();
        assert_eq!(o3.findings.len(), 2);
    }

    #[test]
    fn the_lines_must_start_the_line() {
        assert_eq!(
            missing("See To fix: here\nBad: a\nFixed: b\n"),
            vec!["To fix:"]
        );
    }
}
