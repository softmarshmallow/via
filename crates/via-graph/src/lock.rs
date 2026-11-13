//! One build at a time per build directory (`spec/graph.md` 8): an operating-system advisory
//! lock on `<directory>.lock` beside it (`flock` on macOS and Linux, `LockFileEx` on Windows,
//! through the standard library), released by the system when the process ends, killed or not.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use via_diag::{codes, Diag, Site, Subject, Text};

/// A held lock; dropping it releases the lock.
#[derive(Debug)]
pub struct DirLock {
    _file: File,
    path: PathBuf,
}

/// `<directory>.lock` beside the build directory.
pub fn lock_path(directory: &Path) -> PathBuf {
    let mut name = directory
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".lock");
    directory.with_file_name(name)
}

impl DirLock {
    /// Takes the lock without waiting. Another holder is `graph.busy`; a lock file that cannot
    /// be opened is `graph.busy` too, naming the reason, since the build cannot know it is alone.
    pub fn acquire(directory: &Path) -> Result<DirLock, Diag> {
        let path = lock_path(directory);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| busy(&path, &format!("its lock file cannot be opened ({e})")))?;
        match file.try_lock() {
            Ok(()) => Ok(DirLock { _file: file, path }),
            Err(TryLockError::WouldBlock) => Err(busy(&path, "another build holds its lock")),
            Err(TryLockError::Error(e)) => {
                Err(busy(&path, &format!("its lock cannot be taken ({e})")))
            }
        }
    }

    /// The lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn busy(path: &Path, why: &str) -> Diag {
    Diag::new(
        codes::graph::BUSY,
        Subject::World,
        Site::World,
        Text::new(format!(
            "the build directory cannot be built now: {why} (`{}`)",
            path.display()
        ))
        .unwrap_or_else(|_| via_diag::text!("the build directory is busy")),
        via_diag::text!("wait for the other build to finish, or build into another --out"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_lock_is_busy_and_release_frees_it() {
        let dir = std::env::temp_dir().join(format!("via-graph-lock-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let out = dir.join("build");
        let a = DirLock::acquire(&out).expect("first");
        assert_eq!(a.path(), dir.join("build.lock"));
        let b = DirLock::acquire(&out).expect_err("second");
        assert_eq!(b.code().to_string(), "graph.busy");
        drop(a);
        assert!(DirLock::acquire(&out).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
