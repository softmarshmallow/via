//! Flow routing: how much ground drains through each post (`derived.md` Open: flow accumulation).
//!
//! The ground is first filled (Priority-Flood, Barnes, Lehman and Mulla 2014, see [`crate::flood`]),
//! so every post drains to the extent's edge. Each post then sends its flow by the D-infinity method
//! (Tarboton 1997): of the eight triangular facets around it, each made of the post, one side
//! neighbour and the diagonal neighbour beside it, the facet that falls fastest gives a direction
//! as an angle, and the flow is split between that facet's two neighbours by how near the angle
//! lies to each. A post on a filled flat, where no facet falls, sends its flow to the post that
//! reached it in the flood, which leads out of the flat (Barnes 2014). Flow is spread over the land,
//! not funnelled into eight bearings, so a valley's drainage follows the valley.
//!
//! Posts are taken in the reverse of the flood's order: every post a post sends to was taken
//! earlier by the flood, so every inflow is summed before the post sends its own. A post on the
//! edge with no facet that falls inside the extent sends nothing on: the flow leaves the world.

use crate::error::RasterError;
use crate::flood::{fill, Filled, Whole};
use crate::raster::Raster;
use crate::window::Window;
use via_num::Grid;

/// The facets of Tarboton 1997, table 1, as (side neighbour, diagonal neighbour), counter-clockwise
/// from east.
const FACETS: [((i64, i64), (i64, i64)); 8] = [
    ((1, 0), (1, 1)),
    ((0, 1), (1, 1)),
    ((0, 1), (-1, 1)),
    ((-1, 0), (-1, 1)),
    ((-1, 0), (-1, -1)),
    ((0, -1), (-1, -1)),
    ((0, -1), (1, -1)),
    ((1, 0), (1, -1)),
];

/// Where one post sends its flow: up to two posts and their shares, which sum to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Share {
    pub to: [Option<usize>; 2],
    pub part: [f64; 2],
}

/// The D-infinity receivers of every post over a filled surface (post order index). `None`: the
/// post sends nothing (an outlet on the edge).
pub fn receivers(grid: &Grid, filled: &Filled) -> Result<Vec<Option<Share>>, RasterError> {
    let w = Whole::read(grid, &filled.level)?;
    let r = (grid.resolution as f64) / 1000.0;
    let quarter = via_num::PI / 4.0;
    let at = |i: i64, j: i64| -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < w.nx && (j as usize) < w.ny)
            .then(|| j as usize * w.nx + i as usize)
    };
    let mut out = Vec::with_capacity(w.v.len());
    for k in 0..w.v.len() {
        let (i, j) = w.ij(k);
        let (i, j) = (i as i64, j as i64);
        let e0 = w.v[k];
        // (slope, angle within the facet, side neighbour, diagonal neighbour)
        let mut best: Option<(f64, f64, usize, usize)> = None;
        for &((ai, aj), (bi, bj)) in &FACETS {
            let (Some(a), Some(b)) = (at(i + ai, j + aj), at(i + bi, j + bj)) else {
                continue;
            };
            let (e1, e2) = (w.v[a], w.v[b]);
            let s1 = (e0 - e1) / r;
            let s2 = (e1 - e2) / r;
            let mut ang = via_num::atan2(s2, s1);
            let mut s = (s1 * s1 + s2 * s2).sqrt();
            if ang < 0.0 {
                ang = 0.0;
                s = s1;
            } else if ang > quarter {
                ang = quarter;
                s = (e0 - e2) / (r * 2f64.sqrt());
            }
            if s > 0.0 && best.is_none_or(|b| s > b.0) {
                best = Some((s, ang, a, b));
            }
        }
        let share = match best {
            Some((_, ang, a, b)) => {
                let p2 = ang / quarter;
                let p1 = 1.0 - p2;
                Some(Share {
                    to: [(p1 > 0.0).then_some(a), (p2 > 0.0).then_some(b)],
                    part: [p1, p2],
                })
            }
            None => filled.from[k].map(|f| Share {
                to: [Some(f), None],
                part: [1.0, 0.0],
            }),
        };
        // An edge post that is an outlet sends nothing on, unless the surface falls inward.
        out.push(if w.on_edge(k) && best.is_none() {
            None
        } else {
            share
        });
    }
    Ok(out)
}

/// The area that drains through each post, in square metres: each post's own share of the ground,
/// `r * r`, plus every inflow, routed by D-infinity over the filled surface.
pub fn accumulate<T: Copy + Into<f64>>(
    grid: &Grid,
    values: &Raster<T>,
) -> Result<Raster<f64>, RasterError> {
    let filled = fill(grid, values)?;
    let to = receivers(grid, &filled)?;
    let r = (grid.resolution as f64) / 1000.0;
    let mut acc = vec![r * r; to.len()];
    for &k in filled.order.iter().rev() {
        if let Some(s) = to[k] {
            let a = acc[k];
            for n in 0..2 {
                if let Some(m) = s.to[n] {
                    acc[m] += a * s.part[n];
                }
            }
        }
    }
    Raster::from_values(Window::whole(grid), acc).ok_or(RasterError::Internal {
        reason: "accumulation does not match the grid".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn field(f: impl Fn(f64, f64) -> f64) -> Raster<f32> {
        Raster::from_fn(Window::whole(&g8()), |i, j| {
            Ok::<_, ()>(f(i as f64, j as f64) as f32)
        })
        .unwrap()
    }

    #[test]
    fn a_ramp_drains_in_straight_rows() {
        let a = accumulate(&g8(), &field(|x, _| x)).unwrap();
        // Each row gathers the posts east of it; the west edge holds the whole row.
        for j in 1..8 {
            assert_eq!(a.get(0, j), Some(9.0), "row {j}");
            assert_eq!(a.get(4, j), Some(5.0));
        }
    }

    #[test]
    fn a_tilted_plane_spreads_over_two_neighbours() {
        // Falling towards (-2, -1): every inner post sends to two neighbours, split by angle.
        let g = g8();
        let filled = fill(&g, &field(|x, y| 2.0 * x + y)).unwrap();
        let to = receivers(&g, &filled).unwrap();
        let k = 4 * 9 + 4;
        let s = to[k].unwrap();
        assert_eq!(s.to, [Some(4 * 9 + 3), Some(3 * 9 + 3)]);
        assert!(s.part[0] > 0.0 && s.part[1] > 0.0);
        assert!((s.part[0] + s.part[1] - 1.0).abs() < 1e-15);
    }

    #[test]
    fn a_hollow_is_filled_and_drained_out() {
        // A pit in the middle of a ramp: its flow still reaches the west edge.
        let h = field(|x, y| if (x, y) == (4.0, 4.0) { 0.0 } else { x + 1.0 });
        let a = accumulate(&g8(), &h).unwrap();
        let total: f64 = (0..9).map(|j| a.get(0, j).unwrap()).sum();
        assert_eq!(total, 81.0);
    }
}
