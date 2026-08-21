//! via-corridors — the era-0 corridor stage (ADR 0012).
//!
//! Tier-3 interpretation over declared Tier-2 cost forcing: the stage
//! computes where least-cost movement concentrates (FETE density), how
//! the terrain's gateways interconnect (the trunk network), and
//! accumulated-time fields, under a declared, cited movement model.
//! No realism is claimed; every water-derived number is conditional on
//! the terrain stage's k_Q forcing scale, and the summary carries a
//! degeneracy panel making an inert mode set visible (ADR 0012 D3).

pub mod cost;
pub mod fete;
pub mod solve;
pub mod trunk;

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use via_artifact::{ArtifactEntry, Raster, RunManifest, StageRecord};

use cost::{ModelInputs, MoveModel};
use trunk::{StepMode, TrunkResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CorridorsConfig {
    /// Output namespace label (`corridors.<label>.*`).
    pub label: String,
    /// Which suitability config label's artifacts to consume.
    pub suitability_label: String,
    /// FETE lattice spacing in cells; None = max(grid side / 32, 1),
    /// the size-relative default (ADR 0012 D4).
    pub lattice_spacing_cells: Option<u32>,
    /// Stahlberg multiplicative reuse discount on built land edges
    /// (default the midpoint of the fitted [0.4, 0.5]; single-region
    /// fit, portability flagged).
    pub reuse_alpha: f64,
    /// Declared land speed factor (Tobler's ×0.6 off-path knob;
    /// default 1.0 = the base on-path function, Livingood's
    /// precedent).
    pub offpath_factor: f64,
    /// Livingood 2012 canoe base speed (defensible band 3.5–5).
    pub canoe_speed_kmh: f64,
    /// Whether the coastal-water ribbon is admissible (declared
    /// forcing; no citable era-0 coastal speed exists).
    pub coastal_mode: bool,
    /// Declared coastal speed (cabotage parity with the canoe base).
    pub coastal_speed_kmh: f64,
    /// Optional fetch cap closing exposed coast to era-0 craft.
    pub coastal_exposure_cap_m: Option<f64>,
    /// Flat land↔water switch penalty (declared forcing; Livingood
    /// Table 10.1 half-crossing provenance).
    pub transship_hours: f64,
    /// Wading delay per entered or crossed channel cell (declared
    /// forcing; Livingood Table 10.1 crossing bands).
    pub ford_delay_hours: f64,
    /// People-stability caps, independent, applied to the crossing-flow
    /// depth and velocity (ADR 0012 D3 as amended on research 0015).
    /// Defaults are the traveller envelope — the bounded Ontario form
    /// (D·V ≤ 0.4 m²/s, d ≤ 0.8 m, v ≤ 1.7 m/s) with the depth cap set
    /// by the pedestrian ford standard the USFS Trail Notebook and
    /// Motayed et al. (1982) converge on (0.4–0.6 m at typical flow).
    /// The Cox/AIDR flood-stability bands (0.8/1.2/3.0) describe
    /// trained staff with a rod, cleats and a tag line, and are the
    /// wrong end of the literature for a loaded traveller.
    pub ford_max_dv_m2s: f64,
    pub ford_max_depth_m: f64,
    pub ford_max_velocity_ms: f64,
    /// Herzog sextic evaluation clamp (the ±0.45 fit range; via's
    /// declared inference).
    pub herzog_clamp: f64,
    /// Frozen L&S critical gradients for the energy envelope
    /// (+0.28 / −0.22 m/m; ADR 0012 D2 min form).
    pub ls_crit_up: f64,
    pub ls_crit_down: f64,
}

impl Default for CorridorsConfig {
    fn default() -> Self {
        CorridorsConfig {
            label: "site".into(),
            suitability_label: "site".into(),
            lattice_spacing_cells: None,
            reuse_alpha: 0.45,
            offpath_factor: 1.0,
            canoe_speed_kmh: 4.0,
            coastal_mode: true,
            coastal_speed_kmh: 4.0,
            coastal_exposure_cap_m: None,
            transship_hours: 0.25,
            ford_delay_hours: 0.25,
            ford_max_dv_m2s: 0.4,
            ford_max_depth_m: 0.6,
            ford_max_velocity_ms: 1.7,
            herzog_clamp: 0.45,
            ls_crit_up: 0.28,
            ls_crit_down: -0.22,
        }
    }
}

fn bad(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

pub fn validate_label(label: &str) -> io::Result<()> {
    if label.is_empty()
        || !label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(bad(format!(
            "config label {label:?} must be non-empty [a-z0-9_-]+"
        )));
    }
    Ok(())
}

impl CorridorsConfig {
    pub fn from_json_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn validate(&self) -> io::Result<()> {
        validate_label(&self.label)?;
        validate_label(&self.suitability_label)?;
        // Physical-scale bounds, not just positivity: costs spanning
        // more than ~10 orders of magnitude would let a move's cost
        // vanish in f64 addition against an accumulated distance,
        // which breaks the strict ordering the solver and the density
        // accumulation rest on.
        let pos = |name: &str, v: f64| -> io::Result<()> {
            if !(v.is_finite() && (1.0e-6..=1.0e6).contains(&v)) {
                return Err(bad(format!(
                    "{name} must be finite and within [1e-6, 1e6], got {v}"
                )));
            }
            Ok(())
        };
        if let Some(sp) = self.lattice_spacing_cells {
            if sp == 0 {
                return Err(bad("lattice_spacing_cells must be >= 1".into()));
            }
        }
        if !(self.reuse_alpha.is_finite() && self.reuse_alpha > 0.0 && self.reuse_alpha <= 1.0) {
            return Err(bad(format!(
                "reuse_alpha must be in (0, 1], got {}",
                self.reuse_alpha
            )));
        }
        pos("offpath_factor", self.offpath_factor)?;
        pos("canoe_speed_kmh", self.canoe_speed_kmh)?;
        pos("coastal_speed_kmh", self.coastal_speed_kmh)?;
        if let Some(cap) = self.coastal_exposure_cap_m {
            pos("coastal_exposure_cap_m", cap)?;
        }
        pos("transship_hours", self.transship_hours)?;
        if !(self.ford_delay_hours.is_finite() && (0.0..=1.0e6).contains(&self.ford_delay_hours)) {
            return Err(bad(format!(
                "ford_delay_hours must be finite and within [0, 1e6], got {}",
                self.ford_delay_hours
            )));
        }
        pos("ford_max_dv_m2s", self.ford_max_dv_m2s)?;
        pos("ford_max_depth_m", self.ford_max_depth_m)?;
        pos("ford_max_velocity_ms", self.ford_max_velocity_ms)?;
        pos("herzog_clamp", self.herzog_clamp)?;
        if !(self.ls_crit_up.is_finite()
            && self.ls_crit_up > 0.0
            && self.ls_crit_up <= self.herzog_clamp)
        {
            return Err(bad(format!(
                "ls_crit_up must be in (0, herzog_clamp], got {}",
                self.ls_crit_up
            )));
        }
        if !(self.ls_crit_down.is_finite()
            && self.ls_crit_down < 0.0
            && self.ls_crit_down >= -self.herzog_clamp)
        {
            return Err(bad(format!(
                "ls_crit_down must be in [-herzog_clamp, 0), got {}",
                self.ls_crit_down
            )));
        }
        Ok(())
    }
}

pub fn raster_filename(label: &str, name: &str) -> String {
    format!("corridors.{label}.{name}.vrast")
}

pub fn summary_filename(label: &str) -> String {
    format!("corridors.{label}.json")
}

/// Largest value a reachable cell may carry in an hours raster: the
/// f32 immediately below `f32::MAX`, which is the unreachable/no-node
/// sentinel.
const REACHABLE_MAX_HOURS: f32 = f32::from_bits(f32::MAX.to_bits() - 1);

const RASTER_NAMES: [&str; 5] = [
    "corridor_density",
    "trunk",
    "hours_to_sea_land",
    "hours_to_sea_water",
    "hours_to_trunk",
];

#[derive(Clone, Debug, Serialize)]
pub struct Degeneracy {
    pub river_cells: u64,
    pub ford_passable_channel_fraction: f64,
    pub navigable_channel_fraction: f64,
    pub land_moves: u64,
    pub water_moves: u64,
    pub switch_moves: u64,
}

pub struct CorridorsOutput {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub lattice_spacing: u32,
    pub sources: Vec<u32>,
    pub density: Vec<u32>,
    pub trunk: TrunkResult,
    pub hours_to_sea_land: Vec<f32>,
    pub hours_to_sea_water: Vec<f32>,
    pub hours_to_trunk: Vec<f32>,
    pub degeneracy: Degeneracy,
}

struct Inputs {
    w: u32,
    h: u32,
    dx: f64,
    heights_m: Vec<f64>,
    receivers: Vec<u32>,
    strahler: Vec<u32>,
    water_depth: Vec<f64>,
    navigable: Vec<u32>,
    crossability: Vec<f32>,
    ford_depth: Vec<f32>,
    ford_velocity: Vec<f32>,
    harbour_fetch: Vec<f32>,
    pass_cells: Vec<u32>,
    head_cells: Vec<u32>,
}

fn ensure_grid<T: via_artifact::Scalar>(
    name: &str,
    r: &Raster<T>,
    reference: &Raster<i32>,
) -> io::Result<()> {
    if r.width != reference.width
        || r.height != reference.height
        || r.cell_size_cm != reference.cell_size_cm
    {
        return Err(bad(format!(
            "{name}: grid {}x{} cell {} does not match heights_cm {}x{} cell {} (stale or mixed run directory?)",
            r.width, r.height, r.cell_size_cm, reference.width, reference.height, reference.cell_size_cm
        )));
    }
    Ok(())
}

fn load_inputs(run_dir: &Path, cfg: &CorridorsConfig) -> io::Result<Inputs> {
    let heights: Raster<i32> = Raster::read_file(&run_dir.join("heights_cm.vrast"))?;
    let receivers: Raster<u32> = Raster::read_file(&run_dir.join("receivers.vrast"))?;
    let strahler: Raster<u32> = Raster::read_file(&run_dir.join("strahler.vrast"))?;
    let water_depth: Raster<f32> = Raster::read_file(&run_dir.join("water_depth.vrast"))?;
    ensure_grid("receivers", &receivers, &heights)?;
    ensure_grid("strahler", &strahler, &heights)?;
    ensure_grid("water_depth", &water_depth, &heights)?;
    let sl = &cfg.suitability_label;
    let s_raster = |name: &str| -> io::Result<Raster<f32>> {
        let r: Raster<f32> = Raster::read_file(&run_dir.join(via_suit_raster_filename(sl, name)))?;
        ensure_grid(name, &r, &heights)?;
        Ok(r)
    };
    let navigable: Raster<u32> =
        Raster::read_file(&run_dir.join(via_suit_raster_filename(sl, "navigable")))?;
    ensure_grid("navigable", &navigable, &heights)?;
    let crossability = s_raster("crossability")?;
    let ford_depth = s_raster("ford_depth")?;
    let ford_velocity = s_raster("ford_velocity")?;
    let harbour_fetch = s_raster("harbour_fetch")?;

    let n = (heights.width * heights.height) as usize;
    for (i, &r) in receivers.data.iter().enumerate() {
        if r as usize >= n {
            return Err(bad(format!(
                "receivers[{i}] = {r} out of range for {n} cells (corrupt artifact?)"
            )));
        }
    }

    // Site cells from the suitability summary (a disk artifact for
    // this purpose — ADR 0012 D6).
    let summary_path = run_dir.join(format!("suitability.{sl}.json"));
    let text = std::fs::read_to_string(&summary_path)
        .map_err(|e| bad(format!("{}: {e}", summary_path.display())))?;
    let summary: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| bad(format!("{}: {e}", summary_path.display())))?;
    // Every step is a hard error: a summary whose site arrays are
    // missing, renamed, or malformed would otherwise yield an empty
    // node set and a trunk network that passes every gate while
    // silently omitting passes or heads (ADR 0012 D6: unevaluable is
    // fail).
    let cells_of = |path: &[&str]| -> io::Result<Vec<u32>> {
        let mut node = &summary;
        for key in path {
            node = node.get(key).ok_or_else(|| {
                bad(format!(
                    "{}: missing {} (stale or foreign suitability summary?)",
                    summary_path.display(),
                    path.join(".")
                ))
            })?;
        }
        let arr = node.as_array().ok_or_else(|| {
            bad(format!(
                "{}: {} is not an array",
                summary_path.display(),
                path.join(".")
            ))
        })?;
        arr.iter()
            .map(|s| {
                let cell = s.get("cell").and_then(|c| c.as_u64()).ok_or_else(|| {
                    bad(format!(
                        "{}: a {} entry has no numeric `cell`",
                        summary_path.display(),
                        path.join(".")
                    ))
                })?;
                if cell as usize >= n {
                    return Err(bad(format!(
                        "{}: {} cell {cell} out of range for {n} cells",
                        summary_path.display(),
                        path.join(".")
                    )));
                }
                Ok(cell as u32)
            })
            .collect()
    };
    let pass_cells = cells_of(&["affordances", "passes", "sites"])?;
    let head_cells = cells_of(&["affordances", "navigability", "head_of_navigation_sites"])?;

    Ok(Inputs {
        w: heights.width,
        h: heights.height,
        dx: heights.cell_size_cm as f64 / 100.0,
        heights_m: heights.data.iter().map(|&c| c as f64 / 100.0).collect(),
        receivers: receivers.data,
        strahler: strahler.data,
        water_depth: water_depth.data.iter().map(|&d| d as f64).collect(),
        navigable: navigable.data,
        crossability: crossability.data,
        ford_depth: ford_depth.data,
        ford_velocity: ford_velocity.data,
        harbour_fetch: harbour_fetch.data,
        pass_cells,
        head_cells,
    })
}

