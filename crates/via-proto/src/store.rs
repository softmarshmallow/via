//! Blobs: write-once objects named by digest (`spec/protocol.md` 4.1).
//!
//! A blob is written to `<objects>/tmp/`, then moved to `<objects>/ab/<digest>` by a move that never
//! replaces. Nothing is ever replaced, appended to or deleted while a build runs, so a mapped blob
//! never changes under a reader. That is also what keeps Windows working: a mapped file there
//! cannot be replaced or deleted.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use memmap2::Mmap;

/// The store's folder of objects, and which blobs this build has already checked against their
/// names (each is checked once per build, when first read).
///
/// A name enters `verified` only when this store hashed the very bytes that sit at `ab/<digest>`:
/// either it hashed them on a map, or it hashed a fresh private copy and its own no-replace move
/// put that copy there. A blob that already existed when a copy was settled stays unverified, so a
/// copy damaged in the store is still caught on its first map (4.1).
pub struct Store {
    objects: PathBuf,
    verified: Mutex<BTreeSet<String>>,
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    /// Not 64 lower-case hex digits.
    BadName(String),
    /// The bytes do not digest to the name: `protocol.blob_digest` from a step,
    /// `graph.store_corrupt` in the store.
    Digest {
        name: String,
        actual: String,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "{e}"),
            StoreError::BadName(n) => write!(f, "{n:?} is not a blob name"),
            StoreError::Digest { name, actual } => {
                write!(f, "blob {name} holds bytes whose digest is {actual}")
            }
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

/// The digest of bytes as the store names them: BLAKE3, 64 lower-case hex digits.
pub fn digest(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn is_name(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Store {
    /// Opens (and creates) `objects` and its `tmp/` folder.
    pub fn open(objects: impl Into<PathBuf>) -> io::Result<Store> {
        let objects = objects.into();
        fs::create_dir_all(objects.join("tmp"))?;
        Ok(Store {
            objects,
            verified: Mutex::new(BTreeSet::new()),
        })
    }

    pub fn objects(&self) -> &Path {
        &self.objects
    }

    /// `<objects>/ab/<digest>`.
    pub fn path_of(&self, name: &str) -> PathBuf {
        self.objects.join(&name[..2]).join(name)
    }

    /// A fresh path under `tmp/`. Unique within the machine: process id and a counter, so two
    /// builds sharing a store never collide.
    pub fn tmp_path(&self) -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.objects
            .join("tmp")
            .join(format!("{}-{}", std::process::id(), n))
    }

    /// Writes bytes as a blob and returns its name. A blob that exists already is kept as it is,
    /// and is checked on its first map like any blob this store did not write.
    pub fn put(&self, bytes: &[u8]) -> io::Result<String> {
        let name = digest(bytes);
        // A blob of that name is kept whatever this call writes (`settle` never replaces), so
        // it is not written again: an answer replayed in a later build (`graph.md` 5.1) names
        // the blobs it named when first given.
        if self.path_of(&name).is_file() {
            return Ok(name);
        }
        let tmp = self.tmp_path();
        let written = File::create(&tmp).and_then(|mut f| {
            f.write_all(bytes)?;
            f.sync_all()
        });
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        if self.settle(&tmp, &name)? == Settled::Created {
            self.mark_verified(&name);
        }
        Ok(name)
    }

    /// Takes a file a step wrote into its `out` folder into the store. It is copied into `tmp/`
    /// first and the copy is digested and settled: a worker lives for the whole build (3.2), so the
    /// step, a thread it left running, or a link it made to a file of its own could still write
    /// through the `out` file after this call. Linking it in would let a stored blob change under
    /// a reader. One copy is the price; the file in `out` is left for the call folder's deletion.
    pub fn adopt(&self, file: &Path) -> Result<String, StoreError> {
        let tmp = self.tmp_path();
        let name = match fs::copy(file, &tmp).and_then(|_| digest_file(&tmp)) {
            Ok(n) => n,
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                return Err(e.into());
            }
        };
        if self.settle(&tmp, &name)? == Settled::Created {
            self.mark_verified(&name);
        }
        Ok(name)
    }

    /// Moves `tmp` to the blob's name by a move that never replaces. If the name exists, the new
    /// copy is deleted and the old one kept; the caller learns which, since only a copy it made
    /// itself is known to hold the right bytes.
    fn settle(&self, tmp: &Path, name: &str) -> io::Result<Settled> {
        let dest = self.path_of(name);
        let made = dest
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| move_no_replace(tmp, &dest));
        match made {
            Ok(()) => Ok(Settled::Created),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                fs::remove_file(tmp)?;
                Ok(Settled::Existed)
            }
            Err(e) => {
                let _ = fs::remove_file(tmp);
                Err(e)
            }
        }
    }

    fn mark_verified(&self, name: &str) {
        self.verified
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(name.to_owned());
    }

    /// Maps a blob read-only. The first map in this store's life checks the bytes against the
    /// name; later maps do not, since a blob is never rewritten while a build runs.
    pub fn map(&self, name: &str) -> Result<Mmap, StoreError> {
        if !is_name(name) {
            return Err(StoreError::BadName(name.to_owned()));
        }
        let f = File::open(self.path_of(name))?;
        // SAFETY: blobs are write-once and never modified or deleted while a build runs (4.1), so
        // the mapping cannot change under us.
        let map = unsafe { Mmap::map(&f)? };
        let known = self
            .verified
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(name);
        if !known {
            let actual = digest(&map);
            if actual != name {
                return Err(StoreError::Digest {
                    name: name.to_owned(),
                    actual,
                });
            }
            self.mark_verified(name);
        }
        Ok(map)
    }
}

