//! The two makers text declares in M3 (`spec/geometry.md` 8): `[network.<id>]`, which nodes the
//! paths and region outlines it selects into junctions, edges and areas (3.1, 3.2), and
//! `[cells.<id>]`, which divides regions into cells along a frontage (3.6). Both are makers, not
//! producers (`rules.md` 9): they place no instance. What they make is named by `identity.md` 2.3
//! and selected by `{ of, class, near }` (`identity.md` 3).
//!
//! The kernel does the geometry (`via_geom::network`, `via_geom::cells`); this module reads the
//! two tables, resolves what their keys name, runs the kernel and names the results.

use std::collections::{BTreeMap, BTreeSet};

use via_diag::{codes, Site, Subject, Value as DValue};
use via_doc::{Table, Value, World};
use via_geom::cells::{cells_on_area, CellParams, FrontEdge};
use via_geom::lineref::Line;
use via_geom::names::{cell_keys, network_keys, rest_keys};
use via_geom::network::{network, representative, Network, PostGrid, Side};
use via_geom::noding::Input;
use via_geom::{Region, P};

use crate::decl::{table_of, Decls, Select};
use crate::place::{Geo, Placer, Spot};
use crate::text::{self, at_header, at_key, at_path};

/// `[network.<id>]` (`geometry.md` 8.1).
#[derive(Clone, Debug)]
pub struct NetworkDecl {
    #[allow(dead_code)] // the id the table is keyed by; kept beside its facts
    pub id: String,
    /// Names and `kind:<k>` selectors.
    pub from: Vec<String>,
    /// A region feature whose outline clips the network.
    pub within: Option<String>,
    pub kind: Option<String>,
    #[allow(dead_code)] // the layer rank: a reference reads downward (`documents.md` 5)
    pub rank: i64,
}

/// What a cells table divides (`geometry.md` 8.2, `within`).
#[derive(Clone, Debug)]
pub enum CellsWithin {
    /// A network's id (every area) or a region feature's id.
    Name(String),
    /// `kind:<k>`: every region feature of that kind.
    Kind(String),
    /// One selected area, cell, rest or piece.
    Sel(Select),
}

/// `[cells.<id>]` (`geometry.md` 8.2).
#[derive(Clone, Debug)]
pub struct CellsDecl {
    #[allow(dead_code)] // the id the table is keyed by; kept beside its facts
    pub id: String,
    pub within: CellsWithin,
    /// Names and `kind:<k>` selectors; `None` for every source.
    pub frontage: Option<Vec<String>>,
    pub params: CellParams,
    pub kind: Option<String>,
    #[allow(dead_code)] // the layer rank: a reference reads downward (`documents.md` 5)
    pub rank: i64,
}

/// One thing a maker made, by its produced name.
#[derive(Clone, Debug)]
pub struct MadeThing {
    pub name: String,
    /// `junction`, `edge`, `area`, `cell` or `rest` (`identity.md` 2.3).
    pub class: &'static str,
    /// A junction's point, an edge's path in the travel of its bytewise-lowest source, or a
    /// region.
    pub geo: Geo,
    /// The representative point (a junction's own point, an edge's middle vertex).
    pub point: P,
}

/// What one maker made, in name order.
#[derive(Clone, Debug, Default)]
pub struct Made {
    /// The kind its areas or cells take.
    pub kind: Option<String>,
    pub things: Vec<MadeThing>,
    /// The features and makers it read, for the graph's edges.
    pub reads: BTreeSet<String>,
    /// The features it noded together (a network's inputs), which own their meetings (2.7).
    pub inputs: Vec<String>,
}

/// A network kept for the cells that divide its areas.
#[derive(Clone, Debug)]
pub struct KeptNet {
    pub net: Network,
    pub keys: via_geom::names::NetworkKeys,
}

fn list_of(v: &Value) -> Option<Vec<String>> {
    let Value::List(xs) = v else { return None };
    xs.iter()
        .map(|x| match &x.value {
            Value::Str(s) => Some(s.text.clone()),
            _ => None,
        })
        .collect()
}

