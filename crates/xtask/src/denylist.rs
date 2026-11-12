//! The domain-noun deny-list (`spec/vocabulary.md` 7, principle 5): no core crate contains a word
//! that belongs to a library.
//!
//! The list, its allow-list senses and the crates it covers are read from `spec/vocabulary.toml`
//! (R33). The chapter stays the source, so [`check_sync`] holds the machine copy equal to section
//! 7's prose and table. [`self_test`] plants every listed word before the real scan runs, so a
//! matcher that broke silently cannot report a clean core.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::lex::{self, Unit};
use crate::report::{read_text, rel, CheckError, Outcome};

/// Section 7 as `spec/vocabulary.toml` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    /// In the file's order; a two-word entry is one string with a space.
    pub deny: Vec<String>,
    /// Allowed word to its one allowed sense. The test passes these; review checks the sense.
    pub allow: BTreeMap<String, String>,
    pub crates: Vec<String>,
    pub excluded: Vec<String>,
}

pub const VOCABULARY_TOML: &str = "spec/vocabulary.toml";
pub const VOCABULARY_MD: &str = "spec/vocabulary.md";

fn str_list(t: &toml::Table, key: &str) -> Result<Vec<String>, CheckError> {
    let arr = t
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| CheckError(format!("{VOCABULARY_TOML}: [deny].{key} is not a list")))?;
    arr.iter()
        .map(|v| {
            v.as_str().map(str::to_string).ok_or_else(|| {
                CheckError(format!(
                    "{VOCABULARY_TOML}: [deny].{key} holds a non-string"
                ))
            })
        })
        .collect()
}

impl Vocabulary {
    pub fn parse(text: &str) -> Result<Self, CheckError> {
        let doc: toml::Table =
            toml::from_str(text).map_err(|e| CheckError(format!("{VOCABULARY_TOML}: {e}")))?;
        let deny = doc
            .get("deny")
            .and_then(|v| v.as_table())
            .ok_or_else(|| CheckError(format!("{VOCABULARY_TOML}: no [deny] table")))?;
        let mut allow = BTreeMap::new();
        if let Some(a) = doc.get("allow").and_then(|v| v.as_table()) {
            for (word, entry) in a {
                let sense = entry.get("sense").and_then(|s| s.as_str()).ok_or_else(|| {
                    CheckError(format!("{VOCABULARY_TOML}: [allow.{word}] has no sense"))
                })?;
                allow.insert(word.clone(), sense.to_string());
            }
        }
        Ok(Vocabulary {
            deny: str_list(deny, "words")?,
            allow,
            crates: str_list(deny, "crates")?,
            excluded: str_list(deny, "excluded")?,
        })
    }

    pub fn load(root: &Path) -> Result<Self, CheckError> {
        Self::parse(&read_text(&root.join(VOCABULARY_TOML))?)
    }

    /// The entries the test reports: the list less the allow-list.
    pub fn enforced(&self) -> Vec<Vec<String>> {
        self.deny
            .iter()
            .filter(|w| !self.allow.contains_key(*w))
            .map(|w| w.split_whitespace().map(str::to_string).collect())
            .collect()
    }
}

/// A word matches an entry word when equal, or its plural in `s` or `es` (section 7).
fn word_matches(word: &str, entry: &str) -> bool {
    word == entry || word.strip_suffix('s') == Some(entry) || word.strip_suffix("es") == Some(entry)
}

/// The entries a unit's words hit. A two-word entry matches two words in a row; only the last
/// word takes a plural (`main ways`).
pub fn hits(words: &[String], entries: &[Vec<String>]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        let n = entry.len();
        if n == 0 || words.len() < n {
            continue;
        }
        let hit = words.windows(n).any(|win| {
            win[..n - 1]
                .iter()
                .zip(&entry[..n - 1])
                .all(|(a, b)| a == b)
                && word_matches(&win[n - 1], &entry[n - 1])
        });
        if hit {
            out.push(entry.join(" "));
        }
    }
    out
}

/// Scans one source text; each hit is (line, entry, the identifier or literal it was found in).
pub fn scan_source(
    src: &str,
    entries: &[Vec<String>],
) -> Result<Vec<(usize, String, String)>, lex::LexError> {
    let units: Vec<Unit> = lex::units(src)?;
    let mut out = Vec::new();
    for u in units {
        for entry in hits(&u.words, entries) {
            out.push((u.line, entry, u.text.clone()));
        }
    }
    Ok(out)
}

