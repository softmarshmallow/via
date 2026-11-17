//! Committed step results (`protocol.md` 7.1): `gen/<step id>/` beside the declaring document,
//! written by `via step commit` from a trusted build and read by a build that does not trust the
//! step. The folder holds the step's `document.toml`, the files it names, and `result.json`.
//!
//! `result.json` is canonical JSON: `protocol`, `static_key` (the digest of `key`, the parts of the
//! step's static key that do not depend on this machine's trust), `key`, `trust_digest`, `trace`
//! (each read as `{ "request", "answer" }`, the request as 4.4 normalises it), `facts` (the stated
//! facts it read, `derived.md` 5) and `files` (path in `out` to digest).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use via_diag::{codes, Diag};
use via_graph::{Def, Digest, Inputs, Store};

use crate::json::{obj, parse, J};
use crate::steps;

/// The name of the folder of committed results, beside the declaring document.
pub const GEN: &str = "gen";

/// The folder of a step's committed result: `gen/<id>/`, or `gen/<id>/<item>/` under `for_each`.
pub fn folder(home: &Path, step: &str, item: Option<&str>) -> PathBuf {
    let d = home.join(GEN).join(step);
    match item {
        Some(i) => d.join(i),
        None => d,
    }
}

/// The parts of a step's static key that a committed result is held to (`graph.md` 5.1): its
/// statements, its parameters, its direct digests and its inputs' values, less what says whether
/// this machine trusts it (`trusted`) and which committed folder it read (`gen`), which no trusted
/// build has.
pub fn key_parts(def: &Def, inputs: &Inputs<'_>) -> BTreeMap<String, String> {
    let mut parts = BTreeMap::new();
    for (id, d) in &def.statements {
        parts.insert(format!("statement {id}"), d.hex());
    }
    for (n, v) in &def.params {
        if n == "trusted" || n == "gen" {
            continue;
        }
        parts.insert(format!("param {n}"), v.clone());
    }
    for (l, d) in &def.direct {
        parts.insert(format!("direct {l}"), d.hex());
    }
    for (l, _, v) in inputs.all() {
        parts.insert(format!("edge {l}"), Digest::of(&v.bytes).hex());
    }
    parts
}

/// The static key of [`key_parts`].
pub fn static_key(parts: &BTreeMap<String, String>) -> Digest {
    let pairs: Vec<(String, String)> = parts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    via_graph::digest_of_parts("via-commit-1", &crate::build::versions(), &pairs)
}

/// A committed result as read: its parsed `result.json`.
pub struct Committed {
    pub result: J,
}

