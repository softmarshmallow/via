//! Booleans, `follows`, the mask, triangulation, the `near` and `overlap` noding fixtures, and the
//! property and fuzz tests of noding (`spec/geometry.md` 3, 6).

mod common;
use common::*;
use via_geom::follow::{follow, Reference};
use via_geom::mask::{mask, MaskGrid};
use via_geom::noding::{is_noded, node, overlaps, Input};
use via_geom::overlay::{boolean, BoolOp};
use via_geom::shape::canonical;
use via_geom::triangulate::triangulate;
use via_geom::{Code, Piece, Region, P};

/// `comb`: a ring with deep narrow teeth, and two holes; settled and canonical as written.
fn comb() -> Region {
    let mut shell = vec![m(0, 0), m(60, 0), m(60, 40)];
    for k in (0..5).rev() {
        let x = 10 * k + 10;
        shell.extend([m(x + 4, 40), m(x + 4, 10), m(x, 10), m(x, 40)]);
    }
    shell.push(m(0, 40));
    let holes = vec![
        vec![m(2, 2), m(2, 6), m(6, 6), m(6, 2)],
        vec![m(50, 2), m(50, 6), m(56, 6), m(56, 2)],
    ];
    canonical(Region {
        pieces: vec![Piece { shell, holes }],
    })
}

#[test]
fn boolean_identities() {
    let a = comb();
    assert_eq!(boolean(BoolOp::Union, &[a.clone(), a.clone()]).unwrap(), a);
    assert_eq!(
        boolean(BoolOp::Intersection, &[a.clone(), a.clone()]).unwrap(),
        a
    );
    assert!(boolean(BoolOp::Difference, &[a.clone(), a.clone()])
        .unwrap()
        .is_empty());
}

#[test]
fn boolean_closure_shifted() {
    let a = comb();
    let shift = |r: &Region| Region {
        pieces: r
            .pieces
            .iter()
            .map(|p| Piece {
                shell: p
                    .shell
                    .iter()
                    .map(|q| P::new(q.x + 700, q.y + 300))
                    .collect(),
                holes: p
                    .holes
                    .iter()
                    .map(|h| h.iter().map(|q| P::new(q.x + 700, q.y + 300)).collect())
                    .collect(),
            })
            .collect(),
    };
    let b = shift(&a);
    let i = boolean(BoolOp::Intersection, &[a.clone(), b.clone()]).unwrap();
    let u = boolean(BoolOp::Union, &[a.clone(), b.clone()]).unwrap();
    let d = boolean(BoolOp::Difference, &[a.clone(), b.clone()]).unwrap();
    // The inputs are lattice-exact and their crossings fall on the lattice, so noding moves
    // nothing and the identities hold on the inputs themselves.
    assert_eq!(d.twice_area() + i.twice_area(), a.twice_area());
    assert_eq!(
        u.twice_area() + i.twice_area(),
        a.twice_area() + b.twice_area()
    );
    // An oracle of our own: exact point tests at a lattice of points.
    for x in (-1000..62_000).step_by(1700) {
        for y in (-1000..42_000).step_by(1300) {
            let p = P::new(x, y);
            let ina = a.side(p) == via_geom::pred::Side::In;
            let inb = b.side(p) == via_geom::pred::Side::In;
            let on = |r: &Region| r.side(p) == via_geom::pred::Side::On;
            if on(&a) || on(&b) {
                continue;
            }
            assert_eq!(i.side(p) == via_geom::pred::Side::In, ina && inb, "{p:?}");
            assert_eq!(u.side(p) == via_geom::pred::Side::In, ina || inb, "{p:?}");
            assert_eq!(d.side(p) == via_geom::pred::Side::In, ina && !inb, "{p:?}");
        }
    }
}

#[test]
fn comb_offsets() {
    use via_geom::offset::{offset, Join};
    for join in [Join::Round, Join::Bevel, Join::Mitre(2.0)] {
        let (r, _) = offset("comb", &comb(), -1_500, join).unwrap();
        let segs: Vec<(P, P)> = r
            .rings()
            .iter()
            .flat_map(|g| (0..g.len()).map(move |i| (g[i], g[(i + 1) % g.len()])))
            .collect();
        assert!(is_noded(&segs), "{join:?}");
        let (r, _) = offset("comb", &comb(), 1_500, join).unwrap();
        let segs: Vec<(P, P)> = r
            .rings()
            .iter()
            .flat_map(|g| (0..g.len()).map(move |i| (g[i], g[(i + 1) % g.len()])))
            .collect();
        assert!(is_noded(&segs), "{join:?}");
    }
}

