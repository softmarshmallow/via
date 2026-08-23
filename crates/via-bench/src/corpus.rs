//! Corpus aggregation: per-class reference populations, report figures,
//! and null-model context tables.
//!
//! This lives in via-bench, not the Python sidecar, deliberately: the
//! per-class populations are what future stage gates compare against,
//! and a gate's evidence chain must not pass through the quarantined
//! sidecar (ADR 0009 Decision 4). Everything here reads the fabric
//! JSONs that `reference` wrote and the provenance manifest.
//!
//! Partition discipline (ADR 0008 D6): populations, figures and contact
//! sheets are built from **fitted-partition towns only**. Held-out
//! towns are measured and stored, but no aggregate, figure or sheet
//! published from here includes them — they are spent only at a
//! validation event.
//!
//! Licence (ADR 0010 Decision 5): outputs here are per-class
//! aggregates and produced works, published with OSM attribution.
//! Per-town corpus-scale tables stay in the store; this module never
//! writes one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::measure::quantile;
use crate::render::Canvas;

/// The five reference classes, report order, with slug and operative
/// street set (protocol v1.1 P4: all_ways for contemporary informal).
pub const CLASSES: [(&str, &str, &str); 5] = [
    ("organic pre-modern core", "organic", "carriageway"),
    ("planted pre-modern grid", "planted", "carriageway"),
    ("19th-century survey plat", "plat", "carriageway"),
    ("mid-20th-century suburb", "suburb", "carriageway"),
    ("contemporary informal", "informal", "all_ways"),
];

/// Battery characters in report order: key in the fabric JSON, display
/// label, and whether the below-floor rule can null it (building
/// characters; reported with n_valid either way).
const CHARACTERS: [(&str, &str); 33] = [
    ("nodes", "nodes (interior)"),
    ("edges", "edges (interior)"),
    ("street_km", "street length km"),
    ("meshedness", "meshedness"),
    ("edge_node_ratio", "edge/node ratio"),
    ("k_avg", "avg node degree"),
    ("dead_end_share", "dead-end share"),
    ("deg3_share", "degree-3 share"),
    ("deg4_share", "degree-4 share"),
    ("self_loop_proportion", "self-loop proportion"),
    ("interior_components", "interior components"),
    ("orientation_entropy", "orientation entropy (nats)"),
    ("orientation_order", "orientation order phi"),
    ("circuity_avg", "circuity"),
    ("segment_len_median_m", "segment length median m"),
    ("segment_len_p90_m", "segment length p90 m"),
    ("bc_gini", "betweenness gini"),
    ("bc_max", "betweenness max"),
    ("blocks", "blocks"),
    ("block_area_median_m2", "block area median m2"),
    ("block_area_p90_m2", "block area p90 m2"),
    ("block_corners_median", "block corners median"),
    ("block_compactness_median", "block compactness median"),
    ("block_elongation_median", "block elongation median"),
    ("buildings", "buildings"),
    ("footprint_area_median_m2", "footprint area median m2"),
    ("footprint_area_p90_m2", "footprint area p90 m2"),
    ("bldg_street_dist_median_m", "bldg-street dist median m"),
    ("street_wall_share", "street wall share (inv.)"),
    ("street_fronting_share", "street fronting share (inv.)"),
    ("gsi", "ground space index"),
    ("storeys_mean_tagged", "storeys mean (tagged)"),
    ("storeys_tagged_share", "storeys tagged share"),
];

const ATTRIBUTION: &str =
    "Contains information from OpenStreetMap (c) OpenStreetMap contributors, ODbL";

pub struct TownRow {
    pub town: String,
    pub class: String,
    pub partition: String,
    /// street set label -> fabric character object
    pub fabric: BTreeMap<String, Value>,
}

