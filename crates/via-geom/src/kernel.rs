//! The `via kernel OP --in FILE --out FILE` command's files (`spec/geometry.md` 7): reads the input
//! object, runs one operation, and writes the canonical output object. The command-line wrapper
//! (`via-cli`, or this crate's `kernel` example) only moves bytes and sets the exit status.

use crate::cells::{cells_on_area, cells_on_outline, CellParams, Front, FrontEdge};
use crate::diag::{has_error, Code, GeomDiag};
use crate::follow::{follow, Reference};
use crate::held::{hold, Clear, ClearPlace, Instance, Pin};
use crate::json::{parse, Json};
use crate::lineref::{self, Line};
use crate::mask::{mask, MaskGrid};
use crate::names::{cell_keys, network_keys, rest_keys};
use crate::network::{network, Network, PostGrid, Side};
use crate::noding::{node, overlaps, Input, Noded};
use crate::offset::{offset, stroke, Cap, Join};
use crate::overlay::{boolean, BoolOp};
use crate::shape::{read_path, read_ring, Piece, Region, P};
use std::collections::BTreeMap;
use via_diag::{NoWorld, Site, Subject};

/// The input and output format.
pub const FORMAT: &str = "via-kernel-1";

/// What a run gives the command line.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// The exit status (7.4).
    pub status: u8,
    /// The output file's text; `None` on a usage error (nothing is written).
    pub output: Option<String>,
    /// On a usage error, what is wrong (the member path or argument).
    pub usage: Option<String>,
    /// On `geom.internal`: the reproduction's digest and its `<op>.json` text.
    pub repro: Option<(String, String)>,
}

/// The operations (7.2).
pub const OPS: &[&str] = &[
    "boolean",
    "cells",
    "follow",
    "interpolate",
    "mask",
    "names",
    "network",
    "noding",
    "offset",
    "project",
    "read",
    "reverse",
    "stroke",
    "substring",
    "triangulate",
];

fn usage(what: impl Into<String>) -> Outcome {
    Outcome {
        status: 2,
        output: None,
        usage: Some(what.into()),
        repro: None,
    }
}

type R<T> = Result<T, Fail>;

/// A failure: a usage fault (exit 2) or a kernel diagnostic (exit 1 or 3).
enum Fail {
    Usage(String),
    Geom(GeomDiag),
}

impl From<GeomDiag> for Fail {
    fn from(d: GeomDiag) -> Fail {
        Fail::Geom(d)
    }
}

fn need<'a>(j: &'a Json, k: &str, path: &str) -> R<&'a Json> {
    j.get(k).ok_or_else(|| Fail::Usage(format!("{path}{k}")))
}

fn point(j: &Json, path: &str) -> R<P> {
    let a = j.arr().ok_or_else(|| Fail::Usage(path.to_string()))?;
    if a.len() != 2 {
        return Err(Fail::Usage(path.to_string()));
    }
    let x = a[0].int().ok_or_else(|| Fail::Usage(path.to_string()))?;
    let y = a[1].int().ok_or_else(|| Fail::Usage(path.to_string()))?;
    let p = P::new(x, y);
    if !p.in_range() {
        return Err(Fail::Geom(
            GeomDiag::new(Code::NumOutOfRange, "an input coordinate is out of range")
                .at(P::new(0, 0)),
        ));
    }
    Ok(p)
}

fn points(j: &Json, path: &str) -> R<Vec<P>> {
    let a = j.arr().ok_or_else(|| Fail::Usage(path.to_string()))?;
    a.iter()
        .enumerate()
        .map(|(i, p)| point(p, &format!("{path}[{i}]")))
        .collect()
}

fn region_of(j: &Json, path: &str) -> R<Region> {
    let pieces = j.arr().ok_or_else(|| Fail::Usage(path.to_string()))?;
    let mut out = Vec::new();
    for (i, pc) in pieces.iter().enumerate() {
        let rings = pc
            .arr()
            .ok_or_else(|| Fail::Usage(format!("{path}[{i}]")))?;
        let mut rs: Vec<Vec<P>> = Vec::new();
        for (k, r) in rings.iter().enumerate() {
            rs.push(points(r, &format!("{path}[{i}][{k}]"))?);
        }
        if rs.is_empty() {
            return Err(Fail::Usage(format!("{path}[{i}]")));
        }
        let shell = rs.remove(0);
        out.push(Piece { shell, holes: rs });
    }
    Ok(Region { pieces: out })
}

/// One input shape.
#[derive(Clone, Debug)]
enum Shape {
    Point(P),
    Path(Vec<P>),
    Ring(Vec<P>),
    Region(Region),
}

struct Src {
    source: String,
    shape: Shape,
}

