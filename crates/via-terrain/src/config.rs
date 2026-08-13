use serde::{Deserialize, Serialize};

/// A rock unit in the stratigraphic column (ADR 0007): mechanical
/// parameters plus an index — never a named rock type (ADR 0003).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RockUnit {
    /// Unit thickness (m). The last (bottom) unit ignores this and
    /// extends downward without bound.
    pub thickness_m: f64,
    /// Erodibility multiplier on k_spl where this unit is exposed.
    pub k_mult: f64,
    /// Hillslope diffusivity multiplier on kappa.
    pub kappa_mult: f64,
    /// Solubility in [0, 1]; karst_potential = solubility × discharge.
    pub solubility: f64,
}

impl Default for RockUnit {
    fn default() -> Self {
        Self {
            thickness_m: 500.0,
            k_mult: 1.0,
            kappa_mult: 1.0,
            solubility: 0.0,
        }
    }
}

/// A sinusoidal fold train displacing the column (tier-2 forcing).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FoldTrain {
    pub amplitude_m: f64,
    pub wavelength_m: f64,
    /// Direction along which the displacement varies, degrees from +x
    /// toward +y.
    pub azimuth_deg: f64,
    pub phase_rad: f64,
}

impl Default for FoldTrain {
    fn default() -> Self {
        Self {
            amplitude_m: 0.0,
            wavelength_m: 10_000.0,
            azimuth_deg: 0.0,
            phase_rad: 0.0,
        }
    }
}

/// A fault: vertical throw across a line trace. The discontinuity stays
/// sharp — its surface expression must emerge from differential erosion.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Fault {
    /// A point on the trace, metres from the grid centre.
    pub x_m: f64,
    pub y_m: f64,
    /// Trace direction, degrees from +x toward +y.
    pub azimuth_deg: f64,
    /// Throw (m) added to the column on the trace's left side (the side
    /// toward +90° from the azimuth).
    pub throw_m: f64,
}

impl Default for Fault {
    fn default() -> Self {
        Self {
            x_m: 0.0,
            y_m: 0.0,
            azimuth_deg: 0.0,
            throw_m: 0.0,
        }
    }
}

/// Lithology & structure (ADR 0007): a deformed layer-cake, declared as
/// tier-2 forcing like the uplift field. The default is a single neutral
/// unit — behaviorally identical to the pre-M4 homogeneous substrate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LithologyConfig {
    /// Elevation (m) of the top of unit 0 in undeformed column space.
    pub datum_m: f64,
    /// Units listed top-down from the datum. The top unit also extends
    /// upward without bound.
    pub units: Vec<RockUnit>,
    /// Regional dip: column displacement gains dip[0]·x + dip[1]·y
    /// (x, y metres from the grid centre; components are gradients,
    /// m per m).
    pub dip: [f64; 2],
    pub folds: Vec<FoldTrain>,
    pub faults: Vec<Fault>,
    /// K multiplier where the frozen sediment cover exceeds
    /// `sediment_cover_min_m` (Davy & Lague K contrast). 1 = neutral.
    pub sediment_k_mult: f64,
    pub sediment_cover_min_m: f64,
}

impl Default for LithologyConfig {
    fn default() -> Self {
        Self {
            datum_m: 0.0,
            units: vec![RockUnit::default()],
            dip: [0.0, 0.0],
            folds: Vec::new(),
            faults: Vec::new(),
            sediment_k_mult: 1.0,
            sediment_cover_min_m: 1.0,
        }
    }
}