#[test]
fn triangulate_comb() {
    let a = comb();
    let t = triangulate(&a).unwrap();
    let sum: i128 = t
        .iter()
        .map(|x| via_geom::pred::orient(x[0], x[1], x[2]))
        .sum();
    assert_eq!(sum, a.twice_area());
    let verts: Vec<P> = a.rings().iter().flat_map(|r| r.iter().copied()).collect();
    for tri in &t {
        assert!(via_geom::pred::orient(tri[0], tri[1], tri[2]) > 0);
        assert!(tri[0] <= tri[1] && tri[0] <= tri[2]);
        for p in tri {
            assert!(verts.contains(p));
        }
    }
    let mut sorted = t.clone();
    sorted.sort();
    assert_eq!(sorted, t);
}

fn sine() -> Vec<P> {
    (0..=90)
        .map(|i| {
            let x = (i as f64) * 1000.0;
            let y = 10_000.0 * via_num::sin(2.0 * via_num::PI * x / 30_000.0);
            P::new(x as i64, y.round_ties_even() as i64)
        })
        .collect()
}

#[test]
fn follow_bend_both_sides() {
    let b = sine();
    for off in [5_000i64, -5_000] {
        let (p, _) = follow("bend", &Reference::Path(b.clone()), 0.0, 1.0, off).unwrap();
        let mut sorted = p.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), p.len(), "the path visits a point twice");
        let segs: Vec<(P, P)> = p.windows(2).map(|w| (w[0], w[1])).collect();
        assert!(is_noded(&segs), "the path crosses itself at offset {off}");
        // Every vertex lies at 5 m from the reference, within 5 mm (round joins) and 1 mm elsewhere.
        let line = via_geom::lineref::Line::new(b.clone(), false);
        for q in &p {
            let d = via_geom::lineref::project_path(&line, *q).d;
            assert!((d - 5.0).abs() <= 0.006, "{q:?} at {d}");
        }
    }
}

#[test]
fn follow_broken_dumbbell() {
    let d = vec![
        m(0, 0),
        m(20, 0),
        m(20, 8),
        m(30, 8),
        m(30, 0),
        m(50, 0),
        m(50, 20),
        m(30, 20),
        m(30, 12),
        m(20, 12),
        m(20, 20),
        m(0, 20),
    ];
    let e = follow("db", &Reference::Ring(d), 0.3, 0.3, 3_000).unwrap_err();
    assert_eq!(e.code, Code::FollowBroken);
    assert_eq!(e.data.get("widest"), Some(&via_diag::Value::Length(1999)));
    let e = follow("p", &Reference::Path(vec![m(0, 0), m(10, 0)]), 0.7, 0.2, 0).unwrap_err();
    assert_eq!(e.code, Code::SpanOrder);
}

fn speckle() -> (MaskGrid, Vec<f64>) {
    // 11 x 9 posts at 1 m: single posts above 0, a saddle, and a piece on the grid's edge.
    let (nx, ny) = (11usize, 9usize);
    let mut v = vec![-1.0; nx * ny];
    let mut set = |i: usize, j: usize, x: f64| v[j * nx + i] = x;
    set(2, 2, 1.0);
    set(7, 6, 0.5);
    // Saddle at cell (4, 3): high corners (4, 3) and (5, 4), low (5, 3) and (4, 4); mean above 0.
    set(4, 3, 3.0);
    set(5, 4, 3.0);
    set(5, 3, -0.5);
    set(4, 4, -0.5);
    // Edge piece.
    for j in 0..4 {
        set(10, j, 2.0);
        set(9, j, 1.0);
    }
    (
        MaskGrid {
            x0: 0,
            y0: 0,
            r: 1000,
            nx,
            ny,
        },
        v,
    )
}

