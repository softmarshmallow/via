//! A baseline read from a build directory's files (`build.md` 11.2, `--against DIR`): the
//! manifest, `features.json`, the instance columns and the field files, read back into the shape
//! the change report compares. It holds what the files hold and nothing more: no statements, no
//! lines, no dependencies, so a report against it has `causes: null` (`graph.md` 6.4).
//!
//! The build being reported is read back the same way from the files it is about to write, so
//! both sides of the comparison have passed through one reading, and a difference is a difference
//! a reader of the two directories sees.

use std::collections::BTreeMap;

use via_model::{Build, FeatureOut, FieldOut, FootprintOut, InstanceOut, Tier};
use via_num::Grid;

use crate::json::{parse, J};
use crate::npy::{self, Descr};
use crate::snap::Snapshot;

/// Millimetres from a length JSON writes in metres.
fn mm(j: &J) -> Option<i64> {
    match j {
        J::Num(n) => via_num::parse_length(n).ok(),
        _ => None,
    }
}

fn xy(j: &J) -> Option<(i64, i64)> {
    let a = j.as_arr()?;
    Some((mm(a.first()?)?, mm(a.get(1)?)?))
}

fn ring(j: &J) -> Vec<(i64, i64)> {
    j.as_arr().unwrap_or(&[]).iter().filter_map(xy).collect()
}

/// An array file's elements, as `f32` (`<f4`), `i32` widened (`<i4`) or bytes (`|u1`), with its
/// shape; `None` when it is not an array of the subset.
enum Arr {
    F4(Vec<f32>),
    I4(Vec<i64>),
    U1(Vec<u8>),
}

fn array(bytes: &[u8]) -> Option<(Vec<usize>, Arr)> {
    let h = npy::parse(bytes).ok()?;
    let data = &bytes[h.data_offset..];
    let a = match h.descr {
        Descr::F4 => Arr::F4(
            data.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        ),
        Descr::I4 => Arr::I4(
            data.chunks_exact(4)
                .map(|c| i64::from(i32::from_le_bytes([c[0], c[1], c[2], c[3]])))
                .collect(),
        ),
        Descr::U1 => Arr::U1(data.to_vec()),
    };
    Some((h.shape, a))
}

fn f32s(bytes: Option<Vec<u8>>) -> Vec<f32> {
    match bytes.as_deref().and_then(array) {
        Some((_, Arr::F4(v))) => v,
        _ => Vec::new(),
    }
}

fn i64s(bytes: Option<Vec<u8>>) -> Vec<i64> {
    match bytes.as_deref().and_then(array) {
        Some((_, Arr::I4(v))) => v,
        _ => Vec::new(),
    }
}

/// An attribute's value as text, the form the build keeps it in (`out::attr_json` reads it back).
fn attr_text(v: &J) -> String {
    match v {
        J::Bool(b) => b.to_string(),
        J::Num(n) => n.clone(),
        J::Str(s) => s.clone(),
        other => other.canonical(),
    }
}

/// A fact's value as text, the form `out::fact_json` reads back.
fn fact_text(v: &J) -> String {
    // As the model records a fact: JSON text (`build.md` 11.2).
    v.canonical()
}

/// The facts of an asset the manifest writes (`assets.md` 7).
const FACTS: [&str; 6] = [
    "bounds",
    "footprint",
    "mounts",
    "origin",
    "size",
    "solidity",
];

