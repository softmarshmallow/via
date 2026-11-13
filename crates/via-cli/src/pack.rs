//! `via pack check PACK` and `via pack list PACK` (`assets.md` 8): a pack read without a world.
//! `check` prints the pack's diagnostics and writes nothing; `list` prints each asset's resolved
//! facts as JSON Lines on standard output.

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
        Some(p) => Ok((p, records)),
        None => Err(records),
    }
}

/// `via pack check PACK` (8.1).
pub fn check(path: &Path, at: u32) -> Vec<Record> {
    match read(path, at) {
        Ok((_, records)) | Err(records) => records,
    }
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
fn line(a: &Asset) -> String {
    let mut mounts = J::obj();
    for (n, (at, face)) in &a.mounts {
        mounts.insert(n.clone(), obj([("at", xy(*at)), ("face", J::int(*face))]));
    }
    let mut m = J::obj();
    m.insert("id".into(), J::s(a.id.clone()));
    m.insert("source".into(), J::s(a.source.clone()));
    m.insert("taxon".into(), J::opt(a.taxon.as_deref()));
    m.insert("tags".into(), J::strs(a.tags.iter().map(String::as_str)));
    m.insert("primitive".into(), J::opt(a.primitive.as_deref()));
    m.insert(
        "size".into(),
        J::Arr(a.size.iter().map(|v| mm(*v)).collect()),
    );
    m.insert(
        "bounds".into(),
        J::Arr(a.bounds.iter().map(|v| mm(*v)).collect()),
    );
    m.insert("triangles".into(), J::Num(a.triangles.to_string()));
    m.insert("origin".into(), J::Arr(vec![mm(0), mm(0), mm(0)]));
    m.insert(
        "footprint".into(),
        J::Arr(a.footprint.iter().map(|p| xy(*p)).collect()),
    );
    m.insert(
        "solidity".into(),
        J::s(if a.solid { "solid" } else { "none" }),
    );
    m.insert("sink".into(), dbl(a.sink));
    m.insert("max_relief".into(), a.max_relief.map_or(J::Null, dbl));
    m.insert(
        "colour".into(),
        J::Arr(a.colour.iter().map(|c| dbl(*c)).collect()),
    );
    m.insert("mounts".into(), J::Obj(mounts));
    J::Obj(m).canonical()
}

/// `via pack list PACK [--taxon @<path>] [--tag <tag>]` (8.2).
pub fn list(
    path: &Path,
    at: u32,
    taxon: Option<(u32, String)>,
    tag: Option<(u32, String)>,
) -> Vec<Record> {
    let (p, mut records) = match read(path, at) {
        Ok(x) => x,
        Err(r) => return r,
    };
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
        out.push_str(&line(a));
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
