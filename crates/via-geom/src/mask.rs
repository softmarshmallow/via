//! Mask to regions (`spec/geometry.md` 3.7): marching squares at level 0 (Lorensen and Cline
//! 1987, in two dimensions), every piece kept.
//!
//! Each grid cell gives the polygon of its part above 0 (a whole cell, or the cell cut at the
//! crossings on its edges; a saddle settled by the mean of its four posts). The polygons are
//! noded and united, so the result is valid and settled, and a piece that reaches the grid's
//! edge is closed along it.

use crate::diag::{Code, GeomDiag};
use crate::lineref::round;
use crate::overlay::wind;
use crate::pred::{twice_area, Side};
use crate::shape::{Region, P};
use via_diag::Value;

/// The grid of a mask: origin, spacing and post counts.
#[derive(Clone, Copy, Debug)]
pub struct MaskGrid {
    /// The extent's least corner.
    pub x0: i64,
    /// The extent's least corner.
    pub y0: i64,
    /// Post spacing, millimetres.
    pub r: i64,
    /// Posts along `x`.
    pub nx: usize,
    /// Posts along `y`.
    pub ny: usize,
}

/// The mask of `values` (post order) above 0, over the window `[i0, j0, i1, j1]`: the posts
/// `i0 <= i < i1`, `j0 <= j < j1` (`numbers.md` 6), so the rings are clipped to the rectangle from
/// post `(i0, j0)` to post `(i1 - 1, j1 - 1)`.
pub fn mask(
    grid: MaskGrid,
    values: &[f64],
    formula: &str,
    window: Option<[usize; 4]>,
) -> Result<(Region, Vec<GeomDiag>), GeomDiag> {
    let src = "#mask";
    if values.len() != grid.nx * grid.ny {
        return Err(
            GeomDiag::new(Code::BadParameter, "values do not match the grid")
                .with("parameter", "values"),
        );
    }
    if grid.nx == 0 || grid.ny == 0 {
        return Err(
            GeomDiag::new(Code::BadParameter, "the grid has no posts").with("parameter", "grid")
        );
    }
    // Half-open in, inclusive below: the last post of the window on each axis.
    let [i0, j0, i1, j1] = match window {
        None => [0, 0, grid.nx - 1, grid.ny - 1],
        Some([i0, j0, i1, j1]) if i0 < i1 && j0 < j1 && i1 <= grid.nx && j1 <= grid.ny => {
            [i0, j0, i1 - 1, j1 - 1]
        }
        Some(_) => {
            return Err(GeomDiag::new(
                Code::BadParameter,
                "the window is empty or leaves the grid: `i0 < i1 <= nx`, `j0 < j1 <= ny`",
            )
            .with("parameter", "window"))
        }
    };
    let v = |i: usize, j: usize| values[j * grid.nx + i];
    let post =
        |i: usize, j: usize| P::new(grid.x0 + (i as i64) * grid.r, grid.y0 + (j as i64) * grid.r);
    let mut diags = Vec::new();
    let mut any = false;
    for j in j0..=j1 {
        for i in i0..=i1 {
            if v(i, j) > 0.0 {
                any = true;
            }
        }
    }
    if !any {
        let w: Vec<f64> = (j0..=j1)
            .flat_map(|j| (i0..=i1).map(move |i| (i, j)))
            .map(|(i, j)| v(i, j))
            .collect();
        let lo = via_num::minimum(&w).unwrap_or(0.0);
        let hi = via_num::maximum(&w).unwrap_or(0.0);
        let mut d =
            GeomDiag::new(Code::MaskEmpty, "the mask is above 0 nowhere").with("formula", formula);
        if let (Some(l), Some(h)) = (Value::double(lo), Value::double(hi)) {
            d = d.with("least", l).with("greatest", h);
        }
        diags.push(d);
        return Ok((Region::default(), diags));
    }
    // Crossing on the grid edge from post p (first in post order) to q.
    let crossing = |p: (usize, usize), q: (usize, usize)| -> Result<P, GeomDiag> {
        let (v0, v1) = (v(p.0, p.1), v(q.0, q.1));
        let f = v0 / (v0 - v1);
        let a = post(p.0, p.1);
        let r = grid.r as f64;
        if p.1 == q.1 {
            Ok(P::new(round(src, (a.x as f64) + (f * r))?, a.y))
        } else {
            Ok(P::new(a.x, round(src, (a.y as f64) + (f * r))?))
        }
    };
    let mut polys: Vec<Vec<P>> = Vec::new();
    for j in j0..j1 {
        let mut run: Option<usize> = None;
        for i in i0..=i1 {
            let full = i < i1
                && [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)]
                    .iter()
                    .all(|&(a, b)| v(a, b) > 0.0);
            if full {
                if run.is_none() {
                    run = Some(i);
                }
                continue;
            }
            if let Some(s) = run.take() {
                polys.push(vec![post(s, j), post(i, j), post(i, j + 1), post(s, j + 1)]);
            }
            if i == i1 {
                break;
            }
            let c = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let up: Vec<bool> = c.iter().map(|&(a, b)| v(a, b) > 0.0).collect();
            if !up.iter().any(|&u| u) {
                continue;
            }
            // Edges in counter-clockwise order, each with its first post in post order.
            let edge_first = |k: usize| -> ((usize, usize), (usize, usize)) {
                match k {
                    0 => (c[0], c[1]),
                    1 => (c[1], c[2]),
                    2 => (c[3], c[2]),
                    _ => (c[0], c[3]),
                }
            };
            let saddle = up[0] == up[2] && up[1] == up[3] && up[0] != up[1];
            if saddle {
                let mean = via_num::mean(&c.iter().map(|&(a, b)| v(a, b)).collect::<Vec<_>>())
                    .unwrap_or(0.0);
                if mean <= 0.0 {
                    // Two separate corners.
                    for k in 0..4 {
                        if !up[k] {
                            continue;
                        }
                        let prev = (k + 3) % 4;
                        let (a1, b1) = edge_first(k);
                        let (a0, b0) = edge_first(prev);
                        polys.push(vec![
                            post(c[k].0, c[k].1),
                            crossing(a1, b1)?,
                            crossing(a0, b0)?,
                        ]);
                    }
                    continue;
                }
            }
            let mut poly: Vec<P> = Vec::new();
            for k in 0..4 {
                if up[k] {
                    poly.push(post(c[k].0, c[k].1));
                }
                if up[k] != up[(k + 1) % 4] {
                    let (a, b) = edge_first(k);
                    poly.push(crossing(a, b)?);
                }
            }
            polys.push(poly);
        }
    }
    let polys: Vec<Vec<P>> = polys.into_iter().filter(|p| twice_area(p) > 0).collect();
    let region = if polys.is_empty() {
        Region::default()
    } else {
        let ov = wind(&[polys])?;
        let sel = ov.select(&|w| w[0] > 0);
        ov.region(&sel)
    };
    // Tiny pieces, kept and reported.
    let tiny_limit = 2 * i128::from(grid.r) * i128::from(grid.r);
    let tiny: Vec<&crate::shape::Piece> = region
        .pieces
        .iter()
        .filter(|p| p.twice_area() < tiny_limit)
        .collect();
    if !tiny.is_empty() {
        let pos: Vec<Value> = tiny
            .iter()
            .map(|p| {
                Value::List(vec![
                    Value::Length(p.shell[0].x),
                    Value::Length(p.shell[0].y),
                ])
            })
            .collect();
        diags.push(
            GeomDiag::new(
                Code::MaskTiny,
                format!("{} pieces are smaller than one grid cell; kept", tiny.len()),
            )
            .at(tiny[0].shell[0])
            .with("count", tiny.len() as i64)
            .with("positions", Value::List(pos)),
        );
    }
    // Dropped: a post above 0 that no piece holds.
    for j in j0..=j1 {
        for i in i0..=i1 {
            if v(i, j) > 0.0 && region.side(post(i, j)) == Side::Out {
                diags.push(
                    GeomDiag::new(
                        Code::MaskDropped,
                        "a masked piece had no area on the lattice and was dropped",
                    )
                    .at(post(i, j)),
                );
            }
        }
    }
    Ok((region, diags))
}