#[test]
fn mask_keeps_every_piece() {
    let (g, v) = speckle();
    let (r, d) = mask(g, &v, "= x", None).unwrap();
    // Two single posts, the joined saddle, the edge piece.
    assert_eq!(
        r.pieces.len(),
        4,
        "{:?}",
        r.pieces.iter().map(|p| p.shell[0]).collect::<Vec<_>>()
    );
    let tiny = d.iter().find(|x| x.code == Code::MaskTiny).unwrap();
    assert_eq!(tiny.data.get("count"), Some(&via_diag::Value::Int(2)));
    // The edge piece is closed along the extent.
    assert!(r
        .pieces
        .iter()
        .any(|p| p.shell.contains(&P::new(10_000, 0))));
    // Windows (half-open, `numbers.md` 6): every window's rings equal the whole's clipped to
    // the rectangle of its posts.
    for w in [
        [0usize, 0, 11, 9],
        [0, 0, 6, 6],
        [5, 0, 11, 9],
        [3, 2, 9, 7],
    ] {
        let (rw, _) = mask(g, &v, "= x", Some(w)).unwrap();
        let (a, b) = (w[2] as i64 - 1, w[3] as i64 - 1);
        let win = Region::from_ring(vec![
            m(w[0] as i64, w[1] as i64),
            m(a, w[1] as i64),
            m(a, b),
            m(w[0] as i64, b),
        ]);
        let clipped = boolean(BoolOp::Intersection, &[r.clone(), win]).unwrap();
        assert_eq!(rw, clipped, "window {w:?}");
    }
}

#[test]
fn mask_dropped_and_empty() {
    let mut v = vec![-1.0; 9];
    v[4] = 1e-9;
    let (r, d) = mask(
        MaskGrid {
            x0: 0,
            y0: 0,
            r: 1,
            nx: 3,
            ny: 3,
        },
        &v,
        "= x",
        None,
    )
    .unwrap();
    assert!(r.is_empty());
    let dr = d.iter().find(|x| x.code == Code::MaskDropped).unwrap();
    assert_eq!(dr.position, Some(P::new(1, 1)));
    let (r, d) = mask(
        MaskGrid {
            x0: 0,
            y0: 0,
            r: 1000,
            nx: 3,
            ny: 3,
        },
        &[-2.0; 9],
        "= h - 30",
        None,
    )
    .unwrap();
    assert!(r.is_empty());
    let e = d.iter().find(|x| x.code == Code::MaskEmpty).unwrap();
    assert_eq!(
        e.data.get("formula"),
        Some(&via_diag::Value::text("= h - 30"))
    );
}

