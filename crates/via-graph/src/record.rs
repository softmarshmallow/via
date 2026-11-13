//! The build key, the build record and the null build (`spec/graph.md` 6.2).

use std::collections::BTreeMap;

use crate::digest::{Dec, Digest, Enc};
use crate::id::NodeId;
use crate::key::{digest_of_parts, Versions};
use crate::store::Store;

/// One node in a build record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordNode {
    /// Its key, when worked out.
    pub key: Option<Digest>,
    /// Its value digest, when it has one.
    pub value: Option<Digest>,
    /// Digests that entered its key with no node behind them (a leaf's `bytes`), for the
    /// `edge` cause of the next build.
    pub direct: Vec<(String, Digest)>,
}

/// The build record (6.2): the versions, every node's id, key and value digest, the build's
/// world diagnostics as emitted, the digest of each build file, and whether every step
/// completed. It also keeps each statement's digest, which `merge`'s value holds too, so the
/// `statements` cause needs no model type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRecord {
    /// The four versions.
    pub versions: Versions,
    /// Every node.
    pub nodes: BTreeMap<NodeId, RecordNode>,
    /// Every statement's digest by statement id.
    pub statements: BTreeMap<String, Digest>,
    /// The world diagnostics as emitted, in the caller's encoding (the canonical JSON records).
    pub world_diags: Vec<u8>,
    /// Each build file by path.
    pub files: BTreeMap<String, Digest>,
    /// Whether every step completed.
    pub complete: bool,
}

impl BuildRecord {
    /// The record's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Enc::new();
        e.str("via-build-record-1");
        self.versions.encode(&mut e);
        e.len(self.nodes.len());
        for (id, n) in &self.nodes {
            e.str(id.as_str());
            opt(&mut e, n.key.as_ref());
            opt(&mut e, n.value.as_ref());
            e.len(n.direct.len());
            for (l, d) in &n.direct {
                e.str(l).digest(d);
            }
        }
        e.len(self.statements.len());
        for (s, d) in &self.statements {
            e.str(s).digest(d);
        }
        e.bytes(&self.world_diags);
        e.len(self.files.len());
        for (p, d) in &self.files {
            e.str(p).digest(d);
        }
        e.byte(u8::from(self.complete));
        e.finish()
    }

    /// Reads a record; `None` when the bytes are not one.
    pub fn decode(b: &[u8]) -> Option<BuildRecord> {
        let mut d = Dec::new(b);
        if d.str()? != "via-build-record-1" {
            return None;
        }
        let versions = Versions {
            document: d.str()?,
            protocol: d.str()?,
            build: d.str()?,
            epoch: d.u64()?,
        };
        let mut nodes = BTreeMap::new();
        for _ in 0..d.count()? {
            let id = NodeId::parse(&d.str()?).ok()?;
            let key = read_opt(&mut d)?;
            let value = read_opt(&mut d)?;
            let mut direct = Vec::new();
            for _ in 0..d.count()? {
                direct.push((d.str()?, d.digest()?));
            }
            nodes.insert(id, RecordNode { key, value, direct });
        }
        let mut statements = BTreeMap::new();
        for _ in 0..d.count()? {
            statements.insert(d.str()?, d.digest()?);
        }
        let world_diags = d.bytes()?.to_vec();
        let mut files = BTreeMap::new();
        for _ in 0..d.count()? {
            files.insert(d.str()?, d.digest()?);
        }
        let complete = match d.byte()? {
            0 => false,
            1 => true,
            _ => return None,
        };
        d.done().then_some(BuildRecord {
            versions,
            nodes,
            statements,
            world_diags,
            files,
            complete,
        })
    }

    /// Keeps the record in the store under its build key.
    pub fn put(&self, store: &Store, build_key: &Digest) {
        store.put_build(build_key, &self.encode());
    }

    /// Reads the record of a build key from the store.
    pub fn get(store: &Store, build_key: &Digest) -> Option<BuildRecord> {
        BuildRecord::decode(&store.get_build(build_key)?)
    }
}

fn opt(e: &mut Enc, d: Option<&Digest>) {
    match d {
        None => e.byte(0),
        Some(d) => e.byte(1).digest(d),
    };
}