/// Every `.rs` file under `dir`, sorted, skipping build output.
pub fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, CheckError> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&d)
            .map_err(|e| CheckError(format!("{}: {e}", d.display())))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The deny-list test proper, over every core crate the vocabulary names.
pub fn check(root: &Path) -> Result<Outcome, CheckError> {
    let vocab = Vocabulary::load(root)?;
    let mut o = Outcome::default();
    self_test_with(&vocab, &mut o);
    let entries = vocab.enforced();
    let mut files = 0;
    for krate in &vocab.crates {
        let dir = root.join("crates").join(krate);
        if !dir.is_dir() {
            o.finding(
                VOCABULARY_TOML,
                0,
                format!("core crate `{krate}` is listed but crates/{krate} does not exist"),
            );
            continue;
        }
        for f in rust_files(&dir)? {
            files += 1;
            let src = read_text(&f)?;
            match scan_source(&src, &entries) {
                Ok(found) => {
                    for (line, entry, text) in found {
                        o.finding(
                            rel(root, &f),
                            line,
                            format!(
                                "library noun `{entry}` in `{text}` (spec/vocabulary.md 7: a core \
                                 mechanism named after one is renamed, not exempted)"
                            ),
                        );
                    }
                }
                Err(e) => o.finding(rel(root, &f), e.line, format!("cannot read: {}", e.message)),
            }
        }
    }
    o.note(format!(
        "{} crates, {files} files, {} enforced words ({} allowed in one sense)",
        vocab.crates.len(),
        entries.len(),
        vocab.allow.len()
    ));
    Ok(o)
}

/// Plants every enforced word, singular and plural, in an identifier, a string literal and every
/// kind of comment; the identifier and literal must hit and the comments must not.
fn self_test_with(vocab: &Vocabulary, o: &mut Outcome) {
    let entries = vocab.enforced();
    for entry in &entries {
        let joined = entry.join("_");
        let spaced = entry.join(" ");
        let camel: String = entry
            .iter()
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect();
        let src = format!(
            "// {spaced}\n/* {spaced} /* {spaced} */ */\n/// {spaced}\n//! {spaced}\n\
             fn f() {{ let a_{joined}_b = 1; }}\nstruct X{camel}s;\nconst S: &str = \"{spaced}es!\";\n"
        );
        let want = vec![
            (5, spaced.clone()),
            (6, spaced.clone()),
            (7, spaced.clone()),
        ];
        let got = match scan_source(&src, std::slice::from_ref(entry)) {
            Ok(v) => v.into_iter().map(|(l, e, _)| (l, e)).collect::<Vec<_>>(),
            Err(e) => {
                o.finding(
                    "crates/xtask",
                    0,
                    format!("self-test: planted source fails to lex: {e}"),
                );
                continue;
            }
        };
        if got != want {
            o.finding(
                "crates/xtask",
                0,
                format!("self-test: planted `{spaced}` gave {got:?}, wanted {want:?}"),
            );
        }
    }
    o.note(format!(
        "self-test: {} planted words caught in identifiers and literals, ignored in comments",
        entries.len()
    ));
}

/// Holds `spec/vocabulary.toml` equal to the chapter: the deny-list, allow-list and crate lists
/// to section 7, and the `[key.*]` tables to section 3's key rows ([`crate::keys`]).
pub fn check_sync(root: &Path) -> Result<Outcome, CheckError> {
    let vocab = Vocabulary::load(root)?;
    let md = read_text(&root.join(VOCABULARY_MD))?;
    let mut o = Outcome::default();
    let section = section7(&md).ok_or_else(|| {
        CheckError(format!(
            "{VOCABULARY_MD}: no section headed `## 7. Library words`"
        ))
    })?;

    let prose = words_paragraph(&section);
    if prose != vocab.deny {
        o.finding(
            VOCABULARY_TOML,
            0,
            format!(
                "[deny].words differ from {VOCABULARY_MD} 7: chapter {prose:?}, toml {:?}",
                vocab.deny
            ),
        );
    }
    let table = allow_table(&section);
    let toml_allow: Vec<(String, String)> = vocab
        .allow
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut md_allow = table.clone();
    md_allow.sort();
    if md_allow != toml_allow {
        o.finding(
            VOCABULARY_TOML,
            0,
            format!(
                "[allow] differs from {VOCABULARY_MD} 7's table: chapter {md_allow:?}, toml {toml_allow:?}"
            ),
        );
    }
    for (w, _) in &table {
        if !vocab.deny.contains(w) {
            o.finding(
                VOCABULARY_MD,
                0,
                format!("allowed word `{w}` is not on the list"),
            );
        }
    }
    let (covers, excludes) = crate_lists(&section);
    if covers != vocab.crates || excludes != vocab.excluded {
        o.finding(
            VOCABULARY_TOML,
            0,
            format!(
                "[deny] crates/excluded differ from {VOCABULARY_MD} 7: chapter {covers:?} / {excludes:?}, \
                 toml {:?} / {:?}",
                vocab.crates, vocab.excluded
            ),
        );
    }
    let toml_text = read_text(&root.join(VOCABULARY_TOML))?;
    let keys = crate::keys::compare(&md, &toml_text, VOCABULARY_MD, VOCABULARY_TOML, &mut o)?;
    o.note(format!(
        "{} words, {} allowed senses, {} crates, {keys} section 3 keys compared",
        prose.len(),
        table.len(),
        covers.len()
    ));
    Ok(o)
}