impl Committed {
    /// The committed result in `dir`, when it holds a `result.json` that reads as an object.
    pub fn read(dir: &Path) -> Option<Result<Committed, String>> {
        let p = dir.join("result.json");
        if !p.is_file() {
            return None;
        }
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => return Some(Err(format!("`result.json` cannot be read: {e}"))),
        };
        Some(match parse(&text) {
            Ok(j @ J::Obj(_)) => Ok(Committed { result: j }),
            _ => Err("`result.json` is not a JSON object".into()),
        })
    }

    fn str(&self, k: &str) -> Option<&str> {
        self.result.get(k).and_then(J::as_str)
    }

    /// The first part of the static key that differs from `now`, or the key itself.
    pub fn key_difference(
        &self,
        trust: &Digest,
        parts: &BTreeMap<String, String>,
    ) -> Option<String> {
        if self.str("protocol") != Some(via_proto::PROTOCOL) {
            return Some(format!(
                "its protocol `{}` is not `{}`",
                self.str("protocol").unwrap_or("none"),
                via_proto::PROTOCOL
            ));
        }
        if self.str("trust_digest") != Some(trust.hex().as_str()) {
            return Some("the step's code (its trust digest) differs".into());
        }
        let recorded: BTreeMap<String, String> = match self.result.get("key") {
            Some(J::Obj(m)) => m
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect(),
            _ => BTreeMap::new(),
        };
        // A configured program's digest stands as recorded (`protocol.md` 7.1, M7 close): an
        // untrusted step never runs it here, so this machine's copy, missing or rebuilt from the
        // same source into other bytes, is not what the result is a function of.
        let mut now = parts.clone();
        if let (Some(_), Some(r)) = (now.get("param program"), recorded.get("param program")) {
            now.insert("param program".into(), r.clone());
        }
        let names: std::collections::BTreeSet<&String> =
            recorded.keys().chain(now.keys()).collect();
        for n in names {
            if recorded.get(n) != now.get(n) {
                return Some(format!("the static key's part `{n}` differs"));
            }
        }
        if self.str("static_key") != Some(static_key(&now).hex().as_str()) {
            return Some("the static key differs".into());
        }
        None
    }

    /// Each traced read as (method, normalised request, recorded answer).
    pub fn trace(&self) -> Vec<(String, J, Option<Digest>)> {
        self.result
            .get("trace")
            .and_then(J::as_arr)
            .unwrap_or(&[])
            .iter()
            .map(|t| {
                let req = t.get("request").cloned().unwrap_or(J::Null);
                let method = req
                    .get("method")
                    .and_then(J::as_str)
                    .unwrap_or("")
                    .to_string();
                let answer = t
                    .get("answer")
                    .and_then(J::as_str)
                    .and_then(Digest::from_hex);
                (method, req, answer)
            })
            .collect()
    }

    /// The stated facts the step read.
    pub fn facts(&self) -> Vec<String> {
        self.result
            .get("facts")
            .and_then(J::as_arr)
            .unwrap_or(&[])
            .iter()
            .filter_map(|f| f.as_str().map(str::to_string))
            .collect()
    }

    /// The diagnostics the step raised itself, as the build holds them; `None` when `diags` is
    /// not an encoding of them. A result committed before they were recorded holds none.
    pub fn diags(&self) -> Option<Vec<Diag>> {
        match self.result.get("diags") {
            None => Some(Vec::new()),
            Some(v) => decode_diags(v.as_str()?),
        }
    }

    /// The step's explain texts, as (code, text).
    pub fn explains(&self) -> Vec<(String, Option<String>)> {
        self.result
            .get("explains")
            .and_then(J::as_arr)
            .unwrap_or(&[])
            .iter()
            .filter_map(|e| {
                let a = e.as_arr()?;
                let c = a.first()?.as_str()?.to_string();
                Some((c, a.get(1).and_then(J::as_str).map(str::to_string)))
            })
            .collect()
    }

    /// `files`, path to digest.
    pub fn files(&self) -> BTreeMap<String, Option<Digest>> {
        match self.result.get("files") {
            Some(J::Obj(m)) => m
                .iter()
                .map(|(p, d)| (p.clone(), d.as_str().and_then(Digest::from_hex)))
                .collect(),
            _ => BTreeMap::new(),
        }
    }
}

