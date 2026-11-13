//! The store (`spec/graph.md` 3.2): a folder of write-once files named by digest.
//!
//! | Entry | Path |
//! |---|---|
//! | object | `objects/ab/<value digest>` |
//! | key record | `keys/ab/<key>` |
//! | trace | `traces/<static key>/<trace digest>` |
//! | build record | `builds/<build key>` |
//!
//! Every file is written to `tmp/` and linked into place without replacing: if the target exists
//! the temporary file is deleted and the existing one kept, so the first writer wins and equal
//! names hold equal bytes. The one exception is a step's trace entry under `--rerun-step`
//! ([`Store::replace_trace`]), renamed over the old one. An object is checked against its name when first read in a build; a
//! mismatch, or an entry that cannot be read, is `graph.store_corrupt`, and the entry is dropped.
//! A store that cannot be written is `graph.store_unavailable` once, and the build keeps nothing
//! more. Where the store lives is `build.md` 13 (`.via/store/` beside the world, or `--store`).

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use via_diag::{codes, Diag, Site, Subject, Text};

use crate::digest::{Dec, Digest, Enc};

/// The store's folder name under the world's folder (`build.md` 13).
pub const DEFAULT_STORE: &str = ".via/store";

/// The store of one build.
pub struct Store {
    root: PathBuf,
    unavailable: AtomicBool,
    verified: Mutex<BTreeSet<Digest>>,
    run: Mutex<Vec<Diag>>,
}

/// A key record: the value digest and the node's diagnostics, encoded by its codec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRecord {
    /// The value digest.
    pub value: Digest,
    /// The diagnostics' encoding (`codec.rs`).
    pub diags: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Written {
    Created,
    Existed,
    Failed,
}

static TMP: AtomicU64 = AtomicU64::new(0);

impl Store {
    /// Opens the store at `root`, creating its folders. A store that cannot be created is open
    /// but unavailable: reads find nothing and writes keep nothing.
    pub fn open(root: impl Into<PathBuf>) -> Store {
        // Absolute, so paths handed to a step (which runs in its own folder) still resolve.
        let root = root.into();
        let root = std::path::absolute(&root).unwrap_or(root);
        let s = Store {
            root,
            unavailable: AtomicBool::new(false),
            verified: Mutex::new(BTreeSet::new()),
            run: Mutex::new(Vec::new()),
        };
        if let Err(e) = fs::create_dir_all(s.root.join("tmp")) {
            s.unavailable(&e);
        }
        s
    }

    /// The store's folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The run diagnostics this store raised (`graph.store_corrupt`, `graph.store_unavailable`),
    /// taken out in the order raised.
    pub fn take_run_diags(&self) -> Vec<Diag> {
        self.run
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }

    /// True once a write failed: this build keeps nothing more.
    pub fn is_unavailable(&self) -> bool {
        self.unavailable.load(Ordering::SeqCst)
    }

    fn push(&self, d: Diag) {
        if let Ok(mut v) = self.run.lock() {
            v.push(d);
        }
    }

    fn unavailable(&self, e: &io::Error) {
        if !self.unavailable.swap(true, Ordering::SeqCst) {
            self.push(Diag::new(
                codes::graph::STORE_UNAVAILABLE,
                Subject::World,
                Site::World,
                text(format!(
                    "the store at `{}` cannot be written ({e}); this build keeps nothing more",
                    self.root.display()
                )),
                via_diag::text!(
                    "free space or grant write permission on the store, or pass --store with a folder that can be written"
                ),
            ));
        }
    }

    fn corrupt(&self, entry: &str, why: &str) {
        self.push(Diag::new(
            codes::graph::STORE_CORRUPT,
            Subject::World,
            Site::World,
            text(format!(
                "the store entry `{entry}` {why}; it was dropped and its node ran again"
            )),
            via_diag::text!(
                "nothing to do: the entry is written again; if this repeats, check the disk"
            ),
        ));
    }

    /// `objects/ab/<digest>`.
    pub fn object_path(&self, d: &Digest) -> PathBuf {
        self.root.join("objects").join(d.fan()).join(d.hex())
    }

    fn key_path(&self, k: &Digest) -> PathBuf {
        self.root.join("keys").join(k.fan()).join(k.hex())
    }

    fn trace_dir(&self, static_key: &Digest) -> PathBuf {
        self.root.join("traces").join(static_key.hex())
    }

    fn build_path(&self, k: &Digest) -> PathBuf {
        self.root.join("builds").join(k.hex())
    }

    fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// Writes `bytes` at `target` once.
    fn write_once(&self, target: &Path, bytes: &[u8]) -> Written {
        if self.is_unavailable() {
            return Written::Failed;
        }
        if target.exists() {
            return Written::Existed;
        }
        let n = TMP.fetch_add(1, Ordering::Relaxed);
        let tmp = self
            .root
            .join("tmp")
            .join(format!("{}-{n}", std::process::id()));
        let result = (|| -> io::Result<Written> {
            if let Some(dir) = target.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::create_dir_all(self.root.join("tmp"))?;
            let mut f = fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            match fs::hard_link(&tmp, target) {
                Ok(()) => Ok(Written::Created),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(Written::Existed),
                // A file system without hard links: a rename, which can only ever put equal
                // bytes over equal bytes, since names are digests.
                Err(_) if !target.exists() => fs::rename(&tmp, target).map(|()| Written::Created),
                Err(_) => Ok(Written::Existed),
            }
        })();
        let _ = fs::remove_file(&tmp);
        match result {
            Ok(w) => w,
            Err(e) => {
                self.unavailable(&e);
                Written::Failed
            }
        }
    }

    fn read(&self, p: &Path) -> Result<Option<Vec<u8>>, io::Error> {
        match fs::read(p) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn drop_entry(&self, p: &Path, why: &str) {
        self.corrupt(&self.rel(p), why);
        let _ = fs::remove_file(p);
    }

    /// Puts an object; its name is the digest of its bytes. Returns the digest.
    pub fn put_object(&self, bytes: &[u8]) -> Digest {
        let d = Digest::of(bytes);
        // Only when this store hashed the very bytes now in place: a file that existed already
        // stays unchecked until its first read.
        if self.write_once(&self.object_path(&d), bytes) == Written::Created {
            if let Ok(mut v) = self.verified.lock() {
                v.insert(d);
            }
        }
        d
    }

    /// Reads an object, checked against its name on its first read in this build. A missing
    /// object is `None`; a damaged one is dropped with `graph.store_corrupt` and is `None`.
    pub fn get_object(&self, d: &Digest) -> Option<Vec<u8>> {
        let p = self.object_path(d);
        match self.read(&p) {
            Ok(None) => None,
            Err(_) => {
                self.drop_entry(&p, "could not be read");
                None
            }
            Ok(Some(b)) => {
                let seen = self.verified.lock().is_ok_and(|v| v.contains(d));
                if !seen && Digest::of(&b) != *d {
                    self.drop_entry(&p, "does not match its name");
                    return None;
                }
                if let Ok(mut v) = self.verified.lock() {
                    v.insert(*d);
                }
                Some(b)
            }
        }
    }

    /// True when the object is present (not read, not checked).
    pub fn has_object(&self, d: &Digest) -> bool {
        self.object_path(d).is_file()
    }

    /// Puts a key record.
    pub fn put_key(&self, key: &Digest, rec: &KeyRecord) {
        let mut e = Enc::new();
        e.str("via-keyrec-1").digest(&rec.value).bytes(&rec.diags);
        self.write_once(&self.key_path(key), &e.finish());
    }

    /// Reads a key record; a damaged one is dropped with `graph.store_corrupt`.
    pub fn get_key(&self, key: &Digest) -> Option<KeyRecord> {
        let p = self.key_path(key);
        let b = match self.read(&p) {
            Ok(b) => b?,
            Err(_) => {
                self.drop_entry(&p, "could not be read");
                return None;
            }
        };
        let rec = (|| {
            let mut d = Dec::new(&b);
            if d.str()? != "via-keyrec-1" {
                return None;
            }
            let value = d.digest()?;
            let diags = d.bytes()?.to_vec();
            d.done().then_some(KeyRecord { value, diags })
        })();
        if rec.is_none() {
            self.drop_entry(&p, "is not a key record");
        }
        rec
    }

    /// Drops a key record whose value turned out unreadable, so its node runs again.
    pub fn drop_key(&self, key: &Digest) {
        let _ = fs::remove_file(self.key_path(key));
    }

    /// Puts one trace entry of a step under its static key, named by its trace digest. The
    /// entry's bytes are the caller's; [`Store::traces`] hands them back with their names, and
    /// the caller checks each against its name (the trace part of the entry).
    pub fn put_trace(&self, static_key: &Digest, trace: &Digest, bytes: &[u8]) {
        self.write_once(&self.trace_dir(static_key).join(trace.hex()), bytes);
    }

    /// Every trace entry under a static key as (name, bytes), newest first (by modification
    /// time, then name). Not checked: see [`Store::drop_trace`].
    pub fn traces(&self, static_key: &Digest) -> Vec<(Digest, Vec<u8>)> {
        let dir = self.trace_dir(static_key);
        let Ok(rd) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut found: Vec<(std::time::SystemTime, String, PathBuf)> = Vec::new();
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            let t = ent
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            found.push((t, name, ent.path()));
        }
        found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let mut out = Vec::new();
        for (_, name, p) in found {
            let Some(d) = Digest::from_hex(&name) else {
                continue;
            };
            match self.read(&p) {
                Ok(Some(b)) => out.push((d, b)),
                Ok(None) => {}
                Err(_) => self.drop_entry(&p, "could not be read"),
            }
        }
        out
    }

