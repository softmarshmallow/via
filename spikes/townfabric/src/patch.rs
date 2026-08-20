//! The local terrain patch a town is built on. Samples a terrain run's
//! rasters into a metric frame centred on the settlement.
//!
//! Honest limit: the DEM is 200 m/cell, so a 1.5 km town sees roughly
//! 8 × 8 real elevation samples. Slope and river position are therefore
//! coarse — bilinear interpolation makes them smooth, not detailed. Real
//! lot-scale terrain needs the M7 resolution work.

use std::io;
use std::path::Path;

use via_artifact::raster::Raster;

use crate::geom::{closest_on_segment, dist, P2};

pub struct Patch {
    /// Half-width of the square domain (m); local coordinates run
    /// [−extent, +extent] on both axes.
    pub extent_m: f64,
    /// Grid origin of the patch centre, in DEM cell coordinates.
    cx: f64,
    cy: f64,
    dx: f64,
    w: u32,
    h: u32,
    heights_m: Vec<f64>,
    /// Standing water: ocean and lakes, at DEM resolution.
    water: Vec<bool>,
    /// River centrelines near the patch, as (a, b, half_width_m) in local
    /// metres. A 200 m DEM cell would otherwise make every stream 200 m
    /// wide — wider than any pre-modern bridge — so channel width comes
    /// from hydraulic geometry (w ≈ k·√A, the downstream form of Leopold &
    /// Maddock 1953) and the channel is carried as a line, not a cell.
    rivers: Vec<([f64; 2], [f64; 2], f64)>,
    pub sea_level_m: f64,
    pub site_cell: u32,
    /// How far the local origin had to move off the DEM site cell to find
    /// dry, buildable ground (m). At 200 m cells a river-side site often
    /// lands *in* the channel; towns sit on the bank, not in the water.
    pub site_shift_m: f64,
}

