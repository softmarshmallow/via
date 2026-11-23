//! Packs (`spec/assets.md`, the M1 sections): finding a pack (1.2), reading it (2), primitive
//! facts (6), the taxonomy (5) and the facts the change report compares (7).
//!
//! A pack is flat TOML that `via-doc` does not read (it is no world document), so it is read
//! here with the `toml` crate. Its numbers are written back to their shortest decimal text and
//! read by `via-num` as any length is, so `0.15` is 150 mm exactly. Diagnostics name the pack
//! file and, where the header is found in the text, the line (`data.line`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use via_diag::{codes, Code, Diag, Site, Subject};

use crate::text;

/// The shapes a primitive may take, with their fixed triangle counts (6).
const PRIMITIVES: [(&str, u64); 5] = [
    ("box", 12),
    ("cone", 30),
    ("cylinder", 60),
    ("prism", 8),
    ("sphere", 224),
];

/// The keys of `[asset.<id>.mount.<name>]` (2.7).
const MOUNT_KEYS: [&str; 4] = ["at", "connector", "face", "rise"];

const ASSET_KEYS: [&str; 21] = [
    "collision",
    "colour",
    "detail",
    "engine",
    "file",
    "footprint",
    "front",
    "max_relief",
    "mount",
    "params",
    "pivot",
    "plan",
    "primitive",
    "scale",
    "sink",
    "size",
    "solidity",
    "tags",
    "taxon",
    "triangles",
    "up",
];

/// One asset, its facts resolved (2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub id: String,
    /// `primitive` or `engine`.
    pub source: String,
    pub primitive: Option<String>,
    /// `size` in millimetres; for a model, the extent of its measured `bounds`.
    pub size: [i64; 3],
    pub triangles: u64,
    /// `[x0, y0, z0, x1, y1, z1]` in millimetres.
    pub bounds: [i64; 6],
    /// The footprint in the asset frame, counter-clockwise; empty for a point footprint.
    pub footprint: Vec<(i64, i64)>,
    /// What a model covers in plan (3.4), rings in the asset frame, outer counter-clockwise;
    /// empty where it is the rectangle of `bounds` (and for a primitive or engine-made asset).
    pub plan: Vec<Vec<(i64, i64)>>,
    /// The footprint is declared `"plan"` (2.4): the outer ring of `plan`, else the box.
    pub footprint_plan: bool,
    pub solid: bool,
    /// `solidity = "passable"` (2.4, M5): solid in every rule; only walks read the difference.
    pub passable: bool,
    /// Metres.
    pub sink: f64,
    pub max_relief: Option<f64>,
    pub colour: [f64; 3],
    /// Mounts: name to (`at` in mm, face in millidegrees).
    pub mounts: BTreeMap<String, ((i64, i64), i64)>,
    /// Each mount's `connector` (2.7, M5), where it writes one.
    pub connectors: BTreeMap<String, String>,
    /// Each mount's `rise` (2.7, R75), lattice millimetres, where it is not 0: how far the mount
    /// stands above the surface its instance stands on.
    pub rises: BTreeMap<String, i64>,
    pub taxon: Option<String>,
    pub tags: Vec<String>,
    /// `collision` (2.4): `none`, `box`, `hull` or `mesh`. Carried to the loader; the core reads it
    /// for nothing.
    pub collision: String,
    /// Where the model's own origin lies in the asset frame, millimetres (2.4); `[0, 0, 0]` for a
    /// primitive or engine-made asset.
    pub origin: [i64; 3],
    /// What a model asset holds besides (2.2, 3); `None` for a primitive or engine-made asset.
    pub model: Option<crate::asset::ModelFacts>,
    /// The resolved `params` (2.5: the nearest one taken whole), handed to the loader unread.
    pub params: Option<toml::Table>,
    /// The resolved `shared` (2.5, R66): what the assets filed at its taxon state once, from the
    /// nearest `defaults` that writes it. Never the asset's own; read by the core for nothing.
    pub shared: Option<toml::Table>,
}

impl Asset {
    /// The `solidity` word (2.4): `solid`, `passable` or `none`.
    pub fn solidity(&self) -> &'static str {
        match (self.solid, self.passable) {
            (_, true) => "passable",
            (true, false) => "solid",
            (false, false) => "none",
        }
    }

    /// The 3 by 4 matrix, row by row, taking a point of the model file's node space to the asset
    /// frame (`build.md` 7 `to_asset`): the quarter turn of `front` and `up`, then minus the
    /// pivot, in millimetres. Identity for a primitive or engine-made asset. The model's `scale`
    /// (2.4) multiplies the 3 by 3 part: [`Asset::model_scale`].
    pub fn to_asset(&self) -> [[i64; 4]; 3] {
        match &self.model {
            None => [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]],
            Some(m) => {
                let mut out = [[0; 4]; 3];
                for (i, row) in out.iter_mut().enumerate() {
                    row[..3].copy_from_slice(&m.rows[i]);
                    row[3] = -m.pivot_mm[i];
                }
                out
            }
        }
    }

    /// The model's declared `scale` (2.4), 1 for a primitive or engine-made asset.
    pub fn model_scale(&self) -> f64 {
        self.model.as_ref().map_or(1.0, |m| m.scale)
    }

    /// The facts the change report compares (7), each as canonical text.
    pub fn facts(&self) -> BTreeMap<String, String> {
        let mut f = BTreeMap::new();
        let mm = |v: i64| via_num::write_length(v);
        // `size` is the declared statement of a primitive or engine-made asset (7); a model's is
        // measured, and its `bounds` are the fact.
        if self.model.is_none() {
            f.insert("size".into(), format!("[{}]", self.size.map(mm).join(", ")));
        }
        f.insert(
            "bounds".into(),
            format!("[{}]", self.bounds.map(mm).join(", ")),
        );
        let fp: Vec<String> = self
            .footprint
            .iter()
            .map(|(x, y)| format!("[{}, {}]", mm(*x), mm(*y)))
            .collect();
        f.insert("footprint".into(), format!("[{}]", fp.join(", ")));
        if !self.plan.is_empty() {
            let rings: Vec<String> = self
                .plan
                .iter()
                .map(|r| {
                    let pts: Vec<String> = r
                        .iter()
                        .map(|(x, y)| format!("[{}, {}]", mm(*x), mm(*y)))
                        .collect();
                    format!("[{}]", pts.join(", "))
                })
                .collect();
            f.insert("plan".into(), format!("[{}]", rings.join(", ")));
        }
        f.insert(
            "origin".into(),
            format!("[{}]", self.origin.map(mm).join(", ")),
        );
        f.insert("sink".into(), crate::text::dbl(self.sink));
        // Each value is JSON text, in the form the manifest writes it (`build.md` 11.2): a
        // number, a list, `null`, a string, or for `mounts` the manifest's object.
        f.insert(
            "max_relief".into(),
            self.max_relief
                .map_or_else(|| "null".to_string(), crate::text::dbl),
        );
        f.insert("solidity".into(), format!("\"{}\"", self.solidity()));
        let mounts: Vec<String> = self
            .mounts
            .iter()
            .map(|(n, ((x, y), face))| {
                let c = self
                    .connectors
                    .get(n)
                    .map(|c| format!(",\"connector\":{}", serde_json_str(c)))
                    .unwrap_or_default();
                // `rise` only where it is not 0 (2.7), so a pack with none keeps its facts.
                let r = self
                    .rises
                    .get(n)
                    .map(|r| format!(",\"rise\":{}", mm(*r)))
                    .unwrap_or_default();
                format!(
                    "{}:{{\"at\":[{},{}]{c},\"face\":{face}{r}}}",
                    serde_json_str(n),
                    mm(*x),
                    mm(*y)
                )
            })
            .collect();
        f.insert("mounts".into(), format!("{{{}}}", mounts.join(",")));
        f
    }
}

/// A string as JSON text: quoted, with `"` and `\\` and control characters escaped.
fn serde_json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A material (2.6).
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub colour: [f64; 3],
}

/// A read pack.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pack {
    /// The pack as a record names it: `pack:<file name>`.
    pub display: String,
    pub assets: BTreeMap<String, Asset>,
    /// Every declared taxon, those above each declared one included.
    pub taxa: BTreeSet<String>,
    pub materials: BTreeMap<String, Material>,
    /// The world names a pack that could not be found or read. Its error is reported once; what
    /// reads the pack is skipped, not reported again (`diagnostics.md` 4).
    pub failed: bool,
    /// Where each asset's declared facts are written (7: a declared fact's file and line are the
    /// pack text's): asset id to key to 1-based line; the key `""` is the asset's header.
    pub lines: BTreeMap<String, BTreeMap<String, u32>>,
    /// Where each name the pack declares is declared, for a not-found record's `related`
    /// (`diagnostics.md` 1.3): `asset.<id>`, `material.<name>`, `taxon.<path>` (a taxon implied
    /// by a longer path is declared by the first such path) to its subject and header site.
    pub declared: BTreeMap<String, (Subject, Site)>,
    /// The pack file as found.
    pub path: PathBuf,
    /// Where the pack's files are (2.1 `root`).
    pub root: crate::asset::RootOf,
    /// The facts file (4) as read, its path, and its name as records write it.
    pub facts: crate::asset_facts::FactsFile,
    pub facts_path: PathBuf,
    pub facts_display: String,
    /// The pack path a hint names: as the command line gave it, or the pack file's path when it
    /// is relative, else its file name (a record names no absolute path).
    pub command: String,
    /// Faults of one asset each (8.3): its bound files, reading and measuring its model and its
    /// lower-detail copies. A build raises an asset's only when the world uses it; `via pack
    /// check` raises them all.
    pub asset_diags: BTreeMap<String, Vec<Diag>>,
    /// Built with `--stand-ins` (9): each model asset is drawn as a box of its accepted facts.
    pub stand_ins: bool,
    /// Assets an import's pack binds and this world's does not (`composition.md` 10), by id:
    /// what a build lists, counts against its budgets and hands a lint or export step. This
    /// world's own text never resolves them: placing and selecting read `assets` alone.
    pub brought: BTreeMap<String, Brought>,
}