/// The baseline in a directory's files: `file(path)` gives a file's bytes, `manifest` its
/// manifest (or the world keys a build is about to write). `None` when the manifest has no frame.
pub fn snapshot(file: &dyn Fn(&str) -> Option<Vec<u8>>, manifest: &J) -> Option<Snapshot> {
    let frame = manifest.get("frame")?;
    let e = frame.get("extent")?.as_arr()?;
    let extent = [
        mm(e.first()?)?,
        mm(e.get(1)?)?,
        mm(e.get(2)?)?,
        mm(e.get(3)?)?,
    ];
    let grid = Grid::new(extent, mm(frame.get("resolution")?)?).ok()?;
    let (nx, ny) = (
        usize::try_from(grid.nx).ok()?,
        usize::try_from(grid.ny).ok()?,
    );
    let mut b = Build {
        grid: Some(grid),
        ..Build::default()
    };
    let mut homes = BTreeMap::new();
    let chunks: &[J] = manifest.get("chunks").and_then(J::as_arr).unwrap_or(&[]);
    // Fields (`build.md` 6): a scalar field's one file; a weights field's planes per chunk.
    if let Some(J::Obj(fields)) = manifest.get("fields") {
        for (name, e) in fields {
            let strs = |k: &str| -> Vec<String> {
                e.get(k)
                    .and_then(J::as_arr)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|t| t.as_str().map(str::to_string))
                    .collect()
            };
            let mut f = FieldOut {
                role: e.get("role").and_then(J::as_str).map(str::to_string),
                unit: e.get("unit").and_then(J::as_str).map(str::to_string),
                tags: strs("tags"),
                values: None,
                weights: BTreeMap::new(),
            };
            if e.get("type").and_then(J::as_str) == Some("scalar") {
                let v = f32s(e.get("file").and_then(J::as_str).and_then(file));
                f.values = Some(if v.len() == nx * ny { v } else { Vec::new() });
            } else {
                for c in chunks {
                    let Some(w) = c.get("weights").and_then(|w| w.get(name)) else {
                        continue;
                    };
                    let Some(path) = w.get("file").and_then(J::as_str) else {
                        continue;
                    };
                    let posts: Vec<usize> = c
                        .get("posts")
                        .and_then(J::as_arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|p| p.as_i64().and_then(|v| usize::try_from(v).ok()))
                        .collect();
                    let [i0, j0, i1, j1] = posts[..] else {
                        continue;
                    };
                    let Some((shape, Arr::U1(data))) = file(path).as_deref().and_then(array) else {
                        continue;
                    };
                    let (rows, cols) = (j1 + 1 - j0, i1 + 1 - i0);
                    if shape.len() != 3 || shape[1] != rows || shape[2] != cols {
                        continue;
                    }
                    let mats: Vec<String> = w
                        .get("materials")
                        .and_then(J::as_arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|t| t.as_str().map(str::to_string))
                        .collect();
                    for (k, m) in mats.iter().enumerate() {
                        let plane = f
                            .weights
                            .entry(m.clone())
                            .or_insert_with(|| vec![0.0; nx * ny]);
                        for j in 0..rows {
                            for i in 0..cols {
                                let at = (k * rows + j) * cols + i;
                                let post = (j0 + j) * nx + i0 + i;
                                if let (Some(v), Some(slot)) = (data.get(at), plane.get_mut(post)) {
                                    *slot = f32::from(*v);
                                }
                            }
                        }
                    }
                }
            }
            b.fields.insert(name.clone(), f);
        }
    }
    // Features and footprints (`build.md` 9.1).
    let feats = manifest
        .get("features")
        .and_then(J::as_str)
        .and_then(file)
        .and_then(|bytes| parse(std::str::from_utf8(&bytes).ok()?).ok())
        .unwrap_or(J::Null);
    for f in feats.get("features").and_then(J::as_arr).unwrap_or(&[]) {
        let Some(name) = f.get("name").and_then(J::as_str) else {
            continue;
        };
        let shape = f.get("shape").and_then(J::as_str).unwrap_or("").to_string();
        let mut points = Vec::new();
        let mut rings = Vec::new();
        match shape.as_str() {
            "point" => points.extend(f.get("at").and_then(xy)),
            "path" => {
                points = f
                    .get("points")
                    .and_then(J::as_arr)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(xy)
                    .collect();
            }
            _ => {
                for pc in f.get("pieces").and_then(J::as_arr).unwrap_or(&[]) {
                    let outer = ring(pc.get("outer").unwrap_or(&J::Null));
                    points.extend(outer.iter().copied());
                    let mut r = vec![outer];
                    for h in pc.get("holes").and_then(J::as_arr).unwrap_or(&[]) {
                        r.push(ring(h));
                    }
                    rings.push(r);
                }
            }
        }
        let mut attrs = BTreeMap::new();
        let mut names = BTreeMap::new();
        if let Some(J::Obj(a)) = f.get("attrs") {
            for (k, v) in a {
                match (v.get("name").and_then(J::as_str), v.get("at").and_then(xy)) {
                    (Some(place), Some(p)) if matches!(v, J::Obj(_)) => {
                        names.insert(k.clone(), (place.to_string(), p));
                    }
                    _ => {
                        attrs.insert(k.clone(), attr_text(v));
                    }
                }
            }
        }
        if let Some(l) = f.get("layer").and_then(J::as_str) {
            homes.insert(format!("feature.{name}"), l.to_string());
        }
        b.features.insert(
            name.to_string(),
            FeatureOut {
                kind: f.get("kind").and_then(J::as_str).map(str::to_string),
                tier: match f.get("authority").and_then(J::as_str) {
                    Some("derived") => Some(Tier::Derived),
                    Some("shaped") => Some(Tier::Shaped),
                    Some("anchored") => Some(Tier::Anchored),
                    _ => None,
                },
                shape,
                points,
                facing: None,
                attrs,
                attr_types: BTreeMap::new(),
                names,
                output: None,
                rings,
            },
        );
    }
    for f in feats.get("footprints").and_then(J::as_arr).unwrap_or(&[]) {
        let Some(name) = f.get("instance").and_then(J::as_str) else {
            continue;
        };
        let outer = f
            .get("pieces")
            .and_then(J::as_arr)
            .and_then(|p| p.first())
            .and_then(|p| p.get("outer"))
            .map(ring)
            .unwrap_or_default();
        if let Some(l) = f.get("layer").and_then(J::as_str) {
            homes.insert(format!("instance.{name}"), l.to_string());
        }
        b.footprints.insert(
            name.to_string(),
            FootprintOut {
                kind: f.get("kind").and_then(J::as_str).unwrap_or("").to_string(),
                ring: outer,
            },
        );
    }
    // Instances (`build.md` 8): the columns, each row's asset from the chunks' runs.
    let inst = manifest.get("instances").unwrap_or(&J::Null);
    let col = |c: &str| inst.get(c).and_then(J::as_str).and_then(file);
    let names: Vec<String> = col("names")
        .and_then(|b| String::from_utf8(b).ok())
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let (xs, ys, faces) = (i64s(col("x")), i64s(col("y")), i64s(col("face")));
    let (zs, scales, tints) = (f32s(col("z")), f32s(col("scale")), f32s(col("tint")));
    let mut asset_of: BTreeMap<usize, String> = BTreeMap::new();
    for c in chunks {
        if let Some(J::Obj(runs)) = c.get("instances") {
            for (a, run) in runs {
                let r = run.as_arr().unwrap_or(&[]);
                let (Some(first), Some(n)) = (
                    r.first()
                        .and_then(J::as_i64)
                        .and_then(|v| usize::try_from(v).ok()),
                    r.get(1)
                        .and_then(J::as_i64)
                        .and_then(|v| usize::try_from(v).ok()),
                ) else {
                    continue;
                };
                for row in first..first.saturating_add(n) {
                    asset_of.insert(row, a.clone());
                }
            }
        }
    }
    for (row, name) in names.iter().enumerate() {
        let (Some(x), Some(y)) = (xs.get(row), ys.get(row)) else {
            continue;
        };
        b.instances.insert(
            name.clone(),
            InstanceOut {
                maker: name.split_once('#').map(|(m, _)| m.to_string()),
                asset: asset_of.get(&row).cloned().unwrap_or_default(),
                origin: (*x, *y),
                z: f64::from(zs.get(row).copied().unwrap_or(0.0)),
                facing: faces.get(row).copied().unwrap_or(0),
                scale: f64::from(scales.get(row).copied().unwrap_or(1.0)),
                tint: f64::from(tints.get(row).copied().unwrap_or(0.0)),
                footprint: Vec::new(),
            },
        );
    }
    // Asset facts (`assets.md` 7), as the manifest writes them.
    if let Some(J::Obj(assets)) = manifest.get("assets") {
        for (id, a) in assets {
            let facts: BTreeMap<String, String> = FACTS
                .iter()
                .filter_map(|k| Some((k.to_string(), fact_text(a.get(k)?))))
                .collect();
            b.assets.insert(id.clone(), facts);
        }
    }
    Some(Snapshot {
        build: b,
        lines: BTreeMap::new(),
        homes,
    })
}
