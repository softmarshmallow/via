//! via-bench CLI: measure reference towns and null models under an
//! identified protocol, with full provenance on every number (ADR 0008
//! D11: protocol identifier, code revision, extract hash, seed set).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use via_bench::graph::Graph;
use via_bench::osm::StreetSet;
use via_bench::{ensemble, measure, null, osm, render, Protocol};

#[derive(Parser)]
#[command(
    name = "via-bench",
    about = "benchmark measurement (ADR 0009 D4, ADR 0010)"
)]
enum Cmd {
    /// Measure a reference town from an Overpass extract: both 0012 P4
    /// street sets, rendered panels, provenance-carrying JSON.
    Reference(RefArgs),
    /// Generate and measure a null model ensemble (ADR 0008 D9).
    Null(NullArgs),
}

#[derive(clap::Args)]
struct RefArgs {
    /// Overpass JSON extract (fetched by analysis/fetch_extract.py).
    extract: PathBuf,
    /// Study centre, "lat,lon" — must match the manifest record.
    #[arg(long)]
    centre: String,
    /// Town name for the report.
    #[arg(long)]
    name: String,
    /// Reference class (0012 §5.4), recorded verbatim.
    #[arg(long = "town-class")]
    town_class: String,
    #[arg(long)]
    out: PathBuf,
    /// Study radius in metres (protocol P1).
    #[arg(long, default_value_t = 300.0)]
    radius: f64,
    /// Protocol identifier; pilot-0 until the freeze (ADR 0010 D3).
    #[arg(long, default_value = "pilot-0")]
    protocol_id: String,
    /// Metres per pixel in the town panel.
    #[arg(long, default_value_t = 0.85)]
    panel_m_per_px: f64,
    #[arg(long, default_value_t = 0.2)]
    zoom_m_per_px: f64,
    #[arg(long, default_value_t = 110.0)]
    zoom_half_m: f64,
}

#[derive(clap::Args)]
struct NullArgs {
    /// grid | random | dla
    #[arg(long)]
    model: String,
    #[arg(long)]
    out: PathBuf,
    /// Seeds in the ensemble (ADR 0008 D5: at least five).
    #[arg(long, default_value_t = 5)]
    seeds: u64,
    #[arg(long, default_value_t = 42)]
    global_seed: u64,
    #[arg(long, default_value_t = 300.0)]
    radius: f64,
    /// grid: lattice spacing (m) — matches intersection density.
    #[arg(long, default_value_t = 100.0)]
    spacing: f64,
    /// random: node count — matches the reference class.
    #[arg(long, default_value_t = 300)]
    nodes: usize,
    /// random: target edge/node ratio — matches density.
    #[arg(long, default_value_t = 1.3)]
    target_enr: f64,
    /// dla: particle count.
    #[arg(long, default_value_t = 400)]
    particles: usize,
    /// Protocol identifier stamped into the provenance record.
    #[arg(long, default_value = "v1")]
    protocol_id: String,
}

fn main() -> Result<()> {
    match Cmd::parse() {
        Cmd::Reference(a) => run_reference(a),
        Cmd::Null(a) => run_null(a),
    }
}

fn parse_centre(s: &str) -> Result<(f64, f64)> {
    let mut it = s.split(',');
    let lat: f64 = it.next().context("centre needs lat,lon")?.trim().parse()?;
    let lon: f64 = it.next().context("centre needs lat,lon")?.trim().parse()?;
    Ok((lat, lon))
}

fn run_reference(a: RefArgs) -> Result<()> {
    let (lat, lon) = parse_centre(&a.centre)?;
    let protocol = Protocol {
        id: a.protocol_id.clone(),
        radius_m: a.radius,
        ..Protocol::default()
    };
    std::fs::create_dir_all(&a.out)?;
    let extract_hash = blake3::hash(&std::fs::read(&a.extract)?)
        .to_hex()
        .to_string();

    let mut fabrics = serde_json::Map::new();
    let mut graphs = serde_json::Map::new();
    for set in [StreetSet::Carriageway, StreetSet::AllWays] {
        let r = osm::load(&a.extract, lat, lon, &protocol, set)?;
        let fab = measure::measure(
            &format!("{}-{}", a.name, set.label()),
            &r.graph,
            &r.blocks,
            &r.buildings,
            None,
            Some(protocol.radius_m),
            protocol.min_footprints,
        );
        println!(
            "{:<12} {:<12} {:>5} nodes {:>5} edges  M {:.3}  deadend {:>4.1}%  blocks {:>4}  \
             bldgs {:>5}  H_o {:.3}  bc_gini {:.3}",
            a.name,
            set.label(),
            fab.nodes,
            fab.edges,
            fab.meshedness,
            fab.dead_end_share * 100.0,
            fab.blocks,
            fab.buildings,
            fab.orientation_entropy,
            fab.bc_gini,
        );
        graphs.insert(
            set.label().to_string(),
            measure::export_simple(&r.graph, Some(protocol.radius_m)),
        );
        fabrics.insert(set.label().to_string(), serde_json::to_value(&fab)?);

        if set == StreetSet::Carriageway {
            render_panels(&a, &r)?;
        }
    }

    std::fs::write(
        a.out.join(format!("{}-fabric.json", a.name)),
        serde_json::to_string_pretty(&serde_json::json!({
            "town": a.name,
            "class": a.town_class,
            "source": "OpenStreetMap contributors, ODbL",
            "centre": [lat, lon],
            "protocol": protocol,
            "code_revision": via_bench::code_revision(),
            "extract": {
                "file": a.extract.display().to_string(),
                "blake3": extract_hash,
            },
            "fabric": fabrics,
        }))? + "\n",
    )?;
    std::fs::write(
        a.out.join(format!("{}-graph.json", a.name)),
        serde_json::to_string(&serde_json::Value::Object(graphs))? + "\n",
    )?;
    println!("wrote {}", a.out.display());
    Ok(())
}

