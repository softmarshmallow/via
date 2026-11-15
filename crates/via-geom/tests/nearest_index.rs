//! How the nearest is found (`spec/geometry.md` 3.8): a search through [`Index`] returns the
//! place the scan does, segment, foot and exact distance, ties to the lower segment, and the
//! measures bit for bit. Seeded lines: random walks, a fine comb whose teeth tie, closed rings,
//! one long segment among short ones; points inside, on vertices and far outside the box.

use via_geom::lineref::{
    nearest, project_path, project_path_in, project_region, project_region_in, region_rings, Index,
    Line,
};
use via_geom::{Region, P};

/// A small seeded generator (SplitMix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }
}

fn walk(r: &mut Rng, n: usize, step: i64) -> Vec<P> {
    let mut v = vec![P::new(r.range(0, 200_000), r.range(0, 200_000))];
    while v.len() < n {
        let last = *v.last().unwrap();
        let q = P::new(last.x + r.range(-step, step), last.y + r.range(-step, step));
        if q != last {
            v.push(q);
        }
    }
    v
}

/// Teeth one metre apart, each the same length: a point on the comb's axis is equally near many.
fn comb(teeth: i64) -> Vec<P> {
    let mut v = Vec::new();
    for i in 0..teeth {
        let x = i * 1000;
        if i % 2 == 0 {
            v.push(P::new(x, 0));
            v.push(P::new(x, 5000));
        } else {
            v.push(P::new(x, 5000));
            v.push(P::new(x, 0));
        }
    }
    v
}

fn points(r: &mut Rng, line: &Line, n: usize) -> Vec<P> {
    let mut out: Vec<P> = line.pts.clone();
    for _ in 0..n {
        out.push(P::new(r.range(-50_000, 250_000), r.range(-50_000, 250_000)));
    }
    out.push(P::new(-10_000_000, 3));
    out.push(P::new(10_000_000, -10_000_000));
    out
}

fn same(line: &Line, ps: &[P]) {
    let index = &Index::new(line);
    for &p in ps {
        let a = nearest(line, p);
        let b = index.nearest(line, p);
        assert_eq!((a.k, a.foot), (b.k, b.foot), "at {p:?}");
        assert_eq!(a.dist.cmp_exact(&b.dist), std::cmp::Ordering::Equal);
        let (ma, mb) = (project_path(line, p), project_path_in(line, index, p));
        assert_eq!(ma.t.map(f64::to_bits), mb.t.map(f64::to_bits));
        assert_eq!(ma.d.to_bits(), mb.d.to_bits());
        assert_eq!(ma.sd.to_bits(), mb.sd.to_bits());
        assert_eq!(ma.nearest.0.to_bits(), mb.nearest.0.to_bits());
        assert_eq!(ma.nearest.1.to_bits(), mb.nearest.1.to_bits());
    }
}

#[test]
fn index_equals_scan_on_walks() {
    let mut r = Rng(7);
    for (n, step) in [(20, 30_000), (200, 4_000), (2_000, 800), (500, 60_000)] {
        let line = Line::new(walk(&mut r, n, step), false);
        let ps = points(&mut r, &line, 1_500);
        same(&line, &ps);
    }
}

#[test]
fn index_ties_go_to_the_lower_segment() {
    let line = Line::new(comb(60), false);
    let mut ps: Vec<P> = (0..2_000).map(|i| P::new(i * 31 - 1_000, 2_500)).collect();
    ps.extend((0..200).map(|i| P::new(i * 300 + 500, -7_000)));
    same(&line, &ps);
    // On the axis halfway between two teeth, both are 0.5 m off: the lower one is named.
    let index = Index::new(&line);
    let n = index.nearest(&line, P::new(10_500, 2_500));
    assert_eq!(n.k, 20);
}

#[test]
fn index_with_one_long_segment() {
    let mut v = walk(&mut Rng(11), 300, 500);
    let last = *v.last().unwrap();
    v.push(P::new(last.x + 900_000, last.y + 3));
    let line = Line::new(v, false);
    let ps = points(&mut Rng(12), &line, 2_000);
    same(&line, &ps);
}

#[test]
fn index_equals_scan_on_rings() {
    // A star of 400 points round a centre: a closed ring, read with its closing segment. Each
    // point is a point of a square's outline pulled toward the centre by a seeded fraction.
    let mut r = Rng(3);
    let ring: Vec<P> = (0..400i64)
        .map(|i| {
            let (u, side) = ((i % 100) * 1_800 - 90_000, i / 100);
            let (dx, dy) = [(u, -90_000), (90_000, u), (-u, 90_000), (-90_000, -u)][side as usize];
            let f = r.range(50, 100);
            P::new(100_000 + dx * f / 100, 100_000 + dy * f / 100)
        })
        .collect();
    let line = Line::new(ring.clone(), true);
    let ps = points(&mut r, &line, 2_000);
    same(&line, &ps);
    let region = Region::from_ring(ring);
    let rings = region_rings(&region);
    for &p in &ps {
        let a = project_region(&region, p).unwrap();
        let b = project_region_in(&region, &rings, p).unwrap();
        assert_eq!(a.d.to_bits(), b.d.to_bits());
        assert_eq!(a.sd.to_bits(), b.sd.to_bits());
        assert_eq!(a.t.map(f64::to_bits), b.t.map(f64::to_bits));
    }
}
