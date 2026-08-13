use serde::{Deserialize, Serialize};

/// Terrain stage configuration. Every field is a physical quantity or a
/// derivation threshold — nothing here names a landform.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TerrainConfig {
    pub seed: u64,
    /// Grid side length in cells (the grid is square).
    pub size: u32,
    /// Cell edge length in metres.
    pub cell_size_m: f64,
    /// Timestep in years (the implicit fluvial solve is unconditionally
    /// stable; diffusion is subcycled to its own stability limit).
    pub dt_years: f64,
    pub steps: u32,
    /// Stream-power erodibility K in ∂h/∂t = U − K·A^½·S  (m=0.5, n=1 fixed).
    pub k_spl: f64,
    /// Deposition coefficient G (Davy & Lague 2009 in the Yuan et al. 2019
    /// form): deposition rate = G · q_s / Q, applied on the fluvial domain
    /// only. 0 recovers pure detachment-limited SPL; ~1 is the continental
    /// reference. Steady-state *channel* slopes scale by (1+G); K keeps its
    /// detachment-limited calibration because hillslopes — outside the
    /// deposition domain — set most of the relief (ADR 0004).
    pub g_deposition: f64,
    /// Hillslope diffusivity κ in m²/yr.
    pub kappa: f64,
    /// Peak tectonic uplift rate in m/yr; the uplift field scales this.
    pub uplift_max_m_per_yr: f64,
    pub sea_level_m: f64,
    /// Elevation of the initial surface and of the enforced border ring (m).
    pub base_depth_m: f64,
    /// Minimum drop imposed across filled depressions by priority-flood (m).
    pub epsilon_fill_m: f64,
    /// Minimum drainage area for a cell to count as river (km²).
    pub river_min_area_km2: f64,
    /// Minimum drainage area for slope–area regression membership (km²).
    pub fluvial_min_area_km2: f64,
    /// Mean precipitation over land (m/yr); the orographic pattern is
    /// normalized to this.
    pub precip_mean_m_per_yr: f64,
    /// Mean-annual temperature at sea level (°C).
    pub t_sea_level_c: f64,
    /// Atmospheric lapse rate (°C per km of elevation).
    pub lapse_c_per_km: f64,
    /// Total north→south temperature difference across the grid (°C).
    pub t_meridional_delta_c: f64,
    /// Re-saturation rate of air over open water: fraction of the remaining
    /// humidity deficit recovered per cell along the wind. Forcing (ADR
    /// 0003): the moisture-sweep coefficients are boundary conditions and
    /// live in config; only their ratios matter (the field is normalized to
    /// `precip_mean_m_per_yr`).
    pub evaporation_per_cell: f64,
    /// Background (convective) rainout fraction per cell along the wind.
    pub convective_rainout_per_cell: f64,
    /// Orographic rainout fraction per 100 m of forced lift.
    pub orographic_rainout_per_100m: f64,
    /// Airflow smoothing length (m): lift is measured against an
    /// exponential moving average of the terrain so air columns ride over
    /// gullies instead of dipping into each one.
    pub airflow_smooth_m: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            size: 512,
            cell_size_m: 200.0,
            // The explicit deposition pass bounds dt: G·U·dt must stay
            // small against valley relief or the erode/deposit splitting
            // oscillates (ADR 0004). 1e5 was stable for pure detachment;
            // with G = 1 it diverges.
            dt_years: 1.0e4,
            steps: 3000,
            // K keeps its M2 calibration: deposition acts only on the
            // fluvial domain, so hillslopes — which set most of the relief
            // — see the same steady slopes as M2, while channel slopes
            // carry the (1+G) factor. Rescaling K for (1+G) globally
            // flattens the landscape (measured: island relief fell to a
            // third).
            k_spl: 5.0e-6,
            g_deposition: 1.0,
            kappa: 0.05,
            uplift_max_m_per_yr: 5.0e-4,
            sea_level_m: 0.0,
            base_depth_m: -30.0,
            epsilon_fill_m: 1.0e-6,
            river_min_area_km2: 1.0,
            fluvial_min_area_km2: 0.5,
            precip_mean_m_per_yr: 1.2,
            t_sea_level_c: 15.0,
            lapse_c_per_km: 6.5,
            t_meridional_delta_c: 0.6,
            evaporation_per_cell: 0.06,
            convective_rainout_per_cell: 0.0015,
            orographic_rainout_per_100m: 0.055,
            airflow_smooth_m: 600.0,
        }
    }
}

