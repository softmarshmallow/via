//! The limitations test of R33: the test fails when a limitation sorted "fix" in
//! `docs/design/limitations.md` names a case its chapter's Conformance section lacks.
//!
//! That is all R33 rules. An M1 fix that names no case, or whose case is owed, is reported as a
//! note, not a failure: no ruling says it must name one (W3 is proven by a workspace test, E3 by
//! a lint), and a rule nobody ruled on must not turn the workspace red.
//!
//! The "Proven by" column is prose, so it is read with a small grammar (see [`citations`]): a
//! backticked chapter name, optionally `M2` and `case` or `cases`, then a list of case names in
//! double quotes or backticks, case numbers, or rule-case ids (`r03`; `r30 to r33` is a range).
//! A list continues across `,`, `and`, `to` and parenthetical remarks, and across `;` when the
//! next item is a quoted name, a number or a rule id; a backticked word after `;` or one followed
//! by `'s` is a world or a plan line, not a case. Citations of worlds (`coast (M5)`) and of the
//! plan are not cases and are not checked.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::{read_text, CheckError, Outcome};

pub const LIMITATIONS_MD: &str = "docs/design/limitations.md";

/// One case a row cites.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseRef {
    Number(u32),
    Rule(u32),
    Name(String),
}

impl std::fmt::Display for CaseRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaseRef::Number(n) => write!(f, "case {n}"),
            CaseRef::Rule(n) => write!(f, "r{n:02}"),
            CaseRef::Name(s) => write!(f, "\"{s}\""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Tick(String),
    /// A backticked word followed by `'s` or `'`: something's, not a case.
    TickOwner,
    Quoted(String),
    Num(u32),
    Rule(u32),
    Comma,
    Semi,
    Period,
    Word(String),
}

