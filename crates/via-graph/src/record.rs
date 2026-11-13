//! The build key, the build record and the null build (`spec/graph.md` 6.2).

use std::collections::BTreeMap;

use crate::digest::{Dec, Digest, Enc};
use crate::engine::StepOutcome;
use crate::id::NodeId;
use crate::key::{digest_of_parts, Versions};
use crate::plan::Read;
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
    /// The nodes it reads (its input edges' targets, sorted, once each), so a later build can
    /// find a cause's chain in this graph when the dependency was only here (6.4).
    pub inputs: Vec<NodeId>,
    /// The statements it is made from.
    pub made_from: Vec<String>,
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
    /// Each file of the manifest's `files` (the data files) by path. Only files whose bytes are a
    /// function of the build key are here: the manifest and the reports also depend on where the
    /// directory is and on its baseline, so they are checked against the directory's own
    /// manifest, never against this record (6.2).
    pub files: BTreeMap<String, Digest>,
    /// Whether every step completed.
    pub complete: bool,
    /// Each step node's facts for the run report (6.3), so a null build lists its step objects:
    /// its command, static key, trace, reads, what it returned and its log. The exit status is
    /// not kept: a null build runs nothing, so every `exit` is `null`.
    pub steps: BTreeMap<NodeId, StepOutcome>,
}

fn put_read(e: &mut Enc, r: &Read) {
    e.str(&r.method).digest(&r.request).digest(&r.answer);
}

fn get_read(d: &mut Dec) -> Option<Read> {
    Some(Read {
        method: d.str()?,
        request: d.digest()?,
        answer: d.digest()?,
    })
}

