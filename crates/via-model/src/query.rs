//! The surface questions over a finished build (`surfaces.md` 2): `surfaces_at`, `sample`,
//! `relief` and `meetings`, on the terrain (M1: every surface is the terrain, R25, R31).
//!
//! The terrain is read as `fields.md` 1.2 reads a field: the grid cell by integer arithmetic, a
//! point on a line two cells share taking the cell to its east (north), the last column (row)
//! clamped; the gradient is that interpolant's, exactly (2.2). Relief is exact on the terrain
//! (2.3): the extremes of a bilinear interpolant over a region lie at its posts, its vertices,
//! where its edges cross grid lines, or where the interpolant turns along an edge inside a cell.

use via_geom::overlay::{boolean, BoolOp};
use via_geom::pred::Side;
use via_geom::{Region, P};
use via_num::Grid;
use via_raster::raster::bilinear;

use crate::build::{Build, Meeting};

/// The terrain of a build: its field's name, grid and posts.
pub struct Terrain<'b> {
    pub name: &'b str,
    grid: Grid,
    values: &'b [f32],
}

/// One answer of `sample` (2.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub inside: bool,
    pub z: f64,
    pub gx: f64,
    pub gy: f64,
}

/// One answer of `relief` (2.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relief {
    pub low: f64,
    pub high: f64,
    pub outside: f64,
}

/// A region given to `relief` that has zero area: `surface.empty_region`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyRegion {
    /// Its index in the input.
    pub index: usize,
}

