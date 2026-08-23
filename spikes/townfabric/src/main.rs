//! SPIKE — town fabric: streets, blocks, parcels and buildings for ONE
//! settlement. Not a stage, not gated, not adopted.
//!
//! The macro spike (spikes/settlements) decides where towns are and how
//! big; it stops at a dot. This one takes a dot and grows the thing a
//! player would walk through: a planar street graph over the real terrain
//! patch, the blocks between the streets, the parcels the blocks divide
//! into, and a building on each parcel.
//!
//! Read spikes/townfabric/README.md before believing any number here.

#![allow(dead_code, clippy::too_many_arguments)]

mod font;
mod geom;
mod graph;
mod grow;
mod hierarchy;
mod measure;
mod osm;
mod parcels;
mod patch;
mod render;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use geom::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TownConfig {
    pub name: String,
    /// Bearings (degrees) of the intercity corridors meeting here. The
    /// macro spike's network supplies these in a real pipeline; here they
    /// are declared.
    #[serde(default)]
    pub corridor_bearings_deg: Vec<f64>,
    /// Households to house — the macro spike's population for this town,
    /// divided by household size. The town's extent follows from this.
    pub households: u32,
    /// Cap on growth attempts, so a hopeless site cannot spin forever.
    pub max_steps: u32,
    /// Growth attempts per round; after each round the blocks are
    /// re-subdivided and the built frontier is recomputed.
    #[serde(default = "default_round")]
    pub attempts_per_round: u32,
    #[serde(default)]
    pub growth: grow::GrowthConfig,
    #[serde(default)]
    pub parcels: parcels::ParcelConfig,
    #[serde(default)]
    pub hierarchy: hierarchy::HierarchyConfig,
}

fn default_round() -> u32 {
    350
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SpikeConfig {
    /// Half-width of the town domain (m).
    pub extent_m: f64,
    /// Strahler order at or above which a channel is water the fabric must
    /// bridge rather than cross.
    pub river_min_strahler: u32,
    pub lake_min_depth_m: f64,
    /// Channel width w ≈ coef·√(drainage area km²), in metres.
    pub river_width_coef: f64,
    /// The disc the statistics describe, for both synthetic and reference
    /// towns. One declared protocol, or no number means anything.
    pub study_radius_m: f64,
    pub seed: u64,
    /// Metres per pixel in the town panels and in the zoom panels.
    pub panel_m_per_px: f64,
    pub zoom_m_per_px: f64,
    pub zoom_half_m: f64,
    pub towns: Vec<TownConfig>,
}

impl Default for SpikeConfig {
    fn default() -> Self {
        Self {
            extent_m: 700.0,
            river_min_strahler: 4,
            lake_min_depth_m: 0.5,
            river_width_coef: 4.0,
            study_radius_m: 300.0,
            seed: 42,
            panel_m_per_px: 0.9,
            zoom_m_per_px: 0.22,
            zoom_half_m: 110.0,
            towns: Vec::new(),
        }
    }
}

#[derive(Parser)]
#[command(
    name = "via-spike-townfabric",
    about = "SPIKE: street/parcel/building fabric"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand)]
enum Cmd {
    /// Grow synthetic fabric on a terrain run's town site.
    Grow(GrowArgs),
    /// Measure and render a real town from an OpenStreetMap extract — the
    /// standard the synthetic fabric is judged against.
    Reference(RefArgs),
}

#[derive(clap::Args)]
struct GrowArgs {
    /// Terrain run directory.
    run_dir: PathBuf,
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    out: PathBuf,
    /// Grid cell of the town site. Overrides --from-spike.
    #[arg(long)]
    site_cell: Option<u32>,
    /// Take the site from a settlements spike record (spike.json).
    #[arg(long)]
    from_spike: Option<PathBuf>,
    /// Era name to read from that record.
    #[arg(long, default_value = "medieval")]
    era: String,
    /// Which center of that era, by rank (0 = largest).
    #[arg(long, default_value_t = 0)]
    rank: usize,
}

