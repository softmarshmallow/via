//! `via pack check PACK`, `via pack list PACK` and `via pack accept PACK` (`assets.md` 8, 4.3): a
//! pack read without a world. `check` prints the pack's diagnostics and writes nothing; `list`
//! prints each asset's resolved facts as JSON Lines on standard output, with each asset's instance
//! count in a build directory when asked; `accept` writes the facts file.

use std::path::Path;

use via_diag::{codes, Diag, NoWorld, Record, Site, Subject};
use via_model::pack::{self, Asset, Pack};

use crate::json::{obj, J};
use crate::{txt, usage};

/// Reads the pack at a command-line path, as a build reads the pack it finds (`assets.md` 2).
fn read(path: &Path, at: u32) -> Result<(Pack, Vec<Record>), Vec<Record>> {
    if !path.is_file() {
        return Err(vec![Diag::new(
            codes::pack::NOT_FOUND,
            Subject::Cli,
            Site::Argument(at),
            txt(format!("the pack `{}` is not a file", path.display())),
            via_diag::text!("give the path of a pack file"),
        )
        .emit(&NoWorld)]);
    }
    let display = format!(
        "pack:{}",
        path.file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    );
    let (p, diags) = pack::read(path, &display);
    let records: Vec<Record> = diags.iter().map(|d| d.emit(&NoWorld)).collect();
    match p {
        Some(mut p) => {
            // A hint names the pack as the command line gave it.
            p.command = path.to_string_lossy().replace('\\', "/");
            Ok((p, records))
        }
        None => Err(records),
    }
}

/// `via pack check PACK` (8.1): the pack-wide faults, then every asset's own, the facts file
/// compared, stale entries.
pub fn check(path: &Path, at: u32) -> Vec<Record> {
    match read(path, at) {
        Ok((p, mut records)) => {
            records.extend(p.check_diagnostics().iter().map(|d| d.emit(&NoWorld)));
            records
        }
        Err(records) => records,
    }
}

/// `via pack accept PACK` (4.3): measures and resolves every model asset and writes the facts
/// file, or refuses and writes nothing. Prints the ids whose entries changed, one per line.
pub fn accept(path: &Path, at: u32) -> Vec<Record> {
    let (p, mut records) = match read(path, at) {
        Ok(x) => x,
        Err(mut records) => {
            let mut codes_seen: Vec<String> = records
                .iter()
                .filter(|r| r.severity() == via_diag::Severity::Error)
                .map(|r| r.code().to_string())
                .collect();
            codes_seen.sort();
            codes_seen.dedup();
            records.push(
                Diag::new(
                    codes::pack::ACCEPT_REFUSED,
                    Subject::Cli,
                    Site::Argument(at),
                    txt(format!(
                        "the pack has errors ({}); nothing was written",
                        codes_seen.join(", ")
                    )),
                    via_diag::text!("fix the listed errors, then accept again"),
                )
                .with("codes", via_diag::Value::texts(codes_seen))
                .emit(&NoWorld),
            );
            return records;
        }
    };
    records.extend(p.asset_diags.values().flatten().map(|d| d.emit(&NoWorld)));
    let errors: std::collections::BTreeSet<String> = records
        .iter()
        .filter(|r| r.severity() == via_diag::Severity::Error)
        .map(|r| r.code().to_string())
        .collect();
    match via_model::asset_facts::accept(&p, &errors) {
        Ok(done) => {
            let mut out = String::new();
            for id in done.changed {
                out.push_str(&id);
                out.push('\n');
            }
            print!("{out}");
        }
        Err(d) => records.push(d.emit(&NoWorld)),
    }
    records
}

fn mm(v: i64) -> J {
    J::mm(v)
}

fn xy(p: (i64, i64)) -> J {
    J::Arr(vec![mm(p.0), mm(p.1)])
}

fn dbl(v: f64) -> J {
    J::dbl(v).unwrap_or(J::Null)
}