fn shapes(input: &Json, diags: &mut Vec<GeomDiag>) -> R<Vec<Src>> {
    let list = need(input, "shapes", "")?
        .arr()
        .ok_or_else(|| Fail::Usage("shapes".into()))?;
    let mut out = Vec::new();
    for (i, s) in list.iter().enumerate() {
        let path = format!("shapes[{i}]");
        let source = need(s, "source", &format!("{path}."))?
            .str()
            .ok_or_else(|| Fail::Usage(format!("{path}.source")))?
            .to_string();
        let shape = if let Some(p) = s.get("point") {
            Shape::Point(point(p, &format!("{path}.point"))?)
        } else if let Some(p) = s.get("path") {
            let pts = points(p, &format!("{path}.path"))?;
            match read_path(&source, &pts, diags) {
                Some(v) => Shape::Path(v),
                None => {
                    return Err(Fail::Geom(
                        diags
                            .pop()
                            .unwrap_or_else(|| GeomDiag::new(Code::Degenerate, "degenerate")),
                    ))
                }
            }
        } else if let Some(p) = s.get("ring") {
            let pts = points(p, &format!("{path}.ring"))?;
            match read_ring(&source, &pts, diags) {
                Some(v) => Shape::Ring(v),
                None => {
                    return Err(Fail::Geom(
                        diags
                            .pop()
                            .unwrap_or_else(|| GeomDiag::new(Code::Degenerate, "degenerate")),
                    ))
                }
            }
        } else if let Some(p) = s.get("region") {
            Shape::Region(region_of(p, &format!("{path}.region"))?)
        } else {
            return Err(Fail::Usage(format!("{path}.path")));
        };
        out.push(Src { source, shape });
    }
    crate::noding::check_sources(out.iter().map(|s| s.source.as_str()))?;
    Ok(out)
}

fn inputs_of(srcs: &[Src]) -> Vec<Input> {
    let mut v = Vec::new();
    for s in srcs {
        match &s.shape {
            Shape::Path(p) => v.push(Input {
                source: s.source.clone(),
                pts: p.clone(),
                closed: false,
            }),
            Shape::Ring(r) => v.push(Input {
                source: s.source.clone(),
                pts: r.clone(),
                closed: true,
            }),
            Shape::Region(reg) => {
                for (k, r) in reg.rings().into_iter().enumerate() {
                    let id = if k == 0 {
                        s.source.clone()
                    } else {
                        format!("{}#{k}", s.source)
                    };
                    v.push(Input {
                        source: id,
                        pts: r.to_vec(),
                        closed: true,
                    });
                }
            }
            Shape::Point(_) => {}
        }
    }
    v
}

fn region_shape(s: &Src) -> Option<Region> {
    match &s.shape {
        Shape::Ring(r) => Some(Region::from_ring(r.clone())),
        Shape::Region(r) => Some(r.clone()),
        _ => None,
    }
}

fn j_pt(p: P) -> Json {
    Json::Arr(vec![Json::from_i(p.x), Json::from_i(p.y)])
}

fn j_pts(v: &[P]) -> Json {
    Json::Arr(v.iter().map(|&p| j_pt(p)).collect())
}

fn j_region(r: &Region) -> Json {
    Json::Arr(
        r.pieces
            .iter()
            .map(|pc| Json::Arr(pc.rings().into_iter().map(j_pts).collect()))
            .collect(),
    )
}

fn grid_of(input: &Json) -> R<PostGrid> {
    let g = need(input, "grid", "")?;
    let e = need(g, "extent", "grid.")?
        .arr()
        .ok_or_else(|| Fail::Usage("grid.extent".into()))?;
    if e.len() != 4 {
        return Err(Fail::Usage("grid.extent".into()));
    }
    let mut ext = [0i64; 4];
    for (i, v) in e.iter().enumerate() {
        ext[i] = v
            .int()
            .ok_or_else(|| Fail::Usage(format!("grid.extent[{i}]")))?;
    }
    let r = need(g, "resolution", "grid.")?
        .int()
        .ok_or_else(|| Fail::Usage("grid.resolution".into()))?;
    if r < 1 {
        return Err(Fail::Usage("grid.resolution".into()));
    }
    Ok(PostGrid {
        extent: ext,
        resolution: r,
    })
}

fn param<'a>(input: &'a Json, k: &str) -> Option<&'a Json> {
    input.get("params").and_then(|p| p.get(k))
}

fn param_int(input: &Json, k: &str) -> R<i64> {
    param(input, k)
        .and_then(Json::int)
        .ok_or_else(|| Fail::Usage(format!("params.{k}")))
}

fn join_of(input: &Json) -> R<Join> {
    match param(input, "join").and_then(Json::str) {
        Some("round") => Ok(Join::Round),
        Some("bevel") => Ok(Join::Bevel),
        Some("mitre") => {
            let l = match param(input, "limit") {
                None => 2.0,
                Some(j) => j
                    .double()
                    .ok_or_else(|| Fail::Usage("params.limit".into()))?,
            };
            Ok(Join::Mitre(l))
        }
        _ => Err(Fail::Usage("params.join".into())),
    }
}

fn prov_json(n: &Noded, seg: &crate::noding::Seg) -> Json {
    Json::Arr(
        seg.prov
            .iter()
            .map(|p| {
                Json::obj(vec![
                    ("source", Json::s(&n.sources[p.source])),
                    ("segment", Json::from_i(p.seg as i64)),
                    (
                        "t",
                        Json::Arr(vec![Json::from_f(p.t[0]), Json::from_f(p.t[1])]),
                    ),
                    ("reversed", Json::Bool(p.reversed)),
                ])
            })
            .collect(),
    )
}