    /// Puts a trace entry over the one of the same name, if any: the one place the store
    /// overwrites. A step forced by `--rerun-step` whose value changed for a trace it had
    /// (`graph.md` 5.2) keeps its new value, so later builds take it. Written to a temporary name
    /// and renamed over, so a reader sees the old entry or the new one, never half of one.
    pub fn replace_trace(&self, static_key: &Digest, trace: &Digest, bytes: &[u8]) {
        if self.is_unavailable() {
            return;
        }
        let target = self.trace_dir(static_key).join(trace.hex());
        let n = TMP.fetch_add(1, Ordering::Relaxed);
        let tmp = self
            .root
            .join("tmp")
            .join(format!("{}-{n}", std::process::id()));
        let result = (|| -> io::Result<()> {
            fs::create_dir_all(self.trace_dir(static_key))?;
            fs::create_dir_all(self.root.join("tmp"))?;
            let mut f = fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&tmp, &target)
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            self.unavailable(&e);
        }
    }

    /// Drops a damaged trace entry, with `graph.store_corrupt` saying why.
    pub fn drop_trace(&self, static_key: &Digest, trace: &Digest, why: &str) {
        let p = self.trace_dir(static_key).join(trace.hex());
        self.drop_entry(&p, why);
    }

    /// Puts a build record under its build key.
    pub fn put_build(&self, build_key: &Digest, bytes: &[u8]) {
        let mut e = Enc::new();
        e.str("via-buildrec-1")
            .digest(&Digest::of(bytes))
            .bytes(bytes);
        self.write_once(&self.build_path(build_key), &e.finish());
    }

    /// Reads a build record; a damaged one is dropped with `graph.store_corrupt`.
    pub fn get_build(&self, build_key: &Digest) -> Option<Vec<u8>> {
        let p = self.build_path(build_key);
        let b = match self.read(&p) {
            Ok(b) => b?,
            Err(_) => {
                self.drop_entry(&p, "could not be read");
                return None;
            }
        };
        let body = (|| {
            let mut d = Dec::new(&b);
            if d.str()? != "via-buildrec-1" {
                return None;
            }
            let sum = d.digest()?;
            let body = d.bytes()?.to_vec();
            (d.done() && Digest::of(&body) == sum).then_some(body)
        })();
        if body.is_none() {
            self.drop_entry(&p, "does not match its digest");
        }
        body
    }
}

fn text(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| via_diag::text!("a store entry"))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "via-graph-{name}-{}-{}",
            std::process::id(),
            TMP.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn objects_are_write_once_and_checked() {
        let root = scratch("objects");
        let s = Store::open(&root);
        let d = s.put_object(b"hello");
        assert_eq!(s.put_object(b"hello"), d);
        assert_eq!(s.get_object(&d).as_deref(), Some(&b"hello"[..]));
        // A byte flipped behind the store's back is found on the first read of the next build.
        fs::write(s.object_path(&d), b"hellp").unwrap();
        let s2 = Store::open(&root);
        assert!(s2.get_object(&d).is_none());
        let run = s2.take_run_diags();
        assert_eq!(run.len(), 1);
        assert_eq!(run[0].code().to_string(), "graph.store_corrupt");
        assert!(!s2.has_object(&d), "the entry is dropped");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn key_records_traces_and_builds() {
        let root = scratch("records");
        let s = Store::open(&root);
        let k = Digest::of(b"k");
        let rec = KeyRecord {
            value: Digest::of(b"v"),
            diags: vec![1, 2, 3],
        };
        s.put_key(&k, &rec);
        assert_eq!(s.get_key(&k), Some(rec));
        let t1 = Digest::of(b"one");
        s.put_trace(&k, &t1, b"entry");
        assert_eq!(s.traces(&k), vec![(t1, b"entry".to_vec())]);
        // A rerun's new value replaces the entry of the same name.
        s.put_trace(&k, &t1, b"other");
        assert_eq!(s.traces(&k), vec![(t1, b"entry".to_vec())]);
        s.replace_trace(&k, &t1, b"other");
        assert_eq!(s.traces(&k), vec![(t1, b"other".to_vec())]);
        s.drop_trace(&k, &t1, "does not match its name");
        assert!(s.traces(&k).is_empty());
        assert_eq!(s.take_run_diags().len(), 1);
        s.put_build(&k, b"record");
        assert_eq!(s.get_build(&k).as_deref(), Some(&b"record"[..]));
        fs::write(s.build_path(&k), b"junk").unwrap();
        assert!(s.get_build(&k).is_none());
        assert_eq!(s.take_run_diags().len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn unwritable_store_is_reported_once() {
        use std::os::unix::fs::PermissionsExt;
        let root = scratch("readonly");
        fs::create_dir_all(&root).unwrap();
        let s = Store::open(&root);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
        fs::set_permissions(root.join("tmp"), fs::Permissions::from_mode(0o500)).unwrap();
        s.put_object(b"a");
        s.put_object(b"b");
        let run = s.take_run_diags();
        fs::set_permissions(root.join("tmp"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(run.len(), 1);
        assert_eq!(run[0].code().to_string(), "graph.store_unavailable");
        assert!(s.is_unavailable());
        let _ = fs::remove_dir_all(&root);
    }
}
