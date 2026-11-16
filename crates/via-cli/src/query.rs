//! `via query WORLD surfaces_at|sample|relief|meetings --in FILE --out FILE` (`surfaces.md` 7): one
//! surface question over the whole world. It builds the world with its changes in memory, runs no
//! lint or export step, and writes no build directory, run report or `.previous`. Both files are
//! canonical JSON: one object whose keys are column names, each a list.

use std::path::PathBuf;

use via_diag::nearest::nearest;
use via_diag::{codes, Diag, NoWorld, Record, Site, Subject};
use via_doc::{ChangeRequest, LoadOptions};
use via_geom::{Region, P};
use via_model::query::{meetings, Terrain};

use crate::json::{parse, J};
use crate::{txt, usage};

/// The questions.
pub const QUESTIONS: [&str; 4] = ["meetings", "relief", "sample", "surfaces_at"];

pub struct QueryArgs {
    pub world: PathBuf,
    pub world_arg: u32,
    pub question: (String, u32),
    pub input: Option<(PathBuf, u32)>,
    pub out: (PathBuf, u32),
    pub changes: Vec<ChangeRequest>,
    pub lib: Vec<PathBuf>,
}

fn int_list(j: &J, key: &str) -> Option<Vec<i64>> {
    j.get(key)?.as_arr()?.iter().map(J::as_i64).collect()
}

fn dbl(v: f64) -> J {
    J::dbl(v).unwrap_or(J::Null)
}

/// A ring of `[x, y]` integer millimetres, read as one region.
fn ring(j: &J) -> Option<Region> {
    let pts: Option<Vec<P>> = j
        .as_arr()?
        .iter()
        .map(|p| {
            let a = p.as_arr()?;
            (a.len() == 2).then(|| Some(P::new(a[0].as_i64()?, a[1].as_i64()?)))?
        })
        .collect();
    let pts = pts?;
    if pts.len() < 3 {
        return Some(Region::default());
    }
    let mut a2: i128 = 0;
    for k in 0..pts.len() {
        let (p, q) = (pts[k], pts[(k + 1) % pts.len()]);
        a2 += i128::from(p.x) * i128::from(q.y) - i128::from(q.x) * i128::from(p.y);
    }
    if a2 == 0 {
        return Some(Region::default());
    }
    let mut pts = pts;
    if a2 < 0 {
        pts.reverse();
    }
    Some(Region::from_ring(pts))
}

fn obj_of(cols: Vec<(&str, Vec<J>)>) -> String {
    let mut o = J::obj();
    for (k, v) in cols {
        o.insert(k.into(), J::Arr(v));
    }
    let mut s = J::Obj(o).canonical();
    s.push('\n');
    s
}

