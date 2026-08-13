//! Stage orchestration: the evolution loop, canonical extraction on the
//! final surface, and artifact emission.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::path::Path;

use rayon::prelude::*;

use via_artifact::manifest::{ArtifactEntry, RunManifest};
use via_artifact::raster::Raster;

use crate::climate;
use crate::config::TerrainConfig;
use crate::erosion;
use crate::extract;
use crate::fields;
use crate::flow::{self, DonorGraph};
use crate::gates::{self, GateSamples, GatesReport};
use crate::grid::Grid;
use crate::sediment;

pub struct TerrainOutput {
    pub cfg: TerrainConfig,
    pub grid: Grid,
    /// The true surface (bedrock + sediment), depressions intact.
    pub heights_m: Vec<f64>,
    pub uplift_m_per_yr: Vec<f64>,
    pub receivers: Vec<u32>,
    pub area_cells: Vec<u64>,
    /// Precipitation-weighted upslope accumulation, in equivalent cells.
    pub discharge_cells: Vec<f64>,
    pub precip_m_per_yr: Vec<f64>,
    pub temperature_c: Vec<f64>,
    /// Standing-water depth (m): routed surface − true surface. A spectrum;
    /// consumers threshold it (ADR 0004).
    pub water_depth_m: Vec<f64>,
    /// Sediment thickness (m); bedrock = heights − sediment.
    pub sediment_m: Vec<f64>,
    pub wind: (i32, i32),
    pub strahler: Vec<u32>,
    pub basin: Vec<u32>,
    pub land: Vec<bool>,
    pub gates: GatesReport,
    pub samples: GateSamples,
    /// (step, max |Δh|, mean |Δh|) in metres — the convergence trace.
    pub convergence_m: Vec<(u32, f64, f64)>,
    /// Worst-case Gauss–Seidel iteration count across all steps
    /// (ADR 0006) — observability for the coupled solve.
    pub gs_iterations_max: u32,
}

/// The border ring is the open boundary: heights are pinned to base depth
/// and any sediment parked there has left the system.
fn enforce_border(grid: &Grid, h: &mut [f64], sediment_m: &mut [f64], depth: f64) {
    let (w, hh) = (grid.w, grid.h);
    let mut pin = |i: u32| {
        h[i as usize] = depth;
        sediment_m[i as usize] = 0.0;
    };
    for x in 0..w {
        pin(grid.idx(x, 0));
        pin(grid.idx(x, hh - 1));
    }
    for y in 0..hh {
        pin(grid.idx(0, y));
        pin(grid.idx(w - 1, y));
    }
}

/// Base level = ocean = border-connected water only. Enclosed sub-sea
/// pockets are NOT base level: they are depressions that flood to their
/// spill on the routed surface — real lakes since M3 (ADR 0004).
fn ocean_mask(grid: &Grid, h: &[f64], sea: f64) -> Vec<bool> {
    let n = grid.n();
    let mut ocean = vec![false; n];
    let mut queue = VecDeque::new();
    for i in 0..n as u32 {
        if grid.is_border(i) && h[i as usize] < sea {
            ocean[i as usize] = true;
            queue.push_back(i);
        }
    }
    while let Some(c) = queue.pop_front() {
        grid.for_neighbors(c, |nb, _| {
            let nbu = nb as usize;
            if !ocean[nbu] && h[nbu] < sea {
                ocean[nbu] = true;
                queue.push_back(nb);
            }
        });
    }
    ocean
}