fn noding_members(n: &Noded, out: &mut Vec<(&'static str, Json)>) {
    out.push(("vertices", j_pts(&n.vertices)));
    out.push((
        "segments",
        Json::Arr(
            n.segments
                .iter()
                .map(|s| {
                    Json::obj(vec![
                        ("ends", j_pts(&[s.a, s.b])),
                        ("provenance", prov_json(n, s)),
                    ])
                })
                .collect(),
        ),
    ));
}

fn network_members(net: &Network, out: &mut Vec<(&'static str, Json)>) {
    noding_members(&net.noded, out);
    out.push(("junctions", j_pts(&net.junctions)));
    out.push((
        "edges",
        Json::Arr(
            net.edges
                .iter()
                .map(|e| {
                    Json::obj(vec![
                        (
                            "ends",
                            j_pts(&[net.junctions[e.ends.0], net.junctions[e.ends.1]]),
                        ),
                        ("path", j_pts(&e.path)),
                    ])
                })
                .collect(),
        ),
    ));
    let src = &net.noded.sources;
    let edge_ref = |e: usize| {
        j_pts(&[
            net.junctions[net.edges[e].ends.0],
            net.junctions[net.edges[e].ends.1],
        ])
    };
    out.push((
        "areas",
        Json::Arr(
            net.areas
                .iter()
                .map(|a| {
                    Json::obj(vec![
                        ("region", j_region(&a.region)),
                        ("twice_area", Json::from_i128(a.region.twice_area())),
                        (
                            "sides",
                            Json::Arr(
                                a.sides
                                    .iter()
                                    .map(|&(s, side)| {
                                        Json::obj(vec![
                                            ("source", Json::s(&src[s])),
                                            ("side", Json::s(side.word())),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                        (
                            "edges",
                            Json::Arr(a.edges.iter().map(|&e| edge_ref(e)).collect()),
                        ),
                        (
                            "holes",
                            Json::Arr(
                                a.holes
                                    .iter()
                                    .map(|h| Json::Arr(h.iter().map(|&e| edge_ref(e)).collect()))
                                    .collect(),
                            ),
                        ),
                        (
                            "spurs",
                            Json::Arr(a.spurs.iter().map(|&e| edge_ref(e)).collect()),
                        ),
                        ("point", j_pt(a.point)),
                        (
                            "naming",
                            Json::Arr(a.naming.iter().map(|n| Json::s(n)).collect()),
                        ),
                    ])
                })
                .collect(),
        ),
    ));
    out.push(("components", Json::from_i(net.components as i64)));
}

fn build_network(input: &Json, srcs: &[Src]) -> R<Network> {
    let grid = grid_of(input)?;
    let ins = inputs_of(srcs);
    let clip = match param(input, "clip").and_then(Json::str) {
        None => None,
        Some(id) => Some(
            ins.iter()
                .position(|i| i.source == id && i.closed)
                .ok_or_else(|| Fail::Usage("params.clip".into()))?,
        ),
    };
    let mut naming = Vec::new();
    if let Some(list) = param(input, "naming").and_then(Json::arr) {
        for n in list {
            let id = n.str().ok_or_else(|| Fail::Usage("params.naming".into()))?;
            let p = srcs
                .iter()
                .find_map(|s| match (&s.shape, s.source == id) {
                    (Shape::Point(p), true) => Some(*p),
                    _ => None,
                })
                .ok_or_else(|| Fail::Usage("params.naming".into()))?;
            naming.push((id.to_string(), p));
        }
    }
    Ok(network(ins, clip, &naming, grid)?)
}

fn cells_params(input: &Json) -> R<CellParams> {
    Ok(CellParams {
        cell_length: param_int(input, "cell_length")?,
        band_depth: param_int(input, "band_depth")?,
        min_cell_length: param_int(input, "min_cell_length")?,
        min_cell_area: param(input, "min_cell_area")
            .and_then(Json::int128)
            .ok_or_else(|| Fail::Usage("params.min_cell_area".into()))?,
    })
}

/// Runs cells and returns the network (if any), the cells, and the name of the divided region.
fn run_cells(
    input: &Json,
    srcs: &[Src],
    diags: &mut Vec<GeomDiag>,
) -> R<(Option<Network>, crate::cells::Cells, String)> {
    let params = cells_params(input)?;
    crate::cells::check_params(&params)?;
    let grid = grid_of(input)?;
    if let Some(rid) = param(input, "region").and_then(Json::str) {
        let s = srcs
            .iter()
            .find(|s| s.source == rid)
            .ok_or_else(|| Fail::Usage("params.region".into()))?;
        let reg = region_shape(s).ok_or_else(|| Fail::Usage("params.region".into()))?;
        if reg.ring_count() != 1 {
            return Err(Fail::Geom(
                GeomDiag::new(Code::ManyRings, "the region has more than one ring").source(rid),
            ));
        }
        let c = cells_on_outline(rid, &reg.pieces[0].shell, params, grid)?;
        return Ok((None, c, rid.to_string()));
    }
    let net = build_network(input, srcs)?;
    diags.extend(net.diags.iter().cloned());
    let start = point(
        param(input, "area").ok_or_else(|| Fail::Usage("params.area".into()))?,
        "params.area",
    )?;
    let ai = net
        .areas
        .iter()
        .position(|a| a.region.pieces[0].shell[0] == start)
        .ok_or_else(|| Fail::Usage("params.area".into()))?;
    let fr = param(input, "frontage").ok_or_else(|| Fail::Usage("params.frontage".into()))?;
    let mut front = Vec::new();
    match fr {
        Json::Str(s) if s == "outline" => {
            return Err(Fail::Usage("params.frontage".into()));
        }
        Json::Arr(list) => {
            for (i, f) in list.iter().enumerate() {
                let p = format!("params.frontage[{i}]");
                let sid = f
                    .get("source")
                    .and_then(Json::str)
                    .ok_or_else(|| Fail::Usage(format!("{p}.source")))?;
                let ends = points(
                    f.get("edge")
                        .ok_or_else(|| Fail::Usage(format!("{p}.edge")))?,
                    &format!("{p}.edge"),
                )?;
                let side = match f.get("side").and_then(Json::str) {
                    Some("left") => Side::Left,
                    Some("right") => Side::Right,
                    _ => return Err(Fail::Usage(format!("{p}.side"))),
                };
                let si = net
                    .noded
                    .sources
                    .iter()
                    .position(|s| s == sid)
                    .ok_or_else(|| Fail::Usage(format!("{p}.source")))?;
                let e = net
                    .edges
                    .iter()
                    .position(|e| {
                        e.sources.contains(&si)
                            && ends.len() == 2
                            && net.junctions[e.ends.0] == ends[0]
                            && net.junctions[e.ends.1] == ends[1]
                    })
                    .ok_or_else(|| Fail::Usage(format!("{p}.edge")))?;
                front.push(FrontEdge {
                    edge: e,
                    side,
                    source: si,
                });
            }
        }
        _ => return Err(Fail::Usage("params.frontage".into())),
    }
    let c = cells_on_area(&net, ai, &front, params, grid)?;
    let keys = network_keys(&net);
    let name = keys.areas[ai].clone();
    Ok((Some(net), c, name))
}

fn cells_json(net: Option<&Network>, c: &crate::cells::Cells, names: Option<&[String]>) -> Json {
    Json::Arr(
        c.cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let front = match (cell.front, net) {
                    (Front::Edge(fe), Some(n)) => Json::obj(vec![
                        ("source", Json::s(&n.noded.sources[fe.source])),
                        (
                            "edge",
                            j_pts(&[
                                n.junctions[n.edges[fe.edge].ends.0],
                                n.junctions[n.edges[fe.edge].ends.1],
                            ]),
                        ),
                        ("side", Json::s(fe.side.word())),
                    ]),
                    _ => Json::s("outline"),
                };
                let mut m = vec![
                    ("region", j_region(&cell.region)),
                    ("twice_area", Json::from_i128(cell.region.twice_area())),
                    ("frontage", front),
                    ("station", Json::from_i(cell.station as i64)),
                    (
                        "span",
                        Json::Arr(vec![Json::from_f(cell.span[0]), Json::from_f(cell.span[1])]),
                    ),
                    ("point", j_pt(cell.point)),
                ];
                if let Some(n) = names {
                    m.push(("name", Json::s(&n[i])));
                }
                Json::obj(m)
            })
            .collect(),
    )
}

fn single(srcs: &[Src]) -> R<&Src> {
    if srcs.len() != 1 {
        return Err(Fail::Usage("shapes".into()));
    }
    Ok(&srcs[0])
}

fn line_of(s: &Src) -> R<Line> {
    match &s.shape {
        Shape::Path(p) => Ok(Line::new(p.clone(), false)),
        Shape::Ring(r) => Ok(Line::new(r.clone(), true)),
        Shape::Region(r) if r.ring_count() == 1 => Ok(Line::new(r.pieces[0].shell.clone(), true)),
        Shape::Region(_) => Err(Fail::Geom(
            GeomDiag::new(Code::ManyRings, "a region of more than one ring has no t")
                .source(&s.source),
        )),
        Shape::Point(_) => Err(Fail::Geom(
            GeomDiag::new(Code::NoLength, "a point has no length").source(&s.source),
        )),
    }
}

fn span_of(input: &Json) -> R<(f64, f64)> {
    let s = param(input, "span")
        .and_then(Json::arr)
        .ok_or_else(|| Fail::Usage("params.span".into()))?;
    if s.len() != 2 {
        return Err(Fail::Usage("params.span".into()));
    }
    let a = s[0]
        .double()
        .ok_or_else(|| Fail::Usage("params.span[0]".into()))?;
    let b = s[1]
        .double()
        .ok_or_else(|| Fail::Usage("params.span[1]".into()))?;
    Ok((a, b))
}

fn length_m(j: &Json) -> Option<i64> {
    j.num_text().and_then(|t| via_num::parse_length(t).ok())
}

fn point_m(j: &Json) -> Option<P> {
    let a = j.arr()?;
    if a.len() != 2 {
        return None;
    }
    Some(P::new(length_m(&a[0])?, length_m(&a[1])?))
}

/// One produced thing that can be selected by place.
struct Cand {
    name: String,
    class: &'static str,
    region: Option<Region>,
    point: P,
}

fn near_select<'a>(cands: &'a [Cand], class: &str, near: P) -> Option<&'a Cand> {
    let mut best: Option<(&Cand, i128)> = None;
    for c in cands.iter().filter(|c| c.class == class) {
        let d: i128 = match &c.region {
            Some(r) if r.side(near) != crate::pred::Side::Out => 0,
            Some(r) => {
                // Squared distance to the outline, compared as a double-free integer bound: the
                // nearest vertex distance is enough to rank regions that do not hold the point.
                r.rings()
                    .iter()
                    .flat_map(|ring| ring.iter())
                    .map(|q| {
                        let (dx, dy) = (i128::from(q.x - near.x), i128::from(q.y - near.y));
                        dx * dx + dy * dy
                    })
                    .min()
                    .unwrap_or(i128::MAX)
            }
            None => {
                let (dx, dy) = (
                    i128::from(c.point.x - near.x),
                    i128::from(c.point.y - near.y),
                );
                dx * dx + dy * dy
            }
        };
        let better = match best {
            None => true,
            Some((b, bd)) => d < bd || (d == bd && c.name.as_bytes() < b.name.as_bytes()),
        };
        if better {
            best = Some((c, d));
        }
    }
    best.map(|b| b.0)
}

fn run_names(
    input: &Json,
    srcs: &[Src],
    diags: &mut Vec<GeomDiag>,
) -> R<Vec<(&'static str, Json)>> {
    let of = param(input, "of")
        .and_then(Json::str)
        .ok_or_else(|| Fail::Usage("params.of".into()))?;
    let producer = param(input, "producer")
        .and_then(Json::str)
        .ok_or_else(|| Fail::Usage("params.producer".into()))?
        .to_string();
    let mut out: Vec<(&'static str, Json)> = Vec::new();
    let mut cands: Vec<Cand> = Vec::new();
    let qual = |k: &str| format!("{producer}#{k}");
    match of {
        "network" => {
            let net = build_network(input, srcs)?;
            diags.extend(net.diags.iter().cloned());
            let keys = network_keys(&net);
            out.extend(names_network_members(&net, &keys, &qual, &mut cands));
        }
        "cells" => {
            let (net, c, divided) = run_cells(input, srcs, diags)?;
            diags.extend(c.diags.iter().cloned());
            let names: Vec<String> = match &net {
                Some(n) => {
                    let keys = network_keys(n);
                    out.extend(names_network_members(n, &keys, &qual, &mut cands));
                    cell_keys(n, &keys, &c)
                        .into_iter()
                        .map(|k| qual(&k))
                        .collect()
                }
                None => c
                    .cells
                    .iter()
                    .map(|x| qual(&format!("c_outline_{}", x.station)))
                    .collect(),
            };
            for (i, cell) in c.cells.iter().enumerate() {
                cands.push(Cand {
                    name: names[i].clone(),
                    class: "cell",
                    region: Some(cell.region.clone()),
                    point: cell.point,
                });
            }
            out.push(("cells", cells_json(net.as_ref(), &c, Some(&names))));
            let grid = grid_of(input)?;
            let rest_points: Vec<P> = c
                .rest
                .pieces
                .iter()
                .map(|p| {
                    crate::network::representative(
                        &Region {
                            pieces: vec![p.clone()],
                        },
                        grid,
                    )
                })
                .collect();
            let rk = rest_keys(&qual(&divided), &rest_points);
            for (i, p) in c.rest.pieces.iter().enumerate() {
                cands.push(Cand {
                    name: qual(&rk[i]),
                    class: "rest",
                    region: Some(Region {
                        pieces: vec![p.clone()],
                    }),
                    point: rest_points[i],
                });
            }
        }
        _ => return Err(Fail::Usage("params.of".into())),
    }
    // Instances, footprints, edits.
    let mut instances = Vec::new();
    if let Some(list) = input.get("instances").and_then(Json::arr) {
        for (i, x) in list.iter().enumerate() {
            let p = format!("instances[{i}]");
            instances.push(Instance {
                name: x
                    .get("name")
                    .and_then(Json::str)
                    .ok_or_else(|| Fail::Usage(format!("{p}.name")))?
                    .to_string(),
                producer: x
                    .get("producer")
                    .and_then(Json::str)
                    .ok_or_else(|| Fail::Usage(format!("{p}.producer")))?
                    .to_string(),
                asset: x.get("asset").and_then(Json::str).unwrap_or("").to_string(),
                origin: point(
                    x.get("origin")
                        .ok_or_else(|| Fail::Usage(format!("{p}.origin")))?,
                    &format!("{p}.origin"),
                )?,
                face: x.get("face").and_then(Json::int).unwrap_or(0),
                footprint: region_of(
                    x.get("footprint")
                        .ok_or_else(|| Fail::Usage(format!("{p}.footprint")))?,
                    &format!("{p}.footprint"),
                )?,
            });
        }
    }
    let mut footprints: BTreeMap<String, Region> = BTreeMap::new();
    if let Some(Json::Obj(m)) = input.get("footprints") {
        for (k, v) in m {
            footprints.insert(k.clone(), region_of(v, &format!("footprints.{k}"))?);
        }
    }
    let edits = input.get("edits");
    let by_of = |t: &Json| -> Option<Vec<String>> {
        t.get("by").and_then(Json::arr).map(|a| {
            a.iter()
                .filter_map(|s| s.str().map(str::to_string))
                .collect()
        })
    };
    let resolve_on = |on: &Json, path: &str, diags: &mut Vec<GeomDiag>| -> R<Option<&Cand>> {
        let class = on
            .get("class")
            .and_then(Json::str)
            .ok_or_else(|| Fail::Usage(format!("{path}.class")))?;
        let near = point_m(
            on.get("near")
                .ok_or_else(|| Fail::Usage(format!("{path}.near")))?,
        )
        .ok_or_else(|| Fail::Usage(format!("{path}.near")))?;
        let found = near_select(&cands, class, near);
        if found.is_none() {
            diags.push(GeomDiag::new(Code::NearNone, format!("the maker has no {class}")).at(near));
        }
        Ok(found)
    };
    let mut clears = Vec::new();
    if let Some(Json::Obj(m)) = edits.and_then(|e| e.get("clear")) {
        for (id, t) in m {
            let path = format!("edits.clear.{id}");
            let place = if let Some(on) = t.get("on") {
                let Some(c) = resolve_on(on, &format!("{path}.on"), diags)? else {
                    continue;
                };
                match (t.get("radius").and_then(length_m), &c.region) {
                    (Some(r), _) => ClearPlace::Disc(c.point, r),
                    (None, Some(reg)) => ClearPlace::Inside(reg.clone()),
                    (None, None) => return Err(Fail::Usage(format!("{path}.radius"))),
                }
            } else if let Some(at) = t.get("at") {
                let p = point_m(at).ok_or_else(|| Fail::Usage(format!("{path}.at")))?;
                let r = t
                    .get("radius")
                    .and_then(length_m)
                    .ok_or_else(|| Fail::Usage(format!("{path}.radius")))?;
                ClearPlace::Disc(p, r)
            } else {
                return Err(Fail::Usage(format!("{path}.at")));
            };
            clears.push(Clear {
                id: id.clone(),
                place,
                by: by_of(t),
            });
        }
    }
    let mut pins = Vec::new();
    if let Some(Json::Obj(m)) = edits.and_then(|e| e.get("pin")) {
        for (id, t) in m {
            let path = format!("edits.pin.{id}");
            let asset = t
                .get("asset")
                .and_then(Json::str)
                .ok_or_else(|| Fail::Usage(format!("{path}.asset")))?;
            let origin = if let Some(at) = t.get("at") {
                point_m(at).ok_or_else(|| Fail::Usage(format!("{path}.at")))?
            } else if let Some(on) = t.get("on") {
                let Some(c) = resolve_on(on, &format!("{path}.on"), diags)? else {
                    continue;
                };
                c.point
            } else {
                return Err(Fail::Usage(format!("{path}.at")));
            };
            let face = match t.get("face").and_then(Json::num_text) {
                Some(txt) => {
                    via_num::parse_facing(txt).map_err(|_| Fail::Usage(format!("{path}.face")))?
                }
                None => 0,
            };
            let local = footprints
                .get(asset)
                .ok_or_else(|| Fail::Usage(format!("footprints.{asset}")))?;
            let frame = via_num::Frame::new((origin.x, origin.y), face, 1.0)
                .map_err(|_| Fail::Usage(format!("{path}.face")))?;
            let mut pieces = Vec::new();
            for pc in &local.pieces {
                let map = |r: &[P]| -> R<Vec<P>> {
                    r.iter()
                        .map(|q| {
                            frame
                                .to_world((q.x, q.y))
                                .map(|(x, y)| P::new(x, y))
                                .map_err(|_| {
                                    Fail::Geom(GeomDiag::new(
                                        Code::Range,
                                        "a placed footprint is out of range",
                                    ))
                                })
                        })
                        .collect()
                };
                pieces.push(Piece {
                    shell: map(&pc.shell)?,
                    holes: pc.holes.iter().map(|h| map(h)).collect::<R<Vec<_>>>()?,
                });
            }
            pins.push(Pin {
                id: id.clone(),
                asset: asset.to_string(),
                origin,
                footprint: crate::shape::canonical(Region { pieces }),
                by: by_of(t),
            });
        }
    }
    let (held, hd) = hold(instances, &clears, &pins)?;
    diags.extend(hd);
    out.push((
        "held",
        Json::Arr(
            held.iter()
                .map(|i| {
                    Json::obj(vec![
                        ("name", Json::s(&i.name)),
                        ("producer", Json::s(&i.producer)),
                        ("origin", j_pt(i.origin)),
                    ])
                })
                .collect(),
        ),
    ));
    Ok(out)
}

fn names_network_members(
    net: &Network,
    keys: &crate::names::NetworkKeys,
    qual: &dyn Fn(&str) -> String,
    cands: &mut Vec<Cand>,
) -> Vec<(&'static str, Json)> {
    let mut out = Vec::new();
    out.push((
        "junctions",
        Json::Arr(
            net.junctions
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    cands.push(Cand {
                        name: qual(&keys.junctions[i]),
                        class: "junction",
                        region: None,
                        point: p,
                    });
                    Json::obj(vec![
                        ("point", j_pt(p)),
                        ("name", Json::s(&qual(&keys.junctions[i]))),
                    ])
                })
                .collect(),
        ),
    ));
    out.push((
        "edges",
        Json::Arr(
            net.edges
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let mid = e.path[e.path.len() / 2];
                    cands.push(Cand {
                        name: qual(&keys.edges[i]),
                        class: "edge",
                        region: None,
                        point: mid,
                    });
                    Json::obj(vec![
                        (
                            "ends",
                            j_pts(&[net.junctions[e.ends.0], net.junctions[e.ends.1]]),
                        ),
                        ("name", Json::s(&qual(&keys.edges[i]))),
                    ])
                })
                .collect(),
        ),
    ));
    out.push((
        "areas",
        Json::Arr(
            net.areas
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    cands.push(Cand {
                        name: qual(&keys.areas[i]),
                        class: "area",
                        region: Some(a.region.clone()),
                        point: a.point,
                    });
                    Json::obj(vec![
                        ("start", j_pt(a.region.pieces[0].shell[0])),
                        ("name", Json::s(&qual(&keys.areas[i]))),
                    ])
                })
                .collect(),
        ),
    ));
    out
}