fn via_suit_raster_filename(label: &str, name: &str) -> String {
    format!("suitability.{label}.{name}.vrast")
}

fn resolve_spacing(cfg: &CorridorsConfig, w: u32, h: u32) -> io::Result<u32> {
    let sp = cfg
        .lattice_spacing_cells
        .unwrap_or_else(|| (w.max(h) / 32).max(1));
    // The directed-pair-count u32 guard (ADR 0012 D4): bound the
    // source count by the lattice size.
    let bound = (w.div_ceil(sp) as u64) * (h.div_ceil(sp) as u64);
    if bound.saturating_mul(bound.saturating_sub(1)) > u32::MAX as u64 {
        return Err(bad(format!(
            "lattice spacing {sp} admits up to {bound} sources; the directed pair count would overflow the u32 density raster — raise lattice_spacing_cells"
        )));
    }
    Ok(sp)
}

fn compute(inp: &Inputs, cfg: &CorridorsConfig) -> io::Result<CorridorsOutput> {
    let model_inputs = ModelInputs {
        w: inp.w,
        h: inp.h,
        dx: inp.dx,
        heights_m: &inp.heights_m,
        receivers: &inp.receivers,
        strahler: &inp.strahler,
        water_depth: &inp.water_depth,
        navigable: &inp.navigable,
        crossability: &inp.crossability,
        ford_depth: &inp.ford_depth,
        ford_velocity: &inp.ford_velocity,
        harbour_fetch: &inp.harbour_fetch,
    };
    let model = MoveModel::build(&model_inputs, cfg);
    let spacing = resolve_spacing(cfg, inp.w, inp.h)?;
    let sources = fete::lattice_sources(&model, spacing);
    let density = fete::density(&model, &sources);

    // Trunk: passes ∪ heads ∪ mouths.
    let mouths = trunk::river_mouths(inp.w, inp.h, &inp.receivers, &inp.strahler);
    let nodes = trunk::node_set(&model, &inp.pass_cells, &inp.head_cells, &mouths);
    let trunk = trunk::build(&model, nodes, cfg.reuse_alpha);

    // hours_to_sea: tidewater sources (coastal-water nodes + land
    // nodes of ocean-adjacent cells) at zero.
    let n = model.n_cells();
    let n_u32 = n as u32;
    let mut tidewater: Vec<(u32, f64)> = Vec::new();
    for i in 0..n {
        if model.coastal[i] && model.water_node[i] {
            tidewater.push((n_u32 + i as u32, 0.0));
        }
        if model.land_node[i] {
            let mut shore = false;
            cost::for_neighbors8(inp.w, inp.h, i as u32, |j, _| {
                if model.ocean[j as usize] {
                    shore = true;
                }
            });
            if shore {
                tidewater.push((i as u32, 0.0));
            }
        }
    }
    let sea = solve::dijkstra(&model, &tidewater, None, None);
    let project = |sol: &solve::Solution, water: bool| -> Vec<f32> {
        (0..n)
            .map(|i| {
                let node = if water { n + i } else { i };
                let exists = if water {
                    model.water_node[i]
                } else {
                    model.land_node[i]
                };
                let d = sol.dist[node];
                if exists && d.is_finite() {
                    // Keep every real value strictly below the
                    // sentinel: an f64 beyond f32 range would
                    // otherwise cast to +inf and escape it, and a
                    // value in the top rounding window would collide
                    // with it.
                    (d as f32).min(REACHABLE_MAX_HOURS)
                } else {
                    f32::MAX
                }
            })
            .collect()
    };
    let hours_to_sea_land = project(&sea, false);
    let hours_to_sea_water = project(&sea, true);

    // hours_to_trunk: every trunk-path step, in its traversal mode.
    let mut trunk_sources: Vec<(u32, f64)> = Vec::new();
    {
        let mut seen = vec![false; model.n_nodes()];
        for e in &trunk.edges {
            for &(cell, mode) in &e.path {
                let node = match mode {
                    StepMode::Land => cell,
                    StepMode::Water => n_u32 + cell,
                };
                if !seen[node as usize] {
                    seen[node as usize] = true;
                    trunk_sources.push((node, 0.0));
                }
            }
        }
        trunk_sources.sort_unstable_by_key(|&(node, _)| node);
    }
    let to_trunk = solve::dijkstra(&model, &trunk_sources, None, None);
    let hours_to_trunk = project(&to_trunk, false);

    // Degeneracy panel (ADR 0012 D3).
    let mut river_cells = 0u64;
    let mut passable = 0u64;
    let mut nav_channel = 0u64;
    for i in 0..n {
        if inp.strahler[i] > 0 && !model.ocean[i] && inp.water_depth[i] == 0.0 {
            river_cells += 1;
            if model.land_node[i] {
                passable += 1;
            }
            if inp.navigable[i] == 1 {
                nav_channel += 1;
            }
        }
    }
    let (mut land_moves, mut water_moves, mut switch_moves) = (0u64, 0u64, 0u64);
    for node in 0..model.n_nodes() as u32 {
        let from_water = model.is_water_node(node);
        model.for_moves(node, |m| {
            let to_water = model.is_water_node(m.to);
            match (from_water, to_water) {
                (false, false) => land_moves += 1,
                (true, true) => water_moves += 1,
                _ => switch_moves += 1,
            }
        });
    }
    let frac = |num: u64, den: u64| {
        if den == 0 {
            0.0
        } else {
            num as f64 / den as f64
        }
    };
    Ok(CorridorsOutput {
        w: inp.w,
        h: inp.h,
        dx: inp.dx,
        lattice_spacing: spacing,
        sources,
        density,
        trunk,
        hours_to_sea_land,
        hours_to_sea_water,
        hours_to_trunk,
        degeneracy: Degeneracy {
            river_cells,
            ford_passable_channel_fraction: frac(passable, river_cells),
            navigable_channel_fraction: frac(nav_channel, river_cells),
            land_moves,
            water_moves,
            switch_moves,
        },
    })
}

