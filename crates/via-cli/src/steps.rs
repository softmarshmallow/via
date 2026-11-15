//! Steps (`protocol.md`): the steps a world declares, their code folders and trust digests (7),
//! the carried SDK (8.1), and running a lint step as a plain command (2.2, 6): the request with
//! its `data`, the process with its environment and deadline, and the diagnostics it returns (5).
//!
//! M1 runs lint steps only, as plain commands; `worker = true` is `doc.not_yet` and generate and
//! export steps are not run (`protocol.md`, What arrives when).

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use via_diag::step::{Explains, Mapped, RawValue, StepDiagIn, StepRelatedIn, StepWorld};
use via_diag::{codes, Diag, Site, Subject};
use via_doc::{Value, World};
use via_graph::{Digest, Enc, Store};
use via_model::Build;
use via_num::Grid;

use crate::json::{obj, J};
use crate::txt;

mod carried {
    include!(concat!(env!("OUT_DIR"), "/sdk_files.rs"));
}

/// One `[step.<id>]` of the merged world.
#[derive(Clone, Debug)]
pub struct StepDecl {
    pub id: String,
    pub when: String,
    pub command: Vec<String>,
    pub worker: bool,
    pub params: J,
    pub inputs: Vec<String>,
    /// `for_each` as written (`protocol.md` 1.2).
    pub for_each: Option<String>,
    /// `checks`: the generate steps a lint checks (`derived.md` 5).
    pub checks: Vec<String>,
    /// The layer the step belongs to (`protocol.md` 1, "Which layer").
    pub layer: String,
    /// The folder of the declaring document.
    pub home: PathBuf,
}

impl StepDecl {
    /// The kind a `for_each = "kind:<k>"` selects; `None` without `for_each` or in another form.
    pub fn for_each_kind(&self) -> Option<&str> {
        self.for_each
            .as_deref()?
            .strip_prefix("kind:")
            .filter(|k| !k.is_empty())
    }
}

/// `p` made absolute against the caller's folder, as given when that fails.
fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Where libraries are found (`documents.md` 4), in search order.
#[derive(Clone, Debug, Default)]
pub struct Roots {
    pub world_dir: PathBuf,
    pub lib: Vec<PathBuf>,
    pub via_lib: Vec<PathBuf>,
    pub shipped: Option<PathBuf>,
    /// The libraries the world lists in `include`: the names a `lib:` element may give, and
    /// those `protocol.unknown_library` searches for the nearest.
    pub include: Vec<String>,
}