/// Run the terrain stage. `progress(step, max_dh_m)` is called once per step.
///
/// Panics on an invalid config; callers with a user in front of them should
/// call [`TerrainConfig::validate`] first and report the message.
pub fn run(cfg: &TerrainConfig, progress: &mut dyn FnMut(u32, f64)) -> TerrainOutput {
    if let Err(msg) = cfg.validate() {
        panic!("{msg}");
    }
    let grid = Grid::new(cfg.size, cfg.cell_size_m);
    let uplift = fields::build_uplift(cfg, &grid);
    let wind = climate::prevailing_wind(cfg);
    let mut h = fields::initial_heights(cfg, &grid);
    let mut sediment_m = vec![0.0f64; grid.n()];
    let mut h_prev = vec![0.0f64; grid.n()];
    let mut convergence = Vec::with_capacity(cfg.steps as usize);
    // Whole-run sediment budget; the mass-closure gate audits these. The
    // shallow-pond merge is a declared mass source and is metered alongside.
    let (mut run_det, mut run_dep, mut run_exp) = (0.0f64, 0.0f64, 0.0f64);
    let mut run_pond_merge_m3 = 0.0f64;
    let mut gs_iter_max = 0u32;
    // Deposition acts on the fluvial domain only (ADR 0004): same
    // threshold the slope–area gate uses for channel membership.
    let fluvial_min_cells = (cfg.fluvial_min_area_km2 * 1.0e6) / cfg.cell_area_m2();

    let weights_of = |precip: &[f64]| -> Vec<f64> {
        // Relative to the configured land mean; floored so full rain shadow
        // still routes a trickle rather than a zero-discharge channel.
        precip
            .iter()
            .map(|&p| (p / cfg.precip_mean_m_per_yr).max(0.05))
            .collect()
    };

    for step in 0..cfg.steps {
        h_prev.copy_from_slice(&h);
        erosion::apply_uplift(&mut h, &uplift, cfg.dt_years, cfg.base_depth_m);
        enforce_border(&grid, &mut h, &mut sediment_m, cfg.base_depth_m);
        let is_base = ocean_mask(&grid, &h, cfg.sea_level_m);
        // Flow is routed on the flooded copy; the true surface keeps its
        // deep depressions, which stand as lakes. Shallow ponding is
        // sub-grid noise and merges back into the terrain (ADR 0004).
        let mut h_route = h.clone();
        flow::priority_flood_eps(&grid, &mut h_route, &is_base, cfg.epsilon_fill_m);
        run_pond_merge_m3 +=
            flow::merge_shallow_depressions(&grid, &mut h, &h_route, sediment::MIN_LAKE_DEPTH_M)
                * cfg.cell_area_m2();
        // The step's standing-water state, frozen pre-erosion. The mask
        // must not be re-derived from the evolving surface (see
        // erosion::erode_stream_power).
        let flooded: Vec<bool> = h_route
            .iter()
            .zip(h.iter())
            .map(|(r, t)| r - t > sediment::FLOOD_EPS_M)
            .collect();
        // Climate is recomputed against the evolving topography (the water
        // surface — air rides over lakes, not lake beds): the internal
        // terrain↔precipitation loop.
        let precip = climate::compute_precipitation(cfg, &grid, &h_route, wind);
        let weights = weights_of(&precip);
        // Two-pass routing: the pure-MFD pass finds where water
        // concentrates, the hybrid pass converges those channels while
        // hillslopes and standing water keep spreading (ADR 0005).
        let mfd0 = flow::MfdGraph::build(&grid, &h_route, &is_base, cfg.mfd_exponent);
        let q0 = flow::accumulate_discharge_mfd(&mfd0, &weights);
        let mfd = flow::MfdGraph::build_hybrid(
            &grid,
            &h_route,
            &is_base,
            cfg.mfd_exponent,
            &q0,
            fluvial_min_cells,
            &flooded,
        );
        let discharge = flow::accumulate_discharge_mfd(&mfd, &weights);
        let solved = sediment::solve_implicit(
            &grid,
            &mut h,
            &mfd,
            &discharge,
            &is_base,
            &flooded,
            &h_route,
            cfg.k_spl,
            cfg.g_deposition,
            cfg.dt_years,
            fluvial_min_cells,
            cfg.sea_level_m,
        );
        // Detachment takes sediment cover first, then bedrock; deposition
        // adds to the cover.
        sediment_m
            .par_iter_mut()
            .zip(solved.detached_m.par_iter())
            .zip(solved.deposited_m.par_iter())
            .for_each(|((s, &d), &p)| *s = (*s - d).max(0.0) + p);
        run_det += solved.detached_m3;
        run_dep += solved.deposited_m3;
        run_exp += solved.exported_m3;
        gs_iter_max = gs_iter_max.max(solved.iterations);
        // Hillslope diffusion moves colluvium: falling cells shed sediment
        // first, rising cells gain it.
        let h_before_diff = h.clone();
        erosion::diffuse(&grid, &mut h, &is_base, cfg.kappa, cfg.dt_years);
        sediment_m
            .par_iter_mut()
            .zip(h.par_iter())
            .zip(h_before_diff.par_iter())
            .for_each(|((s, &after), &before)| *s = (*s + (after - before)).max(0.0));
        let max_dh = erosion::max_abs_diff(&h, &h_prev);
        let mean_dh = erosion::mean_abs_diff(&h, &h_prev);
        convergence.push((step, max_dh, mean_dh));
        progress(step, max_dh);
    }

    // Canonical routing on the final surface — this is what the artifacts
    // describe, so it is recomputed once, after the last diffusion pass.
    enforce_border(&grid, &mut h, &mut sediment_m, cfg.base_depth_m);
    let is_base = ocean_mask(&grid, &h, cfg.sea_level_m);
    let mut h_route = h.clone();
    flow::priority_flood_eps(&grid, &mut h_route, &is_base, cfg.epsilon_fill_m);
    run_pond_merge_m3 +=
        flow::merge_shallow_depressions(&grid, &mut h, &h_route, sediment::MIN_LAKE_DEPTH_M)
            * cfg.cell_area_m2();
    let water_depth_m: Vec<f64> = h_route.iter().zip(h.iter()).map(|(r, t)| r - t).collect();
    let precip = climate::compute_precipitation(cfg, &grid, &h_route, wind);
    let temperature = climate::compute_temperature(cfg, &grid, &h_route);
    let weights = weights_of(&precip);
    let flooded_final: Vec<bool> = water_depth_m
        .iter()
        .map(|&wd| wd > sediment::FLOOD_EPS_M)
        .collect();
    let mfd0 = flow::MfdGraph::build(&grid, &h_route, &is_base, cfg.mfd_exponent);
    let q0 = flow::accumulate_discharge_mfd(&mfd0, &weights);
    let mfd = flow::MfdGraph::build_hybrid(
        &grid,
        &h_route,
        &is_base,
        cfg.mfd_exponent,
        &q0,
        fluvial_min_cells,
        &flooded_final,
    );
    let discharge_cells = flow::accumulate_discharge_mfd(&mfd, &weights);
    // The channel tree (max-weight receiver): statistics, extraction, and
    // the `receivers` artifact contract (self-receiver = ocean) all live
    // on the tree; the MFD field carries the water (ADR 0005).
    let receivers = mfd.tree_receivers.clone();
    let donors = DonorGraph::build(&receivers);
    let stack = flow::build_stack(&receivers, &donors);
    let area_cells = flow::accumulate_area(&receivers, &stack);
    let land: Vec<bool> = is_base.iter().map(|&b| !b).collect();
    // River extraction thresholds against the TREE-accumulated flux, not
    // the MFD field: MFD flux is not monotone along the tree (spreading
    // cells pass only their max-weight share down it), so thresholding it
    // breaks Strahler streams mid-channel and re-births them downstream
    // as phantom order-1 heads (measured: 27 breaks at island8k, 6 at
    // research, contaminating the Horton population). Tree flux is
    // monotone by construction, restoring the downstream-closure
    // invariant the extraction and gates machinery assumes (ADR 0005).
    let tree_discharge = flow::accumulate_discharge(&receivers, &stack, &weights);
    let strahler = extract::strahler_orders(
        &receivers,
        &stack,
        &donors,
        &land,
        &tree_discharge,
        cfg.river_min_cells() as f64,
    );
    let basin = extract::label_basins(&receivers, &stack);
    let dist_m = extract::flow_distance_m(&grid, &receivers, &stack);
    let mainstream_m = extract::mainstream_length_m(&receivers, &stack, &dist_m);

    // Diagnostic replay of one step's coupled solve on a scratch copy:
    // the per-cell deposition rate the residual gate needs. The real
    // state is not touched.
    let deposition_rate_m_per_yr: Vec<f64> = {
        let mut h_diag = h.clone();
        let solved = sediment::solve_implicit(
            &grid,
            &mut h_diag,
            &mfd,
            &discharge_cells,
            &is_base,
            &flooded_final,
            &h_route,
            cfg.k_spl,
            cfg.g_deposition,
            cfg.dt_years,
            fluvial_min_cells,
            cfg.sea_level_m,
        );
        solved
            .deposited_m
            .iter()
            .map(|&d| d / cfg.dt_years)
            .collect()
    };

    let (gates_report, samples) = gates::compute(&gates::GateInputs {
        grid: &grid,
        cfg,
        h: &h,
        uplift: &uplift,
        receivers: &receivers,
        mfd: &mfd,
        stack: &stack,
        donors: &donors,
        area_cells: &area_cells,
        discharge_cells: &discharge_cells,
        tree_discharge_cells: &tree_discharge,
        precip: &precip,
        wind,
        strahler: &strahler,
        basin: &basin,
        dist_m: &dist_m,
        mainstream_m: &mainstream_m,
        land: &land,
        water_depth: &water_depth_m,
        sediment_m: &sediment_m,
        deposition_rate_m_per_yr: &deposition_rate_m_per_yr,
        run_detached_m3: run_det,
        run_deposited_m3: run_dep,
        run_exported_m3: run_exp,
        run_pond_merge_m3,
    });

    let mut out = TerrainOutput {
        cfg: cfg.clone(),
        grid,
        heights_m: h,
        uplift_m_per_yr: uplift,
        receivers,
        area_cells,
        discharge_cells,
        precip_m_per_yr: precip,
        temperature_c: temperature,
        water_depth_m,
        sediment_m,
        wind: (wind.dx, wind.dy),
        strahler,
        basin,
        land,
        gates: gates_report,
        samples,
        convergence_m: convergence,
        gs_iterations_max: gs_iter_max,
    };
    out.gates.artifact_blake3 = artifact_hashes(&out);
    out
}

