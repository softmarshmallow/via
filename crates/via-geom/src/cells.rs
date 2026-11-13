//! Cells along a frontage (`spec/geometry.md` 3.6; ours, after the skeleton strips of Vanegas et
//! al. 2012, simplified to straight cuts).

use crate::diag::{Code, GeomDiag};
use crate::lineref::{at_length, Line};
use crate::network::{Network, PostGrid, Side};
use crate::noding::Input;
use crate::offset::{offset, stroke, Cap, Join};
use crate::overlay::{boolean, BoolOp};
use crate::pred::{dir, dot, Wide};
use crate::shape::{Piece, Region, P};
use std::collections::BTreeMap;

/// The parameters of 3.6, millimetres (area in square millimetres).
#[derive(Clone, Copy, Debug)]
pub struct CellParams {
    /// Target cell length along the frontage.
    pub cell_length: i64,
    /// Depth of the band.
    pub band_depth: i64,
    /// Shortest frontage a cell keeps without merging.
    pub min_cell_length: i64,
    /// Least area a cell keeps without merging, mm².
    pub min_cell_area: i128,
}

/// One frontage entry: an edge of the network seen from one side of one of its sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrontEdge {
    /// The edge index in the network.
    pub edge: usize,
    /// The side of the source's travel the region lies on.
    pub side: Side,
    /// The source index in the network's noding.
    pub source: usize,
}

/// What a cell fronts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Front {
    /// An edge side of a network.
    Edge(FrontEdge),
    /// The whole outline of a region feature.
    Outline,
}

/// A cell (3.6).
#[derive(Clone, Debug)]
pub struct Cell {
    /// Its region.
    pub region: Region,
    /// What it fronts.
    pub front: Front,
    /// Its station along the edge side, from 0.
    pub station: usize,
    /// The stretch it lies in, counted along the edge side from 0.
    pub stretch: usize,
    /// Its station within the stretch.
    pub station_in_stretch: usize,
    /// `[from, to]` along the edge, fractions of the edge in its source's direction.
    pub span: [f64; 2],
    /// The representative point.
    pub point: P,
    /// Its frontage length, millimetres.
    pub front_len: f64,
}

/// The result of `cells`.
#[derive(Clone, Debug)]
pub struct Cells {
    /// The cells, in 2.5's order.
    pub cells: Vec<Cell>,
    /// The region less the cells.
    pub rest: Region,
    /// Diagnostics.
    pub diags: Vec<GeomDiag>,
    /// The divided region (the area or region feature).
    pub region: Region,
    /// Twice the area of the region as noded with the band and the cuts: the cells and the rest
    /// sum to it exactly (Closure).
    pub noded_area: i128,
}

/// A frontage path: one edge side in its source's direction, with the stretches on it.
struct FrontPath {
    front: Front,
    line: Line,
    /// Stretch boundaries as arc lengths, with cell counts.
    stretches: Vec<(f64, f64, usize)>,
    /// First global station of each stretch.
    first_station: Vec<usize>,
    /// The left or right normal points into the region.
    inward_left: bool,
}

/// True when the turn from `a` to `b` is a corner: more than 30 degrees, decided exactly.
pub fn is_corner(a: (i64, i64), b: (i64, i64)) -> bool {
    let d = dot(a, b);
    if d <= 0 {
        return true;
    }
    let du = Wide::from_u128(d as u128);
    let lhs = du.mul_wide(du).mul_wide(Wide::from_u128(4));
    let rhs = Wide::from_u128(dot(a, a) as u128)
        .mul_wide(Wide::from_u128(dot(b, b) as u128))
        .mul_wide(Wide::from_u128(3));
    lhs < rhs
}

fn bad(name: &str, v: i64) -> GeomDiag {
    GeomDiag::new(Code::BadParameter, format!("`{name}` is out of range: {v}"))
        .with("parameter", name)
        .with("value", v)
}

/// Checks the parameters (3.6).
pub fn check_params(p: &CellParams) -> Result<(), GeomDiag> {
    if p.cell_length <= 0 {
        return Err(bad("cell_length", p.cell_length));
    }
    if p.band_depth <= 0 {
        return Err(bad("band_depth", p.band_depth));
    }
    if p.min_cell_length < 0 || p.min_cell_length > p.cell_length {
        return Err(bad("min_cell_length", p.min_cell_length));
    }
    if p.min_cell_area < 0 {
        return Err(bad("min_cell_area", p.min_cell_area as i64));
    }
    Ok(())
}