impl<'b> Terrain<'b> {
    /// The build's terrain: its one surface field (`surfaces.md` 1, rule 1).
    pub fn of(b: &'b Build) -> Option<Terrain<'b>> {
        let grid = b.grid?;
        let (name, f) = b
            .fields
            .iter()
            .find(|(_, f)| f.role.as_deref() == Some("surface"))?;
        let values = f.values.as_deref()?;
        (values.len() as u64 == grid.len()).then_some(Terrain { name, grid, values })
    }

    fn post(&self, i: u64, j: u64) -> f64 {
        f64::from(self.values[(j * self.grid.nx + i) as usize])
    }

    /// `fields.md` 1.2, step 1: the grid cell along one axis of an offset `a` in millimetres.
    fn cell(&self, a: i64, n: u64) -> u64 {
        let r = self.grid.resolution;
        let c = a.div_euclid(r).max(0) as u64;
        c.min(n.saturating_sub(2))
    }

    /// Whether a lattice point lies in the bound (the extent), edges in.
    pub fn inside(&self, x: i64, y: i64) -> bool {
        let [x0, y0, x1, y1] = self.grid.extent;
        x >= x0 && x <= x1 && y >= y0 && y <= y1
    }

    /// `sample` at a lattice point (2.2): height and the interpolant's own gradient.
    pub fn sample(&self, x: i64, y: i64) -> Sample {
        if !self.inside(x, y) || self.grid.nx < 2 || self.grid.ny < 2 {
            return Sample {
                inside: false,
                z: 0.0,
                gx: 0.0,
                gy: 0.0,
            };
        }
        let [x0, y0, _, _] = self.grid.extent;
        let r = self.grid.resolution;
        let i = self.cell(x - x0, self.grid.nx);
        let j = self.cell(y - y0, self.grid.ny);
        let xi = x0 + i as i64 * r;
        let yj = y0 + j as i64 * r;
        let u = (x - xi) as f64 / r as f64;
        let v = (y - yj) as f64 / r as f64;
        let (z00, z10, z01, z11) = (
            self.post(i, j),
            self.post(i + 1, j),
            self.post(i, j + 1),
            self.post(i + 1, j + 1),
        );
        let big_r = r as f64 / 1000.0;
        let gx = ((z10 - z00) * (1.0 - v) + (z11 - z01) * v) / big_r;
        let gy = ((z01 - z00) * (1.0 - u) + (z11 - z10) * u) / big_r;
        Sample {
            inside: true,
            z: bilinear(z00, z10, z01, z11, u, v),
            gx,
            gy,
        }
    }

    /// The height at a point given in doubles of millimetres inside the extent: the cell by
    /// 1.2's rule on the point's floor, the fractions in doubles.
    fn z_at(&self, x: f64, y: f64) -> f64 {
        let [x0, y0, _, _] = self.grid.extent;
        let r = self.grid.resolution as f64;
        let ax = x - x0 as f64;
        let ay = y - y0 as f64;
        let i = ((ax / r).floor() as u64).min(self.grid.nx.saturating_sub(2));
        let j = ((ay / r).floor() as u64).min(self.grid.ny.saturating_sub(2));
        let u = (ax - i as f64 * r) / r;
        let v = (ay - j as f64 * r) / r;
        bilinear(
            self.post(i, j),
            self.post(i + 1, j),
            self.post(i, j + 1),
            self.post(i + 1, j + 1),
            u,
            v,
        )
    }

    /// `relief` over one region (2.3): exact on the terrain.
    pub fn relief(&self, region: &Region, index: usize) -> Result<Relief, EmptyRegion> {
        let area = region.twice_area();
        if area <= 0 {
            return Err(EmptyRegion { index });
        }
        let [x0, y0, x1, y1] = self.grid.extent;
        let bound = Region::from_ring(vec![
            P::new(x0, y0),
            P::new(x1, y0),
            P::new(x1, y1),
            P::new(x0, y1),
        ]);
        let inter = boolean(BoolOp::Intersection, &[region.clone(), bound]).unwrap_or_default();
        let ai = inter.twice_area();
        let outside = (area - ai) as f64 / area as f64;
        if inter.is_empty() || ai == 0 {
            return Ok(Relief {
                low: 0.0,
                high: 0.0,
                outside: 1.0,
            });
        }
        let mut vals: Vec<f64> = Vec::new();
        // Posts inside it.
        if let Some((lo, hi)) = inter.bbox() {
            if let Ok(Some(w)) =
                via_raster::window::effect_window(&self.grid, [lo.x, lo.y, hi.x, hi.y], 0)
            {
                for (i, j) in w.posts() {
                    let (x, y) = via_raster::window::post_mm(&self.grid, i, j);
                    if inter.side(P::new(x, y)) != Side::Out {
                        vals.push(self.post(i, j));
                    }
                }
            }
        }
        let r = self.grid.resolution;
        for ring in inter.rings() {
            for k in 0..ring.len() {
                let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
                // Its vertices, read at the lattice point exactly.
                vals.push(self.sample(a.x, a.y).z);
                let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
                // Crossings with grid lines, as fractions along the edge.
                let mut ts: Vec<f64> = vec![0.0, 1.0];
                let lines = |p: i64, q: i64, o: i64| -> Vec<i64> {
                    let (lo, hi) = (p.min(q), p.max(q));
                    let first = o + (lo - o).div_euclid(r) * r + r;
                    let mut v = Vec::new();
                    let mut g = first;
                    while g < hi {
                        v.push(g);
                        g += r;
                    }
                    v
                };
                if a.x != b.x {
                    for g in lines(a.x, b.x, x0) {
                        ts.push((g - a.x) as f64 / (b.x - a.x) as f64);
                    }
                }
                if a.y != b.y {
                    for g in lines(a.y, b.y, y0) {
                        ts.push((g - a.y) as f64 / (b.y - a.y) as f64);
                    }
                }
                ts.sort_by(f64::total_cmp);
                ts.dedup();
                let at = |t: f64| (a.x as f64 + t * dx, a.y as f64 + t * dy);
                for w in ts.windows(2) {
                    let (t0, t1) = (w[0], w[1]);
                    if t1 > 0.0 && t1 < 1.0 {
                        let (x, y) = at(t1);
                        vals.push(self.z_at(x, y));
                    }
                    // The turning point of the interpolant along this piece, inside one cell.
                    let (mx, my) = at((t0 + t1) / 2.0);
                    let i = ((mx - x0 as f64) / r as f64).floor() as u64;
                    let j = ((my - y0 as f64) / r as f64).floor() as u64;
                    let i = i.min(self.grid.nx.saturating_sub(2));
                    let j = j.min(self.grid.ny.saturating_sub(2));
                    let (xi, yj) = ((x0 + i as i64 * r) as f64, (y0 + j as i64 * r) as f64);
                    let rf = r as f64;
                    let (u0, v0) = ((a.x as f64 - xi) / rf, (a.y as f64 - yj) / rf);
                    let (du, dv) = (dx / rf, dy / rf);
                    let (z00, z10, z01, z11) = (
                        self.post(i, j),
                        self.post(i + 1, j),
                        self.post(i, j + 1),
                        self.post(i + 1, j + 1),
                    );
                    let k = z00 - z10 - z01 + z11;
                    let c2 = k * du * dv;
                    let c1 = (z10 - z00) * du + (z01 - z00) * dv + k * (u0 * dv + v0 * du);
                    if c2 != 0.0 {
                        let ts_ = -c1 / (2.0 * c2);
                        if ts_ > t0 && ts_ < t1 {
                            let u = u0 + ts_ * du;
                            let v = v0 + ts_ * dv;
                            vals.push(bilinear(z00, z10, z01, z11, u, v));
                        }
                    }
                }
            }
        }
        let low = via_num::minimum(&vals).unwrap_or(0.0);
        let high = via_num::maximum(&vals).unwrap_or(0.0);
        Ok(Relief { low, high, outside })
    }

    /// `surfaces_at` (2.1): one row per point inside the bound, `(point, surface, z)`.
    pub fn surfaces_at(&self, points: &[(i64, i64)]) -> Vec<(usize, &str, f64)> {
        points
            .iter()
            .enumerate()
            .filter(|(_, (x, y))| self.inside(*x, *y))
            .map(|(k, (x, y))| (k, self.name, self.sample(*x, *y).z))
            .collect()
    }
}

impl<'b> Terrain<'b> {
    /// The terrain as a view gives its posts (a step's stratum, `protocol.md` 3.3), `None` when
    /// the values do not cover the grid.
    pub fn of_values(name: &'b str, grid: Grid, values: &'b [f32]) -> Option<Terrain<'b>> {
        (values.len() as u64 == grid.len()).then_some(Terrain { name, grid, values })
    }
}