/// Load the manifest and every town's fabric JSON; enforce one protocol
/// id across the corpus and collect the revision stamps seen.
pub fn load(
    manifest: &Path,
    measured: &Path,
    protocol_id: &str,
) -> Result<(Vec<TownRow>, Vec<String>)> {
    let mut rows = Vec::new();
    let mut revisions: Vec<String> = Vec::new();
    let text = std::fs::read_to_string(manifest)
        .with_context(|| format!("reading {}", manifest.display()))?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let rec: Value = serde_json::from_str(line)?;
        let town = rec["town"].as_str().context("manifest town")?.to_string();
        let path = measured.join(format!("{town}-fabric.json"));
        let doc: Value = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .with_context(|| format!("{town} not measured ({})", path.display()))?,
        )?;
        let got = doc["protocol"]["id"].as_str().unwrap_or("");
        if got != protocol_id {
            bail!("{town}: protocol id {got:?}, corpus requires {protocol_id:?}");
        }
        let rev = doc["code_revision"].as_str().unwrap_or("").to_string();
        if !revisions.contains(&rev) {
            revisions.push(rev);
        }
        let mut fabric = BTreeMap::new();
        if let Some(obj) = doc["fabric"].as_object() {
            for (set, v) in obj {
                fabric.insert(set.clone(), v.clone());
            }
        }
        rows.push(TownRow {
            town,
            class: rec["class"].as_str().context("manifest class")?.to_string(),
            partition: rec["partition"].as_str().context("partition")?.to_string(),
            fabric,
        });
    }
    Ok((rows, revisions))
}

fn finite_values(rows: &[&TownRow], set: &str, key: &str) -> Vec<f64> {
    rows.iter()
        .filter_map(|r| {
            r.fabric
                .get(set)
                .and_then(|f| f.get(key))
                .and_then(Value::as_f64)
        })
        .filter(|x| x.is_finite())
        .collect()
}

fn stats(values: &[f64]) -> Value {
    let mut v = values.to_vec();
    let q = |v: &mut Vec<f64>, p: f64| {
        let x = quantile(v, p);
        serde_json::Number::from_f64(x)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    };
    serde_json::json!({
        "n_valid": values.len(),
        "min": q(&mut v, 0.0),
        "p10": q(&mut v, 0.10),
        "p25": q(&mut v, 0.25),
        "median": q(&mut v, 0.50),
        "p75": q(&mut v, 0.75),
        "p90": q(&mut v, 0.90),
        "max": q(&mut v, 1.0),
    })
}

/// Per-class populations over fitted towns, both street sets.
pub fn populations(rows: &[TownRow], protocol_id: &str, revisions: &[String]) -> Value {
    let mut classes = serde_json::Map::new();
    for (class, slug, operative) in CLASSES {
        let fitted: Vec<&TownRow> = rows
            .iter()
            .filter(|r| r.class == class && r.partition == "fitted")
            .collect();
        let held_out = rows
            .iter()
            .filter(|r| r.class == class && r.partition == "held-out")
            .count();
        let mut sets = serde_json::Map::new();
        for set in ["carriageway", "all_ways"] {
            let mut chars = serde_json::Map::new();
            for (key, _) in CHARACTERS {
                let vals = finite_values(&fitted, set, key);
                chars.insert(key.to_string(), stats(&vals));
            }
            sets.insert(set.to_string(), Value::Object(chars));
        }
        classes.insert(
            slug.to_string(),
            serde_json::json!({
                "class": class,
                "operative_set": operative,
                "n_fitted": fitted.len(),
                "n_held_out": held_out,
                "fitted_towns": fitted.iter().map(|r| r.town.clone()).collect::<Vec<_>>(),
                "sets": Value::Object(sets),
            }),
        );
    }
    serde_json::json!({
        "kind": "reference populations, fitted partition only (ADR 0008 D6)",
        "protocol_id": protocol_id,
        "code_revisions": revisions,
        "quantiles": "nearest-rank",
        "attribution": ATTRIBUTION,
        "classes": Value::Object(classes),
    })
}

// ---------------------------------------------------------------- figures

const BG: [u8; 3] = [236, 232, 222];
const INK: [u8; 3] = [40, 38, 34];
const DOT: [u8; 3] = [46, 84, 140];
const MEDIAN: [u8; 3] = [188, 60, 44];

