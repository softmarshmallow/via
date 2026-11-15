//! Derived results (`spec/derived.md`): one implementation behind the text forms and the worker's
//! operations (R19b: our libraries reach no further than a user's step).
//!
//! - [`where_region`]: 3.7's mask of a condition, kept to the piece holding a point when `holds` is
//!   written (section 2). It is what `where` and `where` with `holds` lower to, and what a
//!   worker's `derive.where` returns, so the two are byte for byte the same over the same view.
//! - [`route`] and [`route_path`]: the least-cost course through a path's written points
//!   (section 3); `derive.route` is [`route`], the text form is [`route_path`].
//! - [`descend`] and [`spill`]: the batch operations `derive.descend` and `derive.spill`
//!   (section 4), one row per point.
//! - [`shape_key_faults`]: `derive.holds_alone` and `derive.route_alone`, at the key written.
//!
//! The values each takes are those the caller evaluated over the whole extent from the head view
//! (section 1, "Head, not effects"; `authority.md` 2.3): a formula's values at every post, or a
//! field's narrowed values. Nothing here reads an effect or evaluates a formula.
//!
//! The algorithms are `via-raster`'s ([`via_raster::route`], [`via_raster::flood`],
//! [`via_raster::descend`]) and the kernel's ([`via_geom::mask`], [`via_geom::hollow`]).

use crate::text::at_key;
use via_diag::{codes, Diag};
use via_geom::mask::{mask, MaskGrid};
use via_geom::{GeomDiag, Region, P};
use via_num::Grid;
use via_raster::raster::read_at;
use via_raster::{Raster, RasterError, Window};

pub use via_raster::descend::End;

/// A found region: the region, whether `holds` found no piece, and what the mask and `holds` said.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub region: Region,
    /// True when `holds` was written and no piece holds its point (`derive.dry`).
    pub dry: bool,
    /// `geom.mask_*` and `derive.dry` / `derive.open`, for the caller to locate at its key.
    pub diags: Vec<GeomDiag>,
}

fn mask_grid(grid: &Grid) -> Result<MaskGrid, GeomDiag> {
    let n = |v: u64| {
        usize::try_from(v).map_err(|_| {
            GeomDiag::new(via_geom::Code::BadParameter, "the grid is too large")
                .with("parameter", "grid")
        })
    };
    Ok(MaskGrid {
        x0: grid.extent[0],
        y0: grid.extent[1],
        r: grid.resolution,
        nx: n(grid.nx)?,
        ny: n(grid.ny)?,
    })
}

/// A condition's values at every post (post order) as a raster over the whole grid.
fn whole(grid: &Grid, values: &[f64]) -> Result<Raster<f64>, RasterError> {
    Raster::from_values(Window::whole(grid), values.to_vec()).ok_or(RasterError::Internal {
        reason: "values do not match the grid".into(),
    })
}

/// `where`, and `where` with `holds` (`derived.md` 2): the region where `values` (the condition at
/// every post, in post order) is above 0, every piece kept, or with `holds` the one piece holding
/// the point. `formula` is the condition as written, for `geom.mask_empty`'s data.
///
/// A mask with no post above 0 is `geom.mask_empty`; with `holds` it is also dry, and `derive.dry`
/// carries the condition's value at the point (`fields.md` 1.2).
pub fn where_region(
    grid: &Grid,
    values: &[f64],
    formula: &str,
    holds: Option<P>,
) -> Result<Found, GeomDiag> {
    let (region, mut diags) = mask(mask_grid(grid)?, values, formula, None)?;
    let Some(at) = holds else {
        return Ok(Found {
            region,
            dry: false,
            diags,
        });
    };
    let raster = whole(grid, values).map_err(|e| {
        GeomDiag::new(via_geom::Code::BadParameter, e.to_string()).with("parameter", "values")
    })?;
    let value = read_at(grid, &raster, at.x, at.y).map_err(|e| {
        GeomDiag::new(via_geom::Code::BadParameter, e.to_string())
            .with("parameter", "holds")
            .at(at)
    })?;
    let (one, more) = via_geom::hollow::holds(&region, at, grid.extent, value);
    let dry = one.is_empty();
    diags.extend(more);
    Ok(Found {
        region: one,
        dry,
        diags,
    })
}

/// A course before smoothing (`derived.md` 3 steps 2 to 5 without `smooth`): what `derive.route`
/// returns per path.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    pub points: Vec<P>,
    /// The sum of the weights of the steps taken.
    pub cost: f64,
}