/// The library names a world lists in `include`.
pub fn includes(w: &World) -> Vec<String> {
    match w.root().value("include") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|i| match &i.value {
                Value::Str(s) => Some(s.text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `protocol.unknown_library`: its hint and `data.nearest` name the nearest library `include`
/// lists (`protocol.md` 2).
fn unknown_library(step: &str, message: String, lib: &str, roots: &Roots) -> Diag {
    let near = via_diag::nearest::nearest(lib, roots.include.iter().map(String::as_str));
    let d = step_diag(
        step,
        codes::protocol::UNKNOWN_LIBRARY,
        message,
        near.hint("name a library the world lists in `include`"),
    );
    match near.found() {
        Some(n) => d.with("nearest", n),
        None => d,
    }
}

impl Roots {
    /// The folder of library `name`: the first root holding it, as an absolute path. A step runs
    /// in its code folder, so a root given relative to the caller's folder (`--lib lib`) would
    /// otherwise be read again from inside that folder and name nothing.
    pub fn library(&self, name: &str) -> Option<PathBuf> {
        std::iter::once(self.world_dir.join("lib"))
            .chain(self.lib.iter().cloned())
            .chain(self.via_lib.iter().cloned())
            .chain(self.shipped.iter().cloned())
            .map(|r| r.join(name))
            .find(|p| p.is_dir())
            .map(|p| absolute(&p))
    }
}

fn value_json(v: &Value) -> J {
    match v {
        Value::Table(t) => J::Obj(
            t.iter()
                .map(|(k, e)| (k.clone(), value_json(&e.item.value)))
                .collect(),
        ),
        Value::List(xs) => J::Arr(xs.iter().map(|i| value_json(&i.value)).collect()),
        Value::Str(s) => J::s(s.text.clone()),
        Value::Num(n) => J::Num(n.norm.clone()),
        Value::Bool(b) => J::Bool(*b),
        Value::DateTime => J::Null,
    }
}

fn strings(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::List(xs)) => xs
            .iter()
            .filter_map(|i| match &i.value {
                Value::Str(s) => Some(s.text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Every step of the merged world, in id order.
pub fn declared(world: &World, world_dir: &Path) -> Vec<StepDecl> {
    let mut out = Vec::new();
    for id in world.ids("step") {
        let Some(t) = world.entity("step", id) else {
            continue;
        };
        let get = |k: &str| t.get(k).map(|e| &e.item.value);
        let when = match get("when") {
            Some(Value::Str(s)) => s.text.clone(),
            _ => String::new(),
        };
        // The declaring document: a layer's file, or the world's.
        let home = world
            .entity_entry("step", id)
            .and_then(|e| world.sources().get(e.key.src))
            .map(|s| s.name.clone())
            .filter(|n| !n.contains(':'))
            .map_or_else(
                || world_dir.to_path_buf(),
                |n| {
                    world_dir
                        .join(&n)
                        .parent()
                        .map_or_else(|| world_dir.to_path_buf(), Path::to_path_buf)
                },
            );
        out.push(StepDecl {
            id: id.to_string(),
            when,
            command: strings(get("command")),
            worker: matches!(get("worker"), Some(Value::Bool(true))),
            params: get("params").map_or_else(|| J::Obj(J::obj()), value_json),
            inputs: strings(get("inputs")),
            for_each: match get("for_each") {
                Some(Value::Str(s)) => Some(s.text.clone()),
                _ => None,
            },
            checks: strings(get("checks")),
            layer: world.home("step", id).unwrap_or("world").to_string(),
            home,
        });
    }
    out
}

/// A step's code (7): its code folder, if any, and its trust digest.
#[derive(Clone, Debug)]
pub struct Code {
    pub folder: Option<PathBuf>,
    pub files: usize,
    pub trust: Digest,
}

/// A declared input's digest (`graph.md` 5.1): a file's bytes; a folder as the sorted list,
/// bytewise, of every file under it as (path relative to the folder with `/`, file digest), with
/// no name skipped and no line ending rewritten, and a symbolic link entered as its target's
/// text, not followed. `None` when the path does not exist; a file that exists but cannot be
/// read digests as unreadable, never as missing.
pub fn input_digest(p: &Path) -> Option<Digest> {
    fn file(p: &Path) -> Digest {
        std::fs::read(p).map_or_else(|_| Digest::of(b"\0unreadable"), |b| Digest::of(&b))
    }
    fn link(p: &Path) -> Digest {
        let t = std::fs::read_link(p).map_or_else(
            |_| String::from("\0unreadable"),
            |t| t.to_string_lossy().replace('\\', "/"),
        );
        Digest::of(format!("via-link-1\0{t}").as_bytes())
    }
    fn walk(base: &Path, d: &Path, out: &mut Vec<(String, Digest)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            let Ok(rel) = p.strip_prefix(base) else {
                continue;
            };
            let r: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            if ft.is_symlink() {
                out.push((r.join("/"), link(&p)));
            } else if ft.is_dir() {
                walk(base, &p, out);
            } else {
                out.push((r.join("/"), file(&p)));
            }
        }
    }
    let meta = std::fs::symlink_metadata(p).ok()?;
    if meta.file_type().is_symlink() {
        // Followed at the top only when it names a folder; a dangling link is missing.
        std::fs::metadata(p).ok()?;
    }
    if !p.is_dir() {
        return Some(file(p));
    }
    let mut files = Vec::new();
    walk(p, p, &mut files);
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut e = Enc::new();
    e.str("via-input-folder-1").len(files.len());
    for (rel, d) in &files {
        e.str(rel).digest(d);
    }
    Some(Digest::of(&e.finish()))
}

/// The digest of a folder by 7's rule, and its file count.
pub fn folder_digest(dir: &Path) -> (Digest, usize) {
    fn walk(base: &Path, d: &Path, out: &mut Vec<(String, PathBuf)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "__pycache__" {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let p = e.path();
            if ft.is_dir() {
                walk(base, &p, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                let r: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push((r.join("/"), p));
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut e = Enc::new();
    e.str("via-code-folder-1").len(files.len());
    for (rel, p) in &files {
        let mut bytes = std::fs::read(p).unwrap_or_default();
        if !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok() {
            let t = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
            bytes = t.into_bytes();
        }
        e.str(rel).digest(&Digest::of(&bytes));
    }
    (Digest::of(&e.finish()), files.len())
}

pub(crate) fn step_diag(step: &str, code: via_diag::Code, message: String, hint: String) -> Diag {
    Diag::new(
        code,
        Subject::entity("step", step),
        Site::Header,
        txt(message),
        txt(hint),
    )
}

/// The code of a step: its code folder (7) and trust digest. `sdk` is the `--sdk` folder's digest.
pub fn code_of(
    s: &StepDecl,
    roots: &Roots,
    world_dir: &Path,
    sdk: Option<&Digest>,
) -> Result<Code, Diag> {
    let mut folder: Option<PathBuf> = None;
    for el in &s.command {
        if let Some(rest) = el.strip_prefix("lib:") {
            let lib = rest.split('/').next().unwrap_or("");
            folder = roots.library(lib);
            if folder.is_none() {
                return Err(unknown_library(
                    &s.id,
                    format!(
                        "step `{}` names `{el}`, and no library root holds `{lib}`",
                        s.id
                    ),
                    lib,
                    roots,
                ));
            }
            break;
        }
        if Path::new(el).is_absolute() {
            return Err(step_diag(
                &s.id,
                codes::protocol::ABSOLUTE_PATH,
                format!("step `{}`: `{el}` is an absolute path", s.id),
                "write the path relative to the declaring document".into(),
            ));
        }
        let p = s.home.join(el);
        if p.is_file() {
            folder = p.parent().map(Path::to_path_buf);
            break;
        }
    }
    let mut files = 0;
    let mut e = Enc::new();
    e.str("via-trust-1").len(s.command.len());
    for c in &s.command {
        e.str(c);
    }
    if let Some(f) = &folder {
        let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        if canon(world_dir).starts_with(canon(f)) {
            return Err(step_diag(
                &s.id,
                codes::protocol::CODE_FOLDER,
                format!(
                    "step `{}`: its code folder is, or holds, the world's folder, so every edit to the world would need trusting again",
                    s.id
                ),
                "move the script into a folder of its own".into(),
            ));
        }
        let (d, n) = folder_digest(f);
        files = n;
        e.byte(1).digest(&d);
    } else {
        e.byte(0);
    }
    match sdk {
        Some(d) => {
            e.byte(1).digest(d);
        }
        None => {
            e.byte(0);
        }
    }
    Ok(Code {
        folder,
        files,
        trust: Digest::of(&e.finish()),
    })
}

/// The trust digests recorded in `trust.toml`.
pub fn trusted(config: &Path) -> std::collections::BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(config.join("trust.toml")) else {
        return Default::default();
    };
    let Ok(t) = text.parse::<toml::Table>() else {
        return Default::default();
    };
    t.get("trusted")
        .and_then(|v| v.as_table())
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

/// A counter for the SDK's temporary folders: with the process id, unique per call, so two lint
/// steps run by two graph threads never write the same folder.
static SDK_TMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The carried SDK written once to `<store>/sdk/<digest>/`, or `--sdk`'s folder; the folder to put
/// on `PYTHONPATH`. On failure, the path that could not be written and why.
pub fn sdk_folder(store: &Store, sdk: Option<&Path>) -> Result<PathBuf, (PathBuf, std::io::Error)> {
    // Absolute: the step runs in its own working folder, where a relative path means nothing.
    let absolute = |p: PathBuf| std::path::absolute(&p).unwrap_or(p);
    if let Some(s) = sdk {
        return Ok(absolute(s.to_path_buf()));
    }
    let mut e = Enc::new();
    e.str("via-sdk-1").len(carried::FILES.len());
    for (p, b) in carried::FILES {
        e.str(p).digest(&Digest::of(b));
    }
    let d = Digest::of(&e.finish());
    let dir = store.root().join("sdk").join(d.hex());
    if !dir.join(".complete").exists() {
        let n = SDK_TMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp =
            store
                .root()
                .join("sdk")
                .join(format!("{}.tmp{}-{n}", d.hex(), std::process::id()));
        let write = || -> Result<(), (PathBuf, std::io::Error)> {
            for (p, b) in carried::FILES {
                let t = tmp.join(p);
                if let Some(parent) = t.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| (parent.to_path_buf(), e))?;
                }
                std::fs::write(&t, b).map_err(|e| (t.clone(), e))?;
            }
            let c = tmp.join(".complete");
            std::fs::write(&c, b"").map_err(|e| (c.clone(), e))
        };
        if let Err(f) = write() {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(f);
        }
        // A folder already in place (another call or build won the race) is the same bytes.
        if let Err(e) = std::fs::rename(&tmp, &dir) {
            let _ = std::fs::remove_dir_all(&tmp);
            if !dir.join(".complete").exists() {
                return Err((dir, e));
            }
        }
    }
    Ok(absolute(dir))
}

pub(crate) fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// A column of 4.3: inline up to 4,096 values, else a blob in the store.
fn column_num(
    store: &Store,
    dtype: &str,
    shape: Option<Vec<usize>>,
    vals: Vec<J>,
    raw: Vec<u8>,
) -> J {
    let n = vals.len();
    let mut m = J::obj();
    m.insert("dtype".into(), J::s(dtype));
    if let Some(s) = shape {
        m.insert(
            "shape".into(),
            J::Arr(s.into_iter().map(J::count).collect()),
        );
    }
    if n > 4096 {
        let d = store.put_object(&raw);
        m.insert("blob".into(), J::s(d.hex()));
        m.insert("path".into(), J::s(slash(&store.object_path(&d))));
    } else {
        m.insert("values".into(), J::Arr(vals));
    }
    J::Obj(m)
}

fn col_i64(store: &Store, v: &[i64]) -> J {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    column_num(
        store,
        "i64",
        None,
        v.iter().map(|x| J::int(*x)).collect(),
        raw,
    )
}

fn col_str(store: &Store, v: &[String]) -> J {
    if v.len() <= 4096 {
        return obj([
            ("dtype", J::s("str")),
            ("values", J::strs(v.iter().map(String::as_str))),
        ]);
    }
    let mut bytes = Vec::new();
    let mut offs: Vec<i64> = vec![0];
    for s in v {
        bytes.extend_from_slice(s.as_bytes());
        offs.push(i64::try_from(bytes.len()).unwrap_or(i64::MAX));
    }
    let b = store.put_object(&bytes);
    let o = store.put_object(
        &offs
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<u8>>(),
    );
    obj([
        ("blob", J::s(b.hex())),
        ("dtype", J::s("str")),
        ("offsets", J::s(o.hex())),
    ])
}

fn col_u8(store: &Store, v: &[u8]) -> J {
    column_num(
        store,
        "u8",
        None,
        v.iter().map(|x| J::int(i64::from(*x))).collect(),
        v.to_vec(),
    )
}

/// An attribute two kinds give two types (4.3): the attribute, then (kind, type) for each.
pub(crate) struct Mixed {
    attr: String,
    a: (String, String),
    b: (String, String),
}

/// `data.features` (2.2, 4.3): every feature and footprint feature as one feature list. An
/// attribute that two kinds give two types cannot be one column: `Err`, `protocol.column_mixed`.
fn features_data(store: &Store, b: &Build, world: &World, rank: Option<i64>) -> Result<J, Mixed> {
    let sees = |r: Option<i64>| match (rank, r) {
        (Some(own), Some(r)) => r <= own,
        _ => true,
    };
    struct Row {
        name: String,
        kind: String,
        authority: String,
        layer: String,
        geometry: u8,
        ring: Vec<(i64, i64)>,
        attrs: BTreeMap<String, String>,
        types: BTreeMap<String, String>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (n, f) in &b.features {
        if !sees(crate::stepview::view_rank(world, "feature", n)) {
            continue;
        }
        let mut attrs = f.attrs.clone();
        let mut types = f.attr_types.clone();
        for (a, (place, _)) in &f.names {
            attrs.insert(a.clone(), place.clone());
            types.insert(a.clone(), "text".into());
        }
        rows.push(Row {
            types,
            name: n.clone(),
            kind: f.kind.clone().unwrap_or_default(),
            authority: f
                .tier
                .map(|t| match t {
                    via_model::Tier::Derived => "derived",
                    via_model::Tier::Shaped => "shaped",
                    via_model::Tier::Anchored => "anchored",
                })
                .unwrap_or("")
                .into(),
            layer: world.home("feature", n).unwrap_or("").into(),
            geometry: match f.shape.as_str() {
                "point" => 0,
                "path" => 1,
                _ => 2,
            },
            ring: if f.shape == "region" {
                crate::out::canonical_ring(&f.points, true)
            } else {
                f.points.clone()
            },
            attrs,
        });
    }
    for (inst, f) in &b.footprints {
        if !sees(world.rank("instance", inst)) {
            continue;
        }
        rows.push(Row {
            name: format!("{inst}.ground"),
            kind: f.kind.clone(),
            authority: String::new(),
            layer: world.home("instance", inst).unwrap_or("").into(),
            geometry: 2,
            ring: crate::out::canonical_ring(&f.ring, true),
            attrs: BTreeMap::new(),
            types: BTreeMap::new(),
        });
    }
    rows.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let mut coords: Vec<J> = Vec::new();
    let mut raw: Vec<u8> = Vec::new();
    let (mut pieces, mut rings, mut starts) = (vec![0i64], vec![0i64], vec![0i64]);
    for (i, r) in rows.iter().enumerate() {
        for (x, y) in &r.ring {
            coords.push(J::Arr(vec![J::int(*x), J::int(*y)]));
            raw.extend_from_slice(&i32::try_from(*x).unwrap_or(0).to_le_bytes());
            raw.extend_from_slice(&i32::try_from(*y).unwrap_or(0).to_le_bytes());
        }
        starts.push(starts[i] + i64::try_from(r.ring.len()).unwrap_or(0));
        rings.push(i64::try_from(i + 1).unwrap_or(0));
        pieces.push(i64::try_from(i + 1).unwrap_or(0));
    }
    let v = coords.len();
    let mut m = J::obj();
    m.insert(
        "coords".into(),
        column_num(store, "i32", Some(vec![v, 2]), coords, raw),
    );
    m.insert(
        "geometry".into(),
        col_u8(store, &rows.iter().map(|r| r.geometry).collect::<Vec<_>>()),
    );
    m.insert("pieces".into(), col_i64(store, &pieces));
    m.insert("rings".into(), col_i64(store, &rings));
    m.insert("ring_starts".into(), col_i64(store, &starts));
    for (k, get) in [
        ("name", (|r: &Row| r.name.clone()) as fn(&Row) -> String),
        ("kind", |r: &Row| r.kind.clone()),
        ("authority", |r: &Row| r.authority.clone()),
        ("layer", |r: &Row| r.layer.clone()),
    ] {
        m.insert(
            k.into(),
            col_str(store, &rows.iter().map(get).collect::<Vec<_>>()),
        );
    }
    // Attribute columns: f64 where every present value is a number, bool where every one is a
    // bool, else str; with the `<attribute>?` presence column beside each.
    let names: std::collections::BTreeSet<&String> =
        rows.iter().flat_map(|r| r.attrs.keys()).collect();
    for a in names {
        let present: Vec<u8> = rows
            .iter()
            .map(|r| u8::from(r.attrs.contains_key(a)))
            .collect();
        let vals: Vec<Option<&String>> = rows.iter().map(|r| r.attrs.get(a)).collect();
        // Each row's type: as the model typed it, else read from the text (a snapshot's).
        let type_of = |r: &Row, v: &str| -> String {
            r.types.get(a).cloned().unwrap_or_else(|| {
                if v == "true" || v == "false" {
                    "bool".into()
                } else if v.parse::<f64>().is_ok_and(f64::is_finite) {
                    "number".into()
                } else {
                    "text".into()
                }
            })
        };
        let mut seen: Vec<(String, String)> = Vec::new();
        for r in &rows {
            if let Some(v) = r.attrs.get(a) {
                let t = type_of(r, v);
                if let Some(first) = seen.first() {
                    if first.1 != t {
                        return Err(Mixed {
                            attr: a.clone(),
                            a: first.clone(),
                            b: (r.kind.clone(), t),
                        });
                    }
                } else {
                    seen.push((r.kind.clone(), t));
                }
            }
        }
        let ty = seen.first().map_or("text", |s| s.1.as_str());
        let col = if ty == "bool" {
            let b: Vec<u8> = vals
                .iter()
                .map(|v| u8::from(v.is_some_and(|v| v == "true")))
                .collect();
            column_num(
                store,
                "bool",
                None,
                b.iter().map(|x| J::Bool(*x == 1)).collect(),
                b,
            )
        } else if ty == "number" {
            let f: Vec<f64> = vals
                .iter()
                .map(|v| v.and_then(|v| v.parse().ok()).unwrap_or(0.0))
                .collect();
            let raw: Vec<u8> = f.iter().flat_map(|x| x.to_le_bytes()).collect();
            column_num(
                store,
                "f64",
                None,
                f.iter().map(|x| J::dbl(*x).unwrap_or(J::int(0))).collect(),
                raw,
            )
        } else {
            col_str(
                store,
                &vals
                    .iter()
                    .map(|v| v.cloned().unwrap_or_default())
                    .collect::<Vec<_>>(),
            )
        };
        m.insert(a.clone(), col);
        m.insert(format!("{a}?"), col_u8(store, &present));
    }
    Ok(J::Obj(m))
}

/// `data.instances` (2.2): one table, rows by name.
fn instances_data(
    store: &Store,
    b: &Build,
    pack: Option<&via_model::pack::Pack>,
    world: &World,
    rank: Option<i64>,
) -> J {
    let rows: Vec<(&String, &via_model::InstanceOut)> = b
        .instances
        .iter()
        .filter(|(n, i)| {
            let r = match &i.maker {
                Some(p) => crate::stepview::maker_rank(world, p),
                None => world.rank("instance", n),
            };
            match (rank, r) {
                (Some(own), Some(r)) => r <= own,
                _ => true,
            }
        })
        .collect();
    let i32col = |f: &dyn Fn(&via_model::InstanceOut) -> i64| {
        let v: Vec<i32> = rows
            .iter()
            .map(|(_, i)| i32::try_from(f(i)).unwrap_or(0))
            .collect();
        let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        column_num(
            store,
            "i32",
            None,
            v.iter().map(|x| J::int(i64::from(*x))).collect(),
            raw,
        )
    };
    // z, scale and tint are f64 (protocol.md 3.3: heights are not on the lattice).
    let f64col = |f: &dyn Fn(&via_model::InstanceOut) -> f64| {
        let v: Vec<f64> = rows.iter().map(|(_, i)| f(i) + 0.0).collect();
        let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        column_num(
            store,
            "f64",
            None,
            v.iter().map(|x| J::dbl(*x).unwrap_or(J::int(0))).collect(),
            raw,
        )
    };
    let mut m = J::obj();
    m.insert(
        "name".into(),
        col_str(
            store,
            &rows.iter().map(|(n, _)| (*n).clone()).collect::<Vec<_>>(),
        ),
    );
    m.insert(
        "asset".into(),
        col_str(
            store,
            &rows
                .iter()
                .map(|(_, i)| i.asset.clone())
                .collect::<Vec<_>>(),
        ),
    );
    // `by`: the producer; a written instance is its own statement, `pin.<n>` or
    // `instance.<n>` (protocol.md 2.2), so a diagnostic about it has a subject.
    m.insert(
        "by".into(),
        col_str(
            store,
            &rows
                .iter()
                .map(|(n, i)| match &i.maker {
                    Some(p) => p.clone(),
                    None if b.statements.contains_key(&format!("pin.{n}")) => format!("pin.{n}"),
                    None => format!("instance.{n}"),
                })
                .collect::<Vec<_>>(),
        ),
    );
    // `taxon`: the asset's taxon path, "" when it has none, so `@<taxon>` selects without the pack.
    m.insert(
        "taxon".into(),
        col_str(
            store,
            &rows
                .iter()
                .map(|(_, i)| {
                    pack.and_then(|p| p.assets.get(&i.asset))
                        .and_then(|a| a.taxon.clone())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>(),
        ),
    );
    m.insert("x".into(), i32col(&|i| i.origin.0));
    m.insert("y".into(), i32col(&|i| i.origin.1));
    m.insert("z".into(), f64col(&|i| i.z));
    m.insert("face".into(), i32col(&|i| i.facing.rem_euclid(360_000)));
    m.insert("scale".into(), f64col(&|i| i.scale));
    m.insert("tint".into(), f64col(&|i| i.tint));
    J::Obj(m)
}

/// `data.fields` (2.2): every field as a field file in the store.
fn fields_data(store: &Store, b: &Build, grid: &Grid, world: &World, rank: Option<i64>) -> J {
    let (nx, ny) = (
        usize::try_from(grid.nx).unwrap_or(0),
        usize::try_from(grid.ny).unwrap_or(0),
    );
    let mut m = J::obj();
    for (name, f) in &b.fields {
        if let (Some(own), Some(r)) = (rank, crate::stepview::view_rank(world, "field", name)) {
            if r > own {
                continue;
            }
        }
        let (bytes, ty, mats) = match &f.values {
            Some(v) => (crate::npy::f4(&[ny, nx], v), "scalar", None),
            None => {
                let mut all = Vec::new();
                for v in f.weights.values() {
                    all.extend_from_slice(v);
                }
                (
                    crate::npy::f4(&[f.weights.len(), ny, nx], &all),
                    "weights",
                    Some(J::strs(f.weights.keys().map(String::as_str))),
                )
            }
        };
        let d = store.put_object(&bytes);
        let mut e = J::obj();
        e.insert("type".into(), J::s(ty));
        e.insert("role".into(), J::opt(f.role.as_deref()));
        let mut tags = f.tags.clone();
        tags.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        e.insert("tags".into(), J::strs(tags.iter().map(String::as_str)));
        e.insert("unit".into(), J::opt(f.unit.as_deref()));
        e.insert("blob".into(), J::s(d.hex()));
        e.insert("path".into(), J::s(slash(&store.object_path(&d))));
        e.insert("i0".into(), J::int(0));
        e.insert("j0".into(), J::int(0));
        e.insert("i1".into(), J::count(nx));
        e.insert("j1".into(), J::count(ny));
        if let Some(ms) = mats {
            e.insert("materials".into(), ms);
        }
        m.insert(name.clone(), J::Obj(e));
    }
    J::Obj(m)
}

/// What a lint run gives: the step's diagnostics (raised, to be emitted against the world), its
/// explain texts, and the record the run report holds.
pub struct Ran {
    pub diags: Vec<Diag>,
    pub ran: bool,
    pub explains: Vec<(String, Option<String>)>,
    pub record: J,
    /// The digest of the request with its absolute paths left out (`graph.md` 5.1 and 6.3: a
    /// plain command's one read, method `data`), once the request was made.
    pub data: Option<Digest>,
    /// Whether the call completed (`graph.md` 3.4): it started, exited 0 and returned readable
    /// output. A call that never started, failed, was killed or returned nothing readable did
    /// not, and is not stored.
    pub complete: bool,
    /// A worker's traced reads, in the order made (`graph.md` 5.1).
    pub reads: Vec<via_graph::Read>,
    /// The stated facts it read (`derived.md` 5), sorted.
    pub facts: Vec<String>,
    /// A generate step's document, checked.
    pub document: Option<String>,
    /// Whether it returned rows or features.
    pub rows_or_features: bool,
    /// The keys it returned, by table, for `identity.key_collision` across items.
    pub keys: BTreeMap<String, Vec<String>>,
    /// Whether it returned a checked document with tables, which cannot join the build yet.
    pub unjoined: bool,
    /// A traced lint whose read reached a failed or skipped node: skipped, findings dropped.
    pub skipped: bool,
    /// The diagnostics the step raised itself (a worker's `diag.emit` records as served, a plain
    /// command's `diagnostics.json`), as the build holds them: what a committed result replays
    /// (`protocol.md` 7.1).
    pub emitted: Vec<Diag>,
}

impl Ran {
    pub(crate) fn new() -> Ran {
        Ran {
            diags: Vec::new(),
            ran: false,
            explains: Vec::new(),
            record: J::Null,
            data: None,
            complete: false,
            reads: Vec::new(),
            facts: Vec::new(),
            document: None,
            rows_or_features: false,
            keys: BTreeMap::new(),
            unjoined: false,
            skipped: false,
            emitted: Vec::new(),
        }
    }
}

/// What a lint call is handed beyond the step.
pub struct Call<'a> {
    pub world: &'a World,
    pub build: &'a Build,
    pub store: &'a Store,
    pub roots: &'a Roots,
    pub config: Option<&'a Path>,
    pub sdk: Option<&'a Path>,
    pub sdk_digest: Option<&'a Digest>,
    pub seed: u64,
    pub deadline: Duration,
    /// The world's pack: each instance's taxon for `data.instances`.
    pub pack: Option<&'a via_model::pack::Pack>,
    /// `via trust` and, in words, this build's flags that enter the trust digest (`--lib`,
    /// `--sdk`, `--change`, `protocol.md` 7): [`trust_command`].
    pub trust_command: &'a str,
    /// The build's running workers (`protocol.md` 3.2).
    pub pool: Option<&'a crate::stepwork::Pool>,
    /// Under `for_each`, the item of this call.
    pub item: Option<&'a str>,
    /// A lint's `checks` member (`derived.md` 5), `null` otherwise.
    pub checks: J,
    /// The model, for a generate step's head view of its stratum (`protocol.md` 1).
    pub stratum: Option<&'a via_model::ModelPlan<'a>>,
    /// For a traced lint: what failed or was skipped in this build (`diagnostics.md` 4).
    pub bad: Option<&'a crate::stepview::Bad>,
}

impl Call<'_> {
    /// The rank whose stratum a step reads: its layer's for a generate step, `None` (the
    /// finished build) for a lint or export.
    pub fn rank(&self, s: &StepDecl) -> Option<i64> {
        (s.when == "generate")
            .then(|| self.world.layer_rank(&s.layer))
            .flatten()
    }

    /// The request's seed (`protocol.md` 2): the step's stream, and the item's under `for_each`.
    pub fn seed_of(&self, s: &StepDecl) -> u64 {
        via_num::world_seed(i64::try_from(self.seed).unwrap_or(0))
            .map(|w| match self.item {
                Some(i) => via_num::stream(w, &["step", s.id.as_str(), i]),
                None => via_num::stream(w, &["step", s.id.as_str()]),
            })
            .unwrap_or(0)
    }

    /// The world's grid as the request gives it.
    pub fn grid_json(grid: &Grid) -> J {
        let [x0, y0, x1, y1] = grid.extent;
        obj([
            (
                "extent_mm",
                J::Arr(vec![J::int(x0), J::int(y0), J::int(x1), J::int(y1)]),
            ),
            ("nx", J::Num(grid.nx.to_string())),
            ("ny", J::Num(grid.ny.to_string())),
            ("resolution_mm", J::int(grid.resolution)),
        ])
    }
}

/// `via trust WORLD` as a world record may say it: no machine path (`diagnostics.md` 5, R26). The
/// world file is named as records name it, by its file name; the flags that enter the trust
/// digest (`--lib`, `--sdk`, `--change`, `protocol.md` 7) are named in words, so the hint is the
/// same wherever the build was run from.
pub fn trust_command(
    world: &Path,
    lib: &[PathBuf],
    sdk: Option<&Path>,
    changes: &[String],
) -> String {
    let name = world.file_name().map_or_else(
        || "world.toml".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut flags: Vec<&str> = Vec::new();
    if !lib.is_empty() {
        flags.push("`--lib`");
    }
    if sdk.is_some() {
        flags.push("`--sdk`");
    }
    if !changes.is_empty() {
        flags.push("`--change`");
    }
    let mut out = format!("`via trust {name}`");
    match flags.as_slice() {
        [] => {}
        [one] => out.push_str(&format!(" with the same {one} as this build")),
        [rest @ .., last] => out.push_str(&format!(
            " with the same {} and {last} as this build",
            rest.join(", ")
        )),
    }
    out
}

pub(crate) struct Mapper<'a> {
    pub(crate) world: &'a World,
    pub(crate) grid: Option<Grid>,
}

impl Mapper<'_> {
    /// Whether a selector of this class selects anything in the merged world.
    fn selects(&self, class: &str, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        match class {
            "kind" => self.world.entity("kind", name).is_some(),
            "role" | "tag" => self.world.root().table("field").is_some_and(|t| {
                t.iter().any(|(_, e)| match &e.item.value {
                    via_doc::Value::Table(f) => match class {
                        "role" => f.str("role") == Some(name),
                        _ => f.value("tags").is_some_and(|v| match v {
                            via_doc::Value::List(a) => a.iter().any(
                                |x| matches!(&x.value, via_doc::Value::Str(s) if s.text == name),
                            ),
                            _ => false,
                        }),
                    },
                    _ => false,
                })
            }),
            _ => false,
        }
    }
}

impl StepWorld for Mapper<'_> {
    fn map_subject(&self, written: &str) -> Option<Mapped> {
        // The world as a whole (`diagnostics.md` 1.1).
        if written == "world" {
            return Some(Mapped {
                subject: Subject::World,
                produced: None,
                place: None,
            });
        }
        // A selector subject must select something of this world (`diagnostics.md` 6): a kind
        // the merge declares, a role some field holds, a tag some field carries.
        if let Some((class, name)) = written.split_once(':') {
            if matches!(class, "kind" | "role" | "tag") {
                return self.selects(class, name).then(|| Mapped {
                    subject: Subject::Selector(written.to_string()),
                    produced: None,
                    place: None,
                });
            }
        }
        if let Ok(s) = Subject::parse(written) {
            if let Subject::Entity { collection, id, .. } = &s {
                if self.world.entity(collection, id).is_some() {
                    return Some(Mapped {
                        subject: s,
                        produced: None,
                        place: None,
                    });
                }
            }
        }
        if let Some((head, _)) = written.split_once('#') {
            // `instance.<producer>#<key>`, as a hand instance is written, names the same produced
            // instance as the bare `<producer>#<key>` (a run's member, a scatter's tree).
            let cut = head.find('.').map_or(0, |k| k + 1);
            let (producer, written) = (&head[cut..], &written[cut..]);
            let (top, _) = self.world.find(producer)?;
            return Some(Mapped {
                subject: Subject::entity(top, producer),
                produced: Some(written.to_string()),
                place: None,
            });
        }
        let (id, place) = match written.split_once('.') {
            Some((a, b)) => (a, Some(b.to_string())),
            None => (written, None),
        };
        let (top, _) = self.world.find(id)?;
        Some(Mapped {
            subject: Subject::entity(top, id),
            produced: None,
            place,
        })
    }

    fn to_lattice(&self, metres: f64) -> Option<i64> {
        if !metres.is_finite() {
            return None;
        }
        via_num::round_to_lattice(metres * 1000.0).ok()
    }

    fn in_extent(&self, x: i64, y: i64) -> bool {
        self.grid.is_some_and(|g| {
            let [x0, y0, x1, y1] = g.extent;
            x >= x0 && x <= x1 && y >= y0 && y <= y1
        })
    }

    /// The step's `for_each` key, which wrote the selector, else the step's header
    /// (`diagnostics.md` 6). The site is the step table's text, so it is given as that text.
    fn selector_site(&self, step: &str, _selector: &str) -> Site {
        use via_diag::Resolve;
        let subject = Subject::entity("step", step);
        let resolved = self
            .world
            .resolve(&subject, &Site::Key(via_diag::KeyPath::key("for_each")))
            .or_else(|| self.world.resolve(&subject, &Site::Header));
        match resolved.map(|r| r.location) {
            Some(via_diag::Location::Span {
                file,
                line,
                column,
                bytes,
            }) => Site::Text {
                file,
                line,
                column,
                bytes,
            },
            _ => Site::World,
        }
    }
}

fn raw(v: &J) -> RawValue {
    match v {
        J::Null => RawValue::Null,
        J::Bool(b) => RawValue::Bool(*b),
        J::Num(n) => RawValue::Number(n.parse().unwrap_or(f64::NAN)),
        J::Str(s) => RawValue::Text(s.clone()),
        J::Arr(a) => RawValue::List(a.iter().map(raw).collect()),
        J::Obj(m) => RawValue::Table(m.iter().map(|(k, v)| (k.clone(), raw(v))).collect()),
    }
}

fn nums(v: Option<&J>) -> Option<Vec<f64>> {
    v?.as_arr()?
        .iter()
        .map(|x| match x {
            J::Num(n) => n.parse().ok(),
            _ => None,
        })
        .collect()
}

fn step_in(o: &J) -> StepDiagIn {
    let s = |k: &str| o.get(k).and_then(J::as_str).map(str::to_string);
    StepDiagIn {
        severity: s("severity"),
        code: s("code"),
        subject: s("subject"),
        message: s("message"),
        hint: s("hint"),
        position: nums(o.get("position")),
        related: o
            .get("related")
            .and_then(J::as_arr)
            .map(|rs| {
                rs.iter()
                    .map(|r| StepRelatedIn {
                        subject: r.get("subject").and_then(J::as_str).map(str::to_string),
                        position: nums(r.get("position")),
                        note: r.get("note").and_then(J::as_str).map(str::to_string),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        data: o.get("data").map(raw),
        explain: s("explain"),
    }
}

pub(crate) fn resolve_program(name: &str, s: &StepDecl, call: &Call<'_>) -> Result<PathBuf, Diag> {
    if let Some(rest) = name.strip_prefix("lib:") {
        let (lib, path) = rest.split_once('/').unwrap_or((rest, ""));
        return call
            .roots
            .library(lib)
            .map(|d| d.join(path))
            .ok_or_else(|| {
                unknown_library(
                    &s.id,
                    format!(
                        "step `{}` names `{name}`, and no library root holds `{lib}`",
                        s.id
                    ),
                    lib,
                    call.roots,
                )
            });
    }
    if name.contains('/') {
        return Ok(absolute(&s.home.join(name)));
    }
    let lower = name.to_ascii_lowercase();
    let no_program = |why: String| {
        step_diag(
            &s.id,
            codes::protocol::NO_PROGRAM,
            why,
            format!("via config program {name} <path>"),
        )
        .with("program", name.to_string())
    };
    if lower.ends_with(".bat") || lower.ends_with(".cmd") {
        return Err(no_program(format!(
            "step `{}`: `{name}` is a batch file, which is refused as a program",
            s.id
        )));
    }
    let Some(config) = call.config else {
        return Err(no_program(format!(
            "step `{}`: no user config folder, so no program named `{name}`",
            s.id
        )));
    };
    via_proto::program::resolve(name, config).map_err(|_| {
        no_program(format!(
            "step `{}`: no program named `{name}` is configured",
            s.id
        ))
    })
}

/// The request of a call (`protocol.md` 2), given its `data.features` (a plain command's; `None`
/// for a worker, which is handed no `data`), its inputs, the objects folder and the call's `out`.
fn make_request(
    s: &StepDecl,
    call: &Call<'_>,
    grid: &Grid,
    features: Option<J>,
    inputs: Vec<J>,
    objects: String,
    out: String,
) -> J {
    let seed = call.seed_of(s);
    let rank = call.rank(s);
    let mut m = J::obj();
    if let Some(features) = features {
        let mut data = obj([
            ("features", features),
            (
                "fields",
                fields_data(call.store, call.build, grid, call.world, rank),
            ),
            ("inputs", J::Arr(inputs)),
            (
                "instances",
                instances_data(call.store, call.build, call.pack, call.world, rank),
            ),
        ]);
        // `data.pack` (2.2, M2): every asset, every fact, as `pack.query` gives it.
        if let (Some(pack_table), J::Obj(d)) = (pack_data(s, call, grid), &mut data) {
            d.insert("pack".into(), pack_table);
        }
        m.insert("data".into(), data);
    }
    m.insert("checks".into(), call.checks.clone());
    m.insert("for_each_item".into(), J::opt(call.item));
    m.insert("grid".into(), Call::grid_json(grid));
    m.insert("objects".into(), J::s(objects));
    m.insert("out".into(), J::s(out));
    m.insert("params".into(), s.params.clone());
    m.insert("protocol".into(), J::s(via_proto::PROTOCOL));
    m.insert("seed".into(), J::s(format!("{seed:016x}")));
    m.insert("step".into(), J::s(s.id.clone()));
    m.insert("when".into(), J::s(s.when.clone()));
    J::Obj(m)
}

/// `data.pack` (2.2): `pack.query`'s table of every asset with every fact; `None` without a pack.
fn pack_data(s: &StepDecl, call: &Call<'_>, grid: &Grid) -> Option<J> {
    call.pack?;
    let objects = via_proto::store::Store::open(call.store.root().join("objects")).ok()?;
    let view = crate::stepview::View {
        world: call.world,
        build: call.build,
        grid: *grid,
        objects: &objects,
        pack: call.pack,
        step: s,
        rank: None,
        seed: 0,
        stratum: None,
        bad: None,
    };
    let v = view
        .answer(
            "pack.query",
            &serde_json::json!({"select": "*", "columns": "*"}),
        )
        .ok()?;
    let text = serde_json::to_string(&v).ok()?;
    crate::json::parse(&text).ok()
}

/// A worker's request (3.2): the request without `data`.
pub(crate) fn request_for_worker(
    s: &StepDecl,
    call: &Call<'_>,
    grid: &Grid,
    objects: String,
    out: String,
) -> J {
    make_request(s, call, grid, None, Vec::new(), objects, out)
}

/// The digest of the request a lint would be handed, its absolute paths left out (`graph.md`
/// 5.1): part of a plain command's static key, made before the lookup. A request that cannot be
/// made (no grid, an attribute typed two ways, a missing input) is named by a marker: what keeps
/// it from being made is in the world or in the inputs' digests, both in the key.
pub fn request_key(s: &StepDecl, call: &Call<'_>) -> Digest {
    let Some(grid) = call.build.grid else {
        return Digest::of(b"via-request-1 no grid");
    };
    let features = match features_data(call.store, call.build, call.world, call.rank(s)) {
        Ok(f) => f,
        Err(m) => {
            return Digest::of(format!("via-request-1 mixed {}", m.attr).as_bytes());
        }
    };
    let mut sorted_inputs = s.inputs.clone();
    sorted_inputs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let mut inputs: Vec<J> = Vec::new();
    for i in &sorted_inputs {
        match input_digest(&s.home.join(i)) {
            Some(d) => inputs.push(obj([("digest", J::s(d.hex())), ("path", J::s(i.clone()))])),
            None => return Digest::of(b"via-request-1 input missing"),
        }
    }
    let request = make_request(
        s,
        call,
        &grid,
        Some(features),
        inputs,
        String::new(),
        String::new(),
    );
    Digest::of(&request_data(&request).file())
}

/// A request with its absolute paths left out (`graph.md` 5.1): `objects`, `out` and each
/// input's `absolute`. Blobs are named by digest, so it still names every field it hands over.
fn request_data(request: &J) -> J {
    let mut r = request.clone();
    if let J::Obj(m) = &mut r {
        m.remove("objects");
        m.remove("out");
        if let Some(J::Obj(data)) = m.get_mut("data") {
            if let Some(J::Arr(inputs)) = data.get_mut("inputs") {
                for i in inputs.iter_mut() {
                    if let J::Obj(im) = i {
                        im.remove("absolute");
                    }
                }
            }
        }
        if let Some(data) = m.get_mut("data") {
            drop_blob_paths(data);
        }
    }
    r
}

/// Removes `path` from every blob object (one with a `blob` digest) under `j`: a field's file or a
/// column over 4,096 values. The path is where the store is (`<store>/objects/..`); the blob is
/// named by its digest, so nothing a step reads is lost and the read is the same in every store.
fn drop_blob_paths(j: &mut J) {
    match j {
        J::Obj(m) => {
            if m.contains_key("blob") {
                m.remove("path");
            }
            for v in m.values_mut() {
                drop_blob_paths(v);
            }
        }
        J::Arr(a) => {
            for v in a.iter_mut() {
                drop_blob_paths(v);
            }
        }
        _ => {}
    }
}

/// `build.write_failed` for a step's call folder or request, at the step: the path and the
/// system's error.
pub(crate) fn write_failed(step: &str, path: &Path, e: &std::io::Error) -> Diag {
    step_diag(
        step,
        codes::build::WRITE_FAILED,
        format!("step `{step}`: `{}` could not be written: {e}", slash(path)),
        "free space, or fix the folder's permissions".into(),
    )
    .with("path", slash(path))
    .with("error", e.to_string())
}

/// How long a step's output streams may stay open after its deadline before they are abandoned.
const STREAM_GRACE: Duration = Duration::from_secs(2);

/// The most of each stream (standard output, standard error) a step's log keeps.
const LOG_CAP: usize = 1 << 20;

/// One output stream of a step, read on its own thread into a capped buffer. Bytes beyond
/// [`LOG_CAP`] are read and dropped, so the step never blocks on a full pipe, and the log says it
/// was cut. The buffer is shared, so what was read is kept even when the thread is abandoned.
struct Capture {
    buf: std::sync::Arc<std::sync::Mutex<(Vec<u8>, usize)>>,
    done: std::sync::mpsc::Receiver<()>,
    finished: std::cell::Cell<bool>,
}

impl Capture {
    fn start(r: Option<Box<dyn std::io::Read + Send>>) -> Capture {
        let buf = std::sync::Arc::new(std::sync::Mutex::new((Vec::new(), 0usize)));
        let (tx, done) = std::sync::mpsc::channel();
        let shared = buf.clone();
        std::thread::spawn(move || {
            if let Some(mut r) = r {
                let mut chunk = [0u8; 8192];
                loop {
                    match r.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut g = shared
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            let space = LOG_CAP.saturating_sub(g.0.len());
                            let keep = n.min(space);
                            g.0.extend_from_slice(&chunk[..keep]);
                            g.1 += n - keep;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
            }
            let _ = tx.send(());
        });
        Capture {
            buf,
            done,
            finished: std::cell::Cell::new(false),
        }
    }

    /// Waits for the stream's end until `until` (with no end when `None`); false when it is still
    /// open then.
    fn wait(&self, until: Option<Instant>) -> bool {
        if self.finished.get() {
            return true;
        }
        let ok = match until {
            Some(u) => self
                .done
                .recv_timeout(u.saturating_duration_since(Instant::now()))
                .is_ok(),
            None => self.done.recv().is_ok(),
        };
        self.finished.set(ok);
        ok
    }

    /// What was read, with a closing line when bytes were dropped.
    fn take(&self) -> Vec<u8> {
        let g = self
            .buf
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out = g.0.clone();
        if g.1 > 0 {
            out.extend_from_slice(
                format!(
                    "\n[via: log cut at {LOG_CAP} bytes; {} more dropped]\n",
                    g.1
                )
                .as_bytes(),
            );
        }
        if !self.finished.get() {
            out.extend_from_slice(b"\n[via: stream still open at the deadline; abandoned]\n");
        }
        out
    }
}

/// The stated facts among these feature names (`derived.md` 5): written in text, shaped or
/// anchored, sorted.
pub(crate) fn stated_facts(call: &Call<'_>, names: &[String]) -> Vec<String> {
    let mut v: Vec<String> = names
        .iter()
        .filter(|n| {
            // A produced feature (`<step>#<key>`) was found, not stated (`derived.md` 5).
            !n.contains('#')
                && call.world.entity("feature", n).is_some()
                && call.build.features.get(*n).is_some_and(|f| {
                    matches!(
                        f.tier,
                        Some(via_model::Tier::Shaped | via_model::Tier::Anchored)
                    )
                })
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    v.dedup();
    v
}

/// Runs one call of a step, when it is trusted: a worker (3) or a plain command (2.2).
pub fn run_step(s: &StepDecl, code: &Code, trusted: bool, call: &Call<'_>) -> Ran {
    let mut ran = Ran::new();
    if !trusted {
        if s.when == "generate" {
            ran.diags.push(step_diag(
                &s.id,
                codes::protocol::UNTRUSTED,
                format!(
                    "generate step `{}` is not trusted on this machine and has no committed result",
                    s.id
                ),
                format!("run {}, after reading its code folder", call.trust_command),
            ));
        } else {
            ran.diags.push(step_diag(
                &s.id,
                codes::protocol::UNTRUSTED_SKIPPED,
                format!(
                    "{} step `{}` is not trusted on this machine and was skipped",
                    s.when, s.id
                ),
                format!("run {}, after reading its code folder", call.trust_command),
            ));
        }
        return ran;
    }
    if let Some(rank) = call.rank(s) {
        if !s.worker && crate::stepview::stratum_inexact(call.world, call.build, rank) {
            ran.diags.push(step_diag(
                &s.id,
                codes::doc::NOT_YET,
                format!(
                    "plain generate step `{}` reads the view of layer `{}`, which statements of a higher layer change; that view is owed by the model",
                    s.id, s.layer
                ),
                "make the step a worker that reads only what it needs, or move it to the top layer".into(),
            ));
            return ran;
        }
    }
    if s.worker {
        return crate::steprun::run_worker(s, code, call);
    }
    run_plain(s, code, call)
}

/// A call folder's name past the step id: the process, the clock and a count kept for the
/// process, so two `for_each` items of one step that start in the same clock tick (macOS gives
/// microseconds) never share a folder.
pub(crate) fn call_nonce() -> String {
    static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
        CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// Runs one call of a step as a plain command (2.2).
fn run_plain(s: &StepDecl, code: &Code, call: &Call<'_>) -> Ran {
    let mut ran = Ran::new();
    let Some(grid) = call.build.grid else {
        return ran;
    };
    let program = match s.command.first() {
        Some(p) => match resolve_program(p, s, call) {
            Ok(p) => p,
            Err(d) => {
                ran.diags.push(d);
                return ran;
            }
        },
        // `protocol.md` 1: `command` is required, a program and its arguments. Nothing to start
        // is said at the step, never passed as a clean run.
        None => {
            let written = call
                .world
                .entity("step", &s.id)
                .is_some_and(|t| t.get("command").is_some());
            ran.diags.push(if written {
                Diag::new(
                    codes::protocol::START,
                    Subject::entity("step", s.id.clone()),
                    Site::Key(via_diag::KeyPath::key("command")),
                    txt(format!(
                        "step `{}` could not be started: its `command` is empty",
                        s.id
                    )),
                    txt("write the program and its arguments, `command = [\"python\", \"steps/lint.py\"]`".into()),
                )
                .with("kind", "empty")
            } else {
                Diag::new(
                    codes::doc::MISSING_KEY,
                    Subject::entity("step", s.id.clone()),
                    Site::Header,
                    txt(format!("step `{}` has no `command`", s.id)),
                    txt("write the program and its arguments, `command = [\"python\", \"steps/lint.py\"]`".into()),
                )
            });
            return ran;
        }
    };
    // `data.features` first: an attribute two kinds type differently cannot be handed (4.3).
    let features = match features_data(call.store, call.build, call.world, call.rank(s)) {
        Ok(f) => f,
        Err(m) => {
            let kind_related = |k: &str, note: &str| via_diag::Related {
                subject: Subject::entity("kind", k),
                site: Site::Header,
                position: None,
                note: txt(note.to_string()),
            };
            let mut d = step_diag(
                &s.id,
                codes::protocol::COLUMN_MIXED,
                format!(
                    "step `{}`: the attribute `{}` is {} on kind `{}` and {} on kind `{}`, so it cannot be one column",
                    s.id, m.attr, m.a.1, m.a.0, m.b.1, m.b.0
                ),
                format!(
                    "give `{}` one type in both kinds, or read each kind by its own `kind:` selector",
                    m.attr
                ),
            )
            .with("attribute", m.attr.clone())
            .with("kinds", via_diag::Value::texts([m.a.0.clone(), m.b.0.clone()]));
            for (k, _) in [&m.a, &m.b] {
                if !k.is_empty() && call.world.entity("kind", k).is_some() {
                    d = d.related(kind_related(k, "a kind giving the attribute"));
                }
            }
            ran.diags.push(d);
            return ran;
        }
    };
    // The call folder (6.3) and the request (2).
    let nonce = call_nonce();
    let folder = call
        .store
        .root()
        .join("calls")
        .join(format!("{}-{nonce}", s.id));
    // Counted from here, and dropped after the process tree: on Ctrl+C the tree is killed, the
    // call folder removed, and the engine ends (protocol.md 6.2).
    let _live = crate::interrupt::Live::enter(folder.clone());
    let out = folder.join("out");
    if let Err(e) = std::fs::create_dir_all(&out) {
        ran.diags.push(write_failed(&s.id, &out, &e));
        let _ = std::fs::remove_dir_all(&folder);
        return ran;
    }
    let mut inputs: Vec<J> = Vec::new();
    let mut sorted_inputs = s.inputs.clone();
    sorted_inputs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for i in &sorted_inputs {
        let p = s.home.join(i);
        match input_digest(&p) {
            Some(d) => inputs.push(obj([
                ("absolute", J::s(slash(&p))),
                ("digest", J::s(d.hex())),
                ("path", J::s(i.clone())),
            ])),
            None => ran.diags.push(
                step_diag(
                    &s.id,
                    codes::graph::INPUT_MISSING,
                    format!("step `{}`: the input `{i}` does not exist", s.id),
                    "create it, or remove it from `inputs`".into(),
                )
                .with("path", i.clone()),
            ),
        }
    }
    if !ran.diags.is_empty() {
        let _ = std::fs::remove_dir_all(&folder);
        return ran;
    }
    let request = make_request(
        s,
        call,
        &grid,
        Some(features),
        inputs,
        slash(&call.store.root().join("objects")),
        slash(&out),
    );
    ran.data = Some(Digest::of(&request_data(&request).file()));
    let req_path = folder.join("request.json");
    if let Err(e) = std::fs::write(&req_path, request.file()) {
        ran.diags.push(write_failed(&s.id, &req_path, &e));
        let _ = std::fs::remove_dir_all(&folder);
        return ran;
    }
    let mut cmd = std::process::Command::new(&program);
    for a in s.command.iter().skip(1) {
        match a.strip_prefix("lib:") {
            Some(rest) => {
                let (lib, path) = rest.split_once('/').unwrap_or((rest, ""));
                let p = call
                    .roots
                    .library(lib)
                    .map(|d| d.join(path))
                    .unwrap_or_default();
                cmd.arg(p);
            }
            None => {
                cmd.arg(a);
            }
        }
    }
    let workdir = code
        .folder
        .clone()
        .filter(|_| s.command.iter().any(|c| c.starts_with("lib:")))
        .unwrap_or_else(|| s.home.clone());
    cmd.current_dir(workdir)
        .env("VIA_STEP", &req_path)
        .env("VIA_PROTOCOL", via_proto::PROTOCOL)
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONSAFEPATH", "1")
        .env("PYTHONUTF8", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    match sdk_folder(call.store, call.sdk) {
        Ok(sdk) => {
            let mut paths = vec![sdk];
            if let Some(old) = std::env::var_os("PYTHONPATH") {
                paths.extend(std::env::split_paths(&old));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env("PYTHONPATH", joined);
            }
        }
        Err((path, e)) => {
            // Never run without the SDK: its import error would be a wrong cause.
            ran.diags.push(write_failed(&s.id, &path, &e));
            let _ = std::fs::remove_dir_all(&folder);
            return ran;
        }
    }
    let _ = call.sdk_digest;
    let mut tree = match via_proto::process::Tree::spawn(cmd) {
        Ok(t) => t,
        Err(e) => {
            let kind = match e.kind() {
                std::io::ErrorKind::NotFound => "not_found",
                std::io::ErrorKind::PermissionDenied => "denied",
                _ => "other",
            };
            ran.diags.push(
                step_diag(
                    &s.id,
                    codes::protocol::START,
                    format!("step `{}` could not be started ({kind})", s.id),
                    "check the program the step names".into(),
                )
                .with("kind", kind),
            );
            let _ = std::fs::remove_dir_all(&folder);
            return ran;
        }
    };
    let out_t = Capture::start(
        tree.child_mut()
            .stdout
            .take()
            .map(|r| Box::new(r) as Box<dyn std::io::Read + Send>),
    );
    let err_t = Capture::start(
        tree.child_mut()
            .stderr
            .take()
            .map(|r| Box::new(r) as Box<dyn std::io::Read + Send>),
    );
    let start = Instant::now();
    let status = loop {
        if crate::interrupt::pending().is_some() {
            // Ctrl+C or SIGTERM: the step is killed now; `_live` removes the call folder and
            // ends the engine when this returns.
            let _ = tree.kill();
            let _ = std::fs::remove_dir_all(&folder);
            return ran;
        }
        match tree.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) if start.elapsed() > call.deadline => {
                let _ = tree.kill();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };
    // The readers end when every holder of the pipes closes them. A descendant that left the
    // process group (and so outlived the kill) may hold them open: the readers are waited for
    // until the deadline, plus a short grace after a kill, and then abandoned, never joined.
    // Checked: a deadline past what an instant holds waits with no end, never panics.
    let until = start
        .checked_add(call.deadline)
        .and_then(|t| t.checked_add(STREAM_GRACE));
    let streams_done = out_t.wait(until) & err_t.wait(until);
    let mut log = out_t.take();
    log.extend(err_t.take());
    let log_digest = call.store.put_object(&log);
    ran.ran = true;
    let status = if streams_done {
        status
    } else {
        // Its output never ended: it is not a finished call, whatever its exit (6.2).
        let _ = tree.kill();
        None
    };
    let exit = status.and_then(|s| s.code());
    match (status, exit) {
        (None, _) => ran.diags.push(
            step_diag(
                &s.id,
                codes::protocol::DEADLINE,
                if streams_done {
                    format!(
                        "step `{}` did not finish before its deadline and was killed",
                        s.id
                    )
                } else {
                    format!(
                        "step `{}`: its output was still held open at its deadline (a process it started outlived it)",
                        s.id
                    )
                },
                if streams_done {
                    "raise `--step-deadline`, or make the step faster".into()
                } else {
                    "end every process the step starts before it exits".into()
                },
            )
            .with("phase", "step.call")
            .with(
                "deadline",
                i64::try_from(call.deadline.as_secs()).unwrap_or(i64::MAX),
            ),
        ),
        (Some(_), Some(0)) => {}
        (Some(_), code) => {
            let e = code.unwrap_or(-1);
            ran.diags.push(
                step_diag(
                    &s.id,
                    codes::protocol::FAILED,
                    format!("step `{}` exited with status {e}", s.id),
                    "read the step's log in the run report".into(),
                )
                .with("exit", i64::from(e)),
            );
        }
    }
    // What it returned (5).
    let returned_out = crate::stepout::collect(
        &out,
        &s.id,
        &s.when,
        None,
        &grid,
        &crate::stepout::Names {
            world: call.world,
            rank: call.rank(s),
        },
        call.store,
    );
    let returned: Vec<J> = returned_out
        .files
        .iter()
        .map(|(p, d)| obj([("digest", J::s(d.hex())), ("path", J::s(p.clone()))]))
        .collect();
    let links: Vec<String> = Vec::new();
    let generated = exit == Some(0) && s.when == "generate";
    if exit == Some(0) {
        ran.diags.extend(returned_out.diags.iter().cloned());
    }
    if generated {
        ran.rows_or_features = returned_out.returns_rows_or_features();
        ran.keys = returned_out.keys.clone();
        ran.unjoined = returned_out.ok && returned_out.has_tables();
        ran.document = returned_out.document.clone();
        // A plain command read every stated fact in its stratum (`derived.md` 5).
        let view_rows: Vec<String> = call
            .build
            .features
            .keys()
            .filter(|n| {
                let r = call.world.rank("feature", n);
                match (call.rank(s), r) {
                    (Some(own), Some(r)) => r <= own,
                    _ => true,
                }
            })
            .cloned()
            .collect();
        ran.facts = stated_facts(call, &view_rows);
    }
    ran.complete = status.is_some() && exit == Some(0) && links.is_empty() && returned_out.ok;
    let diagnostics_file = out.join("diagnostics.json");
    let is_link =
        std::fs::symlink_metadata(&diagnostics_file).is_ok_and(|m| m.file_type().is_symlink());
    if exit == Some(0) && !is_link {
        if let Ok(text) = std::fs::read_to_string(&diagnostics_file) {
            match crate::json::parse(&text) {
                Ok(J::Arr(items)) if items.iter().all(|i| matches!(i, J::Obj(_))) => {
                    let mapper = Mapper {
                        world: call.world,
                        grid: call.build.grid,
                    };
                    let mut ex = Explains::new();
                    ex.begin_call();
                    let mut ds = Vec::new();
                    for it in &items {
                        let d = step_in(it);
                        if let Some(name) = &d.code {
                            if let Some(extra) = ex.observe(&s.id, name, d.explain.as_deref()) {
                                ds.push(extra);
                            }
                        }
                        ds.push(via_diag::step::arrive(&s.id, &d, &mapper));
                    }
                    let settled = via_diag::step::settle(ds);
                    ran.emitted.clone_from(&settled);
                    ran.diags.extend(settled);
                    ran.explains = ex
                        .entries()
                        .map(|(c, e)| (c.to_string(), e.map(str::to_string)))
                        .collect();
                }
                _ => {
                    // Nothing readable: not complete, so not stored (`graph.md` 3.4).
                    ran.complete = false;
                    ran.diags.push(step_diag(
                        &s.id,
                        codes::protocol::OUTPUT_JSON,
                        format!(
                            "step `{}`: `diagnostics.json` is not a JSON list of objects",
                            s.id
                        ),
                        "write a list of objects, one per diagnostic".into(),
                    ));
                }
            }
        }
    }
    ran.record = obj([
        ("command", J::strs(s.command.iter().map(String::as_str))),
        ("exit", exit.map_or(J::Null, |e| J::int(i64::from(e)))),
        ("id", J::s(format!("step:{}", s.id))),
        ("log", J::s(log_digest.hex())),
        ("returned", J::Arr(returned)),
        ("trust", J::s(code.trust.hex())),
    ]);
    let _ = std::fs::remove_dir_all(&folder);
    ran
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small world of its own: one kind from a local library, one feature of it, one step.
    const WORLD: &str = "name = \"marks\"\nextent = [0, 0, 16, 16]\nresolution = 1\n\
        include = [\"marks\"]\n\n[field.height]\nrole = \"surface\"\nbase = 0\n\n\
        [feature.one]\nkind = \"mark\"\npath = [[2, 8], [12, 8]]\n\n\
        [step.checks]\nwhen = \"lint\"\ncommand = [\"python\", \"steps/lint.py\"]\n";
    const KINDS: &str = "[kind.mark]\nauthority = \"shaped\"\nradius = 2\n\n\
        [kind.mark.effect.e]\nfield = \"role:surface\"\nvalue = \"= v + 1\"\n\
        mask = \"= 1 - smoothstep(0, 1, d)\"\n";

    fn small_world() -> World {
        let dir = std::env::temp_dir().join(format!("via-steps-test-{}", std::process::id()));
        let libs = dir.join("libs");
        std::fs::create_dir_all(libs.join("marks")).expect("a scratch folder");
        std::fs::write(dir.join("world.toml"), WORLD).expect("the world");
        std::fs::write(libs.join("marks/kinds.toml"), KINDS).expect("the library");
        let opts = via_doc::LoadOptions {
            lib_roots: vec![libs],
            via_lib: None,
            shipped_root: None,
            changes: Vec::new(),
        };
        let world = via_doc::load(&dir.join("world.toml"), 2, &opts).world;
        let _ = std::fs::remove_dir_all(&dir);
        world.expect("the world merges")
    }

    fn lint(subject: &str) -> StepDiagIn {
        StepDiagIn {
            severity: Some("warning".into()),
            code: Some("l7".into()),
            subject: Some(subject.into()),
            message: Some("a lint".into()),
            hint: Some("change it".into()),
            position: None,
            related: Vec::new(),
            data: None,
            explain: None,
        }
    }

    /// A selector subject that selects nothing is `diag.malformed`; one that selects something
    /// is located at the step's header, which wrote it (no `for_each` before M3).
    #[test]
    fn selector_subjects_are_checked_and_located_at_the_step() {
        let world = small_world();
        let m = Mapper {
            world: &world,
            grid: None,
        };
        let bad = via_diag::step::arrive("checks", &lint("kind:nothing"), &m);
        assert_eq!(bad.code().to_string(), "diag.malformed");
        let good = via_diag::step::arrive("checks", &lint("kind:mark"), &m);
        assert_eq!(good.code().to_string(), "step.checks.l7");
        match good.site() {
            Site::Text { file, line, .. } => {
                assert_eq!(file, "world.toml");
                let header = WORLD
                    .lines()
                    .position(|l| l.starts_with("[step.checks]"))
                    .expect("a step header");
                assert_eq!(*line as usize, header + 1);
            }
            s => panic!("not at the step: {s:?}"),
        }
        let role = via_diag::step::arrive("checks", &lint("role:nothing"), &m);
        assert_eq!(role.code().to_string(), "diag.malformed");
    }
}