fn run_op(op: &str, input: &Json, diags: &mut Vec<GeomDiag>) -> R<Vec<(&'static str, Json)>> {
    let mut out: Vec<(&'static str, Json)> = Vec::new();
    if op == "mask" {
        let g = grid_of(input)?;
        let vals: Vec<f64> = need(input, "values", "")?
            .arr()
            .ok_or_else(|| Fail::Usage("values".into()))?
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.double()
                    .ok_or_else(|| Fail::Usage(format!("values[{i}]")))
            })
            .collect::<R<_>>()?;
        let formula = need(input, "formula", "")?
            .str()
            .ok_or_else(|| Fail::Usage("formula".into()))?;
        let nx = ((g.extent[2] - g.extent[0]) / g.resolution + 1) as usize;
        let ny = ((g.extent[3] - g.extent[1]) / g.resolution + 1) as usize;
        let window = match param(input, "window").and_then(Json::arr) {
            None => None,
            Some(w) if w.len() == 4 => {
                let mut a = [0usize; 4];
                for (i, x) in w.iter().enumerate() {
                    a[i] = x
                        .int()
                        .ok_or_else(|| Fail::Usage(format!("params.window[{i}]")))?
                        as usize;
                }
                Some(a)
            }
            _ => return Err(Fail::Usage("params.window".into())),
        };
        let mg = MaskGrid {
            x0: g.extent[0],
            y0: g.extent[1],
            r: g.resolution,
            nx,
            ny,
        };
        let (r, d) = mask(mg, &vals, formula, window)?;
        diags.extend(d);
        out.push(("region", j_region(&r)));
        return Ok(out);
    }
    let srcs = shapes(input, diags)?;
    match op {
        "read" => {
            let s = single(&srcs)?;
            let smooth = matches!(param(input, "smooth"), Some(Json::Bool(true)));
            let shape = match &s.shape {
                Shape::Path(p) if smooth => j_pts(&crate::curve::smooth(&s.source, p)?),
                Shape::Path(p) => j_pts(p),
                Shape::Ring(r) if smooth => {
                    let _ = r;
                    return Err(Fail::Geom(
                        GeomDiag::new(Code::SmoothNotPath, "smooth is taken only with a path")
                            .source(&s.source),
                    ));
                }
                Shape::Ring(r) => j_pts(r),
                Shape::Region(r) => j_region(r),
                Shape::Point(p) => j_pt(*p),
            };
            out.push(("shape", shape));
        }
        "noding" => {
            let n = node(inputs_of(&srcs))?;
            diags.extend(overlaps(&n));
            noding_members(&n, &mut out);
        }
        "network" => {
            let net = build_network(input, &srcs)?;
            diags.extend(overlaps(&net.noded));
            diags.extend(net.diags.iter().cloned());
            network_members(&net, &mut out);
        }
        "boolean" => {
            let regs: Vec<Region> = srcs
                .iter()
                .map(|s| region_shape(s).ok_or_else(|| Fail::Usage("shapes".into())))
                .collect::<R<_>>()?;
            let o = match param(input, "op").and_then(Json::str) {
                Some("union") => BoolOp::Union,
                Some("intersection") => BoolOp::Intersection,
                Some("difference") => BoolOp::Difference,
                _ => return Err(Fail::Usage("params.op".into())),
            };
            out.push(("region", j_region(&boolean(o, &regs)?)));
        }
        "offset" => {
            let d = param_int(input, "distance")?;
            let join = join_of(input)?;
            let mut res = Vec::new();
            for s in &srcs {
                let r = region_shape(s).ok_or_else(|| Fail::Usage("shapes".into()))?;
                let (o, dd) = offset(&s.source, &r, d, join)?;
                diags.extend(dd);
                res.push(j_region(&o));
            }
            out.push(("results", Json::Arr(res)));
        }
        "stroke" => {
            let (l, r) = (param_int(input, "left")?, param_int(input, "right")?);
            let join = join_of(input)?;
            let cap = match param(input, "cap").and_then(Json::str) {
                Some("butt") => Cap::Butt,
                Some("round") => Cap::Round,
                _ => return Err(Fail::Usage("params.cap".into())),
            };
            let mut res = Vec::new();
            for s in &srcs {
                let Shape::Path(p) = &s.shape else {
                    return Err(Fail::Usage("shapes".into()));
                };
                res.push(j_region(&stroke(&s.source, p, l, r, join, cap)?));
            }
            out.push(("results", Json::Arr(res)));
        }
        "follow" => {
            let s = single(&srcs)?;
            let rf = match &s.shape {
                Shape::Path(p) => Reference::Path(p.clone()),
                Shape::Ring(r) => Reference::Ring(r.clone()),
                Shape::Region(r) if r.ring_count() == 1 => {
                    Reference::Ring(r.pieces[0].shell.clone())
                }
                Shape::Region(_) => {
                    return Err(Fail::Geom(
                        GeomDiag::new(Code::ManyRings, "a region of more than one ring")
                            .source(&s.source),
                    ))
                }
                Shape::Point(_) => {
                    return Err(Fail::Geom(
                        GeomDiag::new(Code::NoLength, "a point has no length").source(&s.source),
                    ))
                }
            };
            let (a, b) = match param(input, "span") {
                None => (0.0, 1.0),
                Some(_) => span_of(input)?,
            };
            let off = param(input, "offset").and_then(Json::int).unwrap_or(0);
            let (p, d) = follow(&s.source, &rf, a, b, off)?;
            diags.extend(d);
            out.push(("path", j_pts(&p)));
        }
        "cells" => {
            let (net, c, _) = run_cells(input, &srcs, diags)?;
            diags.extend(c.diags.iter().cloned());
            out.push(("cells", cells_json(net.as_ref(), &c, None)));
            out.push(("rest", j_region(&c.rest)));
        }
        "project" => {
            let s = single(&srcs)?;
            let pts = points(need(input, "points", "")?, "points")?;
            let mut res = Vec::new();
            for p in pts {
                let m = match &s.shape {
                    Shape::Path(v) => lineref::project_path(&Line::new(v.clone(), false), p),
                    Shape::Ring(r) => lineref::project_region(&Region::from_ring(r.clone()), p)
                        .ok_or_else(|| Fail::Usage("shapes".into()))?,
                    Shape::Region(r) => {
                        lineref::project_region(r, p).ok_or_else(|| Fail::Usage("shapes".into()))?
                    }
                    Shape::Point(q) => lineref::project_point(*q, p),
                };
                res.push(Json::obj(vec![
                    ("t", m.t.map(Json::from_f).unwrap_or(Json::Null)),
                    ("d", Json::from_f(m.d)),
                    ("sd", Json::from_f(m.sd)),
                    (
                        "nearest",
                        Json::Arr(vec![Json::from_f(m.nearest.0), Json::from_f(m.nearest.1)]),
                    ),
                ]));
            }
            out.push(("results", Json::Arr(res)));
        }
        "interpolate" => {
            let s = single(&srcs)?;
            let line = line_of(s)?;
            let ts = need(input, "t", "")?
                .arr()
                .ok_or_else(|| Fail::Usage("t".into()))?;
            let mut res = Vec::new();
            for (i, t) in ts.iter().enumerate() {
                let t = t.double().ok_or_else(|| Fail::Usage(format!("t[{i}]")))?;
                let at = lineref::interpolate(&line, &s.source, t, 0)?;
                res.push(Json::obj(vec![
                    ("point", j_pt(at.point)),
                    ("bearing", Json::from_i(at.bearing)),
                ]));
            }
            out.push(("results", Json::Arr(res)));
        }
        "substring" => {
            let s = single(&srcs)?;
            let line = line_of(s)?;
            let (a, b) = span_of(input)?;
            out.push(("path", j_pts(&lineref::substring(&line, &s.source, a, b)?)));
        }
        "reverse" => {
            let s = single(&srcs)?;
            let Shape::Path(p) = &s.shape else {
                return Err(Fail::Usage("shapes".into()));
            };
            out.push(("path", j_pts(&lineref::reverse(p))));
        }
        "triangulate" => {
            let s = single(&srcs)?;
            let r = region_shape(s).ok_or_else(|| Fail::Usage("shapes".into()))?;
            let tris = crate::triangulate::triangulate(&r)?;
            out.push((
                "triangles",
                Json::Arr(tris.iter().map(|t| j_pts(t)).collect()),
            ));
        }
        "names" => {
            out.extend(run_names(input, &srcs, diags)?);
        }
        _ => return Err(Fail::Usage("OP".into())),
    }
    Ok(out)
}

