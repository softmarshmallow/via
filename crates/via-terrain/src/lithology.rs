//! Lithology & structure (ADR 0007): a deformed layer-cake evaluated in
//! the material frame.
//!
//! The stratigraphic column (units top-down from a datum) is displaced by
//! a static structural field d(x, y) — dip plane, fold sinusoids, sharp
//! fault throws — precomputed once per run. Rock advects vertically with
//! uplift, so the unit exposed at a cell is looked up at the
//! stratigraphic elevation s = z_bedrock − Σ(applied uplift) − d(x, y):
//! erosion moves s down through the column, pure uplift leaves it fixed.
//! The lookup is frozen per step on the step's starting surface, like the
//! flooded mask (ADR 0004).

use rayon::prelude::*;

use crate::config::LithologyConfig;
use crate::grid::Grid;

pub struct Stratigraphy {
    /// Static structural displacement of the column per cell (m).
    deform: Vec<f64>,
    /// Bottom elevation (column space) of every unit but the last, which
    /// is unbounded below. bottoms[k] = datum − Σ thickness through k.
    bottoms: Vec<f64>,
    k_mult: Vec<f64>,
    kappa_mult: Vec<f64>,
    solubility: Vec<f64>,
}

impl Stratigraphy {
    pub fn build(cfg: &LithologyConfig, grid: &Grid) -> Self {
        let (cx, cy) = ((grid.w as f64 - 1.0) / 2.0, (grid.h as f64 - 1.0) / 2.0);
        let deform: Vec<f64> = (0..grid.n() as u32)
            .into_par_iter()
            .map(|i| {
                let (x, y) = grid.xy(i);
                let px = (x as f64 - cx) * grid.dx;
                let py = (y as f64 - cy) * grid.dx;
                let mut d = cfg.dip[0] * px + cfg.dip[1] * py;
                for f in &cfg.folds {
                    let az = f.azimuth_deg.to_radians();
                    let proj = px * az.cos() + py * az.sin();
                    d += f.amplitude_m
                        * (std::f64::consts::TAU * proj / f.wavelength_m + f.phase_rad).sin();
                }
                for f in &cfg.faults {
                    let az = f.azimuth_deg.to_radians();
                    // Left of the trace = toward +90° from the azimuth.
                    let side = (px - f.x_m) * (-az.sin()) + (py - f.y_m) * az.cos();
                    if side > 0.0 {
                        d += f.throw_m;
                    }
                }
                d
            })
            .collect();
        let mut bottoms = Vec::with_capacity(cfg.units.len().saturating_sub(1));
        let mut z = cfg.datum_m;
        for u in cfg.units.iter().take(cfg.units.len() - 1) {
            z -= u.thickness_m;
            bottoms.push(z);
        }
        Self {
            deform,
            bottoms,
            k_mult: cfg.units.iter().map(|u| u.k_mult).collect(),
            kappa_mult: cfg.units.iter().map(|u| u.kappa_mult).collect(),
            solubility: cfg.units.iter().map(|u| u.solubility).collect(),
        }
    }

    pub fn n_units(&self) -> usize {
        self.k_mult.len()
    }

    /// Unit exposed at cell `i` for a bedrock top `z_bedrock` and the
    /// cell's applied-uplift sum. The top unit extends upward, the last
    /// downward.
    #[inline]
    pub fn exposed_unit(&self, i: usize, z_bedrock: f64, cum_uplift_m: f64) -> u32 {
        let s = z_bedrock - cum_uplift_m - self.deform[i];
        for (k, &b) in self.bottoms.iter().enumerate() {
            if s >= b {
                return k as u32;
            }
        }
        self.bottoms.len() as u32
    }

    /// Exposed unit per cell on a surface (parallel pure map).
    pub fn exposure(&self, h: &[f64], sediment_m: &[f64], cum_uplift_m: &[f64]) -> Vec<u32> {
        (0..h.len())
            .into_par_iter()
            .map(|i| self.exposed_unit(i, h[i] - sediment_m[i], cum_uplift_m[i]))
            .collect()
    }

    #[inline]
    pub fn k_mult(&self, unit: u32) -> f64 {
        self.k_mult[unit as usize]
    }

    #[inline]
    pub fn kappa_mult(&self, unit: u32) -> f64 {
        self.kappa_mult[unit as usize]
    }