/// An asset bound by an import's pack (`composition.md` 10).
#[derive(Clone, Debug, PartialEq)]
pub struct Brought {
    /// The scoped id of the import whose pack binds it (`a`, or `a.b` inside `a`).
    pub from: String,
    /// That pack's root (2.1), where the asset's files lie.
    pub root: PathBuf,
    pub asset: Asset,
}

impl Pack {
    /// The asset a build's instance names: this world's binding, else one an import's pack
    /// brought (`composition.md` 10).
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets
            .get(id)
            .or_else(|| self.brought.get(id).map(|b| &b.asset))
    }

    /// Takes in the assets of an import's pack that this pack does not bind, as brought from
    /// `from`. One binding per id in a build (`import.binding_differs`), so the first pack
    /// taken in for an id is the one.
    pub fn bring(&mut self, from: &str, other: &Pack) {
        if other.failed {
            return;
        }
        for (id, a) in &other.assets {
            if self.assets.contains_key(id) || self.brought.contains_key(id) {
                continue;
            }
            self.brought.insert(
                id.clone(),
                Brought {
                    from: from.to_string(),
                    root: other.root.root.clone(),
                    asset: a.clone(),
                },
            );
        }
    }

    /// The pack path a hint names.
    pub fn command_path(&self) -> &str {
        &self.command
    }

    /// What a build raises about the assets it uses (8.3, 4.2): each used asset's own faults,
    /// `pack.unaccepted` once, and the comparison with the facts file, naming every instance of a
    /// revised asset. `instances` is each instance's name and asset.
    pub fn used_diagnostics<'a>(
        &self,
        used: &BTreeSet<String>,
        instances: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Vec<Diag> {
        if self.failed {
            return Vec::new();
        }
        let mut out = Vec::new();
        for id in used {
            if let Some(d) = self.asset_diags.get(id) {
                out.extend(d.iter().cloned());
            }
        }
        let uses_model = used
            .iter()
            .any(|id| self.assets.get(id).is_some_and(|a| a.model.is_some()));
        if uses_model {
            out.extend(crate::asset_facts::unaccepted(self));
        }
        let mut by_asset: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (name, asset) in instances {
            by_asset
                .entry(asset.to_string())
                .or_default()
                .push(name.to_string());
        }
        for v in by_asset.values_mut() {
            v.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        }
        out.extend(crate::asset_facts::compare(
            self,
            used.iter().map(String::as_str),
            &by_asset,
        ));
        out
    }

    /// What `via pack check` raises beyond the pack-wide faults (8.1): every asset's own faults,
    /// `pack.unaccepted`, the comparison with the facts file (no instances: there is no world)
    /// and `pack.stale_facts`.
    pub fn check_diagnostics(&self) -> Vec<Diag> {
        let mut out: Vec<Diag> = self.asset_diags.values().flatten().cloned().collect();
        out.extend(crate::asset_facts::unaccepted(self));
        out.extend(crate::asset_facts::compare(
            self,
            self.assets.keys().map(String::as_str),
            &BTreeMap::new(),
        ));
        out.extend(crate::asset_facts::stale(self));
        out
    }

    /// The files a build reads through the pack besides the pack file (`graph.md` 1, 2.2 "Bound
    /// files", 4 "An input"): every bound file of every model asset, by its pack path, and the
    /// facts file, absent or not.
    /// A pack with no model asset binds no file and needs no facts file: it has no leaves.
    pub fn leaves(&self) -> Vec<(String, via_graph::Digest)> {
        let mut out: BTreeMap<String, via_graph::Digest> = BTreeMap::new();
        if !self.assets.values().any(|a| a.model.is_some()) {
            return Vec::new();
        }
        for a in self.assets.values() {
            let Some(m) = &a.model else { continue };
            for (rel, d) in &m.files {
                let digest = via_graph::Digest::from_hex(d)
                    .unwrap_or_else(|| via_graph::Digest::of(d.as_bytes()));
                out.insert(self.root.pack_path(rel), digest);
            }
        }
        let facts = match std::fs::read(&self.facts_path) {
            Ok(b) => via_graph::Digest::of(&b),
            Err(_) => via_graph::Digest::of(b"\0absent"),
        };
        out.insert(self.facts_display.clone(), facts);
        out.into_iter().collect()
    }

    /// The declaration of a pack name, `class` one of `asset`, `material`, `taxon`.
    pub fn declaration(&self, class: &str, name: &str) -> Option<(Subject, Site)> {
        self.declared.get(&format!("{class}.{name}")).cloned()
    }

    /// The assets filed at or under a taxon, by id (5).
    pub fn select(&self, taxon: &str) -> Vec<&Asset> {
        let below = format!("{taxon}/");
        self.assets
            .values()
            .filter(|a| {
                a.taxon
                    .as_deref()
                    .is_some_and(|t| t == taxon || t.starts_with(&below))
            })
            .collect()
    }

    /// The assets a taxonomy query selects (`protocol.md` 3.3 `pack.query`, 8.2's `--taxon`):
    /// `"*"` for every asset, an `@<taxon path>` for those at or below it, or asset ids; with
    /// `tag`, only those carrying it. Ordered by id, each once. Read-only (R19b).
    ///
    /// The fault is a code, a message, a hint and the nearest name: `pack.unknown_taxon` for an
    /// undeclared taxon, `asset.unknown` for an id the pack lacks, `asset.empty_selection` for a
    /// declared taxon or a tag that keeps no asset.
    pub fn query(
        &self,
        select: &[String],
        tag: Option<&str>,
    ) -> Result<Vec<&Asset>, (Code, String, String, Option<String>)> {
        let near = |w: &str, names: Vec<String>| {
            via_diag::nearest::nearest(w, names.iter().map(String::as_str))
                .found()
                .map(str::to_string)
        };
        let mut keep: BTreeSet<&str> = BTreeSet::new();
        for s in select {
            if s == "*" {
                keep.extend(self.assets.keys().map(String::as_str));
            } else if let Some(t) = s.strip_prefix('@') {
                if !self.taxa.contains(t) {
                    let n = near(t, self.taxa.iter().cloned().collect());
                    return Err((
                        codes::pack::UNKNOWN_TAXON,
                        format!("the taxon `@{t}` is not declared in the pack"),
                        n.as_ref().map_or_else(
                            || "declare `[taxon.\"<path>\"]` in the pack".to_string(),
                            |n| format!("write `@{n}`"),
                        ),
                        n,
                    ));
                }
                let found = self.select(t);
                if found.is_empty() {
                    let with: Vec<String> = self
                        .taxa
                        .iter()
                        .filter(|x| !self.select(x).is_empty())
                        .cloned()
                        .collect();
                    let n = near(t, with);
                    return Err((
                        codes::asset::EMPTY_SELECTION,
                        format!("`@{t}` keeps no asset of the pack"),
                        n.as_ref().map_or_else(
                            || "name a taxon that has assets".to_string(),
                            |n| format!("write `@{n}`"),
                        ),
                        n,
                    ));
                }
                keep.extend(found.iter().map(|a| a.id.as_str()));
            } else if self.assets.contains_key(s) {
                keep.insert(s.as_str());
            } else {
                let n = near(s, self.assets.keys().cloned().collect());
                return Err((
                    codes::asset::UNKNOWN,
                    format!("the pack has no asset `{s}`"),
                    n.as_ref().map_or_else(
                        || "name an asset of the pack".to_string(),
                        |n| format!("write `{n}`"),
                    ),
                    n,
                ));
            }
        }
        let mut out: Vec<&Asset> = keep
            .into_iter()
            .filter_map(|id| self.assets.get(id))
            .collect();
        if let Some(g) = tag {
            out.retain(|a| a.tags.iter().any(|x| x == g));
            if out.is_empty() {
                let mut with: Vec<String> =
                    self.assets.values().flat_map(|a| a.tags.clone()).collect();
                with.sort();
                with.dedup();
                let n = near(g, with);
                return Err((
                    codes::asset::EMPTY_SELECTION,
                    format!("the tag `{g}` keeps no asset of the selection"),
                    n.as_ref().map_or_else(
                        || "name a tag that assets carry".to_string(),
                        |n| format!("write `{n}`"),
                    ),
                    n,
                ));
            }
        }
        out.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
        Ok(out)
    }
}

/// Where packs are looked for (`documents.md` 4): beside the world, each `--lib`, each
/// `VIA_LIB` entry, the shipped root.
#[derive(Clone, Debug, Default)]
pub struct Roots {
    pub world_dir: PathBuf,
    pub lib_roots: Vec<PathBuf>,
    pub via_lib: Vec<PathBuf>,
    pub shipped: Option<PathBuf>,
}

impl Roots {
    fn named(&self) -> Vec<(String, PathBuf)> {
        let mut v = vec![("beside".to_string(), self.world_dir.join("lib"))];
        for (i, r) in self.lib_roots.iter().enumerate() {
            v.push((format!("--lib {}", i + 1), r.clone()));
        }
        for (i, r) in self.via_lib.iter().enumerate() {
            v.push((format!("VIA_LIB {}", i + 1), r.clone()));
        }
        if let Some(s) = &self.shipped {
            v.push(("shipped".into(), s.clone()));
        }
        v
    }
}

fn pack_diag(code: Code, message: &str, hint: &str) -> Diag {
    text::diag(
        code,
        Subject::World,
        Site::Key(text::path(&["pack"])),
        message,
        hint,
    )
}