/// `sample` on a kind surface (2.2): `z` is `base` at the point; the gradient a central
/// difference of step `h_mm = max(1, floor(r_mm / 2))`, one-sided where one neighbour lies
/// outside the bound, 0 where both do. Outside the bound: `inside = false`, all 0. A `base` that
/// fails at a point asked fails the call (2.7).
pub fn sample_kind(
    s: &crate::surfaces::SurfaceOut,
    r_mm: i64,
    x: i64,
    y: i64,
) -> Result<Sample, Box<via_expr::ExprError>> {
    let Some(z) = s.z((x, y))? else {
        return Ok(Sample {
            inside: false,
            z: 0.0,
            gx: 0.0,
            gy: 0.0,
        });
    };
    let h = (r_mm / 2).max(1);
    let (hh, hh2) = (h as f64 / 1000.0, (2 * h) as f64 / 1000.0);
    let grad = |p: (i64, i64), m: (i64, i64)| -> Result<f64, Box<via_expr::ExprError>> {
        Ok(match (s.z(p)?, s.z(m)?) {
            (Some(a), Some(b)) => (a - b) / hh2,
            (Some(a), None) => (a - z) / hh,
            (None, Some(b)) => (z - b) / hh,
            (None, None) => 0.0,
        })
    };
    Ok(Sample {
        inside: true,
        z,
        gx: grad((x + h, y), (x - h, y))?,
        gy: grad((x, y + h), (x, y - h))?,
    })
}

/// `meetings()` (2.4): every meeting, ordered by `a`, then `b`, bytewise.
pub fn meetings(b: &Build) -> Vec<&Meeting> {
    let mut m: Vec<&Meeting> = b.meetings.iter().collect();
    m.sort_by(|x, y| (x.a.as_bytes(), x.b.as_bytes()).cmp(&(y.a.as_bytes(), y.b.as_bytes())));
    m
}
