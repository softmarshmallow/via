//! The registry generator, shared by `examples/gen_reference.rs` and `tests/generated.rs`.
//!
//! `spec/diagnostics.md` section 2: the registry is one file, `spec/diagnostics.toml`, written from
//! the chapters' Diagnostics tables, and it generates the code constants, `via explain` and the
//! reference page; nothing else lists codes. This module holds the three steps as pure functions
//! of text, so the example writes what the test compares, byte for byte:
//!
//! 1. `derive_rows` reads every chapter's Diagnostics table and fixes every key but `explain` by
//!    the rules of section 2 (the "derived" registry).
//! 2. `sync_registry` writes `spec/diagnostics.toml` from the derived rows, keeping each code's
//!    hand-written `explain` (a new code gets a stub from its summary and hint).
//! 3. `generated_rust` and `reference_page` render the registry into `src/generated.rs` and
//!    `docs/reference/diagnostics.md`.
//!
//! It is not part of the library: the engine parses no TOML and no Markdown at run time.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The workspace root, from this crate's manifest folder.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("workspace root: {e}"))
}

pub const REGISTRY: &str = "spec/diagnostics.toml";
pub const GENERATED_RUST: &str = "crates/via-diag/src/generated.rs";
pub const REFERENCE: &str = "docs/reference/diagnostics.md";

/// One registry row: the keys of section 2's table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub code: String,
    pub severity: String,
    pub chapter: String,
    pub milestone: u32,
    pub summary: String,
    pub data: Vec<String>,
    pub worst: Option<String>,
    pub covered_by: String,
    pub explain: String,
}

/// A row as a chapter table gives it, before the registry: every key but `explain`, plus the
/// hint text, which only seeds a new code's explain stub.
#[derive(Clone, Debug)]
pub struct Derived {
    pub row: Row,
    pub hint: Option<String>,
}

/// Codes `diagnostics.md` section 2 names as covered by a named workspace test, besides every
/// `internal` code.
const WORKSPACE_COVERED: &[&str] = &["build.dir_busy", "graph.busy", "graph.store_unavailable"];

/// Phrases after "nearest" that give a value, a candidate or a distance rather than a nearest name
/// in the sense of section 1.4, so they add no `data.nearest`. The rule "then `nearest` if they
/// give a nearest name" needs this reading; it is recorded in the reference page.
const NOT_A_NEAREST_NAME: &[&str] = &[
    "nearest value",
    "nearest values",
    "nearest resolution",
    "nearest resolutions",
    "nearest that",
    "nearest candidate",
    "nearest instance",
];

