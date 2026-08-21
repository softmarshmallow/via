//! Suitability stage, first slice: generic affordance fields (slope,
//! freshwater distance, coast distance, elevation) and contiguous patches
//! passing config thresholds. The stage knows nothing about what a patch is
//! *for* — the config's `label` names the selection criterion (ADR 0011:
//! curation, not a gate), and the semantics live in the experiment that
//! supplies the config. Output filenames are namespaced by the label so
//! multiple configs can share a run directory.
//!
//! Reads terrain artifacts from a run directory (stages communicate through
//! artifacts, never by calling each other) and writes its own artifacts and
//! stats beside them. Everything here is sequential and deterministic.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use via_artifact::manifest::{ArtifactEntry, RunManifest, StageRecord};
use via_artifact::raster::Raster;

mod confluence;
mod fords;
mod navigability;
mod passes;

pub use confluence::ConfluenceSite;
pub use fords::FordFields;
pub use navigability::{HeadOfNavSite, NavigabilityFields};
pub use passes::SaddleSite;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SuitabilityConfig {
    /// Name of the selection criterion this config defines (e.g.
    /// "village_site"). Pure labeling; the stage attaches no meaning to it.
    /// Output filenames are namespaced by it, so it must match `[a-z0-9_-]+`.
    pub label: String,
    /// Maximum steepest-descent slope (rise/run).
    pub max_slope: f64,
    pub min_elevation_m: f64,
    pub max_elevation_m: f64,
    /// Maximum along-ground distance to fresh water (m) — river cells or
    /// standing water at least `freshwater_min_depth_m` deep.
    pub max_freshwater_dist_m: f64,
    /// Standing-water depth (m) at or above which a cell counts as a
    /// freshwater source (a threshold over the terrain water-depth
    /// spectrum, declared here per ADR 0003).
    pub freshwater_min_depth_m: f64,
    /// Maximum standing-water depth (m) a cell may carry and still be a
    /// candidate — ponded ground is not buildable land.
    pub max_standing_water_m: f64,
    /// Minimum contiguous patch area (hectares).
    pub min_patch_area_ha: f64,
    pub max_patches_reported: u32,
    /// Minimum saddle persistence (m) for a pass site to be reported —
    /// the published pruning floor (Kirmse & de Ferranti 2017, ~30 m).
    pub min_pass_persistence_m: f64,
    /// k_Q: m³/s per relative discharge unit — the single
    /// relative-to-absolute scale (ADR 0011 D1, Tier-2 forcing; the
    /// lumped-calibration practice of Whipple & Tucker 1999). Pure
    /// declared forcing, unverifiable from inside via; the default is a
    /// placeholder scale and experiments must declare their own.
    pub k_q_m3s_per_unit: f64,
    /// Manning roughness n (Chow 1959, natural streams ~0.030–0.050).
    /// One declared value; a per-lithology lookup would be a named
    /// heuristic (0013 §Fords) and is not implemented.
    pub manning_n: f64,
    /// Finnegan et al. (2005) width-to-depth ratio α (the paper's ≈ 20).
    pub finnegan_alpha: f64,
    /// Reach length, in cells along the receivers path, for the
    /// reach-averaged channel slope (0013 §Fords: single-cell slopes are
    /// noisy at cm quantization). Declared parameter.
    pub slope_reach_cells: u32,
    /// Numerical channel-slope floor (declared): Finnegan carries
    /// S^(−3/16) and Manning √S, so a flat routed reach must not divide
    /// by zero.
    pub min_channel_slope: f64,
    /// Langbein (1962) navigability anchor: Ts above this is
    /// unnavigable (published 0.002; Mississippi ≈ 0.00015).
    pub nav_max_ts: f64,
    /// Magirl & Olsen (2009) slope band: probably not navigable above
    /// this (0.0047; 0.0019–0.0047 is their indeterminate band).
    pub nav_max_slope: f64,
    /// Pre-modern navigable-depth anchor (m): Eckoldt's 0.3–0.7 m band
    /// for keelless barges (via Appel et al. 2024); metric values are
    /// conditional on k_Q.
    pub nav_min_depth_m: f64,
}

