//! CONTRIBUTING: same seed → bitwise-identical output, on any core count.
//! A nondeterminism bug is a P0.

use via_terrain::{run, TerrainConfig};

fn small_cfg() -> TerrainConfig {
    TerrainConfig {
        seed: 11,
        size: 96,
        steps: 25,
        ..TerrainConfig::default()
    }
}

#[test]
fn bitwise_identical_across_thread_counts() {
    let cfg = small_cfg();
    let pool1 = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool4 = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let a = pool1.install(|| run(&cfg, &mut |_, _| {}));
    let b = pool4.install(|| run(&cfg, &mut |_, _| {}));

    // Tripwire: the hybrid channelized (SFD) routing path must actually be
    // exercised, or this test silently stops covering it (ADR 0005).
    let fluvial_min = (cfg.fluvial_min_area_km2 * 1.0e6) / cfg.cell_area_m2();
    assert!(
        a.discharge_cells.iter().cloned().fold(0.0, via_num::max) >= fluvial_min,
        "no channelized cells in the determinism scenario; pick a new config"
    );

    // Full-precision heights, bit for bit.
    let bits_a: Vec<u64> = a.heights_m.iter().map(|v| v.to_bits()).collect();
    let bits_b: Vec<u64> = b.heights_m.iter().map(|v| v.to_bits()).collect();
    assert_eq!(bits_a, bits_b, "f64 heights differ across thread counts");

    // Every artifact raster.
    assert_eq!(
        via_terrain::stage::artifact_hashes(&a),
        via_terrain::stage::artifact_hashes(&b),
        "artifact hashes differ across thread counts"
    );

    // Gates, including the embedded hashes.
    assert_eq!(
        serde_json::to_string(&a.gates).unwrap(),
        serde_json::to_string(&b.gates).unwrap(),
        "gates differ across thread counts"
    );

    // QA samples are written to disk too; they are part of the contract.
    assert_eq!(
        serde_json::to_string(&a.samples).unwrap(),
        serde_json::to_string(&b.samples).unwrap(),
        "qa samples differ across thread counts"
    );
}

/// Same contract, on a run whose FINAL surface carries standing water:
/// seed 11 ends dry, so without this the lake code paths (depression
/// merge, flooded-mask physics, lake gates, water-depth artifact) would
/// only ever be hashed against an all-zero field.
#[test]
fn bitwise_identical_with_final_lakes() {
    let cfg = TerrainConfig {
        seed: 6,
        size: 96,
        steps: 25,
        ..TerrainConfig::default()
    };
    let pool1 = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool4 = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let a = pool1.install(|| run(&cfg, &mut |_, _| {}));
    let b = pool4.install(|| run(&cfg, &mut |_, _| {}));
    // The scenario must actually exercise standing water, or this test
    // silently degrades to the dry case.
    assert!(
        a.gates.lake_cells > 0,
        "test config no longer produces final-surface lakes; pick a new seed"
    );
    let bits_a: Vec<u64> = a.water_depth_m.iter().map(|v| v.to_bits()).collect();
    let bits_b: Vec<u64> = b.water_depth_m.iter().map(|v| v.to_bits()).collect();
    assert_eq!(bits_a, bits_b, "water depths differ across thread counts");
    assert_eq!(
        via_terrain::stage::artifact_hashes(&a),
        via_terrain::stage::artifact_hashes(&b),
        "artifact hashes differ across thread counts (lake run)"
    );
    assert_eq!(
        serde_json::to_string(&a.gates).unwrap(),
        serde_json::to_string(&b.gates).unwrap(),
        "gates differ across thread counts (lake run)"
    );
}
