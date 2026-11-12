//! `spec/vocabulary.toml`'s `[key.*]` tables against `spec/vocabulary.md` section 3.
//!
//! Section 7 says the machine copy "also lists every key of section 3" and that the code checks
//! hold the two equal; the deny-list sync covers section 7 only, so without this a key added to
//! or renamed in the chapter would leave the machine list stale with nothing failing.
//!
//! The chapter's key tables are the ones headed `| Key | Takes | Meaning | Chapter |` under a
//! `### <Section> (...)` heading of section 3. A row may name several keys (`` `remove`, `unset` ``);
//! its Takes then splits at `; ` when that gives one part per key (`bool; list of strings`), and
//! otherwise every key takes the whole cell (`` `front`, `up`, `pivot` `` all take `string`;
//! `` `scale`, `tint` `` both take `number, or 2 numbers on a scatter`). A key's defining row is the
//! first one with a non-empty Takes; later rows add sections.

use std::collections::BTreeMap;

use crate::report::{CheckError, Outcome};

/// One key as both sides describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    pub takes: String,
    pub chapter: String,
    /// The section 3 tables it appears in, in the chapter's order.
    pub sections: Vec<String>,
}

/// The `[key.*]` tables of `spec/vocabulary.toml`.
pub fn from_toml(text: &str, file: &str) -> Result<BTreeMap<String, KeyEntry>, CheckError> {
    let doc: toml::Table = toml::from_str(text).map_err(|e| CheckError(format!("{file}: {e}")))?;
    let mut out = BTreeMap::new();
    let Some(keys) = doc.get("key").and_then(|v| v.as_table()) else {
        return Err(CheckError(format!("{file}: no [key.*] tables")));
    };
    for (k, v) in keys {
        let s = |f: &str| {
            v.get(f)
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .ok_or_else(|| CheckError(format!("{file}: [key.\"{k}\"].{f} is not a string")))
        };
        let sections = v
            .get("sections")
            .and_then(|x| x.as_array())
            .ok_or_else(|| CheckError(format!("{file}: [key.\"{k}\"].sections is not a list")))?
            .iter()
            .map(|x| {
                x.as_str().map(str::to_string).ok_or_else(|| {
                    CheckError(format!("{file}: [key.\"{k}\"].sections holds a non-string"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.insert(
            k.clone(),
            KeyEntry {
                takes: s("takes")?,
                chapter: s("chapter")?,
                sections,
            },
        );
    }
    Ok(out)
}

/// A key row of section 3 that cannot be read: (1-based line, what is wrong).
pub type RowError = (usize, String);

/// The keys of section 3's key tables, and the rows that could not be read.
pub fn from_chapter(md: &str) -> (BTreeMap<String, KeyEntry>, Vec<RowError>) {
    let mut out: BTreeMap<String, KeyEntry> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut in3 = false;
    let mut section = String::new();
    let mut key_table = false;
    for (k, line) in md.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("## ") {
            in3 = line.starts_with("## 3. ");
            continue;
        }
        if !in3 {
            continue;
        }
        if let Some(h) = line.strip_prefix("### ") {
            section = h.split(" (").next().unwrap_or(h).trim().to_string();
            key_table = false;
            continue;
        }
        if !line.starts_with('|') {
            key_table = false;
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // A leading and trailing empty piece around the cells.
        let inner = &cells[1..cells.len().saturating_sub(1)];
        if inner.first() == Some(&"Key") {
            key_table = inner == ["Key", "Takes", "Meaning", "Chapter"];
            continue;
        }
        if !key_table || inner.first().is_some_and(|c| c.starts_with("---")) {
            continue;
        }
        if inner.len() != 4 {
            errors.push((
                k + 1,
                format!("key row has {} cells, the table has 4", inner.len()),
            ));
            continue;
        }
        let names = backticked(inner[0]);
        if names.is_empty() {
            errors.push((k + 1, "key row names no backticked key".to_string()));
            continue;
        }
        let parts: Vec<&str> = inner[1].split("; ").collect();
        for (i, name) in names.iter().enumerate() {
            let takes = if parts.len() == names.len() {
                parts[i]
            } else {
                inner[1]
            };
            let e = out.entry(name.clone()).or_insert_with(|| KeyEntry {
                takes: String::new(),
                chapter: inner[3].to_string(),
                sections: Vec::new(),
            });
            if e.takes.is_empty() && !takes.is_empty() {
                takes.clone_into(&mut e.takes);
                inner[3].clone_into(&mut e.chapter);
            }
            if !e.sections.contains(&section) {
                e.sections.push(section.clone());
            }
        }
    }
    (out, errors)
}

fn backticked(s: &str) -> Vec<String> {
    s.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, t)| t.to_string())
        .collect()
}

/// Compares the two sides, adding a finding per difference.
pub fn compare(
    md: &str,
    toml_text: &str,
    md_file: &str,
    toml_file: &str,
    o: &mut Outcome,
) -> Result<usize, CheckError> {
    let machine = from_toml(toml_text, toml_file)?;
    let (chapter, errors) = from_chapter(md);
    for (line, msg) in errors {
        o.finding(md_file, line, format!("section 3: {msg}"));
    }
    if chapter.is_empty() {
        return Err(CheckError(format!(
            "{md_file}: no key table read in section 3"
        )));
    }
    for (k, want) in &chapter {
        match machine.get(k) {
            None => o.finding(
                toml_file,
                0,
                format!("key `{k}` of {md_file} 3 has no [key.\"{k}\"]"),
            ),
            Some(got) if got != want => o.finding(
                toml_file,
                0,
                format!("[key.\"{k}\"] differs from {md_file} 3: chapter {want:?}, toml {got:?}"),
            ),
            Some(_) => {}
        }
    }
    for k in machine.keys().filter(|k| !chapter.contains_key(*k)) {
        o.finding(
            toml_file,
            0,
            format!("[key.\"{k}\"] is not a key of {md_file} 3"),
        );
    }
    Ok(chapter.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "## 3. Keys\n\n### Document (`documents`)\n\n| Key | Takes | Meaning | Chapter |\n|---|---|---|---|\n| `name` | string | x | documents |\n| `remove`, `unset` | bool; list of strings | y | documents |\n\n| Word | Meaning | Chapter |\n|---|---|---|\n| entry | not a key | authority |\n\n### Fields (`fields`)\n\n| Key | Takes | Meaning | Chapter |\n|---|---|---|---|\n| `name` | | See Document. | fields |\n| `scale`, `tint` | number, or 2 | z | rules |\n## 4. Next\n| `late` | string | no | x |\n";

    #[test]
    fn reads_key_rows() {
        let (k, errs) = from_chapter(MD);
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(
            k.keys().cloned().collect::<Vec<_>>(),
            vec!["name", "remove", "scale", "tint", "unset"]
        );
        assert_eq!(
            k["name"],
            KeyEntry {
                takes: "string".into(),
                chapter: "documents".into(),
                sections: vec!["Document".into(), "Fields".into()],
            }
        );
        assert_eq!(k["unset"].takes, "list of strings");
        assert_eq!(k["tint"].takes, "number, or 2");
    }

    #[test]
    fn reports_drift_both_ways() {
        let toml = "[key.\"name\"]\ntakes = \"string\"\nchapter = \"documents\"\nsections = [\"Document\"]\n\
                    [key.\"gone\"]\ntakes = \"x\"\nchapter = \"x\"\nsections = []\n";
        let mut o = Outcome::default();
        compare(MD, toml, "md", "toml", &mut o).expect("reads");
        let msgs: Vec<String> = o.findings.iter().map(|f| f.message.clone()).collect();
        // `name` differs (sections), four keys are missing, `gone` is extra.
        assert_eq!(msgs.len(), 6, "{msgs:?}");
    }

    #[test]
    fn short_row_is_a_finding() {
        let md = "## 3. K\n### P (`assets`)\n| Key | Takes | Meaning | Chapter |\n|---|---|---|---|\n| `a` | table | m |\n";
        let (_, errs) = from_chapter(md);
        assert_eq!(errs.len(), 1);
    }
}
