//! Cells along a frontage (`spec/geometry.md` 3.6, Conformance) and the gate's "every cell
//! touches a frontage edge".

mod common;
use common::*;
use via_geom::cells::{cells_on_area, cells_on_outline, CellParams, Front, FrontEdge};
use via_geom::network::{network, Network, Side};
use via_geom::Code;

fn front_of(net: &Network, src: &str, a: (i64, i64), b: (i64, i64), side: Side) -> FrontEdge {
    let si = net.noded.sources.iter().position(|s| s == src).unwrap();
    let e = net
        .edges
        .iter()
        .position(|e| {
            net.junctions[e.ends.0] == m(a.0, a.1)
                && net.junctions[e.ends.1] == m(b.0, b.1)
                && e.sources.contains(&si)
        })
        .unwrap();
    FrontEdge {
        edge: e,
        side,
        source: si,
    }
}

pub fn first_case(net: &Network) -> Vec<FrontEdge> {
    vec![
        front_of(net, "cross_ns", (50, 0), (50, 50), Side::Left),
        front_of(net, "cross_ew", (0, 50), (50, 50), Side::Right),
    ]
}

fn params(min_area_m2: i128) -> CellParams {
    CellParams {
        cell_length: 10_000,
        band_depth: 10_000,
        min_cell_length: 5_000,
        min_cell_area: min_area_m2 * 1_000_000,
    }
}

#[test]
fn cross_ten_cells() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let c = cells_on_area(&net, 0, &first_case(&net), params(40), grid100()).unwrap();
    assert_eq!(c.cells.len(), 10);
    let mut squares = 0;
    let mut triangles = 0;
    for cell in &c.cells {
        match cell.region.twice_area() {
            200_000_000 => squares += 1,
            100_000_000 => triangles += 1,
            other => panic!("unexpected cell twice-area {other}"),
        }
    }
    assert_eq!((squares, triangles), (8, 2));
    let stations: Vec<usize> = c.cells.iter().map(|x| x.station).collect();
    let mut s = stations.clone();
    s.sort();
    assert_eq!(s, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
    assert_eq!(c.rest.twice_area(), 3_200_000_000);
    assert_eq!(
        c.rest.pieces[0].shell,
        vec![m(0, 0), m(40, 0), m(40, 40), m(0, 40)]
    );
    let sum: i128 =
        c.cells.iter().map(|x| x.region.twice_area()).sum::<i128>() + c.rest.twice_area();
    assert_eq!(sum, c.region.twice_area());
}

#[test]
fn cross_merge_corner_triangles() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let c = cells_on_area(&net, 0, &first_case(&net), params(60), grid100()).unwrap();
    assert_eq!(c.cells.len(), 8);
    let merged = c
        .diags
        .iter()
        .filter(|d| d.code == Code::CellMerged)
        .count();
    assert_eq!(merged, 2);
    for cell in &c.cells {
        if cell.station == 3 {
            assert_eq!(cell.region.twice_area(), 300_000_000);
        }
    }
}

#[test]
fn cross_local_change() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let a = cells_on_area(&net, 0, &first_case(&net), params(40), grid100()).unwrap();
    let mut moved = cross();
    moved[1].pts[0] = m(0, 52);
    let net2 = network(moved, None, &[], grid100()).unwrap();
    let fr2 = vec![
        front_of(&net2, "cross_ns", (50, 0), (50, 51), Side::Left),
        front_of(&net2, "cross_ew", (0, 52), (50, 51), Side::Right),
    ];
    let b = cells_on_area(&net2, 0, &fr2, params(40), grid100()).unwrap();
    // Only cells of the two edge sides exist, so every change is within them (Local).
    assert_eq!(b.cells.len(), a.cells.len());
}

#[test]
fn bad_parameter_and_none_and_small() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    let mut p = params(40);
    p.cell_length = 0;
    let e = cells_on_area(&net, 0, &first_case(&net), p, grid100()).unwrap_err();
    assert_eq!(e.code, Code::BadParameter);
    assert_eq!(
        e.data.get("parameter"),
        Some(&via_diag::Value::text("cell_length"))
    );
    // A 6 m frontage with cell length 10 m: none.
    let small = network(
        vec![ring("sq", &[(0, 0), (6, 0), (6, 6), (0, 6)])],
        None,
        &[],
        grid100(),
    )
    .unwrap();
    let si = 0;
    let fr = vec![FrontEdge {
        edge: 0,
        side: Side::Left,
        source: si,
    }];
    let c = cells_on_area(&small, 0, &fr, params(0), grid100()).unwrap();
    assert!(c.cells.is_empty());
    assert!(c.diags.iter().any(|d| d.code == Code::CellNone));
    // One cell of 500 m2, kept with cell_small.
    let fr = vec![front_of(&net, "cross_ns", (50, 0), (50, 50), Side::Left)];
    let p = CellParams {
        cell_length: 50_000,
        band_depth: 10_000,
        min_cell_length: 0,
        min_cell_area: 600_000_000,
    };
    let c = cells_on_area(&net, 0, &fr, p, grid100()).unwrap();
    assert_eq!(c.cells.len(), 1);
    assert_eq!(c.cells[0].region.twice_area(), 1_000_000_000);
    assert!(c.diags.iter().any(|d| d.code == Code::CellSmall));
}