/// Finds the pack a world's `pack` string names (1.2). `None` with diagnostics when not found.
pub fn find(written: &str, roots: &Roots, diags: &mut Vec<Diag>) -> Option<PathBuf> {
    if written.contains('/') || written.ends_with(".toml") {
        let p = roots.world_dir.join(written);
        if p.is_file() {
            return Some(p);
        }
        diags.push(pack_diag(
            codes::pack::NOT_FOUND,
            &format!("the pack `{written}` is not a file beside the world"),
            "write the path relative to the world file",
        ));
        return None;
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let mut seen_names: BTreeSet<String> = BTreeSet::new();
    for (place, root) in roots.named() {
        let dir = root.join("packs");
        let p = dir.join(format!("{written}.toml"));
        if p.is_file() {
            found.push((place, p));
        }
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if let Some(stem) = n.strip_suffix(".toml") {
                        seen_names.insert(stem.to_string());
                    }
                }
            }
        }
    }
    match found.len() {
        0 => {
            let places: Vec<String> = roots.named().into_iter().map(|(p, _)| p).collect();
            diags.push(text::with_nearest(
                pack_diag(
                    codes::pack::NOT_FOUND,
                    &format!(
                        "no pack `{written}` in the roots searched: {}",
                        places.join(", ")
                    ),
                    &text::near_hint(
                        written,
                        seen_names.iter().map(String::as_str),
                        "write a path to the pack file",
                    ),
                ),
                written,
                seen_names.iter().map(String::as_str),
                text::undeclared,
            ));
            None
        }
        n => {
            if n > 1 {
                let places: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
                diags.push(pack_diag(
                    codes::pack::SHADOWED,
                    &format!(
                        "the pack `{written}` is in {n} roots ({}); the one {} is used",
                        places.join(", "),
                        found[0].0
                    ),
                    "write a path to choose another",
                ));
            }
            Some(found.swap_remove(0))
        }
        .map(|(_, p)| p),
    }
}

/// The byte span a pack fault is located at (`diagnostics.md` 1.2: a pack is text, so a span):
/// the key named `key` first in source order under the subject's table, at any depth (a mount's
/// keys sit in `[asset.<id>.mount.<name>]`); without one, the subject's own key, which is the
/// header that creates it.
fn span_in(
    root: &toml::de::DeTable<'_>,
    subject: &str,
    key: Option<&str>,
) -> Option<std::ops::Range<usize>> {
    let mut table = root;
    let mut at: Option<std::ops::Range<usize>> = None;
    for part in subject.split('.') {
        let Some((k, v)) = table.iter().find(|(k, _)| k.get_ref().as_ref() == part) else {
            break;
        };
        at = Some(k.span());
        match v.get_ref() {
            toml::de::DeValue::Table(t) => table = t,
            _ => break,
        }
    }
    if let Some(key) = key {
        if let Some(s) = first_key(table, key) {
            return Some(s);
        }
    }
    at
}

/// The span of the key named `key` that comes first in the text, at any depth under `table`.
fn first_key(table: &toml::de::DeTable<'_>, key: &str) -> Option<std::ops::Range<usize>> {
    let mut best: Option<std::ops::Range<usize>> = None;
    for (k, v) in table.iter() {
        let here = if k.get_ref().as_ref() == key {
            Some(k.span())
        } else if let toml::de::DeValue::Table(t) = v.get_ref() {
            first_key(t, key)
        } else {
            None
        };
        if let Some(s) = here {
            if best.as_ref().is_none_or(|b| s.start < b.start) {
                best = Some(s);
            }
        }
    }
    best
}

/// 1-based line and column (in Unicode scalar values) of a byte offset.
fn line_column(text: &str, at: usize) -> (u32, u32) {
    let at = at.min(text.len());
    let before = text.get(..at).unwrap_or("");
    let line = before.bytes().filter(|b| *b == b'\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = before.get(start..).map_or(0, |s| s.chars().count()) + 1;
    let clamp = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    (clamp(line), clamp(column))
}

struct Reader<'a> {
    file: String,
    text: &'a str,
    /// The pack with its spans; `None` only when it is not TOML, which is reported first.
    spans: Option<toml::Spanned<toml::de::DeTable<'a>>>,
    diags: Vec<Diag>,
}

impl Reader<'_> {
    fn fault(&mut self, code: Code, subject: &str, key: Option<&str>, message: &str, hint: &str) {
        let subj = Subject::parse(&match key {
            Some(k) => format!("{subject}.{k}"),
            None => subject.to_string(),
        })
        .unwrap_or(Subject::World);
        let span = self
            .spans
            .as_ref()
            .and_then(|t| span_in(t.get_ref(), subject, key))
            .unwrap_or(0..0);
        let site = self.text_site(&span);
        self.diags.push(text::diag(code, subj, site, message, hint));
    }

    /// The last fault with the nearest of `names` to `written` (`diagnostics.md` 1.3).
    fn near<'n>(
        &mut self,
        written: &str,
        names: impl IntoIterator<Item = &'n str>,
        declared: impl FnOnce(&str) -> Option<(Subject, Site)>,
    ) {
        if let Some(d) = self.diags.pop() {
            self.diags
                .push(text::with_nearest(d, written, names, declared));
        }
    }

    /// The pack text at a byte span, as a site.
    fn text_site(&self, span: &std::ops::Range<usize>) -> Site {
        let (line, column) = line_column(self.text, span.start);
        let clamp = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Site::Text {
            file: self.file.clone(),
            line,
            column,
            bytes: [clamp(span.start), clamp(span.end)],
        }
    }

    /// The header of `[<class>.<name>]`, as subject and site, when the text holds it.
    fn declared(&self, class: &str, name: &str) -> Option<(Subject, Site)> {
        let root = self.spans.as_ref()?.get_ref();
        let span = root
            .iter()
            .find(|(k, _)| k.get_ref().as_ref() == class)
            .and_then(|(_, v)| match v.get_ref() {
                toml::de::DeValue::Table(t) => t
                    .iter()
                    .find(|(k, _)| k.get_ref().as_ref() == name)
                    .map(|(k, _)| k.span()),
                _ => None,
            })?;
        let subject = Subject::parse(&if via_doc::is_id(name) {
            format!("{class}.{name}")
        } else {
            format!("{class}.\"{name}\"")
        })
        .ok()?;
        Some((subject, self.text_site(&span)))
    }
}

/// A TOML number back to its decimal text, for `via-num`'s exact readers.
fn num_text(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(format!("{f}")),
        _ => None,
    }
}

fn lengths(v: &toml::Value) -> Option<Vec<i64>> {
    let toml::Value::Array(xs) = v else {
        return None;
    };
    xs.iter()
        .map(|x| num_text(x).and_then(|t| via_num::parse_length(&t).ok()))
        .collect()
}

/// The TOML form of a value, as a message names it.
fn form_word(v: &toml::Value) -> String {
    match v {
        toml::Value::String(_) => "a string".into(),
        toml::Value::Integer(_) | toml::Value::Float(_) => "a number".into(),
        toml::Value::Boolean(_) => "a boolean".into(),
        toml::Value::Datetime(_) => "a date".into(),
        toml::Value::Array(xs) => format!(
            "an array of {} value{}",
            xs.len(),
            if xs.len() == 1 { "" } else { "s" }
        ),
        toml::Value::Table(_) => "a table".into(),
    }
}

/// `size`: three lengths, or the code, message and hint of what is wrong with it.
fn size_of(v: &toml::Value) -> Result<[i64; 3], (Code, String, String)> {
    let wrong = |found: String| {
        (
            codes::doc::TYPE,
            format!("`size` takes an array of three lengths in metres; found {found}"),
            "write `size = [x, y, z]` in metres".to_string(),
        )
    };
    let toml::Value::Array(xs) = v else {
        return Err(wrong(form_word(v)));
    };
    if xs.len() != 3 {
        return Err(wrong(form_word(v)));
    }
    let mut out = [0i64; 3];
    for (i, x) in xs.iter().enumerate() {
        let Some(t) = num_text(x) else {
            return Err(wrong(format!("{} at index {i}", form_word(x))));
        };
        match via_num::parse_length(&t) {
            Ok(mm) => out[i] = mm,
            Err(e) => {
                let hint = match &e {
                    via_num::NumError::TooPrecise { nearest, .. } => {
                        format!("write `{}` or `{}`", nearest[0], nearest[1])
                    }
                    _ => "write a length in range".to_string(),
                };
                return Err((text::num_code(&e), format!("`size` index {i}: {e}"), hint));
            }
        }
    }
    Ok(out)
}

fn doubles(v: &toml::Value) -> Option<Vec<f64>> {
    let toml::Value::Array(xs) = v else {
        return None;
    };
    xs.iter()
        .map(|x| num_text(x).and_then(|t| via_num::parse_double(&t).ok()))
        .collect()
}

fn double(v: &toml::Value) -> Option<f64> {
    num_text(v).and_then(|t| via_num::parse_double(&t).ok())
}

/// The 16-sided ring of a round primitive (6), counter-clockwise from `+x`.
fn ring16(bounds: &[i64; 6]) -> Vec<(i64, i64)> {
    let cx = ((bounds[0] + bounds[3]) as f64) / 2.0;
    let cy = ((bounds[1] + bounds[4]) as f64) / 2.0;
    let rx = ((bounds[3] - bounds[0]) as f64) / 2.0;
    let ry = ((bounds[4] - bounds[1]) as f64) / 2.0;
    (0..16)
        .map(|k| {
            let (s, c) = via_num::facing_sin_cos(22_500 * k);
            let x = via_num::round_to_lattice(cx + rx * c).unwrap_or(0);
            let y = via_num::round_to_lattice(cy + ry * s).unwrap_or(0);
            (x, y)
        })
        .collect()
}

