//! Names by place (`spec/identity.md` 2, 10; kernel cases 47 to 49): digest vectors, names that
//! hold when a distant vertex moves, a clear that removes exactly one cell's building, a pin
//! inside one.

#![allow(clippy::needless_range_loop)]
mod common;
use common::*;
use via_geom::cells::{cells_on_area, CellParams, FrontEdge};
use via_geom::held::{hold, Clear, ClearPlace, Instance, Pin};
use via_geom::json::{parse, Json};
use via_geom::names::{cell_keys, digest, hex, network_keys, piece_keys, Part, Role};
use via_geom::network::{network, Network, Side};
use via_geom::shape::canonical;
use via_geom::{Code, Piece, Region, P};

fn part(j: &Json) -> Part {
    if let Some(n) = j.get("name") {
        return Part::name(n.str().unwrap());
    }
    if let Some(i) = j.get("int") {
        return Part::Int(i.int().unwrap());
    }
    if let Some(s) = j.get("side") {
        return Part::Side(if s.str() == Some("left") {
            Side::Left
        } else {
            Side::Right
        });
    }
    if let Some(r) = j.get("role") {
        return Part::Role(match r.str().unwrap() {
            "start" => Role::Start,
            "end" => Role::End,
            _ => Role::Pass,
        });
    }
    if let Some(p) = j.get("pair") {
        let a = p.arr().unwrap();
        return Part::pair(part(&a[0]), part(&a[1]));
    }
    let s = j.get("set").unwrap().arr().unwrap();
    Part::Set(s.iter().map(part).collect())
}

#[test]
fn digest_vectors() {
    let p = format!(
        "{}/tests/fixtures/identity_digest.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let j = parse(&std::fs::read_to_string(p).unwrap()).unwrap();
    let rows = j.get("rows").unwrap().arr().unwrap();
    assert_eq!(rows.len(), 12);
    for r in rows {
        let r = r.arr().unwrap();
        let class = r[0].str().unwrap();
        let parts: Vec<Part> = r[1].arr().unwrap().iter().map(part).collect();
        assert_eq!(hex(digest(class, &parts)), r[2].str().unwrap(), "{class}");
    }
}

#[test]
fn piece_keys_by_square() {
    let k = piece_keys(&[
        P::new(70_000, 170_000),
        P::new(-1, 5),
        P::new(70_500, 170_000),
    ]);
    assert_eq!(k, vec!["p2_5_0", "pn1_0_0", "p2_5_1"]);
}

fn names(net: &Network) -> (Vec<(P, String)>, Vec<String>, Vec<String>) {
    let k = network_keys(net);
    let j = net
        .junctions
        .iter()
        .copied()
        .zip(k.junctions.iter().cloned())
        .collect();
    let mut e = k.edges.clone();
    e.sort();
    let mut a = k.areas.clone();
    a.sort();
    (j, e, a)
}

/// Case 47: one vertex of one source moved by 1 m, not a junction, no crossing added or lost.
#[test]
fn distant_vertex_keeps_names() {
    let inputs = fixture_paths("linework_a.json");
    let grid = grid_over(&inputs, 2000);
    let base = network(inputs.clone(), None, &[], grid).unwrap();
    let (bj, be, ba) = names(&base);
    let mut tried = 0;
    let mut held = 0;
    'outer: for si in 0..inputs.len() {
        let pts = &inputs[si].pts;
        for vi in 1..pts.len().saturating_sub(1) {
            if base.junctions.binary_search(&pts[vi]).is_ok() {
                continue;
            }
            let mut moved = inputs.clone();
            moved[si].pts[vi] = P::new(pts[vi].x + 1000, pts[vi].y);
            let Ok(net) = network(moved, None, &[], grid) else {
                continue;
            };
            // The case's condition: what crosses what is unchanged.
            if net.junctions.len() != base.junctions.len()
                || net.edges.len() != base.edges.len()
                || net.areas.len() != base.areas.len()
            {
                continue;
            }
            tried += 1;
            let (j, e, a) = names(&net);
            let jn: Vec<&String> = j.iter().map(|x| &x.1).collect();
            let bjn: Vec<&String> = bj.iter().map(|x| &x.1).collect();
            assert_eq!(jn, bjn, "junction names, source {si} vertex {vi}");
            assert_eq!(e, be, "edge names, source {si} vertex {vi}");
            assert_eq!(a, ba, "area names, source {si} vertex {vi}");
            held += 1;
            if tried >= 5 {
                break 'outer;
            }
        }
    }
    eprintln!("MEASURE names: {held} of {tried} moved vertices kept every junction, edge and area name ({} junctions, {} edges, {} areas)", bj.len(), be.len(), ba.len());
    assert!(tried >= 5);
    // The same with the sources in reverse order.
    let mut rev = inputs.clone();
    rev.reverse();
    let r = network(rev, None, &[], grid).unwrap();
    assert_eq!(names(&r), names(&base));
}