impl Decls {
    /// Reads every `[network]` and `[cells]` table.
    pub(crate) fn read_makers(&mut self, world: &World, root: &Table) {
        for (id, t) in table_of(root, "network") {
            let Some(from) = t.value("from").and_then(list_of) else {
                self.push(at_header(
                    codes::doc::MISSING_KEY,
                    "network",
                    id,
                    "the network has no `from`",
                    "write `from = [\"<path or region>\", \"kind:<kind>\"]`",
                ));
                continue;
            };
            self.maker_kind_effects("network", id, t.str("kind"));
            self.networks.insert(
                id.to_string(),
                NetworkDecl {
                    id: id.to_string(),
                    from,
                    within: t.str("within").map(str::to_string),
                    kind: t.str("kind").map(str::to_string),
                    rank: world.rank("network", id).unwrap_or(0),
                },
            );
        }
        for (id, t) in table_of(root, "cells") {
            let within = match (t.value("within"), t.table("within")) {
                (_, Some(w)) => match w.str("of") {
                    Some(of) => match self.read_select("cells", id, w, of, &["within"]) {
                        Some(sel) => CellsWithin::Sel(sel),
                        None => continue,
                    },
                    None => {
                        self.push(at_path(
                            codes::doc::MISSING_KEY,
                            "cells",
                            id,
                            &["within"],
                            "a selection in `within` needs `of`",
                            "write `within = { of = \"<maker>\", class = \"area\", near = [x, y] }`",
                        ));
                        continue;
                    }
                },
                (Some(Value::Str(s)), None) => match s.text.strip_prefix("kind:") {
                    Some(k) => CellsWithin::Kind(k.to_string()),
                    None => CellsWithin::Name(s.text.clone()),
                },
                _ => {
                    self.push(at_header(
                        codes::doc::MISSING_KEY,
                        "cells",
                        id,
                        "the cells table has no `within`",
                        "write `within = \"<network or region>\"`",
                    ));
                    continue;
                }
            };
            let mut ok = true;
            let mut len = |me: &mut Self, key: &str, need: bool| -> i64 {
                match t.value(key).and_then(text::length) {
                    Some(Ok(mm)) => mm,
                    Some(Err(e)) => {
                        me.push(text::num_diag(&e, "cells", id, &[key]));
                        ok = false;
                        0
                    }
                    None if need => {
                        me.push(at_header(
                            codes::doc::MISSING_KEY,
                            "cells",
                            id,
                            &format!("the cells table has no `{key}`"),
                            &format!("write `{key}` in metres"),
                        ));
                        ok = false;
                        0
                    }
                    None => 0,
                }
            };
            let spacing = len(self, "spacing", true);
            let depth = len(self, "depth", true);
            let min_length = len(self, "min_length", false);
            let min_area: i128 = match t.value("min_area") {
                // Square metres read exactly in square millimetres: millionths of a square metre.
                Some(Value::Num(n)) => match via_num::parse_millionths(&n.norm) {
                    Ok(v) => i128::from(v),
                    Err(e) => {
                        self.push(text::num_diag(&e, "cells", id, &["min_area"]));
                        ok = false;
                        0
                    }
                },
                _ => 0,
            };
            if !ok {
                continue;
            }
            // `geometry.md` 8.2: each out-of-range key is `geom.bad_parameter`, naming the key.
            let bad = [
                ("spacing", spacing <= 0),
                ("depth", depth <= 0),
                ("min_length", min_length < 0 || min_length > spacing),
                ("min_area", min_area < 0),
            ];
            let bad: Vec<&str> = bad.iter().filter(|(_, b)| *b).map(|(k, _)| *k).collect();
            for key in &bad {
                self.push(
                    at_key(
                        codes::geom::BAD_PARAMETER,
                        "cells",
                        id,
                        key,
                        &format!("`{key}` is out of range"),
                        "`spacing` and `depth` above 0; `min_length` from 0 to `spacing`; `min_area` at least 0",
                    )
                    .with("parameter", (*key).to_string()),
                );
            }
            if !bad.is_empty() {
                continue;
            }
            self.maker_kind_effects("cells", id, t.str("kind"));
            self.cells.insert(
                id.to_string(),
                CellsDecl {
                    id: id.to_string(),
                    within,
                    frontage: t.value("frontage").and_then(list_of),
                    params: CellParams {
                        cell_length: spacing,
                        band_depth: depth,
                        min_cell_length: min_length,
                        min_cell_area: min_area,
                    },
                    kind: t.str("kind").map(str::to_string),
                    rank: world.rank("cells", id).unwrap_or(0),
                },
            );
        }
    }

    /// `geometry.md` 8.1, Open: a made area or cell is written to `features.json` with its kind,
    /// but the kind's effects do not reach it yet (fields are made before the makers run). A
    /// maker's `kind` with effects is `doc.not_yet`, never a silent no-op.
    fn maker_kind_effects(&mut self, top: &str, id: &str, kind: Option<&str>) {
        let Some(k) = kind else { return };
        let Some(effects) = self.kinds.get(k).map(|k| k.effects.len()) else {
            return;
        };
        if effects > 0 {
            self.push(
                at_key(
                    codes::doc::NOT_YET,
                    top,
                    id,
                    "kind",
                    &format!(
                        "kind `{k}` has {effects} effect(s); effects on a maker's areas and cells arrive in a later M3 slice"
                    ),
                    "give the maker a kind with no effects, or return the regions from a step with that kind",
                )
                .with("kind", k.to_string()),
            );
        }
    }

