//! The build through `via-graph` (`graph.md` 1 and 2): fields as `field` nodes, keyed by the
//! statements they are made from and by the fields they read, so an unchanged rebuild runs no
//! field and a change re-runs only the fields that read it.
//!
//! This is the first cut of the wiring. A field is one node over the whole grid
//! (`field:<name>/v:build/w:0.0.<nx>.<ny>`), not one per chunk; shapes, plans, producers,
//! heights and checks are computed in memory after the field nodes, reading their values. The
//! families `shape`, `instance`, `rule`, `held`, `footprint` and `height` are not nodes yet.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::Diag;
use via_doc::World;
use via_graph::{Dec, Def, Defined, Digest, Enc, Inputs, Known, NodeId, Output, Plan, Store, Val};
use via_raster::{Raster, Window};

use crate::build::{feature_deps, finish, prepare, statements, Build, Options, Prepared};
use crate::decl::Selector;
use crate::resolve::{Fields, Resolved, View};

/// A field node's statements, as (id, digest), and the fields it reads.
type FieldDef = (Vec<(String, Digest)>, BTreeSet<String>);

/// The model's node kinds for the graph.
pub struct ModelPlan {
    p: Prepared,
    nx: u64,
    ny: u64,
    /// Per field: statements (id, digest) and the fields it reads.
    defs: BTreeMap<String, FieldDef>,
}

fn field_id(name: &str, nx: u64, ny: u64) -> NodeId {
    NodeId::of("field", name).view("build").window(0, 0, nx, ny)
}

fn encode(r: Option<&Resolved>) -> Vec<u8> {
    let mut e = Enc::new();
    e.str("via-field-1");
    match r {
        None => {
            e.byte(2);
        }
        Some(r) => {
            match &r.scalar {
                Some(s) => {
                    e.byte(0).len(s.values().len());
                    for v in s.values() {
                        e.u64(u64::from(v.to_bits()));
                    }
                }
                None => {
                    e.byte(1).len(r.weights.len());
                    for (m, s) in &r.weights {
                        e.str(m).len(s.values().len());
                        for v in s.values() {
                            e.u64(u64::from(v.to_bits()));
                        }
                    }
                }
            }
            for set in [&r.writers, &r.reads] {
                e.len(set.len());
                for x in set {
                    e.str(x);
                }
            }
        }
    }
    e.finish()
}

fn decode(bytes: &[u8], window: Window) -> Option<Option<Resolved>> {
    let mut d = Dec::new(bytes);
    if d.str()? != "via-field-1" {
        return None;
    }
    let plane = |d: &mut Dec| -> Option<Raster<f32>> {
        let n = d.count()?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(f32::from_bits(u32::try_from(d.u64()?).ok()?));
        }
        Raster::from_values(window, v)
    };
    let (scalar, weights) = match d.byte()? {
        2 => return Some(None),
        0 => (Some(plane(&mut d)?), BTreeMap::new()),
        1 => {
            let mut w = BTreeMap::new();
            for _ in 0..d.count()? {
                let m = d.str()?;
                w.insert(m, plane(&mut d)?);
            }
            (None, w)
        }
        _ => return None,
    };
    let mut sets = [BTreeSet::new(), BTreeSet::new()];
    for s in &mut sets {
        for _ in 0..d.count()? {
            s.insert(d.str()?);
        }
    }
    let [writers, reads] = sets;
    d.done().then_some(Some(Resolved {
        scalar,
        weights,
        writers,
        reads,
    }))
}