fn render_panels(a: &RefArgs, r: &osm::Reference) -> Result<()> {
    let side = (2.0 * a.radius / a.panel_m_per_px) as u32;
    let mut c = render::Canvas::new(side, side, 1.0 / a.panel_m_per_px, [0.0, 0.0]);
    c.draw_blocks(&r.blocks);
    c.draw_streets(&r.graph);
    c.draw_buildings(&r.buildings);
    let focus = render::densest_point(&r.buildings, a.zoom_half_m);
    c.mark_window(focus, a.zoom_half_m);
    c.label(10, 8, &format!("{} (REAL)", a.name.to_uppercase()), 3);
    c.scale_bar(200.0, "200 M");
    c.img.save(a.out.join(format!("{}-town.png", a.name)))?;

    let zside = (2.0 * a.zoom_half_m / a.zoom_m_per_px) as u32;
    let mut z = render::Canvas::new(zside, zside, 1.0 / a.zoom_m_per_px, focus);
    z.draw_blocks(&r.blocks);
    z.draw_streets(&r.graph);
    z.draw_buildings(&r.buildings);
    z.label(
        10,
        8,
        &format!("{} (REAL) - LOT LEVEL", a.name.to_uppercase()),
        3,
    );
    z.scale_bar(50.0, "50 M");
    z.img.save(a.out.join(format!("{}-lots.png", a.name)))?;
    Ok(())
}

fn run_null(a: NullArgs) -> Result<()> {
    std::fs::create_dir_all(&a.out)?;
    // The full protocol in force — recorded in the provenance block below,
    // never left implicit (ADR 0008 D11).
    let protocol = Protocol {
        id: a.protocol_id.clone(),
        radius_m: a.radius,
        ..Protocol::default()
    };
    let extent = protocol.radius_m * protocol.buffer;
    // Every generator parameter that determines the numbers, recorded so a
    // null artifact is reproducible from its own record.
    const DLA_ATTACH_M: f64 = 30.0;
    const DLA_STEP_M: f64 = 20.0;
    let params = match a.model.as_str() {
        "grid" => serde_json::json!({"spacing_m": a.spacing, "extent_m": extent}),
        "random" => serde_json::json!({
            "nodes": a.nodes, "target_edge_node_ratio": a.target_enr, "extent_m": extent,
        }),
        "dla" => serde_json::json!({
            "particles": a.particles, "attach_m": DLA_ATTACH_M,
            "step_m": DLA_STEP_M, "extent_m": extent,
        }),
        other => anyhow::bail!("unknown null model {other}"),
    };
    let stage = format!("bench-null-{}", a.model);
    let mut runs: Vec<serde_json::Value> = Vec::new();
    for i in 0..a.seeds {
        let seed_v = via_artifact::seed::derive(a.global_seed, &stage, i);
        let g: Graph = match a.model.as_str() {
            "grid" => null::grid(a.spacing, extent),
            "random" => null::random_planar(a.nodes, a.target_enr, extent, seed_v),
            "dla" => null::dla(a.particles, DLA_ATTACH_M, DLA_STEP_M, extent, seed_v),
            other => anyhow::bail!("unknown null model {other}"),
        };
        let blocks: Vec<_> = g
            .faces()
            .into_iter()
            .filter(|f| {
                let ar = via_bench::geom::polygon_area(f).abs();
                (protocol.block_area_min_m2..=protocol.block_area_max_m2).contains(&ar)
            })
            .collect();
        let fab = measure::measure(
            &format!("{}-s{}", a.model, i),
            &g,
            &blocks,
            &[],
            None,
            Some(protocol.radius_m),
            0,
        );
        println!(
            "null {:<7} seed {:>2}  {:>5} nodes  M {:.3}  deadend {:>4.1}%",
            a.model,
            i,
            fab.nodes,
            fab.meshedness,
            fab.dead_end_share * 100.0
        );
        if i == 0 {
            let side = (2.0 * a.radius / 0.85) as u32;
            let mut c = render::Canvas::new(side, side, 1.0 / 0.85, [0.0, 0.0]);
            c.draw_streets(&g);
            c.label(10, 8, &format!("NULL: {}", a.model.to_uppercase()), 3);
            c.scale_bar(200.0, "200 M");
            c.img.save(a.out.join(format!("null-{}.png", a.model)))?;
        }
        runs.push(serde_json::to_value(&fab)?);
    }
    let agg = ensemble::aggregate(&runs);
    std::fs::write(
        a.out.join(format!("null-{}.json", a.model)),
        serde_json::to_string_pretty(&serde_json::json!({
            "model": a.model,
            "params": params,
            "seeds": a.seeds,
            "global_seed": a.global_seed,
            "seed_stage": stage,
            "code_revision": via_bench::code_revision(),
            "protocol": protocol,
            "per_seed": runs,
            "ensemble": agg,
        }))? + "\n",
    )?;
    println!("wrote {}", a.out.display());
    Ok(())
}