fn bilinear(v: &[f64], w: u32, h: u32, x: f64, y: f64) -> f64 {
    let xc = x.clamp(0.0, (w - 1) as f64);
    let yc = y.clamp(0.0, (h - 1) as f64);
    let (x0, y0) = (xc.floor() as u32, yc.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (xc - x0 as f64, yc - y0 as f64);
    let i = |xx: u32, yy: u32| v[(yy * w + xx) as usize];
    let a = i(x0, y0) * (1.0 - fx) + i(x1, y0) * fx;
    let b = i(x0, y1) * (1.0 - fx) + i(x1, y1) * fx;
    a * (1.0 - fy) + b * fy
}

impl Patch {
    pub fn load(
        run_dir: &Path,
        site_cell: u32,
        extent_m: f64,
        river_min_strahler: u32,
        lake_min_depth_m: f64,
        river_width_coef: f64,
    ) -> io::Result<Self> {
        let heights = Raster::<i32>::read_file(&run_dir.join("heights_cm.vrast"))?;
        let strahler = Raster::<u32>::read_file(&run_dir.join("strahler.vrast"))?;
        let wd = Raster::<f32>::read_file(&run_dir.join("water_depth.vrast"))?;
        let receivers = Raster::<u32>::read_file(&run_dir.join("receivers.vrast"))?;
        let (w, h) = (heights.width, heights.height);
        let dx = heights.cell_size_cm as f64 / 100.0;
        let heights_m: Vec<f64> = heights.data.iter().map(|&c| c as f64 / 100.0).collect();
        let discharge = Raster::<f32>::read_file(&run_dir.join("discharge.vrast"))?;
        let water: Vec<bool> = (0..(w as usize * h as usize))
            .map(|i| {
                let ocean = receivers.data[i] as usize == i;
                ocean || wd.data[i] as f64 >= lake_min_depth_m
            })
            .collect();
        let manifest = via_artifact::manifest::RunManifest::load(&run_dir.join("manifest.json"))?;
        let sea_level_m = manifest
            .config
            .get("sea_level_m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // River segments within reach of the patch, in local metres.
        let (cx, cy) = ((site_cell % w) as f64, (site_cell / w) as f64);
        let reach_cells = (extent_m / dx).ceil() as i64 + 2;
        let cell_area_km2 = dx * dx / 1.0e6;
        let mut rivers = Vec::new();
        for gy in (cy as i64 - reach_cells).max(0)..=(cy as i64 + reach_cells).min(h as i64 - 1) {
            for gx in (cx as i64 - reach_cells).max(0)..=(cx as i64 + reach_cells).min(w as i64 - 1)
            {
                let i = (gy * w as i64 + gx) as usize;
                if strahler.data[i] < river_min_strahler {
                    continue;
                }
                let r = receivers.data[i] as usize;
                if r == i {
                    continue;
                }
                let a = [(gx as f64 - cx) * dx, (gy as f64 - cy) * dx];
                let b = [
                    ((r % w as usize) as f64 - cx) * dx,
                    ((r / w as usize) as f64 - cy) * dx,
                ];
                let area_km2 = discharge.data[i] as f64 * cell_area_km2;
                let width = river_width_coef * area_km2.max(0.0).sqrt();
                rivers.push((a, b, 0.5 * width.clamp(2.0, 400.0)));
            }
        }

        let mut me = Self {
            extent_m,
            cx,
            cy,
            dx,
            w,
            h,
            heights_m,
            water,
            rivers,
            sea_level_m,
            site_cell,
            site_shift_m: 0.0,
        };
        // Nudge the origin onto the nearest dry, gentle ground. Scanned in
        // a fixed order, so the shift is reproducible.
        if me.is_water([0.0, 0.0]) || me.elevation([0.0, 0.0]) <= me.sea_level_m {
            let mut best: Option<(f64, [f64; 2])> = None;
            let mut r = 20.0;
            while r <= 500.0 && best.is_none() {
                let steps = ((std::f64::consts::TAU * r / 20.0).ceil() as i64).max(8);
                for k in 0..steps {
                    let a = std::f64::consts::TAU * k as f64 / steps as f64;
                    let p = [r * a.cos(), r * a.sin()];
                    if !me.is_water(p) && me.elevation(p) > me.sea_level_m && me.slope(p) < 0.3 {
                        best = Some((r, p));
                        break;
                    }
                }
                r += 20.0;
            }
            if let Some((d, p)) = best {
                me.cx += p[0] / me.dx;
                me.cy += p[1] / me.dx;
                me.site_shift_m = d;
                // Re-express the river segments in the shifted frame.
                for seg in me.rivers.iter_mut() {
                    seg.0 = [seg.0[0] - p[0], seg.0[1] - p[1]];
                    seg.1 = [seg.1[0] - p[0], seg.1[1] - p[1]];
                }
            }
        }
        Ok(me)
    }

    /// Local metres → DEM cell coordinates.
    #[inline]
    fn to_cell(&self, p: [f64; 2]) -> (f64, f64) {
        (self.cx + p[0] / self.dx, self.cy + p[1] / self.dx)
    }

    pub fn elevation(&self, p: [f64; 2]) -> f64 {
        let (x, y) = self.to_cell(p);
        bilinear(&self.heights_m, self.w, self.h, x, y)
    }

    pub fn is_water(&self, p: [f64; 2]) -> bool {
        let (x, y) = self.to_cell(p);
        let (xi, yi) = (
            (x.round() as i64).clamp(0, self.w as i64 - 1) as u32,
            (y.round() as i64).clamp(0, self.h as i64 - 1) as u32,
        );
        if self.water[(yi * self.w + xi) as usize] {
            return true;
        }
        self.rivers.iter().any(|&(a, b, hw)| {
            let (c, _) = closest_on_segment(p, a, b);
            dist(c, p) <= hw
        })
    }

    /// Width (m) of the widest channel within `r` of p — what a bridge
    /// would have to span.
    pub fn channel_width_m(&self, p: P2, r: f64) -> f64 {
        let mut best = 0.0f64;
        for &(a, b, hw) in &self.rivers {
            let (c, _) = closest_on_segment(p, a, b);
            if dist(c, p) <= r {
                best = best.max(2.0 * hw);
            }
        }
        best
    }

    /// Gradient magnitude (rise/run) from the interpolated surface.
    pub fn slope(&self, p: [f64; 2]) -> f64 {
        let d = self.dx * 0.5;
        let ex = self.elevation([p[0] + d, p[1]]) - self.elevation([p[0] - d, p[1]]);
        let ey = self.elevation([p[0], p[1] + d]) - self.elevation([p[0], p[1] - d]);
        ((ex / (2.0 * d)).powi(2) + (ey / (2.0 * d)).powi(2)).sqrt()
    }

    pub fn in_domain(&self, p: [f64; 2]) -> bool {
        p[0].abs() <= self.extent_m && p[1].abs() <= self.extent_m
    }

    /// Ground a street or a building may occupy.
    pub fn buildable(&self, p: [f64; 2], max_slope: f64) -> bool {
        self.in_domain(p)
            && !self.is_water(p)
            && self.elevation(p) > self.sea_level_m
            && self.slope(p) <= max_slope
    }

    /// Length of the water crossing along pq, sampled — 0 if it stays dry.
    pub fn crossing_len_m(&self, p: [f64; 2], q: [f64; 2]) -> f64 {
        let d = crate::geom::dist(p, q);
        let steps = (d / 8.0).ceil().max(1.0) as usize;
        let mut wet = 0.0;
        for k in 0..=steps {
            let t = k as f64 / steps as f64;
            let x = [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
            if self.is_water(x) {
                wet += d / steps as f64;
            }
        }
        wet
    }
}
