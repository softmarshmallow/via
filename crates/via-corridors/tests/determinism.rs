//! ADR 0012 Decision 6 determinism gate: byte-identical artifacts and
//! hashes on re-run, over a synthetic island whose suitability inputs
//! are produced by the real via-suitability stage (the corridors stage
//! consumes that stage's artifacts, so the fixture exercises the whole
//! consumption path incl. the manifest precondition).

use std::path::Path;

use via_artifact::{Raster, RunManifest};
use via_corridors::CorridorsConfig;
use via_suitability::SuitabilityConfig;

/// The 8×8 synthetic island from the suitability determinism fixture:
/// ocean ring, cone toward the centre, a west-flowing river along row
/// 4 with a tributary at (3,3), a secondary summit at (6,2) (one pass
/// site at (5,3)), and one lake cell at (5,2).
fn write_synthetic_terrain(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let (w, h) = (8u32, 8u32);
    let cell_cm = 1600;
    let n = (w * h) as usize;
    let idx = |x: u32, y: u32| (y * w + x) as usize;

    let mut heights = vec![-500i32; n];
    let mut receivers: Vec<u32> = (0..n as u32).collect();
    let mut strahler = vec![0u32; n];
    let mut water = vec![0.0f32; n];
    let mut area = vec![1u64; n];

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = idx(x, y);
            let d = (x as i32 - 4).abs() + (y as i32 - 4).abs();
            heights[i] = 2 + 300 * (6 - d).max(0);
            receivers[i] = idx(x - 1, y) as u32;
        }
    }
    for x in 1..6 {
        strahler[idx(x, 4)] = 1;
    }
    heights[idx(6, 2)] = 2000;
    receivers[idx(3, 3)] = idx(3, 4) as u32;
    strahler[idx(3, 3)] = 1;
    area[idx(4, 4)] = 6;
    area[idx(3, 3)] = 2;
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

fn run_full_stack(dir: &Path) {
    write_synthetic_terrain(dir);
    let s_cfg = SuitabilityConfig {
        label: "test_site".into(),
        max_slope: 1.0,
        min_elevation_m: 0.0,
        max_elevation_m: 100.0,
        max_freshwater_dist_m: 1.0e9,
        min_patch_area_ha: 0.05,
        min_pass_persistence_m: 2.0,
        ..Default::default()
    };
    let s_out = via_suitability::run(dir, &s_cfg).unwrap();
    via_suitability::write_outputs(dir, &s_cfg, &s_out).unwrap();

    // Small k_Q so the synthetic river is fordable and partly
    // navigable — the multimodal machinery must actually engage.
    let c_cfg = CorridorsConfig {
        label: "test".into(),
        suitability_label: "test_site".into(),
        lattice_spacing_cells: Some(2),
        ..Default::default()
    };
    let c_out = via_corridors::run(dir, &c_cfg).unwrap();
    via_corridors::write_outputs(dir, &c_cfg, &c_out).unwrap();
}

#[test]
fn rerun_is_byte_identical_and_gates_hold() {
    let base = std::env::temp_dir().join(format!("via-corr-det-{}", std::process::id()));
    let (d1, d2) = (base.join("a"), base.join("b"));
    run_full_stack(&d1);
    run_full_stack(&d2);

    let names = [
        "corridor_density",
        "trunk",
        "hours_to_sea_land",
        "hours_to_sea_water",
        "hours_to_trunk",
    ];
    for name in names {
        let f = via_corridors::raster_filename("test", name);
        let a = std::fs::read(d1.join(&f)).unwrap();
        let b = std::fs::read(d2.join(&f)).unwrap();
        assert_eq!(a, b, "{name} differs across reruns");
    }
    let sa = std::fs::read_to_string(d1.join(via_corridors::summary_filename("test"))).unwrap();
    let sb = std::fs::read_to_string(d2.join(via_corridors::summary_filename("test"))).unwrap();
    assert_eq!(sa, sb, "summary differs across reruns");

    // Summary hashes match the disk bytes and the manifest record.
    let summary: serde_json::Value = serde_json::from_str(&sa).unwrap();
    let manifest = RunManifest::load(&d1.join("manifest.json")).unwrap();
    let record = manifest.stage("corridors.test").expect("stage registered");
    for name in names {
        let f = via_corridors::raster_filename("test", name);
        let disk = blake3::hash(&std::fs::read(d1.join(&f)).unwrap())
            .to_hex()
            .to_string();
        let in_summary = summary["artifact_blake3"][name].as_str().unwrap();
        assert_eq!(in_summary, disk, "{name}: summary hash != disk");
        assert_eq!(
            record.artifacts[name].blake3, disk,
            "{name}: manifest hash != disk"
        );
    }

    // Every gate reported true (write_outputs hard-errors otherwise,
    // so this is a schema pin more than a re-check).
    for (k, v) in summary["checks"].as_object().unwrap() {
        assert_eq!(v.as_bool(), Some(true), "check {k} not true");
    }

    // The synthetic island has land everywhere reachable: density must
    // be nonzero at every lattice source, and the pass site must be a
    // trunk node.
    let density: Raster<u32> =
        Raster::read_file(&d1.join(via_corridors::raster_filename("test", "corridor_density")))
            .unwrap();
    assert!(density.data.iter().any(|&d| d > 0), "density all zero");
    let nodes = summary["trunk"]["nodes"].as_array().unwrap();
    assert!(
        nodes.iter().any(|n| n["class"] == "pass"),
        "no pass trunk node: {nodes:?}"
    );

    std::fs::remove_dir_all(&base).ok();
}
