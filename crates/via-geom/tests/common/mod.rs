//! Helpers shared by the kernel tests.
#![allow(dead_code)]

use via_geom::json::{parse, Json};
use via_geom::network::PostGrid;
use via_geom::noding::Input;
use via_geom::P;

pub fn m(x: i64, y: i64) -> P {
    P::new(x * 1000, y * 1000)
}

pub fn path(id: &str, pts: &[(i64, i64)]) -> Input {
    Input {
        source: id.to_string(),
        pts: pts.iter().map(|&(x, y)| m(x, y)).collect(),
        closed: false,
    }
}

pub fn ring(id: &str, pts: &[(i64, i64)]) -> Input {
    Input {
        source: id.to_string(),
        pts: pts.iter().map(|&(x, y)| m(x, y)).collect(),
        closed: true,
    }
}

/// Section 5's three guides.
pub fn cross() -> Vec<Input> {
    vec![
        ring("frame_a", &[(0, 0), (100, 0), (100, 100), (0, 100)]),
        path("cross_ew", &[(0, 50), (100, 50)]),
        path("cross_ns", &[(50, 0), (50, 100)]),
    ]
}

pub fn grid100() -> PostGrid {
    PostGrid {
        extent: [0, 0, 100_000, 100_000],
        resolution: 500,
    }
}

fn fixture(name: &str) -> Json {
    let p = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    parse(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// A fixture's paths as noding inputs.
pub fn fixture_paths(name: &str) -> Vec<Input> {
    let j = fixture(name);
    j.get("shapes")
        .unwrap()
        .arr()
        .unwrap()
        .iter()
        .map(|s| Input {
            source: s.get("source").unwrap().str().unwrap().to_string(),
            pts: s
                .get("path")
                .unwrap()
                .arr()
                .unwrap()
                .iter()
                .map(|p| {
                    let a = p.arr().unwrap();
                    P::new(a[0].int().unwrap(), a[1].int().unwrap())
                })
                .collect(),
            closed: false,
        })
        .collect()
}

/// The oracle's reference numbers.
pub fn reference() -> Json {
    let p = format!(
        "{}/../../fixtures/kernel/townland/reference.json",
        env!("CARGO_MANIFEST_DIR")
    );
    parse(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// A grid over the bounding box of some inputs, at a resolution in millimetres.
pub fn grid_over(inputs: &[Input], r: i64) -> PostGrid {
    let pts = inputs.iter().flat_map(|i| i.pts.iter().copied());
    let (lo, hi) = via_geom::shape::bbox(pts).unwrap();
    let x0 = lo.x.div_euclid(r) * r;
    let y0 = lo.y.div_euclid(r) * r;
    let x1 = (hi.x.div_euclid(r) + 1) * r;
    let y1 = (hi.y.div_euclid(r) + 1) * r;
    PostGrid {
        extent: [x0, y0, x1, y1],
        resolution: r,
    }
}

/// A small deterministic generator for property tests (SplitMix64 through via-num).
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(1);
        via_num::splitmix64(self.0)
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
}

/// The member of the reference that holds `key` (found by shape, so no test names a library word).
pub fn reference_part(key: &str) -> Json {
    match reference() {
        Json::Obj(m) => m.into_values().find(|v| v.get(key).is_some()).unwrap(),
        _ => panic!("reference.json is not an object"),
    }
}