    /// The names a maker table reads by its keys, `kind:` selectors expanded to the features of
    /// that kind, for causes (`identity.md` 5).
    pub(crate) fn maker_reads(&self, id: &str) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let mut sels: Vec<&String> = Vec::new();
        if let Some(n) = self.networks.get(id) {
            sels.extend(n.from.iter());
            names.extend(n.within.clone());
        }
        if let Some(c) = self.cells.get(id) {
            match &c.within {
                CellsWithin::Name(n) => names.push(n.clone()),
                CellsWithin::Kind(k) => {
                    names.extend(
                        self.features
                            .iter()
                            .filter(|(_, f)| f.kind.as_deref() == Some(k.as_str()))
                            .map(|(i, _)| i.clone()),
                    );
                    names.extend(self.makers_of_kind(k).into_iter().filter(|m| m != id));
                }
                CellsWithin::Sel(sel) => names.push(sel.of.clone()),
            }
            sels.extend(c.frontage.iter().flatten());
        }
        for s in sels {
            match s.strip_prefix("kind:") {
                Some(k) => names.extend(
                    self.features
                        .iter()
                        .filter(|(_, f)| f.kind.as_deref() == Some(k))
                        .map(|(i, _)| i.clone()),
                ),
                None => names.push(s.clone()),
            }
        }
        names
    }

    /// The network and cells tables whose `kind` is `k`, in id order (`geometry.md` 8.2: a
    /// `kind:` selector in `within` takes the areas and cells a table gives that kind).
    pub(crate) fn makers_of_kind(&self, k: &str) -> Vec<String> {
        let nets = self
            .networks
            .iter()
            .filter(|(_, n)| n.kind.as_deref() == Some(k))
            .map(|(i, _)| i.clone());
        let cells = self
            .cells
            .iter()
            .filter(|(_, c)| c.kind.as_deref() == Some(k))
            .map(|(i, _)| i.clone());
        let mut out: Vec<String> = nets.chain(cells).collect();
        out.sort();
        out
    }

    /// What a declared id that is not a feature is, in words, for a diagnostic.
    pub(crate) fn what_is(&self, id: &str) -> Option<&'static str> {
        if self.networks.contains_key(id) {
            Some("a network")
        } else if self.cells.contains_key(id) {
            Some("a cells table")
        } else if self.steps.contains_key(id) {
            Some("a step")
        } else if self.scatters.contains_key(id) {
            Some("a scatter")
        } else if self.instances.contains_key(id) {
            Some("an instance")
        } else {
            match self.declared.get(id) {
                Some(&"pin") => Some("a pin"),
                Some(&"instance") => Some("an instance"),
                Some(&"feature") => Some("a feature"),
                _ => None,
            }
        }
    }

    /// Whether an id names a network or a cells table.
    pub(crate) fn is_maker_table(&self, id: &str) -> bool {
        self.networks.contains_key(id) || self.cells.contains_key(id)
    }

    /// The top table of a maker table's id.
    pub(crate) fn maker_top(&self, id: &str) -> Option<&'static str> {
        if self.networks.contains_key(id) {
            Some("network")
        } else if self.cells.contains_key(id) {
            Some("cells")
        } else {
            None
        }
    }
}

/// The edge's path in the travel of its bytewise-lowest source (`identity.md` 3: `t = 0` at the
/// start of that travel).
fn edge_path(net: &Network, e: usize) -> Vec<P> {
    let edge = &net.edges[e];
    let srcs = &net.noded.sources;
    let lowest = edge
        .sources
        .iter()
        .copied()
        .min_by(|&a, &b| srcs[a].as_bytes().cmp(srcs[b].as_bytes()));
    let h0 = edge.half[0];
    let along = net.noded.segments[h0 / 2]
        .prov
        .iter()
        .find(|p| Some(p.source) == lowest)
        .is_none_or(|p| h0.is_multiple_of(2) != p.reversed);
    let mut path = edge.path.clone();
    if !along {
        path.reverse();
    }
    path
}

fn region_geo(region: Region) -> Geo {
    let line = (region.pieces.len() == 1 && region.pieces[0].holes.is_empty())
        .then(|| Line::new(region.pieces[0].shell.clone(), true));
    Geo::Region { region, line }
}

/// A region feature's outline as one ring source: `Ok(None)` when it is not a region,
/// `Err(rings)` when its outline is more than one ring (`geom.many_rings`, `geometry.md` 8.1).
fn ring_of(g: &Geo) -> Result<Option<Vec<P>>, usize> {
    match g {
        Geo::Region { region, .. } => {
            let rings: usize = region.pieces.iter().map(|p| 1 + p.holes.len()).sum();
            if rings != 1 {
                return Err(rings);
            }
            Ok(region.pieces.first().map(|p| p.shell.clone()))
        }
        _ => Ok(None),
    }
}

