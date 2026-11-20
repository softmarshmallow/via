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
    let (held, diags) = hold(inst.clone(), std::slice::from_ref(&clear), &[]).unwrap();
    assert_eq!(held.len(), 9);
    assert!(!held.iter().any(|i| i.name == inst[target].name));
    // The removal is the clear's: the change report's cause (identity.md 5).
    let by = via_geom::held::removed_by(&[inst[target].clone()], &[clear], &[]).unwrap();
    assert_eq!(
        by,
        vec![(inst[target].name.clone(), "clear.one".to_string())]
    );
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
    let gone: Vec<_> = inst
        .iter()
        .filter(|i| !held.iter().any(|h| h.name == i.name))
        .cloned()
        .collect();
    let by = via_geom::held::removed_by(&gone, &[], &[pin(None)]).unwrap();
    assert_eq!(by.len(), 1);
    assert_eq!(by[0].1, "pin.well");
    let removed = diags.iter().find(|d| d.code == Code::PinRemoved).unwrap();
    assert_eq!(removed.data.get("count"), Some(&via_diag::Value::Int(1)));
    let (held, diags) = hold(inst.clone(), &[], &[pin(Some(vec!["trees_other".into()]))]).unwrap();
    assert_eq!(held.len(), 10);
    assert!(diags.iter().any(|d| d.code == Code::PinOverlap));
    assert!(!diags.iter().any(|d| d.code == Code::PinRemoved));
    // identity.md 4.2 (R65): a pin removes, and overlaps, only what stands on its level. On
    // another level than every instance it removes nothing and says nothing; two pins at one
    // plan point on two levels do not overlap, and on one level they do.
    let apart = via_geom::held::Levels {
        instance: &|_, _| false,
        pin: &|_, _| false,
    };
    let (held, diags) = via_geom::held::hold_on(inst.clone(), &[], &[pin(None)], &apart).unwrap();
    assert_eq!(held.len(), 10);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(
        via_geom::held::removed_by_on(&gone, &[], &[pin(None)], &apart)
            .unwrap()
            .is_empty()
    );
    let other = Pin {
        id: "well_up".into(),
        ..pin(None)
    };
    let two = [pin(None), other];
    let (_, diags) = via_geom::held::hold_on(Vec::new(), &[], &two, &apart).unwrap();
    assert!(!diags.iter().any(|d| d.code == Code::PinsOverlap));
    let (_, diags) = hold(Vec::new(), &[], &two).unwrap();
    assert!(diags.iter().any(|d| d.code == Code::PinsOverlap));
}

/// identity.md 2.3, roles by travel: where the clip cuts a source, the kept part starts or ends
/// there, so a junction on the outline is named by that source; a crossing added earlier along the
/// outline renames none of the others (case `clip_cut_role`).
#[test]
fn clip_cut_source_names_its_junction() {
    let square = ring("frame", &[(10, 10), (190, 10), (190, 90), (10, 90)]);
    let a = path("a", &[(50, 0), (50, 100)]);
    let b = path("b", &[(150, 0), (150, 100)]);
    let c = path("c", &[(30, 0), (30, 100)]);
    let keyed = |inputs: Vec<via_geom::noding::Input>| {
        let clip = inputs.len() - 1;
        let grid = grid_over(&inputs, 2000);
        let net = network(inputs, Some(clip), &[], grid).unwrap();
        names(&net).0
    };
    let base = keyed(vec![a.clone(), b.clone(), square.clone()]);
    let more = keyed(vec![c, a, b, square]);
    let pair = |n: &str, r: Role| Part::pair(Part::name(n), Part::Role(r));
    let want = |n: &str, r: Role| {
        format!(
            "j{}_0",
            hex(digest(
                "junction",
                &[Part::Set(vec![pair("#clip", Role::Pass), pair(n, r)])]
            ))
        )
    };
    let at = |js: &[(P, String)], x: i64, y: i64| {
        js.iter()
            .find(|(p, _)| *p == P::new(x, y))
            .map(|(_, k)| k.clone())
            .unwrap()
    };
    assert_eq!(at(&base, 50_000, 10_000), want("a", Role::Start));
    assert_eq!(at(&base, 50_000, 90_000), want("a", Role::End));
    assert_eq!(at(&base, 150_000, 10_000), want("b", Role::Start));
    for (x, y) in [
        (50_000, 10_000),
        (50_000, 90_000),
        (150_000, 10_000),
        (150_000, 90_000),
    ] {
        assert_eq!(at(&base, x, y), at(&more, x, y), "({x}, {y})");
    }
}
