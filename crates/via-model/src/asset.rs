//! Model assets (`assets.md` 2.2 "Bound files", 2.3, 2.4 "Pivot", 3): finding the files a model
//! asset binds, measuring the model and its lower-detail copies, and placing its pivot.
//!
//! [`crate::pack`] reads the pack text and calls [`measure`] for each asset whose source is a
//! model. Every fault raised here concerns one asset only, and fails a build only when the world
//! uses that asset (8.3); the pack reader keeps them apart for that reason.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use via_diag::{codes, Code, Diag, Site, Subject, Value};

use crate::asset_gltf::{Axes, Measure, Model};
use crate::text;

/// The pivot rules of 2.4.
pub const PIVOTS: [&str; 3] = ["base_centre", "base_corner", "origin"];

/// The collision words of 2.4.
pub const COLLISIONS: [&str; 4] = ["none", "box", "hull", "mesh"];

/// What a model asset holds besides the facts every asset has (2.2, 2.3, 3, 4.1).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelFacts {
    /// `file`, relative to `root`, as written.
    pub file: String,
    /// `detail`, finest first, as written.
    pub detail: Vec<String>,
    pub front: String,
    pub up: String,
    pub pivot: String,
    /// The quarter turn from model axes to the asset frame, row by row (`build.md` 7).
    pub rows: [[i64; 3]; 3],
    /// The pivot in the asset frame's axes before it is placed, millimetres (2.4).
    pub pivot_mm: [i64; 3],
    /// Triangles of each `detail` copy, in `detail`'s order (2.4).
    pub detail_triangles: Vec<u64>,
    /// Every bound file: its path relative to `root`, `/` separators, to its digest (4.1 `files`).
    pub files: BTreeMap<String, String>,
    /// The model, then each `detail` copy: its pack path (relative to `root`, `build.md` 3.4) and
    /// digest (`build.md` 7 `files`).
    pub copies: Vec<(String, String)>,
    /// Whether the model measured; when not, its facts are not known and the asset carries an
    /// error that fails any build using it.
    pub measured: bool,
    /// The declared factor from the model's units to metres (2.4, `scale`; 1 by default): every
    /// measured fact is the scaled model's, and `to_asset` carries it (`build.md` 7).
    pub scale: f64,
}

/// Where a pack's files are: the pack file's folder, `root`, and `root` as written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RootOf {
    pub root: PathBuf,
    /// `root` as written, as parts relative to the pack file's folder (empty for the folder).
    pub written: Vec<String>,
}

impl RootOf {
    /// A path relative to `root` as the pack path a record names (`documents.md` 2.1:
    /// `pack:` and the path relative to the pack file's folder).
    pub fn pack_path(&self, rel: &str) -> String {
        let mut parts = self.written.clone();
        for p in rel.split('/') {
            match p {
                "" | "." => {}
                ".." if parts.last().is_some_and(|l| l != "..") => {
                    parts.pop();
                }
                _ => parts.push(p.to_string()),
            }
        }
        format!("pack:{}", parts.join("/"))
    }
}

/// A fault, as code, message and hint.
type Why = (Code, String, String);

/// Joins `written` onto `folder` (both relative to `root`, by parts) by text (`documents.md` 7.2:
/// `..` resolved before anything is opened). `None` when it climbs out of `root`.
fn join_in_root(folder: &[String], written: &str) -> Option<Vec<String>> {
    let mut out = folder.to_vec();
    for p in written.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            _ => out.push(p.to_string()),
        }
    }
    Some(out)
}