#[derive(clap::Args)]
struct RefArgs {
    /// Overpass JSON extract.
    osm: PathBuf,
    /// Query centre, "lat,lon".
    #[arg(long)]
    centre: String,
    /// Study radius (m): statistics describe this disc.
    #[arg(long)]
    radius: f64,
    /// Fabric is imported this much further out and participates in the
    /// graph, so streets leaving the study area are not counted as dead
    /// ends and routes through the edge still exist.
    #[arg(long, default_value_t = 1.45)]
    buffer: f64,
    /// Label for the report and the panel.
    #[arg(long)]
    name: String,
    #[arg(long)]
    out: PathBuf,
    /// Metres per pixel in the town panel and the lot-level zoom.
    #[arg(long, default_value_t = 0.85)]
    panel_m_per_px: f64,
    #[arg(long, default_value_t = 0.2)]
    zoom_m_per_px: f64,
    #[arg(long, default_value_t = 110.0)]
    zoom_half_m: f64,
    /// Render the panel without burnt-in labels or scale bar, for figures
    /// that carry their own typography.
    #[arg(long)]
    bare: bool,
    /// Count footpaths, alleys and steps as fabric. In a medieval core
    /// these carry real movement, and excluding them understates
    /// connectivity — so the target number depends on this switch.
    #[arg(long)]
    include_paths: bool,
}

#[derive(Serialize)]
struct TownReport {
    name: String,
    growth: grow::GrowthConfig,
    parcels: parcels::ParcelConfig,
    hierarchy: hierarchy::HierarchyConfig,
    households: u32,
    sites_placed: u32,
    built_radius_m: f64,
    /// Radius actually holding 90% of the buildings.
    observed_r90_m: f64,
    growth_attempts: u32,
    fabric: measure::Fabric,
    /// The same town measured under the reference protocol (lanes and
    /// alleys excluded), which is the only like-for-like comparison.
    fabric_streets_only: measure::Fabric,
}

fn resolve_site(cli: &GrowArgs) -> Result<u32> {
    if let Some(c) = cli.site_cell {
        return Ok(c);
    }
    let path = cli
        .from_spike
        .as_ref()
        .context("need --site-cell or --from-spike")?;
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let eras = v["eras"].as_array().context("spike record has no eras")?;
    let era = eras
        .iter()
        .find(|e| e["name"] == cli.era.as_str())
        .with_context(|| format!("era {} not in {}", cli.era, path.display()))?;
    let cells = era["measurements"]
        .get("centers")
        .and(era["top_cells"].as_array())
        .context("spike record has no top_cells — rerun the settlements spike")?;
    let c = cells
        .get(cli.rank)
        .with_context(|| format!("era {} has no center of rank {}", cli.era, cli.rank))?;
    Ok(c.as_u64().context("bad cell")? as u32)
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Grow(a) => run_grow(a),
        Cmd::Reference(a) => run_reference(a),
    }
}

/// Measure and render a real town. Everything downstream of the import is
/// the same code path the synthetic fabric uses.
fn run_reference(a: RefArgs) -> Result<()> {
    let (lat, lon) = {
        let mut it = a.centre.split(',');
        let lat: f64 = it.next().context("centre needs lat,lon")?.trim().parse()?;
        let lon: f64 = it.next().context("centre needs lat,lon")?.trim().parse()?;
        (lat, lon)
    };
    std::fs::create_dir_all(&a.out)?;
    let t0 = Instant::now();
    let r = osm::load(&a.osm, lat, lon, a.radius, a.buffer, 6.0, a.include_paths)?;
    let fab = measure::measure(
        &a.name,
        &r.graph,
        &r.blocks,
        &r.buildings,
        None,
        Some(a.radius),
    );
    println!(
        "{:<14} {:>5} nodes {:>5} edges  M {:.3}  deadend {:>4.1}%  blocks {:>4}  bldgs {:>5}  \
         wall {:>4.1}%  street-dist {:.1} m  backbone {:.2}  [{:.1}s]",
        a.name,
        fab.nodes,
        fab.edges,
        fab.meshedness,
        fab.dead_end_share * 100.0,
        fab.blocks,
        fab.buildings,
        fab.street_wall_share * 100.0,
        fab.bldg_street_dist_median_m,
        fab.backbone_concentration,
        t0.elapsed().as_secs_f64()
    );

    let side = (2.0 * a.radius / a.panel_m_per_px) as u32;
    let mut c = render::Canvas::new(side, side, 1.0 / a.panel_m_per_px, [0.0, 0.0]);
    c.draw_blocks(&r.blocks);
    let refw = hierarchy::HierarchyConfig::default();
    c.draw_streets(&r.graph, |cl| refw.width_of(cl));
    c.draw_buildings(&r.buildings);
    let focus = densest_point(&r.buildings, a.zoom_half_m);
    c.mark_window(focus, a.zoom_half_m);
    if !a.bare {
        c.label(10, 8, &format!("{} (REAL)", a.name.to_uppercase()), 3);
        c.label(
            10,
            34,
            &format!(
                "{} BLOCKS  {} BUILDINGS  MESHEDNESS {:.2}",
                fab.blocks, fab.buildings, fab.meshedness
            ),
            2,
        );
        c.scale_bar(200.0, "200 M");
    }
    c.img.save(a.out.join(format!("{}-town.png", a.name)))?;

    let zside = (2.0 * a.zoom_half_m / a.zoom_m_per_px) as u32;
    let mut z = render::Canvas::new(zside, zside, 1.0 / a.zoom_m_per_px, focus);
    z.draw_blocks(&r.blocks);
    z.draw_streets(&r.graph, |cl| refw.width_of(cl));
    z.draw_buildings(&r.buildings);
    if !a.bare {
        z.label(
            10,
            8,
            &format!("{} (REAL) - LOT LEVEL", a.name.to_uppercase()),
            3,
        );
        z.scale_bar(50.0, "50 M");
    }
    z.img.save(a.out.join(format!("{}-lots.png", a.name)))?;

    std::fs::write(
        a.out.join(format!("{}-fabric.json", a.name)),
        serde_json::to_string_pretty(&serde_json::json!({
            "reference": a.name,
            "source": "OpenStreetMap contributors, ODbL",
            "centre": [lat, lon],
            "radius_m": a.radius,
            "fabric": fab,
        }))? + "\n",
    )?;
    println!("wrote {}", a.out.display());
    Ok(())
}