/// The least-cost course through `waypoints` over `cost` (the cost formula at every post, in post
/// order). A cost at or below 0 is `formula.range` at the first such post
/// ([`RasterError::CostRange`]).
pub fn route(grid: &Grid, cost: &[f64], waypoints: &[P]) -> Result<Course, RasterError> {
    let c = whole(grid, cost)?;
    let w: Vec<(i64, i64)> = waypoints.iter().map(|p| (p.x, p.y)).collect();
    let found = via_raster::route::route(grid, &c, &w)?;
    Ok(Course {
        points: found
            .points
            .into_iter()
            .map(|(x, y)| P::new(x, y))
            .collect(),
        cost: found.cost,
    })
}

/// Why a written `route` made no path.
#[derive(Clone, Debug, PartialEq)]
pub enum RouteFault {
    /// The search's fault (`formula.range` and the like), to locate at `route`.
    Raster(RasterError),
    /// The smoothing's fault, to locate at `smooth`.
    Geom(GeomDiag),
}

/// `route` on a written path (`derived.md` 3): [`route`], then, when the feature writes
/// `smooth = true`, smoothed through the course's points (`geometry.md` 2.4). `source` names the
/// feature for the kernel's diagnostics.
pub fn route_path(
    source: &str,
    grid: &Grid,
    cost: &[f64],
    waypoints: &[P],
    smooth: bool,
) -> Result<Vec<P>, RouteFault> {
    let c = route(grid, cost, waypoints).map_err(RouteFault::Raster)?;
    if !smooth {
        return Ok(c.points);
    }
    // `derived.md` 3 step 6: the curve is held to the extent the course was found over. Through
    // a course along the extent's edge it bulges past the edge (case d25), and a kind's
    // `smoothed_along` would read the field beyond it. A written waypoint stays where it
    // is written; a sample beyond the extent moves onto its edge.
    let s = via_geom::curve::smooth(source, &c.points).map_err(RouteFault::Geom)?;
    let [x0, y0, x1, y1] = grid.extent;
    let mut out: Vec<P> = Vec::with_capacity(s.len());
    for p in s {
        let q = if waypoints.contains(&p) {
            p
        } else {
            P::new(p.x.clamp(x0, x1), p.y.clamp(y0, y1))
        };
        if out.last() != Some(&q) {
            out.push(q);
        }
    }
    Ok(out)
}

/// One row of `derive.descend` (`derived.md` 4.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Descent {
    /// The line; one point when the line does not move (returned as a point feature).
    pub points: Vec<P>,
    pub end: End,
}

/// `derive.descend`: the line water would follow from each point over `field` (a field's narrowed
/// values at every post).
pub fn descend(
    grid: &Grid,
    field: &Raster<f32>,
    points: &[P],
) -> Result<Vec<Descent>, RasterError> {
    points
        .iter()
        .map(|p| {
            let d = via_raster::descend::descend(grid, field, p.x, p.y)?;
            Ok(Descent {
                points: d.points.into_iter().map(|(x, y)| P::new(x, y)).collect(),
                end: d.end,
            })
        })
        .collect()
}

/// One row of `derive.spill` (`derived.md` 4.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spill {
    pub level: f64,
    pub dry: bool,
}

/// `derive.spill`: the level the hollow holding each point fills to, with the radius beside it
/// (millimetres, 0 for none).
pub fn spill(
    grid: &Grid,
    field: &Raster<f32>,
    points: &[P],
    radius: &[i64],
) -> Result<Vec<Spill>, RasterError> {
    if radius.len() != points.len() {
        return Err(RasterError::Internal {
            reason: format!("{} points but {} radii", points.len(), radius.len()),
        });
    }
    points
        .iter()
        .zip(radius)
        .map(|(p, &r)| {
            let s = via_raster::flood::spill(grid, field, p.x, p.y, r)?;
            Ok(Spill {
                level: s.level,
                dry: s.dry,
            })
        })
        .collect()
}