/// Opens a path relative to `root`, given as parts, by `documents.md` 7.2's walk, then refuses a
/// symbolic link on the way (2.2: the bytes in the folder are the input).
fn open_in_root(root: &Path, parts: &[String], shown: &str) -> Result<PathBuf, Why> {
    let joined = parts.join("/");
    if joined.is_empty() {
        return Err((
            codes::pack::MISSING_FILE,
            format!("the path `{shown}` names no file"),
            "name a file under the pack's `root`".into(),
        ));
    }
    let found = via_doc::open_file(root, &joined).map_err(|(code, message, hint)| {
        let code = if code == codes::doc::FILE_MISSING {
            codes::pack::MISSING_FILE
        } else {
            code
        };
        (code, message, hint)
    })?;
    let mut at = root.to_path_buf();
    for p in parts {
        at.push(p);
        if at
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err((
                codes::pack::OUTSIDE_ROOT,
                format!("`{shown}` passes through a symbolic link below `root`"),
                "move the file itself under `root`, or move `root`".into(),
            ));
        }
    }
    if !found.is_file() {
        return Err((
            codes::pack::MISSING_FILE,
            format!("`{shown}` is not a file"),
            "name a file under the pack's `root`".into(),
        ));
    }
    Ok(found)
}

/// A pack-text path (`file`, a `detail` copy) opened under `root`.
fn open_written(root: &Path, written: &str) -> Result<(PathBuf, Vec<String>), Why> {
    let Some(parts) = join_in_root(&[], written) else {
        return Err((
            codes::pack::OUTSIDE_ROOT,
            format!("`{written}` climbs out of the pack's `root`"),
            "move the file under `root`, or move `root`".into(),
        ));
    };
    open_in_root(root, &parts, written).map(|p| (p, parts))
}

/// Whether a URI has a scheme (RFC 3986 3.1): letters, then letters, digits, `+`, `-` or `.`,
/// then `:`, before any `/`.
fn has_scheme(uri: &str) -> bool {
    let Some((head, _)) = uri.split_once(':') else {
        return false;
    };
    let mut cs = head.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphabetic())
        && cs.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// One read copy: its bytes' digest and its measure.
struct Copy {
    measure: Measure,
    untextured: bool,
}

/// What [`measure`] gives back.
pub struct Measured {
    pub facts: ModelFacts,
    /// `[x0, y0, z0, x1, y1, z1]` in the asset frame's axes before the pivot, mm.
    pub raw: Option<[i64; 6]>,
    pub triangles: u64,
    /// What the main model covers in plan (3.4), in `raw`'s axes and millimetres; `None` where
    /// it is the rectangle of `raw` or nothing has area in plan.
    pub plan: Option<Vec<Vec<[i64; 2]>>>,
    /// The main model's `mount_<name>` empties (2.7), in `raw`'s axes: name, origin, face.
    pub mounts: Vec<(String, [i64; 3], i64)>,
}

/// Collects the diagnostics of one model asset.
pub struct Faults<'a> {
    pub subject: &'a str,
    pub diags: Vec<Diag>,
    /// The pack-text site of a key of the asset, for faults in its text (`file`, `detail`).
    pub site_of: &'a dyn Fn(&str) -> Site,
}

impl Faults<'_> {
    fn subject(&self, key: Option<&str>) -> Subject {
        let s = match key {
            Some(k) => format!("{}.{k}", self.subject),
            None => self.subject.to_string(),
        };
        Subject::parse(&s).unwrap_or(Subject::World)
    }

    /// A fault located at a key of the asset's text.
    fn at_key(&mut self, key: &str, (code, message, hint): Why) {
        let site = (self.site_of)(key);
        self.diags.push(text::diag(
            code,
            self.subject(Some(key)),
            site,
            &message,
            &hint,
        ));
    }

    /// A fault inside a model file: the "file" location form with `data.pointer` (3.1).
    fn in_file(&mut self, file: &str, pointer: Option<&str>, (code, message, hint): Why) {
        let mut locator = BTreeMap::new();
        if let Some(p) = pointer {
            locator.insert("pointer".to_string(), Value::text(p));
        }
        let site = Site::File {
            file: file.to_string(),
            locator,
        };
        self.diags
            .push(text::diag(code, self.subject(None), site, &message, &hint));
    }
}