fn cell_size_cm(cfg: &TerrainConfig) -> u32 {
    (cfg.cell_size_m * 100.0).round() as u32
}

pub fn raster_heights_cm(o: &TerrainOutput) -> Raster<i32> {
    let data = o
        .heights_m
        .iter()
        .map(|&v| (v * 100.0).round() as i32)
        .collect();
    Raster::from_data(o.grid.w, o.grid.h, cell_size_cm(&o.cfg), data)
}

pub fn raster_receivers(o: &TerrainOutput) -> Raster<u32> {
    Raster::from_data(
        o.grid.w,
        o.grid.h,
        cell_size_cm(&o.cfg),
        o.receivers.clone(),
    )
}

pub fn raster_area_cells(o: &TerrainOutput) -> Raster<u64> {
    Raster::from_data(
        o.grid.w,
        o.grid.h,
        cell_size_cm(&o.cfg),
        o.area_cells.clone(),
    )
}

pub fn raster_strahler(o: &TerrainOutput) -> Raster<u32> {
    Raster::from_data(o.grid.w, o.grid.h, cell_size_cm(&o.cfg), o.strahler.clone())
}

pub fn raster_basin(o: &TerrainOutput) -> Raster<u32> {
    Raster::from_data(o.grid.w, o.grid.h, cell_size_cm(&o.cfg), o.basin.clone())
}