    #[inline]
    pub fn solubility(&self, unit: u32) -> f64 {
        self.solubility[unit as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Fault, FoldTrain, LithologyConfig, RockUnit};

    fn two_units() -> Vec<RockUnit> {
        vec![
            RockUnit {
                thickness_m: 100.0,
                k_mult: 1.0,
                ..RockUnit::default()
            },
            RockUnit {
                k_mult: 0.25,
                ..RockUnit::default()
            },
        ]
    }

    /// Flat column: the unit boundary sits exactly at datum − thickness,
    /// and pure uplift does not change the exposed unit (material frame).
    #[test]
    fn material_frame_and_boundaries() {
        let grid = Grid::new(16, 100.0);
        let cfg = LithologyConfig {
            units: two_units(),
            ..LithologyConfig::default()
        };
        let s = Stratigraphy::build(&cfg, &grid);
        let i = grid.n() / 2;
        assert_eq!(s.exposed_unit(i, 0.0, 0.0), 0, "at datum: top unit");
        assert_eq!(s.exposed_unit(i, -100.0, 0.0), 0, "boundary belongs up");
        assert_eq!(s.exposed_unit(i, -100.1, 0.0), 1, "below: unit 1");
        assert_eq!(s.exposed_unit(i, 500.0, 0.0), 0, "top unit extends up");
        // 300 m of rock uplifted, surface eroded to 250: exhumation has
        // cut 50 m INTO the column top... s = 250 − 300 = −50 → unit 0.
        assert_eq!(s.exposed_unit(i, 250.0, 300.0), 0);
        // Deeper erosion: surface at 150 with 300 m uplifted → s = −150.
        assert_eq!(s.exposed_unit(i, 150.0, 300.0), 1);
    }

    /// Dip tilts the boundary across the grid; a fault offsets one side.
    #[test]
    fn dip_and_fault_displace_the_column() {
        let grid = Grid::new(16, 100.0);
        let cfg = LithologyConfig {
            units: two_units(),
            dip: [0.1, 0.0], // rises 0.1 m per m toward +x
            faults: vec![Fault {
                x_m: 0.0,
                y_m: 0.0,
                azimuth_deg: 0.0, // trace along +x; +y side is thrown
                throw_m: 200.0,
            }],
            ..LithologyConfig::default()
        };
        let s = Stratigraphy::build(&cfg, &grid);
        // Two cells on the same row (y = 4, below the fault trace at
        // centre y = 7.5 → side < 0, no throw), far left vs far right:
        // the column is displaced up by dip·px, so at the same world z
        // the right cell sits deeper in the column.
        let left = grid.idx(1, 4) as usize;
        let right = grid.idx(14, 4) as usize;
        let z = -60.0;
        assert_eq!(s.exposed_unit(left, z, 0.0), 0, "column low on the left");
        assert_eq!(s.exposed_unit(right, z, 0.0), 1, "column high on the right");
        // Across the fault (same x as `left`, y above centre): +200 m
        // throw pushes the same world z deeper into the column.
        let thrown = grid.idx(1, 12) as usize;
        assert_eq!(
            s.exposed_unit(thrown, z, 0.0),
            1,
            "hanging side sits deeper"
        );
    }

    /// A fold train modulates exposure periodically along its azimuth.
    #[test]
    fn folds_are_periodic() {
        let grid = Grid::new(64, 100.0);
        let cfg = LithologyConfig {
            units: two_units(),
            folds: vec![FoldTrain {
                amplitude_m: 150.0,
                wavelength_m: 1600.0, // 16 cells
                azimuth_deg: 0.0,
                phase_rad: 0.0,
            }],
            ..LithologyConfig::default()
        };
        let s = Stratigraphy::build(&cfg, &grid);
        let y = 32u32;
        let z = -100.0;
        let units: Vec<u32> = (0..64)
            .map(|x| s.exposed_unit(grid.idx(x, y) as usize, z, 0.0))
            .collect();
        // One full wavelength apart → identical exposure.
        for x in 0..48usize {
            assert_eq!(units[x], units[x + 16], "period broken at x={x}");
        }
        // And the fold actually changes exposure somewhere.
        assert!(units.contains(&0) && units.contains(&1));
    }
}