/// Reads one copy and the files it references, adding each bound file to `files`.
#[allow(clippy::too_many_arguments)]
fn read_copy(
    root: &RootOf,
    path: &Path,
    parts: &[String],
    axes: Axes,
    want_plan: bool,
    scale: f64,
    files: &mut BTreeMap<String, String>,
    f: &mut Faults<'_>,
) -> Option<(Copy, String)> {
    let rel = parts.join("/");
    let shown = root.pack_path(&rel);
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            f.in_file(
                &shown,
                None,
                (
                    codes::pack::BAD_MODEL,
                    format!("the model cannot be read: {e}"),
                    "check the file".into(),
                ),
            );
            return None;
        }
    };
    let digest = via_graph::Digest::of(&bytes).hex();
    files.insert(rel.clone(), digest.clone());
    let fault = |f: &mut Faults<'_>, x: crate::asset_gltf::Fault| {
        f.in_file(&shown, x.pointer.as_deref(), (x.code, x.message, x.hint));
    };
    let model = match Model::open(&bytes) {
        Ok(m) => m,
        Err(x) => {
            fault(f, x);
            return None;
        }
    };
    let uris = match model.uris() {
        Ok(u) => u,
        Err(x) => {
            fault(f, x);
            return None;
        }
    };
    let folder: Vec<String> = parts[..parts.len().saturating_sub(1)].to_vec();
    let mut external: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    let mut ok = true;
    for u in uris {
        let d = &u.decoded;
        let outside = if d.starts_with('/') || has_scheme(d) {
            None
        } else {
            join_in_root(&folder, d)
        };
        let Some(uparts) = outside else {
            f.in_file(
                &shown,
                Some(&u.pointer),
                (
                    codes::pack::OUTSIDE_ROOT,
                    format!(
                        "the model's URI `{}` reaches outside the pack's `root`",
                        u.written
                    ),
                    "move the file under `root` and reference it relatively, or move `root`".into(),
                ),
            );
            ok = false;
            continue;
        };
        match open_in_root(&root.root, &uparts, &u.written) {
            Ok(p) => match std::fs::read(&p) {
                Ok(b) => {
                    files.insert(uparts.join("/"), via_graph::Digest::of(&b).hex());
                    if let Some(i) = u.buffer {
                        external.insert(i, b);
                    }
                }
                Err(e) => {
                    f.in_file(
                        &shown,
                        Some(&u.pointer),
                        (
                            codes::pack::MISSING_FILE,
                            format!("the model's URI `{}` cannot be read: {e}", u.written),
                            "check the file".into(),
                        ),
                    );
                    ok = false;
                }
            },
            Err((code, message, hint)) => {
                f.in_file(
                    &shown,
                    Some(&u.pointer),
                    (
                        code,
                        format!("the model's URI `{}`: {message}", u.written),
                        hint,
                    ),
                );
                ok = false;
            }
        }
    }
    if !ok {
        return None;
    }
    match model.measure(&external, axes, want_plan, scale) {
        Ok(m) => Some((
            Copy {
                untextured: m.untextured,
                measure: m,
            },
            digest,
        )),
        Err(x) => {
            fault(f, x);
            None
        }
    }
}

/// The pivot of a pivot rule over raw bounds (2.4).
pub fn pivot_of(rule: &str, raw: &[i64; 6]) -> [i64; 3] {
    match rule {
        "base_corner" => [raw[0], raw[1], raw[2]],
        "origin" => [0, 0, 0],
        _ => [
            (raw[0] + raw[3]).div_euclid(2),
            (raw[1] + raw[4]).div_euclid(2),
            raw[2],
        ],
    }
}

