//! Offsetting and stroking (`spec/geometry.md` 3.4, Conformance) and the gate's bank: the course
//! stroked at 8.5 m and 17 m without crossing itself.

mod common;
use common::*;
use via_geom::noding::is_noded;
use via_geom::offset::{offset, stroke, Cap, Join};
use via_geom::{Code, Region, P};

fn square(x: i64, y: i64, s: i64) -> Region {
    Region::from_ring(vec![m(x, y), m(x + s, y), m(x + s, y + s), m(x, y + s)])
}

pub fn valid(r: &Region) -> bool {
    let segs: Vec<(P, P)> = r
        .rings()
        .iter()
        .flat_map(|ring| (0..ring.len()).map(move |i| (ring[i], ring[(i + 1) % ring.len()])))
        .collect();
    is_noded(&segs) && r.pieces.iter().all(|p| p.twice_area() > 0)
}

#[test]
fn inset_concave_l() {
    // An L of 100 m arms, 40 m wide, inset 10 m.
    let l = Region::from_ring(vec![
        m(0, 0),
        m(100, 0),
        m(100, 40),
        m(40, 40),
        m(40, 100),
        m(0, 100),
    ]);
    // The inset L: arms 20 m wide, from 10 m to 90 m. At the reflex corner the offset lines meet
    // at the arms' corner (30, 30) (mitre); the bevel's chord (40, 30)-(30, 40) adds a 50 m2
    // triangle; the round join, the true inset, adds the square less a quarter disc.
    let arms = 2 * (80_000i128 * 20_000 + 60_000 * 20_000);
    let (r, _) = offset("l", &l, -10_000, Join::Mitre(2.0)).unwrap();
    assert!(valid(&r));
    assert_eq!(r.twice_area(), arms);
    let (r, _) = offset("l", &l, -10_000, Join::Bevel).unwrap();
    assert!(valid(&r));
    assert_eq!(r.twice_area(), arms + 100_000_000);
    let (r, _) = offset("l", &l, -10_000, Join::Round).unwrap();
    assert!(valid(&r));
    assert_eq!(r.pieces.len(), 1);
    let want = (arms as f64) + 2.0 * (100.0 - 25.0 * std::f64::consts::PI) * 1e6;
    // Chords stray at most 5 mm from the arc of radius 10 m: area within 5 mm × arc length.
    assert!(
        ((r.twice_area() as f64) - want).abs() < 2.0 * 5.0 * 15_708.0,
        "{}",
        r.twice_area()
    );
}

#[test]
fn mitre_limit_cuts_square() {
    let (r, _) = offset("sq", &square(0, 0, 10), 2_000, Join::Mitre(1.2)).unwrap();
    assert!(valid(&r));
    // Each corner cut square to its bisector at 2.4 m from the corner: 8 vertices.
    assert_eq!(r.pieces[0].shell.len(), 8);
    for p in &r.pieces[0].shell {
        // The cut line at the corner (10, 10): x + y = 20 m + 2.4 m * sqrt(2), within 1 mm.
        if p.x > 10_000 && p.y > 10_000 {
            let s = (p.x + p.y) as f64;
            let want = 20_000.0 + 2_400.0 * 2f64.sqrt();
            assert!((s - want).abs() <= 2.0, "{p:?}");
        }
    }
}

#[test]
fn split_and_empty() {
    // Dumbbell: two 20 m squares joined by a 4 m neck, 10 m long.
    let d = Region::from_ring(vec![
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
    ]);
    let (r, diags) = offset("db", &d, -2_500, Join::Round).unwrap();
    assert_eq!(r.pieces.len(), 2);
    let split = diags.iter().find(|x| x.code == Code::OffsetSplit).unwrap();
    assert_eq!(split.data.get("count"), Some(&via_diag::Value::Int(2)));
    let (r, diags) = offset("sq", &square(0, 0, 4), -3_000, Join::Round).unwrap();
    assert!(r.is_empty());
    assert!(diags.iter().any(|x| x.code == Code::OffsetEmpty));
}

#[test]
fn range_error() {
    let e = offset("far", &square(2_147_470, 0, 10), 20_000, Join::Round).unwrap_err();
    assert_eq!(e.code, Code::Range);
}

#[test]
fn stroke_turn_all_joins_caps() {
    let p = vec![m(0, 0), m(30, 0), m(30, 30)];
    for join in [Join::Round, Join::Bevel, Join::Mitre(2.0)] {
        for cap in [Cap::Butt, Cap::Round] {
            let r = stroke("t", &p, 3_000, 3_000, join, cap).unwrap();
            assert!(valid(&r), "{join:?} {cap:?}");
            assert_eq!(r.pieces.len(), 1);
        }
    }
    let r = stroke("t", &p, 1, 1, Join::Round, Cap::Round).unwrap();
    assert!(valid(&r));
}

/// The gate: the course offset 8.5 m and 17 m both sides, round joins and caps, without
/// self-intersection; areas as measures against reference.json.
#[test]
fn course_bank() {
    let course = &fixture_paths("course_a.json")[0].pts;
    let part = common::reference_part("offsets");
    let offs = part.get("offsets").unwrap().arr().unwrap();
    for (w, o) in [(8_500i64, &offs[0]), (17_000, &offs[1])] {
        let t = std::time::Instant::now();
        let r = stroke("course", course, w, w, Join::Round, Cap::Round).unwrap();
        let secs = t.elapsed().as_secs_f64();
        assert!(valid(&r), "self-intersection at {w} mm");
        let area = (r.twice_area() as f64) / 2e6;
        let refa = o.get("area_m2").unwrap().double().unwrap();
        eprintln!(
            "MEASURE course_a stroke {w} mm both sides: pieces {}, holes {}, vertices {}, area {area:.2} m2 \
             (reference {refa}, diff {:.2} m2, {:.4} %), {secs:.3} s",
            r.pieces.len(),
            r.pieces.iter().map(|p| p.holes.len()).sum::<usize>(),
            r.pieces.iter().map(|p| p.shell.len()).sum::<usize>(),
            area - refa,
            100.0 * (area - refa) / refa
        );
        assert_eq!(r.pieces.len(), 1);
        assert!(((area - refa) / refa).abs() < 0.005);
    }
}
