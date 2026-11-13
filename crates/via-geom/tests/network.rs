//! Network and noding cases of `spec/geometry.md` (Conformance) and the Track A gate: faces of
//! the linework close and count; noding is idempotent and order-free.

mod common;
use common::*;
use via_geom::network::{network, Side};
use via_geom::noding::{is_noded, node, Input};
use via_geom::{Code, P};

fn sides(net: &via_geom::network::Network, a: usize) -> Vec<(String, Side)> {
    net.areas[a]
        .sides
        .iter()
        .map(|&(s, side)| (net.noded.sources[s].clone(), side))
        .collect()
}

#[test]
fn cross_counts_sides_points() {
    let net = network(cross(), None, &[], grid100()).unwrap();
    assert_eq!(net.noded.vertices.len(), 9);
    assert_eq!(net.noded.segments.len(), 12);
    assert_eq!(net.junctions.len(), 5);
    assert_eq!(net.edges.len(), 8);
    assert_eq!(net.areas.len(), 4);
    assert_eq!(net.components, 1);
    let starts: Vec<P> = net
        .areas
        .iter()
        .map(|a| a.region.pieces[0].shell[0])
        .collect();
    assert_eq!(starts, vec![m(0, 0), m(0, 50), m(50, 0), m(50, 50)]);
    let points: Vec<P> = net.areas.iter().map(|a| a.point).collect();
    assert_eq!(points, vec![m(25, 25), m(25, 75), m(75, 25), m(75, 75)]);
    for a in &net.areas {
        assert_eq!(a.region.twice_area(), 5_000_000_000);
    }
    use Side::*;
    let s = |v: &[(&str, Side)]| {
        v.iter()
            .map(|&(a, b)| (a.to_string(), b))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        sides(&net, 0),
        s(&[("cross_ew", Right), ("cross_ns", Left), ("frame_a", Left)])
    );
    assert_eq!(
        sides(&net, 1),
        s(&[("cross_ew", Left), ("cross_ns", Left), ("frame_a", Left)])
    );
    assert_eq!(
        sides(&net, 2),
        s(&[("cross_ew", Right), ("cross_ns", Right), ("frame_a", Left)])
    );
    assert_eq!(
        sides(&net, 3),
        s(&[("cross_ew", Left), ("cross_ns", Right), ("frame_a", Left)])
    );
}

#[test]
fn cross_moved_end_keeps_sides() {
    let mut inputs = cross();
    inputs[2].pts[1] = m(60, 100);
    let net = network(inputs, None, &[], grid100()).unwrap();
    let base = network(cross(), None, &[], grid100()).unwrap();
    for a in 0..4 {
        assert_eq!(sides(&net, a), sides(&base, a));
    }
}

#[test]
fn cross_pond() {
    let mut inputs = cross();
    inputs.push(ring("pond_a", &[(10, 10), (20, 10), (20, 20), (10, 20)]));
    let net = network(inputs, None, &[], grid100()).unwrap();
    assert_eq!(net.noded.vertices.len(), 13);
    assert_eq!(net.noded.segments.len(), 16);
    assert_eq!(net.junctions.len(), 6);
    assert_eq!(net.edges.len(), 9);
    assert_eq!(net.areas.len(), 5);
    assert_eq!(net.components, 2);
    assert_eq!(net.areas[0].region.twice_area(), 4_800_000_000);
    let total: i128 = net.areas.iter().map(|a| a.region.twice_area()).sum();
    // The pond's own area counts too: 4 areas of the frame sum to the frame less the pond.
    assert_eq!(total, 20_000_000_000);
    let base = network(cross(), None, &[], grid100()).unwrap();
    assert_eq!(sides(&net, 0), sides(&base, 0));
    assert_eq!(net.areas[0].holes.len(), 1);
}

#[test]
fn cross_spur_and_dead_end() {
    let mut inputs = cross();
    inputs.push(path("spur_a", &[(0, 25), (20, 25)]));
    let net = network(inputs, None, &[], grid100()).unwrap();
    let base = network(cross(), None, &[], grid100()).unwrap();
    assert_eq!(sides(&net, 0), sides(&base, 0));
    assert_eq!(net.areas[0].spurs.len(), 1);
    let dead = net.diags.iter().find(|d| d.code == Code::DeadEnd).unwrap();
    assert_eq!(dead.data.get("count"), Some(&via_diag::Value::Int(1)));
}

#[test]
fn distant_edge_leaves_first_area() {
    let mut a = cross();
    a.push(path("east_a", &[(100, 80), (130, 80)]));
    let mut b = cross();
    b.push(path("east_a", &[(100, 80), (130, 95)]));
    let na = network(a, None, &[], grid100()).unwrap();
    let nb = network(b, None, &[], grid100()).unwrap();
    assert_eq!(na.areas[0].region, nb.areas[0].region);
    assert_eq!(na.areas[0].point, nb.areas[0].point);
    assert_eq!(sides(&na, 0), sides(&nb, 0));
}

#[test]
fn no_area_warns() {
    let net = network(
        vec![
            path("a", &[(0, 0), (10, 10)]),
            path("b", &[(0, 10), (10, 0)]),
        ],
        None,
        &[],
        grid100(),
    )
    .unwrap();
    assert!(net.diags.iter().any(|d| d.code == Code::NoArea));
}

