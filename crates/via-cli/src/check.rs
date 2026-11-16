//! `via check DIR` (`build.md` 14): reads a build directory alone, writes nothing into it, and
//! reports each failure as a `check.*` error naming the file and where in it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use via_diag::{codes, Code, Diag, Site, Subject, Value};
use via_graph::Digest;

use crate::json::{first_non_canonical, parse, J};
use crate::layout::Layout;
use crate::npy::{self, Descr};
use crate::txt;

fn fault(code: Code, file: &str, message: String, locator: &[(&str, Value)]) -> Diag {
    let mut l = BTreeMap::new();
    for (k, v) in locator {
        l.insert((*k).to_string(), v.clone());
    }
    Diag::new(
        code,
        Subject::World,
        Site::File {
            file: file.to_string(),
            locator: l,
        },
        txt(message),
        via_diag::text!("build again; `via check` never repairs"),
    )
}

/// The `id` of a manifest by section 12: the digest of the manifest less `id`, `run` and
/// `locations`, in canonical JSON.
pub fn id_of(manifest: &J) -> String {
    let mut m = match manifest {
        J::Obj(m) => m.clone(),
        _ => BTreeMap::new(),
    };
    for k in ["id", "run", "locations"] {
        m.remove(k);
    }
    Digest::of(J::Obj(m).canonical().as_bytes()).hex()
}

fn absolute(s: &str) -> bool {
    let b = s.as_bytes();
    s.starts_with('/')
        || s.starts_with('\\')
        || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

/// The members holding an absolute path, each named by its key path (`features[0].name`), the
/// form subjects use: a JSON pointer starts with `/` and would itself read as a path.
fn find_absolute(v: &J, path: &str, out: &mut Vec<String>) {
    match v {
        J::Str(s) if absolute(s) => out.push(path.to_string()),
        J::Arr(a) => {
            for (i, x) in a.iter().enumerate() {
                find_absolute(x, &format!("{path}[{i}]"), out);
            }
        }
        J::Obj(m) => {
            for (k, x) in m {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                find_absolute(x, &p, out);
            }
        }
        _ => {}
    }
}

struct Files<'a> {
    dir: &'a Path,
    cache: BTreeMap<String, Option<Vec<u8>>>,
}

impl Files<'_> {
    fn get(&mut self, p: &str) -> Option<&Vec<u8>> {
        let dir = self.dir;
        self.cache
            .entry(p.to_string())
            .or_insert_with(|| std::fs::read(dir.join(p)).ok())
            .as_ref()
    }
}

