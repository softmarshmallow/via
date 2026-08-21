use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use via_terrain::gates::Gate;
use via_terrain::{stage, TerrainConfig};
use via_viz::VizInput;

#[derive(Parser)]
#[command(name = "via", version, about = "via pipeline runner")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the terrain stage: artifacts, gates, and the QA map pack.
    Terrain(TerrainArgs),
    /// Run the suitability stage against an existing terrain run directory.
    Suitability(SuitabilityArgs),
    /// Run the ecology stage against an existing terrain run directory.
    Ecology(EcologyArgs),
}

#[derive(Args)]
struct EcologyArgs {
    /// Terrain run directory (contains heights_cm.vrast, precip.vrast, …).
    run_dir: PathBuf,
    /// Path to an EcologyConfig JSON; defaults apply if omitted.
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Args)]
struct SuitabilityArgs {
    /// Terrain run directory (contains heights_cm.vrast, manifest.json, …).
    run_dir: PathBuf,
    /// Path to a SuitabilityConfig JSON; defaults apply if omitted.
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Args)]
struct TerrainArgs {
    /// Path to a TerrainConfig JSON (e.g. an experiment config); omitted
    /// fields take defaults. Without this, the research default applies.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Overrides the config's seed.
    #[arg(long)]
    seed: Option<u64>,
    /// Grid side length in cells (overrides the config).
    #[arg(long)]
    size: Option<u32>,
    /// Step count (overrides the config).
    #[arg(long)]
    steps: Option<u32>,
    /// Cell edge length in metres (overrides the config).
    #[arg(long)]
    cell: Option<f64>,
    /// Output run directory (default: runs/terrain-s<seed>-<size>).
    #[arg(long)]
    out: Option<PathBuf>,
}

fn gate_line(name: &str, g: &Gate, extra: &str) -> String {
    let status = if g.pass {
        "PASS"
    } else if g.advisory {
        "ADVISORY"
    } else {
        "FAIL"
    };
    format!(
        "  {name:<24} {:>10.4}   [{:.2}, {:.2}]  {status}{extra}",
        g.value, g.lo, g.hi
    )
}