fn fmt_val(x: f64) -> String {
    if x == 0.0 {
        "0".to_string()
    } else if x.abs() >= 1000.0 {
        format!("{:.0}", x)
    } else if x.abs() >= 10.0 {
        format!("{:.1}", x)
    } else {
        format!("{:.3}", x)
    }
}

/// One strip figure: a row of fitted-town dots per class, median ticked.
pub fn character_figure(rows: &[TownRow], key: &str, label: &str) -> Canvas {
    let left = 250i64;
    let width = 960u32;
    let row_h = 46i64;
    let top = 50i64;
    let height = (top + row_h * CLASSES.len() as i64 + 58) as u32;
    let mut c = Canvas::new(width, height, 1.0, [0.0, 0.0]);
    for p in c.img.pixels_mut() {
        *p = image::Rgb(BG);
    }
    c.label(10, 8, &label.to_uppercase(), 2);

    // Shared axis over every drawn value.
    let mut all: Vec<f64> = Vec::new();
    let mut per_class: Vec<(String, Vec<f64>)> = Vec::new();
    for (class, slug, operative) in CLASSES {
        let fitted: Vec<&TownRow> = rows
            .iter()
            .filter(|r| r.class == class && r.partition == "fitted")
            .collect();
        let vals = finite_values(&fitted, operative, key);
        all.extend(&vals);
        // Each row draws its class's operative set; the one class whose
        // operative set differs (informal, all_ways) is tagged so the
        // mixed-set axis is never read as one street set.
        let tag = if operative == "all_ways" { " AW" } else { "" };
        per_class.push((format!("{slug}{tag}"), vals));
    }
    if all.is_empty() {
        c.label(left as i32, top as i32, "NO FINITE VALUES", 2);
        return c;
    }
    let lo = all.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = all.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = if (hi - lo).abs() < 1e-12 {
        1.0
    } else {
        hi - lo
    };
    let (alo, ahi) = (lo - 0.05 * span, hi + 0.05 * span);
    let x_of = |v: f64| -> i64 {
        left + ((v - alo) / (ahi - alo) * (width as i64 - left - 40) as f64) as i64
    };

    for (i, (slug, vals)) in per_class.iter().enumerate() {
        let y = top + row_h * i as i64 + row_h / 2;
        c.label(10, (y - 8) as i32, &slug.to_uppercase(), 2);
        c.label(
            (left - 60) as i32,
            (y - 8) as i32,
            &format!("N{:>2}", vals.len()),
            2,
        );
        // baseline
        for x in left..(width as i64 - 40) {
            c.blend(x, y, INK, 0.12);
        }
        for &v in vals {
            let x = x_of(v);
            for dy in -3i64..=3 {
                for dx in -3i64..=3 {
                    if dx * dx + dy * dy <= 9 {
                        c.blend(x + dx, y + dy, DOT, 0.55);
                    }
                }
            }
        }
        if !vals.is_empty() {
            let mut v = vals.clone();
            let med = quantile(&mut v, 0.5);
            let x = x_of(med);
            for dy in -9i64..=9 {
                c.blend(x, y + dy, MEDIAN, 0.9);
                c.blend(x + 1, y + dy, MEDIAN, 0.9);
            }
        }
    }
    // axis with three tick labels
    let ax_y = top + row_h * CLASSES.len() as i64 + 8;
    for x in left..(width as i64 - 40) {
        c.blend(x, ax_y, INK, 0.6);
    }
    for (v, align_right) in [(alo, false), ((alo + ahi) / 2.0, false), (ahi, true)] {
        let x = x_of(v);
        for dy in 0..5i64 {
            c.blend(x, ax_y + dy, INK, 0.8);
        }
        let text = fmt_val(v);
        let tx = if align_right {
            x - 6 * 2 * text.len() as i64
        } else {
            x + 3
        };
        c.label(tx as i32, (ax_y + 7) as i32, &text, 2);
    }
    c.label(
        10,
        height as i32 - 14,
        "FITTED TOWNS ONLY. OSM (c) OPENSTREETMAP CONTRIBUTORS, ODBL",
        1,
    );
    c
}

