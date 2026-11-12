//! The terminal form (`spec/diagnostics.md` section 7), written to standard error by the CLI.
//!
//! The layout follows the Rust compiler's: a head line `severity[code] subject: message`, then
//! indented lines for the location, hint and any related things, chain, position, count and
//! occurrences. At most ten records per code, then `... and N more <code>`; infos only as a count
//! unless asked for. Last, one line per lint step and the totals. Conformance compares only the
//! totals line, so the rest may change without a spec change.

use crate::collect::{lint_counts, order, Counts, Lint};
use crate::json::{format_double, format_metres};
use crate::record::Record;
use crate::site::Location;
use crate::Severity;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// How to print.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Print info records in full (`--info`); otherwise only counted.
    pub info: bool,
    /// Colour the severities. The CLI sets it off when `NO_COLOR` is set or standard error is
    /// not a terminal.
    pub colour: bool,
}

/// The text for these records and lint steps, ending with the totals line.
pub fn render(records: &[Record], lints: &[Lint], opts: Options) -> String {
    let mut sorted: Vec<&Record> = records.iter().collect();
    sorted.sort_by(|a, b| order(a, b));
    let mut out = String::new();
    let mut shown: BTreeMap<&str, u32> = BTreeMap::new();
    let mut hidden: BTreeMap<&str, u32> = BTreeMap::new();
    for r in &sorted {
        if r.severity == Severity::Info && !opts.info {
            continue;
        }
        let n = shown.entry(&r.code).or_insert(0);
        if *n >= 10 {
            *hidden.entry(&r.code).or_insert(0) += 1;
            continue;
        }
        *n += 1;
        record(&mut out, r, opts.colour);
    }
    for (code, n) in hidden {
        let _ = writeln!(out, "... and {n} more {code}");
    }
    let mut lint_list: Vec<&Lint> = lints.iter().collect();
    lint_list.sort_by(|a, b| a.step.as_bytes().cmp(b.step.as_bytes()));
    for l in lint_list {
        if l.ran {
            let c = lint_counts(records, &l.step);
            let _ = writeln!(out, "lint {}: {}", esc(&l.step), totals(c));
        } else {
            let _ = writeln!(out, "lint {}: skipped", esc(&l.step));
        }
    }
    let _ = writeln!(out, "{}", totals_line(records));
    out
}

/// The summary line conformance compares: `E errors, W warnings, I infos`, with
/// `(N failed by --fail-on)` when there are any.
pub fn totals_line(records: &[Record]) -> String {
    let c = Counts::of(records);
    let mut s = totals(c);
    if c.failed_on > 0 {
        let _ = write!(s, " ({} failed by --fail-on)", c.failed_on);
    }
    s
}

fn totals(c: Counts) -> String {
    format!(
        "{} errors, {} warnings, {} infos",
        c.error, c.warning, c.info
    )
}

fn record(out: &mut String, r: &Record, colour: bool) {
    let sev = r.severity.as_str();
    let sev = if colour {
        let code = match r.severity {
            Severity::Error => "31",
            Severity::Warning => "33",
            Severity::Info => "36",
        };
        format!("\u{1b}[1;{code}m{sev}\u{1b}[0m")
    } else {
        sev.to_string()
    };
    let _ = write!(
        out,
        "{sev}[{}] {}: {}",
        r.code,
        esc(&r.subject),
        esc(&r.message)
    );
    if r.failed_on {
        out.push_str(" (failed by --fail-on)");
    }
    out.push('\n');
    let _ = writeln!(out, "  --> {}", location(&r.location));
    let _ = writeln!(out, "  hint: {}", esc(&r.hint));
    for rel in &r.related {
        let _ = write!(
            out,
            "  related: {} at {}: {}",
            esc(&rel.subject),
            location(&rel.location),
            esc(&rel.note)
        );
        if let Some(p) = rel.position {
            let _ = write!(out, " at {}", position(p));
        }
        out.push('\n');
    }
    for c in &r.chain {
        let _ = write!(out, "  through {} `{}`", c.through.as_str(), esc(&c.name));
        if let Some(o) = &c.owner {
            let _ = write!(out, " (owner {})", esc(o));
        }
        let _ = writeln!(out, " at {}:{}", esc(&c.file), c.line);
    }
    if let Some(p) = r.position {
        let _ = writeln!(out, "  position: {}", position(p));
    }
    if r.count > 1 || r.count_stated {
        let _ = writeln!(out, "  count: {}", r.count);
    }
    for o in &r.occurrences {
        out.push_str("  occurrence:");
        if let Some(n) = &o.name {
            let _ = write!(out, " {}", esc(n));
        }
        if let Some(p) = o.position {
            let _ = write!(out, " at {}", position(p));
        }
        if let Some(v) = &o.value {
            let mut s = String::new();
            v.to_json().write(&mut s);
            let _ = write!(out, " value {}", esc(&s));
        }
        out.push('\n');
    }
}

fn location(l: &Location) -> String {
    match l {
        Location::Argument(n) => format!("argument {n}"),
        Location::File { file } => esc(file),
        Location::Span {
            file, line, column, ..
        } => format!("{}:{line}:{column}", esc(file)),
    }
}

fn position(p: crate::Position) -> String {
    let mut s = format!("({}, {}", format_metres(p.x_mm()), format_metres(p.y_mm()));
    if let Some(z) = p.z() {
        let _ = write!(s, ", {}", format_double(z.get()));
    }
    s.push(')');
    s
}

/// Escapes control characters in user text, so a message cannot move the cursor or clear the
/// screen.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control() {
            let _ = write!(o, "\\u{{{:x}}}", c as u32);
        } else {
            o.push(c);
        }
    }
    o
}