fn read_opt(d: &mut Dec) -> Option<Option<Digest>> {
    match d.byte()? {
        0 => Some(None),
        1 => Some(Some(d.digest()?)),
        _ => None,
    }
}

/// The parts of a build key (6.2). Everything enters by place and source path only, never an
/// absolute path; the caller writes each part as text.
#[derive(Clone, Debug, Default)]
pub struct BuildKeyParts {
    /// The requested changes, in the order requested.
    pub changes: Vec<String>,
    /// Each `source` and `input` path with its digest.
    pub files: Vec<(String, Digest)>,
    /// Each lookup the build made: the name, the path found, and each candidate before it
    /// that was absent, written as one line by the caller (`documents.md` 4).
    pub lookups: Vec<String>,
    /// Each step's trust digest, by step id.
    pub trust: Vec<(String, Digest)>,
    /// `--lib`, `--stand-ins` and `--fail-on`, each as (flag, value) in the order given.
    pub flags: Vec<(String, String)>,
}

/// The build key (6.2).
pub fn build_key(versions: &Versions, parts: &BuildKeyParts) -> Digest {
    let mut ps: Vec<(String, String)> = Vec::new();
    for (i, c) in parts.changes.iter().enumerate() {
        ps.push((format!("change {i:08}"), c.clone()));
    }
    for (p, d) in &parts.files {
        ps.push((format!("file {p}"), d.hex()));
    }
    for (i, l) in parts.lookups.iter().enumerate() {
        ps.push((format!("lookup {i:08}"), l.clone()));
    }
    for (s, d) in &parts.trust {
        ps.push((format!("trust {s}"), d.hex()));
    }
    for (i, (f, v)) in parts.flags.iter().enumerate() {
        ps.push((format!("flag {i:08} {f}"), v.clone()));
    }
    digest_of_parts("via-build-key", versions, &ps)
}

/// Whether the null build applies (6.2): the build key equals `run.key` in the directory's
/// current manifest, its record is in the store, every step completed, and no `--against` is
/// given. Then no node is touched and the record is the build.
pub fn null_build(
    store: &Store,
    key: &Digest,
    manifest_run_key: Option<&Digest>,
    against: bool,
) -> Option<BuildRecord> {
    if against || manifest_run_key != Some(key) {
        return None;
    }
    let rec = BuildRecord::get(store, key)?;
    rec.complete.then_some(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BuildRecord {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId::parse("source:world.toml").unwrap(),
            RecordNode {
                key: Some(Digest([1; 32])),
                value: Some(Digest([2; 32])),
                direct: vec![("bytes".into(), Digest([3; 32]))],
            },
        );
        nodes.insert(
            NodeId::parse("merge").unwrap(),
            RecordNode {
                key: None,
                value: None,
                direct: vec![],
            },
        );
        BuildRecord {
            versions: Versions::test(),
            nodes,
            statements: [("field.height".to_string(), Digest([4; 32]))].into(),
            world_diags: b"[]".to_vec(),
            files: [("manifest.json".to_string(), Digest([5; 32]))].into(),
            complete: true,
        }
    }

    #[test]
    fn record_round_trips_and_null_build_applies() {
        let r = sample();
        assert_eq!(BuildRecord::decode(&r.encode()), Some(r.clone()));
        assert!(BuildRecord::decode(&r.encode()[1..]).is_none());
        let root = std::env::temp_dir().join(format!("via-graph-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let s = Store::open(&root);
        let k = Digest([9; 32]);
        r.put(&s, &k);
        assert_eq!(null_build(&s, &k, Some(&k), false), Some(r.clone()));
        assert!(null_build(&s, &k, Some(&k), true).is_none());
        assert!(null_build(&s, &k, None, false).is_none());
        let mut inc = r;
        inc.complete = false;
        let k2 = Digest([8; 32]);
        inc.put(&s, &k2);
        assert!(null_build(&s, &k2, Some(&k2), false).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_key_holds_its_parts() {
        let v = Versions::test();
        let a = BuildKeyParts {
            changes: vec!["downstream".into()],
            files: vec![("world.toml".into(), Digest([1; 32]))],
            ..Default::default()
        };
        let mut b = a.clone();
        b.changes.clear();
        assert_ne!(build_key(&v, &a), build_key(&v, &b));
        assert_eq!(build_key(&v, &a), build_key(&v, &a.clone()));
    }
}
