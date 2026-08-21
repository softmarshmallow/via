//! Ecology stage: biome classes and continuous vegetation attributes,
//! derived from terrain + climate artifacts. Class from measurement, never
//! declaration: the classifier is the Whittaker (1975) mean-annual
//! temperature × precipitation diagram (a documented piecewise
//! approximation of the published figure), with terrain-side overrides that
//! are themselves measurements — standing water depth (lake/wetland), high
//! topographic wetness, bare rock by slope.
//!
//! Epistemic tiers (ADR 0003): the biome classification is **standard**
//! (Whittaker 1975, cited and fixed; the thresholds in `EcologyConfig` are
//! declared interpretation constants over the terrain stage's water-depth
//! spectrum). The vegetation attributes — density and canopy formulas —
//! are **heuristic**: our invention, plausible-shaped, from no paper.
//! Consumers wanting only defensible fields should stop at the rasters the
//! terrain stage ships plus the biome classes.
//!
//! Everything is a pure, sequential function of the artifacts; determinism
//! is trivial. No vegetation is placed — attributes are per-cell fields for
//! downstream stages to consume.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use via_artifact::manifest::{ArtifactEntry, RunManifest, StageRecord};
use via_artifact::raster::Raster;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EcologyConfig {
    /// Steepest-descent slope above which a cell is bare rock.
    pub rock_slope: f64,
    /// Standing-water depth (m) at or above which a cell is ponded enough
    /// for wetland (marsh); below `lake_min_water_m` it stays vegetated.
    pub wetland_min_water_m: f64,
    /// Standing-water depth (m) at or above which a cell is open water
    /// (lake). Thresholds over the terrain water-depth spectrum are this
    /// stage's declared interpretation constants (ADR 0003/0004).
    pub lake_min_water_m: f64,
    /// Wetland by wetness: TWI at or above this, with precipitation at or
    /// above `wetland_min_precip_mm`.
    pub wetland_min_twi: f64,
    pub wetland_min_precip_mm: f64,
}

impl Default for EcologyConfig {
    fn default() -> Self {
        Self {
            rock_slope: 0.70,
            wetland_min_water_m: 0.05,
            lake_min_water_m: 0.30,
            wetland_min_twi: 9.5,
            wetland_min_precip_mm: 600.0,
        }
    }
}

impl EcologyConfig {
    pub fn from_json_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("cannot parse config {}: {e}", path.display()))
    }
}

/// Biome classes. The u32 raster stores these discriminants; the JSON
/// report carries the legend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Biome {
    Water = 0,
    Tundra = 1,
    BorealForest = 2,
    TemperateSeasonalForest = 3,
    TemperateRainforest = 4,
    TropicalRainforest = 5,
    TropicalSeasonalForest = 6,
    Savanna = 7,
    Desert = 8,
    Grassland = 9,
    Shrubland = 10,
    Wetland = 11,
    BareRock = 12,
    /// Standing fresh water (a measurement over water depth, not authored
    /// taxonomy): ocean stays `Water`, lakes are their own class.
    Lake = 13,
}

pub const BIOME_NAMES: [&str; 14] = [
    "water",
    "tundra",
    "boreal_forest",
    "temperate_seasonal_forest",
    "temperate_rainforest",
    "tropical_rainforest",
    "tropical_seasonal_forest",
    "savanna",
    "desert",
    "grassland",
    "shrubland",
    "wetland",
    "bare_rock",
    "lake",
];