/// Every failure of the ten checks.
pub fn check(dir: &Path) -> Vec<Diag> {
    let mut out = Vec::new();
    let mut files = Files {
        dir,
        cache: BTreeMap::new(),
    };
    // 1. The manifest.
    let Some(mbytes) = files.get("manifest.json").cloned() else {
        let mut d = fault(
            codes::check::FORMAT,
            "manifest.json",
            "the directory holds no `manifest.json`".into(),
            &[],
        );
        if dir.join("diagnostics.json").exists() {
            d = d.with("diagnostics", "diagnostics.json");
        }
        return vec![d];
    };
    let manifest = match std::str::from_utf8(&mbytes).ok().map(parse) {
        Some(Ok(m)) => m,
        Some(Err(e)) => {
            return vec![fault(
                codes::check::FORMAT,
                "manifest.json",
                format!(
                    "`manifest.json` is not JSON: {} at byte {}",
                    e.reason, e.offset
                ),
                &[("offset", Value::from(e.offset as i64))],
            )]
        }
        None => {
            return vec![fault(
                codes::check::FORMAT,
                "manifest.json",
                "`manifest.json` is not UTF-8".into(),
                &[],
            )]
        }
    };
    if manifest.get("format").and_then(J::as_str) != Some(crate::out::FORMAT) {
        return vec![fault(
            codes::check::FORMAT,
            "manifest.json",
            format!(
                "`manifest.json` has the format {}, which this reader does not know",
                manifest
                    .get("format")
                    .map_or_else(|| "(none)".to_string(), J::canonical)
            ),
            &[],
        )];
    }
    if let Some(off) = first_non_canonical(&mbytes) {
        out.push(fault(
            codes::check::FORMAT,
            "manifest.json",
            format!("`manifest.json` is not canonical from byte {off}"),
            &[("offset", Value::from(off as i64))],
        ));
    }
    // 2. The id.
    let want = id_of(&manifest);
    match manifest.get("id").and_then(J::as_str) {
        Some(id) if id == want => {}
        got => out.push(
            fault(
                codes::check::ID,
                "manifest.json",
                format!(
                    "`id` is {}; the digest recomputed by section 12 is {want}",
                    got.unwrap_or("missing")
                ),
                &[("key", Value::from("id"))],
            )
            .with("expected", want.clone()),
        ),
    }
    // 3. Listed files.
    let mut listed: BTreeMap<String, &J> = BTreeMap::new();
    if let Some(J::Obj(m)) = manifest.get("files") {
        for (k, v) in m {
            listed.insert(k.clone(), v);
        }
    }
    if let Some(J::Obj(m)) = manifest.get("run").and_then(|r| r.get("reports")) {
        for (k, v) in m {
            listed.insert(k.clone(), v);
        }
    }
    for (p, rec) in &listed {
        let Some(b) = files.get(p).cloned() else {
            out.push(fault(
                codes::check::MISSING,
                p,
                format!("`{p}` is listed and absent"),
                &[],
            ));
            continue;
        };
        let (bytes, digest) = (
            rec.get("bytes").and_then(J::as_i64),
            rec.get("digest").and_then(J::as_str),
        );
        let got = Digest::of(&b).hex();
        if bytes != i64::try_from(b.len()).ok() || digest != Some(got.as_str()) {
            out.push(
                fault(
                    codes::check::DIGEST,
                    p,
                    format!(
                        "`{p}` is {} bytes with digest {got}; its record says {} bytes, digest {}",
                        b.len(),
                        bytes.map_or_else(|| "no".to_string(), |n| n.to_string()),
                        digest.unwrap_or("none")
                    ),
                    &[],
                )
                .with("digest", got),
            );
        }
    }
    for f in crate::dir::listing(dir) {
        if f != "manifest.json" && f != ".previous" && !listed.contains_key(&f) {
            out.push(fault(
                codes::check::UNLISTED,
                &f,
                format!("`{f}` is in the directory and no list names it"),
                &[],
            ));
        }
    }
    // 4 to 9: array files.
    let posts = manifest
        .get("frame")
        .and_then(|f| f.get("posts"))
        .and_then(J::as_arr)
        .map(|a| a.iter().filter_map(J::as_i64).collect::<Vec<_>>())
        .unwrap_or_default();
    let (nx, ny) = (
        usize::try_from(*posts.first().unwrap_or(&0)).unwrap_or(0),
        usize::try_from(*posts.get(1).unwrap_or(&0)).unwrap_or(0),
    );
    let mut arrays: Vec<(String, Descr, Vec<usize>)> = Vec::new();
    if let Some(J::Obj(fs)) = manifest.get("fields") {
        for e in fs.values() {
            if let Some(f) = e.get("file").and_then(J::as_str) {
                arrays.push((f.to_string(), Descr::F4, vec![ny, nx]));
            }
        }
    }
    let chunks: Vec<J> = manifest
        .get("chunks")
        .and_then(J::as_arr)
        .map(<[J]>::to_vec)
        .unwrap_or_default();
    for c in &chunks {
        let p: Vec<i64> = c
            .get("posts")
            .and_then(J::as_arr)
            .map(|a| a.iter().filter_map(J::as_i64).collect())
            .unwrap_or_default();
        if p.len() != 4 {
            continue;
        }
        if let Some(J::Obj(w)) = c.get("weights") {
            for e in w.values() {
                if let Some(f) = e.get("file").and_then(J::as_str) {
                    let planes = e.get("materials").and_then(J::as_arr).map_or(0, <[J]>::len);
                    arrays.push((
                        f.to_string(),
                        Descr::U1,
                        vec![
                            planes,
                            usize::try_from(p[3] - p[1] + 1).unwrap_or(0),
                            usize::try_from(p[2] - p[0] + 1).unwrap_or(0),
                        ],
                    ));
                }
            }
        }
    }
    let count = manifest
        .get("instances")
        .and_then(|i| i.get("count"))
        .and_then(J::as_i64)
        .and_then(|c| usize::try_from(c).ok())
        .unwrap_or(0);
    for (c, d) in [
        ("x", Descr::I4),
        ("y", Descr::I4),
        ("z", Descr::F4),
        ("face", Descr::I4),
        ("scale", Descr::F4),
        ("tint", Descr::F4),
    ] {
        arrays.push((format!("instances/{c}.npy"), d, vec![count]));
    }
    let mut columns: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    for (path, descr, shape) in &arrays {
        let Some(b) = files.get(path).cloned() else {
            continue;
        };
        let h = match npy::parse(&b) {
            Ok(h) => h,
            Err((off, why)) => {
                out.push(fault(
                    codes::check::HEADER,
                    path,
                    format!("`{path}`: {why}"),
                    &[("offset", Value::from(off as i64))],
                ));
                continue;
            }
        };
        if h.descr != *descr || &h.shape != shape {
            out.push(fault(
                codes::check::SHAPE,
                path,
                format!(
                    "`{path}` holds `{}` {:?}; the manifest says `{}` {:?}",
                    h.descr.text(),
                    h.shape,
                    descr.text(),
                    shape
                ),
                &[],
            ));
            continue;
        }
        let data = &b[h.data_offset..];
        match h.descr {
            Descr::F4 => {
                for (i, c) in data.chunks_exact(4).enumerate() {
                    let v = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                    if !v.is_finite() || v.to_bits() == 0x8000_0000 {
                        let what = if v.is_nan() {
                            "NaN"
                        } else if v == f32::INFINITY {
                            "+inf"
                        } else if v == f32::NEG_INFINITY {
                            "-inf"
                        } else {
                            "-0"
                        };
                        let cols = match h.shape.as_slice() {
                            [_, c] => (*c).max(1),
                            _ => 0,
                        };
                        let (place, mut facts) = if cols > 0 {
                            let (row, col) = (i / cols, i % cols);
                            (
                                format!("element {i} (row {row}, column {col})"),
                                vec![
                                    ("row", Value::from(row as i64)),
                                    ("column", Value::from(col as i64)),
                                ],
                            )
                        } else {
                            (format!("element {i}"), Vec::new())
                        };
                        let fix = if what == "-0" {
                            "a build writes 0, never -0"
                        } else {
                            "a build writes only finite values"
                        };
                        facts.push(("index", Value::from(i as i64)));
                        out.push(fault(
                            codes::check::VALUE,
                            path,
                            format!("`{path}`: {place} is {what}; {fix}"),
                            &facts,
                        ));
                        break;
                    }
                }
            }
            Descr::I4 => {
                let v: Vec<i32> = data
                    .chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                columns.insert(path.clone(), v);
            }
            Descr::U1 => {
                // 6: each post sums to 255 or 0.
                let (planes, n) = (shape[0], shape[1] * shape[2]);
                for p in 0..n {
                    let s: u32 = (0..planes).map(|k| u32::from(data[k * n + p])).sum();
                    if s != 255 && s != 0 {
                        out.push(fault(
                            codes::check::WEIGHTS_SUM,
                            path,
                            format!("`{path}`: post {p} sums to {s}, neither 255 nor 0"),
                            &[("post", Value::from(p as i64))],
                        ));
                        break;
                    }
                }
            }
        }
    }
    // 7. Names.
    if let Some(b) = files.get("instances/names.txt").cloned() {
        let text = String::from_utf8_lossy(&b);
        let lines: Vec<&str> = if text.is_empty() {
            Vec::new()
        } else {
            text.strip_suffix('\n')
                .unwrap_or(&text)
                .split('\n')
                .collect()
        };
        let mut seen = BTreeSet::new();
        if lines.len() != count || (!text.is_empty() && !text.ends_with('\n')) {
            out.push(fault(
                codes::check::NAMES,
                "instances/names.txt",
                format!(
                    "`instances/names.txt` has {} lines; `count` is {count}",
                    lines.len()
                ),
                &[],
            ));
        }
        for (i, l) in lines.iter().enumerate() {
            let ok = !l.is_empty()
                && l.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.#".contains(&c));
            if !ok || !seen.insert(*l) {
                out.push(fault(
                    codes::check::NAMES,
                    "instances/names.txt",
                    format!(
                        "`instances/names.txt` line {}: `{l}` is repeated or outside the alphabet",
                        i + 1
                    ),
                    &[("line", Value::from(i as i64 + 1))],
                ));
                break;
            }
        }
    }
    // 8. Facings.
    if let Some(f) = columns.get("instances/face.npy") {
        if let Some(i) = f.iter().position(|v| !(0..360_000).contains(v)) {
            out.push(fault(
                codes::check::FACE,
                "instances/face.npy",
                format!("row {i}: face {} is outside [0, 360000)", f[i]),
                &[("row", Value::from(i as i64))],
            ));
        }
    }
    // 9. Chunk rows.
    let frame_grid = manifest.get("frame").and_then(|f| {
        let e: Vec<i64> = f
            .get("extent")?
            .as_arr()?
            .iter()
            .filter_map(mm_of)
            .collect();
        let r = mm_of(f.get("resolution")?)?;
        via_num::Grid::new([*e.first()?, *e.get(1)?, *e.get(2)?, *e.get(3)?], r).ok()
    });
    if let (Some(g), Some(xs), Some(ys)) = (
        frame_grid,
        columns.get("instances/x.npy"),
        columns.get("instances/y.npy"),
    ) {
        // The side the manifest's `frame.chunk` states (M6), else the default (5).
        let side = manifest
            .get("frame")
            .and_then(|f| f.get("chunk"))
            .and_then(mm_of);
        let layout = Layout::with_side(&g, side);
        let mut next = 0usize;
        for (ci, c) in chunks.iter().enumerate() {
            let mut ranges: Vec<(usize, usize)> = Vec::new();
            if let Some(J::Obj(m)) = c.get("instances") {
                for v in m.values() {
                    let a: Vec<i64> = v
                        .as_arr()
                        .map(|a| a.iter().filter_map(J::as_i64).collect())
                        .unwrap_or_default();
                    if a.len() == 2 {
                        ranges.push((
                            usize::try_from(a[0]).unwrap_or(0),
                            usize::try_from(a[1]).unwrap_or(0),
                        ));
                    }
                }
            }
            ranges.sort_unstable();
            for (first, k) in ranges {
                if first != next {
                    out.push(fault(
                        codes::check::CHUNK_ROWS,
                        "manifest.json",
                        format!("chunk {ci}: rows start at {first}; the next row is {next}"),
                        &[("chunk", Value::from(ci as i64))],
                    ));
                }
                for row in first..first + k {
                    if let (Some(x), Some(y)) = (xs.get(row), ys.get(row)) {
                        let (x, y) = (i64::from(*x), i64::from(*y));
                        let [x0, y0, x1, y1] = g.extent;
                        // A row outside the extent lies in no chunk; `chunk_of` would clamp it.
                        let inside = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
                        if !inside || layout.chunk_of(x, y) != ci {
                            out.push(fault(
                                codes::check::CHUNK_ROWS,
                                "instances/x.npy",
                                format!("row {row} lies outside its chunk {ci}"),
                                &[("row", Value::from(row as i64))],
                            ));
                        }
                    }
                }
                next = first + k;
            }
        }
        if next != count {
            out.push(fault(
                codes::check::CHUNK_ROWS,
                "manifest.json",
                format!("the chunks' rows end at {next}; there are {count}"),
                &[],
            ));
        }
    }
    // 10. JSON and text files: canonical, no absolute path.
    let mut texts: Vec<String> = vec!["manifest.json".into()];
    if dir.join(".previous").exists() {
        texts.push(".previous".into());
    }
    texts.extend(
        listed
            .keys()
            .filter(|p| !p.starts_with("export/") && (p.ends_with(".json") || p.ends_with(".txt")))
            .cloned(),
    );
    for p in texts {
        let Some(b) = files.get(&p).cloned() else {
            continue;
        };
        if p.ends_with(".json") || p == ".previous" {
            if p != "manifest.json" {
                if let Some(off) = first_non_canonical(&b) {
                    out.push(fault(
                        codes::check::NOT_CANONICAL,
                        &p,
                        format!("`{p}` is not canonical from byte {off}"),
                        &[("offset", Value::from(off as i64))],
                    ));
                    continue;
                }
            }
            if let Ok(v) = parse(&String::from_utf8_lossy(&b)) {
                let mut hits = Vec::new();
                find_absolute(&v, "", &mut hits);
                if let Some(h) = hits.first() {
                    out.push(fault(
                        codes::check::ABSOLUTE_PATH,
                        &p,
                        format!("`{p}` holds an absolute path at `{h}`"),
                        &[("key", Value::from(h.clone()))],
                    ));
                }
            }
        } else {
            let t = String::from_utf8_lossy(&b);
            if let Some(i) = t.lines().position(absolute) {
                out.push(fault(
                    codes::check::ABSOLUTE_PATH,
                    &p,
                    format!("`{p}` line {} holds an absolute path", i + 1),
                    &[("line", Value::from(i as i64 + 1))],
                ));
            }
        }
    }
    out
}

/// Millimetres from exact metres written by the build.
fn mm_of(v: &J) -> Option<i64> {
    let J::Num(t) = v else { return None };
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.as_str()),
    };
    let (int, frac) = t.split_once('.').unwrap_or((t, ""));
    if frac.len() > 3 {
        return None;
    }
    let mut f = frac.to_string();
    while f.len() < 3 {
        f.push('0');
    }
    let v = int.parse::<i64>().ok()? * 1000 + f.parse::<i64>().ok()?;
    Some(if neg { -v } else { v })
}