fn first_cells(net: &Network) -> via_geom::cells::Cells {
    let idx = |s: &str| net.noded.sources.iter().position(|x| x == s).unwrap();
    let edge = |a: P, b: P, s: usize| {
        net.edges
            .iter()
            .position(|e| {
                net.junctions[e.ends.0] == a
                    && net.junctions[e.ends.1] == b
                    && e.sources.contains(&s)
            })
            .unwrap()
    };
    let ns = idx("cross_ns");
    let ew = idx("cross_ew");
    let front = vec![
        FrontEdge {
            edge: edge(m(50, 0), m(50, 50), ns),
            side: Side::Left,
            source: ns,
        },
        FrontEdge {
            edge: edge(m(0, 50), m(50, 50), ew),
            side: Side::Right,
            source: ew,
        },
    ];
    cells_on_area(
        net,
        0,
        &front,
        CellParams {
            cell_length: 10_000,
            band_depth: 10_000,
            min_cell_length: 5_000,
            min_cell_area: 40_000_000,
        },
        grid100(),
    )
    .unwrap()
}

fn homes(net: &Network, cells: &via_geom::cells::Cells) -> (Vec<Instance>, Vec<String>) {
    let keys = network_keys(net);
    let ck = cell_keys(net, &keys, cells);
    let inst = cells
        .cells
        .iter()
        .zip(&ck)
        .map(|(c, k)| Instance {
            name: format!("homes#{k}"),
            producer: "homes".into(),
            asset: "house".into(),
            origin: c.point,
            face: 0,
            footprint: c.region.clone(),
        })
        .collect();
    (inst, ck)
}

/// Case 48: a clear on the station 2 cell of the `cross_ns` side removes exactly that cell's
/// instance; a clear on the rest is empty.
#[test]
fn clear_one_cell() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let cells = first_cells(&net);
    assert_eq!(cells.cells.len(), 10);
    let (inst, ck) = homes(&net, &cells);
    let mut unique = ck.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 10, "cell keys are unique");
    let ns = net
        .noded
        .sources
        .iter()
        .position(|x| x == "cross_ns")
        .unwrap();
    let target = cells
        .cells
        .iter()
        .position(|c| {
            c.station == 2 && matches!(c.front, via_geom::cells::Front::Edge(fe) if fe.source == ns)
        })
        .unwrap();
    let clear = Clear {
        id: "one".into(),
        place: ClearPlace::Inside(cells.cells[target].region.clone()),
        by: None,
    };
    let (held, diags) = hold(inst.clone(), &[clear], &[]).unwrap();
    assert_eq!(held.len(), 9);
    assert!(!held.iter().any(|i| i.name == inst[target].name));
    let kept: Vec<&String> = held.iter().map(|i| &i.name).collect();
    for (i, x) in inst.iter().enumerate() {
        if i != target {
            assert!(kept.contains(&&x.name));
        }
    }
    assert!(diags.is_empty());
    let rest = Clear {
        id: "rest".into(),
        place: ClearPlace::Inside(cells.rest.clone()),
        by: None,
    };
    let (held, diags) = hold(inst, &[rest], &[]).unwrap();
    assert_eq!(held.len(), 10);
    assert!(diags.iter().any(|d| d.code == Code::ClearEmpty));
}

/// Case 49: a pin inside the station 2 cell's instance.
#[test]
fn pin_in_cell() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let cells = first_cells(&net);
    let (inst, _) = homes(&net, &cells);
    let ns = net
        .noded
        .sources
        .iter()
        .position(|x| x == "cross_ns")
        .unwrap();
    let target = cells
        .cells
        .iter()
        .position(|c| {
            c.station == 2 && matches!(c.front, via_geom::cells::Front::Edge(fe) if fe.source == ns)
        })
        .unwrap();
    let o = cells.cells[target].point;
    let fp = canonical(Region {
        pieces: vec![Piece {
            shell: vec![
                P::new(o.x - 1000, o.y - 1000),
                P::new(o.x + 1000, o.y - 1000),
                P::new(o.x + 1000, o.y + 1000),
                P::new(o.x - 1000, o.y + 1000),
            ],
            holes: vec![],
        }],
    });
    let pin = |by: Option<Vec<String>>| Pin {
        id: "well".into(),
        asset: "well".into(),
        origin: o,
        footprint: fp.clone(),
        by,
    };
    let (held, diags) = hold(inst.clone(), &[], &[pin(None)]).unwrap();
    assert_eq!(held.len(), 9);
    let removed = diags.iter().find(|d| d.code == Code::PinRemoved).unwrap();
    assert_eq!(removed.data.get("count"), Some(&via_diag::Value::Int(1)));
    let (held, diags) = hold(inst, &[], &[pin(Some(vec!["trees_other".into()]))]).unwrap();
    assert_eq!(held.len(), 10);
    assert!(diags.iter().any(|d| d.code == Code::PinOverlap));
    assert!(!diags.iter().any(|d| d.code == Code::PinRemoved));
}