/// Piecewise approximation of the Whittaker biome diagram over mean-annual
/// temperature (°C) and precipitation (mm/yr).
pub fn whittaker(t_c: f64, p_mm: f64) -> Biome {
    if t_c < -5.0 {
        Biome::Tundra
    } else if t_c < 3.0 {
        if p_mm < 250.0 {
            Biome::Grassland
        } else {
            Biome::BorealForest
        }
    } else if t_c < 12.0 {
        if p_mm < 250.0 {
            Biome::Grassland
        } else if p_mm < 600.0 {
            Biome::Shrubland
        } else if p_mm < 1700.0 {
            Biome::TemperateSeasonalForest
        } else {
            Biome::TemperateRainforest
        }
    } else if t_c < 20.0 {
        if p_mm < 250.0 {
            Biome::Desert
        } else if p_mm < 550.0 {
            Biome::Grassland
        } else if p_mm < 900.0 {
            Biome::Shrubland
        } else if p_mm < 2200.0 {
            Biome::TemperateSeasonalForest
        } else {
            Biome::TemperateRainforest
        }
    } else if p_mm < 400.0 {
        Biome::Desert
    } else if p_mm < 1100.0 {
        Biome::Savanna
    } else if p_mm < 2400.0 {
        Biome::TropicalSeasonalForest
    } else {
        Biome::TropicalRainforest
    }
}

pub struct EcologyOutput {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub biome: Vec<u32>,
    pub veg_density: Vec<f64>,
    pub canopy_height_m: Vec<f64>,
    pub twi: Vec<f64>,
    /// (biome name, cell count, area in hectares), descending by area.
    pub class_areas: Vec<(String, u64, f64)>,
}

#[inline]
fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Per-biome canopy ceiling (m); scaled by local vegetation density.
fn canopy_base(b: Biome) -> f64 {
    match b {
        Biome::Water => 0.0,
        Biome::Tundra => 0.3,
        Biome::BorealForest => 15.0,
        Biome::TemperateSeasonalForest => 24.0,
        Biome::TemperateRainforest => 35.0,
        Biome::TropicalRainforest => 38.0,
        Biome::TropicalSeasonalForest => 22.0,
        Biome::Savanna => 6.0,
        Biome::Desert => 0.5,
        Biome::Grassland => 0.6,
        Biome::Shrubland => 3.0,
        Biome::Wetland => 2.5,
        Biome::BareRock => 0.0,
        Biome::Lake => 0.0,
    }
}

/// The artifact grids of a run directory must agree; a mismatch means a
/// stale or mixed directory and the stage fails loudly with a diagnosis.
fn ensure_grid<T>(name: &str, r: &Raster<T>, base: &Raster<i32>) -> io::Result<()> {
    if (r.width, r.height, r.cell_size_cm) != (base.width, base.height, base.cell_size_cm) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "artifact {name}.vrast grid {}×{} @ {} cm does not match heights_cm \
                 {}×{} @ {} cm — stale or mixed run directory?",
                r.width, r.height, r.cell_size_cm, base.width, base.height, base.cell_size_cm
            ),
        ));
    }
    Ok(())
}