/// Run the stage: load inputs, verify manifest preconditions, compute.
pub fn run(run_dir: &Path, cfg: &CorridorsConfig) -> io::Result<CorridorsOutput> {
    cfg.validate()?;
    let manifest = RunManifest::load(&run_dir.join("manifest.json"))?;
    if manifest.stage("terrain").is_none() {
        return Err(bad("manifest has no terrain stage record".into()));
    }
    let skey = format!("suitability.{}", cfg.suitability_label);
    if manifest.stage(&skey).is_none() {
        return Err(bad(format!(
            "manifest has no {skey} stage record — run `via suitability` first"
        )));
    }
    let inp = load_inputs(run_dir, cfg)?;
    compute(&inp, cfg)
}

#[derive(Serialize)]
struct Summary<'a> {
    stage: &'static str,
    config: &'a CorridorsConfig,
    provenance: BTreeMap<&'static str, &'static str>,
    metric_values_conditional_on_k_q: bool,
    degeneracy: &'a Degeneracy,
    lattice: LatticeInfo,
    trunk: TrunkSummary<'a>,
    checks: BTreeMap<&'static str, bool>,
    artifact_blake3: &'a BTreeMap<String, String>,
}

#[derive(Serialize)]
struct LatticeInfo {
    spacing_cells: u32,
    sources: u32,
    directed_pairs: u64,
}