impl Default for SuitabilityConfig {
    fn default() -> Self {
        Self {
            label: "site".to_string(),
            max_slope: 0.105,
            min_elevation_m: 2.0,
            max_elevation_m: 80.0,
            max_freshwater_dist_m: 400.0,
            freshwater_min_depth_m: 0.30,
            max_standing_water_m: 0.05,
            min_patch_area_ha: 12.0,
            max_patches_reported: 8,
            min_pass_persistence_m: 30.0,
            k_q_m3s_per_unit: 1.0,
            manning_n: 0.035,
            finnegan_alpha: 20.0,
            slope_reach_cells: 5,
            min_channel_slope: 1.0e-5,
            nav_max_ts: 0.002,
            nav_max_slope: 0.0047,
            nav_min_depth_m: 0.5,
        }
    }
}

impl SuitabilityConfig {
    pub fn from_json_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("cannot parse config {}: {e}", path.display()))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchInfo {
    pub rank: u32,
    pub cells: u64,
    pub area_ha: f64,
    /// Patch centroid in cell coordinates.
    pub centroid_x: f64,
    pub centroid_y: f64,
    pub mean_slope: f64,
    pub mean_elevation_m: f64,
    pub min_freshwater_dist_m: f64,
    pub min_coast_dist_m: f64,
}

pub struct SuitabilityOutput {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub sea_level_m: f64,
    pub slope: Vec<f64>,
    pub freshwater_dist_m: Vec<f64>,
    pub coast_dist_m: Vec<f64>,
    pub suitable: Vec<bool>,
    /// 0 = not in a reported patch; otherwise the patch rank (1 = largest).
    pub patch_rank: Vec<u32>,
    pub patches: Vec<PatchInfo>,
    /// Whether any patch met the config thresholds — the config's selection
    /// criterion (ADR 0011 D2: curation, not a gate).
    pub criterion_met: bool,
    /// Confluence sites in ascending cell order (ADR 0011 D3/D4).
    pub confluences: Vec<ConfluenceSite>,
    /// Pass (saddle) sites in ascending cell order (ADR 0011 D3/D4).
    pub passes: Vec<SaddleSite>,
    /// Ford hydraulic spectra (width, depth, velocity, crossability).
    pub ford: FordFields,
    /// Navigability spectrum and banded predicate.
    pub nav: NavigabilityFields,
    /// Head-of-navigation sites in ascending cell order (ADR 0011 D3/D6).
    pub heads_of_navigation: Vec<HeadOfNavSite>,
    /// The terrain channelization threshold, restated beside any reported
    /// confluence count (ADR 0011 D4); absent if the terrain config lacks it.
    pub river_min_area_km2: Option<f64>,
}

/// Output filenames are namespaced by the config label; restrict the label
/// so it can never escape the run directory or collide across configs.
fn validate_label(label: &str) -> io::Result<()> {
    let ok = !label.is_empty()
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("config label {label:?} must be non-empty and match [a-z0-9_-]+"),
        ))
    }
}

/// Namespaced raster filename: `suitability.<label>.<name>.vrast`.
pub fn raster_filename(label: &str, name: &str) -> String {
    format!("suitability.{label}.{name}.vrast")
}

/// Namespaced stage-summary filename: `suitability.<label>.json`.
pub fn summary_filename(label: &str) -> String {
    format!("suitability.{label}.json")
}

#[inline]
fn f64_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 {
        !b
    } else {
        b ^ (1u64 << 63)
    }
}

/// Visit in-bounds D8 neighbours of `i` in fixed scan order.
#[inline]
pub(crate) fn for_neighbors8(w: u32, h: u32, i: u32, mut f: impl FnMut(u32, f64)) {
    const D8: [(i32, i32, f64); 8] = [
        (-1, -1, std::f64::consts::SQRT_2),
        (0, -1, 1.0),
        (1, -1, std::f64::consts::SQRT_2),
        (-1, 0, 1.0),
        (1, 0, 1.0),
        (-1, 1, std::f64::consts::SQRT_2),
        (0, 1, 1.0),
        (1, 1, std::f64::consts::SQRT_2),
    ];
    let (x, y) = ((i % w) as i64, (i / w) as i64);
    for &(dx, dy, fac) in &D8 {
        let (nx, ny) = (x + dx as i64, y + dy as i64);
        if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
            f((ny * w as i64 + nx) as u32, fac);
        }
    }
}

