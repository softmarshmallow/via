//! SPIKE — settlement systems across eras. Not a stage, not gated, not
//! adopted. It exists to test one claim from docs/research/humanity: that
//! era is *forcing*, not a different model — that one mechanism chain
//! (land productivity → multimodal travel cost → Harris–Wilson center
//! dynamics → corridor network) reproduces medieval, frontier and modern
//! settlement patterns when only its declared parameters change.
//!
//! Read spikes/settlements/README.md before believing any number here.

// A spike keeps fields and helpers it does not use yet — the point is to
// have them at hand while exploring, not to ship a minimal API.
#![allow(dead_code, clippy::too_many_arguments)]

mod cost;
mod fields;
mod font;
mod hw;
mod measure;
mod network;
mod render;
mod sites;
mod util;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EraConfig {
    pub name: String,
    #[serde(default)]
    pub travel: cost::TravelConfig,
    #[serde(default)]
    pub dynamics: hw::DynamicsConfig,
    #[serde(default)]
    pub network: network::NetworkConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SpikeConfig {
    pub land: fields::LandConfig,
    pub sites: sites::SiteConfig,
    /// Demand is aggregated into blocks of this many cells per side; the
    /// cost matrix is sites × blocks, so this sets the run's cost.
    pub demand_block_cells: u32,
    /// Centers closer than this are one agglomeration when the
    /// distributional statistics are computed (docs/research/humanity/0011:
    /// the delineation must be declared before any band means anything).
    pub delineation_km: f64,
    pub eras: Vec<EraConfig>,
}

impl Default for SpikeConfig {
    fn default() -> Self {
        Self {
            land: Default::default(),
            sites: Default::default(),
            demand_block_cells: 8,
            delineation_km: 5.0,
            eras: Vec::new(),
        }
    }
}

#[derive(Parser)]
#[command(
    name = "via-spike-settlements",
    about = "SPIKE: era-as-forcing settlement systems"
)]
struct Cli {
    /// Terrain run directory (needs heights, receivers, strahler, precip,
    /// temperature, sediment, discharge, water_depth).
    run_dir: PathBuf,
    /// Spike config JSON (land, sites, eras).
    #[arg(long)]
    config: PathBuf,
    /// Output directory for figures and the stats record.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Serialize)]
struct EraReport {
    name: String,
    travel: cost::TravelConfig,
    dynamics: hw::DynamicsConfig,
    network: network::NetworkConfig,
    measurements: measure::Measurements,
    /// Sizes of the ten largest centers (people).
    top_sizes: Vec<f64>,
    /// Grid cells of the ten largest centers, so a downstream spike can
    /// pick a town to grow fabric on.
    top_cells: Vec<u32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg: SpikeConfig = serde_json::from_slice(
        &std::fs::read(&cli.config).with_context(|| format!("reading {}", cli.config.display()))?,
    )
    .context("parsing spike config")?;
    anyhow::ensure!(!cfg.eras.is_empty(), "config declares no eras");
    std::fs::create_dir_all(&cli.out)?;

    let t0 = Instant::now();
    let land = fields::load(&cli.run_dir, &cfg.land)
        .with_context(|| format!("loading run {}", cli.run_dir.display()))?;
    let n = land.w as usize * land.h as usize;
    let land_cells = (0..n).filter(|&i| land.land[i] && !land.lake[i]).count();
    let food_base: f64 = (0..n).map(|i| land.population_capacity(i)).sum();
    println!(
        "land {}×{} @ {:.0} m — {:.0} km² above water, food base {:.0} people",
        land.w,
        land.h,
        land.dx,
        land_cells as f64 * land.cell_area_km2,
        food_base
    );

    // Demand blocks: one representative cell each, era-independent.
    let b = cfg.demand_block_cells.max(1);
    let bw = land.w.div_ceil(b);
    let bh = land.h.div_ceil(b);
    let mut origin = vec![0.0f64; (bw * bh) as usize];
    let mut block_rep = vec![u32::MAX; (bw * bh) as usize];
    {
        let mut best = vec![f64::NEG_INFINITY; (bw * bh) as usize];
        for i in 0..n {
            if !land.land[i] || land.lake[i] {
                continue;
            }
            let (x, y) = land.xy(i);
            let bi = ((y / b) * bw + (x / b)) as usize;
            origin[bi] += land.population_capacity(i);
            if land.productivity[i] > best[bi] {
                best[bi] = land.productivity[i];
                block_rep[bi] = i as u32;
            }
        }
    }
    let live_blocks: Vec<usize> = (0..origin.len())
        .filter(|&i| origin[i] > 0.0 && block_rep[i] != u32::MAX)
        .collect();
    let origins: Vec<f64> = live_blocks.iter().map(|&i| origin[i]).collect();
    let reps: Vec<u32> = live_blocks.iter().map(|&i| block_rep[i]).collect();

