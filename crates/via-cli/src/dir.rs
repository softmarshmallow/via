//! Writing and replacing a build directory (`build.md` 13): what counts as written by `via`,
//! refusing to delete anything else, recovering from a build killed between its renames, and
//! replacing the directory whole by writing a partial folder beside it and swapping it in.
//!
//! The lock (`<out>.lock`) is `via-graph`'s [`via_graph::DirLock`]; the caller holds it around
//! every call here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use via_diag::{codes, Diag, Site, Subject};

use crate::txt;

/// The test switch of `build.md` case 24: with this variable set, `via build` stops the process
/// right after the first rename of the swap, as a kill there would.
pub const STOP_BETWEEN_RENAMES: &str = "VIA_TEST_STOP_BETWEEN_RENAMES";

/// `<out><suffix>` beside `out`.
pub fn sibling(out: &Path, suffix: &str) -> PathBuf {
    let mut name = out
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    out.with_file_name(name)
}

fn shown(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

pub fn not_a_build(p: &Path, why: &str) -> Diag {
    Diag::new(
        codes::build::NOT_A_BUILD,
        Subject::World,
        Site::World,
        txt(format!("`{}` {why}; nothing was deleted", shown(p))),
        via_diag::text!("name an empty or new folder"),
    )
    .with("path", shown(p))
}

pub fn write_failed(p: &Path, e: &std::io::Error) -> Diag {
    Diag::new(
        codes::build::WRITE_FAILED,
        Subject::World,
        Site::World,
        txt(format!("cannot write `{}`: {e}", shown(p))),
        via_diag::text!("free space, or fix the folder's permissions"),
    )
    .with("path", shown(p))
}

/// A folder holding a `manifest.json` with a `format` starting `via-build-`, or a `.previous` of
/// the form of 11.2.
pub fn written_by_via(dir: &Path) -> bool {
    if let Ok(t) = std::fs::read_to_string(dir.join("manifest.json")) {
        if let Ok(j) = crate::json::parse(&t) {
            if j.get("format")
                .and_then(crate::json::J::as_str)
                .is_some_and(|f| f.starts_with("via-build-"))
            {
                return true;
            }
        }
    }
    if let Ok(t) = std::fs::read_to_string(dir.join(".previous")) {
        if let Ok(j) = crate::json::parse(&t) {
            return j.get("format").and_then(crate::json::J::as_str) == Some(PREVIOUS_FORMAT);
        }
    }
    false
}

/// The format of `.previous`.
pub const PREVIOUS_FORMAT: &str = "via-previous-0";

fn is_empty_dir(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut d| d.next().is_none())
}

/// Checks that `p` may be replaced: absent, an empty folder, or a folder written by `via`.
fn replaceable(p: &Path) -> Result<(), Diag> {
    match std::fs::symlink_metadata(p) {
        Err(_) => Ok(()),
        Ok(m) if !m.is_dir() => Err(not_a_build(p, "is a file")),
        Ok(_) if is_empty_dir(p) || written_by_via(p) => Ok(()),
        Ok(_) => Err(not_a_build(p, "is a non-empty folder `via` did not write")),
    }
}

/// The checks of 13 before anything is touched: `--out`, `.partial` and `.old`, and an `--out`
/// that is the world file's folder or holds it.
pub fn check_out(out: &Path, world_dir: &Path) -> Result<(), Diag> {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (o, w) = (canon(out), canon(world_dir));
    if w.starts_with(&o) {
        return Err(not_a_build(out, "is the world file's folder or holds it"));
    }
    replaceable(out)?;
    replaceable(&sibling(out, ".partial"))?;
    replaceable(&sibling(out, ".old"))?;
    Ok(())
}

/// Recovery at start, under the lock: a missing directory whose `.old` was written by `via` is
/// renamed back; then any `.partial` and `.old` written by `via` (or empty) are deleted.
pub fn recover(out: &Path) {
    let old = sibling(out, ".old");
    if !out.exists() && old.is_dir() && written_by_via(&old) {
        let _ = std::fs::rename(&old, out);
    }
    for p in [sibling(out, ".partial"), old] {
        if p.is_dir() && (written_by_via(&p) || is_empty_dir(&p)) {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

/// Replaces `out` whole with `files` (build paths to bytes), `.previous` written first.
pub fn replace(out: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), Diag> {
    let partial = sibling(out, ".partial");
    let old = sibling(out, ".old");
    if partial.exists() {
        std::fs::remove_dir_all(&partial).map_err(|e| write_failed(&partial, &e))?;
    }
    std::fs::create_dir_all(&partial).map_err(|e| write_failed(&partial, &e))?;
    let mut order: Vec<(&String, &Vec<u8>)> = files.iter().collect();
    order.sort_by_key(|(p, _)| p.as_str() != ".previous");
    for (path, bytes) in order {
        let target = partial.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| write_failed(parent, &e))?;
        }
        std::fs::write(&target, bytes).map_err(|e| write_failed(&target, &e))?;
    }
    let had_old = out.exists();
    if had_old {
        std::fs::rename(out, &old).map_err(|e| busy_or_failed(out, &e))?;
        if std::env::var_os(STOP_BETWEEN_RENAMES).is_some() {
            std::process::exit(9);
        }
    }
    if let Err(e) = std::fs::rename(&partial, out) {
        if had_old {
            let _ = std::fs::rename(&old, out);
        }
        return Err(busy_or_failed(out, &e));
    }
    if had_old {
        let _ = std::fs::remove_dir_all(&old);
    }
    Ok(())
}

fn busy_or_failed(p: &Path, e: &std::io::Error) -> Diag {
    if e.kind() == std::io::ErrorKind::PermissionDenied && cfg!(windows) {
        Diag::new(
            codes::build::DIR_BUSY,
            Subject::World,
            Site::World,
            txt(format!(
                "`{}` could not be replaced: a file in it is open ({e})",
                shown(p)
            )),
            via_diag::text!("close the program holding it, or build to another `--out`"),
        )
        .with("path", shown(p))
    } else {
        write_failed(p, e)
    }
}

/// Every file under `dir` as a build path, sorted.
pub fn listing(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, d: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("via-cli-dir-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn replaces_whole_and_leaves_nothing_beside() {
        let root = tmp("replace");
        let out = root.join("b");
        let mut files = BTreeMap::new();
        files.insert(".previous".to_string(), b"{}\n".to_vec());
        files.insert("a/x.txt".to_string(), b"1".to_vec());
        replace(&out, &files).unwrap();
        files.remove("a/x.txt");
        files.insert("y.txt".to_string(), b"2".to_vec());
        replace(&out, &files).unwrap();
        assert_eq!(listing(&out), vec![".previous", "y.txt"]);
        assert!(!sibling(&out, ".old").exists());
        assert!(!sibling(&out, ".partial").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_a_folder_it_did_not_write() {
        let root = tmp("refuse");
        let out = root.join("b");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("notes.txt"), b"mine").unwrap();
        let world = root.join("w");
        std::fs::create_dir_all(&world).unwrap();
        assert!(check_out(&out, &world).is_err());
        assert!(check_out(&root, &world).is_err());
        assert!(out.join("notes.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovers_an_old_directory() {
        let root = tmp("recover");
        let out = root.join("b");
        let old = sibling(&out, ".old");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(
            old.join(".previous"),
            b"{\"format\":\"via-previous-0\",\"id\":null,\"manifest\":null}\n",
        )
        .unwrap();
        recover(&out);
        assert!(out.join(".previous").exists());
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
