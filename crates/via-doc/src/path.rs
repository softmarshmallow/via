//! Paths in text (`spec/documents.md` 7.2): their form, and opening them with exact case.

use std::path::{Component, Path, PathBuf};

/// Why a written path is not of the portable form.
pub(crate) fn form(p: &str) -> Result<Vec<String>, String> {
    if p.is_empty() {
        return Err("the path is empty".into());
    }
    if p.contains('\\') {
        return Err("it holds `\\`; paths in text use `/` on every system".into());
    }
    let b = p.as_bytes();
    if b[0] == b'/' {
        return Err("it is absolute; write it relative to the file that names it".into());
    }
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return Err(
            "it starts with a drive letter; write it relative to the file that names it".into(),
        );
    }
    let mut parts = Vec::new();
    for part in p.split('/') {
        if part.is_empty() {
            return Err("it has an empty part (`//` or a trailing `/`)".into());
        }
        if part != "." && part != ".." {
            if let Some(c) = part
                .chars()
                .find(|c| !(' '..='~').contains(c) || "\\:*?\"<>|".contains(*c))
            {
                return Err(format!(
                    "the part `{part}` holds `{}`, which is not portable",
                    c.escape_default()
                ));
            }
            if part.ends_with(' ') || part.ends_with('.') {
                return Err(format!("the part `{part}` ends in a space or a `.`"));
            }
            let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
            let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.len() == 4
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'));
            if device {
                return Err(format!("the part `{part}` is a Windows device name"));
            }
        }
        parts.push(part.to_string());
    }
    Ok(parts)
}

/// Resolves `..` and `.` parts by text against a folder given as parts relative to the world
/// folder. The result may start with `..` parts, which climb above the world folder.
pub(crate) fn join(folder: &[String], parts: &[String]) -> Vec<String> {
    let mut out: Vec<String> = folder.to_vec();
    for p in parts {
        match p.as_str() {
            "." => {}
            ".." => {
                if out.last().is_some_and(|l| l != "..") {
                    out.pop();
                } else {
                    out.push("..".into());
                }
            }
            _ => out.push(p.clone()),
        }
    }
    out
}

/// What the walk found wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Walk {
    /// The part at this index differs in case from the name on disk.
    Case { index: usize, on_disk: String },
    /// The part at this index is not there; the nearest entry of its folder, if any.
    Missing { index: usize, entries: Vec<String> },
}

/// Opens `parts` from `base` part by part (7.2): an entry equal bytewise is followed; one equal
/// after ASCII case folding is [`Walk::Case`]; none is [`Walk::Missing`]. `..` parts are followed
/// as written. A folder that cannot be listed is passed through by existence alone.
pub(crate) fn walk(base: &Path, parts: &[String]) -> Result<PathBuf, Walk> {
    let mut at = base.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        if part == ".." || part == "." {
            at.push(part);
            continue;
        }
        match std::fs::read_dir(&at) {
            Ok(rd) => {
                let mut names: Vec<String> = rd
                    .filter_map(Result::ok)
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                // Sorted by bytes, so the name a case mismatch reports does not follow
                // directory order.
                names.sort();
                if names.iter().any(|n| n == part) {
                    at.push(part);
                } else if let Some(n) = names.iter().find(|n| n.eq_ignore_ascii_case(part)) {
                    return Err(Walk::Case {
                        index,
                        on_disk: n.clone(),
                    });
                } else {
                    return Err(Walk::Missing {
                        index,
                        entries: names,
                    });
                }
            }
            Err(_) => {
                let next = at.join(part);
                if next.symlink_metadata().is_ok() {
                    at = next;
                } else {
                    return Err(Walk::Missing {
                        index,
                        entries: Vec::new(),
                    });
                }
            }
        }
    }
    Ok(at)
}

/// A `file` value of text opened from the folder of the document that names it (7.2), for a
/// reader outside this crate (a field's `file`, `fields.md` 4.1). The fault is the code, the
/// message and the hint: `doc.path_form`, `doc.path_case` or `doc.file_missing`.
pub fn open_file(
    folder: &Path,
    written: &str,
) -> Result<PathBuf, (via_diag::Code, String, String)> {
    use via_diag::codes;
    let parts = form(written).map_err(|why| {
        (
            codes::doc::PATH_FORM,
            format!("the path `{written}` is not portable: {why}"),
            "write a relative path with `/`, of printable ASCII parts".to_string(),
        )
    })?;
    walk(folder, &parts).map_err(|w| match w {
        Walk::Case { index, on_disk } => (
            codes::doc::PATH_CASE,
            format!(
                "the part `{}` of `{written}` is `{on_disk}` on disk; case must match on every system",
                parts.get(index).cloned().unwrap_or_default()
            ),
            format!("write `{on_disk}`"),
        ),
        Walk::Missing { index, entries } => {
            let part = parts.get(index).cloned().unwrap_or_default();
            // Hidden entries (`.via`, a tool's own folders) are not names a text means, and
            // listing them would tie the hint to whatever else shares the folder (R26).
            let visible = entries
                .iter()
                .map(String::as_str)
                .filter(|e| !e.starts_with('.') || part.starts_with('.'));
            let near = via_diag::nearest::nearest(&part, visible);
            (
                codes::doc::FILE_MISSING,
                format!(
                    "the file `{written}` is not found: no `{part}` in `{}`",
                    folder.join(parts[..index].join("/")).display()
                ),
                match near {
                    via_diag::nearest::Nearest::Found(n) => format!("write `{n}`"),
                    other => other.hint("correct the path"),
                },
            )
        }
    })
}

/// A command-line path as a base and parts to walk: an absolute path walks from its root, a
/// relative one from the working folder.
pub(crate) fn split_cli(p: &Path) -> (PathBuf, Vec<String>) {
    let mut base = PathBuf::new();
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => base.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => parts.push("..".to_string()),
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
        }
    }
    if base.as_os_str().is_empty() {
        base = PathBuf::from(".");
    }
    (base, parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_form() {
        assert!(form("layers/d.toml").is_ok());
        assert!(form("../shared/n.toml").is_ok());
        // Case path_form.
        for bad in [
            "layers\\d.toml",
            "C:x.toml",
            "c:/x.toml",
            "/abs.toml",
            "//host/x",
            "aux.toml",
            "Com3",
            "a/b.",
            "a//b",
            "a/b c ",
            "q?.toml",
        ] {
            assert!(form(bad).is_err(), "{bad}");
        }
        assert!(form("com10.toml").is_ok());
        assert!(form("~/x.toml").is_ok());
    }

    #[test]
    fn dots_resolve_by_text() {
        let j = join(
            &["layers".into()],
            &["..".into(), "x".into(), ".".into(), "y".into()],
        );
        assert_eq!(j, vec!["x", "y"]);
        let j = join(&[], &["..".into(), "..".into(), "z".into()]);
        assert_eq!(j, vec!["..", "..", "z"]);
    }
}