#[derive(Serialize)]
struct TrunkSummary<'a> {
    nodes: &'a [trunk::TrunkNode],
    edges: &'a [trunk::TrunkEdge],
    junctions: &'a [trunk::Junction],
}

/// Write artifacts, run every gate against the bytes on disk, write
/// the summary, and register the stage record in the manifest.
pub fn write_outputs(
    run_dir: &Path,
    cfg: &CorridorsConfig,
    out: &CorridorsOutput,
) -> io::Result<()> {
    let cell_cm = (out.dx * 100.0).round() as u32;
    let path = |name: &str| run_dir.join(raster_filename(&cfg.label, name));
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();

    let write_u32 =
        |name: &str, data: &[u32], hashes: &mut BTreeMap<String, String>| -> io::Result<()> {
            let r = Raster::from_data(out.w, out.h, cell_cm, data.to_vec());
            r.write_file(&path(name))?;
            hashes.insert(name.to_string(), r.blake3_hex());
            Ok(())
        };
    let write_f32 =
        |name: &str, data: &[f32], hashes: &mut BTreeMap<String, String>| -> io::Result<()> {
            let r = Raster::from_data(out.w, out.h, cell_cm, data.to_vec());
            r.write_file(&path(name))?;
            hashes.insert(name.to_string(), r.blake3_hex());
            Ok(())
        };
    write_u32("corridor_density", &out.density, &mut hashes)?;
    write_u32("trunk", &out.trunk.raster, &mut hashes)?;
    write_f32("hours_to_sea_land", &out.hours_to_sea_land, &mut hashes)?;
    write_f32("hours_to_sea_water", &out.hours_to_sea_water, &mut hashes)?;
    write_f32("hours_to_trunk", &out.hours_to_trunk, &mut hashes)?;

    // ---- Gates (ADR 0012 D6): full recompute from the disk-read
    // inputs through the same code path, byte-compared against the
    // just-written artifacts.
    let inp = load_inputs(run_dir, cfg)?;
    let recomputed = compute(&inp, cfg)?;
    let gate = |name: &str, ok: bool| -> io::Result<()> {
        if !ok {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("gate failed: {name} (unevaluable or mismatch against disk recompute)"),
            ));
        }
        Ok(())
    };
    let read_u32 =
        |name: &str| -> io::Result<Vec<u32>> { Ok(Raster::<u32>::read_file(&path(name))?.data) };
    let read_f32 =
        |name: &str| -> io::Result<Vec<f32>> { Ok(Raster::<f32>::read_file(&path(name))?.data) };
    // Grid agreement: every emitted raster, read back, carries the
    // geometry of the terrain heights it was derived from.
    {
        let heights: Raster<i32> = Raster::read_file(&run_dir.join("heights_cm.vrast"))?;
        for name in RASTER_NAMES {
            let (w, h, cell) = match name {
                "corridor_density" | "trunk" => {
                    let r = Raster::<u32>::read_file(&path(name))?;
                    (r.width, r.height, r.cell_size_cm)
                }
                _ => {
                    let r = Raster::<f32>::read_file(&path(name))?;
                    (r.width, r.height, r.cell_size_cm)
                }
            };
            gate(
                "grid_agreement",
                (w, h, cell) == (heights.width, heights.height, heights.cell_size_cm),
            )?;
        }
    }
    gate(
        "density_definition",
        read_u32("corridor_density")? == recomputed.density,
    )?;
    gate(
        "trunk_definition",
        read_u32("trunk")? == recomputed.trunk.raster,
    )?;
    let f32_eq = |a: &[f32], b: &[f32]| {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
    };
    gate(
        "hours_to_sea_definition",
        f32_eq(
            &read_f32("hours_to_sea_land")?,
            &recomputed.hours_to_sea_land,
        ) && f32_eq(
            &read_f32("hours_to_sea_water")?,
            &recomputed.hours_to_sea_water,
        ),
    )?;
    gate(
        "hours_to_trunk_definition",
        f32_eq(&read_f32("hours_to_trunk")?, &recomputed.hours_to_trunk),
    )?;
    // Density bounds.
    let s = recomputed.sources.len() as u64;
    let pairs = s * s.saturating_sub(1);
    gate(
        "density_bounds",
        out.density.iter().all(|&d| (d as u64) <= pairs),
    )?;
    // Every source reachable from at least one other source carries
    // >= 1 (its own outgoing paths traverse it).
    {
        let model_inputs = ModelInputs {
            w: inp.w,
            h: inp.h,
            dx: inp.dx,
            heights_m: &inp.heights_m,
            receivers: &inp.receivers,
            strahler: &inp.strahler,
            water_depth: &inp.water_depth,
            navigable: &inp.navigable,
            crossability: &inp.crossability,
            ford_depth: &inp.ford_depth,
            ford_velocity: &inp.ford_velocity,
            harbour_fetch: &inp.harbour_fetch,
        };
        let model = MoveModel::build(&model_inputs, cfg);
        for &src in &recomputed.sources {
            if out.density[src as usize] == 0 {
                // Only legitimate for an isolated source: verify.
                let sol = solve::dijkstra(&model, &[(src, 0.0)], None, None);
                let reachable = recomputed
                    .sources
                    .iter()
                    .any(|&t| t != src && sol.dist[t as usize].is_finite());
                gate("density_source_coverage", !reachable)?;
            }
        }
        // Trunk identities: stored paths connected, admissible, and
        // exactly re-costed; endpoints equal the recomputed node set;
        // junctions equal the union-graph recomputation.
        let rec_nodes_json = serde_json::to_string(&recomputed.trunk.nodes)?;
        let out_nodes_json = serde_json::to_string(&out.trunk.nodes)?;
        gate("trunk_node_set", rec_nodes_json == out_nodes_json)?;
        let rec_edges_json = serde_json::to_string(&recomputed.trunk.edges)?;
        let out_edges_json = serde_json::to_string(&out.trunk.edges)?;
        gate("trunk_edges", rec_edges_json == out_edges_json)?;
        let rec_junctions_json = serde_json::to_string(&recomputed.trunk.junctions)?;
        let out_junctions_json = serde_json::to_string(&out.trunk.junctions)?;
        gate("trunk_junctions", rec_junctions_json == out_junctions_json)?;
        for e in &out.trunk.edges {
            let mut prev: Option<(u32, StepMode)> = None;
            let mut hours = 0.0;
            for &(cell, mode) in &e.path {
                if let Some((pc, pm)) = prev {
                    let from = match pm {
                        StepMode::Land => pc,
                        StepMode::Water => inp.w * inp.h + pc,
                    };
                    let to = match mode {
                        StepMode::Land => cell,
                        StepMode::Water => inp.w * inp.h + cell,
                    };
                    match model.move_cost(from, to) {
                        Some(c) => hours += c,
                        None => gate("trunk_path_admissible", false)?,
                    }
                }
                prev = Some((cell, mode));
            }
            gate("trunk_path_cost", hours == e.hours_ab || e.path.len() <= 1)?;
        }
    }

    // ---- Summary.
    let mut provenance: BTreeMap<&'static str, &'static str> = BTreeMap::new();
    provenance.insert(
        "movement_time",
        "standard (Tobler 1993 technical report; L&S Eq. 20 envelope at the closed-form ±1/3.5) — via assembly",
    );
    provenance.insert(
        "movement_energy",
        "standard (Herzog IA36 sextic, J/kg/m — the primary's kilo-joule label slip corrected per 0013; frozen L&S ±0.28/−0.22 min-form envelope) — standard links, via assembly; curve shape citable, magnitudes carry the elite-athlete caveat",
    );
    provenance.insert(
        "water_modes",
        "standard values (Livingood 2012 canoe ± current; Cox/AIDR caps via 0013), via assembly; coastal speed and transshipment are declared forcing (documented literature absences, 0014)",
    );
    provenance.insert(
        "corridor_density",
        "standard mechanism (White & Barber 2012 FETE), via cost model and multimodal graph — declared adaptations: 16-neighbour land moves (Herzog IA36), water participation, size-relative lattice default",
    );
    provenance.insert(
        "trunk",
        "standard links (Stahlberg 2023 reuse discount; Gabriel prune per Groenhuijzen & Verhagen 2017; ascending-cost order per Molinero & Hernando 2020 under uniform masses), via assembly — no published pipeline combines them (0014)",
    );
    provenance.insert(
        "hours_to_sea_land",
        "heuristic (via-defined tidewater source set) over the declared movement model",
    );
    provenance.insert(
        "hours_to_sea_water",
        "heuristic (via-defined tidewater source set) over the declared movement model",
    );
    provenance.insert(
        "hours_to_trunk",
        "heuristic (via-defined trunk source set) over the declared movement model",
    );
    let mut checks: BTreeMap<&'static str, bool> = BTreeMap::new();
    for name in [
        "grid_agreement",
        "density_definition",
        "trunk_definition",
        "hours_to_sea_definition",
        "hours_to_trunk_definition",
        "density_bounds",
        "density_source_coverage",
        "trunk_node_set",
        "trunk_edges",
        "trunk_junctions",
        "trunk_path_admissible",
        "trunk_path_cost",
    ] {
        checks.insert(name, true);
    }
    let summary = Summary {
        stage: "corridors",
        config: cfg,
        provenance,
        metric_values_conditional_on_k_q: true,
        degeneracy: &out.degeneracy,
        lattice: LatticeInfo {
            spacing_cells: out.lattice_spacing,
            sources: out.sources.len() as u32,
            directed_pairs: pairs,
        },
        trunk: TrunkSummary {
            nodes: &out.trunk.nodes,
            edges: &out.trunk.edges,
            junctions: &out.trunk.junctions,
        },
        checks,
        artifact_blake3: &hashes,
    };
    std::fs::write(
        run_dir.join(summary_filename(&cfg.label)),
        serde_json::to_string_pretty(&summary)? + "\n",
    )?;

    // ---- Manifest registration (rasters only, the house rule).
    let manifest_path = run_dir.join("manifest.json");
    let mut manifest = RunManifest::load(&manifest_path)?;
    let mut artifacts: BTreeMap<String, ArtifactEntry> = BTreeMap::new();
    for name in RASTER_NAMES {
        artifacts.insert(
            name.to_string(),
            ArtifactEntry {
                file: raster_filename(&cfg.label, name),
                blake3: hashes[name].clone(),
            },
        );
    }
    let mut crate_versions = BTreeMap::new();
    crate_versions.insert(
        "via-corridors".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    manifest.stages.insert(
        format!("corridors.{}", cfg.label),
        StageRecord {
            config: serde_json::to_value(cfg)?,
            crate_versions,
            artifacts,
        },
    );
    manifest.save(&manifest_path)
}
