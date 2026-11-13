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
    /// The folder of the declaring document.
    pub home: PathBuf,
}

/// Where libraries are found (`documents.md` 4), in search order.
#[derive(Clone, Debug, Default)]
pub struct Roots {
    pub world_dir: PathBuf,
    pub lib: Vec<PathBuf>,
    pub via_lib: Vec<PathBuf>,
    pub shipped: Option<PathBuf>,
}

impl Roots {
    /// The folder of library `name`: the first root holding it.
    pub fn library(&self, name: &str) -> Option<PathBuf> {
        std::iter::once(self.world_dir.join("lib"))
            .chain(self.lib.iter().cloned())
            .chain(self.via_lib.iter().cloned())
            .chain(self.shipped.iter().cloned())
            .map(|r| r.join(name))
            .find(|p| p.is_dir())
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

fn step_diag(step: &str, code: via_diag::Code, message: String, hint: String) -> Diag {
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
                return Err(step_diag(
                    &s.id,
                    codes::protocol::UNKNOWN_LIBRARY,
                    format!(
                        "step `{}` names `{el}`, and no library root holds `{lib}`",
                        s.id
                    ),
                    "name a library the world lists in `include`".into(),
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

/// The carried SDK written once to `<store>/sdk/<digest>/`, or `--sdk`'s folder; the folder to put
/// on `PYTHONPATH`.
pub fn sdk_folder(store: &Store, sdk: Option<&Path>) -> Option<PathBuf> {
    // Absolute: the step runs in its own working folder, where a relative path means nothing.
    let absolute = |p: PathBuf| std::path::absolute(&p).unwrap_or(p);
    if let Some(s) = sdk {
        return Some(absolute(s.to_path_buf()));
    }
    let mut e = Enc::new();
    e.str("via-sdk-1").len(carried::FILES.len());
    for (p, b) in carried::FILES {
        e.str(p).digest(&Digest::of(b));
    }
    let d = Digest::of(&e.finish());
    let dir = store.root().join("sdk").join(d.hex());
    if !dir.join(".complete").exists() {
        let tmp = store
            .root()
            .join("sdk")
            .join(format!("{}.tmp{}", d.hex(), std::process::id()));
        for (p, b) in carried::FILES {
            let t = tmp.join(p);
            if let Some(parent) = t.parent() {
                std::fs::create_dir_all(parent).ok()?;
            }
            std::fs::write(&t, b).ok()?;
        }
        std::fs::write(tmp.join(".complete"), b"").ok()?;
        if std::fs::rename(&tmp, &dir).is_err() {
            let _ = std::fs::remove_dir_all(&tmp);
        }
    }
    Some(absolute(dir))
}

fn slash(p: &Path) -> String {
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

/// `data.features` (2.2, 4.3): every feature and footprint feature as one feature list.
fn features_data(store: &Store, b: &Build, world: &World) -> J {
    struct Row {
        name: String,
        kind: String,
        authority: String,
        layer: String,
        geometry: u8,
        ring: Vec<(i64, i64)>,
        attrs: BTreeMap<String, String>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (n, f) in &b.features {
        let mut attrs = f.attrs.clone();
        for (a, (place, _)) in &f.names {
            attrs.insert(a.clone(), place.clone());
        }
        rows.push(Row {
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
        rows.push(Row {
            name: format!("{inst}.ground"),
            kind: f.kind.clone(),
            authority: String::new(),
            layer: world.home("instance", inst).unwrap_or("").into(),
            geometry: 2,
            ring: crate::out::canonical_ring(&f.ring, true),
            attrs: BTreeMap::new(),
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
        let all = |f: &dyn Fn(&str) -> bool| vals.iter().flatten().all(|v| f(v));
        let col = if all(&|v| v == "true" || v == "false") {
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
        } else if all(&|v| v.parse::<f64>().is_ok_and(f64::is_finite)) {
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
    J::Obj(m)
}

/// `data.instances` (2.2): one table, rows by name.
fn instances_data(store: &Store, b: &Build, pack: Option<&via_model::pack::Pack>) -> J {
    let rows: Vec<(&String, &via_model::InstanceOut)> = b.instances.iter().collect();
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
fn fields_data(store: &Store, b: &Build, grid: &Grid) -> J {
    let (nx, ny) = (
        usize::try_from(grid.nx).unwrap_or(0),
        usize::try_from(grid.ny).unwrap_or(0),
    );
    let mut m = J::obj();
    for (name, f) in &b.fields {
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
}

struct Mapper<'a> {
    world: &'a World,
    grid: Option<Grid>,
}

impl StepWorld for Mapper<'_> {
    fn map_subject(&self, written: &str) -> Option<Mapped> {
        if let Some(sel) = written.strip_prefix("kind:") {
            let _ = sel;
            return Some(Mapped {
                subject: Subject::Selector(written.to_string()),
                produced: None,
                place: None,
            });
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
        if let Some((producer, _)) = written.split_once('#') {
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

    fn selector_site(&self, _step: &str, _selector: &str) -> Site {
        Site::World
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

fn resolve_program(name: &str, s: &StepDecl, call: &Call<'_>) -> Result<PathBuf, Diag> {
    if let Some(rest) = name.strip_prefix("lib:") {
        let (lib, path) = rest.split_once('/').unwrap_or((rest, ""));
        return call
            .roots
            .library(lib)
            .map(|d| d.join(path))
            .ok_or_else(|| {
                step_diag(
                    &s.id,
                    codes::protocol::UNKNOWN_LIBRARY,
                    format!(
                        "step `{}` names `{name}`, and no library root holds `{lib}`",
                        s.id
                    ),
                    "name a library the world lists in `include`".into(),
                )
            });
    }
    if name.contains('/') {
        return Ok(s.home.join(name));
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

/// Runs one lint step as a plain command (2.2), when it is trusted.
pub fn run_lint(s: &StepDecl, code: &Code, trusted: bool, call: &Call<'_>) -> Ran {
    let mut ran = Ran {
        diags: Vec::new(),
        ran: false,
        explains: Vec::new(),
        record: J::Null,
    };
    if !trusted {
        ran.diags.push(
            step_diag(
                &s.id,
                codes::protocol::UNTRUSTED_SKIPPED,
                format!(
                    "lint step `{}` is not trusted on this machine and was skipped",
                    s.id
                ),
                "run `via trust <world>` after reading its code folder".into(),
            )
            .with("trust", code.trust.hex()),
        );
        return ran;
    }
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
        None => return ran,
    };
    // The call folder (6.3) and the request (2).
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let folder = call
        .store
        .root()
        .join("calls")
        .join(format!("{}-{nonce}", s.id));
    let out = folder.join("out");
    if std::fs::create_dir_all(&out).is_err() {
        return ran;
    }
    let mut inputs: Vec<J> = Vec::new();
    let mut sorted_inputs = s.inputs.clone();
    sorted_inputs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for i in &sorted_inputs {
        let p = s.home.join(i);
        let digest = if p.is_dir() {
            Some(folder_digest(&p).0)
        } else {
            std::fs::read(&p).ok().map(|b| Digest::of(&b))
        };
        match digest {
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
        return ran;
    }
    let seed = via_num::world_seed(i64::try_from(call.seed).unwrap_or(0))
        .map(|w| via_num::stream(w, &["step", s.id.as_str()]))
        .unwrap_or(0);
    let [x0, y0, x1, y1] = grid.extent;
    let request = obj([
        (
            "data",
            obj([
                (
                    "features",
                    features_data(call.store, call.build, call.world),
                ),
                ("fields", fields_data(call.store, call.build, &grid)),
                ("inputs", J::Arr(inputs)),
                (
                    "instances",
                    instances_data(call.store, call.build, call.pack),
                ),
            ]),
        ),
        ("for_each_item", J::Null),
        (
            "grid",
            obj([
                (
                    "extent_mm",
                    J::Arr(vec![J::int(x0), J::int(y0), J::int(x1), J::int(y1)]),
                ),
                ("nx", J::Num(grid.nx.to_string())),
                ("ny", J::Num(grid.ny.to_string())),
                ("resolution_mm", J::int(grid.resolution)),
            ]),
        ),
        ("objects", J::s(slash(&call.store.root().join("objects")))),
        ("out", J::s(slash(&out))),
        ("params", s.params.clone()),
        ("protocol", J::s(via_proto::PROTOCOL)),
        ("seed", J::s(format!("{seed:016x}"))),
        ("step", J::s(s.id.clone())),
        ("when", J::s(s.when.clone())),
    ]);
    let req_path = folder.join("request.json");
    if std::fs::write(&req_path, request.file()).is_err() {
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
    if let Some(sdk) = sdk_folder(call.store, call.sdk) {
        let mut paths = vec![sdk];
        if let Some(old) = std::env::var_os("PYTHONPATH") {
            paths.extend(std::env::split_paths(&old));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env("PYTHONPATH", joined);
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
    let take = |r: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            if let Some(mut r) = r {
                let _ = r.read_to_end(&mut b);
            }
            b
        })
    };
    let out_t = take(
        tree.child_mut()
            .stdout
            .take()
            .map(|r| Box::new(r) as Box<dyn std::io::Read + Send>),
    );
    let err_t = take(
        tree.child_mut()
            .stderr
            .take()
            .map(|r| Box::new(r) as Box<dyn std::io::Read + Send>),
    );
    let start = Instant::now();
    let status = loop {
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
    let mut log = out_t.join().unwrap_or_default();
    log.extend(err_t.join().unwrap_or_default());
    let log_digest = call.store.put_object(&log);
    ran.ran = true;
    let exit = status.and_then(|s| s.code());
    match (status, exit) {
        (None, _) => ran.diags.push(step_diag(
            &s.id,
            codes::protocol::DEADLINE,
            format!(
                "step `{}` did not finish before its deadline and was killed",
                s.id
            ),
            "raise `--step-deadline`, or make the step faster".into(),
        )),
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
    let mut returned: Vec<J> = Vec::new();
    for f in crate::dir::listing(&out) {
        if let Ok(b) = std::fs::read(out.join(&f)) {
            let d = call.store.put_object(&b);
            returned.push(obj([("digest", J::s(d.hex())), ("path", J::s(f.clone()))]));
        }
        if f == "document.toml" {
            ran.diags.push(step_diag(
                &s.id,
                codes::protocol::OUTPUT_TABLE,
                format!(
                    "lint step `{}` wrote a document; a lint returns diagnostics only",
                    s.id
                ),
                "remove `document.toml` from what the step writes".into(),
            ));
        }
    }
    if exit == Some(0) {
        if let Ok(text) = std::fs::read_to_string(out.join("diagnostics.json")) {
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
                    ran.diags.extend(via_diag::step::settle(ds));
                    ran.explains = ex
                        .entries()
                        .map(|(c, e)| (c.to_string(), e.map(str::to_string)))
                        .collect();
                }
                _ => ran.diags.push(step_diag(
                    &s.id,
                    codes::protocol::OUTPUT_JSON,
                    format!(
                        "step `{}`: `diagnostics.json` is not a JSON list of objects",
                        s.id
                    ),
                    "write a list of objects, one per diagnostic".into(),
                )),
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