/// Runs the command.
pub fn query(a: &QueryArgs) -> Vec<Record> {
    let (q, qa) = (&a.question.0, a.question.1);
    if !QUESTIONS.contains(&q.as_str()) {
        let near = nearest(q, QUESTIONS);
        let mut d = usage(
            qa,
            format!("`{q}` is not a surface question"),
            near.hint("ask `surfaces_at`, `sample`, `relief` or `meetings`"),
        );
        if let Some(n) = near.found() {
            d = d.with("nearest", n);
        }
        return vec![d.emit(&NoWorld)];
    }
    let input = match (&a.input, q.as_str()) {
        (Some((_, at)), "meetings") => {
            return vec![usage(
                *at,
                "`meetings` takes no `--in`".into(),
                "remove `--in`".into(),
            )
            .emit(&NoWorld)];
        }
        (None, "meetings") => None,
        (None, _) => {
            return vec![usage(
                qa,
                format!("`{q}` needs `--in FILE`"),
                "write `--in FILE`".into(),
            )
            .emit(&NoWorld)];
        }
        (Some((p, at)), _) => {
            let text = std::fs::read_to_string(p).ok();
            match text.as_deref().map(parse) {
                Some(Ok(j @ J::Obj(_))) => Some((j, *at)),
                _ => {
                    return vec![usage(
                        *at,
                        format!("`{}` is not one JSON object", p.display()),
                        "write the columns as one JSON object (surfaces.md 7)".into(),
                    )
                    .emit(&NoWorld)];
                }
            }
        }
    };
    let opts = LoadOptions::from_env(a.lib.clone(), a.changes.clone());
    let l = via_doc::load(&a.world, a.world_arg, &opts);
    let mut records = l.records;
    let Some(world) = l.world else {
        return records;
    };
    let b = via_model::build(
        &world,
        &via_model::Options::from_load(&a.world, &opts),
        None,
    );
    for d in &b.diagnostics {
        if *d.code() != via_diag::DiagCode::Core(codes::identity::NO_PREVIOUS) {
            records.push(d.emit(&world));
        }
    }
    if records.iter().any(Record::fails) {
        return records;
    }
    let Some(terrain) = Terrain::of(&b) else {
        records.push(
            usage(
                a.world_arg,
                "the world has no terrain to ask".into(),
                "declare one field with `role = \"surface\"`".into(),
            )
            .emit(&NoWorld),
        );
        return records;
    };
    let bad = |at: u32, what: &str| {
        usage(
            at,
            format!("the input's `{what}` is missing or not of its form"),
            "see surfaces.md 7 for the columns".into(),
        )
        .emit(&NoWorld)
    };
    let surface_of = |j: &J, at: u32| -> Result<(), Record> {
        match j.get("surface").and_then(J::as_str) {
            Some(s) if s == terrain.name => Ok(()),
            Some(s) => {
                let near = nearest(s, [terrain.name]);
                let mut d = usage(
                    at,
                    format!("`{s}` is not a surface of this world"),
                    near.hint("in M1 the one surface is the terrain"),
                );
                if let Some(n) = near.found() {
                    d = d.with("nearest", n);
                }
                Err(d.emit(&NoWorld))
            }
            None => Err(bad(at, "surface")),
        }
    };
    let points = |j: &J, at: u32| -> Result<Vec<(i64, i64)>, Record> {
        let (Some(xs), Some(ys)) = (int_list(j, "x"), int_list(j, "y")) else {
            return Err(bad(at, "x` or `y"));
        };
        if xs.len() != ys.len() {
            return Err(bad(at, "x` and `y` of one length, and"));
        }
        Ok(xs.into_iter().zip(ys).collect())
    };
    let text = match (q.as_str(), &input) {
        ("surfaces_at", Some((j, at))) => {
            let pts = match points(j, *at) {
                Ok(p) => p,
                Err(r) => return vec![r],
            };
            let mut rows: Vec<(usize, &str, f64)> = terrain.surfaces_at(&pts);
            // Kind surfaces (2.1, M3): every one whose bound holds the point.
            for (k, (x, y)) in pts.iter().enumerate() {
                if !terrain.inside(*x, *y) {
                    continue;
                }
                for s in b.surfaces.values() {
                    // A `base` that fails at a point fails the whole call (2.7).
                    match s.z((*x, *y)) {
                        Ok(Some(z)) => rows.push((k, s.place.as_str(), z)),
                        Ok(None) => {}
                        Err(e) => return vec![s.fault(&e).emit(&NoWorld)],
                    }
                }
            }
            rows.sort_by(|a, c| {
                a.0.cmp(&c.0)
                    .then(a.2.total_cmp(&c.2))
                    .then(a.1.as_bytes().cmp(c.1.as_bytes()))
            });
            obj_of(vec![
                ("point", rows.iter().map(|r| J::count(r.0)).collect()),
                ("surface", rows.iter().map(|r| J::s(r.1)).collect()),
                ("z", rows.iter().map(|r| dbl(r.2)).collect()),
            ])
        }
        ("sample", Some((j, at))) => {
            if let Err(r) = surface_of(j, *at) {
                return vec![r];
            }
            let pts = match points(j, *at) {
                Ok(p) => p,
                Err(r) => return vec![r],
            };
            let s: Vec<_> = pts.iter().map(|(x, y)| terrain.sample(*x, *y)).collect();
            obj_of(vec![
                ("gx", s.iter().map(|r| dbl(r.gx)).collect()),
                ("gy", s.iter().map(|r| dbl(r.gy)).collect()),
                ("inside", s.iter().map(|r| J::Bool(r.inside)).collect()),
                ("z", s.iter().map(|r| dbl(r.z)).collect()),
            ])
        }
        ("relief", Some((j, at))) => {
            if let Err(r) = surface_of(j, *at) {
                return vec![r];
            }
            let Some(rs) = j.get("regions").and_then(J::as_arr) else {
                return vec![bad(*at, "regions")];
            };
            let mut out = Vec::new();
            for (k, r) in rs.iter().enumerate() {
                let Some(region) = ring(r) else {
                    return vec![bad(*at, "regions")];
                };
                match terrain.relief(&region, k) {
                    Ok(x) => out.push(x),
                    Err(e) => {
                        records.push(
                            Diag::new(
                                codes::surface::EMPTY_REGION,
                                Subject::Cli,
                                Site::Argument(*at),
                                txt(format!(
                                    "region {} given to `relief` has zero area",
                                    e.index
                                )),
                                via_diag::text!("give a region with area"),
                            )
                            .emit(&NoWorld),
                        );
                        return records;
                    }
                }
            }
            obj_of(vec![
                ("high", out.iter().map(|r| dbl(r.high)).collect()),
                ("low", out.iter().map(|r| dbl(r.low)).collect()),
                ("outside", out.iter().map(|r| dbl(r.outside)).collect()),
            ])
        }
        _ => {
            let m = meetings(&b);
            obj_of(vec![
                ("a", m.iter().map(|r| J::s(r.a.clone())).collect()),
                ("b", m.iter().map(|r| J::s(r.b.clone())).collect()),
                (
                    "connector_a",
                    m.iter().map(|r| J::opt(r.connector_a.as_deref())).collect(),
                ),
                (
                    "connector_b",
                    m.iter().map(|r| J::opt(r.connector_b.as_deref())).collect(),
                ),
                (
                    "face_a",
                    m.iter().map(|r| r.face_a.map_or(J::Null, J::int)).collect(),
                ),
                (
                    "face_b",
                    m.iter().map(|r| r.face_b.map_or(J::Null, J::int)).collect(),
                ),
                ("gap", m.iter().map(|r| J::int(r.gap)).collect()),
                (
                    "turn",
                    m.iter().map(|r| r.turn.map_or(J::Null, J::int)).collect(),
                ),
                ("x", m.iter().map(|r| J::int(r.at.0)).collect()),
                ("y", m.iter().map(|r| J::int(r.at.1)).collect()),
                ("z_a", m.iter().map(|r| dbl(r.z_a)).collect()),
                ("z_b", m.iter().map(|r| dbl(r.z_b)).collect()),
            ])
        }
    };
    let (out, oa) = (&a.out.0, a.out.1);
    if let Err(e) = std::fs::write(out, text) {
        records.push(
            Diag::new(
                codes::build::WRITE_FAILED,
                Subject::Cli,
                Site::Argument(oa),
                txt(format!("cannot write `{}`: {e}", out.display())),
                via_diag::text!("free space, or fix the folder's permissions"),
            )
            .emit(&NoWorld),
        );
    }
    records
}