fn is_ccw_simple(ring: &[(i64, i64)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut seen = BTreeSet::new();
    for p in ring {
        if !seen.insert(*p) {
            return false;
        }
    }
    let mut a2: i128 = 0;
    for k in 0..ring.len() {
        let (x0, y0) = ring[k];
        let (x1, y1) = ring[(k + 1) % ring.len()];
        a2 += i128::from(x0) * i128::from(y1) - i128::from(x1) * i128::from(y0);
    }
    a2 > 0
}

/// Reads a pack file. Pack-wide faults (8.3) come back as diagnostics.
pub fn read(path: &Path, display: &str) -> (Option<Pack>, Vec<Diag>) {
    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return (
                None,
                vec![pack_diag(
                    codes::pack::NOT_FOUND,
                    &format!("the pack `{display}` cannot be read: {e}"),
                    "check the file",
                )],
            )
        }
    };
    // Syntax under `documents.md` 1's rules, through the documents' own reader: the same forms,
    // messages and hints as a world file, each located by line, column and bytes.
    let syntax = via_doc::read::syntax_faults(raw.clone(), display);
    if !syntax.is_empty() {
        let shown = String::from_utf8_lossy(&raw);
        let diags = syntax
            .into_iter()
            .map(|f| {
                let start = f.bytes[0] as usize;
                let (line, column) = line_column(&shown, start);
                let mut d = text::diag(
                    codes::doc::SYNTAX,
                    Subject::World,
                    Site::Text {
                        file: display.to_string(),
                        line,
                        column,
                        bytes: f.bytes,
                    },
                    &f.message,
                    &f.hint,
                );
                for (k, v) in f.data {
                    d = d.with(k, v);
                }
                d
            })
            .collect();
        return (None, diags);
    }
    let Ok(bytes) = String::from_utf8(raw) else {
        // `syntax_faults` reports bytes that are not UTF-8; this is not reached.
        return (None, Vec::new());
    };
    let mut r = Reader {
        file: display.to_string(),
        text: &bytes,
        spans: toml::de::DeTable::parse(&bytes).ok(),
        diags: Vec::new(),
    };
    let table: toml::Table = match bytes.parse() {
        Ok(t) => t,
        Err(e) => {
            // Our reader took the text and the TOML crate did not: located by the crate's span,
            // with its one-line message.
            let span = e.span().unwrap_or(0..0);
            let site = r.text_site(&span);
            let first = e.message().lines().next().unwrap_or("").trim().to_string();
            r.diags.push(text::diag(
                codes::doc::SYNTAX,
                Subject::World,
                site,
                &format!("the pack is not TOML here: {first}"),
                "write the value or header in TOML 1.0's form",
            ));
            return (None, r.diags);
        }
    };
    let facts_path = crate::asset_facts::path_of(path);
    let mut pack = Pack {
        display: display.to_string(),
        path: path.to_path_buf(),
        facts_display: format!(
            "pack:{}",
            facts_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("facts.toml")
        ),
        facts_path,
        command: if path.is_relative() {
            path.to_string_lossy().replace('\\', "/")
        } else {
            path.file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
        },
        ..Pack::default()
    };
    // 2.1: `root`, relative to the pack file; the pack file's folder by default.
    // A pack named with no folder (`pack.toml`) is in the working folder, `.`.
    let here = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let root = match table.get("root") {
        None => Some(here),
        Some(toml::Value::String(s)) => {
            let dir = here.join(s);
            if !dir.is_dir() {
                r.fault(
                    codes::pack::MISSING_FILE,
                    "pack",
                    Some("root"),
                    &format!("the pack's `root` `{s}` is not a folder"),
                    "write `root` relative to the pack file, or create the folder",
                );
            }
            pack.root.written = s
                .split('/')
                .filter(|p| !p.is_empty() && *p != ".")
                .map(str::to_string)
                .collect();
            Some(dir)
        }
        Some(_) => None,
    };
    if let Some(dir) = &root {
        pack.root.root = dir.clone();
    }
    // 4: the facts file, a pack-wide input (8.3: a malformed one fails every build).
    pack.facts = match std::fs::read(&pack.facts_path) {
        Err(_) => crate::asset_facts::FactsFile::Absent,
        Ok(bytes) => {
            let parsed = String::from_utf8(bytes)
                .map_err(|_| ("the facts file is not UTF-8".to_string(), 1))
                .and_then(|t| crate::asset_facts::parse(&t));
            match parsed {
                Ok(m) => crate::asset_facts::FactsFile::Present(m),
                Err((why, line)) => {
                    r.diags.push(text::diag(
                        codes::pack::BAD_FACTS,
                        Subject::parse("pack").unwrap_or(Subject::World),
                        Site::Text {
                            file: pack.facts_display.clone(),
                            line,
                            column: 1,
                            bytes: [0, 0],
                        },
                        &format!("{} is not in its form: {why}", pack.facts_display),
                        &format!("delete it and run `via pack accept {}`", pack.command),
                    ));
                    crate::asset_facts::FactsFile::Bad
                }
            }
        }
    };
    // 2.5: `[defaults]` and each taxon's `defaults`.
    let mut taxon_defaults: BTreeMap<String, toml::Table> = BTreeMap::new();
    let pack_defaults = match table.get("defaults") {
        Some(toml::Value::Table(d)) => {
            check_defaults(&mut r, "defaults", d);
            d.clone()
        }
        _ => toml::Table::new(),
    };
    if let Some(toml::Value::Table(t)) = table.get("taxon") {
        for (path, v) in t {
            if let Some(toml::Value::Table(d)) = v.get("defaults") {
                check_defaults(&mut r, &format!("taxon.\"{path}\".defaults"), d);
                taxon_defaults.insert(path.clone(), d.clone());
            }
            // 5: a taxon's keys are `defaults`, or none. A fact written beside `defaults` (K6's
            // `shared`) reached no asset and was said by nothing; nor did a taxon, or its
            // `defaults`, written as no table (F1's review).
            let Some(keys) = v.as_table() else {
                r.fault(
                    codes::doc::TYPE,
                    &format!("taxon.\"{path}\""),
                    None,
                    &format!("a taxon is a table; found {}", form_word(v)),
                    &format!("write `[taxon.\"{path}\"]`, with `defaults` in it or nothing"),
                );
                continue;
            };
            if let Some(d) = keys.get("defaults").filter(|d| !d.is_table()) {
                r.fault(
                    codes::doc::TYPE,
                    &format!("taxon.\"{path}\""),
                    Some("defaults"),
                    &format!("`defaults` takes a table; found {}", form_word(d)),
                    "write `defaults = { ... }`",
                );
            }
            {
                for k in keys.keys().filter(|k| k.as_str() != "defaults") {
                    let hint = if DEFAULT_KEYS.contains(&k.as_str()) {
                        format!("write it in the taxon's `defaults` (`defaults = {{ {k} = ... }}`, or under `[taxon.\"{path}\".defaults]`)")
                    } else if keys.get(k).is_some_and(toml::Value::is_table) {
                        format!("a taxon's path is one quoted key: `[taxon.\"{path}/{k}\"]`, not `[taxon.{path}.{k}]`")
                    } else {
                        "a taxon holds `defaults`, or nothing".to_string()
                    };
                    r.fault(
                        codes::doc::UNKNOWN_KEY,
                        &format!("taxon.\"{path}\""),
                        Some(k),
                        &format!("`{k}` is not a key of a taxon, so no asset takes it"),
                        &hint,
                    );
                }
            }
        }
    }
    for (k, v) in &table {
        match (k.as_str(), v) {
            ("taxon", toml::Value::Table(t)) => {
                for path in t.keys() {
                    let segs: Vec<&str> = path.split('/').collect();
                    if segs.iter().any(|s| !via_doc::is_id(s)) {
                        r.fault(
                            codes::identity::BAD_ID,
                            &format!("taxon.\"{path}\""),
                            None,
                            &format!("the taxon path `{path}` has a segment that is not an id"),
                            "write ids joined by `/`",
                        );
                        continue;
                    }
                    for n in 1..=segs.len() {
                        pack.taxa.insert(segs[..n].join("/"));
                    }
                }
            }
            ("material", toml::Value::Table(t)) => {
                for (name, m) in t {
                    let colour = match m.get("colour").and_then(doubles) {
                        Some(c) if c.len() == 3 => [c[0], c[1], c[2]],
                        _ => [0.8, 0.8, 0.8],
                    };
                    // 2.6: a material's `file` is checked from M1, relative to `root`. A missing
                    // `root` is reported once, below, and stands for its files.
                    if let (Some(f), Some(dir)) = (m.get("file").and_then(|v| v.as_str()), &root) {
                        if dir.is_dir() && !dir.join(f).is_file() {
                            r.fault(
                                codes::pack::MISSING_FILE,
                                &format!("material.{name}"),
                                Some("file"),
                                &format!("the material's file `{f}` does not exist under `root`"),
                                "write the path relative to the pack's `root`",
                            );
                        }
                    }
                    let has_file = m.get("file").is_some();
                    let has_repeat = m.get("repeat").is_some();
                    if has_file != has_repeat {
                        r.fault(
                            codes::pack::MATERIAL_REPEAT,
                            &format!("material.{name}"),
                            None,
                            "a material has `file` without `repeat`, or `repeat` without `file`",
                            "write both, or neither",
                        );
                    }
                    pack.materials.insert(name.clone(), Material { colour });
                }
            }
            ("asset" | "root" | "defaults", _) => {}
            (other, _) => r.fault(
                codes::doc::UNKNOWN_KEY,
                "pack",
                None,
                &format!("`{other}` is not a key of a pack"),
                "a pack holds `root`, `[defaults]`, `[taxon]`, `[asset]` and `[material]`",
            ),
        }
    }
    if let Some(toml::Value::Table(assets)) = table.get("asset") {
        for (id, a) in assets {
            let toml::Value::Table(a) = a else { continue };
            let inherited = Inherited {
                pack: &pack_defaults,
                taxa: &taxon_defaults,
            };
            let files = root.as_ref().filter(|d| d.is_dir()).map(|_| &pack.root);
            if let Some((asset, own)) = read_asset(&mut r, id, a, &pack.taxa, &inherited, files) {
                if !own.is_empty() {
                    pack.asset_diags.insert(id.clone(), own);
                }
                pack.assets.insert(id.clone(), asset);
            }
        }
    }
    if let Some(spans) = r.spans.as_ref() {
        let root = spans.get_ref();
        for id in pack.assets.keys() {
            let subject = format!("asset.{id}");
            let mut m = BTreeMap::new();
            if let Some(h) = span_in(root, &subject, None) {
                m.insert(String::new(), line_column(r.text, h.start).0);
            }
            let table = root
                .iter()
                .find(|(k, _)| k.get_ref().as_ref() == "asset")
                .and_then(|(_, v)| match v.get_ref() {
                    toml::de::DeValue::Table(t) => t
                        .iter()
                        .find(|(k, _)| k.get_ref().as_ref() == id.as_str())
                        .and_then(|(_, v)| match v.get_ref() {
                            toml::de::DeValue::Table(t) => Some(t),
                            _ => None,
                        }),
                    _ => None,
                });
            if let Some(t) = table {
                for (k, _) in t.iter() {
                    m.insert(
                        k.get_ref().to_string(),
                        line_column(r.text, k.span().start).0,
                    );
                }
            }
            pack.lines.insert(id.clone(), m);
        }
    }
    let mut declared = BTreeMap::new();
    for id in pack.assets.keys() {
        if let Some(d) = r.declared("asset", id) {
            declared.insert(format!("asset.{id}"), d);
        }
    }
    for name in pack.materials.keys() {
        if let Some(d) = r.declared("material", name) {
            declared.insert(format!("material.{name}"), d);
        }
    }
    if let Some(toml::Value::Table(t)) = table.get("taxon") {
        for path in t.keys() {
            let Some(d) = r.declared("taxon", path) else {
                continue;
            };
            let segs: Vec<&str> = path.split('/').collect();
            for n in (1..=segs.len()).rev() {
                let at = format!("taxon.{}", segs[..n].join("/"));
                if n == segs.len() {
                    declared.insert(at, d.clone());
                } else {
                    declared.entry(at).or_insert_with(|| d.clone());
                }
            }
        }
    }
    pack.declared = declared;
    (Some(pack), r.diags)
}