#[test]
fn duplicate_source() {
    let e = node(vec![
        path("a", &[(0, 0), (1, 1)]),
        path("a", &[(0, 1), (1, 0)]),
    ])
    .unwrap_err();
    assert_eq!(e.code, Code::DuplicateSource);
    assert_eq!(e.source.as_deref(), Some("a"));
}

fn bytes(inputs: Vec<Input>) -> String {
    let n = node(inputs).unwrap();
    let mut s = String::new();
    for seg in &n.segments {
        s.push_str(&format!("{:?}{:?}", seg.a, seg.b));
        for p in &seg.prov {
            s.push_str(&format!(
                "|{}:{}:{:?}:{}",
                n.sources[p.source],
                p.seg,
                p.t.map(f64::to_bits),
                p.reversed
            ));
        }
        s.push('\n');
    }
    s
}

#[test]
fn linework_gate() {
    let inputs = fixture_paths("linework_a.json");
    let grid = grid_over(&inputs, 2000);
    let net = network(inputs.clone(), None, &[], grid).unwrap();
    let segs: Vec<(P, P)> = net.noded.segments.iter().map(|s| (s.a, s.b)).collect();
    assert!(is_noded(&segs), "a segment crosses another");
    // Euler and closure are checked inside `network` (geom.internal otherwise); restate them.
    let (v, e, a) = (
        net.noded.vertices.len() as i64,
        segs.len() as i64,
        net.areas.len() as i64,
    );
    assert_eq!(v - e + a, net.components as i64);
    let (jv, je) = (net.junctions.len() as i64, net.edges.len() as i64);
    assert_eq!(jv - je + a, net.components as i64);
    for ar in &net.areas {
        assert!(ar.region.twice_area() > 0);
    }
    // Measures against reference.json (read, never tuned to).
    let st = common::reference_part("faces");
    let faces = st.get("faces").unwrap().int().unwrap();
    let total_ref = st.get("total_area_m2").unwrap().double().unwrap();
    let total: i128 = net.areas.iter().map(|x| x.region.twice_area()).sum();
    let total_m2 = (total as f64) / 2.0 / 1e6;
    eprintln!(
        "MEASURE linework_a: areas {} (reference faces {faces}), total {total_m2:.2} m2 (reference {total_ref}), \
         vertices {v}, segments {e}, junctions {jv}, edges {je}, components {}",
        net.areas.len(),
        net.components
    );
    let mut ours: Vec<f64> = net
        .areas
        .iter()
        .map(|x| (x.region.twice_area() as f64) / 2e6)
        .collect();
    ours.sort_by(f64::total_cmp);
    eprintln!(
        "MEASURE linework_a smallest areas m2: {:?}",
        &ours[..ours.len().min(12)]
    );
    assert!((total_m2 - total_ref).abs() / total_ref < 0.01);
}

#[test]
fn linework_noding_idempotent_and_order_free() {
    let inputs = fixture_paths("linework_a.json");
    let once = node(inputs.clone()).unwrap();
    // Noding the output again: each segment as a path of its own source.
    let again_in: Vec<Input> = once
        .segments
        .iter()
        .enumerate()
        .map(|(i, s)| Input {
            source: format!("s{i:06}"),
            pts: vec![s.a, s.b],
            closed: false,
        })
        .collect();
    let again = node(again_in).unwrap();
    let a: Vec<(P, P)> = once.segments.iter().map(|s| (s.a, s.b)).collect();
    let b: Vec<(P, P)> = again.segments.iter().map(|s| (s.a, s.b)).collect();
    assert_eq!(a, b);
    let mut rev = inputs.clone();
    rev.reverse();
    let mut rng = Rng(7);
    let mut shuf = inputs.clone();
    for i in (1..shuf.len()).rev() {
        let j = rng.range(0, i as i64) as usize;
        shuf.swap(i, j);
    }
    let base = bytes(inputs);
    assert_eq!(bytes(rev), base);
    assert_eq!(bytes(shuf), base);
}

#[test]
fn cross_shuffled_same() {
    let mut a = cross();
    a.reverse();
    assert_eq!(bytes(a), bytes(cross()));
}

/// The bound of the chosen variant (iterated snap rounding), measured: how far a noded segment's
/// ends lie from the source segment they name, and how far a source's vertices lie from the
/// noded segments.
#[test]
fn linework_noding_bound() {
    let inputs = fixture_paths("linework_a.json");
    let n = node(inputs).unwrap();
    let mut worst = 0f64;
    for s in &n.segments {
        for p in &s.prov {
            let src = &n.inputs[p.source];
            let (a, b) = (src.pts[p.seg], src.pts[(p.seg + 1) % src.pts.len()]);
            let line = via_geom::lineref::Line::new(vec![a, b], false);
            for q in [s.a, s.b] {
                let d = via_geom::lineref::project_path(&line, q).d * 1000.0;
                if d > worst {
                    worst = d;
                }
            }
        }
    }
    eprintln!("MEASURE noding bound on linework_a: {worst:.4} mm (plain snap rounding: 0.7071 mm)");
    assert!(worst <= 1.5);
}