fn run_terrain(args: TerrainArgs) -> Result<()> {
    let mut cfg = match &args.config {
        Some(path) => TerrainConfig::from_json_file(path).map_err(anyhow::Error::msg)?,
        None => TerrainConfig::default(),
    };
    if let Some(seed) = args.seed {
        cfg.seed = seed;
    }
    if let Some(size) = args.size {
        cfg.size = size;
    }
    if let Some(steps) = args.steps {
        cfg.steps = steps;
    }
    if let Some(cell) = args.cell {
        cfg.cell_size_m = cell;
    }
    cfg.validate().map_err(anyhow::Error::msg)?;
    let out_dir = args
        .out
        .unwrap_or_else(|| PathBuf::from(format!("runs/terrain-s{}-{}", cfg.seed, cfg.size)));

    let km = cfg.size as f64 * cfg.cell_size_m / 1000.0;
    println!(
        "via terrain — seed {} — {}×{} cells at {} m ({km:.0}×{km:.0} km) — {} steps × {} yr",
        cfg.seed, cfg.size, cfg.size, cfg.cell_size_m, cfg.steps, cfg.dt_years
    );

    let t0 = Instant::now();
    let report_every = (cfg.steps / 12).max(1);
    let mut progress = |step: u32, max_dh: f64| {
        if (step + 1).is_multiple_of(report_every) || step + 1 == cfg.steps {
            println!(
                "  step {:>4}/{}  max|Δh| = {:>8.3} m",
                step + 1,
                cfg.steps,
                max_dh
            );
            let _ = std::io::stdout().flush();
        }
    };
    let out = via_terrain::run(&cfg, &mut progress);
    let sim_ms = t0.elapsed().as_millis();

    stage::write_run(&out_dir, &out)?;

    // Stats record: timing and the convergence trace. Timing varies run to
    // run, so this file is excluded from the determinism contract.
    let stats = serde_json::json!({
        "wall_ms_simulation": sim_ms,
        "steps": cfg.steps,
        "gs_iterations_max": out.gs_iterations_max,
        "convergence_dh_m": out.convergence_m,
    });
    std::fs::write(
        out_dir.join("stats.json"),
        serde_json::to_string_pretty(&stats)? + "\n",
    )?;

    // QA map pack.
    let t1 = Instant::now();
    let render_dir = out_dir.join("render");
    std::fs::create_dir_all(&render_dir)?;
    let inp = VizInput {
        w: out.grid.w,
        h: out.grid.h,
        dx: out.grid.dx,
        sea_level: cfg.sea_level_m,
        heights_m: &out.heights_m,
        receivers: &out.receivers,
        discharge_cells: &out.discharge_cells,
        strahler: &out.strahler,
        basin: &out.basin,
        land: &out.land,
        water_depth: &out.water_depth_m,
        lake_min_depth_m: LAKE_RENDER_MIN_M,
        river_min_cells: cfg.river_min_cells(),
    };
    let relief = via_viz::render_relief(&inp);
    let accum = via_viz::render_accumulation(&inp);
    let basins = via_viz::render_basins(&inp);
    let shade = via_viz::render_hillshade_gray(&inp);
    relief.save(render_dir.join("relief_rivers.png"))?;
    accum.save(render_dir.join("flow_accumulation.png"))?;
    basins.save(render_dir.join("basins.png"))?;
    shade.save(render_dir.join("hillshade.png"))?;
    via_viz::compose_sheet(&[&relief, &accum, &basins, &shade], 12)
        .save(render_dir.join("sheet.png"))?;
    // M3 fields: sediment thickness (m) and standing-water depth (m).
    via_viz::render_scalar(
        &inp,
        &out.sediment_m,
        &[
            (0.0, [236, 232, 222]),
            (0.5, [214, 196, 158]),
            (2.0, [188, 158, 110]),
            (8.0, [150, 112, 72]),
            (32.0, [96, 70, 46]),
        ],
    )
    .save(render_dir.join("sediment.png"))?;
    // The water-depth panel shows the spectrum itself, so lake cells must
    // not be masked out as water: render with the lake threshold disabled.
    let inp_spectrum = VizInput {
        lake_min_depth_m: f64::INFINITY,
        ..inp
    };
    via_viz::render_scalar(
        &inp_spectrum,
        &out.water_depth_m,
        &[
            (0.0, [236, 238, 240]),
            (0.05, [186, 208, 222]),
            (0.3, [120, 165, 200]),
            (2.0, [60, 110, 170]),
            (10.0, [28, 60, 120]),
        ],
    )
    .save(render_dir.join("water_depth.png"))?;
    // M4 panels (ADR 0007), only when the column is non-trivial: exposed
    // unit (categorical — every cell sits exactly on an integer stop) and
    // the karst-potential spectrum when any unit is soluble.
    if cfg.lithology.units.len() > 1 {
        const LITH_COLORS: [[u8; 3]; 16] = [
            [225, 213, 189],
            [166, 118, 90],
            [116, 158, 189],
            [186, 179, 100],
            [140, 102, 152],
            [104, 152, 112],
            [204, 140, 158],
            [96, 112, 132],
            [214, 170, 118],
            [88, 130, 104],
            [176, 144, 196],
            [148, 148, 74],
            [98, 140, 168],
            [190, 108, 84],
            [130, 170, 158],
            [160, 126, 108],
        ];
        // Beyond 16 units, later cycles darken by 30% per lap so no two
        // unit indices ever share an RGB (the config caps units at 64).
        let unit_color = |k: usize| -> [u8; 3] {
            let base = LITH_COLORS[k % LITH_COLORS.len()];
            let lap = (k / LITH_COLORS.len()) as u32;
            base.map(|c| {
                let mut v = c as f64;
                for _ in 0..lap {
                    v *= 0.7;
                }
                v.round() as u8
            })
        };
        let unit_f: Vec<f64> = out.lithology_unit.iter().map(|&u| u as f64).collect();
        let stops: Vec<(f64, [u8; 3])> = (0..cfg.lithology.units.len())
            .map(|k| (k as f64, unit_color(k)))
            .collect();
        via_viz::render_scalar(&inp, &unit_f, &stops).save(render_dir.join("lithology.png"))?;
    }
    let karst_max = out.karst_potential.iter().copied().fold(0.0f64, f64::max);
    if karst_max > 0.0 {
        via_viz::render_scalar(
            &inp,
            &out.karst_potential,
            &[
                (0.0, [238, 236, 230]),
                (karst_max * 0.02, [200, 190, 214]),
                (karst_max * 0.2, [150, 120, 180]),
                (karst_max, [84, 44, 130]),
            ],
        )
        .save(render_dir.join("karst_potential.png"))?;
    }
    println!(
        "  simulation {:.1}s, render {:.1}s → {}",
        sim_ms as f64 / 1000.0,
        t1.elapsed().as_millis() as f64 / 1000.0,
        out_dir.display()
    );

    // Gates summary.
    let g = &out.gates;
    println!("\nGATES");
    println!(
        "{}",
        gate_line(
            "slope–area θ",
            &g.slope_area_theta,
            &format!(
                "   (R² = {:.3}, n = {}, {} flat cells excluded)",
                g.slope_area_r2, g.slope_area_n_cells, g.slope_area_flat_cells_excluded
            ),
        )
    );
    println!(
        "{}",
        gate_line("SPL residual (median)", &g.spl_residual_median, "")
    );
    match &g.hack_exponent {
        Some(gate) => println!(
            "{}",
            gate_line(
                "Hack exponent h",
                gate,
                &format!("   ({} subbasins)", g.hack_n_basins),
            )
        ),
        None => println!(
            "  Hack exponent h        (insufficient basins: {})",
            g.hack_n_basins
        ),
    }
    match &g.horton_bifurcation {
        Some(gate) => println!(
            "{}",
            gate_line(
                "Horton Rb",
                gate,
                &format!(
                    "   (RL = {})",
                    g.horton_length_ratio
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "—".into())
                ),
            )
        ),
        None => println!("  Horton Rb              (insufficient stream orders)"),
    }
    println!(
        "{}",
        gate_line("hypsometric integral", &g.hypsometric_integral, "")
    );
    println!("{}", gate_line("pit cells", &g.pit_cells, ""));
    println!(
        "{}",
        gate_line("drainage completeness", &g.drainage_completeness, "")
    );
    println!(
        "  sediment mass closure  {:>10.2e}   [{:.0e}, {:.0e}]  {}   (det {:.3e} m³ = dep {:.3e} + exp {:.3e}; pond merge {:.2e})",
        g.sediment_mass_closure.value,
        g.sediment_mass_closure.lo,
        g.sediment_mass_closure.hi,
        if g.sediment_mass_closure.pass { "PASS" } else { "FAIL" },
        g.sediment_detached_m3,
        g.sediment_deposited_m3,
        g.sediment_exported_m3,
        g.pond_merge_m3,
    );
    println!(
        "{}",
        gate_line(
            "lake level spread (m)",
            &g.lake_level_spread_m,
            &format!("   ({} lake cells ≥ 0.3 m)", g.lake_cells),
        )
    );
    match &g.floodplain_slope_ratio {
        Some(gate) => println!(
            "{}",
            gate_line(
                "floodplain/channel S",
                gate,
                &format!("   (mean sediment on land {:.2} m)", g.sediment_mean_land_m),
            )
        ),
        None => println!(
            "  floodplain/channel S   (no floodplain cells; mean sediment {:.2} m)",
            g.sediment_mean_land_m
        ),
    }
    println!(
        "{}",
        gate_line(
            "rain-shadow ratio",
            &g.rain_shadow_ratio,
            &format!("   (wind [{}, {}])", g.wind.0, g.wind.1),
        )
    );
    if let Some(gate) = &g.unit_spl_consistency {
        println!(
            "{}",
            gate_line(
                "unit SPL consistency",
                gate,
                &format!(
                    "   ({} units; θ fit on K class {:.3e})",
                    g.unit_spl_units, g.slope_area_modal_k
                ),
            )
        );
    }
    println!(
        "  land fraction          {:>10.4}\n  max elevation          {:>8.1} m\n  interior water cells   {:>7}",
        g.land_fraction, g.max_elevation_m, g.interior_water_cells
    );
    println!(
        "\nOVERALL: {}",
        if g.overall_pass { "PASS" } else { "FAIL" }
    );
    Ok(())
}

