//! The local frame (`spec/geometry.md` 2.6, 3.8): the vector of 2.6 bit for bit, the region
//! cases, `on` at a vertex, the round trips, reading written shapes and `smooth`.

#![allow(clippy::type_complexity)]
mod common;
use common::*;
use via_geom::lineref::{interpolate, project_path, project_region, substring, Line};
use via_geom::shape::{read_path, read_ring};
use via_geom::{Code, Region, P};

/// The beck and lanes of 2.6's vector (named neutrally here: the core names no library noun).
fn stream_a() -> Line {
    Line::new(vec![m(150, 256), m(140, 170), m(160, 90), m(150, 0)], false)
}

#[test]
fn frame_vector() {
    let s = stream_a();
    assert_eq!(s.length(), 259595.40718138084);
    let at = interpolate(&s, "s", 0.55, 0).unwrap();
    assert_eq!(at.point, P::new(153630, 115480));
    let west = Line::new(vec![m(0, 120), at.point], false);
    let east = Line::new(vec![at.point, m(256, 140)], false);
    let rows: [(&Line, (i64, i64), f64, f64, f64); 8] = [
        (
            &s,
            (160000, 171293),
            0.347370404256366,
            19.716448566078615,
            19.716448566078615,
        ),
        (
            &s,
            (160000, 171294),
            0.31966700518731006,
            19.716689495547183,
            19.716689495547183,
        ),
        (
            &s,
            (140000, 88654),
            0.6648343886445807,
            19.729033859375637,
            -19.729033859375637,
        ),
        (
            &s,
            (140000, 88655),
            0.6375139173642725,
            19.729060418580506,
            -19.729060418580506,
        ),
        (
            &west,
            (163625, 115186),
            1.0,
            9.999323027085383,
            9.999323027085383,
        ),
        (
            &west,
            (163625, 115185),
            1.0,
            9.99935247903583,
            -9.99935247903583,
        ),
        (
            &east,
            (143905, 113150),
            0.0,
            10.000226247440605,
            -10.000226247440605,
        ),
        (
            &east,
            (143905, 113151),
            0.0,
            9.999993299997756,
            9.999993299997756,
        ),
    ];
    for (line, (x, y), t, d, sd) in rows {
        let r = project_path(line, P::new(x, y));
        assert_eq!(
            r.t.unwrap().to_bits(),
            t.to_bits(),
            "t at {x},{y}: {}",
            r.t.unwrap()
        );
        assert_eq!(r.d.to_bits(), d.to_bits(), "d at {x},{y}: {}", r.d);
        assert_eq!(r.sd.to_bits(), sd.to_bits(), "sd at {x},{y}: {}", r.sd);
    }
}

#[test]
fn region_measures() {
    let sq = Region::from_ring(vec![m(0, 0), m(10, 0), m(10, 10), m(0, 10)]);
    let r = project_region(&sq, m(5, 2)).unwrap();
    assert_eq!((r.t, r.sd, r.d), (Some(0.125), 2.0, 0.0));
    let r = project_region(&sq, m(5, -3)).unwrap();
    assert_eq!((r.sd, r.d), (-3.0, 3.0));
    let r = project_region(&sq, m(10, 5)).unwrap();
    assert_eq!((r.t, r.sd, r.d), (Some(0.375), 0.0, 0.0));
    assert!(r.sd.is_sign_positive());
    // `on` that square at t = 0.125, offset 2 m (inward, left of counter-clockwise travel).
    let line = Line::new(sq.pieces[0].shell.clone(), true);
    assert_eq!(
        interpolate(&line, "sq", 0.125, 2000).unwrap().point,
        m(5, 2)
    );
}

#[test]
fn on_at_vertex_uses_outgoing_normal() {
    let s = stream_a();
    let t = project_path(&s, m(140, 170)).t.unwrap();
    let at = interpolate(&s, "s", t, 12_000).unwrap();
    // The outgoing segment runs from (140, 170) to (160, 90); its left normal is (80, 20) / |.|.
    let l = (80f64 * 80.0 + 20.0 * 20.0).sqrt();
    let want = P::new(
        (140_000.0 + 12_000.0 * 80.0 / l).round_ties_even() as i64,
        (170_000.0 + 12_000.0 * 20.0 / l).round_ties_even() as i64,
    );
    assert_eq!(at.point, want);
    assert_eq!(at.k, 1);
    // t = 1: the last segment's normal.
    let e = interpolate(&s, "s", 1.0, 1000).unwrap();
    assert_eq!(e.k, 2);
    assert_eq!(interpolate(&s, "s", 1.5, 0).unwrap_err().code, Code::TRange);
    assert_eq!(
        interpolate(&s, "s", f64::NAN, 0).unwrap_err().code,
        Code::TRange
    );
    assert_eq!(
        substring(&s, "s", 0.7, 0.2).unwrap_err().code,
        Code::SpanOrder
    );
}

