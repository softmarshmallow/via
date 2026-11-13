//! Packs (`spec/assets.md`, the M1 sections): finding a pack (1.2), reading it (2), primitive
//! facts (6), the taxonomy (5) and the facts the change report compares (7).
//!
//! A pack is flat TOML that `via-doc` does not read (it is no world document), so it is read
//! here with the `toml` crate. Its numbers are written back to their shortest decimal text and
//! read by `via-num` as any length is, so `0.15` is 150 mm exactly. Diagnostics name the pack
//! file and, where the header is found in the text, the line (`data.line`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use via_diag::{codes, Code, Diag, Site, Subject, Value as DValue};

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
}

impl Pack {
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
            diags.push(pack_diag(
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

/// The line of the first occurrence of `needle` in `text`, 1-based.
fn line_of(text: &str, needle: &str) -> Option<i64> {
    let at = text.find(needle)?;
    Some(text[..at].bytes().filter(|b| *b == b'\n').count() as i64 + 1)
}

struct Reader<'a> {
    file: String,
    text: &'a str,
    diags: Vec<Diag>,
}

impl Reader<'_> {
    fn fault(&mut self, code: Code, subject: &str, key: Option<&str>, message: &str, hint: &str) {
        let mut loc = BTreeMap::new();
        let needle = match key {
            Some(_) | None => subject_header(subject),
        };
        if let Some(l) = line_of(self.text, &needle) {
            loc.insert("line".to_string(), DValue::Int(l));
        }
        let subj = Subject::parse(&match key {
            Some(k) => format!("{subject}.{k}"),
            None => subject.to_string(),
        })
        .unwrap_or(Subject::World);
        self.diags.push(text::diag(
            code,
            subj,
            Site::File {
                file: self.file.clone(),
                locator: loc,
            },
            message,
            hint,
        ));
    }
}

fn subject_header(subject: &str) -> String {
    match subject.split_once('.') {
        Some((top, id)) => format!("[{top}.{id}"),
        None => subject.to_string(),
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
    let bytes = match std::fs::read_to_string(path) {
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
    let mut r = Reader {
        file: display.to_string(),
        text: &bytes,
        diags: Vec::new(),
    };
    let table: toml::Table = match bytes.parse() {
        Ok(t) => t,
        Err(e) => {
            r.diags.push(text::diag(
                codes::doc::SYNTAX,
                Subject::World,
                Site::File {
                    file: display.to_string(),
                    locator: BTreeMap::new(),
                },
                &format!("the pack is not valid TOML: {e}"),
                "fix the pack's syntax",
            ));
            return (None, r.diags);
        }
    };
    let mut pack = Pack {
        display: display.to_string(),
        ..Pack::default()
    };
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
            if let Some(asset) = read_asset(&mut r, id, a, &pack.taxa) {
                pack.assets.insert(id.clone(), asset);
            }
        }
    }
    (Some(pack), r.diags)
}

fn read_asset(
    r: &mut Reader<'_>,
    id: &str,
    a: &toml::Table,
    taxa: &BTreeSet<String>,
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
    let size = match a.get("size").and_then(lengths) {
        Some(s) if s.len() == 3 => [s[0], s[1], s[2]],
        _ => {
            r.fault(
                codes::asset::SOURCE,
                &subject,
                Some("size"),
                "the asset lacks `size`, three lengths",
                "write `size = [x, y, z]` in metres",
            );
            return None;
        }
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
                return None;
            }
        },
        None => match a.get("triangles").and_then(toml::Value::as_integer) {
            Some(t) if t >= 0 => t as u64,
            _ => {
                r.fault(
                    codes::asset::SOURCE,
                    &subject,
                    Some("triangles"),
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
    let solid = match a.get("solidity").and_then(|v| v.as_str()) {
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
    let sink = a.get("sink").and_then(double).unwrap_or(0.0);
    let max_relief = a.get("max_relief").and_then(double);
    if max_relief.is_some_and(|m| m < 0.0) {
        r.fault(
            codes::pack::RANGE,
            &subject,
            Some("max_relief"),
            "`max_relief` must be 0 or more",
            "write a number of metres, 0 or more",
        );
    }
    let colour = match a.get("colour").and_then(doubles) {
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
                r.fault(
                    codes::identity::RESERVED,
                    &subject,
                    Some("mount"),
                    &format!("a mount may not be named `{name}`"),
                    "choose another name",
                );
                continue;
            }
            if m.get("connector").is_some() {
                r.fault(
                    codes::doc::NOT_YET,
                    &subject,
                    Some("mount"),
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