/// One asset's line (8.2): `id`, `source`, `taxon`, `tags` and every resolved fact (2.4, 2.7).
fn line(a: &Asset, instances: Option<usize>) -> String {
    let mut mounts = J::obj();
    for (n, (at, face)) in &a.mounts {
        let mut o = obj([("at", xy(*at)), ("face", J::int(*face))]);
        if let (Some(c), J::Obj(m)) = (a.connectors.get(n), &mut o) {
            m.insert("connector".into(), J::s(c.clone()));
        }
        mounts.insert(n.clone(), o);
    }
    let mut m = J::obj();
    m.insert("id".into(), J::s(a.id.clone()));
    m.insert("source".into(), J::s(a.source.clone()));
    m.insert("taxon".into(), J::opt(a.taxon.as_deref()));
    m.insert("tags".into(), J::strs(a.tags.iter().map(String::as_str)));
    m.insert("primitive".into(), J::opt(a.primitive.as_deref()));
    m.insert(
        "size".into(),
        if a.model.is_some() {
            J::Null
        } else {
            J::Arr(a.size.iter().map(|v| mm(*v)).collect())
        },
    );
    m.insert(
        "bounds".into(),
        J::Arr(a.bounds.iter().map(|v| mm(*v)).collect()),
    );
    m.insert("triangles".into(), J::Num(a.triangles.to_string()));
    m.insert(
        "origin".into(),
        J::Arr(a.origin.iter().map(|v| mm(*v)).collect()),
    );
    m.insert("collision".into(), J::s(a.collision.clone()));
    if let Some(md) = &a.model {
        m.insert("file".into(), J::s(md.file.clone()));
        m.insert(
            "detail".into(),
            J::strs(md.detail.iter().map(String::as_str)),
        );
        m.insert(
            "detail_triangles".into(),
            J::Arr(
                md.detail_triangles
                    .iter()
                    .map(|t| J::Num(t.to_string()))
                    .collect(),
            ),
        );
        m.insert("front".into(), J::s(md.front.clone()));
        m.insert("up".into(), J::s(md.up.clone()));
        m.insert("pivot".into(), J::s(md.pivot.clone()));
        let mut files = J::obj();
        for (k, v) in &md.files {
            files.insert(k.clone(), J::s(v.clone()));
        }
        m.insert("files".into(), J::Obj(files));
    }
    if let Some(n) = instances {
        m.insert("instances".into(), J::count(n));
    }
    m.insert(
        "footprint".into(),
        J::Arr(a.footprint.iter().map(|p| xy(*p)).collect()),
    );
    m.insert("solidity".into(), J::s(a.solidity()));
    m.insert("sink".into(), dbl(a.sink));
    m.insert("max_relief".into(), a.max_relief.map_or(J::Null, dbl));
    m.insert(
        "colour".into(),
        J::Arr(a.colour.iter().map(|c| dbl(*c)).collect()),
    );
    m.insert("mounts".into(), J::Obj(mounts));
    J::Obj(m).canonical()
}

/// How many instances of each asset a build directory's instance list holds (8.2 `--build`),
/// read from its files without building; `None` when the folder is no build directory.
fn usage_in(dir: &Path) -> Option<std::collections::BTreeMap<String, usize>> {
    let bytes = std::fs::read(dir.join("manifest.json")).ok()?;
    let manifest = crate::json::parse(std::str::from_utf8(&bytes).ok()?).ok()?;
    let read = |p: &str| -> Option<Vec<u8>> {
        // Only a plain path inside the directory is read.
        let safe = !p.is_empty()
            && Path::new(p)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)));
        safe.then(|| std::fs::read(dir.join(p)).ok()).flatten()
    };
    let snap = crate::filebase::snapshot(&read, &manifest)?;
    let mut out = std::collections::BTreeMap::new();
    for i in snap.build.instances.values() {
        *out.entry(i.asset.clone()).or_insert(0) += 1;
    }
    Some(out)
}