pub fn raster_uplift(o: &TerrainOutput) -> Raster<f32> {
    let data = o.uplift_m_per_yr.iter().map(|&v| v as f32).collect();
    Raster::from_data(o.grid.w, o.grid.h, cell_size_cm(&o.cfg), data)
}

fn f32_raster(o: &TerrainOutput, src: &[f64]) -> Raster<f32> {
    let data = src.iter().map(|&v| v as f32).collect();
    Raster::from_data(o.grid.w, o.grid.h, cell_size_cm(&o.cfg), data)
}

pub fn raster_precip(o: &TerrainOutput) -> Raster<f32> {
    f32_raster(o, &o.precip_m_per_yr)
}

pub fn raster_temperature(o: &TerrainOutput) -> Raster<f32> {
    f32_raster(o, &o.temperature_c)
}

pub fn raster_discharge(o: &TerrainOutput) -> Raster<f32> {
    f32_raster(o, &o.discharge_cells)
}

pub fn raster_water_depth(o: &TerrainOutput) -> Raster<f32> {
    f32_raster(o, &o.water_depth_m)
}

pub fn raster_sediment(o: &TerrainOutput) -> Raster<f32> {
    f32_raster(o, &o.sediment_m)
}

/// blake3 of every artifact raster — the determinism witness recorded in
/// both gates.json and the manifest.
pub fn artifact_hashes(o: &TerrainOutput) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("heights_cm".into(), raster_heights_cm(o).blake3_hex());
    m.insert("receivers".into(), raster_receivers(o).blake3_hex());
    m.insert("area_cells".into(), raster_area_cells(o).blake3_hex());
    m.insert("strahler".into(), raster_strahler(o).blake3_hex());
    m.insert("basin".into(), raster_basin(o).blake3_hex());
    m.insert("uplift".into(), raster_uplift(o).blake3_hex());
    m.insert("precip".into(), raster_precip(o).blake3_hex());
    m.insert("temperature".into(), raster_temperature(o).blake3_hex());
    m.insert("discharge".into(), raster_discharge(o).blake3_hex());
    m.insert("water_depth".into(), raster_water_depth(o).blake3_hex());
    m.insert("sediment".into(), raster_sediment(o).blake3_hex());
    m
}

