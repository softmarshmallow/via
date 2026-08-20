//! Terrain-derived fields: what the land offers, before anyone moves.
//!
//! Everything here is read from a terrain run's artifacts (the stage
//! contract: no stage calls another). Two fields come out:
//!
//! - `productivity` — people the cell can feed per km², a Monod-style
//!   product of slope, soil, moisture, warmth and water access. It is the
//!   origin field O_i the settlement dynamics allocate.
//! - `site_quality` — how buildable a cell is (slope, dry ground, water
//!   at hand). Deliberately thin: harbour advantage, market position, and
//!   defensive value must come out of the cost surface and the dynamics,
//!   not out of a hand-written site bonus.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use via_artifact::manifest::RunManifest;
use via_artifact::raster::Raster;

use crate::util::{f64_key, for_neighbors8};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LandConfig {
    /// Slope (rise/run) at which arable value has fallen by 1/e.
    pub slope_decay: f64,
    /// Sediment thickness (m) at which soil value has risen by 1−1/e.
    pub soil_depth_half_m: f64,
    /// Value floor on bare rock (0 = rock feeds nobody).
    pub soil_floor: f64,
    /// Half-saturation precipitation (m/yr) in the Monod moisture term.
    pub precip_half_m: f64,
    /// Optimum and width (°C) of the Gaussian warmth term.
    pub temp_opt_c: f64,
    pub temp_width_c: f64,
    /// Distance to fresh water (m) beyond which access decays, and its
    /// decay length (m).
    pub water_free_m: f64,
    pub water_decay_m: f64,
    /// People per km² on a perfect cell. Sets the units of every size in
    /// the run; held equal across eras on purpose (see README).
    pub carrying_capacity_per_km2: f64,
    /// Standing water (m) above which a cell is not habitable ground.
    pub max_standing_water_m: f64,
    /// Discharge (equivalent cells) at or above which a channel counts as
    /// navigable water in the cost surface.
    pub navigable_min_cells: f64,
    /// Site quality: slope at which buildability has fallen by 1/e.
    pub build_slope_decay: f64,
}

impl Default for LandConfig {
    fn default() -> Self {
        Self {
            slope_decay: 0.12,
            soil_depth_half_m: 0.5,
            soil_floor: 0.25,
            precip_half_m: 0.35,
            temp_opt_c: 14.0,
            temp_width_c: 12.0,
            water_free_m: 1000.0,
            water_decay_m: 2500.0,
            carrying_capacity_per_km2: 120.0,
            max_standing_water_m: 0.05,
            navigable_min_cells: 20000.0,
            build_slope_decay: 0.10,
        }
    }
}

pub struct Land {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub sea_level_m: f64,
    pub heights_m: Vec<f64>,
    /// Terrain plus standing water — the surface travel and building see.
    pub surface_m: Vec<f64>,
    pub land: Vec<bool>,
    pub lake: Vec<bool>,
    pub receivers: Vec<u32>,
    pub slope: Vec<f64>,
    pub discharge_cells: Vec<f64>,
    pub water_depth_m: Vec<f64>,
    pub strahler: Vec<u32>,
    pub freshwater_dist_m: Vec<f64>,
    /// Navigable channel or standing water — the water mode's support.
    pub navigable: Vec<bool>,
    /// People per km² the cell can feed.
    pub productivity: Vec<f64>,
    /// Dimensionless [0, 1] buildability.
    pub site_quality: Vec<f64>,
    pub cell_area_km2: f64,
}

impl Land {
    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.w + x) as usize
    }
    #[inline]
    pub fn xy(&self, i: usize) -> (u32, u32) {
        (i as u32 % self.w, i as u32 / self.w)
    }
    /// People the cell feeds.
    #[inline]
    pub fn population_capacity(&self, i: usize) -> f64 {
        self.productivity[i] * self.cell_area_km2
    }
}

fn read_f32(dir: &Path, name: &str, base: &Raster<i32>) -> io::Result<Vec<f64>> {
    let r = Raster::<f32>::read_file(&dir.join(format!("{name}.vrast")))?;
    if (r.width, r.height) != (base.width, base.height) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("artifact {name} grid does not match heights_cm — mixed run directory?"),
        ));
    }
    Ok(r.data.iter().map(|&v| v as f64).collect())
}

