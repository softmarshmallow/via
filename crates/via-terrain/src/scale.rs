//! Parameters derived from the cell size.
//!
//! The model's defaults were calibrated for 200 m cells under tectonic uplift. At game scale they
//! are wrong: the spike's `basin` (12.5 m cells, no uplift) needed four of them found by trial
//! (`TODO.md`, "The Rust model's defaults assume 200 m cells"). So each scale-dependent parameter
//! is a power law of the cell size through the two calibrated points, 200 m and 12.5 m:
//!
//! `p(c) = p200 * (c / 200) ^ e`, with `e = ln(p12.5 / p200) / ln(12.5 / 200)`.
//!
//! This is a fit through two measurements, not physics: between them it interpolates, beyond them
//! it extrapolates, and a world outside 5 m to 400 m cells should state its own values. Parameters
//! that are not scale-dependent (the MFD exponent, the deposition coefficient, sea level, the
//! climate's coefficients other than the airflow length) keep their defaults.
//!
//! All maths goes through `via-num` (R26), so a preset is the same bits on every system.

use via_num as num;

use crate::config::TerrainConfig;

/// The two calibrated cell sizes, metres.
pub const COARSE_M: f64 = 200.0;
pub const FINE_M: f64 = 12.5;

/// One scale-dependent parameter: its calibrated value at each of the two cell sizes.
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub coarse: f64,
    pub fine: f64,
}

impl Anchor {
    /// The power law through both anchors, at `cell_m`.
    pub fn at(self, cell_m: f64) -> f64 {
        let e = num::ln(self.fine / self.coarse) / num::ln(FINE_M / COARSE_M);
        self.coarse * num::pow(cell_m / COARSE_M, e)
    }
}

/// Stream-power erodibility K (yr⁻¹ m^(1-2m)).
pub const ERODIBILITY: Anchor = Anchor {
    coarse: 5.0e-6,
    fine: 8.0e-6,
};
/// Hillslope diffusivity κ (m²/yr).
pub const DIFFUSIVITY: Anchor = Anchor {
    coarse: 0.05,
    fine: 0.006,
};
/// Timestep (years).
pub const TIMESTEP: Anchor = Anchor {
    coarse: 1.0e4,
    fine: 1.0e3,
};
/// Steps of the run.
pub const STEPS: Anchor = Anchor {
    coarse: 3000.0,
    fine: 160.0,
};
/// Drainage area at which a cell carries a stream (km²).
pub const STREAM_AREA: Anchor = Anchor {
    coarse: 1.5,
    fine: 0.03,
};
/// Drainage area of the fluvial (depositing) domain (km²).
pub const FLUVIAL_AREA: Anchor = Anchor {
    coarse: 0.5,
    fine: 0.008,
};

/// The airflow smoothing length is three cells at both scales.
const AIRFLOW_CELLS: f64 = 3.0;

/// The model's configuration for cells of `cell_m` metres: the defaults, with every
/// scale-dependent parameter taken from its power law. Steps are rounded half to even and are at
/// least 1.
pub fn preset(cell_m: f64) -> TerrainConfig {
    let steps = STEPS.at(cell_m).round_ties_even();
    TerrainConfig {
        cell_size_m: cell_m,
        k_spl: ERODIBILITY.at(cell_m),
        kappa: DIFFUSIVITY.at(cell_m),
        dt_years: TIMESTEP.at(cell_m),
        steps: if steps < 1.0 { 1 } else { steps as u32 },
        river_min_area_km2: STREAM_AREA.at(cell_m),
        fluvial_min_area_km2: FLUVIAL_AREA.at(cell_m),
        airflow_smooth_m: AIRFLOW_CELLS * cell_m,
        ..TerrainConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        let d = a - b;
        let tol = 1e-12 * if b < 0.0 { -b } else { b };
        -tol <= d && d <= tol
    }

    /// Each law passes through both of its anchors.
    #[test]
    fn anchors_are_met() {
        for a in [
            ERODIBILITY,
            DIFFUSIVITY,
            TIMESTEP,
            STEPS,
            STREAM_AREA,
            FLUVIAL_AREA,
        ] {
            assert!(close(a.at(COARSE_M), a.coarse), "{a:?} at 200 m");
            assert!(close(a.at(FINE_M), a.fine), "{a:?} at 12.5 m");
        }
    }

    /// The defaults are the 200 m calibration, so the coarse preset changes none of them.
    #[test]
    fn coarse_preset_is_the_default() {
        let p = preset(COARSE_M);
        let d = TerrainConfig::default();
        assert!(close(p.k_spl, d.k_spl));
        assert!(close(p.kappa, d.kappa));
        assert!(close(p.dt_years, d.dt_years));
        assert_eq!(p.steps, d.steps);
        assert!(close(p.river_min_area_km2, d.river_min_area_km2));
        assert!(close(p.fluvial_min_area_km2, d.fluvial_min_area_km2));
        assert!(close(p.airflow_smooth_m, d.airflow_smooth_m));
    }

    /// The fine preset is the spike's four numbers found by trial at 12.5 m.
    #[test]
    fn fine_preset_is_the_basin_calibration() {
        let p = preset(FINE_M);
        assert!(close(p.k_spl, 8.0e-6));
        assert!(close(p.kappa, 0.006));
        assert!(close(p.dt_years, 1000.0));
        assert_eq!(p.steps, 160);
        assert!(p.validate().is_ok());
    }

    /// Between the anchors every law is monotone in the cell size, and every preset validates.
    #[test]
    fn presets_between_are_monotone_and_valid() {
        let mut last = preset(5.0);
        for c in [6.25, 10.0, 12.5, 25.0, 50.0, 100.0, 200.0, 400.0] {
            let p = preset(c);
            assert!(p.validate().is_ok(), "{c} m");
            assert!(p.kappa > last.kappa && p.dt_years > last.dt_years && p.k_spl < last.k_spl);
            assert!(p.river_min_area_km2 > last.river_min_area_km2);
            last = p;
        }
    }
}