#[test]
fn outline_cells() {
    let c = cells_on_outline(
        "plot_a",
        &[m(0, 0), m(40, 0), m(40, 30), m(0, 30)],
        params(0),
        grid100(),
    )
    .unwrap();
    assert!(!c.cells.is_empty());
    assert!(c.cells.iter().all(|x| x.front == Front::Outline));
    let sum: i128 =
        c.cells.iter().map(|x| x.region.twice_area()).sum::<i128>() + c.rest.twice_area();
    assert_eq!(sum, c.region.twice_area());
}

/// The gate: cells on every area of the linework, frontage every side on its shell; every cell
/// borders a frontage edge along at least the minimum, or carries `geom.cell_small`.
#[test]
fn linework_every_cell_fronts() {
    let inputs = fixture_paths("linework_a.json");
    let grid = grid_over(&inputs, 2000);
    let net = network(inputs, None, &[], grid).unwrap();
    let p = CellParams {
        cell_length: 20_000,
        band_depth: 30_000,
        min_cell_length: 8_000,
        min_cell_area: 100_000_000,
    };
    let (mut total, mut small, mut none, mut closure_ok, mut areas_done) = (0, 0, 0, 0, 0);
    let mut max_dev: i128 = 0;
    let mut failures = Vec::new();
    for ai in 0..net.areas.len() {
        // Frontage: each edge on the shell, from the side of each of its sources facing the area.
        let a = &net.areas[ai];
        let c = a.cycle;
        let mut front = Vec::new();
        for &h in &net.arr.cycles[c] {
            if net.arr.face_left(h ^ 1) == Some(c) {
                continue;
            }
            let e = net.edge_of_half[h];
            for pr in &net.noded.segments[h / 2].prov {
                let along = (h % 2 == 0) != pr.reversed;
                let fe = FrontEdge {
                    edge: e,
                    side: if along { Side::Left } else { Side::Right },
                    source: pr.source,
                };
                if !front.contains(&fe) {
                    front.push(fe);
                }
            }
        }
        match cells_on_area(&net, ai, &front, p, grid) {
            Ok(cs) => {
                areas_done += 1;
                total += cs.cells.len();
                small += cs
                    .diags
                    .iter()
                    .filter(|d| d.code == Code::CellSmall)
                    .count();
                none += cs.diags.iter().filter(|d| d.code == Code::CellNone).count();
                for cell in &cs.cells {
                    let fronts = cell.front_len >= p.min_cell_length as f64;
                    let flagged = cs
                        .diags
                        .iter()
                        .any(|d| d.code == Code::CellSmall && d.position == Some(cell.point));
                    assert!(
                        fronts || flagged,
                        "area {ai}: a cell fronts {} mm and is not flagged",
                        cell.front_len
                    );
                    assert!(
                        cell.front_len > 0.0,
                        "area {ai}: a cell does not touch its frontage"
                    );
                }
                let sum: i128 = cs.cells.iter().map(|x| x.region.twice_area()).sum::<i128>()
                    + cs.rest.twice_area();
                assert_eq!(sum, cs.noded_area, "area {ai}: closure on the noded region");
                closure_ok += 1;
                let dev = (cs.noded_area - cs.region.twice_area()).abs();
                if dev > max_dev {
                    max_dev = dev;
                }
                if dev > 1_000_000 {
                    eprintln!(
                        "DEBUG area {ai}: region {} noded {} holes {} cells {}",
                        cs.region.twice_area(),
                        cs.noded_area,
                        cs.region.pieces[0].holes.len(),
                        cs.cells.len()
                    );
                }
            }
            Err(e) => {
                eprintln!("area {ai}: {} {}", e.code.name(), e.message);
                failures.push((ai, -1));
            }
        }
    }
    eprintln!(
        "MEASURE linework cells: areas {areas_done}/{}, cells {total}, cell_small {small}, cell_none {none}, closure exact {closure_ok}, largest |noded - region| twice-area {max_dev} mm2, failures {failures:?}",
        net.areas.len()
    );
    assert_eq!(areas_done, net.areas.len());
    assert!(total > 0);
}
