//! ADR 0011 Decision 6 determinism gate: byte-identical artifacts and
//! hashes on re-run. The run manifest is the determinism witness (the
//! multi-stage schema evolution recorded in ADR 0011 Consequences), so the
//! test pins summary hash == manifest hash == disk bytes for every
//! artifact, and covers the v1-manifest migration path.

use std::path::Path;

use via_artifact::{Raster, RunManifest};
use via_suitability::SuitabilityConfig;

/// An 8×8 synthetic island: ocean border ring (self-receivers), land
/// interior rising toward the centre, a west-flowing river along row 4
/// joined by a tributary from the north at (3,4), and one lake cell.
/// Just enough terrain-artifact surface for the stage.
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
    let mut area = vec![1u64; n];

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
    // A secondary summit at (6,2): the col to the main cone sits at
    // (5,3) (1202 cm), so the cone's peak (1802 cm) dies there with
    // persistence 6.0 m — one pass site above a low test floor.
    heights[idx(6, 2)] = 2000;
    // Tributary joining the main stem: (3,3) drains south into (3,4).
    receivers[idx(3, 3)] = idx(3, 4) as u32;
    strahler[idx(3, 3)] = 1;
    area[idx(4, 4)] = 6; // main-stem donor drainage at the junction
    area[idx(3, 3)] = 2; // tributary donor drainage
    water[idx(5, 2)] = 0.5;
    let mut discharge = vec![0.05f32; n];
    for i in 0..n {
        if strahler[i] > 0 {
            discharge[i] = 100.0;
        }
    }

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
    Raster::from_data(w, h, cell_cm, area)
        .write_file(&dir.join("area_cells.vrast"))
        .unwrap();
    Raster::from_data(w, h, cell_cm, discharge)
        .write_file(&dir.join("discharge.vrast"))
        .unwrap();
    // Written in the version-1 single-stage shape deliberately: the stage
    // must migrate legacy run directories transparently.
    let v1 = serde_json::json!({
        "stage": "terrain",
        "seed": 0,
        "config": { "sea_level_m": 0.0, "precip_mean_m_per_yr": 1.2 },
        "crate_versions": {},
        "artifacts": {}
    });
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&v1).unwrap() + "\n",
    )
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
        min_pass_persistence_m: 2.0,
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
    assert_eq!(hashes.len(), 13);

    for name in [
        "slope",
        "freshwater_dist",
        "coast_dist",
        "patch_rank",
        "ford_width",
        "ford_depth",
        "ford_velocity",
        "crossability",
        "navigability_ts",
        "navigable",
        "harbour_fetch",
        "harbour_depth_window",
        "harbour_sediment",
    ] {
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

    // The tributary junction is reported as a confluence site with the
    // exact Benda symmetry ratio, and the definition gate passed.
    let sites = summary["affordances"]["confluences"]["sites"]
        .as_array()
        .unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0]["x"], 3);
    assert_eq!(sites[0]["y"], 4);
    assert_eq!(sites[0]["river_donors"], 2);
    assert!((sites[0]["symmetry_ratio"].as_f64().unwrap() - 2.0 / 6.0).abs() < 1e-12);
    assert_eq!(summary["checks"]["confluence_definition"], true);
    assert_eq!(summary["checks"]["spectrum_identities"], true);
    assert_eq!(summary["checks"]["head_of_navigation_definition"], true);
    assert_eq!(summary["checks"]["fetch_bounds"], true);

    // The col between the cone and the secondary summit is the one pass
    // site above the floor: persistence = 18.02 − 12.02 = 6.0 m.
    let pass_sites = summary["affordances"]["passes"]["sites"]
        .as_array()
        .unwrap();
    assert_eq!(pass_sites.len(), 1);
    assert_eq!(pass_sites[0]["x"], 5);
    assert_eq!(pass_sites[0]["y"], 3);
    assert!((pass_sites[0]["persistence_m"].as_f64().unwrap() - 6.0).abs() < 1e-9);

    // Shore slope is measured to the sea surface, never through the
    // ocean water column: at (1,4), 9.02 m beside the sea, the slope is
    // 9.02/16 = 0.56375 (through-column would read 0.87625).
    let slope =
        Raster::<f32>::read_file(&d1.join(via_suitability::raster_filename("test_site", "slope")))
            .unwrap();
    let s = slope.data[(4 * 8 + 1) as usize] as f64;
    assert!((s - 0.563_75).abs() < 1e-4, "shore slope {s}");

    // The lake cell carries still-water crossability equal to its depth.
    let cross = Raster::<f32>::read_file(&d1.join(via_suitability::raster_filename(
        "test_site",
        "crossability",
    )))
    .unwrap();
    assert_eq!(cross.data[(2 * 8 + 5) as usize], 0.5);

    // The stage registered itself in the (now version-2) manifest, hashes
    // agree with the summary's, and the terrain record survived migration.
    let m = RunManifest::load(&d1.join("manifest.json")).unwrap();
    let rec = m.stage("suitability.test_site").expect("stage registered");
    assert_eq!(rec.artifacts.len(), 13);
    for (name, entry) in &rec.artifacts {
        assert_eq!(hashes[name].as_str().unwrap(), entry.blake3);
        assert_eq!(
            entry.file,
            via_suitability::raster_filename("test_site", name)
        );
    }
    assert!(
        m.stage("terrain").is_some(),
        "terrain record lost in migration"
    );
    std::fs::remove_dir_all(&base).ok();
}