fn left_normal(d: (i64, i64)) -> (f64, f64) {
    let l = (dot(d, d) as f64).sqrt();
    ((-(d.1 as f64)) / l, (d.0 as f64) / l)
}

fn pt(source: &str, x: f64, y: f64) -> Result<P, GeomDiag> {
    Ok(P::new(
        crate::lineref::round(source, x)?,
        crate::lineref::round(source, y)?,
    ))
}

/// Cells on an area of a network, with frontage entries (3.6).
pub fn cells_on_area(
    net: &Network,
    area: usize,
    front: &[FrontEdge],
    params: CellParams,
    grid: PostGrid,
) -> Result<Cells, GeomDiag> {
    check_params(&params)?;
    let arr = &net.arr;
    let a = &net.areas[area];
    let region = a.region.clone();
    let c = a.cycle;
    let in_face = |h: usize| arr.face_left(h) == Some(c);
    let shell: Vec<usize> = arr.cycles[c]
        .iter()
        .copied()
        .filter(|&h| !in_face(h ^ 1))
        .collect();
    // Which shell half-edges front, and by which entry.
    let entry_of = |h: usize| -> Option<usize> {
        let e = net.edge_of_half[h];
        for p in &net.noded.segments[h / 2].prov {
            let along = h.is_multiple_of(2) != p.reversed;
            let side = if along { Side::Left } else { Side::Right };
            if let Some(i) = front
                .iter()
                .position(|f| f.edge == e && f.side == side && f.source == p.source)
            {
                return Some(i);
            }
        }
        None
    };
    let fr: Vec<Option<usize>> = shell.iter().map(|&h| entry_of(h)).collect();
    if fr.iter().all(Option::is_none) {
        return Err(GeomDiag::new(
            Code::BadParameter,
            "no frontage entry lies on the region's shell",
        )
        .with("parameter", "frontage"));
    }
    // Chains: maximal runs of fronting half-edges, cyclically.
    let n = shell.len();
    let mut chains: Vec<Vec<usize>> = Vec::new();
    if fr.iter().all(Option::is_some) {
        chains.push((0..n).collect());
    } else {
        let start = (0..n).find(|&i| fr[i].is_none()).unwrap_or(0);
        let mut cur: Vec<usize> = Vec::new();
        for s in 1..=n {
            let i = (start + s) % n;
            if fr[i].is_some() {
                cur.push(i);
            } else if !cur.is_empty() {
                chains.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            chains.push(cur);
        }
    }
    let closed_chain = chains.len() == 1 && chains[0].len() == n && fr.iter().all(Option::is_some);
    // Band.
    let src = "#band";
    let band = if closed_chain {
        let shell_region = Region {
            pieces: vec![Piece {
                shell: region.pieces[0].shell.clone(),
                holes: Vec::new(),
            }],
        };
        let (inner, _) = offset(src, &shell_region, -params.band_depth, Join::Mitre(2.0))?;
        boolean(BoolOp::Difference, &[region.clone(), inner])?
    } else {
        let mut strokes = Vec::new();
        for ch in &chains {
            let mut pts: Vec<P> = ch.iter().map(|&i| arr.org(shell[i])).collect();
            pts.push(arr.dst(shell[*ch.last().unwrap_or(&0)]));
            strokes.push(stroke(
                src,
                &pts,
                params.band_depth,
                0,
                Join::Mitre(2.0),
                Cap::Butt,
            )?);
        }
        let u = boolean(BoolOp::Union, &strokes)?;
        boolean(BoolOp::Intersection, &[u, region.clone()])?
    };
    // Frontage paths, one per entry that appears on the shell.
    let mut used: Vec<usize> = fr.iter().flatten().copied().collect();
    used.sort();
    used.dedup();
    let mut fpaths: Vec<FrontPath> = Vec::new();
    let mut diags = Vec::new();
    for &i in &used {
        let fe = front[i];
        let edge = &net.edges[fe.edge];
        // Source direction along the edge.
        let h0 = edge.half[0];
        let along = net.noded.segments[h0 / 2]
            .prov
            .iter()
            .find(|p| p.source == fe.source)
            .is_some_and(|p| h0.is_multiple_of(2) != p.reversed);
        let mut path = edge.path.clone();
        if !along {
            path.reverse();
        }
        let inward_left = fe.side == Side::Left;
        fpaths.push(front_path(
            Front::Edge(fe),
            path,
            inward_left,
            params,
            &mut diags,
        )?);
    }
    // Cuts.
    let mut inputs: Vec<Input> = Vec::new();
    for (k, r) in band.rings().into_iter().enumerate() {
        inputs.push(Input {
            source: format!("#band{k:06}"),
            pts: r.to_vec(),
            closed: true,
        });
    }
    // A cut reaches its own band's far side and stops just past it (5): never into the band of
    // another frontage across a narrow region.
    let depth = params.band_depth as f64;
    let reach = depth + CUT_PAST;
    let mut ncut = 0usize;
    for (fi, fp) in fpaths.iter().enumerate() {
        inputs.push(Input {
            source: format!("#f{fi:06}"),
            pts: fp.line.pts.clone(),
            closed: false,
        });
        for (si, &(s0, s1, cnt)) in fp.stretches.iter().enumerate() {
            let _ = si;
            if cnt == 0 {
                continue;
            }
            for k in 1..cnt {
                let s = s0 + ((s1 - s0) * (k as f64)) / (cnt as f64);
                let at = at_length(&fp.line, src, s, false, 0)?;
                let (a0, b0) = fp.line.seg(at.k);
                let nl = left_normal(dir(a0, b0));
                let nin = if fp.inward_left { nl } else { (-nl.0, -nl.1) };
                let st = pt(
                    src,
                    (at.point.x as f64) - (nin.0 * 10.0),
                    (at.point.y as f64) - (nin.1 * 10.0),
                )?;
                let en = pt(
                    src,
                    (at.point.x as f64) + (nin.0 * reach),
                    (at.point.y as f64) + (nin.1 * reach),
                )?;
                inputs.push(Input {
                    source: format!("#cut{ncut:06}"),
                    pts: vec![st, en],
                    closed: false,
                });
                ncut += 1;
            }
        }
    }
    // Bisector cuts at stretch ends inside chains: corners and changes of edge.
    for ch in &chains {
        let m = ch.len();
        let inner: Vec<usize> = if closed_chain {
            (0..m).collect()
        } else {
            (1..m).collect()
        };
        for j in inner {
            let hin = shell[ch[(j + m - 1) % m]];
            let hout = shell[ch[j]];
            let v = arr.org(hout);
            let (din, dout) = (dir(arr.org(hin), v), dir(v, arr.dst(hout)));
            let change = net.edge_of_half[hin] != net.edge_of_half[hout];
            if !(change || is_corner(din, dout)) {
                continue;
            }
            let (n1, n2) = (left_normal(din), left_normal(dout));
            let s = (n1.0 + n2.0, n1.1 + n2.1);
            let sl = ((s.0 * s.0) + (s.1 * s.1)).sqrt();
            if sl == 0.0 {
                continue;
            }
            // To the mitre point of the two sides' far lines, `2 band_depth / |n1 + n2|`, at most
            // the mitre limit's `2 band_depth`.
            let to_mitre = via_num::min((2.0 * depth) / sl, 2.0 * depth) + CUT_PAST;
            let en = pt(
                src,
                (v.x as f64) + ((s.0 / sl) * to_mitre),
                (v.y as f64) + ((s.1 / sl) * to_mitre),
            )?;
            inputs.push(Input {
                source: format!("#cut{ncut:06}"),
                pts: vec![v, en],
                closed: false,
            });
            ncut += 1;
        }
    }
    // The far side of each chain's band: its frontage offset inward by `band_depth`, mitre limit
    // 2, as the band's own outline runs there (2). It closes each cell where bands of two
    // frontages meet across a region narrower than twice the depth; what lies past it borders no
    // frontage and is `rest`.
    for (k, ch) in chains.iter().enumerate() {
        let mut pts: Vec<P> = ch.iter().map(|&i| arr.org(shell[i])).collect();
        if !closed_chain {
            pts.push(arr.dst(shell[*ch.last().unwrap_or(&0)]));
        }
        pts.dedup();
        if pts.len() < 2 {
            continue;
        }
        let far_pts = far_line(src, &pts, closed_chain, depth)?;
        inputs.push(Input {
            source: format!("#cut_far{k:06}"),
            pts: far_pts,
            closed: false,
        });
    }
    // The region's own outline joins the band and the cuts, so the areas partition the noded
    // region: the cells, and the rest. Segments of cuts outside the band are dropped.
    for (k, r) in region.rings().into_iter().enumerate() {
        inputs.push(Input {
            source: format!("#region{k:06}"),
            pts: r.to_vec(),
            closed: true,
        });
    }
    let mut noded = crate::noding::node(inputs)?;
    {
        let srcs = noded.sources.clone();
        noded.segments.retain(|sg| {
            let m = (
                i128::from(sg.a.x) + i128::from(sg.b.x),
                i128::from(sg.a.y) + i128::from(sg.b.y),
            );
            // Outline segments are kept as noded (snapping may move them off the exact outline);
            // a segment of cuts alone is kept only inside the band.
            let cut_only = sg
                .prov
                .iter()
                .all(|p| srcs[p.source].starts_with("#cut") || srcs[p.source].starts_with("#f"));
            !cut_only || band.side_scaled(m, 2) != crate::pred::Side::Out
        });
        let mut vs: std::collections::BTreeSet<P> = std::collections::BTreeSet::new();
        for sg in &noded.segments {
            vs.insert(sg.a);
            vs.insert(sg.b);
        }
        noded.vertices = vs.into_iter().collect();
    }
    let cnet = crate::network::build(noded, &[], grid)?;
    if std::env::var("VIA_GEOM_DEBUG").is_ok() {
        let all: i128 = cnet.areas.iter().map(|a| a.region.twice_area()).sum();
        eprintln!(
            "cnet areas {} sum {} region {} band {} comps {}",
            cnet.areas.len(),
            all,
            region.twice_area(),
            band.twice_area(),
            cnet.components
        );
        for a in &cnet.areas {
            eprintln!(
                "  area {} pt {:?} side {:?}",
                a.region.twice_area(),
                a.point,
                region.side(a.point)
            );
        }
    }
    // Areas inside a hole of the region are not part of it.
    let inside: Vec<bool> = cnet
        .areas
        .iter()
        .map(|a| region.side(a.point) != crate::pred::Side::Out)
        .collect();
    let noded_area: i128 = cnet
        .areas
        .iter()
        .zip(&inside)
        .filter(|(_, &i)| i)
        .map(|(a, _)| a.region.twice_area())
        .sum();
    // Assign areas of the cell network to stations.
    let fsrc: BTreeMap<usize, usize> = cnet
        .noded
        .sources
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            s.strip_prefix("#f")
                .and_then(|n| n.parse::<usize>().ok())
                .map(|f| (i, f))
        })
        .collect();
    struct Proto {
        region: Region,
        borders: BTreeMap<(usize, usize), f64>,
        tspan: BTreeMap<usize, [f64; 2]>,
    }
    let mut protos: Vec<Proto> = Vec::new();
    for (ca, _) in cnet.areas.iter().zip(&inside).filter(|(_, &i)| i) {
        let cc = ca.cycle;
        let mut borders: BTreeMap<(usize, usize), f64> = BTreeMap::new();
        let mut tspan: BTreeMap<usize, [f64; 2]> = BTreeMap::new();
        for &h in &cnet.arr.cycles[cc] {
            let seg = &cnet.noded.segments[h / 2];
            for p in &seg.prov {
                if let Some(&fi) = fsrc.get(&p.source) {
                    let fp = &fpaths[fi];
                    let l = fp.line.length();
                    let tm = (p.t[0] + p.t[1]) / 2.0;
                    let sm = tm * l;
                    let Some(st) = station_at(fp, sm) else {
                        continue;
                    };
                    let len = ((p.t[1] - p.t[0]).abs()) * l;
                    *borders.entry((fi, st)).or_insert(0.0) += len;
                    let e = tspan
                        .entry(fi)
                        .or_insert([f64::INFINITY, f64::NEG_INFINITY]);
                    let (lo, hi) = if p.t[0] < p.t[1] {
                        (p.t[0], p.t[1])
                    } else {
                        (p.t[1], p.t[0])
                    };
                    if lo < e[0] {
                        e[0] = lo;
                    }
                    if hi > e[1] {
                        e[1] = hi;
                    }
                }
            }
        }
        protos.push(Proto {
            region: ca.region.clone(),
            borders,
            tspan,
        });
    }
    let mut cells: Vec<Cell> = Vec::new();
    let mut rest_parts: Vec<Region> = Vec::new();
    // Every area that takes one station is that station's one cell (6): `identity.md` names a
    // cell by (frontage, station), so a station split in two by the far line is joined again.
    let mut by_station: BTreeMap<(usize, usize), Vec<Proto>> = BTreeMap::new();
    for pr in protos {
        let Some((&key, _)) = pr.borders.iter().next() else {
            rest_parts.push(pr.region);
            continue;
        };
        if pr.borders.len() > 1 {
            diags.push(
                GeomDiag::new(
                    Code::CellMerged,
                    "a cell borders several stations; it takes the lowest",
                )
                .at(representative_or(&pr.region, grid)),
            );
        }
        by_station.entry(key).or_default().push(pr);
    }
    for ((fi, st), parts) in by_station {
        let fp = &fpaths[fi];
        let stretch = fp.first_station.iter().rposition(|&f| f <= st).unwrap_or(0);
        let mut front_len = 0.0;
        let mut span = [f64::INFINITY, f64::NEG_INFINITY];
        let mut regions = Vec::new();
        for pr in parts {
            front_len += pr.borders.values().sum::<f64>();
            if let Some(t) = pr.tspan.get(&fi) {
                span = [via_num::min(span[0], t[0]), via_num::max(span[1], t[1])];
            }
            regions.push(pr.region);
        }
        let region = if regions.len() == 1 {
            regions.pop().unwrap_or_default()
        } else {
            boolean(BoolOp::Union, &regions)?
        };
        if span[0] > span[1] {
            span = [0.0, 0.0];
        }
        cells.push(Cell {
            point: crate::network::representative(&region, grid),
            region,
            front: fp.front,
            station: st,
            stretch,
            station_in_stretch: st - fp.first_station[stretch],
            span,
            front_len,
        });
    }
    cells.sort_by(|x, y| {
        order_key(x)
            .partial_cmp(&order_key(y))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Short cells.
    let short = |c: &Cell| {
        c.front_len < params.min_cell_length as f64
            || c.region.twice_area() < 2 * params.min_cell_area
    };
    loop {
        let mut done = true;
        for i in 0..cells.len() {
            if !short(&cells[i]) {
                continue;
            }
            let same = |j: usize| cells[j].front == cells[i].front;
            let before = (0..i).rev().find(|&j| same(j));
            let after = ((i + 1)..cells.len()).find(|&j| same(j));
            let Some(target) = before.or(after) else {
                continue;
            };
            let merged = boolean(
                BoolOp::Union,
                &[cells[target].region.clone(), cells[i].region.clone()],
            )?;
            diags.push(
                GeomDiag::new(
                    Code::CellMerged,
                    "a short cell was merged into its neighbour",
                )
                .at(cells[i].point)
                .with("station", cells[i].station as i64),
            );
            let (fl, sp) = (cells[i].front_len, cells[i].span);
            let t = &mut cells[target];
            t.region = merged;
            t.front_len += fl;
            t.span = [
                via_num::min(t.span[0], sp[0]),
                via_num::max(t.span[1], sp[1]),
            ];
            t.point = crate::network::representative(&t.region, grid);
            cells.remove(i);
            done = false;
            break;
        }
        if done {
            break;
        }
    }
    for c in &cells {
        if short(c) {
            diags.push(
                GeomDiag::new(
                    Code::CellSmall,
                    "a short cell had no neighbour to merge into; kept",
                )
                .at(c.point),
            );
        }
    }
    let rest = boolean(BoolOp::Union, &rest_parts)?;
    let sum: i128 = cells.iter().map(|c| c.region.twice_area()).sum::<i128>() + rest.twice_area();
    if sum != noded_area {
        return Err(crate::overlay::internal(
            "cells",
            &format!("closure: cells and rest {sum} != noded region {noded_area}"),
        ));
    }
    Ok(Cells {
        cells,
        rest,
        diags,
        region,
        noded_area,
    })
}

/// The far side of a chain's band (2): each segment offset `depth` into the region (to the left of
/// travel), consecutive offsets meeting where their lines cross, as the mitre of the band's own
/// outline does; past the mitre limit 2 the corner is bevelled. Closed chains end where they
/// start.
fn far_line(src: &str, pts: &[P], closed: bool, depth: f64) -> Result<Vec<P>, GeomDiag> {
    let n = pts.len();
    let m = if closed { n } else { n - 1 };
    let mut normals = Vec::with_capacity(m);
    for k in 0..m {
        normals.push(left_normal(dir(pts[k], pts[(k + 1) % n])));
    }
    let off = |p: P, u: (f64, f64)| {
        pt(
            src,
            (p.x as f64) + (u.0 * depth),
            (p.y as f64) + (u.1 * depth),
        )
    };
    let mut out: Vec<P> = Vec::new();
    if !closed {
        out.push(off(pts[0], normals[0])?);
    }
    let joins = if closed { m } else { m - 1 };
    for j in 0..joins {
        let (u0, u1) = (normals[j], normals[(j + 1) % m]);
        let v = pts[(j + 1) % n];
        let s = (u0.0 + u1.0, u0.1 + u1.1);
        let s2 = (s.0 * s.0) + (s.1 * s.1);
        if 4.0 <= 4.0 * s2 {
            // Ratio 2 / |u0 + u1| at most the limit 2: the lines' crossing.
            let k = (2.0 * depth) / s2;
            out.push(pt(src, (v.x as f64) + (s.0 * k), (v.y as f64) + (s.1 * k))?);
        } else {
            out.push(off(v, u0)?);
            out.push(off(v, u1)?);
        }
    }
    if closed {
        if let Some(&first) = out.first() {
            out.push(first);
        }
    } else {
        out.push(off(pts[n - 1], normals[m - 1])?);
    }
    out.dedup();
    Ok(out)
}

/// How far a cut runs past its band's far side, millimetres: enough that a cut rounded to the
/// lattice still crosses the far line it ends at.
const CUT_PAST: f64 = 2.0;

fn representative_or(r: &Region, grid: PostGrid) -> P {
    crate::network::representative(r, grid)
}

fn order_key(c: &Cell) -> (u8, usize, u8, usize) {
    match c.front {
        Front::Edge(fe) => (
            0,
            fe.edge,
            if fe.side == Side::Left { 0 } else { 1 },
            c.station,
        ),
        Front::Outline => (1, 0, 0, c.station),
    }
}

/// The global station holding arc length `s`, or `None` in a stretch with no cell.
fn station_at(fp: &FrontPath, s: f64) -> Option<usize> {
    for (i, &(s0, s1, cnt)) in fp.stretches.iter().enumerate() {
        if s >= s0 && s <= s1 {
            if cnt == 0 {
                return None;
            }
            let f = (s - s0) / (s1 - s0) * (cnt as f64);
            let mut k = f.floor() as usize;
            if k >= cnt {
                k = cnt - 1;
            }
            return Some(fp.first_station[i] + k);
        }
    }
    None
}

fn front_path(
    front: Front,
    path: Vec<P>,
    inward_left: bool,
    params: CellParams,
    diags: &mut Vec<GeomDiag>,
) -> Result<FrontPath, GeomDiag> {
    let line = Line::new(path, false);
    // Stretch ends: corners.
    let mut cuts: Vec<f64> = vec![0.0];
    for i in 1..line.pts.len().saturating_sub(1) {
        let (a, b) = (
            dir(line.pts[i - 1], line.pts[i]),
            dir(line.pts[i], line.pts[i + 1]),
        );
        if is_corner(a, b) {
            cuts.push(line.cum[i]);
        }
    }
    cuts.push(line.length());
    let mut stretches = Vec::new();
    let mut first_station = Vec::new();
    let mut acc = 0usize;
    for w in cuts.windows(2) {
        let l = w[1] - w[0];
        let cnt = (l / (params.cell_length as f64)).floor() as usize;
        if cnt == 0 {
            let at = at_length(&line, "#band", w[0], false, 0)
                .map(|a| a.point)
                .unwrap_or(line.pts[0]);
            diags.push(
                GeomDiag::new(
                    Code::CellNone,
                    "a stretch of frontage is shorter than one cell",
                )
                .at(at),
            );
        }
        first_station.push(acc);
        stretches.push((w[0], w[1], cnt));
        acc += cnt;
    }
    Ok(FrontPath {
        front,
        line,
        stretches,
        first_station,
        inward_left,
    })
}

/// Cells along the whole outline of a region feature (frontage `outline`): the outline is made a
/// network of one edge, and its one area divided as any other.
pub fn cells_on_outline(
    source: &str,
    ring: &[P],
    params: CellParams,
    grid: PostGrid,
) -> Result<Cells, GeomDiag> {
    check_params(&params)?;
    let net = crate::network::network(
        vec![Input {
            source: source.to_string(),
            pts: ring.to_vec(),
            closed: true,
        }],
        None,
        &[],
        grid,
    )?;
    if net.areas.len() != 1 {
        return Err(
            GeomDiag::new(Code::NoLength, "the region has no single outline").source(source),
        );
    }
    let front: Vec<FrontEdge> = (0..net.edges.len())
        .map(|e| FrontEdge {
            edge: e,
            side: Side::Left,
            source: 0,
        })
        .collect();
    let mut out = cells_on_area(&net, 0, &front, params, grid)?;
    for c in &mut out.cells {
        c.front = Front::Outline;
    }
    Ok(out)
}