/// Standing-water depth at or above which renders draw a water surface.
/// Matches the via-ecology lake threshold so map packs and biome maps agree.
const LAKE_RENDER_MIN_M: f64 = 0.30;

/// Terrain rasters loaded for rendering downstream-stage overlays.
struct RunRasters {
    w: u32,
    h: u32,
    dx: f64,
    sea: f64,
    heights_m: Vec<f64>,
    receivers: Vec<u32>,
    discharge: Vec<f64>,
    strahler: Vec<u32>,
    basin: Vec<u32>,
    land: Vec<bool>,
    water_depth: Vec<f64>,
    river_min_cells: u64,
}

impl RunRasters {
    fn load(dir: &std::path::Path) -> Result<Self> {
        use via_artifact::raster::Raster;
        let heights = Raster::<i32>::read_file(&dir.join("heights_cm.vrast"))?;
        let receivers = Raster::<u32>::read_file(&dir.join("receivers.vrast"))?;
        let discharge = Raster::<f32>::read_file(&dir.join("discharge.vrast"))?;
        let strahler = Raster::<u32>::read_file(&dir.join("strahler.vrast"))?;
        let basin = Raster::<u32>::read_file(&dir.join("basin.vrast"))?;
        let water_depth = Raster::<f32>::read_file(&dir.join("water_depth.vrast"))?;
        for (name, w, h, cell) in [
            (
                "receivers",
                receivers.width,
                receivers.height,
                receivers.cell_size_cm,
            ),
            (
                "discharge",
                discharge.width,
                discharge.height,
                discharge.cell_size_cm,
            ),
            (
                "strahler",
                strahler.width,
                strahler.height,
                strahler.cell_size_cm,
            ),
            ("basin", basin.width, basin.height, basin.cell_size_cm),
            (
                "water_depth",
                water_depth.width,
                water_depth.height,
                water_depth.cell_size_cm,
            ),
        ] {
            if (w, h, cell) != (heights.width, heights.height, heights.cell_size_cm) {
                anyhow::bail!(
                    "artifact {name}.vrast grid {w}×{h} @ {cell} cm does not match \
                     heights_cm {}×{} @ {} cm — stale or mixed run directory?",
                    heights.width,
                    heights.height,
                    heights.cell_size_cm
                );
            }
        }
        let manifest = via_artifact::RunManifest::load(&dir.join("manifest.json"))?;
        let tcfg = manifest.stage_config("terrain");
        let sea = tcfg
            .and_then(|c| c.get("sea_level_m"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let river_min_km2 = tcfg
            .and_then(|c| c.get("river_min_area_km2"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let dx = heights.cell_size_cm as f64 / 100.0;
        let heights_m: Vec<f64> = heights.data.iter().map(|&cm| cm as f64 / 100.0).collect();
        // The stage's authoritative base-level mask: self-receiver = ocean
        // (the pit gate guarantees no land self-receivers). Re-deriving land
        // from cm-quantized heights flips shoreline cells.
        let land: Vec<bool> = receivers
            .data
            .iter()
            .enumerate()
            .map(|(i, &r)| r as usize != i)
            .collect();
        Ok(Self {
            w: heights.width,
            h: heights.height,
            dx,
            sea,
            heights_m,
            receivers: receivers.data,
            discharge: discharge.data.iter().map(|&v| v as f64).collect(),
            strahler: strahler.data,
            basin: basin.data,
            land,
            water_depth: water_depth.data.iter().map(|&v| v as f64).collect(),
            river_min_cells: ((river_min_km2 * 1.0e6) / (dx * dx)).ceil() as u64,
        })
    }

    fn viz(&self) -> via_viz::VizInput<'_> {
        via_viz::VizInput {
            w: self.w,
            h: self.h,
            dx: self.dx,
            sea_level: self.sea,
            heights_m: &self.heights_m,
            receivers: &self.receivers,
            discharge_cells: &self.discharge,
            strahler: &self.strahler,
            basin: &self.basin,
            land: &self.land,
            water_depth: &self.water_depth,
            lake_min_depth_m: LAKE_RENDER_MIN_M,
            river_min_cells: self.river_min_cells,
        }
    }
}

fn run_suitability(args: SuitabilityArgs) -> Result<()> {
    let cfg = match &args.config {
        Some(p) => {
            via_suitability::SuitabilityConfig::from_json_file(p).map_err(anyhow::Error::msg)?
        }
        None => via_suitability::SuitabilityConfig::default(),
    };
    let dir = &args.run_dir;
    let out = via_suitability::run(dir, &cfg)?;
    via_suitability::write_outputs(dir, &cfg, &out)?;

    let rr = RunRasters::load(dir)?;
    let inp = rr.viz();
    std::fs::create_dir_all(dir.join("render"))?;
    via_viz::render_suitability(&inp, &out.suitable, &out.patch_rank)
        .save(dir.join(format!("render/suitability.{}.png", cfg.label)))?;

    // Affordance and harbour panels, with the two QA diagnostics
    // (ADR 0008 D10 visual channel — never gates, never in the summary).
    use via_artifact::raster::Raster;
    let area_cells = Raster::<u64>::read_file(&dir.join("area_cells.vrast"))?;
    let surface_m: Vec<f64> = rr
        .heights_m
        .iter()
        .zip(rr.water_depth.iter())
        .zip(rr.land.iter())
        .map(|((&hm, &wd), &l)| if l { hm + wd } else { rr.sea })
        .collect();
    let qa_cols =
        via_suitability::qa::basin_boundary_cols(rr.w, rr.h, &rr.heights_m, &rr.basin, &rr.land);
    let qa_cp = via_suitability::qa::filet_change_points(
        rr.w,
        rr.h,
        &via_suitability::qa::StemInputs {
            receivers: &rr.receivers,
            strahler: &rr.strahler,
            area_cells: &area_cells.data,
            land: &rr.land,
            surface_m: &surface_m,
        },
        10,
    );
    let confluence_cells: Vec<u32> = out.confluences.iter().map(|s| s.cell).collect();
    let pass_cells: Vec<u32> = out.passes.iter().map(|s| s.cell).collect();
    let head_cells: Vec<u32> = out.heads_of_navigation.iter().map(|s| s.cell).collect();
    via_viz::render_affordances(
        &inp,
        &via_viz::AffordanceOverlay {
            crossability: &out.ford.crossability,
            confluence_cells: &confluence_cells,
            pass_cells: &pass_cells,
            head_cells: &head_cells,
            qa_col_cells: &qa_cols,
            qa_change_point_cells: &qa_cp,
        },
    )
    .save(dir.join(format!("render/suitability.{}.affordances.png", cfg.label)))?;
    via_viz::render_harbour(&inp, &out.harbour.fetch_m, &out.harbour.sediment)
        .save(dir.join(format!("render/suitability.{}.harbour.png", cfg.label)))?;

    println!("SUITABILITY — criterion '{}'", cfg.label);
    println!("  rank   area_ha   centroid_cell   mean_slope   elev_m   fw_dist_m   coast_m");
    for p in &out.patches {
        println!(
            "  {:>4}  {:>8.1}   ({:>5.0},{:>5.0})   {:>9.4}  {:>7.1}   {:>9.0}   {:>7.0}",
            p.rank,
            p.area_ha,
            p.centroid_x,
            p.centroid_y,
            p.mean_slope,
            p.mean_elevation_m,
            p.min_freshwater_dist_m,
            p.min_coast_dist_m
        );
    }
    // ADR 0011 D4: the channelization threshold is restated with any
    // reported confluence count.
    let river_min = match out.river_min_area_km2 {
        Some(v) => format!("{v} km²"),
        None => "undeclared".to_string(),
    };
    println!(
        "\n  {} confluence site(s) (river threshold {}), {} pass site(s) ≥ {} m \
         persistence, {} head(s) of navigation",
        out.confluences.len(),
        river_min,
        out.passes.len(),
        cfg.min_pass_persistence_m,
        out.heads_of_navigation.len()
    );
    // QA correspondence: saddles within one cell (Chebyshev) of a
    // basin-boundary col — a diagnostic, not an invariant.
    let col_set: std::collections::HashSet<(i64, i64)> = qa_cols
        .iter()
        .map(|&c| ((c % rr.w) as i64, (c / rr.w) as i64))
        .collect();
    let near = out
        .passes
        .iter()
        .filter(|s| {
            (-1..=1).any(|oy: i64| {
                (-1..=1).any(|ox: i64| col_set.contains(&(s.x as i64 + ox, s.y as i64 + oy)))
            })
        })
        .count();
    println!(
        "  QA (visual channel): {} basin-boundary col(s), {}/{} pass site(s) within \
         1 cell of one; {} Filet change-point(s)",
        qa_cols.len(),
        near,
        out.passes.len(),
        qa_cp.len()
    );
    println!(
        "CRITERION {}: {}   ({} patch(es) ≥ {} ha)",
        cfg.label,
        if out.criterion_met {
            "SATISFIED"
        } else {
            "NOT SATISFIED"
        },
        out.patches.len(),
        cfg.min_patch_area_ha
    );
    Ok(())
}

/// Presentation palette for `via_ecology::Biome`, indexed by discriminant.
const BIOME_COLORS: [[u8; 3]; 14] = [
    [52, 88, 148],   // water (ocean)
    [205, 210, 195], // tundra
    [70, 105, 85],   // boreal forest
    [92, 145, 78],   // temperate seasonal forest
    [40, 115, 75],   // temperate rainforest
    [18, 95, 52],    // tropical rainforest
    [75, 135, 65],   // tropical seasonal forest
    [205, 185, 95],  // savanna
    [228, 208, 152], // desert
    [172, 192, 105], // grassland
    [152, 162, 92],  // shrubland
    [75, 140, 145],  // wetland
    [132, 128, 124], // bare rock
    [96, 148, 200],  // lake (lighter than ocean)
];

fn run_ecology(args: EcologyArgs) -> Result<()> {
    use via_artifact::raster::Raster;
    let cfg = match &args.config {
        Some(p) => via_ecology::EcologyConfig::from_json_file(p).map_err(anyhow::Error::msg)?,
        None => via_ecology::EcologyConfig::default(),
    };
    let dir = &args.run_dir;
    let out = via_ecology::run(dir, &cfg)?;
    via_ecology::write_outputs(dir, &cfg, &out)?;

    let rr = RunRasters::load(dir)?;
    let inp = rr.viz();
    let render_dir = dir.join("render");
    std::fs::create_dir_all(&render_dir)?;

    let biome_img = via_viz::render_classes(&inp, &out.biome, &BIOME_COLORS);
    biome_img.save(render_dir.join("biome.png"))?;

    let precip = Raster::<f32>::read_file(&dir.join("precip.vrast"))?;
    let precip_f64: Vec<f64> = precip.data.iter().map(|&v| v as f64).collect();
    let precip_img = via_viz::render_scalar(
        &inp,
        &precip_f64,
        &[
            (0.2, [216, 196, 150]),
            (0.8, [170, 190, 120]),
            (1.2, [110, 170, 150]),
            (1.8, [60, 130, 170]),
            (2.6, [30, 80, 150]),
        ],
    );
    precip_img.save(render_dir.join("precip.png"))?;

    let temp = Raster::<f32>::read_file(&dir.join("temperature.vrast"))?;
    let temp_f64: Vec<f64> = temp.data.iter().map(|&v| v as f64).collect();
    let temp_img = via_viz::render_scalar(
        &inp,
        &temp_f64,
        &[
            (0.0, [60, 80, 160]),
            (8.0, [120, 170, 190]),
            (14.0, [230, 225, 170]),
            (20.0, [230, 160, 90]),
            (26.0, [190, 60, 50]),
        ],
    );
    temp_img.save(render_dir.join("temperature.png"))?;

    let veg_img = via_viz::render_scalar(
        &inp,
        &out.veg_density,
        &[
            (0.0, [235, 230, 210]),
            (0.3, [190, 200, 140]),
            (0.6, [110, 160, 90]),
            (1.0, [30, 100, 50]),
        ],
    );
    veg_img.save(render_dir.join("veg_density.png"))?;

    via_viz::compose_sheet(&[&biome_img, &precip_img, &temp_img, &veg_img], 12)
        .save(render_dir.join("ecology_sheet.png"))?;

    println!("ECOLOGY — biome classes and vegetation attributes");
    println!("  {:<28} {:>10} {:>12}", "biome", "cells", "area_ha");
    for (name, cells, ha) in &out.class_areas {
        println!("  {name:<28} {cells:>10} {ha:>12.1}");
    }
    let land_cells: u64 = out.class_areas.iter().map(|(_, c, _)| c).sum();
    let veg_mean = if land_cells > 0 {
        out.veg_density.iter().sum::<f64>() / land_cells as f64
    } else {
        0.0
    };
    println!(
        "\n  {} biome classes on land, mean vegetation density {:.3}",
        out.class_areas.len(),
        veg_mean
    );
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Terrain(args) => run_terrain(args),
        Cmd::Suitability(args) => run_suitability(args),
        Cmd::Ecology(args) => run_ecology(args),
    }
}