/// Render every character figure into `out`.
pub fn figures(rows: &[TownRow], out: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(out)?;
    let mut written = Vec::new();
    for (key, label) in CHARACTERS {
        let c = character_figure(rows, key, label);
        let path = out.join(format!("{key}.png"));
        c.img.save(&path)?;
        written.push(path);
    }
    Ok(written)
}

// ----------------------------------------------------------- null tables

/// Markdown table: one class's fitted population vs its matched null
/// ensembles (ADR 0008 D9), street characters only. `nulls` maps model
/// name -> the null run JSON (with per-seed values and ensemble stats).
pub fn null_table(
    rows: &[TownRow],
    class: &str,
    operative: &str,
    nulls: &[(String, Value)],
) -> String {
    let fitted: Vec<&TownRow> = rows
        .iter()
        .filter(|r| r.class == class && r.partition == "fitted")
        .collect();
    let street_chars: &[&str] = &[
        "meshedness",
        "k_avg",
        "dead_end_share",
        "deg4_share",
        "orientation_entropy",
        "orientation_order",
        "circuity_avg",
        "bc_gini",
        "bc_max",
        "segment_len_median_m",
    ];
    let mut s = String::new();
    s.push_str("| character | class p10 | class median | class p90 |");
    for (m, _) in nulls {
        s.push_str(&format!(" {m} mean +/- sd |"));
    }
    s.push('\n');
    s.push_str("| --- | --- | --- | --- |");
    for _ in nulls {
        s.push_str(" --- |");
    }
    s.push('\n');
    for key in street_chars {
        let vals = finite_values(&fitted, operative, key);
        let mut v = vals.clone();
        s.push_str(&format!(
            "| {key} | {} | {} | {} |",
            fmt_val(quantile(&mut v, 0.10)),
            fmt_val(quantile(&mut v, 0.50)),
            fmt_val(quantile(&mut v, 0.90)),
        ));
        for (_, doc) in nulls {
            let mean = doc["ensemble"]["mean"][*key].as_f64();
            let sd = doc["ensemble"]["sd"][*key].as_f64();
            match (mean, sd) {
                (Some(m), Some(d)) => s.push_str(&format!(" {} +/- {} |", fmt_val(m), fmt_val(d))),
                _ => s.push_str(" - |"),
            }
        }
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------- contact sheets

/// One contact sheet per class: fitted towns' panel PNGs, thumbnailed.
pub fn sheet(rows: &[TownRow], measured: &Path, class: &str, slug: &str) -> Result<Option<Canvas>> {
    let fitted: Vec<&TownRow> = rows
        .iter()
        .filter(|r| r.class == class && r.partition == "fitted")
        .collect();
    if fitted.is_empty() {
        return Ok(None);
    }
    let thumb = 224u32;
    let label_h = 16u32;
    let cols = 5usize;
    let rows_n = fitted.len().div_ceil(cols);
    let w = cols as u32 * (thumb + 8) + 8;
    let h = 40 + rows_n as u32 * (thumb + label_h + 10) + 24;
    let mut c = Canvas::new(w, h, 1.0, [0.0, 0.0]);
    c.label(10, 8, &format!("{} - FITTED TOWNS", slug.to_uppercase()), 2);
    for (i, r) in fitted.iter().enumerate() {
        let path = measured.join(format!("{}-town.png", r.town));
        let img = image::open(&path)
            .with_context(|| format!("panel missing: {}", path.display()))?
            .to_rgb8();
        let small =
            image::imageops::resize(&img, thumb, thumb, image::imageops::FilterType::Triangle);
        let x0 = 8 + (i % cols) as u32 * (thumb + 8);
        let y0 = 40 + (i / cols) as u32 * (thumb + label_h + 10);
        image::imageops::replace(&mut c.img, &small, x0 as i64, y0 as i64);
        c.label(
            x0 as i32,
            (y0 + thumb + 2) as i32,
            &r.town.to_uppercase(),
            1,
        );
    }
    c.label(
        10,
        h as i32 - 14,
        "OSM (c) OPENSTREETMAP CONTRIBUTORS, ODBL",
        1,
    );
    Ok(Some(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn town(name: &str, class: &str, partition: &str, meshedness: f64) -> TownRow {
        let mut fabric = BTreeMap::new();
        for set in ["carriageway", "all_ways"] {
            fabric.insert(
                set.to_string(),
                serde_json::json!({"meshedness": meshedness, "nodes": 50}),
            );
        }
        TownRow {
            town: name.to_string(),
            class: class.to_string(),
            partition: partition.to_string(),
            fabric,
        }
    }

    /// ADR 0008 D6: a held-out town must never enter a published
    /// aggregate. The fitted list, the counts, and the statistics all
    /// have to exclude it.
    #[test]
    fn populations_exclude_held_out() {
        let rows = vec![
            town("a", "organic pre-modern core", "fitted", 0.10),
            town("b", "organic pre-modern core", "fitted", 0.20),
            town("c", "organic pre-modern core", "held-out", 0.90),
        ];
        let p = populations(&rows, "v1.1", &["rev".into()]);
        let cls = &p["classes"]["organic"];
        assert_eq!(cls["n_fitted"], 2);
        assert_eq!(cls["n_held_out"], 1);
        let towns: Vec<&str> = cls["fitted_towns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(!towns.contains(&"c"));
        // Max over fitted towns only: 0.9 must not appear.
        let max = cls["sets"]["carriageway"]["meshedness"]["max"]
            .as_f64()
            .unwrap();
        assert!((max - 0.20).abs() < 1e-12);
        assert_eq!(cls["sets"]["carriageway"]["meshedness"]["n_valid"], 2);
    }

    /// NaN (serialized null) values are dropped, with n_valid honest.
    #[test]
    fn populations_drop_non_finite() {
        let mut t = town("a", "planted pre-modern grid", "fitted", 0.10);
        t.fabric.insert(
            "carriageway".to_string(),
            serde_json::json!({"meshedness": null, "nodes": 50}),
        );
        let rows = vec![t, town("b", "planted pre-modern grid", "fitted", 0.30)];
        let p = populations(&rows, "v1.1", &[]);
        let m = &p["classes"]["planted"]["sets"]["carriageway"]["meshedness"];
        assert_eq!(m["n_valid"], 1);
        assert!((m["median"].as_f64().unwrap() - 0.30).abs() < 1e-12);
    }

    /// The strip figure draws fitted dots only and survives classes
    /// with no towns at all.
    #[test]
    fn character_figure_smoke() {
        let rows = vec![
            town("a", "organic pre-modern core", "fitted", 0.10),
            town("h", "organic pre-modern core", "held-out", 0.90),
        ];
        let c = character_figure(&rows, "meshedness", "meshedness");
        assert!(c.img.width() > 0);
        let empty: Vec<TownRow> = Vec::new();
        let c2 = character_figure(&empty, "meshedness", "meshedness");
        assert!(c2.img.width() > 0);
    }

    /// Null tables carry class quantiles beside each model's ensemble.
    #[test]
    fn null_table_shape() {
        let rows = vec![
            town("a", "19th-century survey plat", "fitted", 0.20),
            town("b", "19th-century survey plat", "fitted", 0.30),
        ];
        let null_doc = serde_json::json!({
            "ensemble": {"mean": {"meshedness": 0.25}, "sd": {"meshedness": 0.01}}
        });
        let t = null_table(
            &rows,
            "19th-century survey plat",
            "carriageway",
            &[("grid".to_string(), null_doc)],
        );
        assert!(t.contains("| meshedness |"));
        assert!(t.contains("grid mean +/- sd"));
        assert!(t.contains("0.250 +/- 0.010"));
    }
}