/// Whether a settle moved the new copy into place or found the name taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Settled {
    Created,
    Existed,
}

/// The digest of a file that only this store can write (a `tmp/` copy of its own).
fn digest_file(path: &Path) -> io::Result<String> {
    let f = File::open(path)?;
    if f.metadata()?.len() == 0 {
        return Ok(digest(&[]));
    }
    // SAFETY: the file is this store's private copy under `tmp/`, which nothing else writes.
    let map = unsafe { Mmap::map(&f)? };
    Ok(digest(&map))
}

/// `link` then `unlink`: `link` fails with `AlreadyExists` instead of replacing.
#[cfg(unix)]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::hard_link(from, to)?;
    fs::remove_file(from)
}

/// `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`. `std::fs::rename` replaces on Windows, which
/// fails on a mapped file, so it is not used.
#[cfg(windows)]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let wide = |p: &Path| -> Vec<u16> { p.as_os_str().encode_wide().chain(Some(0)).collect() };
    let (f, t) = (wide(from), wide(to));
    // SAFETY: both are NUL-terminated UTF-16 strings that outlive the call.
    let ok = unsafe { MoveFileExW(f.as_ptr(), t.as_ptr(), 0) };
    if ok == 0 {
        let e = io::Error::last_os_error();
        // ERROR_ALREADY_EXISTS (183) and ERROR_FILE_EXISTS (80) map to AlreadyExists in std.
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_names_by_blake3_and_never_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("objects")).unwrap();
        let a = store.put(b"abc").unwrap();
        assert_eq!(
            a,
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert_eq!(fs::read(store.path_of(&a)).unwrap(), b"abc");
        assert_eq!(store.put(b"abc").unwrap(), a);
        // tmp/ is left empty: the second copy was deleted, the first kept.
        assert_eq!(
            fs::read_dir(dir.path().join("objects/tmp"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(&store.map(&a).unwrap()[..], b"abc");
    }

    #[test]
    fn a_damaged_blob_is_caught_on_first_map() {
        let dir = tempfile::tempdir().unwrap();
        let objects = dir.path().join("objects");
        let name = Store::open(&objects).unwrap().put(b"abc").unwrap();
        fs::write(objects.join(&name[..2]).join(&name), b"abd").unwrap();
        let fresh = Store::open(&objects).unwrap();
        assert!(matches!(fresh.map(&name), Err(StoreError::Digest { .. })));
        assert!(matches!(fresh.map("nope"), Err(StoreError::BadName(_))));
    }

    #[test]
    fn a_damaged_blob_settled_over_is_still_caught() {
        let dir = tempfile::tempdir().unwrap();
        let objects = dir.path().join("objects");
        let name = Store::open(&objects).unwrap().put(b"abc").unwrap();
        fs::write(objects.join(&name[..2]).join(&name), b"abd").unwrap();
        // The same bytes put, then adopted, through one store: the damaged copy is kept (never
        // replaced) and was never read, so the first map must check it.
        let store = Store::open(&objects).unwrap();
        assert_eq!(store.put(b"abc").unwrap(), name);
        let out = dir.path().join("x");
        fs::write(&out, b"abc").unwrap();
        assert_eq!(store.adopt(&out).unwrap(), name);
        assert!(matches!(store.map(&name), Err(StoreError::Digest { .. })));
    }

    #[test]
    fn adopt_copies_an_out_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("objects")).unwrap();
        let out = dir.path().join("x.npy");
        fs::write(&out, b"field").unwrap();
        let name = store.adopt(&out).unwrap();
        assert_eq!(name, digest(b"field"));
        assert_eq!(fs::read(store.path_of(&name)).unwrap(), b"field");
        // A write through the out file after adoption does not reach the blob.
        fs::write(&out, b"fiel!").unwrap();
        assert_eq!(&store.map(&name).unwrap()[..], b"field");
        fs::write(&out, b"field").unwrap();
        assert_eq!(store.adopt(&out).unwrap(), name);
        assert_eq!(
            fs::read_dir(dir.path().join("objects/tmp"))
                .unwrap()
                .count(),
            0
        );
    }
}