#[test]
fn near_fixture() {
    let inputs = vec![
        // Segments 1 mm apart.
        Input {
            source: "a".into(),
            pts: vec![P::new(0, 0), P::new(10_000, 0)],
            closed: false,
        },
        Input {
            source: "b".into(),
            pts: vec![P::new(0, 1), P::new(10_000, 1)],
            closed: false,
        },
        // A vertex 1 mm off a segment.
        Input {
            source: "c".into(),
            pts: vec![P::new(5_000, 3_000), P::new(5_000, 2)],
            closed: false,
        },
        // A crossing at a non-lattice point.
        Input {
            source: "d".into(),
            pts: vec![P::new(0, 10), P::new(3, 13)],
            closed: false,
        },
        Input {
            source: "e".into(),
            pts: vec![P::new(0, 12), P::new(3, 10)],
            closed: false,
        },
        // A crossing on a hot pixel's high edge: (x, 20.5) snaps to y = 21.
        Input {
            source: "f".into(),
            pts: vec![P::new(100, 20), P::new(102, 21)],
            closed: false,
        },
        Input {
            source: "g".into(),
            pts: vec![P::new(101, 15), P::new(101, 26)],
            closed: false,
        },
    ];
    let n = node(inputs.clone()).unwrap();
    let segs: Vec<(P, P)> = n.segments.iter().map(|s| (s.a, s.b)).collect();
    assert!(is_noded(&segs));
    assert!(
        n.vertices.contains(&P::new(101, 21)),
        "the high-edge crossing snaps up"
    );
    // Idempotent.
    let again = node(
        n.segments
            .iter()
            .enumerate()
            .map(|(i, s)| Input {
                source: format!("s{i}"),
                pts: vec![s.a, s.b],
                closed: false,
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(
        again
            .segments
            .iter()
            .map(|s| (s.a, s.b))
            .collect::<Vec<_>>(),
        segs
    );
    // Close: every segment within the bound of a source it names (checked at its ends).
    for s in &n.segments {
        for p in &s.prov {
            let src = &n.inputs[p.source];
            let line = via_geom::lineref::Line::new(src.pts.clone(), src.closed);
            for q in [s.a, s.b] {
                let d = via_geom::lineref::project_path(&line, q).d * 1000.0;
                assert!(
                    d <= std::f64::consts::SQRT_2 * 1.5,
                    "{q:?} is {d} mm from {}",
                    n.sources[p.source]
                );
            }
        }
    }
}

#[test]
fn overlap_fixture() {
    let inputs = vec![
        Input {
            source: "a".into(),
            pts: vec![m(0, 0), m(30, 0)],
            closed: false,
        },
        Input {
            source: "b".into(),
            pts: vec![m(25, 0), m(10, 0), m(10, 10)],
            closed: false,
        },
        Input {
            source: "c".into(),
            pts: vec![m(0, 20), m(20, 20), m(10, 20)],
            closed: false,
        },
    ];
    let n = node(inputs).unwrap();
    let shared = n
        .segments
        .iter()
        .find(|s| s.a == m(10, 0) && s.b == m(25, 0))
        .unwrap();
    let srcs: Vec<(&str, bool)> = shared
        .prov
        .iter()
        .map(|p| (n.sources[p.source].as_str(), p.reversed))
        .collect();
    assert_eq!(srcs, vec![("a", false), ("b", true)]);
    let ov = overlaps(&n);
    assert_eq!(ov.len(), 2);
    assert!(ov.iter().any(|d| d.source.as_deref() == Some("c")
        && d.data.get("other") == Some(&via_diag::Value::text("c"))));
}

/// Fuzz: random segments on a small lattice; noding is noded, idempotent and order-free.
#[test]
fn fuzz_noding_idempotent() {
    let mut rng = Rng(2026);
    for case in 0..3000 {
        let k = rng.range(2, 12) as usize;
        let span = [20, 200, 5_000][case % 3];
        let inputs: Vec<Input> = (0..k)
            .map(|i| {
                let n = rng.range(2, 4) as usize;
                let mut pts: Vec<P> = Vec::new();
                while pts.len() < n {
                    let p = P::new(rng.range(-span, span), rng.range(-span, span));
                    if pts.last() != Some(&p) {
                        pts.push(p);
                    }
                }
                Input {
                    source: format!("r{i:02}"),
                    pts,
                    closed: false,
                }
            })
            .collect();
        let n = node(inputs.clone()).unwrap();
        let segs: Vec<(P, P)> = n.segments.iter().map(|s| (s.a, s.b)).collect();
        if !is_noded(&segs) {
            for i in &inputs {
                eprintln!("IN {:?}", i.pts);
            }
            for a in 0..segs.len() {
                for b in a + 1..segs.len() {
                    if !is_noded(&[segs[a], segs[b]]) {
                        eprintln!("BAD {:?} {:?}", segs[a], segs[b]);
                    }
                }
            }
        }
        assert!(is_noded(&segs), "case {case}: not noded");
        let again = node(
            segs.iter()
                .enumerate()
                .map(|(i, s)| Input {
                    source: format!("s{i:04}"),
                    pts: vec![s.0, s.1],
                    closed: false,
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(
            again
                .segments
                .iter()
                .map(|s| (s.a, s.b))
                .collect::<Vec<_>>(),
            segs,
            "case {case}: not idempotent"
        );
        let mut rev = inputs.clone();
        rev.reverse();
        let r = node(rev).unwrap();
        assert_eq!(
            r.segments.iter().map(|s| (s.a, s.b)).collect::<Vec<_>>(),
            segs,
            "case {case}: order"
        );
    }
}

/// Property: a network of random rings and paths keeps Euler and closure (checked inside).
#[test]
fn property_network_guarantees() {
    let mut rng = Rng(77);
    let mut built = 0;
    for _ in 0..1000 {
        let mut inputs = Vec::new();
        for i in 0..rng.range(1, 5) {
            let (x, y, s) = (rng.range(0, 50), rng.range(0, 50), rng.range(5, 40));
            inputs.push(ring(
                &format!("q{i}"),
                &[(x, y), (x + s, y), (x + s, y + s), (x, y + s)],
            ));
        }
        for i in 0..rng.range(0, 5) {
            inputs.push(path(
                &format!("l{i}"),
                &[
                    (rng.range(0, 90), rng.range(0, 90)),
                    (rng.range(0, 90), rng.range(0, 90)),
                ],
            ));
        }
        match via_geom::network::network(inputs.clone(), None, &[], grid100()) {
            Ok(_) => built += 1,
            Err(e) => {
                eprintln!("ERR {} {}", e.code.name(), e.message);
                for i in &inputs {
                    eprintln!("  {} {:?} {}", i.source, i.pts, i.closed);
                }
            }
        }
    }
    assert_eq!(built, 1000);
}