/// Reads every `spec/*.md` chapter's Diagnostics tables. Errors name the file and the row.
pub fn derive_rows(spec: &Path) -> Result<Vec<Derived>, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(spec)
        .map_err(|e| format!("{}: {e}", spec.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    let mut out: Vec<Derived> = Vec::new();
    for file in files {
        let chapter = file.file_stem().unwrap().to_string_lossy().into_owned();
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        out.extend(chapter_rows(&chapter, &text)?);
    }
    let mut seen = BTreeMap::new();
    for d in &out {
        if let Some(other) = seen.insert(d.row.code.clone(), d.row.chapter.clone()) {
            return Err(format!(
                "{} is in the Diagnostics tables of both {other}.md and {}.md",
                d.row.code, d.row.chapter
            ));
        }
    }
    out.sort_by(|a, b| a.row.code.cmp(&b.row.code));
    Ok(out)
}

/// The rows of one chapter: every table under a level-2 heading that ends in "Diagnostics" whose
/// header starts `Code | Severity`.
pub fn chapter_rows(chapter: &str, text: &str) -> Result<Vec<Derived>, String> {
    let mut rows = Vec::new();
    let mut in_section = false;
    let mut header: Option<Vec<String>> = None;
    for (i, line) in text.lines().enumerate() {
        if let Some(h) = line.strip_prefix("## ") {
            in_section = h.trim_end().ends_with("Diagnostics");
            header = None;
            continue;
        }
        if !in_section {
            continue;
        }
        if !line.starts_with('|') {
            header = None;
            continue;
        }
        let cells = split_row(line);
        match &header {
            None => {
                header = Some(cells);
            }
            Some(h) => {
                if cells
                    .iter()
                    .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
                {
                    continue;
                }
                if h.len() < 2 || h[0] != "Code" || h[1] != "Severity" {
                    continue;
                }
                let at = format!("{chapter}.md line {}", i + 1);
                rows.push(derive_row(chapter, h, &cells).map_err(|e| format!("{at}: {e}"))?);
            }
        }
    }
    Ok(rows)
}

/// Splits a Markdown table row into trimmed cells; `\|` is a literal bar.
fn split_row(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|');
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            cur.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

fn derive_row(chapter: &str, header: &[String], cells: &[String]) -> Result<Derived, String> {
    let col = |names: &[&str]| header.iter().position(|h| names.contains(&h.as_str()));
    let get = |i: Option<usize>| i.and_then(|i| cells.get(i)).map(|s| s.as_str());
    let code = cells[0].trim_matches('`').to_string();
    if !code_well_formed(&code) {
        return Err(format!("`{code}` is not a code of the form group.name"));
    }
    let severity = cells.get(1).cloned().unwrap_or_default();
    if !["error", "warning", "info"].contains(&severity.as_str()) {
        return Err(format!("{code}: severity `{severity}`"));
    }
    let summary = get(col(&["Says", "Meaning", "Raised when"]))
        .ok_or_else(|| format!("{code}: no Says, Meaning or Raised when column"))?
        .to_string();
    let hint_col = get(col(&["Hint"]))
        .map(str::to_string)
        .filter(|h| !h.is_empty());
    let milestone = match get(col(&["M", "From"])) {
        Some(m) => first_number(m).ok_or_else(|| format!("{code}: milestone `{m}`"))?,
        None => summary_milestone(&summary).unwrap_or(1),
    };
    let mut data: Vec<String> = Vec::new();
    for text in [Some(summary.as_str()), hint_col.as_deref()]
        .into_iter()
        .flatten()
    {
        for name in data_names(text) {
            if !data.contains(&name) {
                data.push(name);
            }
        }
    }
    let gives_nearest = [Some(summary.as_str()), hint_col.as_deref()]
        .into_iter()
        .flatten()
        .any(gives_nearest_name);
    if gives_nearest && !data.iter().any(|d| d == "nearest") {
        data.push("nearest".to_string());
    }
    let worst = summary
        .find("worst first by data.")
        .map(|i| ident_at(&summary[i + "worst first by data.".len()..]));
    let name = code.rsplit('.').next().unwrap_or_default();
    let covered_by = if code.starts_with("load.") {
        "reader"
    } else if name == "internal" || WORKSPACE_COVERED.contains(&code.as_str()) {
        "workspace"
    } else {
        "conformance"
    };
    // The hint for an explain stub: the Hint column, else a "Hint:" sentence in the summary.
    let hint = hint_col.or_else(|| {
        summary
            .find("Hint:")
            .map(|i| summary[i + 5..].trim().to_string())
    });
    Ok(Derived {
        row: Row {
            code,
            severity,
            chapter: chapter.to_string(),
            milestone,
            summary,
            data,
            worst,
            covered_by: covered_by.to_string(),
            explain: String::new(),
        },
        hint,
    })
}

pub fn code_well_formed(code: &str) -> bool {
    let parts: Vec<&str> = code.split('.').collect();
    parts.len() == 2 && parts.iter().all(|p| ident_ok(p))
}

fn ident_ok(p: &str) -> bool {
    let mut cs = p.chars();
    matches!(cs.next(), Some('a'..='z')) && cs.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

fn ident_at(s: &str) -> String {
    s.chars()
        .take_while(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
        .collect()
}

fn first_number(s: &str) -> Option<u32> {
    let digits: String = s
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// A milestone named in the summary: `(Mn` (graph, protocol, build) or a sentence `Mn.` / `Mn,`
/// (authority). Section 2 names only `(Mn)`; authority.md writes "M2." without the parenthesis,
/// and reading that as milestone 1 would register M2 codes as M1.
fn summary_milestone(summary: &str) -> Option<u32> {
    let b = summary.as_bytes();
    for i in 0..b.len() {
        if b[i] != b'M' {
            continue;
        }
        let before_ok = i == 0 || b[i - 1] == b'(' || b[i - 1] == b' ';
        let digits: String = summary[i + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !before_ok || digits.is_empty() {
            continue;
        }
        let after = summary[i + 1 + digits.len()..].chars().next();
        if matches!(after, None | Some('.') | Some(')') | Some(',')) {
            return digits.parse().ok();
        }
    }
    None
}

fn data_names(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("data.") {
        let preceded_ok = i == 0 || !rest.as_bytes()[i - 1].is_ascii_alphanumeric();
        let name = ident_at(&rest[i + 5..]);
        if preceded_ok && !name.is_empty() {
            out.push(name.clone());
        }
        rest = &rest[i + 5..];
    }
    out
}

fn gives_nearest_name(text: &str) -> bool {
    let lower = text.to_lowercase();
    let mut rest = lower.as_str();
    while let Some(i) = rest.find("nearest") {
        let tail = &rest[i..];
        if !NOT_A_NEAREST_NAME.iter().any(|p| tail.starts_with(p)) {
            return true;
        }
        rest = &rest[i + 7..];
    }
    false
}

/// Parses `spec/diagnostics.toml` into rows sorted by code.
pub fn parse_registry(text: &str) -> Result<Vec<Row>, String> {
    let table: toml::Table = text.parse().map_err(|e| format!("{REGISTRY}: {e}"))?;
    let mut rows = Vec::new();
    for (group, codes) in &table {
        let codes = codes
            .as_table()
            .ok_or_else(|| format!("{REGISTRY}: `{group}` is not a table of codes"))?;
        for (name, row) in codes {
            let code = format!("{group}.{name}");
            let t = row
                .as_table()
                .ok_or_else(|| format!("{REGISTRY}: `{code}` is not a table"))?;
            for key in t.keys() {
                if ![
                    "severity",
                    "chapter",
                    "milestone",
                    "summary",
                    "data",
                    "worst",
                    "covered_by",
                    "explain",
                ]
                .contains(&key.as_str())
                {
                    return Err(format!("{REGISTRY}: `{code}` has unknown key `{key}`"));
                }
            }
            let s = |k: &str| -> Result<String, String> {
                t.get(k)
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .ok_or_else(|| format!("{REGISTRY}: `{code}` lacks the string `{k}`"))
            };
            let data = match t.get("data") {
                None => Vec::new(),
                Some(v) => v
                    .as_array()
                    .ok_or_else(|| format!("{REGISTRY}: `{code}.data` is not a list"))?
                    .iter()
                    .map(|x| x.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| format!("{REGISTRY}: `{code}.data` holds a non-string"))?,
            };
            let row = Row {
                severity: s("severity")?,
                chapter: s("chapter")?,
                milestone: t
                    .get("milestone")
                    .and_then(|v| v.as_integer())
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or_else(|| format!("{REGISTRY}: `{code}` lacks the integer `milestone`"))?,
                summary: s("summary")?,
                data,
                worst: t.get("worst").and_then(|v| v.as_str()).map(str::to_string),
                covered_by: t
                    .get("covered_by")
                    .and_then(|v| v.as_str())
                    .unwrap_or("conformance")
                    .to_string(),
                explain: s("explain")?,
                code,
            };
            check_row(&row)?;
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| a.code.cmp(&b.code));
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
    for r in &rows {
        let group = r.code.split('.').next().unwrap();
        if let Some(prev) = owner.insert(group, &r.chapter) {
            if prev != r.chapter {
                return Err(format!(
                    "group `{group}` is owned by both {prev}.md and {}.md (section 2: one chapter per group)",
                    r.chapter
                ));
            }
        }
    }
    Ok(rows)
}

fn check_row(r: &Row) -> Result<(), String> {
    let at = |m: String| format!("{REGISTRY}: `{}`: {m}", r.code);
    if !code_well_formed(&r.code) {
        return Err(at("not of the form group.name".into()));
    }
    if r.code.starts_with("step.") {
        return Err(at("the group `step` holds only a step's own codes".into()));
    }
    if !["error", "warning", "info"].contains(&r.severity.as_str()) {
        return Err(at(format!("severity `{}`", r.severity)));
    }
    if !["conformance", "reader", "workspace"].contains(&r.covered_by.as_str()) {
        return Err(at(format!("covered_by `{}`", r.covered_by)));
    }
    if let Some(w) = &r.worst {
        if !r.data.contains(w) {
            return Err(at(format!("worst `{w}` is not a data field")));
        }
    }
    if r.explain.trim().is_empty() || r.summary.trim().is_empty() {
        return Err(at("empty summary or explain".into()));
    }
    Ok(())
}

/// The registry file from derived rows, keeping each existing code's `explain`.
pub fn sync_registry(derived: &[Derived], existing: &[Row]) -> String {
    let old: BTreeMap<&str, &str> = existing
        .iter()
        .map(|r| (r.code.as_str(), r.explain.as_str()))
        .collect();
    let mut out = String::from(
        "# The diagnostic code registry (spec/diagnostics.md section 2; R1: flat TOML keyed by code).\n\
         # Every key but `explain` is derived from the chapters' Diagnostics tables by section 2's\n\
         # rules; `cargo run -p via-diag --example gen_reference -- --sync` rewrites them and keeps\n\
         # `explain`. `explain` is hand-written: what the code means, what to change, one bad and one\n\
         # fixed example, in neutral names (the deny-list of spec/vocabulary.md 7 applies).\n",
    );
    for d in derived {
        let r = &d.row;
        let explain = match old.get(r.code.as_str()) {
            Some(e) => e.to_string(),
            None => stub_explain(r, d.hint.as_deref()),
        };
        let _ = writeln!(out, "\n[{}]", r.code);
        let _ = writeln!(out, "severity = {}", toml_str(&r.severity));
        let _ = writeln!(out, "chapter = {}", toml_str(&r.chapter));
        let _ = writeln!(out, "milestone = {}", r.milestone);
        let _ = writeln!(out, "summary = {}", toml_str(&r.summary));
        if !r.data.is_empty() {
            let list: Vec<String> = r.data.iter().map(|d| toml_str(d)).collect();
            let _ = writeln!(out, "data = [{}]", list.join(", "));
        }
        if let Some(w) = &r.worst {
            let _ = writeln!(out, "worst = {}", toml_str(w));
        }
        if r.covered_by != "conformance" {
            let _ = writeln!(out, "covered_by = {}", toml_str(&r.covered_by));
        }
        let _ = writeln!(out, "explain = \"\"\"\n{}\"\"\"", toml_ml(&explain));
    }
    out
}

/// An explain text for a code that has none yet: the summary and the hint, with no examples. The
/// reference page marks these, so they are visibly unfinished rather than silently thin.
fn stub_explain(r: &Row, hint: Option<&str>) -> String {
    let mut s = format!("{}\n", r.summary.replace('`', ""));
    if let Some(h) = hint {
        if !r.summary.contains("Hint:") {
            let _ = writeln!(s, "To fix: {}", h.replace('`', ""));
        }
    }
    s
}

fn toml_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn toml_ml(s: &str) -> String {
    let mut s = s.replace('\\', "\\\\").replace("\"\"\"", "\"\"\\\"");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Compares the registry with the chapters on every derived key; one line per difference.
pub fn compare(derived: &[Derived], registry: &[Row]) -> Vec<String> {
    let reg: BTreeMap<&str, &Row> = registry.iter().map(|r| (r.code.as_str(), r)).collect();
    let der: BTreeMap<&str, &Row> = derived
        .iter()
        .map(|d| (d.row.code.as_str(), &d.row))
        .collect();
    let mut out = Vec::new();
    for (code, d) in &der {
        match reg.get(code) {
            None => out.push(format!("{code}: in {}.md, not in the registry", d.chapter)),
            Some(r) => {
                let mut r2 = (*r).clone();
                r2.explain.clear();
                if &r2 != *d {
                    out.push(format!(
                        "{code}: registry {r2:?} differs from chapter {d:?}"
                    ));
                }
            }
        }
    }
    for code in reg.keys() {
        if !der.contains_key(code) {
            out.push(format!(
                "{code}: in the registry, in no chapter's Diagnostics table"
            ));
        }
    }
    out
}

/// `src/generated.rs`: the row table and one constant per code, grouped by group.
pub fn generated_rust(rows: &[Row]) -> String {
    let mut o = String::from(
        "// @generated by `cargo run -p via-diag --example gen_reference` from spec/diagnostics.toml.\n\
         // Do not edit; `cargo test -p via-diag` fails when this file is stale.\n\n\
         use crate::registry::{CoveredBy, Row};\n\
         use crate::Severity;\n\n",
    );
    let _ = writeln!(
        o,
        "/// Every core code, sorted bytewise by code.\npub(crate) static ROWS: [Row; {}] = [",
        rows.len()
    );
    for r in rows {
        let data: Vec<String> = r.data.iter().map(|d| format!("{d:?}")).collect();
        let _ = writeln!(
            o,
            "    Row {{\n        code: {:?},\n        severity: Severity::{},\n        chapter: {:?},\n        milestone: {},\n        summary: {:?},\n        data: &[{}],\n        worst: {},\n        covered_by: CoveredBy::{},\n        explain: {:?},\n    }},",
            r.code,
            capital(&r.severity),
            r.chapter,
            r.milestone,
            r.summary,
            data.join(", "),
            r.worst.as_ref().map_or("None".to_string(), |w| format!("Some({w:?})")),
            capital(&r.covered_by),
            r.explain,
        );
    }
    o.push_str("];\n");
    let mut groups: BTreeMap<&str, Vec<(usize, &str)>> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        let (g, n) = r.code.split_once('.').unwrap();
        groups.entry(g).or_default().push((i, n));
    }
    let _ = writeln!(
        o,
        "\n/// The core codes as constants: the only way to name a core code in the engine, so it\n/// cannot raise an unregistered one (spec/diagnostics.md section 2)."
    );
    o.push_str("pub mod codes {\n");
    for (g, names) in groups {
        let chapter = &rows[names[0].0].chapter;
        let _ = writeln!(o, "    /// The group `{g}`, owned by `spec/{chapter}.md`.");
        let _ = writeln!(o, "    pub mod {g} {{\n        use crate::registry::Code;");
        for (i, n) in names {
            let _ = writeln!(
                o,
                "        /// `{g}.{n}` ({}): {}\n        pub const {}: Code = Code::from_index({i});",
                rows[i].severity,
                doc_line(&rows[i].summary),
                n.to_uppercase()
            );
        }
        o.push_str("    }\n");
    }
    o.push_str("}\n");
    o
}

/// A summary as one doc line: Markdown specials outside code spans are escaped, so rustdoc reads
/// `<a>` and `[x]` as text, not as HTML or links.
fn doc_line(s: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for c in s.chars() {
        match c {
            '`' => {
                in_code = !in_code;
                out.push(c);
            }
            '\n' => out.push(' '),
            '<' | '>' | '[' | ']' if !in_code => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `docs/reference/diagnostics.md` (R33): every code, by group, with its explain text.
pub fn reference_page(rows: &[Row]) -> String {
    let mut o = String::from(
        "<!-- @generated by `cargo run -p via-diag --example gen_reference` from spec/diagnostics.toml. Do not edit. -->\n\n\
         # Diagnostic codes\n\n\
         Every core code the engine can raise, generated from the registry `spec/diagnostics.toml`\n\
         (`spec/diagnostics.md` section 2). `via explain CODE` prints the same text. A step's own codes,\n\
         `step.<id>.<name>`, are not listed: their text comes with the step.\n\n\
         Columns: the code, its one severity, the milestone that first raises it, the chapter whose\n\
         Diagnostics table holds its row, and the `data` fields its records carry. `data.nearest` is\n\
         added where the row gives a nearest name (`spec/diagnostics.md` 1.4); a row that gives the\n\
         nearest value, resolution, candidate or instance does not count as one.\n",
    );
    let mut groups: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
    for r in rows {
        groups
            .entry(r.code.split('.').next().unwrap())
            .or_default()
            .push(r);
    }
    let stubs = rows.iter().filter(|r| !has_examples(&r.explain)).count();
    let _ = writeln!(
        o,
        "\n{} codes in {} groups. {} have no bad and fixed example yet; their explain text is the\nchapter's summary and hint.\n",
        rows.len(),
        groups.len(),
        stubs
    );
    o.push_str("| Group | Chapter | Codes |\n|---|---|---|\n");
    for (g, rs) in &groups {
        let _ = writeln!(
            o,
            "| [`{g}`](#{g}) | `spec/{}.md` | {} |",
            rs[0].chapter,
            rs.len()
        );
    }
    for (g, rs) in &groups {
        let _ = writeln!(o, "\n## {g}\n");
        o.push_str("| Code | Severity | M | Data | Summary |\n|---|---|---|---|---|\n");
        for r in rs {
            let data: Vec<String> = r.data.iter().map(|d| format!("`{d}`")).collect();
            let _ = writeln!(
                o,
                "| `{}` | {} | {} | {} | {} |",
                r.code,
                r.severity,
                r.milestone,
                data.join(", "),
                r.summary.replace('|', "\\|")
            );
        }
        for r in rs {
            let _ = writeln!(o, "\n### `{}`\n", r.code);
            let mut facts = format!(
                "{}, milestone {}, `spec/{}.md`",
                r.severity, r.milestone, r.chapter
            );
            if let Some(w) = &r.worst {
                let _ = write!(facts, ", worst first by `data.{w}`");
            }
            if r.covered_by != "conformance" {
                let _ = write!(facts, ", covered by the {} tests", r.covered_by);
            }
            let _ = writeln!(o, "{facts}.\n\n```text\n{}```", ensure_nl(&r.explain));
        }
    }
    o
}

pub fn has_examples(explain: &str) -> bool {
    explain.lines().any(|l| l.starts_with("Bad:"))
        && explain.lines().any(|l| l.starts_with("Fixed:"))
}

fn ensure_nl(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}