/// `geom.many_rings` on a maker's key: the region named has more than one ring.
fn many_rings(top: &str, id: &str, key: &str, name: &str, rings: usize) -> via_diag::Diag {
    at_key(
        codes::geom::MANY_RINGS,
        top,
        id,
        key,
        &format!("`{name}` is a region of {rings} rings; a source is one ring"),
        "select one piece, or split the region into features of one ring each",
    )
    .with("feature", name.to_string())
}

/// The frontage entries of one area: each shell edge side whose edge lies on a selected source,
/// the side facing the area (`geometry.md` 8.2).
fn area_front(net: &Network, area: usize, selected: &dyn Fn(&str) -> bool) -> Vec<FrontEdge> {
    let arr = &net.arr;
    let c = net.areas[area].cycle;
    let in_face = |h: usize| arr.face_left(h) == Some(c);
    let mut out: BTreeSet<FrontEdge> = BTreeSet::new();
    for &h in &arr.cycles[c] {
        if in_face(h ^ 1) {
            continue;
        }
        for p in &net.noded.segments[h / 2].prov {
            if !selected(&net.noded.sources[p.source]) {
                continue;
            }
            let along = h.is_multiple_of(2) != p.reversed;
            out.insert(FrontEdge {
                edge: net.edge_of_half[h],
                side: if along { Side::Left } else { Side::Right },
                source: p.source,
            });
        }
    }
    out.into_iter().collect()
}