fn tokens(s: &str) -> Vec<Tok> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        match ch {
            '(' => {
                // Parenthetical remarks annotate the citation before them; skipped whole.
                let mut depth = 0;
                while i < c.len() {
                    if c[i] == '(' {
                        depth += 1;
                    } else if c[i] == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                i += 1;
            }
            '"' => {
                let end = (i + 1..c.len()).find(|&j| c[j] == '"').unwrap_or(c.len());
                out.push(Tok::Quoted(c[i + 1..end].iter().collect()));
                i = end + 1;
            }
            '`' => {
                let end = (i + 1..c.len()).find(|&j| c[j] == '`').unwrap_or(c.len());
                let word: String = c[i + 1..end.min(c.len())].iter().collect();
                i = end + 1;
                if c.get(i) == Some(&'\'') {
                    out.push(Tok::TickOwner);
                    i += 1;
                } else {
                    out.push(Tok::Tick(word));
                }
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            ';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            '.' | ':' => {
                out.push(Tok::Period);
                i += 1;
            }
            c0 if c0.is_alphanumeric() => {
                let start = i;
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '-') {
                    i += 1;
                }
                let w: String = c[start..i].iter().collect();
                if let Ok(n) = w.parse::<u32>() {
                    out.push(Tok::Num(n));
                } else if let Some(n) = w
                    .strip_prefix('r')
                    .filter(|d| d.len() >= 2)
                    .and_then(|d| d.parse::<u32>().ok())
                {
                    out.push(Tok::Rule(n));
                } else {
                    out.push(Tok::Word(w));
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// The (chapter, case) pairs a "Proven by" cell cites. `chapters` are the spec chapter names.
pub fn citations(cell: &str, chapters: &[String]) -> Vec<(String, CaseRef)> {
    let mut out = Vec::new();
    let mut chapter: Option<String> = None;
    let mut after_semi = false;
    let mut range_from: Option<u32> = None;
    for t in tokens(cell) {
        let is_chapter = matches!(&t, Tok::Tick(s) if chapters.contains(s));
        if is_chapter {
            if let Tok::Tick(s) = &t {
                chapter = Some(s.clone());
            }
            after_semi = false;
            continue;
        }
        let Some(ch) = chapter.clone() else {
            continue;
        };
        match t {
            Tok::Quoted(s) => out.push((ch, CaseRef::Name(s))),
            Tok::Tick(s) if !after_semi => out.push((ch, CaseRef::Name(s))),
            Tok::Num(n) if !after_semi || out.last().is_some() => {
                out.push((ch, CaseRef::Number(n)))
            }
            Tok::Rule(n) => {
                if let Some(from) = range_from.take() {
                    for k in from + 1..=n {
                        out.push((ch.clone(), CaseRef::Rule(k)));
                    }
                } else {
                    out.push((ch, CaseRef::Rule(n)));
                }
            }
            Tok::Word(w) if w == "to" => {
                if let Some((_, CaseRef::Rule(n))) = out.last() {
                    range_from = Some(*n);
                }
            }
            Tok::Word(w) if w == "and" || w == "case" || w == "cases" || is_milestone(&w) => {}
            Tok::Comma => {}
            Tok::Semi => {
                after_semi = true;
                continue;
            }
            _ => chapter = None,
        }
        after_semi = false;
    }
    out
}

fn is_milestone(w: &str) -> bool {
    w.strip_prefix('M')
        .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
}

/// The case entries of one chapter's Conformance section, in the three forms the chapters use:
/// table rows (named by their first cell), numbered items (by number, and a bold title), and
/// bullets (by their opening words).
#[derive(Debug, Default)]
pub struct CaseIndex {
    table: Vec<String>,
    numbered: BTreeMap<u32, String>,
    bullets: Vec<String>,
}

impl CaseIndex {
    pub fn from_chapter(md: &str) -> Option<Self> {
        let mut idx = CaseIndex::default();
        let mut inside = false;
        let mut found = false;
        for line in md.lines() {
            let line = line.trim_end_matches('\r');
            if line.starts_with("## ") {
                inside = line.trim_end().ends_with("Conformance");
                found |= inside;
                continue;
            }
            if !inside {
                continue;
            }
            let t = line.trim_start();
            if let Some(row) = t.strip_prefix('|') {
                let first = row.split('|').next().unwrap_or("").trim();
                if !first.is_empty() && !first.starts_with("---") {
                    idx.table.push(first.to_string());
                }
            } else if let Some(b) = t.strip_prefix("- ") {
                idx.bullets.push(b.to_string());
            } else if let Some((n, rest)) = t.split_once(". ") {
                if let Ok(n) = n.parse::<u32>() {
                    let title = rest
                        .strip_prefix("**")
                        .and_then(|r| r.split_once("**"))
                        .map(|(t, _)| t.trim_end_matches('.').to_string())
                        .unwrap_or_default();
                    idx.numbered.insert(n, title);
                }
            }
        }
        found.then_some(idx)
    }

    pub fn holds(&self, case: &CaseRef) -> bool {
        match case {
            CaseRef::Number(n) => {
                self.numbered.contains_key(n) || self.table.iter().any(|c| *c == n.to_string())
            }
            CaseRef::Rule(n) => {
                let id = format!("r{n:02}");
                self.table
                    .iter()
                    .any(|c| c.split_whitespace().next() == Some(id.as_str()))
            }
            CaseRef::Name(s) => {
                let want = s.to_lowercase();
                let bare = want.replace('`', "");
                self.table.iter().any(|c| {
                    let c = c.to_lowercase();
                    c == want
                        || c.replace('`', "") == bare
                        || c.split_whitespace().any(|w| w.trim_matches('`') == bare)
                }) || self.numbered.values().any(|t| t.to_lowercase() == want)
                    || self.bullets.iter().any(|b| {
                        // `cross` names the bullet "`cross`; network; ...": backticks aside, the
                        // bullet opens with the name and the name ends there.
                        let b = b.to_lowercase();
                        let b_bare = b.replace('`', "");
                        b.starts_with(&want)
                            || b_bare.strip_prefix(&bare).is_some_and(|rest| {
                                !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                            })
                    })
            }
        }
    }
}

/// A limitations row of the sorted tables.
#[derive(Debug, Clone)]
pub struct Row {
    pub line: usize,
    pub id: String,
    pub verdict: String,
    pub r#where: String,
    pub proven_by: String,
}

pub fn rows(md: &str) -> Vec<Row> {
    let mut out = Vec::new();
    for (k, line) in md.lines().enumerate() {
        let cells: Vec<&str> = line
            .trim_end_matches('\r')
            .split('|')
            .map(str::trim)
            .collect();
        // | # | Limitation | Verdict | Where | Proven by | Until then |
        if cells.len() != 8 {
            continue;
        }
        let id = cells[1];
        let is_id = id.starts_with('S') || id.starts_with('W') || id.starts_with('E');
        if !is_id || !id.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        out.push(Row {
            line: k + 1,
            id: id.to_string(),
            verdict: cells[3].to_string(),
            r#where: cells[4].to_string(),
            proven_by: cells[5].to_string(),
        });
    }
    out
}

fn mentions_m1(s: &str) -> bool {
    s.split(|c: char| !c.is_alphanumeric()).any(|w| w == "M1")
}

pub fn check(root: &Path) -> Result<Outcome, CheckError> {
    let md = read_text(&root.join(LIMITATIONS_MD))?;
    let mut chapters = Vec::new();
    let mut indexes = BTreeMap::new();
    let mut entries: Vec<_> = std::fs::read_dir(root.join("spec"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    entries.sort();
    for p in entries {
        let Some(name) = p.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if let Some(idx) = CaseIndex::from_chapter(&read_text(&p)?) {
            indexes.insert(name.clone(), idx);
        }
        chapters.push(name);
    }

    let mut o = Outcome::default();
    let all = rows(&md);
    let (mut fixes, mut m1, mut cited) = (0, 0, 0);
    let (mut uncited, mut owed) = (Vec::new(), Vec::new());
    for row in &all {
        if row.verdict != "fix" {
            continue;
        }
        fixes += 1;
        let is_m1 = mentions_m1(&row.r#where);
        let cites = citations(&row.proven_by, &chapters);
        cited += cites.len();
        for (chapter, case) in &cites {
            match indexes.get(chapter) {
                None => o.finding(
                    LIMITATIONS_MD,
                    row.line,
                    format!("{}: cites `{chapter}` {case}, but that chapter has no Conformance section", row.id),
                ),
                Some(idx) if !idx.holds(case) => o.finding(
                    LIMITATIONS_MD,
                    row.line,
                    format!("{}: cites `{chapter}` {case}, which spec/{chapter}.md's Conformance section lacks", row.id),
                ),
                Some(_) => {}
            }
        }
        if is_m1 {
            m1 += 1;
            if cites.is_empty() {
                uncited.push(format!("{} (line {})", row.id, row.line));
            }
            if row.proven_by.to_lowercase().contains("owed") {
                owed.push(format!("{} (line {})", row.id, row.line));
            }
        }
    }
    if fixes == 0 {
        return Err(CheckError(format!(
            "{LIMITATIONS_MD}: no row sorted \"fix\" was read"
        )));
    }
    o.note(format!(
        "{} rows, {fixes} sorted fix ({m1} in M1), {cited} case citations checked against {} chapters",
        all.len(),
        indexes.len()
    ));
    if !uncited.is_empty() {
        o.note(format!(
            "M1 fixes proven by something other than a conformance case: {}",
            uncited.join(", ")
        ));
    }
    if !owed.is_empty() {
        o.note(format!("M1 fixes whose case is owed: {}", owed.join(", ")));
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch() -> Vec<String> {
        [
            "build",
            "rules",
            "authority",
            "assets",
            "geometry",
            "graph",
            "protocol",
            "fields",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn reads_citations() {
        let c = citations(
            "`rules` r03, r04; `authority` case `pad_order`; `assets` case \"Pack swap\"; `ford`'s \"Pack swapped\" row.",
            &ch(),
        );
        assert_eq!(
            c,
            vec![
                ("rules".into(), CaseRef::Rule(3)),
                ("rules".into(), CaseRef::Rule(4)),
                ("authority".into(), CaseRef::Name("pad_order".into())),
                ("assets".into(), CaseRef::Name("Pack swap".into())),
            ]
        );
        let c = citations("`rules` r30 to r33; `ford`'s \"x\" lint.", &ch());
        assert_eq!(c.len(), 4);
        let c = citations(
            "`geometry` cases `cross`, \"two paths\"; `townland` (M3).",
            &ch(),
        );
        assert_eq!(c.len(), 2);
        let c = citations(
            "`build` cases 11 (\"No stale file\"), 8 (\"Complete\"), 21.",
            &ch(),
        );
        assert_eq!(
            c.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
            vec![CaseRef::Number(11), CaseRef::Number(8), CaseRef::Number(21)]
        );
        let c = citations(
            "`fields` M2 cases \"step view\" (12, not 14), \"step two rows\"",
            &ch(),
        );
        assert_eq!(c.len(), 2);
        let c = citations(
            "Plan M3, challenges 34, 38; `geometry` cases \"`speckle`; mask\"",
            &ch(),
        );
        assert_eq!(
            c,
            vec![("geometry".into(), CaseRef::Name("`speckle`; mask".into()))]
        );
    }

    #[test]
    fn indexes_case_forms() {
        let md = "# X\n## 1. Body\n| nope | M1 |\n## Conformance\n\n| Case | M |\n|---|---|\n| r03 pad | M1 |\n| order_key | M1 |\n\n1. **Null** (M0). text\n2. more\n- `speckle`; mask; things\n## Open\n";
        let idx = CaseIndex::from_chapter(md).expect("has section");
        assert!(idx.holds(&CaseRef::Rule(3)));
        assert!(idx.holds(&CaseRef::Name("order_key".into())));
        assert!(idx.holds(&CaseRef::Number(2)));
        assert!(idx.holds(&CaseRef::Name("null".into())));
        assert!(idx.holds(&CaseRef::Name("`speckle`; mask".into())));
        assert!(idx.holds(&CaseRef::Name("speckle".into())));
        assert!(!idx.holds(&CaseRef::Name("speck".into())));
        assert!(!idx.holds(&CaseRef::Name("nope".into())));
        assert!(!idx.holds(&CaseRef::Number(3)));
    }
}