/// Runs one operation on the input file's text. `in_arg` is the index of the input file's path
/// argument after `--in`, for the records' `argument` location.
pub fn run(op: &str, input_text: &str, in_arg: u32) -> Outcome {
    if !OPS.contains(&op) {
        return usage("OP");
    }
    let Ok(input) = parse(input_text) else {
        return usage("--in");
    };
    match input.get("@format").and_then(Json::str) {
        Some(FORMAT) => {}
        _ => return usage("@format"),
    }
    let mut diags: Vec<GeomDiag> = Vec::new();
    let result = run_op(op, &input, &mut diags);
    let members = match result {
        Ok(m) => m,
        Err(Fail::Usage(m)) => return usage(m),
        Err(Fail::Geom(d)) => {
            diags.push(d);
            Vec::new()
        }
    };
    let internal = diags.iter().any(|d| d.code == Code::Internal);
    let status: u8 = if internal {
        3
    } else if has_error(&diags) {
        1
    } else {
        0
    };
    let mut repro = None;
    if internal {
        let key = blake3::hash(input_text.as_bytes()).to_hex().to_string();
        for d in diags.iter_mut().filter(|d| d.code == Code::Internal) {
            d.data
                .insert("repro".into(), via_diag::Value::text(key.clone()));
        }
        repro = Some((key, input_text.to_string()));
    }
    let records: Vec<via_diag::Record> = diags
        .iter()
        .map(|d| {
            d.to_diag(Subject::Cli, Site::Argument(in_arg))
                .emit(&NoWorld)
        })
        .collect();
    let doc = via_diag::document(&records, &[], status);
    let diag_list = parse(&doc)
        .ok()
        .and_then(|j| j.get("diagnostics").cloned())
        .unwrap_or(Json::Arr(Vec::new()));
    let mut obj: Vec<(&str, Json)> = vec![
        ("@format", Json::s(FORMAT)),
        ("op", Json::s(op)),
        ("diagnostics", diag_list),
    ];
    if status == 0 {
        obj.extend(members);
    }
    Outcome {
        status,
        output: Some(Json::obj(obj).to_text()),
        usage: None,
        repro,
    }
}