/// Multi-source Dijkstra over the D8 grid metric. Heap ties break on cell
/// index; deterministic.
fn dijkstra_dist_m(w: u32, h: u32, dx: f64, is_source: &[bool]) -> Vec<f64> {
    let n = w as usize * h as usize;
    let mut dist = vec![f64::INFINITY; n];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    for i in 0..n {
        if is_source[i] {
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

/// Steepest-descent slope per cell (0 for ocean cells), measured over the
/// **effective surface** — terrain plus standing water. Since M3 the
/// heights artifact carries true lake bathymetry; a shore cell must see
/// its neighbour's water surface, not the drop through the water column,
/// and lake beds read the flat surface like the ocean does.
fn slope_field(w: u32, h: u32, dx: f64, eff_heights_m: &[f64], land: &[bool]) -> Vec<f64> {
    let n = w as usize * h as usize;
    let mut slope = vec![0.0f64; n];
    for i in 0..n as u32 {
        if !land[i as usize] {
            continue;
        }
        let hi = eff_heights_m[i as usize];
        let mut best = 0.0f64;
        for_neighbors8(w, h, i, |nb, fac| {
            let s = (hi - eff_heights_m[nb as usize]) / (fac * dx);
            if s > best {
                best = s;
            }
        });
        slope[i as usize] = best;
    }
    slope
}

/// 4-connected components over `suitable`, in scan order (deterministic).
/// Returns each patch's member cells; patches ≥ `min_cells` only.
fn extract_patches(w: u32, h: u32, suitable: &[bool], min_cells: u64) -> Vec<Vec<u32>> {
    let n = w as usize * h as usize;
    let mut seen = vec![false; n];
    let mut patches = Vec::new();
    for start in 0..n as u32 {
        if !suitable[start as usize] || seen[start as usize] {
            continue;
        }
        let mut cells = Vec::new();
        let mut queue = VecDeque::new();
        seen[start as usize] = true;
        queue.push_back(start);
        while let Some(c) = queue.pop_front() {
            cells.push(c);
            let (x, y) = ((c % w) as i64, (c / w) as i64);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
                    let nb = (ny * w as i64 + nx) as u32;
                    if suitable[nb as usize] && !seen[nb as usize] {
                        seen[nb as usize] = true;
                        queue.push_back(nb);
                    }
                }
            }
        }
        if cells.len() as u64 >= min_cells {
            patches.push(cells);
        }
    }
    // Largest first; scan order (first cell index) breaks ties.
    patches.sort_by_key(|p| (Reverse(p.len()), p[0]));
    patches
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
pub fn run(run_dir: &Path, cfg: &SuitabilityConfig) -> io::Result<SuitabilityOutput> {
    validate_label(&cfg.label)?;
    let heights = Raster::<i32>::read_file(&run_dir.join("heights_cm.vrast"))?;
    let receivers = Raster::<u32>::read_file(&run_dir.join("receivers.vrast"))?;
    let strahler = Raster::<u32>::read_file(&run_dir.join("strahler.vrast"))?;
    let water_depth = Raster::<f32>::read_file(&run_dir.join("water_depth.vrast"))?;
    let area_cells = Raster::<u64>::read_file(&run_dir.join("area_cells.vrast"))?;
    let discharge = Raster::<f32>::read_file(&run_dir.join("discharge.vrast"))?;
    ensure_grid("receivers", &receivers, &heights)?;
    ensure_grid("strahler", &strahler, &heights)?;
    ensure_grid("water_depth", &water_depth, &heights)?;
    ensure_grid("area_cells", &area_cells, &heights)?;
    ensure_grid("discharge", &discharge, &heights)?;
    let manifest = RunManifest::load(&run_dir.join("manifest.json"))?;
    let sea = manifest
        .stage_config("terrain")
        .and_then(|c| c.get("sea_level_m"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let river_min_area_km2 = manifest
        .stage_config("terrain")
        .and_then(|c| c.get("river_min_area_km2"))
        .and_then(|v| v.as_f64());
    let (w, h) = (heights.width, heights.height);
    let dx = heights.cell_size_cm as f64 / 100.0;
    let n = w as usize * h as usize;

    let heights_m: Vec<f64> = heights.data.iter().map(|&cm| cm as f64 / 100.0).collect();
    // The stage's authoritative base-level mask: self-receiver = ocean (the
    // pit gate guarantees no land self-receivers).
    let land: Vec<bool> = receivers
        .data
        .iter()
        .enumerate()
        .map(|(i, &r)| r as usize != i)
        .collect();
    // Effective surface: terrain + standing water (slopes must not measure
    // through lake water columns).
    let eff_heights_m: Vec<f64> = heights_m
        .iter()
        .zip(water_depth.data.iter())
        .map(|(&hm, &wd)| hm + wd as f64)
        .collect();
    let slope = slope_field(w, h, dx, &eff_heights_m, &land);
    // Fresh water = rivers or standing water deep enough (lakes); ocean is
    // the coast field, not a freshwater source.
    let is_freshwater: Vec<bool> = strahler
        .data
        .iter()
        .zip(water_depth.data.iter())
        .enumerate()
        .map(|(i, (&o, &wd))| land[i] && (o > 0 || wd as f64 >= cfg.freshwater_min_depth_m))
        .collect();
    let is_ocean: Vec<bool> = land.iter().map(|&l| !l).collect();
    let freshwater_dist_m = dijkstra_dist_m(w, h, dx, &is_freshwater);
    let coast_dist_m = dijkstra_dist_m(w, h, dx, &is_ocean);

    let suitable: Vec<bool> = (0..n)
        .map(|i| {
            land[i]
                && (water_depth.data[i] as f64) <= cfg.max_standing_water_m
                && slope[i] <= cfg.max_slope
                && heights_m[i] >= sea + cfg.min_elevation_m
                && heights_m[i] <= sea + cfg.max_elevation_m
                && freshwater_dist_m[i] <= cfg.max_freshwater_dist_m
        })
        .collect();

    let cell_area_ha = dx * dx / 10_000.0;
    let min_cells = (cfg.min_patch_area_ha / cell_area_ha).ceil() as u64;
    let mut patch_lists = extract_patches(w, h, &suitable, min_cells);
    patch_lists.truncate(cfg.max_patches_reported as usize);

    let mut patch_rank = vec![0u32; n];
    let mut patches = Vec::new();
    for (k, cells) in patch_lists.iter().enumerate() {
        let rank = k as u32 + 1;
        let (mut sx, mut sy, mut ssl, mut sel) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut min_fw, mut min_co) = (f64::INFINITY, f64::INFINITY);
        for &c in cells {
            patch_rank[c as usize] = rank;
            sx += (c % w) as f64;
            sy += (c / w) as f64;
            ssl += slope[c as usize];
            sel += heights_m[c as usize];
            min_fw = min_fw.min(freshwater_dist_m[c as usize]);
            min_co = min_co.min(coast_dist_m[c as usize]);
        }
        let m = cells.len() as f64;
        patches.push(PatchInfo {
            rank,
            cells: cells.len() as u64,
            area_ha: m * cell_area_ha,
            centroid_x: sx / m,
            centroid_y: sy / m,
            mean_slope: ssl / m,
            mean_elevation_m: sel / m,
            min_freshwater_dist_m: min_fw,
            min_coast_dist_m: min_co,
        });
    }

    let criterion_met = !patches.is_empty();
    let confluences = confluence::detect(w, h, &receivers.data, &strahler.data, &area_cells.data);
    let saddle_sites = passes::detect(w, h, &heights.data, &land, cfg.min_pass_persistence_m);
    // Water surface for channel slopes: effective heights on land, sea
    // level on ocean — a mouth reach drops to the sea, not through
    // bathymetry.
    let surface_m: Vec<f64> = eff_heights_m
        .iter()
        .zip(land.iter())
        .map(|(&e, &l)| if l { e } else { sea })
        .collect();
    let ford = fords::compute(
        w,
        h,
        dx,
        &surface_m,
        &receivers.data,
        &land,
        &strahler.data,
        &water_depth.data,
        &discharge.data,
        &fords::FordParams {
            k_q_m3s_per_unit: cfg.k_q_m3s_per_unit,
            manning_n: cfg.manning_n,
            finnegan_alpha: cfg.finnegan_alpha,
            slope_reach_cells: cfg.slope_reach_cells,
            min_channel_slope: cfg.min_channel_slope,
        },
    );
    let nav = navigability::compute(
        &land,
        &strahler.data,
        &water_depth.data,
        &ford,
        &navigability::NavParams {
            max_ts: cfg.nav_max_ts,
            max_slope: cfg.nav_max_slope,
            min_depth_m: cfg.nav_min_depth_m,
            manning_n: cfg.manning_n,
        },
    );
    let heads_of_navigation = navigability::head_of_navigation(
        w,
        h,
        &receivers.data,
        &land,
        &water_depth.data,
        &nav,
        &ford,
    );
    Ok(SuitabilityOutput {
        w,
        h,
        dx,
        sea_level_m: sea,
        slope,
        freshwater_dist_m,
        coast_dist_m,
        suitable,
        patch_rank,
        patches,
        criterion_met,
        confluences,
        passes: saddle_sites,
        ford,
        nav,
        heads_of_navigation,
        river_min_area_km2,
    })
}

/// Write the stage's artifacts and stats record into the run directory.
pub fn write_outputs(
    run_dir: &Path,
    cfg: &SuitabilityConfig,
    out: &SuitabilityOutput,
) -> io::Result<()> {
    validate_label(&cfg.label)?;
    let cell_cm = (out.dx * 100.0).round() as u32;
    let mut hashes = BTreeMap::new();
    let path = |name: &str| run_dir.join(raster_filename(&cfg.label, name));
    // Distance fields: unreachable cells carry the f32::MAX sentinel.
    let dist_f32 = |src: &[f64]| -> Vec<f32> {
        src.iter()
            .map(|&v| if v.is_finite() { v as f32 } else { f32::MAX })
            .collect()
    };

    let slope_f32: Vec<f32> = out.slope.iter().map(|&v| v as f32).collect();
    let r = Raster::from_data(out.w, out.h, cell_cm, slope_f32);
    r.write_file(&path("slope"))?;
    hashes.insert("slope".to_string(), r.blake3_hex());

    let r = Raster::from_data(out.w, out.h, cell_cm, dist_f32(&out.freshwater_dist_m));
    r.write_file(&path("freshwater_dist"))?;
    hashes.insert("freshwater_dist".to_string(), r.blake3_hex());

    let r = Raster::from_data(out.w, out.h, cell_cm, dist_f32(&out.coast_dist_m));
    r.write_file(&path("coast_dist"))?;
    hashes.insert("coast_dist".to_string(), r.blake3_hex());

    let r = Raster::from_data(out.w, out.h, cell_cm, out.patch_rank.clone());
    r.write_file(&path("patch_rank"))?;
    hashes.insert("patch_rank".to_string(), r.blake3_hex());

    for (name, data) in [
        ("ford_width", &out.ford.width_m),
        ("ford_depth", &out.ford.depth_m),
        ("ford_velocity", &out.ford.velocity_ms),
        ("crossability", &out.ford.crossability),
        ("navigability_ts", &out.nav.ts),
    ] {
        let r = Raster::from_data(out.w, out.h, cell_cm, data.clone());
        r.write_file(&path(name))?;
        hashes.insert(name.to_string(), r.blake3_hex());
    }
    let r = Raster::from_data(out.w, out.h, cell_cm, out.nav.navigable.clone());
    r.write_file(&path("navigable"))?;
    hashes.insert("navigable".to_string(), r.blake3_hex());

    // ADR 0011 D6 gates: definitional invariants recomputed from the
    // artifacts on disk, by a different traversal than the detector's.
    // Binary; unevaluable or failed is a hard error (ADR 0001 QA contract).
    let receivers = Raster::<u32>::read_file(&run_dir.join("receivers.vrast"))?;
    let strahler = Raster::<u32>::read_file(&run_dir.join("strahler.vrast"))?;
    let water_depth = Raster::<f32>::read_file(&run_dir.join("water_depth.vrast"))?;
    let land_mask: Vec<bool> = receivers
        .data
        .iter()
        .enumerate()
        .map(|(i, &r)| r as usize != i)
        .collect();
    let confluence_ok = confluence::verify_definition(
        out.w,
        out.h,
        &receivers.data,
        &strahler.data,
        &out.confluences,
    );
    if !confluence_ok {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "confluence-definition gate failed: emitted sites do not match \
             the receivers/strahler artifacts",
        ));
    }
    let spectrum_ok =
        fords::verify_identities(&land_mask, &strahler.data, &water_depth.data, &out.ford);
    if !spectrum_ok {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "spectrum-identity gate failed: crossability does not equal the \
             f32 product of its emitted factors (or the still-water depth) \
             on some cell",
        ));
    }
    // Head-of-navigation gate: the predicate is read back from the file
    // just written — the emitted artifact, not the in-memory copy.
    let navigable_disk = Raster::<u32>::read_file(&path("navigable"))?;
    let heads_ok = navigability::verify_heads(
        &receivers.data,
        &land_mask,
        &navigable_disk.data,
        &out.heads_of_navigation,
    );
    if !heads_ok {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "head-of-navigation gate failed: emitted sites are not exactly \
             the mouth-connected navigable cells without a donor in the set",
        ));
    }

    let report = serde_json::json!({
        "stage": "suitability",
        "config": cfg,
        // ADR 0003 (via ADR 0011 D1): every output declares standard vs
        // heuristic; keys match `artifact_blake3`.
        "provenance": {
            "slope": "standard: steepest-descent slope, D8 convention (O'Callaghan & Mark 1984), on the effective surface (terrain + standing water)",
            "freshwater_dist": "standard: shortest along-ground distance in the D8 grid metric (multi-source Dijkstra) to the declared freshwater set — river cells (strahler > 0) or standing water >= freshwater_min_depth_m",
            "coast_dist": "standard: shortest along-ground distance in the D8 grid metric (multi-source Dijkstra) to ocean cells (self-receivers)",
            "patch_rank": "heuristic (ADR 0003): 4-connected components of the config-thresholded suitability predicate, ranked by area",
            "confluences": "standard: junction = river cell with >= 2 river donors on the receivers tree (Strahler 1957 frame); symmetry ratio = smaller/larger donor drainage from area_cells (Benda et al. 2004); two largest donors when a D8 cell has more than two is a declared adaptation",
            "passes": "standard: Morse saddles by superlevel-set union-find sweep over height-sorted cells, persistence = paired peak - saddle (Edelsbrunner et al. 2002; Kirmse & de Ferranti 2017 instantiation and ~30 m pruning floor); Peucker-Douglas (1975) 8-ring recorded per site as diagnostic (Takahashi et al. 1995 predict grid inconsistency); declared adaptations: land-cell domain, out-of-grid neighbours count lower, equal-height ties by ascending cell index, multi-merge cells record the largest dying persistence",
            "ford_width": "standard: Finnegan et al. (2005) W = [alpha(alpha+2)^(2/3)]^(3/8) (nQ)^(3/8) S^(-3/16); Q = k_Q x relative discharge (declared forcing, Whipple & Tucker 1999 lumped-calibration practice); reach-averaged water-surface slope over a declared reach; n from Chow (1959)",
            "ford_depth": "standard: Manning (1891) wide-channel closure d = (n (Q/W) / sqrt(S))^(3/5) on channel cells; the terrain still-water depth on standing water",
            "ford_velocity": "standard: continuity v = Q/(W d), identical to Manning v = (1/n) d^(2/3) sqrt(S) by construction; zero on standing water",
            "crossability": "standard: D x V product, the flume-verified stability currency (Cox, Shand & Blacka 2010; AIDR Guideline 7-3), computed from the emitted f32 factors so the identity is exact; still-water depth alone on standing water; the published bands are consumer config — none baked (ADR 0011 D2/D4)",
            "navigability_ts": "standard: Langbein (1962) minimum specific tractive force Ts = V^2(f+0.6)/(1600 D^(4/3)), imperial-unit constants — SI inputs converted; f from Manning n via Darcy-Weisbach f = 8gn^2/R^(1/3) (R ~ d, the ford chain's friction assumption); 0 on still water; f32::MAX outside the water domain",
            "navigable": "standard: banded predicate — Langbein anchor Ts <= nav_max_ts (0.002 published), Magirl & Olsen (2009) slope band S <= nav_max_slope (dimensionless, as-is), pre-modern depth anchor d >= nav_min_depth_m (Eckoldt 0.3-0.7 m via Appel et al. 2024); still water by depth alone; depth values conditional on k_Q",
            "head_of_navigation": "standard: compositional (ADR 0011 D6) — cells of the mouth-connected navigable set (connected downstream to a river mouth along the receivers tree) with no donor in that set",
        },
        // ADR 0011 D2: curation, not a gate — the criterion's semantics live
        // in the experiment config that names it.
        "selection": { "criterion": cfg.label, "satisfied": out.criterion_met },
        "patches": out.patches,
        // Site records in fixed, documented order (ascending cell index).
        // The channelization threshold that parameterizes the confluence
        // count is restated beside it (ADR 0011 D4).
        "affordances": {
            "confluences": {
                "river_min_area_km2": out.river_min_area_km2,
                "sites": out.confluences,
            },
            "passes": {
                "min_persistence_m": cfg.min_pass_persistence_m,
                "sites": out.passes,
            },
            // ADR 0011 Consequences: every metric number downstream of
            // k_Q is conditional on this declared, unverifiable forcing
            // constant — restated wherever such numbers appear.
            "fords": {
                "k_q_m3s_per_unit": cfg.k_q_m3s_per_unit,
                "metric_values_conditional_on_k_q": true,
            },
            "navigability": {
                "nav_max_ts": cfg.nav_max_ts,
                "nav_max_slope": cfg.nav_max_slope,
                "nav_min_depth_m": cfg.nav_min_depth_m,
                "metric_values_conditional_on_k_q": true,
                "head_of_navigation_sites": out.heads_of_navigation,
            },
        },
        // ADR 0011 D6 artifact-contract checks; all must hold or the stage
        // errors before writing this summary.
        "checks": {
            "confluence_definition": confluence_ok,
            "spectrum_identities": spectrum_ok,
            "head_of_navigation_definition": heads_ok,
        },
        "artifact_blake3": hashes,
    });
    std::fs::write(
        run_dir.join(summary_filename(&cfg.label)),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;

    // Register this stage instance in the run manifest under its output
    // namespace (ADR 0011 Consequences: downstream artifacts belong in the
    // manifest; rasters only, like the terrain stage — summaries are not
    // artifacts).
    let manifest_path = run_dir.join("manifest.json");
    let mut manifest = RunManifest::load(&manifest_path)?;
    let artifacts = hashes
        .iter()
        .map(|(name, hash)| {
            (
                name.clone(),
                ArtifactEntry {
                    file: raster_filename(&cfg.label, name),
                    blake3: hash.clone(),
                },
            )
        })
        .collect();
    let mut crate_versions = BTreeMap::new();
    crate_versions.insert(
        "via-suitability".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    manifest.stages.insert(
        format!("suitability.{}", cfg.label),
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
    fn dijkstra_distances_are_grid_metric() {
        // 5×5, single source at the corner.
        let mut src = vec![false; 25];
        src[0] = true;
        let d = dijkstra_dist_m(5, 5, 10.0, &src);
        assert_eq!(d[0], 0.0);
        assert_eq!(d[4], 40.0); // 4 straight steps
        assert!((d[24] - 4.0 * 10.0 * std::f64::consts::SQRT_2).abs() < 1e-9); // diagonal
        assert!((d[5 + 2] - (10.0 * std::f64::consts::SQRT_2 + 10.0)).abs() < 1e-9);
    }

    #[test]
    fn patches_are_ranked_by_size_deterministically() {
        // Two patches: a 2×2 block and a 1×3 strip, min 2 cells.
        let w = 6;
        let mut suitable = vec![false; 36];
        for &(x, y) in &[(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
            suitable[(y * w + x) as usize] = true;
        }
        for &(x, y) in &[(4u32, 4u32), (5, 4), (4, 5)] {
            suitable[(y * w + x) as usize] = true;
        }
        let patches = extract_patches(w, 6, &suitable, 2);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].len(), 4);
        assert_eq!(patches[1].len(), 3);
        assert_eq!(patches[0][0], 0); // scan order within the patch
    }

    #[test]
    fn labels_are_confined_to_safe_filename_characters() {
        for ok in ["village_site", "site-2", "a"] {
            assert!(validate_label(ok).is_ok(), "{ok:?} should be accepted");
        }
        for bad in ["", "Village", "a b", "a/b", "../x", "sité"] {
            assert!(validate_label(bad).is_err(), "{bad:?} should be rejected");
        }
        assert_eq!(
            raster_filename("village_site", "slope"),
            "suitability.village_site.slope.vrast"
        );
        assert_eq!(
            summary_filename("village_site"),
            "suitability.village_site.json"
        );
    }
}
