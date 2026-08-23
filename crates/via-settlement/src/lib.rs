//! Stage 4 — settlement systems (ADR 0013).
//!
//! Harris–Wilson allocation over the ADR 0012 corridor cost field. The
//! population budget is Tier-2 forcing (Decision 3): the engine
//! distributes it and cannot supply it, so every size here is a share
//! of a declared total, and no claim rests on that total's level.

pub mod cost;
pub mod graph;
pub mod hw;

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use via_artifact::Raster;
use via_corridors::trunk::{Junction, SiteClass, TrunkEdge, TrunkNode};

use cost::CostField;
use hw::HwParams;

fn bad(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementConfig {
    pub label: String,
    pub corridors_label: String,
    pub suitability_label: String,

    /// Returns to scale and distance decay. Calibrated, not declared
    /// (ADR 0013 D1) — these defaults are a starting point for the
    /// sweep, not a claim.
    pub alpha: f64,
    /// Per hour of corridor time.
    pub beta: f64,
    /// Minimum-size term, as a multiple of the mean origin mass
    /// `ΣO/M`. Expressed relatively because the absolute value has no
    /// meaning without the normalisation, and because it controls both
    /// the floor's share of the budget and the flow's slowest mode.
    pub delta_rel: f64,
    /// Intrazonal cost `c_ii = gamma × min_{k≠i} c_ik` (ADR 0013 D4).
    pub gamma_intrazonal: f64,
    /// Damping for the equilibrium solver.
    pub relaxation: f64,
    pub max_iters: u32,
    pub tol: f64,
    /// Components smaller than this are excluded and counted.
    pub min_component: usize,

    /// Declared origin-mass weighting (ADR 0013 D4, `heuristic`).
    pub w_freshwater: f64,
    pub w_workable: f64,
    pub w_coast: f64,
    pub freshwater_scale_m: f64,
    pub coast_scale_m: f64,
    pub slope_ref: f64,
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            label: "site".into(),
            corridors_label: "site".into(),
            suitability_label: "site".into(),
            alpha: 1.05,
            beta: 0.15,
            delta_rel: 0.05,
            gamma_intrazonal: 0.5,
            relaxation: 0.5,
            max_iters: 20_000,
            tol: 1e-10,
            min_component: 2,
            w_freshwater: 0.5,
            w_workable: 0.3,
            w_coast: 0.2,
            freshwater_scale_m: 5_000.0,
            coast_scale_m: 50_000.0,
            slope_ref: 0.15,
        }
    }
}

pub fn validate_label(label: &str) -> io::Result<()> {
    if label.is_empty()
        || !label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(bad(format!(
            "label {label:?} must be non-empty ASCII lowercase, digits or underscore"
        )));
    }
    Ok(())
}