fn section7(md: &str) -> Option<String> {
    let mut out = String::new();
    let mut inside = false;
    for line in md.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("## ") {
            if inside {
                break;
            }
            inside = line.starts_with("## 7. Library words");
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The first paragraph: "Words that belong to libraries ...: road, lane, ... headroom."
fn words_paragraph(section: &str) -> Vec<String> {
    let para: String = section
        .split("\n\n")
        .find(|p| p.contains("Words that belong to libraries"))
        .unwrap_or("")
        .replace('\n', " ");
    let list = para.split_once(':').map(|(_, l)| l).unwrap_or("");
    list.trim()
        .trim_end_matches('.')
        .split(',')
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Rows `| \`word\` | sense |` of the allow table.
fn allow_table(section: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in section.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() == 4 && cells[1].starts_with('`') && cells[1].ends_with('`') {
            out.push((cells[1].trim_matches('`').to_string(), cells[2].to_string()));
        }
    }
    out
}

/// The "Covers ... It excludes ..." bullet's backticked crate names.
fn crate_lists(section: &str) -> (Vec<String>, Vec<String>) {
    let flat = section.replace('\n', " ");
    let Some(start) = flat.find("Covers ") else {
        return (Vec::new(), Vec::new());
    };
    let rest = &flat[start..];
    let (covers, after) = rest.split_once("It excludes").unwrap_or((rest, ""));
    let after = after.split_once('.').map(|(a, _)| a).unwrap_or(after);
    (backticked(covers), backticked(after))
}

fn backticked(s: &str) -> Vec<String> {
    s.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, t)| t.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(v: &[&str]) -> Vec<Vec<String>> {
        v.iter()
            .map(|w| w.split_whitespace().map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn catches_planted_nouns_and_ignores_comments() {
        let e = entries(&["road", "main way", "lake", "bridge", "waterfront"]);
        let src = r##"
// a road in a comment
/* lakes /* nested rivers */ and a bridge */
/// a doc comment about a main way
fn roadway() {}
fn width_of(road_width: u32) {}
struct MainWays;
const S: &str = "two lakes";
const R: &str = r#"the-waterfront"#;
fn bridges() -> &'static str { "" }
"##;
        let got: Vec<(usize, String)> = scan_source(src, &e)
            .expect("lexes")
            .into_iter()
            .map(|(l, w, _)| (l, w))
            .collect();
        assert_eq!(
            got,
            vec![
                (6, "road".to_string()),
                (7, "main way".to_string()),
                (8, "lake".to_string()),
                (9, "waterfront".to_string()),
                (10, "bridge".to_string()),
            ]
        );
    }

    #[test]
    fn allow_list_passes() {
        let v = Vocabulary::parse(
            "[deny]\nwords=[\"tree\",\"road\"]\ncrates=[]\nexcluded=[]\n[allow.tree]\nsense=\"x\"\n",
        )
        .expect("parses");
        let e = v.enforced();
        assert_eq!(e, entries(&["road"]));
        let got = scan_source("let t: BTreeMap<u8, u8>; let tree = 1; let roads = 2;", &e)
            .expect("lexes");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn plural_rules() {
        assert!(word_matches("roads", "road"));
        assert!(word_matches("beaches", "beach"));
        assert!(!word_matches("roadway", "road"));
        assert!(!word_matches("railroad", "road"));
    }

    #[test]
    fn self_test_passes_on_real_vocabulary() {
        let root = crate::repo_root();
        let v = Vocabulary::load(&root).expect("vocabulary loads");
        let mut o = Outcome::default();
        self_test_with(&v, &mut o);
        assert!(o.findings.is_empty(), "{:?}", o.findings);
    }
}