/// Run the stage against a terrain run directory.
pub fn run(run_dir: &Path, cfg: &EcologyConfig) -> io::Result<EcologyOutput> {
    let heights = Raster::<i32>::read_file(&run_dir.join("heights_cm.vrast"))?;
    let receivers = Raster::<u32>::read_file(&run_dir.join("receivers.vrast"))?;
    let precip = Raster::<f32>::read_file(&run_dir.join("precip.vrast"))?;
    let temperature = Raster::<f32>::read_file(&run_dir.join("temperature.vrast"))?;
    let discharge = Raster::<f32>::read_file(&run_dir.join("discharge.vrast"))?;
    let water_depth = Raster::<f32>::read_file(&run_dir.join("water_depth.vrast"))?;
    ensure_grid("receivers", &receivers, &heights)?;
    ensure_grid("precip", &precip, &heights)?;
    ensure_grid("temperature", &temperature, &heights)?;
    ensure_grid("discharge", &discharge, &heights)?;
    ensure_grid("water_depth", &water_depth, &heights)?;
    let (w, h) = (heights.width, heights.height);
    let dx = heights.cell_size_cm as f64 / 100.0;
    let n = w as usize * h as usize;

    let heights_m: Vec<f64> = heights.data.iter().map(|&cm| cm as f64 / 100.0).collect();
    // Effective surface for slope/TWI: terrain + standing water. Heights
    // carry true lake bathymetry since M3; measuring slope through a
    // neighbour's water column would paint bare-rock rings on gentle
    // shores and suppress wetlands exactly at lake fringes.
    let eff_heights_m: Vec<f64> = heights_m
        .iter()
        .zip(water_depth.data.iter())
        .map(|(&hm, &wd)| hm + wd as f64)
        .collect();
    // The stage's authoritative base-level mask: self-receiver = ocean (the
    // pit gate guarantees no land self-receivers). Re-deriving land from
    // cm-quantized heights flips shoreline cells.
    let land: Vec<bool> = receivers
        .data
        .iter()
        .enumerate()
        .map(|(i, &r)| r as usize != i)
        .collect();

    // Steepest-descent slope (same definition as the suitability stage).
    let slope: Vec<f64> = (0..n as u32)
        .map(|i| {
            if !land[i as usize] {
                return 0.0;
            }
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let hi = eff_heights_m[i as usize];
            let mut best = 0.0f64;
            for (ddx, ddy, fac) in [
                (-1i64, -1i64, std::f64::consts::SQRT_2),
                (0, -1, 1.0),
                (1, -1, std::f64::consts::SQRT_2),
                (-1, 0, 1.0),
                (1, 0, 1.0),
                (-1, 1, std::f64::consts::SQRT_2),
                (0, 1, 1.0),
                (1, 1, std::f64::consts::SQRT_2),
            ] {
                let (nx, ny) = (x + ddx, y + ddy);
                if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
                    let s = (hi - eff_heights_m[(ny * w as i64 + nx) as usize]) / (fac * dx);
                    if s > best {
                        best = s;
                    }
                }
            }
            best
        })
        .collect();

    // Topographic wetness index over the discharge field:
    // TWI = ln(a / tanβ), a = specific catchment area (m).
    let twi: Vec<f64> = (0..n)
        .map(|i| {
            if !land[i] {
                return 0.0;
            }
            let a = (discharge.data[i] as f64).max(0.05) * dx;
            (a / slope[i].max(1.0e-3)).ln()
        })
        .collect();

    let mut biome = vec![0u32; n];
    let mut veg = vec![0.0f64; n];
    let mut canopy = vec![0.0f64; n];
    for i in 0..n {
        let wd = water_depth.data[i] as f64;
        let b = if !land[i] {
            Biome::Water
        } else if wd >= cfg.lake_min_water_m {
            // Open standing water outranks every terrestrial class,
            // including bare rock — a flooded cliff face is still a lake.
            Biome::Lake
        } else if slope[i] > cfg.rock_slope {
            Biome::BareRock
        } else {
            let p_mm = precip.data[i] as f64 * 1000.0;
            let t_c = temperature.data[i] as f64;
            let ponded = wd >= cfg.wetland_min_water_m;
            let soggy = twi[i] >= cfg.wetland_min_twi && p_mm >= cfg.wetland_min_precip_mm;
            if ponded || soggy {
                Biome::Wetland
            } else {
                whittaker(t_c, p_mm)
            }
        };
        biome[i] = b as u32;
        if land[i] && b != Biome::BareRock && b != Biome::Lake {
            let p_mm = precip.data[i] as f64 * 1000.0;
            let t_c = temperature.data[i] as f64;
            let moisture = clamp01(p_mm / 1400.0 + 0.02 * (twi[i] - 6.0));
            let warmth = clamp01((t_c + 6.0) / 22.0);
            let slope_penalty = 1.0 - clamp01((slope[i] - 0.35) / 0.40);
            let mut d = clamp01(moisture * warmth) * slope_penalty;
            if b == Biome::Wetland {
                d = (d + 0.30).min(0.80);
            }
            veg[i] = d;
            canopy[i] = canopy_base(b) * (0.40 + 0.60 * d);
        }
    }

    let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
    for &b in &biome {
        *counts.entry(b).or_insert(0) += 1;
    }
    let cell_ha = dx * dx / 10_000.0;
    let mut class_areas: Vec<(String, u64, f64)> = counts
        .iter()
        .filter(|&(&b, _)| b != Biome::Water as u32)
        .map(|(&b, &c)| (BIOME_NAMES[b as usize].to_string(), c, c as f64 * cell_ha))
        .collect();
    class_areas.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    Ok(EcologyOutput {
        w,
        h,
        dx,
        biome,
        veg_density: veg,
        canopy_height_m: canopy,
        twi,
        class_areas,
    })
}