/// Measures a model asset (3): its file, then each `detail` copy (3.3), with the bound files
/// they reference (2.2), and its plan (3.4) where `want_plan`. Faults go to `f`.
#[allow(clippy::too_many_arguments)]
pub fn measure(
    root: &RootOf,
    file: &str,
    detail: &[String],
    front: &str,
    up: &str,
    pivot: &str,
    want_plan: bool,
    scale: f64,
    f: &mut Faults<'_>,
) -> Measured {
    // A pair that is no rotation was reported by the pack reader (`pack.bad_axes`); the model
    // is then measured in glTF's own convention, so its other faults still show.
    let axes = Axes::parse(front, up).unwrap_or(Axes {
        front: crate::asset_gltf::Axis { index: 2, sign: 1 },
        up: crate::asset_gltf::Axis { index: 1, sign: 1 },
    });
    let mut facts = ModelFacts {
        file: file.to_string(),
        detail: detail.to_vec(),
        front: front.to_string(),
        up: up.to_string(),
        pivot: pivot.to_string(),
        rows: axes.rows(),
        pivot_mm: [0; 3],
        detail_triangles: Vec::new(),
        files: BTreeMap::new(),
        copies: Vec::new(),
        measured: false,
        scale,
    };
    let mut out = Measured {
        facts: facts.clone(),
        raw: None,
        triangles: 0,
        plan: None,
        mounts: Vec::new(),
    };
    let main = match open_written(&root.root, file) {
        Ok((path, parts)) => {
            let r = read_copy(
                root,
                &path,
                &parts,
                axes,
                want_plan,
                scale,
                &mut facts.files,
                f,
            );
            if let Some((_, d)) = &r {
                facts.copies.push((parts.join("/"), d.clone()));
            }
            r.map(|(c, _)| c)
        }
        Err(why) => {
            f.at_key("file", why);
            None
        }
    };
    let mut details: Vec<Option<Copy>> = Vec::new();
    for (k, d) in detail.iter().enumerate() {
        match open_written(&root.root, d) {
            Ok((path, parts)) => {
                let r = read_copy(root, &path, &parts, axes, false, scale, &mut facts.files, f);
                if let Some((_, dg)) = &r {
                    facts.copies.push((parts.join("/"), dg.clone()));
                }
                details.push(r.map(|(c, _)| c));
            }
            Err((code, message, hint)) => {
                f.at_key(
                    "detail",
                    (code, format!("`detail` index {k}: {message}"), hint),
                );
                details.push(None);
            }
        }
    }
    let Some(main) = main else {
        out.facts = facts;
        return out;
    };
    if details.iter().any(Option::is_none) {
        out.facts = facts;
        return out;
    }
    let raw = main.measure.raw;
    let p = pivot_of(pivot, &raw);
    facts.pivot_mm = p;
    if main.untextured {
        f.diags.push(text::diag(
            codes::asset::UNTEXTURED,
            f.subject(None),
            (f.site_of)("file"),
            "no material of the model has a base colour texture",
            "bind the texture in the model if it should have one",
        ));
    }
    for (k, c) in details.into_iter().flatten().enumerate() {
        facts.detail_triangles.push(c.measure.triangles);
        let cr = c.measure.raw;
        // 3.3: the copy moved by the main model's pivot correction, compared in plan.
        if cr[0] < raw[0] || cr[1] < raw[1] || cr[3] > raw[3] || cr[4] > raw[4] {
            f.diags.push(
                text::diag(
                    codes::asset::DETAIL_BOUNDS,
                    f.subject(Some("detail")),
                    (f.site_of)("detail"),
                    &format!(
                        "the lower-detail copy `{}` reaches outside the main model's plan bounds",
                        detail[k]
                    ),
                    "re-export the copy inside the main model",
                )
                .with("index", k as i64),
            );
        }
        let cp = pivot_of(pivot, &cr);
        if (0..3).any(|i| (cp[i] - p[i]).abs() > 1) {
            f.diags.push(
                text::diag(
                    codes::asset::DETAIL_PIVOT,
                    f.subject(Some("detail")),
                    (f.site_of)("detail"),
                    &format!(
                        "the lower-detail copy `{}` puts its pivot elsewhere than the main model, by more than 1 mm",
                        detail[k]
                    ),
                    "re-export the copy with the main model's origin",
                )
                .with("index", k as i64),
            );
        }
    }
    facts.measured = true;
    out.facts = facts;
    out.raw = Some(raw);
    out.triangles = main.measure.triangles;
    out.mounts = main.measure.mounts.clone();
    // 3.4: traced only for an asset that asks (`plan = true`, or `footprint = "plan"`).
    out.plan = if want_plan {
        crate::asset_gltf::plan_rings(
            &main.measure.plan_tris,
            [raw[0], raw[1], raw[2]],
            [raw[3], raw[4], raw[5]],
        )
    } else {
        None
    };
    out
}
