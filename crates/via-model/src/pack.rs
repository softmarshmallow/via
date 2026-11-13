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

const ASSET_KEYS: [&str; 18] = [
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
    "primitive",
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
    /// `size` in millimetres.
    pub size: [i64; 3],
    pub triangles: u64,
    /// `[x0, y0, z0, x1, y1, z1]` in millimetres.
    pub bounds: [i64; 6],
    /// The footprint in the asset frame, counter-clockwise; empty for a point footprint.
    pub footprint: Vec<(i64, i64)>,
    pub solid: bool,
    /// Metres.
    pub sink: f64,
    pub max_relief: Option<f64>,
    pub colour: [f64; 3],
    /// Mounts: name to (`at` in mm, face in millidegrees).
    pub mounts: BTreeMap<String, ((i64, i64), i64)>,
    pub taxon: Option<String>,
    pub tags: Vec<String>,
}

impl Asset {
    /// The facts the change report compares (7), each as canonical text.
    pub fn facts(&self) -> BTreeMap<String, String> {
        let mut f = BTreeMap::new();
        let mm = |v: i64| via_num::write_length(v);
        f.insert("size".into(), format!("[{}]", self.size.map(mm).join(", ")));
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
        f.insert("origin".into(), "[0, 0, 0]".into());
        f.insert("sink".into(), crate::text::dbl(self.sink));
        f.insert(
            "max_relief".into(),
            self.max_relief
                .map_or_else(|| "none".to_string(), crate::text::dbl),
        );
        f.insert(
            "solidity".into(),
            if self.solid { "solid" } else { "none" }.into(),
        );
        let mounts: Vec<String> = self
            .mounts
            .iter()
            .map(|(n, ((x, y), face))| format!("{n}@[{}, {}]/{face}", mm(*x), mm(*y)))
            .collect();
        f.insert("mounts".into(), format!("[{}]", mounts.join(", ")));
        f
    }
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
}

impl Pack {
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
    let mut pack = Pack {
        display: display.to_string(),
        ..Pack::default()
    };
    // 2.1: `root`, relative to the pack file; the pack file's folder by default.
    let here = path.parent().map(Path::to_path_buf).unwrap_or_default();
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
            Some(dir)
        }
        Some(_) => None,
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
            if let Some(asset) = read_asset(&mut r, id, a, &pack.taxa, &inherited) {
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
const DEFAULT_KEYS: [&str; 8] = [
    "colour",
    "front",
    "max_relief",
    "params",
    "pivot",
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
) -> Option<Asset> {
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
        if !ASSET_KEYS.contains(&k.as_str()) {
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
    for k in ["file", "detail"] {
        if a.contains_key(k) {
            r.fault(
                codes::doc::NOT_YET,
                &subject,
                Some(k),
                &format!("`{k}` on an asset arrives in M2"),
                "use a primitive or an engine-made asset in M1",
            );
            return None;
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
    if primitive.is_some() == engine {
        r.fault(
            codes::asset::SOURCE,
            &subject,
            None,
            "an asset needs exactly one source: `primitive` or `engine = true`",
            "write exactly one of `file`, `primitive`, `engine = true`",
        );
        return None;
    }
    // 2.2: `front`, `up` and `pivot` are fixed on a primitive and an engine-made asset, and
    // `triangles` on a primitive; only the asset's own table can write one (defaults skip it).
    for k in ["front", "up", "pivot", "triangles"] {
        if a.contains_key(k) && (k != "triangles" || primitive.is_some()) {
            r.fault(
                codes::asset::DECLARED_MEASURED,
                &subject,
                Some(k),
                &format!(
                    "`{k}` is fixed on a {} asset",
                    if engine { "engine-made" } else { "primitive" }
                ),
                &format!("remove `{k}`"),
            );
        }
    }
    let taxon_of = a.get("taxon").and_then(|v| v.as_str());
    // A declared fact: the asset's own value, else an inherited one (2.5).
    let declared = |k: &str| a.get(k).or_else(|| inherited.get(taxon_of, k));
    // 8.1: an absent `size` is the source's fault; a present one of the wrong form is `doc.type`,
    // and a length off the lattice `num.too_precise` (`numbers.md` 3.1), as in a world file.
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
    let footprint = match a.get("footprint") {
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
            _ => vec![
                (bounds[0], bounds[1]),
                (bounds[3], bounds[1]),
                (bounds[3], bounds[4]),
                (bounds[0], bounds[4]),
            ],
        },
    };
    let solid = match declared("solidity").and_then(|v| v.as_str()) {
        None | Some("solid") => true,
        Some("none") => false,
        Some("passable") => {
            r.fault(
                codes::doc::NOT_YET,
                &subject,
                Some("solidity"),
                "`passable` arrives in M5",
                "write `solid` or `none`",
            );
            true
        }
        Some(o) => {
            r.fault(
                codes::doc::TYPE,
                &subject,
                Some("solidity"),
                &format!("`{o}` is not a solidity"),
                &text::near_hint(o, ["solid", "none"], "write `solid` or `none`"),
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
            if m.get("connector").is_some() {
                r.fault(
                    codes::doc::NOT_YET,
                    &format!("{subject}.mount.{name}"),
                    Some("connector"),
                    "`connector` on an asset mount arrives in M5",
                    "remove `connector`",
                );
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
    Some(Asset {
        id: id.to_string(),
        source: if engine { "engine" } else { "primitive" }.into(),
        primitive,
        size,
        triangles,
        bounds,
        footprint,
        solid,
        sink,
        max_relief,
        colour,
        mounts,
        taxon,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