    let st = sites::select(&land, &cfg.sites);
    println!(
        "{} candidate sites, {} demand blocks ({} m each)  [{:.1}s]",
        st.len(),
        reps.len(),
        b as f64 * land.dx,
        t0.elapsed().as_secs_f64()
    );

    let mut reports = Vec::new();
    let mut maps = Vec::new();
    let mut rank_series = Vec::new();
    let mut spacing_series = Vec::new();
    let mut isochrones = Vec::new();
    let mut isochrone_origin: Option<u32> = None;
    let mut drawn_surfaces: Vec<String> = Vec::new();

    for (ei, era) in cfg.eras.iter().enumerate() {
        let te = Instant::now();
        let surface = cost::CostSurface::build(&land, &era.travel);

        // One Dijkstra per candidate: hours to every demand block and to
        // every other candidate. Rows are independent, so the parallel map
        // is order-free and the result is reproducible.
        let rows: Vec<(Vec<f64>, Vec<f64>)> = st
            .cell
            .par_iter()
            .map(|&c| {
                let t = surface.time_from(c);
                let to_blocks = reps.iter().map(|&r| t[r as usize]).collect::<Vec<_>>();
                let to_sites = st.cell.iter().map(|&o| t[o as usize]).collect::<Vec<_>>();
                (to_blocks, to_sites)
            })
            .collect();
        let pair_hours: Vec<Vec<f64>> = rows.iter().map(|(_, s)| s.clone()).collect();
        // Block-major for the solver's hot loop.
        let mut cost_bj = vec![vec![f64::INFINITY; st.len()]; reps.len()];
        for (j, (to_blocks, _)) in rows.iter().enumerate() {
            for (bi, &c) in to_blocks.iter().enumerate() {
                cost_bj[bi][j] = c;
            }
        }
        drop(rows);

        let eq = hw::solve(&cost_bj, &origins, &st.quality, &era.dynamics);
        let mut centers: Vec<(usize, u32, f64)> = (0..st.len())
            .filter(|&j| eq.alive[j])
            .map(|j| (j, st.cell[j], eq.w[j]))
            .collect();
        centers.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap().then(a.0.cmp(&b.0)));

        let net = network::build(
            &surface,
            &centers,
            &pair_hours,
            era.dynamics.beta_per_hour,
            land.dx,
            &era.network,
        );

        let m = measure::measure(
            &land,
            st.len(),
            &centers,
            eq.mean_travel_hours,
            eq.allocated_people,
            net.total_length_km,
            net.reused_length_km,
            eq.converged,
            eq.iterations,
            cfg.delineation_km,
        );
        println!(
            "{:<21} {:>4} centers {:>4} agglom  spacing {:>5.1} km  travel {:.2} h  \
             zeta(agg) {:>5.2}  primacy {:>5.1}%  roads {:>5.0} km  iter {:>5}{}  [{:.1}s]",
            era.name,
            m.centers,
            m.agglomerations,
            m.spacing_median_km,
            m.mean_travel_hours,
            m.agglomeration_zeta,
            m.primacy * 100.0,
            m.network_km,
            m.iterations,
            if m.converged { "" } else { " NOCONV" },
            te.elapsed().as_secs_f64()
        );

        let accent = render::ERA_COLORS[ei % render::ERA_COLORS.len()];
        maps.push(render::settlement_map(
            &land,
            &centers,
            &net,
            &era.name.to_uppercase(),
            &m,
            accent,
        ));

        let sizes: Vec<f64> = centers.iter().map(|&(_, _, w)| w).collect();
        let cells: Vec<u32> = centers.iter().map(|&(_, c, _)| c).collect();
        // Plot the unit the rank-size literature fits, and the unit the
        // reported zeta is computed on: agglomerations, not raw centers.
        rank_series.push((
            era.name.to_uppercase(),
            measure::agglomerate(&land, &cells, &sizes, cfg.delineation_km),
            accent,
        ));
        spacing_series.push((
            era.name.to_uppercase(),
            nearest_spacing_km(&land, &cells),
            accent,
        ));