/// Diagnostics as hex of the store's encoding (`via_graph::CoreCodec`), which reads back equal;
/// `None` when they would not.
pub fn encode_diags(diags: &[Diag]) -> Option<String> {
    use via_graph::DiagCodec;
    let b = via_graph::CoreCodec.encode(diags)?;
    Some(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// [`encode_diags`] read back.
pub fn decode_diags(hex: &str) -> Option<Vec<Diag>> {
    use via_graph::DiagCodec;
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let b: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    via_graph::CoreCodec.decode(&b?)
}

/// `protocol.untrusted_stale`: the committed result no longer matches, naming the first
/// difference.
pub fn stale(step: &str, label: &str, why: &str, trust_command: &str) -> Diag {
    steps::step_diag(
        step,
        codes::protocol::UNTRUSTED_STALE,
        format!(
            "the committed result `{label}` of step `{step}` no longer matches this build: {why}"
        ),
        format!(
            "run {trust_command} and build, or commit a fresh result with `via step commit` on a machine that trusts the step"
        ),
    )
}

/// `protocol.untrusted_result`: the build used the committed result.
pub fn used(step: &str, label: &str) -> Diag {
    steps::step_diag(
        step,
        codes::protocol::UNTRUSTED_RESULT,
        format!(
            "step `{step}` is not trusted on this machine; its committed result `{label}` matched and was used"
        ),
        "none needed; trust the step to run it here".into(),
    )
}

/// A committed folder as a diagnostic names it, beside its declaring document.
pub fn label(step: &str, item: Option<&str>) -> String {
    match item {
        Some(i) => format!("{GEN}/{step}/{i}/"),
        None => format!("{GEN}/{step}/"),
    }
}

/// The files of one committed result, from a step node's value in the build record: the
/// document, the files it returned, and `result.json`. `None` when the value holds no document.
pub fn files_of(value: &J, store: &Store) -> Option<BTreeMap<String, Vec<u8>>> {
    let doc = Digest::from_hex(value.get("document")?.as_str()?)?;
    let text = via_graph::object_bytes(&store.get_object(&doc)?);
    let c = value.get("commit")?;
    let mut out: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    out.insert("document.toml".into(), text);
    let mut files = BTreeMap::new();
    for f in value.get("files").and_then(J::as_arr).unwrap_or(&[]) {
        let p = f.get("path")?.as_str()?.to_string();
        let d = Digest::from_hex(f.get("digest")?.as_str()?)?;
        if p == "document.toml" || p == "result.json" {
            continue;
        }
        out.insert(p.clone(), via_graph::object_bytes(&store.get_object(&d)?));
        files.insert(p, J::s(d.hex()));
    }
    let mut trace = Vec::new();
    let objects = via_proto::store::Store::open(store.root().join("objects")).ok()?;
    for t in c.get("trace").and_then(J::as_arr).unwrap_or(&[]) {
        let req = Digest::from_hex(t.get("request")?.as_str()?)?;
        let bytes = via_graph::object_bytes(&store.get_object(&req)?);
        let request = parse(std::str::from_utf8(&bytes).ok()?).ok()?;
        // Each blob the request names travels with it (`protocol.md` 7.1).
        let mut names = Vec::new();
        blobs_in(&request, &mut names);
        for n in names {
            let b = std::fs::read(objects.path_of(&n)).ok()?;
            out.insert(format!("{BLOBS}/{n}"), b);
        }
        trace.push(obj([
            ("answer", t.get("answer")?.clone()),
            ("request", request),
        ]));
    }
    let result = obj([
        (
            "diagnostics",
            c.get("diagnostics").cloned().unwrap_or(J::Arr(Vec::new())),
        ),
        (
            "diags",
            c.get("diags")
                .cloned()
                .unwrap_or_else(|| J::s(encode_diags(&[]).unwrap_or_default())),
        ),
        (
            "explains",
            c.get("explains").cloned().unwrap_or(J::Arr(Vec::new())),
        ),
        (
            "facts",
            value.get("facts").cloned().unwrap_or(J::Arr(Vec::new())),
        ),
        ("files", J::Obj(files.into_iter().collect())),
        ("key", c.get("key")?.clone()),
        ("protocol", J::s(via_proto::PROTOCOL)),
        ("static_key", c.get("static_key")?.clone()),
        ("trace", J::Arr(trace)),
        ("trust_digest", c.get("trust_digest")?.clone()),
    ]);
    out.insert("result.json".into(), result.file());
    Some(out)
}

/// The folder of a committed result that holds the blobs its trace names (`protocol.md` 7.1).
pub const BLOBS: &str = "result.blobs";

/// The blob names of every column in a request (4.2: a column with `dtype` and `blob`, and a
/// string column's `offsets`).
fn blobs_in(v: &J, out: &mut Vec<String>) {
    match v {
        J::Obj(m) => {
            if m.contains_key("dtype") {
                for k in ["blob", "offsets"] {
                    if let Some(n) = m.get(k).and_then(J::as_str) {
                        if via_proto::store::is_name(n) && !out.iter().any(|o| o == n) {
                            out.push(n.to_string());
                        }
                    }
                }
            }
            for x in m.values() {
                blobs_in(x, out);
            }
        }
        J::Arr(a) => a.iter().for_each(|x| blobs_in(x, out)),
        _ => {}
    }
}

/// Puts the blobs committed in `dir` into the objects store, each checked against its name. The
/// first that does not match, or cannot be put, as a reason for `protocol.untrusted_stale`.
pub fn restore_blobs(dir: &Path, objects: &via_proto::store::Store) -> Result<(), String> {
    let Ok(rd) = std::fs::read_dir(dir.join(BLOBS)) else {
        return Ok(());
    };
    let mut names: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for n in names {
        let b = std::fs::read(dir.join(BLOBS).join(&n))
            .map_err(|_| format!("`{BLOBS}/{n}` cannot be read"))?;
        if via_proto::store::digest(&b) != n {
            return Err(format!("`{BLOBS}/{n}` does not match its name"));
        }
        objects
            .put(&b)
            .map_err(|_| format!("`{BLOBS}/{n}` cannot be put in the store"))?;
    }
    Ok(())
}

/// Replaces `dir` whole with `files`, by a temporary folder renamed over it. True when the
/// folder's bytes changed.
pub fn replace(dir: &Path, files: &BTreeMap<String, Vec<u8>>) -> std::io::Result<bool> {
    let before = steps::input_digest(dir);
    let parent = dir.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let name = dir
        .file_name()
        .map_or_else(|| "step".into(), |n| n.to_string_lossy().into_owned());
    let tmp = parent.join(format!(".{name}.new-{}", std::process::id()));
    let old = parent.join(format!(".{name}.old-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    for (p, b) in files {
        let t = tmp.join(p);
        if let Some(d) = t.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&t, b)?;
    }
    let after = steps::input_digest(&tmp);
    if dir.exists() {
        std::fs::rename(dir, &old)?;
    }
    if let Err(e) = std::fs::rename(&tmp, dir) {
        let _ = std::fs::rename(&old, dir);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(before != after)
}

/// `protocol.commit_refused`, at the world file's argument.
pub fn refused(at: u32) -> Diag {
    Diag::new(
        codes::protocol::COMMIT_REFUSED,
        via_diag::Subject::Cli,
        via_diag::Site::Argument(at),
        crate::txt("the build has errors, so `via step commit` wrote nothing".into()),
        crate::txt("fix the build's errors, then commit again".into()),
    )
}

/// `via step commit WORLD [--change C]... [--step ID]...` (`protocol.md` 7.1): builds as `via
/// build` does, then writes the result of every generate step, or of each `--step`, to its `gen/`
/// folder. A build with any error writes nothing (`protocol.commit_refused`). Prints the folders
/// whose bytes changed.
pub fn run(a: &crate::build::Args, only: &[(u32, String)]) -> crate::build::Outcome {
    let mut o = crate::build::run(a);
    if via_diag::exit_status(&o.records) >= 1 {
        o.records
            .push(refused(a.world_arg).emit(&via_diag::NoWorld));
        return o;
    }
    let mut faults: Vec<Diag> = Vec::new();
    let wdir = crate::build::world_dir(&a.world);
    let store_root = a
        .store
        .clone()
        .unwrap_or_else(|| wdir.join(via_graph::DEFAULT_STORE));
    let store = Store::open(&store_root);
    let record = o
        .out
        .as_ref()
        .and_then(|d| std::fs::read(d.join("manifest.json")).ok())
        .and_then(|b| crate::build::manifest_ids(&b))
        .and_then(|(_, _, k)| Digest::from_hex(&k))
        .and_then(|k| via_graph::BuildRecord::get(&store, &k));
    let opts = via_doc::LoadOptions::from_env(a.lib.clone(), a.changes.clone());
    let loaded = via_doc::load(&a.world, a.world_arg, &opts);
    let (Some(rec), Some(world)) = (record, loaded.world.as_ref()) else {
        o.records
            .push(refused(a.world_arg).emit(&via_diag::NoWorld));
        return o;
    };
    let decls: Vec<steps::StepDecl> = steps::declared(world, &wdir)
        .into_iter()
        .filter(|s| s.when == "generate")
        .collect();
    for (at, id) in only {
        if !decls.iter().any(|s| &s.id == id) {
            let near = via_diag::nearest::nearest(id, decls.iter().map(|s| s.id.as_str()));
            faults.push(crate::usage(
                *at,
                format!("`--step {id}` names no generate step of this world"),
                near.hint("name a step whose `when` is `generate`"),
            ));
        }
    }
    let mut changed: Vec<String> = Vec::new();
    if faults.is_empty() {
        for s in &decls {
            if !only.is_empty() && !only.iter().any(|(_, i)| i == &s.id) {
                continue;
            }
            let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
            let mut nodes: Vec<&via_graph::NodeId> = rec
                .nodes
                .keys()
                .filter(|n| n.as_str().starts_with("step:"))
                .filter(|n| crate::build::graph::split_step(n).0 == s.id)
                .filter(|n| s.for_each.is_none() || crate::build::graph::split_step(n).1.is_some())
                .collect();
            nodes.sort();
            for n in nodes {
                let item = crate::build::graph::split_step(n).1;
                let Some(value) = rec.nodes[n]
                    .value
                    .and_then(|d| store.get_object(&d))
                    .map(|b| via_graph::object_bytes(&b))
                    .and_then(|b| parse(std::str::from_utf8(&b).ok()?).ok())
                else {
                    continue;
                };
                let Some(fs) = files_of(&value, &store) else {
                    continue;
                };
                for (p, b) in fs {
                    let p = match &item {
                        Some(i) => format!("{i}/{p}"),
                        None => p,
                    };
                    files.insert(p, b);
                }
            }
            if files.is_empty() {
                continue;
            }
            let dir = folder(&s.home, &s.id, None);
            match replace(&dir, &files) {
                Ok(true) => changed.push(dir.display().to_string()),
                Ok(false) => {}
                Err(e) => faults.push(Diag::new(
                    codes::build::WRITE_FAILED,
                    via_diag::Subject::entity("step", s.id.clone()),
                    via_diag::Site::Header,
                    crate::txt(format!("cannot write `{}`: {e}", dir.display())),
                    crate::txt("free space, or fix the folder's permissions".into()),
                )),
            }
        }
    }
    for c in &changed {
        println!("{c}");
    }
    o.records
        .extend(faults.iter().map(|d| d.emit(&via_diag::NoWorld)));
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A committed result with `parts` as its key, for the step's trust digest `trust`.
    fn committed(trust: &Digest, parts: &BTreeMap<String, String>) -> Committed {
        let key = J::Obj(
            parts
                .iter()
                .map(|(k, v)| (k.clone(), J::s(v.clone())))
                .collect(),
        );
        Committed {
            result: obj([
                ("protocol", J::s(via_proto::PROTOCOL)),
                ("trust_digest", J::s(trust.hex())),
                ("static_key", J::s(static_key(parts).hex())),
                ("key", key),
            ]),
        }
    }

    /// `protocol.md` 7.1 (M7 close): a configured program's digest stands as recorded, whether
    /// this machine lacks the program or has a copy rebuilt into other bytes; every other part
    /// must match.
    #[test]
    fn program_digest_stands_as_recorded() {
        let trust = Digest::of(b"trust");
        let mut parts = BTreeMap::new();
        parts.insert("param program".to_string(), Digest::of(b"built here").hex());
        parts.insert("param seed".to_string(), "31".to_string());
        let c = committed(&trust, &parts);
        assert_eq!(c.key_difference(&trust, &parts), None);
        let mut rebuilt = parts.clone();
        rebuilt.insert("param program".into(), Digest::of(b"built there").hex());
        assert_eq!(c.key_difference(&trust, &rebuilt), None);
        let mut missing = parts.clone();
        missing.insert("param program".into(), "missing".into());
        assert_eq!(c.key_difference(&trust, &missing), None);
        let mut reseeded = rebuilt.clone();
        reseeded.insert("param seed".into(), "32".into());
        assert_eq!(
            c.key_difference(&trust, &reseeded).as_deref(),
            Some("the static key's part `param seed` differs")
        );
    }
}