#[test]
fn round_trips() {
    let s = stream_a();
    let mut rng = Rng(11);
    for _ in 0..1000 {
        let t = (rng.next() >> 11) as f64 / (1u64 << 53) as f64;
        let p = interpolate(&s, "s", t, 0).unwrap().point;
        let t2 = project_path(&s, p).t.unwrap();
        assert!((t2 - t).abs() * s.length() <= 1.0, "t {t} -> {t2}");
    }
    for _ in 0..1000 {
        let p = P::new(rng.range(100_000, 200_000), rng.range(-10_000, 270_000));
        let r = project_path(&s, p);
        let q = interpolate(&s, "s", r.t.unwrap(), 0).unwrap().point;
        let (dx, dy) = ((q.x as f64) - r.nearest.0, (q.y as f64) - r.nearest.1);
        assert!((dx * dx + dy * dy).sqrt() <= 1.0);
    }
}

#[test]
fn reading_shapes() {
    let mut d = Vec::new();
    // Clockwise ring: stored counter-clockwise, first written point kept.
    let r = read_ring("r", &[m(0, 0), m(0, 10), m(10, 10), m(10, 0)], &mut d).unwrap();
    assert_eq!(r[0], m(0, 0));
    assert!(via_geom::pred::twice_area(&r) > 0);
    // Bow tie.
    assert!(read_ring("b", &[m(0, 0), m(10, 10), m(10, 0), m(0, 10)], &mut d).is_none());
    let e = d.pop().unwrap();
    assert_eq!((e.code, e.position), (Code::SelfCrossing, Some(m(5, 5))));
    // Zero-width spike.
    assert!(read_ring(
        "s",
        &[m(0, 0), m(10, 0), m(10, 10), m(10, 0), m(0, 5)],
        &mut d
    )
    .is_none());
    let e = d.pop().unwrap();
    assert_eq!((e.code, e.position), (Code::SelfCrossing, Some(m(10, 0))));
    // Repeats and degenerate.
    let mut d = Vec::new();
    let p = read_path("p", &[m(0, 0), m(0, 0), m(10, 0)], &mut d).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, Code::RepeatedPoint);
    assert!(read_path("p", &[m(5, 5), m(5, 5)], &mut d).is_none());
    assert_eq!(d.last().unwrap().code, Code::Degenerate);
    assert!(read_ring("r", &[m(0, 0), m(10, 0), m(20, 0)], &mut d).is_none());
    assert_eq!(d.last().unwrap().code, Code::Degenerate);
}

#[test]
fn smooth_keeps_written_points() {
    let pts = vec![m(0, 0), m(20, 10), m(40, 0)];
    let s = via_geom::curve::smooth("p", &pts).unwrap();
    for p in &pts {
        assert!(s.contains(p));
    }
    // Each span holds ceil(c / 250 mm) equal steps of the curve parameter (2.4). Equal steps of
    // the parameter are not equal lengths, so a step may pass 250 mm where the curve runs fast.
    let span_steps = |a: P, b: P| {
        let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
        ((dx * dx + dy * dy).sqrt() / 250.0).ceil() as usize
    };
    let i1 = s.iter().position(|p| *p == pts[1]).unwrap();
    assert_eq!(i1, span_steps(pts[0], pts[1]));
    assert_eq!(s.len() - 1 - i1, span_steps(pts[1], pts[2]));
    let mut longest = 0f64;
    for w in s.windows(2) {
        let (dx, dy) = ((w[1].x - w[0].x) as f64, (w[1].y - w[0].y) as f64);
        let l = (dx * dx + dy * dy).sqrt();
        if l > longest {
            longest = l;
        }
    }
    eprintln!("MEASURE smooth: longest step {longest:.1} mm");
}