/// Multi-source Dijkstra in metres over the D8 grid metric.
fn dist_from(w: u32, h: u32, dx: f64, src: &[bool]) -> Vec<f64> {
    let n = w as usize * h as usize;
    let mut dist = vec![f64::INFINITY; n];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    for (i, &s) in src.iter().enumerate() {
        if s {
            dist[i] = 0.0;
            heap.push(Reverse((f64_key(0.0), i as u32)));
        }
    }
    while let Some(Reverse((k, i))) = heap.pop() {
        if k > f64_key(dist[i as usize]) {
            continue;
        }
        let di = dist[i as usize];
        for_neighbors8(w, h, i, |nb, fac| {
            let nd = di + fac * dx;
            if nd < dist[nb as usize] {
                dist[nb as usize] = nd;
                heap.push(Reverse((f64_key(nd), nb)));
            }
        });
    }
    dist
}

pub fn load(run_dir: &Path, cfg: &LandConfig) -> io::Result<Land> {
    let heights = Raster::<i32>::read_file(&run_dir.join("heights_cm.vrast"))?;
    let receivers = Raster::<u32>::read_file(&run_dir.join("receivers.vrast"))?;
    let strahler = Raster::<u32>::read_file(&run_dir.join("strahler.vrast"))?;
    let (w, h) = (heights.width, heights.height);
    let dx = heights.cell_size_cm as f64 / 100.0;
    let n = w as usize * h as usize;

    let water_depth_m = read_f32(run_dir, "water_depth", &heights)?;
    let precip_m = read_f32(run_dir, "precip", &heights)?;
    let temp_c = read_f32(run_dir, "temperature", &heights)?;
    let sediment_m = read_f32(run_dir, "sediment", &heights)?;
    let discharge_cells = read_f32(run_dir, "discharge", &heights)?;

    let manifest = RunManifest::load(&run_dir.join("manifest.json"))?;
    let sea_level_m = manifest
        .config
        .get("sea_level_m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let heights_m: Vec<f64> = heights.data.iter().map(|&cm| cm as f64 / 100.0).collect();
    // Ocean = self-receiver (the pit gate guarantees no land self-receivers).
    let land: Vec<bool> = receivers
        .data
        .iter()
        .enumerate()
        .map(|(i, &r)| r as usize != i)
        .collect();
    let lake: Vec<bool> = (0..n)
        .map(|i| land[i] && water_depth_m[i] > cfg.max_standing_water_m)
        .collect();
    let surface_m: Vec<f64> = (0..n).map(|i| heights_m[i] + water_depth_m[i]).collect();

    // Steepest-descent slope over the effective surface, as the suitability
    // stage measures it: a shore cell must not see the drop through a lake's
    // water column.
    let mut slope = vec![0.0f64; n];
    for i in 0..n as u32 {
        if !land[i as usize] {
            continue;
        }
        let hi = surface_m[i as usize];
        let mut best = 0.0f64;
        for_neighbors8(w, h, i, |nb, fac| {
            let s = (hi - surface_m[nb as usize]) / (fac * dx);
            if s > best {
                best = s;
            }
        });
        slope[i as usize] = best;
    }

    let is_fresh: Vec<bool> = (0..n)
        .map(|i| land[i] && (strahler.data[i] > 0 || lake[i]))
        .collect();
    let freshwater_dist_m = dist_from(w, h, dx, &is_fresh);

    let navigable: Vec<bool> = (0..n)
        .map(|i| !land[i] || lake[i] || discharge_cells[i] >= cfg.navigable_min_cells)
        .collect();

    let cell_area_km2 = dx * dx / 1.0e6;
    let mut productivity = vec![0.0f64; n];
    let mut site_quality = vec![0.0f64; n];
    for i in 0..n {
        if !land[i] || lake[i] {
            continue;
        }
        let f_slope = (-slope[i] / cfg.slope_decay).exp();
        let f_soil = cfg.soil_floor
            + (1.0 - cfg.soil_floor) * (1.0 - (-sediment_m[i] / cfg.soil_depth_half_m).exp());
        let f_moist = precip_m[i] / (precip_m[i] + cfg.precip_half_m);
        let dt = (temp_c[i] - cfg.temp_opt_c) / cfg.temp_width_c;
        let f_temp = (-dt * dt).exp();
        let d = freshwater_dist_m[i];
        let f_water = if d <= cfg.water_free_m {
            1.0
        } else {
            (-(d - cfg.water_free_m) / cfg.water_decay_m).exp()
        };
        productivity[i] =
            cfg.carrying_capacity_per_km2 * f_slope * f_soil * f_moist * f_temp * f_water;
        site_quality[i] = (-slope[i] / cfg.build_slope_decay).exp() * f_water;
    }

    Ok(Land {
        w,
        h,
        dx,
        sea_level_m,
        heights_m,
        surface_m,
        land,
        lake,
        receivers: receivers.data.clone(),
        slope,
        discharge_cells,
        water_depth_m,
        strahler: strahler.data.clone(),
        freshwater_dist_m,
        navigable,
        productivity,
        site_quality,
        cell_area_km2,
    })
}