/// The keys `defaults` may hold (2.5).
const DEFAULT_KEYS: [&str; 12] = [
    "collision",
    "colour",
    "front",
    "max_relief",
    "params",
    "pivot",
    "plan",
    "scale",
    "shared",
    "sink",
    "solidity",
    "up",
];

/// The six axis words of 2.3.
const AXES: [&str; 6] = ["+x", "-x", "+y", "-y", "+z", "-z"];

/// Checks a `defaults` table (2.5): its keys, and its axes (2.3, a pack-wide check, 8.3).
fn check_defaults(r: &mut Reader<'_>, subject: &str, d: &toml::Table) {
    for k in d.keys() {
        if !DEFAULT_KEYS.contains(&k.as_str()) {
            r.fault(
                codes::doc::UNKNOWN_KEY,
                subject,
                Some(k),
                &format!("`{k}` is not a key `defaults` may hold"),
                &text::near_hint(k, DEFAULT_KEYS, "see assets.md 2.5"),
            );
            r.near(k, DEFAULT_KEYS, text::undeclared);
        }
    }
    // `params` and `shared` are tables (2.5): another value there was read as none, and hid the
    // table of the taxon above without a word.
    for k in ["params", "shared"] {
        if let Some(v) = d.get(k).filter(|v| !v.is_table()) {
            r.fault(
                codes::doc::TYPE,
                subject,
                Some(k),
                &format!("`{k}` takes a table; found {}", form_word(v)),
                &format!("write `{k} = {{ ... }}`"),
            );
        }
    }
    check_axes(r, subject, d);
}

/// `pack.bad_axes` (2.3): `front` or `up` outside the six words, or both on one axis.
fn check_axes(r: &mut Reader<'_>, subject: &str, t: &toml::Table) {
    let word = |k: &str| t.get(k).and_then(|v| v.as_str());
    for k in ["front", "up"] {
        if let Some(w) = word(k) {
            if !AXES.contains(&w) {
                r.fault(
                    codes::pack::BAD_AXES,
                    subject,
                    Some(k),
                    &format!("`{w}` is not an axis"),
                    &text::near_hint(w, AXES, "write one of `+x`, `-x`, `+y`, `-y`, `+z`, `-z`"),
                );
                return;
            }
        }
    }
    let front = word("front").unwrap_or("+z");
    let up = word("up").unwrap_or("+y");
    if front.get(1..) == up.get(1..) {
        r.fault(
            codes::pack::BAD_AXES,
            subject,
            Some(if t.contains_key("up") { "up" } else { "front" }),
            &format!("`front = \"{front}\"` and `up = \"{up}\"` lie on one axis"),
            "name two different axes: the mapping is a rotation",
        );
    }
}

/// Where an asset's declared facts come from besides its own table (2.5).
struct Inherited<'a> {
    pack: &'a toml::Table,
    taxa: &'a BTreeMap<String, toml::Table>,
}

impl Inherited<'_> {
    /// The value of `key` for an asset filed under `taxon`: its taxon's defaults, then each
    /// taxon above it, nearest first, then the pack's. The asset's own table is read first by
    /// the caller.
    fn get(&self, taxon: Option<&str>, key: &str) -> Option<&toml::Value> {
        let mut t = taxon;
        while let Some(path) = t {
            if let Some(v) = self.taxa.get(path).and_then(|d| d.get(key)) {
                return Some(v);
            }
            t = path.rsplit_once('/').map(|(up, _)| up);
        }
        self.pack.get(key)
    }
}