impl Placer<'_> {
    fn post_grid(&self) -> Option<PostGrid> {
        let g = self.d.grid?;
        Some(PostGrid {
            extent: g.extent,
            resolution: g.resolution,
        })
    }

    /// The features a list of names and `kind:` selectors selects, in name order, and for each
    /// named point feature a `geom.network_point` when `points_fault` is set.
    fn selected_features(&self, list: &[String]) -> (BTreeSet<String>, Vec<String>) {
        let mut names = BTreeSet::new();
        let mut named = Vec::new();
        for s in list {
            if let Some(k) = s.strip_prefix("kind:") {
                for (fid, f) in &self.d.features {
                    if f.kind.as_deref() == Some(k) {
                        names.insert(fid.clone());
                    }
                }
            } else {
                names.insert(s.clone());
                named.push(s.clone());
            }
        }
        (names, named)
    }

    /// What a maker made, made once.
    pub(crate) fn made(&mut self, id: &str) -> Option<Made> {
        if let Some(m) = self.out.made.get(id) {
            return Some(m.clone());
        }
        if !self.d.is_maker_table(id) || self.failed.contains(id) {
            return None;
        }
        let top = self.d.maker_top(id).unwrap_or("network");
        if !self.enter(id, top) {
            return None;
        }
        let m = if top == "network" {
            self.make_network(id)
        } else {
            self.make_cells(id)
        };
        self.leave();
        match &m {
            Some(m) => {
                self.out.made.insert(id.to_string(), m.clone());
            }
            None => {
                self.failed.insert(id.to_string());
            }
        }
        m
    }

    fn make_network(&mut self, id: &str) -> Option<Made> {
        let n = self.d.networks.get(id)?.clone();
        let pg = self.post_grid()?;
        let (names, named) = self.selected_features(&n.from);
        let mut inputs: Vec<Input> = Vec::new();
        let mut used = Vec::new();
        let mut reads: BTreeSet<String> = BTreeSet::new();
        let mut fault = false;
        for name in &names {
            if !self.d.features.contains_key(name) {
                // A feature whose own text failed is reported there; anything else is named here.
                if self.d.declared.get(name.as_str()) != Some(&"feature") {
                    let what = match self.d.what_is(name) {
                        Some(w) => format!("`{name}` is {w}, not a path or region feature"),
                        None => format!("`{name}` names nothing"),
                    };
                    self.diags.push(
                        at_key(
                            codes::identity::UNKNOWN_PLACE,
                            "network",
                            id,
                            "from",
                            &what,
                            "name a path or region feature, or select by `kind:<kind>`",
                        )
                        .with("name", name.clone()),
                    );
                    fault = true;
                }
                continue;
            }
            reads.insert(name.clone());
            let Some(g) = self.feature(name) else {
                continue;
            };
            match &g {
                Geo::Point { .. } => {
                    if named.contains(name) {
                        self.diags.push(
                            at_key(
                                codes::geom::NETWORK_POINT,
                                "network",
                                id,
                                "from",
                                &format!(
                                    "`{name}` is a point feature; a network is made of paths and region outlines"
                                ),
                                "name a path or region, or place the point `on` one",
                            )
                            .with("feature", name.clone()),
                        );
                        fault = true;
                    }
                }
                Geo::Path { line } => {
                    inputs.push(Input {
                        source: name.clone(),
                        pts: line.pts.clone(),
                        closed: false,
                    });
                    used.push(name.clone());
                }
                Geo::Region { .. } => match ring_of(&g) {
                    Ok(Some(r)) => {
                        inputs.push(Input {
                            source: name.clone(),
                            pts: r,
                            closed: true,
                        });
                        used.push(name.clone());
                    }
                    Ok(None) => {}
                    Err(rings) => {
                        self.diags
                            .push(many_rings("network", id, "from", name, rings));
                        fault = true;
                    }
                },
            }
        }
        let mut clip = None;
        if let Some(w) = &n.within {
            reads.insert(w.clone());
            let ring = match self.feature(w).as_ref().map(ring_of) {
                Some(Err(rings)) => {
                    self.diags
                        .push(many_rings("network", id, "within", w, rings));
                    return None;
                }
                Some(Ok(r)) => r,
                None => None,
            };
            match ring {
                Some(r) => {
                    clip = Some(inputs.len());
                    inputs.push(Input {
                        source: w.clone(),
                        pts: r,
                        closed: true,
                    });
                }
                None => {
                    self.diags.push(at_key(
                        codes::identity::UNKNOWN_PLACE,
                        "network",
                        id,
                        "within",
                        &format!("`{w}` is not a region feature"),
                        "name a region feature to clip by",
                    ));
                    fault = true;
                }
            }
        }
        if fault {
            return None;
        }
        let mut made = Made {
            kind: n.kind.clone(),
            reads,
            inputs: used,
            ..Made::default()
        };
        if inputs.is_empty() {
            self.diags.push(at_header(
                codes::geom::NO_AREA,
                "network",
                id,
                "the network's `from` selects nothing, so it encloses no area",
                "select paths or regions in `from`",
            ));
            return Some(made);
        }
        let net = match network(inputs, clip, &[], pg) {
            Ok(net) => net,
            Err(e) => {
                self.diags
                    .push(e.to_diag(Subject::entity("network", id), Site::Header));
                return None;
            }
        };
        for g in &net.diags {
            self.diags
                .push(g.to_diag(Subject::entity("network", id), Site::Header));
        }
        let keys = network_keys(&net);
        let q = |k: &str| format!("{id}#{k}");
        for (i, &p) in net.junctions.iter().enumerate() {
            made.things.push(MadeThing {
                name: q(&keys.junctions[i]),
                class: "junction",
                geo: Geo::Point { p, facing: None },
                point: p,
            });
        }
        for (i, e) in net.edges.iter().enumerate() {
            let path = edge_path(&net, i);
            made.things.push(MadeThing {
                name: q(&keys.edges[i]),
                class: "edge",
                point: e.path[e.path.len() / 2],
                geo: Geo::Path {
                    line: Line::new(path, false),
                },
            });
        }
        for (i, a) in net.areas.iter().enumerate() {
            made.things.push(MadeThing {
                name: q(&keys.areas[i]),
                class: "area",
                geo: region_geo(a.region.clone()),
                point: a.point,
            });
        }
        made.things
            .sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        self.out.nets.insert(id.to_string(), KeptNet { net, keys });
        Some(made)
    }

    fn make_cells(&mut self, id: &str) -> Option<Made> {
        let c = self.d.cells.get(id)?.clone();
        let pg = self.post_grid()?;
        let mut reads: BTreeSet<String> = BTreeSet::new();
        // The regions divided: (divided region's name, what it is). A `Div::Area` is pushed only
        // through `area_of`, which found its network kept in `out.nets`, so the `?` lookups of a
        // kept network below cannot fail. A missing grid (no `extent`), a failed network or a
        // failed selection was reported where it failed.
        enum Div {
            Area(String, usize),
            Feature(String, Region),
            Region(String, Region),
        }
        let mut divs: Vec<Div> = Vec::new();
        let area_of = |me: &Self, net_id: &str, name: &str| -> Option<usize> {
            let kn = me.out.nets.get(net_id)?;
            let key = name.strip_prefix(&format!("{net_id}#"))?;
            kn.keys.areas.iter().position(|k| k == key)
        };
        match &c.within {
            CellsWithin::Name(n) if self.d.networks.contains_key(n) => {
                reads.insert(n.clone());
                let m = self.made(n)?;
                for t in m.things.iter().filter(|t| t.class == "area") {
                    if let Some(a) = area_of(self, n, &t.name) {
                        divs.push(Div::Area(n.clone(), a));
                    }
                }
            }
            CellsWithin::Name(n) => {
                reads.insert(n.clone());
                match self.feature(n) {
                    Some(Geo::Region { region, .. }) => divs.push(Div::Feature(n.clone(), region)),
                    Some(_) => {
                        self.diags.push(at_key(
                            codes::identity::UNKNOWN_PLACE,
                            "cells",
                            id,
                            "within",
                            &format!("`{n}` is not a region"),
                            "name a network or a region feature",
                        ));
                        return None;
                    }
                    None => {
                        if !self.d.declared.contains_key(n.as_str()) {
                            self.diags.push(at_key(
                                codes::identity::UNKNOWN_PLACE,
                                "cells",
                                id,
                                "within",
                                &format!("`{n}` names nothing"),
                                "name a network or a region feature",
                            ));
                        }
                        return None;
                    }
                }
            }
            CellsWithin::Kind(k) => {
                let ids: Vec<String> = self
                    .d
                    .features
                    .iter()
                    .filter(|(_, f)| f.kind.as_deref() == Some(k.as_str()))
                    .map(|(i, _)| i.clone())
                    .collect();
                for f in ids {
                    if let Some(Geo::Region { region, .. }) = self.feature(&f) {
                        reads.insert(f.clone());
                        divs.push(Div::Feature(f, region));
                    }
                }
                // And every area or cell a table gives that kind (8.2); never this table's own.
                for m in self.d.makers_of_kind(k) {
                    if m == id {
                        continue;
                    }
                    reads.insert(m.clone());
                    let Some(made) = self.made(&m) else { continue };
                    let is_net = self.d.networks.contains_key(&m);
                    for t in &made.things {
                        let Geo::Region { region, .. } = &t.geo else {
                            continue;
                        };
                        match t.class {
                            "area" if is_net => match area_of(self, &m, &t.name) {
                                Some(a) => divs.push(Div::Area(m.clone(), a)),
                                None => divs.push(Div::Region(t.name.clone(), region.clone())),
                            },
                            "cell" => divs.push(Div::Region(t.name.clone(), region.clone())),
                            _ => {}
                        }
                    }
                }
            }
            CellsWithin::Sel(sel) => {
                reads.insert(sel.of.clone());
                let spot = self.select("cells", id, sel)?;
                let name = self
                    .out
                    .selections
                    .get(&format!("cells.{id}"))
                    .map(|s| s.0.clone())
                    .unwrap_or_default();
                let Spot::Long(Geo::Region { region, .. }) = spot else {
                    self.diags.push(at_path(
                        codes::identity::NEAR_NONE,
                        "cells",
                        id,
                        &["within"],
                        "`within` takes a region: an `area`, `cell`, `rest` or `piece`",
                        "select a region class",
                    ));
                    return None;
                };
                match name.split_once('#') {
                    Some((maker, _)) if self.d.networks.contains_key(maker) => {
                        match area_of(self, maker, &name) {
                            Some(a) => divs.push(Div::Area(maker.to_string(), a)),
                            None => divs.push(Div::Region(name, region)),
                        }
                    }
                    _ => divs.push(Div::Region(name, region)),
                }
            }
        }
        let frontage = c.frontage.clone();
        let (front_names, _) = match &frontage {
            Some(list) => self.selected_features(list),
            None => (BTreeSet::new(), Vec::new()),
        };
        let selected = |s: &str| frontage.is_none() || front_names.contains(s);
        let mut made = Made {
            kind: c.kind.clone(),
            reads,
            ..Made::default()
        };
        // Which regions have a frontage side at all (`geom.frontage_none`).
        let mut fronts: Vec<Option<Vec<FrontEdge>>> = Vec::new();
        for d in &divs {
            fronts.push(match d {
                Div::Area(n, a) => {
                    let kn = self.out.nets.get(n)?;
                    let f = area_front(&kn.net, *a, &selected);
                    (!f.is_empty()).then_some(f)
                }
                Div::Feature(n, _) => selected(n).then(Vec::new),
                Div::Region(n, _) => selected(n).then(Vec::new),
            });
        }
        if !divs.is_empty() && fronts.iter().all(Option::is_none) {
            self.diags.push(at_key(
                codes::geom::FRONTAGE_NONE,
                "cells",
                id,
                "frontage",
                "the `frontage` selects no side of any region this table divides, so nothing is divided",
                "select the sources the cells should face, or leave `frontage` out for every side",
            ));
            return Some(made);
        }
        let q = |k: &str| format!("{id}#{k}");
        for (d, front) in divs.iter().zip(fronts) {
            let divided = match d {
                Div::Area(n, a) => {
                    let kn = self.out.nets.get(n)?;
                    format!("{n}#{}", kn.keys.areas[*a])
                }
                Div::Feature(n, _) | Div::Region(n, _) => n.clone(),
            };
            let whole = match d {
                Div::Area(n, a) => self.out.nets.get(n)?.net.areas[*a].region.clone(),
                Div::Feature(_, r) | Div::Region(_, r) => r.clone(),
            };
            let Some(front) = front else {
                // A region nothing faces is all rest, and says nothing (8.2).
                let rk = rest_keys(&q(&divided), &[representative(&whole, pg)]);
                made.things.push(MadeThing {
                    name: q(&rk[0]),
                    class: "rest",
                    point: representative(&whole, pg),
                    geo: region_geo(whole),
                });
                continue;
            };
            if whole.pieces.len() != 1 {
                self.diags.push(at_key(
                    codes::geom::MANY_RINGS,
                    "cells",
                    id,
                    "within",
                    &format!("`{divided}` is a region of more than one piece"),
                    "divide its pieces one by one through a selection",
                ));
                continue;
            }
            // A region feature is its own source, side left: its outline made a network of one
            // source, every edge side facing in.
            let (owned, net_id) = match d {
                Div::Area(n, _) => (None, Some(n.clone())),
                Div::Feature(n, r) | Div::Region(n, r) => {
                    let net = match network(
                        vec![Input {
                            source: n.clone(),
                            pts: r.pieces[0].shell.clone(),
                            closed: true,
                        }],
                        None,
                        &[],
                        pg,
                    ) {
                        Ok(net) => net,
                        Err(e) => {
                            self.diags
                                .push(e.to_diag(Subject::entity("cells", id), Site::Header));
                            continue;
                        }
                    };
                    let keys = network_keys(&net);
                    (Some(KeptNet { net, keys }), None)
                }
            };
            let kn: &KeptNet = match (&owned, &net_id) {
                (Some(k), _) => k,
                (None, Some(n)) => self.out.nets.get(n)?,
                _ => continue,
            };
            let (area, front) = match d {
                Div::Area(_, a) => (*a, front),
                _ => {
                    if kn.net.areas.len() != 1 {
                        // A read region's outline is simple (2.3), so this is a returned or
                        // selected region that touches itself: said, never skipped silently.
                        self.diags.push(
                            at_key(
                                codes::geom::SELF_CROSSING,
                                "cells",
                                id,
                                "within",
                                &format!(
                                    "`{divided}`'s outline encloses {} areas, not one",
                                    kn.net.areas.len()
                                ),
                                "divide a region whose outline does not touch itself",
                            )
                            .with("feature", divided.clone()),
                        );
                        continue;
                    }
                    let f = area_front(&kn.net, 0, &|_| true);
                    (0, f)
                }
            };
            let cells = match cells_on_area(&kn.net, area, &front, c.params, pg) {
                Ok(x) => x,
                Err(e) => {
                    self.diags
                        .push(e.to_diag(Subject::entity("cells", id), Site::Header));
                    continue;
                }
            };
            let mut diags = Vec::new();
            for g in &cells.diags {
                diags.push(g.to_diag(Subject::entity("cells", id), Site::Header));
            }
            let ck = cell_keys(&kn.net, &kn.keys, &cells);
            let mut things = Vec::new();
            for (i, cell) in cells.cells.iter().enumerate() {
                things.push(MadeThing {
                    name: q(&ck[i]),
                    class: "cell",
                    point: cell.point,
                    geo: region_geo(cell.region.clone()),
                });
            }
            let rest_points: Vec<P> = cells
                .rest
                .pieces
                .iter()
                .map(|p| {
                    representative(
                        &Region {
                            pieces: vec![p.clone()],
                        },
                        pg,
                    )
                })
                .collect();
            let rk = rest_keys(&q(&divided), &rest_points);
            for (i, p) in cells.rest.pieces.iter().enumerate() {
                things.push(MadeThing {
                    name: q(&rk[i]),
                    class: "rest",
                    point: rest_points[i],
                    geo: region_geo(Region {
                        pieces: vec![p.clone()],
                    }),
                });
            }
            self.diags.extend(diags);
            made.things.extend(things);
        }
        made.things
            .sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Some(made)
    }

    /// A selection by place from a network or cells table (`identity.md` 3): the thing of the
    /// class nearest `near`, ties to the bytewise-least name.
    pub(crate) fn select_made(&mut self, top: &str, id: &str, sel: &Select) -> Option<Spot> {
        let key: &[&str] = if matches!(top, "scatter" | "cells") {
            &["within", "of"]
        } else {
            &["on", "of"]
        };
        let classes: &[&str] = if self.d.networks.contains_key(&sel.of) {
            &["junction", "edge", "area"]
        } else {
            &["cell", "rest"]
        };
        let made = self.made(&sel.of)?;
        let near = P::new(sel.near.0, sel.near.1);
        let Some(class) = sel.class.clone() else {
            let mut counts = Vec::new();
            for c in classes {
                counts.push(format!(
                    "`{c}` ({})",
                    made.things.iter().filter(|t| t.class == *c).count()
                ));
            }
            let mut path: Vec<&str> = key[..1].to_vec();
            path.push("of");
            self.diags.push(
                at_path(
                    codes::identity::OF_AMBIGUOUS,
                    top,
                    id,
                    &path,
                    &format!(
                        "`{}` makes {} classes, and the selection names none",
                        sel.of,
                        classes.len()
                    ),
                    &format!("write `class`: one of {}", counts.join(", ")),
                )
                .with("classes", DValue::texts(classes.iter().copied())),
            );
            return None;
        };
        // Things in name order and a strict `<` on exact integer distances (`identity.md` 3): a
        // tie keeps the bytewise-least name.
        let mut best: Option<(i128, P, &MadeThing)> = None;
        for t in made.things.iter().filter(|t| t.class == class) {
            let (d, at) = t.geo.nearest_mm(near);
            if best.as_ref().is_none_or(|(b, ..)| d < *b) {
                best = Some((d, at, t));
            }
        }
        let Some((_, nearest, t)) = best else {
            self.near_none(&sel.of, &class, &[(top, id)]);
            return None;
        };
        let at = match &t.geo {
            Geo::Path { .. } => nearest,
            _ => t.point,
        };
        self.out
            .selections
            .insert(format!("{top}.{id}"), (t.name.clone(), at));
        Some(match &t.geo {
            Geo::Point { p, facing } => Spot::At {
                p: *p,
                facing: *facing,
                end: false,
            },
            g => Spot::Long(g.clone()),
        })
    }

    /// Makes every network and cells table, in id order.
    pub(crate) fn make_all(&mut self) {
        let ids: Vec<String> = self
            .d
            .networks
            .keys()
            .chain(self.d.cells.keys())
            .cloned()
            .collect();
        for id in ids {
            let _ = self.made(&id);
        }
    }
}