/// Write the run directory: rasters, gates.json, qa_samples.json, manifest.
pub fn write_run(dir: &Path, o: &TerrainOutput) -> io::Result<RunManifest> {
    std::fs::create_dir_all(dir)?;
    let mut artifacts = BTreeMap::new();
    {
        let mut put = |name: &str, file: &str, bytes_hash: String| {
            artifacts.insert(
                name.to_string(),
                ArtifactEntry {
                    file: file.to_string(),
                    blake3: bytes_hash,
                },
            );
        };
        let r = raster_heights_cm(o);
        r.write_file(&dir.join("heights_cm.vrast"))?;
        put("heights_cm", "heights_cm.vrast", r.blake3_hex());
        let r = raster_receivers(o);
        r.write_file(&dir.join("receivers.vrast"))?;
        put("receivers", "receivers.vrast", r.blake3_hex());
        let r = raster_area_cells(o);
        r.write_file(&dir.join("area_cells.vrast"))?;
        put("area_cells", "area_cells.vrast", r.blake3_hex());
        let r = raster_strahler(o);
        r.write_file(&dir.join("strahler.vrast"))?;
        put("strahler", "strahler.vrast", r.blake3_hex());
        let r = raster_basin(o);
        r.write_file(&dir.join("basin.vrast"))?;
        put("basin", "basin.vrast", r.blake3_hex());
        let r = raster_uplift(o);
        r.write_file(&dir.join("uplift.vrast"))?;
        put("uplift", "uplift.vrast", r.blake3_hex());
        let r = raster_precip(o);
        r.write_file(&dir.join("precip.vrast"))?;
        put("precip", "precip.vrast", r.blake3_hex());
        let r = raster_temperature(o);
        r.write_file(&dir.join("temperature.vrast"))?;
        put("temperature", "temperature.vrast", r.blake3_hex());
        let r = raster_discharge(o);
        r.write_file(&dir.join("discharge.vrast"))?;
        put("discharge", "discharge.vrast", r.blake3_hex());
        let r = raster_water_depth(o);
        r.write_file(&dir.join("water_depth.vrast"))?;
        put("water_depth", "water_depth.vrast", r.blake3_hex());
        let r = raster_sediment(o);
        r.write_file(&dir.join("sediment.vrast"))?;
        put("sediment", "sediment.vrast", r.blake3_hex());
    }

    let gates_json = serde_json::to_string_pretty(&o.gates)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(dir.join("gates.json"), gates_json + "\n")?;
    let samples_json = serde_json::to_string_pretty(&o.samples)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(dir.join("qa_samples.json"), samples_json + "\n")?;

    let mut crate_versions = BTreeMap::new();
    crate_versions.insert(
        "via-terrain".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    let manifest = RunManifest {
        stage: "terrain".to_string(),
        seed: o.cfg.seed,
        config: serde_json::to_value(&o.cfg)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        crate_versions,
        artifacts,
    };
    manifest.save(&dir.join("manifest.json"))?;
    Ok(manifest)
}