fn read_asset(
    r: &mut Reader<'_>,
    id: &str,
    a: &toml::Table,
    taxa: &BTreeSet<String>,
    inherited: &Inherited<'_>,
    files: Option<&crate::asset::RootOf>,
) -> Option<(Asset, Vec<Diag>)> {
    let subject = format!("asset.{id}");
    if !via_doc::is_id(id) {
        r.fault(
            codes::identity::BAD_ID,
            "pack",
            None,
            &format!("the asset id `{id}` is not an id"),
            &format!("write `{}`", via_doc::repair_id(id)),
        );
        return None;
    }
    for k in a.keys() {
        if k == "shared" {
            // What assets filed together state once is never one asset's (2.5, R66).
            r.fault(
                codes::doc::UNKNOWN_KEY,
                &subject,
                Some(k),
                "`shared` is not a key of an asset: it is what the assets of a taxon state once",
                "write it in the taxon's table: `defaults = { shared = { ... } }` (assets.md 2.5)",
            );
        } else if !ASSET_KEYS.contains(&k.as_str()) {
            r.fault(
                codes::doc::UNKNOWN_KEY,
                &subject,
                Some(k),
                &format!("`{k}` is not a key of an asset"),
                &text::near_hint(k, ASSET_KEYS, "see assets.md 2.2"),
            );
            r.near(k, ASSET_KEYS, text::undeclared);
        }
    }
    let primitive = a
        .get("primitive")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let engine = a
        .get("engine")
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    let file = match a.get("file") {
        None => None,
        Some(toml::Value::String(f)) => Some(f.clone()),
        Some(v) => {
            r.fault(
                codes::doc::TYPE,
                &subject,
                Some("file"),
                &format!("`file` takes a string; found {}", form_word(v)),
                "write `file = \"<path under root>\"`",
            );
            return None;
        }
    };
    let markers =
        usize::from(file.is_some()) + usize::from(primitive.is_some()) + usize::from(engine);
    if markers != 1 {
        r.fault(
            codes::asset::SOURCE,
            &subject,
            None,
            "an asset needs exactly one source: `file`, `primitive` or `engine = true`",
            "write exactly one of `file`, `primitive`, `engine = true`",
        );
        return None;
    }
    // 2.2: `detail` is a model's key only.
    if file.is_none() && a.contains_key("detail") {
        r.fault(
            codes::doc::UNKNOWN_KEY,
            &subject,
            Some("detail"),
            "`detail` is a key of a model asset only",
            "remove `detail`, or give the asset a `file`",
        );
        return None;
    }
    // 2.2: a fact the source measures or fixes; only the asset's own table can write one
    // (defaults skip it). A model measures `size` and `triangles`; a primitive fixes `front`, `up`,
    // `pivot` and `triangles`; an engine-made asset fixes `front`, `up` and `pivot`.
    let fixed: &[&str] = if file.is_some() {
        &["size", "triangles"]
    } else if engine {
        &["front", "up", "pivot"]
    } else {
        &["front", "up", "pivot", "triangles"]
    };
    for k in fixed {
        if a.contains_key(*k) {
            let what = if file.is_some() {
                "measured from a model asset's file"
            } else if engine {
                "fixed on an engine-made asset"
            } else {
                "fixed on a primitive asset"
            };
            r.fault(
                codes::asset::DECLARED_MEASURED,
                &subject,
                Some(k),
                &format!("`{k}` is {what}"),
                &if file.is_some() {
                    format!("remove `{k}`; `via pack list` shows the measured value")
                } else {
                    format!("remove `{k}`")
                },
            );
        }
    }
    let taxon_of = a.get("taxon").and_then(|v| v.as_str());
    // A declared fact: the asset's own value, else an inherited one (2.5).
    let declared = |k: &str| a.get(k).or_else(|| inherited.get(taxon_of, k));
    let collision = enum_fact(
        r,
        &subject,
        "collision",
        declared("collision"),
        &crate::asset::COLLISIONS,
        "none",
    );
    let header = r.declared("asset", id).unwrap_or_else(|| {
        (
            Subject::parse(&subject).unwrap_or(Subject::World),
            Site::World,
        )
    });
    // Faults of this asset alone (8.3): kept apart by the caller.
    let mut own: Vec<Diag> = Vec::new();
    let mut model: Option<crate::asset::ModelFacts> = None;
    let mut origin = [0i64; 3];
    let mut measured = false;
    // 2.4, 3.4: `plan` is true or false on any asset; only a model measures one.
    if let Some(v) = declared("plan").filter(|v| !v.is_bool()) {
        r.fault(
            codes::doc::TYPE,
            &subject,
            Some("plan"),
            &format!("`plan` takes true or false; found {}", form_word(v)),
            "write `plan = true` to have the model's plan measured (assets.md 3.4)",
        );
    }
    // 2.4: `scale` is a model's (its units to metres); a primitive's `size` is already metres.
    if file.is_none() && a.get("scale").is_some() {
        r.fault(
            codes::doc::TYPE,
            &subject,
            Some("scale"),
            "`scale` is a model's: a primitive or engine-made asset is made at its `size`",
            "write the size in metres, and take `scale` out",
        );
    }
    let mut plan: Vec<Vec<(i64, i64)>> = Vec::new();
    let (size, triangles, bounds) = if let Some(file) = &file {
        let detail: Vec<String> = match a.get("detail") {
            None => Vec::new(),
            Some(toml::Value::Array(xs)) if xs.iter().all(toml::Value::is_str) => xs
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            Some(v) => {
                r.fault(
                    codes::doc::TYPE,
                    &subject,
                    Some("detail"),
                    &format!("`detail` takes a list of strings; found {}", form_word(v)),
                    "write `detail = [\"<path under root>\", ...]`, finest first",
                );
                return None;
            }
        };
        // 2.2: naming `file` again, or one path twice.
        let mut seen: BTreeSet<&str> = BTreeSet::from([file.as_str()]);
        let mut kept: Vec<String> = Vec::new();
        for d in &detail {
            if seen.insert(d.as_str()) {
                kept.push(d.clone());
            } else {
                r.fault(
                    codes::asset::DETAIL_DUPLICATE,
                    &subject,
                    Some("detail"),
                    &format!("`detail` names `{d}` again, or names `file`"),
                    "remove the repeat",
                );
            }
        }
        let word = |k: &str, default: &'static str| -> String {
            declared(k)
                .and_then(|v| v.as_str())
                .unwrap_or(default)
                .to_string()
        };
        let (front, up) = (word("front", "+z"), word("up", "+y"));
        // 2.3: the resolved pair; a word inherited and wrong was reported where it is written.
        let words_ok = AXES.contains(&front.as_str()) && AXES.contains(&up.as_str());
        if a.contains_key("front") || a.contains_key("up") || words_ok {
            let mut t = toml::Table::new();
            t.insert("front".into(), toml::Value::String(front.clone()));
            t.insert("up".into(), toml::Value::String(up.clone()));
            check_axes(r, &subject, &t);
        }
        let pivot = enum_fact(
            r,
            &subject,
            "pivot",
            declared("pivot"),
            &crate::asset::PIVOTS,
            "base_centre",
        );
        // 3.4: the plan only where the asset asks for it (checked for every asset, above).
        // 2.3, 2.4: `scale`, the model's units to metres, a number above 0 (1 by default).
        let scale = match declared("scale") {
            None => 1.0,
            Some(v) => match v.as_float().or_else(|| v.as_integer().map(|i| i as f64)) {
                // 2.4: from 1/1000 to 1000 (ours): past it a model's points leave the lattice, or
                // its bounds round to nothing, and the fault would be the model's (F8's review).
                Some(x) if x.is_finite() && x > 0.0 && !(0.001..=1000.0).contains(&x) => {
                    r.fault(
                        codes::pack::RANGE,
                        &subject,
                        Some("scale"),
                        &format!("`scale = {v}` is outside 0.001 to 1000"),
                        "write the factor from the model's units to metres (a model in centimetres: 0.01)",
                    );
                    1.0
                }
                Some(x) if x.is_finite() && x > 0.0 => x,
                _ => {
                    r.fault(
                        codes::doc::TYPE,
                        &subject,
                        Some("scale"),
                        &format!("`scale` takes a number above 0; found {v}"),
                        "write the factor from the model's units to metres, e.g. `scale = 4`",
                    );
                    1.0
                }
            },
        };
        let want_plan = declared("plan")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
            || matches!(a.get("footprint"), Some(toml::Value::String(w)) if w == "plan");
        let m = match files {
            Some(files) => {
                let reader: &Reader<'_> = r;
                let site_of = |k: &str| -> Site {
                    let span = reader
                        .spans
                        .as_ref()
                        .and_then(|t| span_in(t.get_ref(), &subject, Some(k)))
                        .unwrap_or(0..0);
                    reader.text_site(&span)
                };
                let mut f = crate::asset::Faults {
                    subject: &subject,
                    diags: Vec::new(),
                    site_of: &site_of,
                };
                let m = crate::asset::measure(
                    files, file, &kept, &front, &up, &pivot, want_plan, scale, &mut f,
                );
                own.extend(f.diags);
                m
            }
            // A missing `root` is reported once, pack-wide; its models are not measured.
            None => crate::asset::Measured {
                facts: crate::asset::ModelFacts {
                    file: file.clone(),
                    detail: kept.clone(),
                    front: front.clone(),
                    up: up.clone(),
                    pivot: pivot.clone(),
                    rows: crate::asset_gltf::Axes::parse(&front, &up)
                        .map_or([[-1, 0, 0], [0, 0, 1], [0, 1, 0]], |x| x.rows()),
                    pivot_mm: [0; 3],
                    detail_triangles: Vec::new(),
                    files: BTreeMap::new(),
                    copies: Vec::new(),
                    measured: false,
                    scale,
                },
                raw: None,
                triangles: 0,
                plan: None,
            },
        };
        // 3.4: the plan moved by the pivot as `bounds` is, rings of points in the asset frame.
        plan = m.plan.as_ref().map_or_else(Vec::new, |rings| {
            let p = m.facts.pivot_mm;
            rings
                .iter()
                .map(|r| r.iter().map(|q| (q[0] - p[0], q[1] - p[1])).collect())
                .collect()
        });
        let bounds = match m.raw {
            Some(raw) => {
                let p = m.facts.pivot_mm;
                origin = [-p[0], -p[1], -p[2]];
                measured = true;
                [
                    raw[0] - p[0],
                    raw[1] - p[1],
                    raw[2] - p[2],
                    raw[3] - p[0],
                    raw[4] - p[1],
                    raw[5] - p[2],
                ]
            }
            None => [0; 6],
        };
        if measured && origin != [0, 0, 0] {
            let shown: Vec<String> = origin.iter().map(|v| via_num::write_length(*v)).collect();
            own.push(text::diag(
                codes::asset::PIVOT_MOVED,
                header.0.clone(),
                header.1.clone(),
                &format!(
                    "the model's origin is not at its `{pivot}` pivot; the build moved it: origin = [{}]",
                    shown.join(", ")
                ),
                "write `pivot = \"origin\"` to keep the model's own origin",
            ));
        }
        let size = [
            bounds[3] - bounds[0],
            bounds[4] - bounds[1],
            bounds[5] - bounds[2],
        ];
        let triangles = m.triangles;
        model = Some(m.facts);
        (size, triangles, bounds)
    } else {
        // 8.1: an absent `size` is the source's fault; a present one of the wrong form is
        // `doc.type`, and a length off the lattice `num.too_precise` (`numbers.md` 3.1), as in a
        // world file.
        let size = match a.get("size") {
            None => {
                r.fault(
                    codes::asset::SOURCE,
                    &subject,
                    None,
                    "the asset lacks `size`, three lengths",
                    "write `size = [x, y, z]` in metres",
                );
                return None;
            }
            Some(v) => match size_of(v) {
                Ok(s) => s,
                Err((code, message, hint)) => {
                    r.fault(code, &subject, Some("size"), &message, &hint);
                    return None;
                }
            },
        };
        if size.iter().any(|v| *v <= 0) {
            r.fault(
                codes::pack::RANGE,
                &subject,
                Some("size"),
                "`size` must be above 0 on every axis",
                "write lengths above 0",
            );
            return None;
        }
        let triangles = match &primitive {
            Some(p) => match PRIMITIVES.iter().find(|(n, _)| n == p) {
                Some((_, t)) => *t,
                None => {
                    r.fault(
                        codes::asset::UNKNOWN_PRIMITIVE,
                        &subject,
                        Some("primitive"),
                        &format!("`{p}` is not a primitive shape"),
                        &text::near_hint(p, PRIMITIVES.iter().map(|(n, _)| *n), "see assets.md 6"),
                    );
                    r.near(p, PRIMITIVES.iter().map(|(n, _)| *n), text::undeclared);
                    return None;
                }
            },
            None => match a.get("triangles").and_then(toml::Value::as_integer) {
                Some(t) if t >= 0 => t as u64,
                _ => {
                    // A missing key is located at the table that should hold it (1.2).
                    r.fault(
                        codes::asset::SOURCE,
                        &subject,
                        a.contains_key("triangles").then_some("triangles"),
                        "an engine-made asset needs `triangles`, 0 or more",
                        "write `triangles = <count>`",
                    );
                    return None;
                }
            },
        };
        let [x, y, z] = size;
        let bounds = [
            -x.div_euclid(2),
            -y.div_euclid(2),
            0,
            x - x.div_euclid(2),
            y - y.div_euclid(2),
            z,
        ];
        (size, triangles, bounds)
    };
    let footprint = match a.get("footprint") {
        // 2.4: `footprint = "plan"`, a model's measured plan (3.4) where it is one simple ring,
        // else the box where none is written. A primitive or engine-made asset keeps its own
        // (a cylinder's round one). A plan of several pieces, or a ring that touches itself at a
        // corner, is no footprint: said, never one piece of it taken (F5's review).
        Some(toml::Value::String(w)) if w == "plan" && model.is_some() => {
            match plan_footprint(&plan, &bounds) {
                Some(f) => f,
                None => {
                    r.fault(
                        codes::asset::BAD_FOOTPRINT,
                        &subject,
                        Some("footprint"),
                        "the model's measured plan is not one simple ring: it covers pieces apart, or touches itself at a corner",
                        "declare the footprint as points",
                    );
                    return None;
                }
            }
        }
        Some(toml::Value::String(w)) if w == "plan" => match primitive.as_deref() {
            Some("cylinder" | "cone" | "sphere") => ring16(&bounds),
            _ => vec![
                (bounds[0], bounds[1]),
                (bounds[3], bounds[1]),
                (bounds[3], bounds[4]),
                (bounds[0], bounds[4]),
            ],
        },
        Some(toml::Value::String(w)) => {
            r.fault(
                codes::asset::BAD_FOOTPRINT,
                &subject,
                Some("footprint"),
                &format!("`footprint = \"{w}\"`: the one word it takes is \"plan\""),
                "write `footprint = \"plan\"`, or the footprint as points",
            );
            return None;
        }
        Some(v) => {
            let pts: Option<Vec<(i64, i64)>> = match v {
                toml::Value::Array(xs) => xs
                    .iter()
                    .map(|p| lengths(p).filter(|l| l.len() == 2).map(|l| (l[0], l[1])))
                    .collect(),
                _ => None,
            };
            match pts {
                Some(p) if is_ccw_simple(&p) => p,
                _ => {
                    r.fault(
                        codes::asset::BAD_FOOTPRINT,
                        &subject,
                        Some("footprint"),
                        "the footprint is not a simple polygon of three or more distinct points, counter-clockwise",
                        "reverse the points, or remove the repeated one",
                    );
                    return None;
                }
            }
        }
        None => match primitive.as_deref() {
            Some("cylinder" | "cone" | "sphere") => ring16(&bounds),
            // 2.4: a measured footprint of zero area is a point footprint at the pivot.
            _ if model.is_some() && (bounds[0] == bounds[3] || bounds[1] == bounds[4]) => {
                if measured {
                    own.push(text::diag(
                        codes::asset::POINT_FOOTPRINT,
                        header.0.clone(),
                        header.1.clone(),
                        "the model has no extent in plan; the asset has a point footprint",
                        "declare `footprint` if it should claim ground",
                    ));
                }
                Vec::new()
            }
            _ => vec![
                (bounds[0], bounds[1]),
                (bounds[3], bounds[1]),
                (bounds[3], bounds[4]),
                (bounds[0], bounds[4]),
            ],
        },
    };
    let passable = declared("solidity").and_then(|v| v.as_str()) == Some("passable");
    let solid = match declared("solidity").and_then(|v| v.as_str()) {
        None | Some("solid" | "passable") => true,
        Some("none") => false,
        Some(o) => {
            r.fault(
                codes::doc::TYPE,
                &subject,
                Some("solidity"),
                &format!("`{o}` is not a solidity"),
                &text::near_hint(
                    o,
                    ["solid", "passable", "none"],
                    "write `solid`, `passable` or `none`",
                ),
            );
            true
        }
    };
    let sink = declared("sink").and_then(double).unwrap_or(0.0);
    let max_relief = declared("max_relief").and_then(double);
    if max_relief.is_some_and(|m| m < 0.0) {
        r.fault(
            codes::pack::RANGE,
            &subject,
            Some("max_relief"),
            "`max_relief` must be 0 or more",
            "write a number of metres, 0 or more",
        );
    }
    let colour = match declared("colour").and_then(doubles) {
        Some(c) if c.len() == 3 => {
            if c.iter().any(|v| !(0.0..=1.0).contains(v)) {
                r.fault(
                    codes::pack::RANGE,
                    &subject,
                    Some("colour"),
                    "each part of `colour` lies in 0 to 1",
                    "write linear red, green, blue, each 0 to 1",
                );
            }
            [c[0], c[1], c[2]]
        }
        _ => [0.8, 0.8, 0.8],
    };
    let taxon = a.get("taxon").and_then(|v| v.as_str()).map(str::to_string);
    if let Some(t) = &taxon {
        if !taxa.contains(t) {
            r.fault(
                codes::pack::UNKNOWN_TAXON,
                &subject,
                Some("taxon"),
                &format!("the taxon `{t}` is not declared"),
                &text::near_hint(
                    t,
                    taxa.iter().map(String::as_str),
                    "declare `[taxon.\"<path>\"]`",
                ),
            );
            let declared = |n: &str| r.declared("taxon", n);
            let at = via_diag::nearest::nearest(t, taxa.iter().map(String::as_str))
                .found()
                .and_then(declared);
            r.near(t, taxa.iter().map(String::as_str), |_| at);
        }
    }
    let tags = a
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|xs| {
            xs.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut mounts = BTreeMap::new();
    let mut connectors = BTreeMap::new();
    let mut rises = BTreeMap::new();
    if let Some(toml::Value::Table(ms)) = a.get("mount") {
        for (name, m) in ms {
            if matches!(name.as_str(), "start" | "end" | "ground") {
                let free = via_doc::free_name(name, &|c| ms.contains_key(c));
                r.fault(
                    codes::identity::RESERVED,
                    &subject,
                    Some("mount"),
                    &format!("a mount may not be named `{name}`"),
                    &format!("choose another name, such as `{free}`"),
                );
                if let Some(d) = r.diags.pop() {
                    r.diags
                        .push(d.with("nearest", via_diag::Value::text(free.clone())));
                }
                continue;
            }
            match m.get("connector") {
                None => {}
                Some(toml::Value::String(c)) => {
                    connectors.insert(name.clone(), c.clone());
                }
                Some(_) => r.fault(
                    codes::doc::TYPE,
                    &format!("{subject}.mount.{name}"),
                    Some("connector"),
                    "`connector` is a word",
                    "write `connector = \"<word>\"`",
                ),
            }
            // 2.7: the keys of an asset mount. A kind mount's (`t`, `offset`, `surface`, `meets`)
            // are not taken: an asset has no path, and its mount lies on what its instance
            // stands on.
            if let Some(mt) = m.as_table() {
                for k in mt.keys() {
                    if !MOUNT_KEYS.contains(&k.as_str()) {
                        r.fault(
                            codes::doc::UNKNOWN_KEY,
                            &format!("{subject}.mount.{name}"),
                            Some(k),
                            &format!("`{k}` is not a key of an asset mount"),
                            &match k.as_str() {
                                "t" | "offset" => "an asset has no path: write `at = [x, y]` in the asset frame".to_string(),
                                "surface" => "remove it: an asset mount lies on what its instance stands on; `rise` lifts it above that".to_string(),
                                "meets" => "remove it: an instance's mount meets a path end written at it, or another instance's mount at the same point".to_string(),
                                _ => text::near_hint(k, MOUNT_KEYS, "see assets.md 2.7"),
                            },
                        );
                        r.near(k, MOUNT_KEYS, text::undeclared);
                    }
                }
            }
            // `rise` (2.7, R75): lattice metres above the surface the instance stands on.
            match m.get("rise") {
                None => {}
                Some(v) => match num_text(v).and_then(|t| via_num::parse_length(&t).ok()) {
                    Some(0) => {}
                    Some(rise) => {
                        rises.insert(name.clone(), rise);
                    }
                    None => r.fault(
                        codes::doc::TYPE,
                        &format!("{subject}.mount.{name}"),
                        Some("rise"),
                        &format!("`rise` takes a length in metres; found {}", form_word(v)),
                        "write `rise = <metres>`, the mount's height above what the instance stands on",
                    ),
                },
            }
            let at = m
                .get("at")
                .and_then(lengths)
                .filter(|l| l.len() == 2)
                .map(|l| (l[0], l[1]));
            let face = m
                .get("face")
                .and_then(num_text)
                .and_then(|t| via_num::parse_facing(&t).ok())
                .unwrap_or(0);
            match at {
                Some(at) => {
                    mounts.insert(name.clone(), (at, face));
                }
                None => r.fault(
                    codes::doc::MISSING_KEY,
                    &subject,
                    Some("mount"),
                    &format!("the mount `{name}` has no `at`"),
                    "write `at = [x, y]` in the asset frame",
                ),
            }
        }
    }
    let params = declared("params").and_then(|v| v.as_table()).cloned();
    // `shared` is never the asset's own (2.5): a taxon's or the pack's, the nearest taken whole.
    let shared = inherited
        .get(taxon_of, "shared")
        .and_then(|v| v.as_table())
        .cloned();
    let asset = Asset {
        id: id.to_string(),
        source: if model.is_some() {
            "model"
        } else if engine {
            "engine"
        } else {
            "primitive"
        }
        .into(),
        primitive,
        size,
        triangles,
        bounds,
        footprint,
        footprint_plan: matches!(a.get("footprint"), Some(toml::Value::String(w)) if w == "plan"),
        plan,
        solid,
        passable,
        sink,
        max_relief,
        colour,
        mounts,
        connectors,
        rises,
        taxon,
        tags,
        collision,
        origin,
        model,
        params,
        shared,
    };
    Some((asset, own))
}

