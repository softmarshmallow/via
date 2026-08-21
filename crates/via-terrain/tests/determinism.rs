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
        a.discharge_cells.iter().cloned().fold(0.0, f64::max) >= fluvial_min,
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

/// Same contract on a layered substrate (ADR 0007): a dipped, folded,
/// faulted three-unit column with per-unit K and κ contrast, a soluble
/// unit, and a non-neutral sediment K — every lithology code path
/// (exposure lookup, per-cell-K solve, variable-κ diffusion, karst
/// spectrum, unit-restricted gates) under the bitwise contract.
#[test]
fn bitwise_identical_with_layered_column() {
    use via_terrain::config::{Fault, FoldTrain, LithologyConfig, RockUnit};
    let cfg = TerrainConfig {
        seed: 11,
        size: 96,
        steps: 25,
        lithology: LithologyConfig {
            datum_m: 150.0,
            units: vec![
                RockUnit {
                    thickness_m: 120.0,
                    k_mult: 1.0,
                    kappa_mult: 1.0,
                    solubility: 0.0,
                },
                RockUnit {
                    thickness_m: 90.0,
                    k_mult: 0.3,
                    kappa_mult: 0.4,
                    solubility: 0.8,
                },
                RockUnit {
                    k_mult: 1.2,
                    ..RockUnit::default()
                },
            ],
            dip: [0.01, 0.004],
            folds: vec![FoldTrain {
                amplitude_m: 80.0,
                wavelength_m: 6000.0,
                azimuth_deg: 25.0,
                phase_rad: 0.7,
            }],
            faults: vec![Fault {
                x_m: -1000.0,
                y_m: 500.0,
                azimuth_deg: 60.0,
                throw_m: 150.0,
            }],
            sediment_k_mult: 1.5,
            sediment_kappa_mult: 0.8,
            sediment_cover_min_m: 0.5,
        },
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
    // Tripwires: the scenario must actually expose multiple units on land
    // and produce a nonzero karst spectrum, or the new paths go untested.
    let mut units_on_land: Vec<u32> = a
        .lithology_unit
        .iter()
        .zip(a.land.iter())
        .filter(|&(_, &l)| l)
        .map(|(&u, _)| u)
        .collect();
    units_on_land.sort_unstable();
    units_on_land.dedup();
    assert!(
        units_on_land.len() >= 2,
        "layered scenario exposes only {units_on_land:?}; pick a new config"
    );
    assert!(
        a.karst_potential.iter().any(|&k| k > 0.0),
        "layered scenario produces no karst potential; pick a new config"
    );
    let bits_a: Vec<u64> = a.heights_m.iter().map(|v| v.to_bits()).collect();
    let bits_b: Vec<u64> = b.heights_m.iter().map(|v| v.to_bits()).collect();
    assert_eq!(
        bits_a, bits_b,
        "layered heights differ across thread counts"
    );
    assert_eq!(
        via_terrain::stage::artifact_hashes(&a),
        via_terrain::stage::artifact_hashes(&b),
        "artifact hashes differ across thread counts (layered run)"
    );
    assert_eq!(
        serde_json::to_string(&a.gates).unwrap(),
        serde_json::to_string(&b.gates).unwrap(),
        "gates differ across thread counts (layered run)"
    );
}

#[test]
fn write_run_roundtrip_matches_embedded_hashes() {
    let cfg = small_cfg();
    let out = run(&cfg, &mut |_, _| {});
    let dir = std::env::temp_dir().join(format!("via-roundtrip-{}", std::process::id()));
    let manifest = via_terrain::write_run(&dir, &out).unwrap();

    // The hashes embedded in gates.json, the manifest hashes, and the bytes
    // on disk must all agree.
    for (name, entry) in &manifest.stages["terrain"].artifacts {
        let bytes = std::fs::read(dir.join(&entry.file)).unwrap();
        let disk_hash = blake3::hash(&bytes).to_hex().to_string();
        assert_eq!(
            &disk_hash, &entry.blake3,
            "manifest hash != disk bytes for {name}"
        );
        assert_eq!(
            Some(&disk_hash),
            out.gates.artifact_blake3.get(name),
            "gates.json hash != disk bytes for {name}"
        );
    }
    // Heights survive the write/read cycle exactly.
    let back = via_artifact::Raster::<i32>::read_file(&dir.join("heights_cm.vrast")).unwrap();
    assert_eq!(back, via_terrain::stage::raster_heights_cm(&out));
    std::fs::remove_dir_all(&dir).ok();
}