/// The written-shape keys of `derived.md` (sections 2 and 3) that stand alone: `holds` without
/// `where` is `derive.holds_alone`, `route` on a feature that is not a written `path` is
/// `derive.route_alone`, each at its key.
pub fn shape_key_faults(
    id: &str,
    has_where: bool,
    has_holds: bool,
    has_route: bool,
    is_path: bool,
) -> Vec<Diag> {
    let mut out = Vec::new();
    if has_holds && !has_where {
        out.push(at_key(
            codes::derive::HOLDS_ALONE,
            "feature",
            id,
            "holds",
            "`holds` is written without `where`: it keeps one piece of a `where` region",
            "write `where`, or remove `holds`",
        ));
    }
    if has_route && !is_path {
        out.push(at_key(
            codes::derive::ROUTE_ALONE,
            "feature",
            id,
            "route",
            "`route` is written on a feature without `path`: it finds a course through a path's points",
            "give the path its ends with `starts_at` and `ends_at`, or remove `route`",
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn posts(f: impl Fn(i64, i64) -> f64) -> Vec<f64> {
        let mut v = Vec::new();
        for j in 0..9 {
            for i in 0..9 {
                v.push(f(i, j));
            }
        }
        v
    }

    fn twin(x: i64, y: i64) -> f64 {
        ((x - 2).abs() + (y - 4).abs()).min((x - 6).abs() + (y - 4).abs()) as f64
    }

    #[test]
    fn holds_keeps_the_piece_of_the_plain_where() {
        let v = posts(|x, y| 1.5 - twin(x, y));
        let all = where_region(&g8(), &v, "= 1.5 - f('height')", None).unwrap();
        let one = where_region(&g8(), &v, "", Some(P::new(2_000, 4_000))).unwrap();
        assert_eq!(all.region.pieces.len(), 2);
        assert_eq!(one.region.pieces.len(), 1);
        assert!(all.region.pieces.contains(&one.region.pieces[0]));
        assert!(!one.dry && one.diags.is_empty());
        let dry = where_region(&g8(), &v, "", Some(P::new(4_000, 0))).unwrap();
        assert!(dry.dry && dry.region.is_empty());
        assert_eq!(dry.diags[0].code, via_geom::Code::DeriveDry);
    }

    #[test]
    fn route_text_and_batch_agree() {
        let c = posts(|_, _| 1.0);
        let w = [P::new(0, 0), P::new(2_000, 1_000)];
        let batch = route(&g8(), &c, &w).unwrap();
        let text = route_path("course", &g8(), &c, &w, false).unwrap();
        assert_eq!(batch.points, text);
        assert_eq!(text, vec![P::new(0, 0), P::new(2_000, 1_000)]);
        let zero = posts(|_, _| 0.0);
        let e = route_path("course", &g8(), &zero, &w, false).unwrap_err();
        match e {
            RouteFault::Raster(r) => {
                let d = r.diag(
                    via_diag::Subject::entity("feature", "course"),
                    via_diag::Site::Key(via_diag::KeyPath::key("route")),
                );
                assert_eq!(*d.code(), via_diag::DiagCode::Core(codes::formula::RANGE));
                assert_eq!(d.position(), Some(via_diag::Position::plan(0, 0)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn batches_run_one_row_per_point() {
        let g = g8();
        let bowl = Raster::from_fn(Window::whole(&g), |i, j| {
            let (x, y) = (i as i64 - 4, j as i64 - 4);
            Ok::<_, ()>((x * x + y * y) as f32)
        })
        .unwrap();
        let s = spill(&g, &bowl, &[P::new(4_000, 4_000); 2], &[0, 2_000]).unwrap();
        assert_eq!(
            s,
            vec![
                Spill {
                    level: 16.0,
                    dry: false
                },
                Spill {
                    level: 5.0,
                    dry: false
                }
            ]
        );
        let d = descend(&g, &bowl, &[P::new(8_000, 8_000), P::new(4_000, 4_000)]).unwrap();
        assert_eq!(d[0].points.len(), 5);
        assert_eq!(d[1].points, vec![P::new(4_000, 4_000)]);
        assert_eq!((d[0].end, d[1].end), (End::Pit, End::Pit));
    }

    #[test]
    fn lone_keys_are_located_at_their_key() {
        let d = shape_key_faults("tarn", false, true, true, false);
        assert_eq!(d.len(), 2);
        assert_eq!(
            *d[0].code(),
            via_diag::DiagCode::Core(codes::derive::HOLDS_ALONE)
        );
        assert_eq!(
            *d[1].code(),
            via_diag::DiagCode::Core(codes::derive::ROUTE_ALONE)
        );
        assert!(shape_key_faults("course", false, false, true, true).is_empty());
    }
}