impl TerrainConfig {
    /// Load a config from a JSON file. Omitted fields take defaults
    /// (`#[serde(default)]`), so experiment configs state only what they
    /// change. The core carries no named presets: scale- or game-specific
    /// parameter sets live outside the workspace crates, as files.
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("cannot parse config {}: {e}", path.display()))
    }

    /// A stage that cannot work with what it was given fails loudly
    /// (CONTRIBUTING) — but with a diagnosis, not a numeric symptom.
    pub fn validate(&self) -> Result<(), String> {
        let bad = |msg: &str| Err(format!("invalid terrain config: {msg}"));
        if self.size < 16 {
            return bad("size must be at least 16 cells");
        }
        if self.size > 16384 {
            return bad("size above 16384 cells is not supported (memory)");
        }
        if !(self.cell_size_m.is_finite() && self.cell_size_m > 0.0) {
            return bad("cell_size_m must be finite and positive");
        }
        if !(self.dt_years.is_finite() && self.dt_years > 0.0) {
            return bad("dt_years must be finite and positive");
        }
        if self.steps == 0 {
            return bad("steps must be at least 1");
        }
        if !(self.k_spl.is_finite() && self.k_spl >= 0.0) {
            return bad("k_spl must be finite and non-negative");
        }
        if !(self.kappa.is_finite() && self.kappa >= 0.0) {
            return bad("kappa must be finite and non-negative");
        }
        if !(self.g_deposition.is_finite() && self.g_deposition >= 0.0) {
            return bad("g_deposition must be finite and non-negative");
        }
        if !(self.epsilon_fill_m.is_finite() && self.epsilon_fill_m > 0.0) {
            return bad("epsilon_fill_m must be finite and positive");
        }
        if !(self.base_depth_m.is_finite() && self.base_depth_m < self.sea_level_m) {
            return bad("base_depth_m must be below sea_level_m");
        }
        if !(self.uplift_max_m_per_yr.is_finite() && self.uplift_max_m_per_yr > 0.0) {
            return bad("uplift_max_m_per_yr must be finite and positive");
        }
        if !(self.precip_mean_m_per_yr.is_finite() && self.precip_mean_m_per_yr > 0.0) {
            return bad("precip_mean_m_per_yr must be finite and positive");
        }
        // Load-bearing thresholds: fluvial_min defines the deposition
        // domain (ADR 0004) — zero or negative would put every dry cell in
        // it and reintroduce the hillslope-trap smearing.
        if !(self.river_min_area_km2.is_finite() && self.river_min_area_km2 > 0.0) {
            return bad("river_min_area_km2 must be finite and positive");
        }
        if !(self.fluvial_min_area_km2.is_finite() && self.fluvial_min_area_km2 > 0.0) {
            return bad("fluvial_min_area_km2 must be finite and positive");
        }
        if !(self.lapse_c_per_km.is_finite() && self.t_sea_level_c.is_finite()) {
            return bad("temperature parameters must be finite");
        }
        if !(self.evaporation_per_cell.is_finite()
            && (0.0..=1.0).contains(&self.evaporation_per_cell))
        {
            return bad("evaporation_per_cell must be within [0, 1]");
        }
        if !(self.convective_rainout_per_cell.is_finite()
            && self.convective_rainout_per_cell >= 0.0)
        {
            return bad("convective_rainout_per_cell must be finite and non-negative");
        }
        if !(self.orographic_rainout_per_100m.is_finite()
            && self.orographic_rainout_per_100m >= 0.0)
        {
            return bad("orographic_rainout_per_100m must be finite and non-negative");
        }
        if !(self.airflow_smooth_m.is_finite() && self.airflow_smooth_m > 0.0) {
            return bad("airflow_smooth_m must be finite and positive");
        }
        Ok(())
    }

    pub fn cell_area_m2(&self) -> f64 {
        self.cell_size_m * self.cell_size_m
    }

    pub fn river_min_cells(&self) -> u64 {
        ((self.river_min_area_km2 * 1.0e6) / self.cell_area_m2()).ceil() as u64
    }
}