impl LithologyConfig {
    /// Some(shared multiplier) when every unit diffuses alike — the
    /// uniform-κ fast path that keeps homogeneous runs bitwise identical.
    pub fn uniform_kappa_mult(&self) -> Option<f64> {
        let first = self.units.first().map(|u| u.kappa_mult)?;
        self.units
            .iter()
            .all(|u| u.kappa_mult == first)
            .then_some(first)
    }
}

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
    /// MFD partition exponent p (Freeman 1991): flow to each downslope
    /// neighbour ∝ slope^p. Discretization parameter, not forcing
    /// (ADR 0005); 1.1 is Freeman's calibrated value.
    pub mfd_exponent: f64,
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
    /// Stratigraphic column and structure (ADR 0007); the default single
    /// neutral unit reproduces the homogeneous substrate exactly.
    pub lithology: LithologyConfig,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            size: 512,
            cell_size_m: 200.0,
            // dt is bounded by per-step topology freezing, not by the
            // solver: routing, climate, and the flooded mask are frozen
            // within a step, so the standing sediment blanket scales with
            // the per-step deposit lump (measured at 30 Myr: mean blanket
            // 1.4 / 2.6 / 6.0 m at dt 1e4 / 2e4 / 5e4 — no convergence
            // plateau; ADR 0006). The implicit solve removed the explosive
            // splitting oscillation of ADR 0004, so larger dt now degrades
            // gracefully instead of diverging — but the research preset
            // keeps dt at its accuracy limit.
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
            mfd_exponent: 1.1,
            uplift_max_m_per_yr: 5.0e-4,
            sea_level_m: 0.0,
            base_depth_m: -30.0,
            epsilon_fill_m: 1.0e-6,
            // Extraction threshold recalibrated for the MFD discharge
            // field (ADR 0005): the smoother hybrid flux crosses a given
            // threshold at more marginal heads, inflating first-order
            // stream counts; 1.5 km² selects a network comparable to the
            // 1.0 km² D8 one.
            river_min_area_km2: 1.5,
            fluvial_min_area_km2: 0.5,
            precip_mean_m_per_yr: 1.2,
            t_sea_level_c: 15.0,
            lapse_c_per_km: 6.5,
            t_meridional_delta_c: 0.6,
            evaporation_per_cell: 0.06,
            convective_rainout_per_cell: 0.0015,
            orographic_rainout_per_100m: 0.055,
            airflow_smooth_m: 600.0,
            lithology: LithologyConfig::default(),
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
        if !(self.mfd_exponent.is_finite() && self.mfd_exponent > 0.0) {
            return bad("mfd_exponent must be finite and positive");
        }
        if !(self.epsilon_fill_m.is_finite() && self.epsilon_fill_m >= 1.0e-9) {
            // Below ~1e-9, hc + ε rounds back to hc on kilometre-scale
            // terrain and priority-flood stops guaranteeing strict
            // descent — stranding cells with no downslope neighbour.
            return bad("epsilon_fill_m must be finite and at least 1e-9");
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
        let lith = &self.lithology;
        if lith.units.is_empty() {
            return bad("lithology.units must hold at least one unit");
        }
        if lith.units.len() > 64 {
            return bad("lithology.units above 64 units is not supported");
        }
        for (k, u) in lith.units.iter().enumerate() {
            // The last unit's thickness is unused (unbounded basement),
            // but a nonsense value is still a config error worth naming.
            if !(u.thickness_m.is_finite() && u.thickness_m > 0.0) {
                return bad(&format!(
                    "lithology.units[{k}].thickness_m must be finite and positive"
                ));
            }
            if !(u.k_mult.is_finite() && u.k_mult > 0.0) {
                return bad(&format!(
                    "lithology.units[{k}].k_mult must be finite and positive"
                ));
            }
            if !(u.kappa_mult.is_finite() && u.kappa_mult >= 0.0) {
                return bad(&format!(
                    "lithology.units[{k}].kappa_mult must be finite and non-negative"
                ));
            }
            if !(u.solubility.is_finite() && (0.0..=1.0).contains(&u.solubility)) {
                return bad(&format!(
                    "lithology.units[{k}].solubility must lie in [0, 1]"
                ));
            }
        }
        if !(lith.datum_m.is_finite() && lith.dip[0].is_finite() && lith.dip[1].is_finite()) {
            return bad("lithology datum and dip must be finite");
        }
        for (k, f) in lith.folds.iter().enumerate() {
            if !(f.amplitude_m.is_finite() && f.amplitude_m >= 0.0) {
                return bad(&format!(
                    "lithology.folds[{k}].amplitude_m must be finite and non-negative"
                ));
            }
            if !(f.wavelength_m.is_finite() && f.wavelength_m > 0.0) {
                return bad(&format!(
                    "lithology.folds[{k}].wavelength_m must be finite and positive"
                ));
            }
            if !(f.azimuth_deg.is_finite() && f.phase_rad.is_finite()) {
                return bad(&format!(
                    "lithology.folds[{k}] azimuth and phase must be finite"
                ));
            }
        }
        for (k, f) in lith.faults.iter().enumerate() {
            if !(f.x_m.is_finite()
                && f.y_m.is_finite()
                && f.azimuth_deg.is_finite()
                && f.throw_m.is_finite())
            {
                return bad(&format!("lithology.faults[{k}] parameters must be finite"));
            }
        }
        if !(lith.sediment_k_mult.is_finite() && lith.sediment_k_mult > 0.0) {
            return bad("lithology.sediment_k_mult must be finite and positive");
        }
        if !(lith.sediment_cover_min_m.is_finite() && lith.sediment_cover_min_m >= 0.0) {
            return bad("lithology.sediment_cover_min_m must be finite and non-negative");
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