/// Write the stage's artifacts and stats record into the run directory.
pub fn write_outputs(run_dir: &Path, cfg: &EcologyConfig, out: &EcologyOutput) -> io::Result<()> {
    let cell_cm = (out.dx * 100.0).round() as u32;
    let mut hashes = BTreeMap::new();

    let r = Raster::from_data(out.w, out.h, cell_cm, out.biome.clone());
    r.write_file(&run_dir.join("biome.vrast"))?;
    hashes.insert("biome".to_string(), r.blake3_hex());

    for (name, data) in [
        ("veg_density", &out.veg_density),
        ("canopy_height", &out.canopy_height_m),
        ("twi", &out.twi),
    ] {
        let f: Vec<f32> = data.iter().map(|&v| v as f32).collect();
        let r = Raster::from_data(out.w, out.h, cell_cm, f);
        r.write_file(&run_dir.join(format!("{name}.vrast")))?;
        hashes.insert(name.to_string(), r.blake3_hex());
    }

    let land_cells: u64 = out.class_areas.iter().map(|(_, c, _)| c).sum();
    let veg_mean = if land_cells > 0 {
        out.veg_density.iter().sum::<f64>() / land_cells as f64
    } else {
        0.0
    };
    let report = serde_json::json!({
        "stage": "ecology",
        "config": cfg,
        // ADR 0003: every output declares standard vs heuristic.
        "provenance": {
            "biome": "standard: Whittaker (1975) T×P classes; overrides are measurements (water depth, TWI, slope)",
            "twi": "standard: ln(a/tanβ), Beven & Kirkby (1979)",
            "veg_density": "heuristic (ADR 0003): moisture × warmth × slope penalty",
            "canopy_height": "heuristic (ADR 0003): per-biome ceiling scaled by density",
        },
        "legend": BIOME_NAMES,
        "class_areas": out.class_areas.iter().map(|(name, cells, ha)| {
            serde_json::json!({ "biome": name, "cells": cells, "area_ha": ha })
        }).collect::<Vec<_>>(),
        "biome_class_count": out.class_areas.len(),
        "mean_veg_density_land": veg_mean,
        "artifact_blake3": hashes,
    });
    std::fs::write(
        run_dir.join("ecology.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;

    // Register the stage in the run manifest (ADR 0011 Consequences:
    // downstream artifacts belong in the manifest; rasters only, like the
    // terrain stage — summaries are not artifacts).
    let manifest_path = run_dir.join("manifest.json");
    let mut manifest = RunManifest::load(&manifest_path)?;
    let artifacts = hashes
        .iter()
        .map(|(name, hash)| {
            (
                name.clone(),
                ArtifactEntry {
                    file: format!("{name}.vrast"),
                    blake3: hash.clone(),
                },
            )
        })
        .collect();
    let mut crate_versions = BTreeMap::new();
    crate_versions.insert(
        "via-ecology".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    manifest.stages.insert(
        "ecology".to_string(),
        StageRecord {
            config: serde_json::to_value(cfg)?,
            crate_versions,
            artifacts,
        },
    );
    manifest.save(&manifest_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whittaker_matches_reference_points() {
        assert_eq!(whittaker(-10.0, 500.0), Biome::Tundra);
        assert_eq!(whittaker(0.0, 600.0), Biome::BorealForest);
        assert_eq!(whittaker(8.0, 1000.0), Biome::TemperateSeasonalForest);
        assert_eq!(whittaker(8.0, 2000.0), Biome::TemperateRainforest);
        assert_eq!(whittaker(15.0, 100.0), Biome::Desert);
        assert_eq!(whittaker(15.0, 1100.0), Biome::TemperateSeasonalForest);
        assert_eq!(whittaker(15.0, 700.0), Biome::Shrubland);
        assert_eq!(whittaker(25.0, 800.0), Biome::Savanna);
        assert_eq!(whittaker(26.0, 3000.0), Biome::TropicalRainforest);
        assert_eq!(whittaker(5.0, 150.0), Biome::Grassland);
    }
}