/// The produced region features of every maker, by name: (kind, region).
pub(crate) fn region_features(
    made: &BTreeMap<String, Made>,
) -> Vec<(String, Option<String>, Region)> {
    let mut out = Vec::new();
    for m in made.values() {
        for t in &m.things {
            if let Geo::Region { region, .. } = &t.geo {
                let kind = if t.class == "rest" {
                    None
                } else {
                    m.kind.clone()
                };
                out.push((t.name.clone(), kind, region.clone()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(source: &str, pts: &[(i64, i64)], closed: bool) -> Input {
        Input {
            source: source.to_string(),
            pts: pts
                .iter()
                .map(|&(x, y)| P::new(x * 1000, y * 1000))
                .collect(),
            closed,
        }
    }

    fn grid() -> PostGrid {
        PostGrid {
            extent: [0, 0, 100_000, 100_000],
            resolution: 500,
        }
    }

    /// A 40 m square ring `r` with a path `p` along its south side for 20 m (an overlap) and a
    /// spur `s` into it from the north side.
    fn net() -> Network {
        network(
            vec![
                input("r", &[(10, 10), (50, 10), (50, 50), (10, 50)], true),
                input("p", &[(10, 10), (30, 10)], false),
                input("s", &[(30, 50), (30, 35)], false),
            ],
            None,
            &[],
            grid(),
        )
        .expect("network")
    }

    fn sources(n: &Network, f: &[FrontEdge]) -> BTreeSet<String> {
        f.iter()
            .map(|e| n.noded.sources[e.source].clone())
            .collect()
    }

    #[test]
    fn area_front_takes_shell_sides_of_selected_sources() {
        let n = net();
        assert_eq!(n.areas.len(), 1);
        let all = area_front(&n, 0, &|_| true);
        // Every shell edge side of `r`, plus `p` where it overlaps the south side; the spur `s` is
        // not frontage yet (geometry.md 8.2, Spurs: Open).
        assert_eq!(
            sources(&n, &all),
            ["p", "r"].iter().map(|s| s.to_string()).collect()
        );
        let only_p = area_front(&n, 0, &|s| s == "p");
        assert!(!only_p.is_empty());
        assert!(only_p.iter().all(|e| n.noded.sources[e.source] == "p"));
        // `p` runs east along the south side, the area on its left.
        assert!(only_p.iter().all(|e| e.side == Side::Left));
        assert!(area_front(&n, 0, &|s| s == "s").is_empty());
    }

    #[test]
    fn ring_of_refuses_more_than_one_ring() {
        let sq = |x: i64| via_geom::Piece {
            shell: vec![
                P::new(x, 0),
                P::new(x + 10, 0),
                P::new(x + 10, 10),
                P::new(x, 10),
            ],
            holes: Vec::new(),
        };
        let one = region_geo(Region {
            pieces: vec![sq(0)],
        });
        assert_eq!(ring_of(&one).map(|r| r.map(|v| v.len())), Ok(Some(4)));
        let two = region_geo(Region {
            pieces: vec![sq(0), sq(20)],
        });
        assert_eq!(ring_of(&two), Err(2));
        let pt = Geo::Point {
            p: P::new(0, 0),
            facing: None,
        };
        assert_eq!(ring_of(&pt), Ok(None));
    }
}