impl BuildRecord {
    /// The record's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Enc::new();
        e.str("via-build-record-3");
        self.versions.encode(&mut e);
        e.len(self.nodes.len());
        // Inputs by their place in the node list, which is in id order; an input that is not a
        // node of the record cannot be on a chain and is left out.
        let place: BTreeMap<&NodeId, usize> = self
            .nodes
            .keys()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        for (id, n) in &self.nodes {
            e.str(id.as_str());
            opt(&mut e, n.key.as_ref());
            opt(&mut e, n.value.as_ref());
            e.len(n.direct.len());
            for (l, d) in &n.direct {
                e.str(l).digest(d);
            }
            let ins: Vec<usize> = n
                .inputs
                .iter()
                .filter_map(|t| place.get(t).copied())
                .collect();
            e.len(ins.len());
            for i in ins {
                e.u64(u64::try_from(i).unwrap_or(u64::MAX));
            }
            e.len(n.made_from.len());
            for s in &n.made_from {
                e.str(s);
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
        e.len(self.steps.len());
        for (id, s) in &self.steps {
            e.str(id.as_str()).len(s.command.len());
            for c in &s.command {
                e.str(c);
            }
            opt(&mut e, s.static_key.as_ref());
            opt(&mut e, s.trace.as_ref());
            e.len(s.reads.len());
            for r in &s.reads {
                put_read(&mut e, r);
            }
            match &s.returned {
                None => {
                    e.byte(0);
                }
                Some((v, files)) => {
                    e.byte(1).digest(v).len(files.len());
                    for (p, d) in files {
                        e.str(p).digest(d);
                    }
                }
            }
            opt(&mut e, s.log.as_ref());
        }
        e.finish()
    }

    /// Reads a record; `None` when the bytes are not one.
    pub fn decode(b: &[u8]) -> Option<BuildRecord> {
        let mut d = Dec::new(b);
        if d.str()? != "via-build-record-3" {
            return None;
        }
        let versions = Versions {
            document: d.str()?,
            protocol: d.str()?,
            build: d.str()?,
            epoch: d.u64()?,
        };
        // Inputs name nodes by place, so they are resolved once every id is read.
        let mut list: Vec<(NodeId, RecordNode, Vec<usize>)> = Vec::new();
        for _ in 0..d.count()? {
            let id = NodeId::parse(&d.str()?).ok()?;
            let key = read_opt(&mut d)?;
            let value = read_opt(&mut d)?;
            let mut direct = Vec::new();
            for _ in 0..d.count()? {
                direct.push((d.str()?, d.digest()?));
            }
            let mut ins = Vec::new();
            for _ in 0..d.count()? {
                ins.push(usize::try_from(d.u64()?).ok()?);
            }
            let mut made_from = Vec::new();
            for _ in 0..d.count()? {
                made_from.push(d.str()?);
            }
            let node = RecordNode {
                key,
                value,
                direct,
                inputs: Vec::new(),
                made_from,
            };
            list.push((id, node, ins));
        }
        let ids: Vec<NodeId> = list.iter().map(|n| n.0.clone()).collect();
        let mut nodes = BTreeMap::new();
        for (id, mut node, ins) in list {
            for i in ins {
                node.inputs.push(ids.get(i)?.clone());
            }
            nodes.insert(id, node);
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
        let mut steps = BTreeMap::new();
        for _ in 0..d.count()? {
            let id = NodeId::parse(&d.str()?).ok()?;
            let mut command = Vec::new();
            for _ in 0..d.count()? {
                command.push(d.str()?);
            }
            let static_key = read_opt(&mut d)?;
            let trace = read_opt(&mut d)?;
            let mut reads = Vec::new();
            for _ in 0..d.count()? {
                reads.push(get_read(&mut d)?);
            }
            let returned = match d.byte()? {
                0 => None,
                1 => {
                    let v = d.digest()?;
                    let mut files = Vec::new();
                    for _ in 0..d.count()? {
                        files.push((d.str()?, d.digest()?));
                    }
                    Some((v, files))
                }
                _ => return None,
            };
            let log = read_opt(&mut d)?;
            steps.insert(
                id,
                StepOutcome {
                    command,
                    static_key,
                    trace,
                    reads,
                    exit: None,
                    returned,
                    log,
                },
            );
        }
        d.done().then_some(BuildRecord {
            versions,
            nodes,
            statements,
            world_diags,
            files,
            complete,
            steps,
        })
    }

    /// Keeps the record in the store under its build key.
    pub fn put(&self, store: &Store, build_key: &Digest) {
        store.put_build(build_key, &self.encode());
    }

    /// Each node's input edges, as [`crate::causes_from`] takes them (labels are not kept).
    pub fn edges(&self) -> BTreeMap<NodeId, Vec<(String, NodeId)>> {
        self.nodes
            .iter()
            .map(|(id, n)| {
                (
                    id.clone(),
                    n.inputs
                        .iter()
                        .map(|t| (String::new(), t.clone()))
                        .collect(),
                )
            })
            .collect()
    }

    /// The statements each node is made from, as [`crate::causes_from`] takes them.
    pub fn made_from(&self) -> BTreeMap<NodeId, Vec<String>> {
        self.nodes
            .iter()
            .map(|(id, n)| (id.clone(), n.made_from.clone()))
            .collect()
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
                inputs: vec![],
                made_from: vec!["world".into()],
            },
        );
        nodes.insert(
            NodeId::parse("merge").unwrap(),
            RecordNode {
                key: None,
                value: None,
                direct: vec![],
                inputs: vec![NodeId::parse("source:world.toml").unwrap()],
                made_from: vec![],
            },
        );
        BuildRecord {
            versions: Versions::test(),
            nodes,
            statements: [("field.height".to_string(), Digest([4; 32]))].into(),
            world_diags: b"[]".to_vec(),
            files: [("fields/height.npy".to_string(), Digest([5; 32]))].into(),
            complete: true,
            steps: [(
                NodeId::parse("step:checks").unwrap(),
                StepOutcome {
                    command: vec!["python".into(), "check.py".into()],
                    static_key: Some(Digest([6; 32])),
                    trace: None,
                    reads: vec![Read {
                        method: "data".into(),
                        request: Digest([7; 32]),
                        answer: Digest([8; 32]),
                    }],
                    exit: None,
                    returned: Some((
                        Digest([9; 32]),
                        vec![("diagnostics.json".into(), Digest([10; 32]))],
                    )),
                    log: Some(Digest([11; 32])),
                },
            )]
            .into(),
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
        // A later build with the same key replaces the record: complete now, the null build
        // applies again (6.2).
        let mut fixed = inc;
        fixed.complete = true;
        fixed.put(&s, &k2);
        assert_eq!(null_build(&s, &k2, Some(&k2), false), Some(fixed.clone()));
        // The graph's edges and makers come back by id.
        assert_eq!(
            fixed
                .edges()
                .get(&NodeId::parse("merge").unwrap())
                .map(Vec::len),
            Some(1)
        );
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