        // Isochrones from one fixed place, so the panels differ only by era.
        // Eras that share a travel surface would draw the same panel twice.
        // If an era leaves no centers at all, fall back to the best
        // candidate so the figure still reports what happened.
        let fallback = centers.first().map(|c| c.1).unwrap_or(st.cell[0]);
        let origin_cell = *isochrone_origin.get_or_insert(fallback);
        let travel_key = serde_json::to_string(&era.travel)?;
        if !drawn_surfaces.contains(&travel_key) {
            drawn_surfaces.push(travel_key);
            let hours = surface.time_from(origin_cell);
            isochrones.push(render::isochrone_map(
                &land,
                &hours,
                &[0.5, 1.0, 2.0, 4.0],
                &format!("{} REACH 0.5/1/2/4 H", era.name.to_uppercase()),
                accent,
                origin_cell,
            ));
        }

        reports.push(EraReport {
            name: era.name.clone(),
            travel: era.travel.clone(),
            dynamics: era.dynamics.clone(),
            network: era.network.clone(),
            measurements: m,
            top_sizes: sizes.iter().take(10).cloned().collect(),
            top_cells: centers.iter().take(10).map(|&(_, c, _)| c).collect(),
        });
    }

    // Figures.
    let cols = if maps.len() > 3 { 2 } else { maps.len() as u32 };
    let sheet = render::compose(
        &maps,
        cols,
        14,
        "SPIKE: ONE MECHANISM CHAIN - ONLY THE ERA PARAMETERS DIFFER",
    );
    sheet.save(cli.out.join("eras.png"))?;

    // Ramp stops follow the field's own distribution: most of this land is
    // steep or thin-soiled, so a 0–160 ramp would render it uniformly blank.
    let mut prod_sorted: Vec<f64> = (0..n)
        .filter(|&i| land.land[i] && !land.lake[i])
        .map(|i| land.productivity[i])
        .collect();
    prod_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |f: f64| prod_sorted[((prod_sorted.len() as f64 - 1.0) * f) as usize];
    let prod_stops = [
        (0.0, [250, 249, 240]),
        (q(0.50), [224, 234, 190]),
        (q(0.80), [150, 194, 122]),
        (q(0.95), [58, 132, 82]),
        (q(0.999), [20, 72, 50]),
    ];
    let mut input_panels = vec![render::field_map(
        &land,
        &land.productivity,
        &prod_stops,
        "FOOD BASE (PEOPLE/KM2)",
    )];
    input_panels.extend(isochrones);
    let inputs = render::compose(
        &input_panels,
        2,
        14,
        "INPUTS: FOOD BASE, AND HOW FAR ONE FIXED PLACE REACHES PER ERA",
    );
    inputs.save(cli.out.join("inputs.png"))?;

    let charts = render::compose(
        &[
            render::rank_size_chart(&rank_series, 760, 560),
            render::spacing_chart(&spacing_series, 760, 560),
        ],
        2,
        14,
        "MEASUREMENTS VS CANDIDATE GATES (RESEARCH/HUMANITY/0009)",
    );
    charts.save(cli.out.join("measurements.png"))?;

    let record = serde_json::json!({
        "spike": "settlements",
        "status": "research spike — not a stage, not gated, nothing adopted",
        "run_dir": cli.run_dir.display().to_string(),
        "grid": { "w": land.w, "h": land.h, "cell_m": land.dx },
        "land_area_km2": land_cells as f64 * land.cell_area_km2,
        "food_base_people": food_base,
        "candidate_sites": st.len(),
        "demand_blocks": reps.len(),
        "config": cfg,
        "eras": reports,
        "wall_s": t0.elapsed().as_secs_f64(),
    });
    std::fs::write(
        cli.out.join("spike.json"),
        serde_json::to_string_pretty(&record)? + "\n",
    )?;
    println!(
        "wrote {} [{:.1}s]",
        cli.out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn nearest_spacing_km(land: &fields::Land, cells: &[u32]) -> Vec<f64> {
    let w = land.w as i64;
    let mut out = Vec::with_capacity(cells.len());
    for (a, &ca) in cells.iter().enumerate() {
        let (xa, ya) = ((ca as i64 % w) as f64, (ca as i64 / w) as f64);
        let mut best = f64::INFINITY;
        for (b, &cb) in cells.iter().enumerate() {
            if a == b {
                continue;
            }
            let (xb, yb) = ((cb as i64 % w) as f64, (cb as i64 / w) as f64);
            let d = ((xa - xb).powi(2) + (ya - yb).powi(2)).sqrt();
            if d < best {
                best = d;
            }
        }
        if best.is_finite() {
            out.push(best * land.dx / 1000.0);
        }
    }
    out
}
