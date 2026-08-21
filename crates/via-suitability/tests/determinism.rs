//! ADR 0011 Decision 6 determinism gate: byte-identical artifacts and
//! hashes on re-run. Until the multi-stage manifest schema evolution lands,
//! the stage summary's recorded hashes are the determinism witness, so the
//! test also pins summary hash == disk bytes for every artifact.

use std::collections::BTreeMap;
use std::path::Path;

use via_artifact::{Raster, RunManifest};
use via_suitability::SuitabilityConfig;

/// An 8×8 synthetic island: ocean border ring (self-receivers), land
/// interior rising toward the centre, a west-flowing river along row 4,
/// and one lake cell. Just enough terrain-artifact surface for the stage.
fn write_synthetic_terrain(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let (w, h) = (8u32, 8u32);
    let cell_cm = 1600; // 16 m cells
    let n = (w * h) as usize;
    let idx = |x: u32, y: u32| (y * w + x) as usize;

    let mut heights = vec![-500i32; n]; // ocean floor, sea level 0
    let mut receivers: Vec<u32> = (0..n as u32).collect(); // self-receiver = ocean
    let mut strahler = vec![0u32; n];
    let mut water = vec![0.0f32; n];

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = idx(x, y);
            let d = (x as i32 - 4).abs() + (y as i32 - 4).abs();
            heights[i] = 2 + 300 * (6 - d).max(0);
            receivers[i] = idx(x - 1, y) as u32; // everything drains west
        }
    }
    for x in 1..6 {
        strahler[idx(x, 4)] = 1;
    }
    water[idx(5, 2)] = 0.5;

    Raster::from_data(w, h, cell_cm, heights)
        .write_file(&dir.join("heights_cm.vrast"))
        .unwrap();
    Raster::from_data(w, h, cell_cm, receivers)
        .write_file(&dir.join("receivers.vrast"))
        .unwrap();
    Raster::from_data(w, h, cell_cm, strahler)
        .write_file(&dir.join("strahler.vrast"))
        .unwrap();
    Raster::from_data(w, h, cell_cm, water)
        .write_file(&dir.join("water_depth.vrast"))
        .unwrap();
    RunManifest {
        stage: "terrain".to_string(),
        seed: 0,
        config: serde_json::json!({ "sea_level_m": 0.0 }),
        crate_versions: BTreeMap::new(),
        artifacts: BTreeMap::new(),
    }
    .save(&dir.join("manifest.json"))
    .unwrap();
}

#[test]
fn rerun_is_byte_identical_and_summary_hashes_match_disk() {
    let base = std::env::temp_dir().join(format!("via-suit-det-{}", std::process::id()));
    let (d1, d2) = (base.join("a"), base.join("b"));
    write_synthetic_terrain(&d1);
    write_synthetic_terrain(&d2);

    let cfg = SuitabilityConfig {
        label: "test_site".to_string(),
        max_slope: 1.0,
        min_elevation_m: 0.0,
        max_elevation_m: 100.0,
        max_freshwater_dist_m: 1.0e9,
        min_patch_area_ha: 0.05,
        ..Default::default()
    };
    for d in [&d1, &d2] {
        let out = via_suitability::run(d, &cfg).unwrap();
        assert!(out.criterion_met, "synthetic island should yield a patch");
        via_suitability::write_outputs(d, &cfg, &out).unwrap();
    }

    let summary_name = via_suitability::summary_filename("test_site");
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(d1.join(&summary_name)).unwrap()).unwrap();
    assert_eq!(summary["selection"]["criterion"], "test_site");
    let hashes = summary["artifact_blake3"].as_object().unwrap();
    assert_eq!(hashes.len(), 4);

    for name in ["slope", "freshwater_dist", "coast_dist", "patch_rank"] {
        let file = via_suitability::raster_filename("test_site", name);
        let b1 = std::fs::read(d1.join(&file)).unwrap();
        let b2 = std::fs::read(d2.join(&file)).unwrap();
        assert_eq!(b1, b2, "{file} differs across identical runs");
        let disk_hash = blake3::hash(&b1).to_hex().to_string();
        assert_eq!(
            hashes[name].as_str().unwrap(),
            disk_hash,
            "summary hash != disk bytes for {name}"
        );
        // Every artifact carries a provenance label (ADR 0011 D1).
        let label = summary["provenance"][name].as_str().unwrap();
        assert!(
            label.starts_with("standard") || label.starts_with("heuristic"),
            "provenance for {name} must start with standard/heuristic"
        );
    }
    assert_eq!(
        std::fs::read(d1.join(&summary_name)).unwrap(),
        std::fs::read(d2.join(&summary_name)).unwrap(),
        "summaries differ across identical runs"
    );
    std::fs::remove_dir_all(&base).ok();
}