fn run_grow(cli: GrowArgs) -> Result<()> {
    let cfg: SpikeConfig = serde_json::from_slice(
        &std::fs::read(&cli.config).with_context(|| format!("reading {}", cli.config.display()))?,
    )
    .context("parsing spike config")?;
    anyhow::ensure!(!cfg.towns.is_empty(), "config declares no towns");
    std::fs::create_dir_all(&cli.out)?;

    let site_cell = resolve_site(&cli)?;
    let patch = patch::Patch::load(
        &cli.run_dir,
        site_cell,
        cfg.extent_m,
        cfg.river_min_strahler,
        cfg.lake_min_depth_m,
        cfg.river_width_coef,
    )
    .with_context(|| format!("loading patch from {}", cli.run_dir.display()))?;
    println!(
        "site cell {} (origin shifted {:.0} m to dry ground) — elevation {:.0} m, \
         slope {:.3}, domain {:.0} × {:.0} m",
        site_cell,
        patch.site_shift_m,
        patch.elevation([0.0, 0.0]),
        patch.slope([0.0, 0.0]),
        2.0 * cfg.extent_m,
        2.0 * cfg.extent_m
    );

    let t0 = Instant::now();
    let mut panels = Vec::new();
    let mut zooms = Vec::new();
    let mut reports = Vec::new();

    for (ti, town) in cfg.towns.iter().enumerate() {
        let tt = Instant::now();
        let mut g = graph::Graph::new(40.0);
        grow::seed_corridors(&mut g, &patch, &town.corridor_bearings_deg, &town.growth);
        if let Some(plat) = &town.growth.plat {
            grow::stamp_plat(&mut g, &patch, &town.growth, plat);
        }
        let seed = via_artifact::seed::derive(cfg.seed, "townfabric", ti as u64);
        // A plot is a household holding, so the town's extent follows from
        // plots, not from buildings (a plot carries a frontage building and
        // any number of rear outbuildings).
        let lot_area = town.parcels.frontage_module_m * town.parcels.plot_depth_m;
        let mut rng = grow::Rng::new(seed);
        let mut placed = 0u32;
        let mut spent = 0u32;
        let mut blocks: Vec<Vec<P2>> = Vec::new();
        let mut par = Vec::new();
        let mut bld = Vec::new();
        let mut radius = grow::radius_for(0, lot_area, &town.growth);
        // Streets are laid, the blocks they enclose fill with lots, and the
        // frontier moves out only as far as the households housed so far
        // require. Growth stops when the town's people are housed.
        while spent < town.max_steps {
            let n = town.attempts_per_round.min(town.max_steps - spent);
            let round_placed = grow::grow_round(&mut g, &patch, &town.growth, radius, n, &mut rng);
            placed += round_placed;
            spent += n;
            // Infill first; when the fabric inside the frontier can take no
            // more, the town extends. A settlement that cannot densify
            // spreads — it does not stop existing.
            if round_placed == 0 {
                radius *= 1.22;
                if radius > patch.extent_m {
                    break;
                }
                continue;
            }

            blocks = g
                .faces()
                .into_iter()
                .filter(|f| {
                    let a = polygon_area(f).abs();
                    a >= town.parcels.min_block_area_m2 && a <= town.parcels.max_block_area_m2
                })
                .collect();
            // Demand pressure drives the burgage cycle: high at the centre,
            // falling outward. Step 3 replaces this distance proxy with
            // network centrality, which is what actually concentrates
            // demand in a real town.
            let demand_scale = (radius * 0.6).max(60.0);
            // Streets earn their class from angular choice, and their
            // width follows from the class. This has to happen before
            // subdivision: the frontage line sits at the fronting street's
            // own half-width.
            hierarchy::classify(&mut g, &town.hierarchy);
            let hcfg = &town.hierarchy;
            let gref = &g;
            let (p2, b2) = parcels::subdivide(
                &blocks,
                &town.parcels,
                seed ^ 0x9E37,
                |p| (-len(p) / demand_scale).exp(),
                // The fronting street's own half-width.
                |p| {
                    gref.nearest_on_edges(p)
                        .map(|(e, _, _)| hcfg.width_of(gref.edges[e].class) * 0.5)
                        .unwrap_or(3.0)
                },
            );
            par = p2;
            bld = b2;
            // The frontier moves out as plots are laid — and this must be
            // updated before the stopping test, or the radius keeps the
            // value it was seeded with (it did, for two of three towns).
            radius = radius.max(grow::radius_for(
                par.len().max(1) as u32,
                lot_area,
                &town.growth,
            ));
            if par.len() as u32 >= town.households {
                break;
            }
        }
        // Same treatment the reference towns get: statistics describe the
        // built-up disc, while corridor tails running off to the domain
        // edge still participate in the graph. The disc is *observed* (the
        // radius holding 90% of buildings), not the radius the growth rule
        // believes in — those disagree badly, which is itself a finding.
        let mut br: Vec<f64> = bld.iter().map(|b| len(polygon_centroid(&b.poly))).collect();
        br.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let r90 = if br.is_empty() {
            radius
        } else {
            br[((br.len() as f64 - 1.0) * 0.9) as usize]
        };
        let fab = measure::measure(
            &town.name,
            &g,
            &blocks,
            &bld,
            Some(&par),
            Some(cfg.study_radius_m),
        );
        // Protocol parity: the reference towns are measured over the
        // mapped street network, which excludes footways and alleys. The
        // Lane class is this spike's equivalent, so the comparable
        // measurement drops it — from the graph *and* from the blocks it
        // encloses, exactly as the reference does.
        let g_streets = g.filtered(|c| c != graph::Class::Lane);
        let blocks_streets: Vec<Vec<P2>> = g_streets
            .faces()
            .into_iter()
            .filter(|f| {
                let a = polygon_area(f).abs();
                a >= town.parcels.min_block_area_m2 && a <= town.parcels.max_block_area_m2
            })
            .collect();
        let fab_streets = measure::measure(
            &format!("{} (streets only)", town.name),
            &g_streets,
            &blocks_streets,
            &bld,
            None,
            Some(cfg.study_radius_m),
        );
        println!(
            "  streets-only parity: M {:.3}  dead {:.2}  blocks {:>3}  setback {:.1} m  wall {:.2}  \
             compact {:.2}",
            fab_streets.meshedness,
            fab_streets.dead_end_share,
            fab_streets.blocks,
            fab_streets.bldg_street_dist_median_m,
            fab_streets.street_wall_share,
            fab_streets.block_compactness_median
        );
        println!(
            "{:<16} {:>5} nodes {:>5} edges  M {:.3}  deadend {:>4.1}%  blocks {:>4}  \
             bldgs {:>5}  wall {:>4.1}%  street-dist {:.1} m  backbone {:.2}  [{:.1}s]",
            town.name,
            fab.nodes,
            fab.edges,
            fab.meshedness,
            fab.dead_end_share * 100.0,
            fab.blocks,
            fab.buildings,
            fab.street_wall_share * 100.0,
            fab.bldg_street_dist_median_m,
            fab.backbone_concentration,
            tt.elapsed().as_secs_f64()
        );

        // Town panel.
        let side = (2.0 * cfg.extent_m / cfg.panel_m_per_px) as u32;
        let mut c = render::Canvas::new(side, side, 1.0 / cfg.panel_m_per_px, [0.0, 0.0]);
        c.draw_terrain(&patch);
        c.draw_blocks(&blocks);
        c.draw_streets(&g, |cl| town.hierarchy.width_of(cl));
        c.draw_parcels(&par);
        c.draw_buildings(&bld);
        // The zoom window sits on the densest part of the fabric.
        let focus = densest_point(&bld, cfg.zoom_half_m);
        c.mark_window(focus, cfg.zoom_half_m);
        c.label(10, 8, &town.name.to_uppercase(), 3);
        c.label(
            10,
            34,
            &format!(
                "{} BLOCKS  {} LOTS  {} BUILDINGS",
                fab.blocks,
                fab.parcels.unwrap_or(0),
                fab.buildings
            ),
            2,
        );
        c.label(
            10,
            52,
            &format!(
                "MESHEDNESS {:.2}  DEAD ENDS {:.0}%  WALL {:.0}%  BACKBONE {:.2}",
                fab.meshedness,
                fab.dead_end_share * 100.0,
                fab.street_wall_share * 100.0,
                fab.backbone_concentration
            ),
            2,
        );
        c.scale_bar(200.0, "200 M");
        c.img.save(
            cli.out
                .join(format!("{}-town.png", town.name.replace(' ', "-"))),
        )?;
        panels.push(c.img);

        // Zoom panel: individual lots and buildings.
        let zside = (2.0 * cfg.zoom_half_m / cfg.zoom_m_per_px) as u32;
        let mut z = render::Canvas::new(zside, zside, 1.0 / cfg.zoom_m_per_px, focus);
        z.draw_terrain(&patch);
        z.draw_blocks(&blocks);
        z.draw_streets(&g, |cl| town.hierarchy.width_of(cl));
        z.draw_parcels(&par);
        z.draw_buildings(&bld);
        z.label(
            10,
            8,
            &format!("{} - LOT LEVEL", town.name.to_uppercase()),
            3,
        );
        z.label(
            10,
            34,
            &format!(
                "MEDIAN LOT {:.0} M2  FRONTAGE {:.1} M  FOOTPRINT {:.0} M2",
                fab.parcel_area_median_m2.unwrap_or(f64::NAN),
                fab.frontage_median_m.unwrap_or(f64::NAN),
                fab.footprint_area_median_m2
            ),
            2,
        );
        z.scale_bar(50.0, "50 M");
        z.img.save(
            cli.out
                .join(format!("{}-lots.png", town.name.replace(' ', "-"))),
        )?;
        zooms.push(z.img);

        reports.push(TownReport {
            name: town.name.clone(),
            growth: town.growth.clone(),
            parcels: town.parcels.clone(),
            hierarchy: town.hierarchy.clone(),
            households: town.households,
            sites_placed: placed,
            built_radius_m: radius,
            observed_r90_m: r90,
            growth_attempts: spent,
            fabric: fab,
            fabric_streets_only: fab_streets,
        });
    }

    render::compose(
        &panels,
        panels.len() as u32,
        14,
        "SPIKE: TOWN FABRIC - ONE SITE, ONE MECHANISM SET, ERA PARAMETERS ONLY",
    )
    .save(cli.out.join("fabric.png"))?;
    render::compose(
        &zooms,
        zooms.len() as u32,
        14,
        "LOT LEVEL: PARCELS AND BUILDING FOOTPRINTS (RED WINDOW ON THE TOWN PANELS)",
    )
    .save(cli.out.join("lots.png"))?;

    let record = serde_json::json!({
        "spike": "townfabric",
        "status": "research spike — not a stage, not gated, nothing adopted",
        "run_dir": cli.run_dir.display().to_string(),
        "site_cell": site_cell,
        "config": cfg,
        "towns": reports,
        "wall_s": t0.elapsed().as_secs_f64(),
    });
    std::fs::write(
        cli.out.join("townfabric.json"),
        serde_json::to_string_pretty(&record)? + "\n",
    )?;
    println!(
        "wrote {} [{:.1}s]",
        cli.out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Centre of the densest built cluster, for the zoom window.
fn densest_point(buildings: &[parcels::Building], half: f64) -> P2 {
    if buildings.is_empty() {
        return [0.0, 0.0];
    }
    let cs: Vec<P2> = buildings
        .iter()
        .map(|b| polygon_centroid(&b.poly))
        .collect();
    let mut best = ([0.0, 0.0], -1.0);
    for &c in &cs {
        let n = cs
            .iter()
            .filter(|&&q| (q[0] - c[0]).abs() <= half && (q[1] - c[1]).abs() <= half)
            .count() as f64;
        if n > best.1 {
            best = (c, n);
        }
    }
    best.0
}
