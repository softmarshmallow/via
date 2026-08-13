//! Climate boundary conditions, computed against the evolving topography
//! inside the terrain stage (the stage boundary stays one-directional;
//! the terrain↔precipitation loop is internal).
//!
//! Precipitation: a single-layer moisture-advection sweep along the
//! prevailing wind — air saturates over open water, precipitates a
//! background (convective) fraction everywhere and an orographic fraction
//! proportional to forced lift, and dries crossing ridges (rain shadow).
//! This is the standard first-order orographic model; no seasonality.
//!
//! Temperature: sea-level base minus lapse, plus a weak meridional gradient
//! and low-amplitude noise. Mean-annual values; no seasonality.

use rayon::prelude::*;

use crate::config::TerrainConfig;
use crate::grid::Grid;
use crate::noise;
use via_artifact::seed;

/// Prevailing wind: one of the 8 D8 directions, derived from the seed like
/// every other boundary condition.
#[derive(Clone, Copy, Debug)]
pub struct Wind {
    pub dx: i32,
    pub dy: i32,
}

pub fn prevailing_wind(cfg: &TerrainConfig) -> Wind {
    const DIRS: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];
    let k = (seed::derive(cfg.seed, "climate", 0) % 8) as usize;
    Wind {
        dx: DIRS[k].0,
        dy: DIRS[k].1,
    }
}

/// Humidity normalization anchor. The sweep coefficients themselves are
/// forcing and live in `TerrainConfig` (ADR 0003); this is just the unit
/// the deficit is measured against, and the field is normalized afterward.
const HUMIDITY_CAPACITY: f64 = 1.0;

/// Precipitation field (m/yr), normalized so the land mean equals
/// `cfg.precip_mean_m_per_yr`. Each wind-parallel line is independent and
/// swept sequentially; lines run in parallel (deterministic per-cell work).
pub fn compute_precipitation(cfg: &TerrainConfig, grid: &Grid, h: &[f64], wind: Wind) -> Vec<f64> {
    let (w, hh) = (grid.w as i64, grid.h as i64);
    let sea = cfg.sea_level_m;
    let mut raw = vec![0.0f64; grid.n()];

    // Starting cells of every wind-parallel line: the upwind border(s).
    let mut starts: Vec<(i64, i64)> = Vec::new();
    if wind.dx > 0 {
        starts.extend((0..hh).map(|y| (0i64, y)));
    }
    if wind.dx < 0 {
        starts.extend((0..hh).map(|y| (w - 1, y)));
    }
    if wind.dy > 0 {
        starts.extend((0..w).map(|x| (x, 0i64)));
    }
    if wind.dy < 0 {
        starts.extend((0..w).map(|x| (x, hh - 1)));
    }
    starts.sort_unstable();
    starts.dedup();

    // March each line; write into disjoint per-line buffers, then scatter.
    let step_m = if wind.dx != 0 && wind.dy != 0 {
        grid.dx * std::f64::consts::SQRT_2
    } else {
        grid.dx
    };
    let ema_alpha = 1.0 - (-step_m / cfg.airflow_smooth_m).exp();
    let (evap, convective, oro_per_100m) = (
        cfg.evaporation_per_cell,
        cfg.convective_rainout_per_cell,
        cfg.orographic_rainout_per_100m,
    );
    let lines: Vec<Vec<(u32, f64)>> = starts
        .par_iter()
        .map(|&(x0, y0)| {
            let mut out = Vec::new();
            let mut humidity = HUMIDITY_CAPACITY; // air arrives from open sea
            let mut air_h = sea; // smoothed height the air column rides at
            let (mut x, mut y) = (x0, y0);
            while x >= 0 && y >= 0 && x < w && y < hh {
                let i = (y * w + x) as u32;
                let hc = h[i as usize].max(sea);
                let air_prev = air_h;
                air_h += ema_alpha * (hc - air_h);
                if h[i as usize] < sea {
                    humidity += evap * (HUMIDITY_CAPACITY - humidity);
                    out.push((i, convective * humidity));
                } else {
                    let lift = (air_h - air_prev).max(0.0);
                    let rainout = convective + oro_per_100m * (lift / 100.0);
                    let p = (rainout * humidity).min(humidity);
                    humidity -= p;
                    out.push((i, p));
                }
                x += wind.dx as i64;
                y += wind.dy as i64;
            }
            out
        })
        .collect();
    for line in &lines {
        for &(i, p) in line {
            // Diagonal winds visit some cells from two start lines; keep the max
            // rather than double-counting.
            if p > raw[i as usize] {
                raw[i as usize] = p;
            }
        }
    }

    // Two passes of a 3×3 box blur: rain does not fall in wind-parallel
    // stripes. Pure per-cell map, parallel-safe.
    for _ in 0..2 {
        let src = raw.clone();
        raw = (0..grid.n() as u32)
            .into_par_iter()
            .map(|i| {
                let (x, y) = grid.xy(i);
                let (x, y) = (x as i64, y as i64);
                let mut sum = 0.0;
                let mut count = 0.0;
                for oy in -1..=1i64 {
                    for ox in -1..=1i64 {
                        let (nx, ny) = (x + ox, y + oy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < hh {
                            sum += src[(ny * w + nx) as usize];
                            count += 1.0;
                        }
                    }
                }
                sum / count
            })
            .collect();
    }

    // Normalize to the configured mean over land (sequential sum: the
    // scale factor reaches output).
    let (mut land_sum, mut land_n) = (0.0f64, 0u64);
    for i in 0..grid.n() {
        if h[i] >= sea {
            land_sum += raw[i];
            land_n += 1;
        }
    }
    let scale = if land_n > 0 && land_sum > 0.0 {
        cfg.precip_mean_m_per_yr * land_n as f64 / land_sum
    } else {
        0.0
    };
    raw.par_iter_mut().for_each(|p| *p *= scale);
    raw
}

/// Mean-annual temperature (°C): sea-level base − lapse·elevation, a weak
/// north→south gradient, and low-amplitude noise for texture. Water cells
/// get the sea-level value.
pub fn compute_temperature(cfg: &TerrainConfig, grid: &Grid, h: &[f64]) -> Vec<f64> {
    let s_noise = seed::derive(cfg.seed, "climate", 1);
    let sea = cfg.sea_level_m;
    let (w, hh) = (grid.w as f64, grid.h as f64);
    (0..grid.n() as u32)
        .into_par_iter()
        .map(|i| {
            let (x, y) = grid.xy(i);
            let u = (x as f64 + 0.5) / w;
            let v = (y as f64 + 0.5) / hh;
            let elev = (h[i as usize] - sea).max(0.0);
            cfg.t_sea_level_c - cfg.lapse_c_per_km * elev / 1000.0
                + cfg.t_meridional_delta_c * (v - 0.5)
                + 0.6 * noise::fbm(s_noise, u * 8.0, v * 8.0, 3)
        })
        .collect()
}