impl ModelPlan {
    /// The plan of a world: each field's statements and reads, worked out before anything runs.
    fn new(world: &World, p: Prepared) -> Option<ModelPlan> {
        let grid = p.decls.grid?;
        let stmts = statements(world);
        let digest_of = |s: &str| -> Option<Digest> {
            if let Some(asset) = s.strip_prefix("asset:") {
                let facts = p.pack.assets.get(asset)?.facts();
                return Some(Digest::of(format!("{facts:?}").as_bytes()));
            }
            stmts
                .get(s)
                .map(|keys| Digest::of(format!("{s}{keys:?}").as_bytes()))
        };
        let fields = Fields::new(&p.decls, &p.placed, grid);
        let resolve_sel = |sel: &str| -> Vec<String> {
            p.decls
                .fields
                .values()
                .filter(|f| match sel.strip_prefix("tag:") {
                    Some(t) => f.tags.iter().any(|x| x == t),
                    None => f.name == sel,
                })
                .map(|f| f.name.clone())
                .collect()
        };
        let reads_of = |fid: Option<via_expr::FormulaId>, out: &mut BTreeSet<String>| {
            if let Some(prog) = fid.and_then(|f| p.decls.formulas.programs.get(&f)) {
                for (class, sel) in &prog.reads.selectors {
                    if *class == via_expr::Class::Field {
                        out.extend(resolve_sel(sel));
                    }
                }
            }
        };
        let mut defs = BTreeMap::new();
        for (name, fd) in &p.decls.fields {
            let mut deps: BTreeSet<String> =
                BTreeSet::from([format!("field.{name}"), "world".into()]);
            let mut reads = BTreeSet::new();
            reads_of(fd.base, &mut reads);
            for r in &fd.rows {
                reads_of(r.value, &mut reads);
                reads_of(r.mask, &mut reads);
            }
            for w in &fields.writers {
                let k = &p.decls.kinds[&w.kind];
                let matched: Vec<&crate::decl::Effect> = k
                    .effects
                    .values()
                    .filter(|e| match &e.selector {
                        Selector::Role(r) => fd.role == Some(*r),
                        Selector::Tag(t) => fd.tags.iter().any(|x| x == t),
                    })
                    .collect();
                if matched.is_empty() {
                    continue;
                }
                deps.insert(format!("kind.{}", w.kind));
                let base = w.name.trim_end_matches(".ground");
                deps.extend(feature_deps(&p.decls, base, &mut BTreeSet::new()));
                for e in matched {
                    reads_of(e.value, &mut reads);
                    reads_of(e.mask, &mut reads);
                    reads_of(e.max_change, &mut reads);
                }
            }
            let statements: Vec<(String, Digest)> = deps
                .iter()
                .filter_map(|s| digest_of(s).map(|d| (s.clone(), d)))
                .collect();
            defs.insert(name.clone(), (statements, reads));
        }
        // Fields in a loop of reads keep no edges: each computes alone and reports the cycle.
        let names: Vec<String> = defs.keys().cloned().collect();
        let reach = |from: &str| -> BTreeSet<String> {
            let mut seen = BTreeSet::new();
            let mut stack = vec![from.to_string()];
            while let Some(n) = stack.pop() {
                for r in defs.get(&n).map(|d| d.1.clone()).unwrap_or_default() {
                    if seen.insert(r.clone()) {
                        stack.push(r);
                    }
                }
            }
            seen
        };
        let cyclic: BTreeSet<String> = names
            .iter()
            .filter(|n| reach(n).contains(*n))
            .cloned()
            .collect();
        for n in &cyclic {
            if let Some(d) = defs.get_mut(n) {
                d.1.clear();
            }
        }
        drop(fields);
        Some(ModelPlan {
            nx: grid.nx,
            ny: grid.ny,
            p,
            defs,
        })
    }

    /// Every field node.
    pub fn roots(&self) -> Vec<NodeId> {
        self.defs
            .keys()
            .map(|n| field_id(n, self.nx, self.ny))
            .collect()
    }
}

impl Plan for ModelPlan {
    fn define(&self, id: &NodeId, _known: &Known<'_>) -> Defined {
        let name = id.to_string();
        let name = name
            .trim_start_matches("field:")
            .split("/v:")
            .next()
            .unwrap_or("")
            .to_string();
        let Some((statements, reads)) = self.defs.get(&name) else {
            return Defined::Def(Def::default());
        };
        Defined::Def(Def {
            subject: Some(via_diag::Subject::entity("field", name.clone())),
            statements: statements.clone(),
            params: vec![
                ("view".into(), "build".into()),
                ("window".into(), format!("0.0.{}.{}", self.nx, self.ny)),
            ],
            edges: reads
                .iter()
                .map(|r| (format!("base reads {r}"), field_id(r, self.nx, self.ny)))
                .collect(),
            ..Def::default()
        })
    }

    fn compute(&self, id: &NodeId, _def: &Def, inputs: &Inputs<'_>) -> Output {
        let name = id.to_string();
        let name = name
            .trim_start_matches("field:")
            .split("/v:")
            .next()
            .unwrap_or("")
            .to_string();
        let Some(grid) = self.p.decls.grid else {
            return Output::of(Val::plain(encode(None)));
        };
        let fields = Fields::new(&self.p.decls, &self.p.placed, grid);
        let window = Window::whole(&grid);
        for (_, node, val) in inputs.all() {
            let other = node.to_string();
            let other = other
                .trim_start_matches("field:")
                .split("/v:")
                .next()
                .unwrap_or("")
                .to_string();
            if let Some(r) = decode(&val.bytes, window) {
                fields.seed(&other, View::Build, r);
            }
        }
        let r = fields.get(&name, View::Build);
        let diags: Vec<Diag> = fields.diags.borrow().clone();
        Output::of(Val {
            bytes: std::sync::Arc::new(encode(r.as_deref())),
            diags: std::sync::Arc::new(diags),
        })
    }
}

/// A build through the graph: field nodes run or are reused by key, the rest computed from
/// their values. The run says which nodes ran and why (`graph.md` 6.3).
pub fn build_graph(
    world: &World,
    options: &Options,
    baseline: Option<&Build>,
    store: &Store,
    graph: &via_graph::Options,
) -> (Build, Option<via_graph::Run>) {
    let p = prepare(world, options);
    let Some(grid) = p.decls.grid else {
        return (
            finish(world, p, BTreeMap::new(), Vec::new(), baseline),
            None,
        );
    };
    let Some(plan) = ModelPlan::new(world, p) else {
        return (Build::default(), None);
    };
    let roots = plan.roots();
    let run = via_graph::run(&plan, store, graph, &roots);
    let window = Window::whole(&grid);
    let mut seeds = BTreeMap::new();
    for (name, id) in plan.defs.keys().zip(&roots) {
        if let Some(v) = run.value(id, store, &plan) {
            if let Some(r) = decode(&v.bytes, window) {
                seeds.insert(name.clone(), r);
            }
        }
    }
    let seeded_diags = run.world_diags.clone();
    let ModelPlan { p, .. } = plan;
    (finish(world, p, seeds, seeded_diags, baseline), Some(run))
}