/// `via pack list PACK [--taxon @<path>] [--tag <tag>] [--build DIR]` (8.2).
pub fn list(
    path: &Path,
    at: u32,
    taxon: Option<(u32, String)>,
    tag: Option<(u32, String)>,
    build: Option<(u32, String)>,
) -> Vec<Record> {
    let (p, mut records) = match read(path, at) {
        Ok(x) => x,
        Err(r) => return r,
    };
    let usage_counts = match &build {
        None => None,
        Some((ba, dir)) => match usage_in(Path::new(dir)) {
            Some(u) => Some(u),
            None => {
                records.push(
                    usage(
                        *ba,
                        format!("`--build {dir}` is not a build directory"),
                        "give the folder a build wrote, the one holding `manifest.json`".into(),
                    )
                    .emit(&NoWorld),
                );
                return records;
            }
        },
    };
    // Each asset's own faults (a moved pivot, a point footprint, an unreadable model): a listed
    // fact is only as good as its measuring.
    records.extend(p.asset_diags.values().flatten().map(|d| d.emit(&NoWorld)));
    if records
        .iter()
        .any(|r| r.severity() == via_diag::Severity::Error)
    {
        return records;
    }
    let mut keep: Vec<&Asset> = p.assets.values().collect();
    if let Some((ta, t)) = &taxon {
        let Some(t) = t.strip_prefix('@') else {
            records.push(
                usage(
                    *ta,
                    format!("`--taxon {t}` is not a taxon: write `@<path>`"),
                    "write `--taxon @<path>`".into(),
                )
                .emit(&NoWorld),
            );
            return records;
        };
        if !p.taxa.contains(t) {
            let written: Vec<String> = p.taxa.iter().map(|x| format!("@{x}")).collect();
            let near =
                via_diag::nearest::nearest(&format!("@{t}"), written.iter().map(String::as_str));
            let mut d = Diag::new(
                codes::pack::UNKNOWN_TAXON,
                Subject::Cli,
                Site::Argument(*ta),
                txt(format!("the taxon `@{t}` is not declared in the pack")),
                txt(near.hint("declare `[taxon.\"<path>\"]` in the pack")),
            );
            if let Some(n) = near.found() {
                d = d.with("nearest", n);
            }
            records.push(d.emit(&NoWorld));
            return records;
        }
        let below = format!("{t}/");
        keep.retain(|a| {
            a.taxon
                .as_deref()
                .is_some_and(|x| x == t || x.starts_with(&below))
        });
        if keep.is_empty() {
            let with: Vec<String> = p
                .taxa
                .iter()
                .filter(|x| !p.select(x).is_empty())
                .map(|x| format!("@{x}"))
                .collect();
            records.push(empty(*ta, &format!("@{t}"), &with));
            return records;
        }
    }
    if let Some((ga, g)) = &tag {
        keep.retain(|a| a.tags.iter().any(|x| x == g));
        if keep.is_empty() {
            let mut with: Vec<String> = p.assets.values().flat_map(|a| a.tags.clone()).collect();
            with.sort();
            with.dedup();
            records.push(empty(*ga, g, &with));
            return records;
        }
    }
    keep.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    let mut out = String::new();
    for a in keep {
        let n = usage_counts
            .as_ref()
            .map(|u| u.get(&a.id).copied().unwrap_or(0));
        out.push_str(&line(a, n));
        out.push('\n');
    }
    print!("{out}");
    records
}

/// `asset.empty_selection`: a declared taxon or a tag that keeps no asset, with the nearest one
/// that has assets.
fn empty(at: u32, what: &str, with: &[String]) -> Record {
    let near = via_diag::nearest::nearest(what, with.iter().map(String::as_str));
    let mut d = Diag::new(
        codes::asset::EMPTY_SELECTION,
        Subject::Cli,
        Site::Argument(at),
        txt(format!("`{what}` keeps no asset of the pack")),
        txt(near.hint("name a taxon or a tag that has assets")),
    );
    if let Some(n) = near.found() {
        d = d.with("nearest", n);
    }
    d.emit(&NoWorld)
}