/// `footprint = "plan"` (2.4): the measured plan's one outer ring (3.4), where it is one and
/// simple; the rectangle of `bounds` where no plan is written; `None` for a plan of several
/// pieces or a ring that touches itself.
pub(crate) fn plan_footprint(
    plan: &[Vec<(i64, i64)>],
    bounds: &[i64; 6],
) -> Option<Vec<(i64, i64)>> {
    let area2 = |r: &Vec<(i64, i64)>| -> i128 {
        (0..r.len())
            .map(|k| {
                let (a, b) = (r[k], r[(k + 1) % r.len()]);
                i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1)
            })
            .sum()
    };
    let outer: Vec<&Vec<(i64, i64)>> = plan.iter().filter(|r| area2(r) > 0).collect();
    match outer.as_slice() {
        [] => Some(vec![
            (bounds[0], bounds[1]),
            (bounds[3], bounds[1]),
            (bounds[3], bounds[4]),
            (bounds[0], bounds[4]),
        ]),
        [one] if is_ccw_simple(one) => Some((*one).clone()),
        _ => None,
    }
}

/// An enumerated declared fact (2.4: `pivot`, `collision`): one of `words`, else `doc.type`
/// naming the allowed words and the nearest; absent, its default.
fn enum_fact(
    r: &mut Reader<'_>,
    subject: &str,
    key: &str,
    v: Option<&toml::Value>,
    words: &[&str],
    default: &str,
) -> String {
    let allowed = words
        .iter()
        .map(|w| format!("`{w}`"))
        .collect::<Vec<_>>()
        .join(", ");
    match v {
        None => default.to_string(),
        Some(toml::Value::String(w)) if words.contains(&w.as_str()) => w.clone(),
        Some(toml::Value::String(w)) => {
            r.fault(
                codes::doc::TYPE,
                subject,
                Some(key),
                &format!("`{w}` is not a `{key}`; one of {allowed}"),
                &text::near_hint(w, words.iter().copied(), &format!("write one of {allowed}")),
            );
            r.near(w, words.iter().copied(), text::undeclared);
            default.to_string()
        }
        Some(other) => {
            r.fault(
                codes::doc::TYPE,
                subject,
                Some(key),
                &format!("`{key}` takes one of {allowed}; found {}", form_word(other)),
                &format!("write one of {allowed}"),
            );
            default.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped(name: &str) -> Pack {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../lib/packs")
            .join(name);
        let (p, diags) = read(&path, "pack:test.toml");
        assert!(
            diags
                .iter()
                .all(|d| d.severity() != via_diag::Severity::Error),
            "{diags:?}"
        );
        p.expect("the pack reads")
    }

    #[test]
    fn the_starter_pack_measures() {
        let p = shipped("starter.toml");
        assert!(p
            .asset_diags
            .values()
            .flatten()
            .all(|d| d.severity() != via_diag::Severity::Error));
        let oak = &p.assets["tree_oak"];
        assert_eq!(oak.source, "model");
        assert_eq!(oak.bounds, [-2500, -2500, 0, 2500, 2500, 7000]);
        assert_eq!(oak.origin, [0, 0, 0]);
        assert_eq!(oak.collision, "hull");
        let m = oak.model.as_ref().expect("a model");
        assert_eq!(m.detail_triangles.len(), 1);
        assert!(m.detail_triangles[0] < oak.triangles);
        assert_eq!(p.assets["bush_round"].collision, "none");
        assert!(!p.assets["bush_round"].solid);
        assert_eq!(p.assets["house_low"].collision, "box");
        assert_eq!(p.assets["house_low"].max_relief, Some(0.6));
        // The shipped facts file is current: nothing to report.
        assert!(p
            .check_diagnostics()
            .iter()
            .all(|d| d.severity() == via_diag::Severity::Info
                && *d.code() != via_diag::DiagCode::Core(codes::asset::REMODELLED)
                && *d.code() != via_diag::DiagCode::Core(codes::asset::REFILED)));
        assert!(matches!(p.facts, crate::asset_facts::FactsFile::Present(_)));
    }

    #[test]
    fn taxonomy_queries() {
        let dir = std::env::temp_dir().join(format!("via-pack-query-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary folder");
        let path = dir.join("q.toml");
        std::fs::write(
            &path,
            "[taxon.\"kit/arch\"]\n[taxon.\"kit/wall\"]\n[taxon.\"loose\"]\n\n\
             [asset.b]\nprimitive = \"box\"\nsize = [1, 1, 1]\ntaxon = \"kit/wall\"\ntags = [\"old\"]\n\n\
             [asset.a]\nprimitive = \"box\"\nsize = [1, 1, 1]\ntaxon = \"kit/arch\"\n\n\
             [asset.c]\nprimitive = \"box\"\nsize = [1, 1, 1]\ntaxon = \"kit\"\n\n\
             [asset.d]\nprimitive = \"box\"\nsize = [1, 1, 1]\n",
        )
        .expect("the pack is written");
        let (p, _) = read(&path, "pack:q.toml");
        let _ = std::fs::remove_dir_all(&dir);
        let p = p.expect("the pack reads");
        let ids = |v: Vec<&Asset>| v.iter().map(|a| a.id.clone()).collect::<Vec<_>>();
        assert_eq!(
            ids(p.query(&["@kit".into()], None).unwrap()),
            ["a", "b", "c"]
        );
        assert_eq!(
            ids(p.query(&["d".into(), "@kit/wall".into()], None).unwrap()),
            ["b", "d"]
        );
        assert_eq!(p.query(&["*".into()], None).unwrap().len(), 4);
        assert_eq!(ids(p.query(&["*".into()], Some("old")).unwrap()), ["b"]);
        let e = p.query(&["@kit/wal".into()], None).unwrap_err();
        assert_eq!(e.0, codes::pack::UNKNOWN_TAXON);
        assert_eq!(e.3.as_deref(), Some("kit/wall"));
        assert_eq!(
            p.query(&["@loose".into()], None).unwrap_err().0,
            codes::asset::EMPTY_SELECTION
        );
        assert_eq!(
            p.query(&["e".into()], None).unwrap_err().0,
            codes::asset::UNKNOWN
        );
        assert_eq!(
            p.query(&["*".into()], Some("new")).unwrap_err().0,
            codes::asset::EMPTY_SELECTION
        );
    }

    #[test]
    fn box_bounds_and_ring() {
        let b = [-500, -1000, 0, 501, 1000, 3000];
        assert_eq!(
            [-(1001_i64.div_euclid(2)), -1000, 0, 1001 - 500, 1000, 3000],
            b
        );
        let ring = ring16(&[-1000, -1000, 0, 1000, 1000, 1000]);
        assert_eq!(ring.len(), 16);
        assert_eq!(ring[0], (1000, 0));
        assert_eq!(ring[4], (0, 1000));
        assert!(is_ccw_simple(&ring));
    }
    /// A pack of one box with the given mount tables, read from a temporary folder.
    fn mounted(tag: &str, mounts: &str) -> (Option<Pack>, Vec<Diag>) {
        let dir = std::env::temp_dir().join(format!("via-pack-rise-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary folder");
        let path = dir.join("p.toml");
        std::fs::write(
            &path,
            format!("[asset.box]\nprimitive = \"box\"\nsize = [4, 0.3, 6]\n\n{mounts}"),
        )
        .expect("the pack is written");
        let got = read(&path, "pack:p.toml");
        let _ = std::fs::remove_dir_all(&dir);
        got
    }

    #[test]
    fn a_mounts_rise_is_a_fact_only_where_it_is_not_zero() {
        let (p, diags) = mounted(
            "facts",
            "[asset.box.mount.port]\nat = [0, 0.15]\nconnector = \"pipe\"\nrise = 5\n\n\
             [asset.box.mount.low]\nat = [1, 0.15]\nrise = 0\n\n\
             [asset.box.mount.pit]\nat = [2, 0.15]\nrise = -0.5\n\n\
             [asset.box.mount.side]\nat = [-1, 0.15]\n",
        );
        assert!(diags.is_empty(), "{diags:?}");
        let p = p.expect("the pack reads");
        let a = &p.assets["box"];
        assert_eq!(a.rises.get("port"), Some(&5000));
        assert_eq!(a.rises.get("pit"), Some(&-500));
        assert_eq!(a.rises.get("low"), None);
        assert_eq!(a.rises.get("side"), None);
        // The facts a build's digests and the manifest read: `rise` last, and only where set.
        let facts = a.facts();
        assert_eq!(
            facts["mounts"],
            "{\"low\":{\"at\":[1,0.15],\"face\":0},\
              \"pit\":{\"at\":[2,0.15],\"face\":0,\"rise\":-0.5},\
              \"port\":{\"at\":[0,0.15],\"connector\":\"pipe\",\"face\":0,\"rise\":5},\
              \"side\":{\"at\":[-1,0.15],\"face\":0}}"
        );
        // And the facts file's line (4.1).
        let entry = crate::asset_facts::entry(a);
        let line = &entry
            .iter()
            .find(|(k, _)| *k == "mounts")
            .expect("mounts")
            .1;
        assert!(
            line.contains("port = { at = [0, 0.15], connector = \"pipe\", face = 0, rise = 5 }"),
            "{line}"
        );
        assert!(
            line.contains("low = { at = [1, 0.15], face = 0 }"),
            "{line}"
        );
    }

    #[test]
    fn a_rise_that_is_no_length_and_a_key_no_mount_takes_are_refused() {
        let code = |d: &Diag| d.code().to_string();
        let (_, diags) = mounted(
            "word",
            "[asset.box.mount.port]\nat = [0, 0.15]\nrise = \"high\"\n",
        );
        assert_eq!(diags.iter().map(code).collect::<Vec<_>>(), ["doc.type"]);
        // Finer than the lattice is no lattice length.
        let (_, diags) = mounted(
            "fine",
            "[asset.box.mount.port]\nat = [0, 0.15]\nrise = 0.0005\n",
        );
        assert_eq!(diags.iter().map(code).collect::<Vec<_>>(), ["doc.type"]);
        for key in [
            "t = 0.5",
            "offset = 1",
            "surface = \"top\"",
            "meets = \"low\"",
            "rize = 5",
        ] {
            let (_, diags) = mounted(
                "key",
                &format!("[asset.box.mount.port]\nat = [0, 0.15]\n{key}\n"),
            );
            assert_eq!(
                diags.iter().map(code).collect::<Vec<_>>(),
                ["doc.unknown_key"],
                "{key}"
            );
        }
        // The misspelt key is told its neighbour.
        let (_, diags) = mounted("near", "[asset.box.mount.port]\nat = [0, 0.15]\nrize = 5\n");
        assert!(
            diags[0].hint().to_string().contains("rise"),
            "{:?}",
            diags[0]
        );
    }
}