pub fn summary_filename(label: &str) -> String {
    format!("settlement.{label}.json")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settlement {
    /// Vertex id within the stage's own cost field.
    pub vertex: u32,
    pub cell: u32,
    pub x: u32,
    pub y: u32,
    /// Trunk-node class where the vertex is one; junction-only
    /// vertices carry None, which is a real distinction: they are
    /// corridor forks, not terrain gateways.
    pub class: Option<SiteClass>,
    pub component: i32,
    /// Share of the declared budget.
    pub share: f64,
    /// Revenue at equilibrium.
    pub demand: f64,
    /// True where the share sits at the `δ/κ` floor within tolerance.
    pub at_floor: bool,
    /// Corridor attachments: (trunk edge id, hours on that edge).
    pub attachments: Vec<(u32, f64)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentReport {
    pub id: i32,
    pub vertices: usize,
    pub solved: bool,
    pub converged: bool,
    pub iters: u32,
    pub residual: f64,
    pub kappa: f64,
    pub floor_share: f64,
    pub above_floor: usize,
}

#[derive(Clone, Debug)]
pub struct SettlementOutput {
    pub w: u32,
    pub h: u32,
    pub settlements: Vec<Settlement>,
    pub components: Vec<ComponentReport>,
    pub excluded_vertices: usize,
    pub vertices: usize,
    pub arcs: usize,
    pub node_cells: usize,
    pub junction_only_cells: usize,
}

#[derive(Deserialize)]
struct TrunkJson {
    nodes: Vec<TrunkNode>,
    edges: Vec<TrunkEdge>,
    junctions: Vec<Junction>,
}

#[derive(Deserialize)]
struct CorridorsJson {
    trunk: TrunkJson,
}

pub struct Inputs {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub nodes: Vec<TrunkNode>,
    pub edges: Vec<TrunkEdge>,
    pub junctions: Vec<Junction>,
    pub freshwater_dist: Vec<f32>,
    pub coast_dist: Vec<f32>,
    pub slope: Vec<f32>,
}

fn suit_raster(run_dir: &Path, label: &str, name: &str) -> io::Result<Raster<f32>> {
    Raster::read_file(&run_dir.join(format!("suitability.{label}.{name}.vrast")))
}

pub fn load_inputs(run_dir: &Path, cfg: &SettlementConfig) -> io::Result<Inputs> {
    validate_label(&cfg.label)?;
    let heights: Raster<i32> = Raster::read_file(&run_dir.join("heights_cm.vrast"))?;
    let path = run_dir.join(format!("corridors.{}.json", cfg.corridors_label));
    let text = std::fs::read_to_string(&path).map_err(|e| {
        bad(format!(
            "cannot read {}: {e}. Run the corridors stage first.",
            path.display()
        ))
    })?;
    let cj: CorridorsJson = serde_json::from_str(&text).map_err(|e| {
        bad(format!(
            "{} is not a corridors summary this stage understands: {e}. \
             If it predates the ADR 0012 D5 amendment it will lack \
             cum_hours_ab/cum_hours_ba, which this stage requires.",
            path.display()
        ))
    })?;
    let sl = &cfg.suitability_label;
    let fresh = suit_raster(run_dir, sl, "freshwater_dist")?;
    let coast = suit_raster(run_dir, sl, "coast_dist")?;
    let slope = suit_raster(run_dir, sl, "slope")?;
    for (n, r) in [
        ("freshwater_dist", &fresh),
        ("coast_dist", &coast),
        ("slope", &slope),
    ] {
        if r.width != heights.width || r.height != heights.height {
            return Err(bad(format!(
                "{n}: grid {}x{} does not match heights_cm {}x{}",
                r.width, r.height, heights.width, heights.height
            )));
        }
    }
    Ok(Inputs {
        w: heights.width,
        h: heights.height,
        dx: heights.cell_size_cm as f64 / 100.0,
        nodes: cj.trunk.nodes,
        edges: cj.trunk.edges,
        junctions: cj.trunk.junctions,
        freshwater_dist: fresh.data,
        coast_dist: coast.data,
        slope: slope.data,
    })
}

/// Declared origin-mass weighting at a cell (ADR 0013 D4). Labelled
/// `heuristic`: the components are cited fields, the combination is
/// via's and is exposed in config for sweeping. No composite score is
/// emitted — this value is an input to the engine, not an output.
fn origin_mass(inp: &Inputs, cfg: &SettlementConfig, cell: usize) -> f64 {
    let sentinel = |v: f32| !v.is_finite() || v >= f32::MAX * 0.5;
    let fresh = inp.freshwater_dist[cell];
    let coast = inp.coast_dist[cell];
    let slope = inp.slope[cell];
    let f = if sentinel(fresh) {
        0.0
    } else {
        (-(fresh as f64) / cfg.freshwater_scale_m).exp()
    };
    let c = if sentinel(coast) {
        0.0
    } else {
        (-(coast as f64) / cfg.coast_scale_m).exp()
    };
    let s = if sentinel(slope) {
        0.0
    } else {
        (1.0 - (slope as f64 / cfg.slope_ref).clamp(0.0, 1.0)).max(0.0)
    };
    (cfg.w_freshwater * f + cfg.w_workable * s + cfg.w_coast * c).max(0.0)
}

pub fn compute(inp: &Inputs, cfg: &SettlementConfig) -> io::Result<SettlementOutput> {
    if cfg.delta_rel <= 0.0 {
        return Err(bad(
            "delta_rel must be > 0: at δ = 0 the Gibbs measure is unnormalisable \
             and abandoned sites become absorbing (ADR 0013 Decision 2)"
                .into(),
        ));
    }
    let cf = CostField::build(&inp.nodes, &inp.edges, &inp.junctions);
    let comp = graph::components(&cf);
    let arcs: usize = cf.arcs.iter().map(|a| a.len()).sum();
    let node_cells = cf.node_of_vertex.iter().filter(|v| v.is_some()).count();

    // Raw origin mass per vertex, then per-component budget shares
    // proportional to raw mass — a declared split, recorded.
    let raw: Vec<f64> = cf
        .cells
        .iter()
        .map(|&c| origin_mass(inp, cfg, c as usize))
        .collect();
    let raw_total: f64 = raw.iter().sum();
    if !raw_total.is_finite() || raw_total <= 0.0 {
        return Err(bad(
            "origin mass is zero at every trunk vertex: the weighting or its \
             scales are degenerate for this world"
                .into(),
        ));
    }

    let ncomp = comp.iter().copied().max().unwrap_or(-1) + 1;
    let mut settlements: Vec<Settlement> = Vec::new();
    let mut reports: Vec<ComponentReport> = Vec::new();
    let mut excluded = 0usize;

    for cid in 0..ncomp {
        let verts: Vec<u32> = (0..cf.len() as u32)
            .filter(|&v| comp[v as usize] == cid)
            .collect();
        let cmass: f64 = verts.iter().map(|&v| raw[v as usize]).sum();
        if verts.len() < cfg.min_component || !cmass.is_finite() || cmass <= 0.0 {
            excluded += verts.len();
            reports.push(ComponentReport {
                id: cid,
                vertices: verts.len(),
                solved: false,
                converged: false,
                iters: 0,
                residual: f64::NAN,
                kappa: f64::NAN,
                floor_share: f64::NAN,
                above_floor: 0,
            });
            continue;
        }
        let k_total = cmass / raw_total;
        let o: Vec<f64> = verts.iter().map(|&v| raw[v as usize] / raw_total).collect();
        let (kern, _c_ii) = graph::kernel_for(&cf, &verts, cfg.beta, cfg.gamma_intrazonal);
        let o_sum: f64 = o.iter().sum();
        let delta = cfg.delta_rel * o_sum / verts.len() as f64;
        let p = HwParams {
            alpha: cfg.alpha,
            beta: cfg.beta,
            epsilon: 1.0,
            dt: 1.0,
            delta,
            k_total,
            max_iters: cfg.max_iters,
            tol: cfg.tol,
        };
        let sol = hw::solve_equilibrium(&o, &kern, &p, cfg.relaxation);
        let floor = delta / sol.kappa;
        let mut above = 0usize;
        for (li, &v) in verts.iter().enumerate() {
            let at_floor = (sol.w[li] - floor) / floor < 1e-6;
            if !at_floor {
                above += 1;
            }
            let cell = cf.cells[v as usize];
            settlements.push(Settlement {
                vertex: v,
                cell,
                x: cell % inp.w,
                y: cell / inp.w,
                class: cf.node_of_vertex[v as usize].map(|ni| inp.nodes[ni as usize].class),
                component: cid,
                share: sol.w[li],
                demand: sol.d[li],
                at_floor,
                attachments: Vec::new(),
            });
        }
        reports.push(ComponentReport {
            id: cid,
            vertices: verts.len(),
            solved: true,
            converged: sol.converged,
            iters: sol.iters,
            residual: sol.residual,
            kappa: sol.kappa,
            floor_share: floor,
            above_floor: above,
        });
    }

    // Attachments: every trunk edge whose path touches the settlement
    // cell, with the recorded time of that edge. Composed from the
    // records, never re-derived.
    let mut by_cell: BTreeMap<u32, Vec<(u32, f64)>> = BTreeMap::new();
    for (ei, e) in inp.edges.iter().enumerate() {
        for &(c, _) in &e.path {
            by_cell.entry(c).or_default().push((ei as u32, e.hours_ab));
        }
    }
    for s in settlements.iter_mut() {
        if let Some(list) = by_cell.get(&s.cell) {
            let mut v = list.clone();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v.dedup_by(|a, b| a.0 == b.0);
            s.attachments = v;
        }
    }
    settlements.sort_by(|a, b| a.cell.cmp(&b.cell));

    Ok(SettlementOutput {
        w: inp.w,
        h: inp.h,
        settlements,
        components: reports,
        excluded_vertices: excluded,
        vertices: cf.len(),
        arcs,
        node_cells,
        junction_only_cells: cf.len() - node_cells,
    })
}

/// Rung 2 of the ADR 0013 Decision 7 null ladder: **suitability alone,
/// no interaction.**
///
/// The generator and this null consume the *same* origin-mass field
/// and the *same* seed set; the only difference is that the null skips
/// the spatial-interaction term entirely and allocates the budget in
/// proportion to `O_i`. That is what makes it the fair adversary — a
/// difference between the two can only come from `c_ij`.
///
/// It is deliberately not a point-process simulation: via's settlement
/// *positions* are fixed by the corridor network, so the thing that
/// varies between generator and null is which vertices carry mass, and
/// that is what this compares.
pub fn null_suitability_only(inp: &Inputs, cfg: &SettlementConfig) -> io::Result<Vec<Settlement>> {
    let cf = CostField::build(&inp.nodes, &inp.edges, &inp.junctions);
    let comp = graph::components(&cf);
    let raw: Vec<f64> = cf
        .cells
        .iter()
        .map(|&c| origin_mass(inp, cfg, c as usize))
        .collect();
    let raw_total: f64 = raw.iter().sum();
    if !raw_total.is_finite() || raw_total <= 0.0 {
        return Err(bad("origin mass is zero at every trunk vertex".into()));
    }
    let ncomp = comp.iter().copied().max().unwrap_or(-1) + 1;
    let mut out = Vec::new();
    for cid in 0..ncomp {
        let verts: Vec<u32> = (0..cf.len() as u32)
            .filter(|&v| comp[v as usize] == cid)
            .collect();
        let cmass: f64 = verts.iter().map(|&v| raw[v as usize]).sum();
        if verts.len() < cfg.min_component || !cmass.is_finite() || cmass <= 0.0 {
            continue;
        }
        for &v in &verts {
            let cell = cf.cells[v as usize];
            out.push(Settlement {
                vertex: v,
                cell,
                x: cell % inp.w,
                y: cell / inp.w,
                class: cf.node_of_vertex[v as usize].map(|ni| inp.nodes[ni as usize].class),
                component: cid,
                share: raw[v as usize] / raw_total,
                demand: f64::NAN,
                at_floor: false,
                attachments: Vec::new(),
            });
        }
    }
    out.sort_by(|a, b| a.cell.cmp(&b.cell));
    Ok(out)
}
