//! Applying entries to a field (`fields.md` 5): one rule for `then` rows and kind effects, in the
//! order the caller gives (`authority.md` 4), by window.
//!
//! An entry is a closure the caller builds from its formulas: at a post, given `v` (the value so
//! far), it gives `Some(Mix)` or, beyond a bounded effect's radius, `None`, where nothing is
//! evaluated and nothing changes (5.3). This crate checks the mask's range and applies the rule;
//! it evaluates no formula itself.

use crate::error::RasterError;
use crate::raster::Raster;
use crate::window::{post_mm, Window};
use std::collections::BTreeMap;
use via_num::Grid;

/// What one entry gives at one post: its `value` and its `mask`, both doubles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    pub value: f64,
    pub mask: f64,
}

/// An entry failed: either this crate's check, or the caller's own evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum Fault<E> {
    Raster(RasterError),
    Entry(E),
}

impl<E> From<RasterError> for Fault<E> {
    fn from(e: RasterError) -> Fault<E> {
        Fault::Raster(e)
    }
}

/// The mask's range, checked where it is evaluated (`fields.md` 5; `formulas.md` 6.1 owns the
/// code): 0 to 1, never clamped. A NaN fails too.
fn checked_mask(grid: &Grid, i: u64, j: u64, m: f64) -> Result<f64, RasterError> {
    if (0.0..=1.0).contains(&m) {
        Ok(m)
    } else {
        Err(RasterError::MaskRange {
            at: post_mm(grid, i, j),
            mask: m,
        })
    }
}

/// One entry on a scalar field (`fields.md` 5.1), at every post of `target` inside `bound` (the
/// effect's window of 5.3; `None` for the whole extent): `v = (1 - m) * v + m * value`, in
/// doubles, in that order, with no fused multiply-add. The posts are visited in post order, so the
/// first fault is the least post.
pub fn apply_scalar<E>(
    grid: &Grid,
    target: &mut Raster<f64>,
    bound: Option<Window>,
    mut entry: impl FnMut(u64, u64, f64) -> Result<Option<Mix>, E>,
) -> Result<(), Fault<E>> {
    let tw = target.window();
    let w = match bound {
        Some(b) => match tw.intersect(&b) {
            Some(w) => w,
            None => return Ok(()),
        },
        None => tw,
    };
    let values = target.values_mut();
    for (i, j) in w.posts() {
        let Some(k) = tw.offset(i, j) else { continue };
        let v = values[k];
        let Some(mix) = entry(i, j, v).map_err(Fault::Entry)? else {
            continue;
        };
        let m = checked_mask(grid, i, j, mix.mask)?;
        let out = (1.0 - m) * v + m * mix.value;
        values[k] =
            via_num::finite(out).map_err(|e| RasterError::from_num(e, post_mm(grid, i, j)))?;
    }
    Ok(())
}

/// A `weights` field's values over a window: one plane per material that some entry painted in it
/// (with a mask above 0 at some post), and no plane for any other (`fields.md` 6.3). Materials are
/// in id order, bytewise. A material with no plane weighs 0 at every post.
#[derive(Clone, Debug, PartialEq)]
pub struct Weights {
    window: Window,
    planes: BTreeMap<String, Raster<f64>>,
}

impl Weights {
    /// No material anywhere: the start of every weights field (`fields.md` 3).
    pub fn empty(window: Window) -> Weights {
        Weights {
            window,
            planes: BTreeMap::new(),
        }
    }

    /// The window.
    pub fn window(&self) -> Window {
        self.window
    }

    /// The planes, in id order.
    pub fn planes(&self) -> &BTreeMap<String, Raster<f64>> {
        &self.planes
    }

    /// One entry with `paint` (`fields.md` 5.2), at every post inside `bound`:
    ///
    /// ```text
    /// w[k] = (1 - m) * w[k]              for every material k other than paint
    /// w[paint] = (1 - m) * w[paint] + m
    /// ```
    ///
    /// in doubles, materials in id order. The masks are evaluated first, in post order, so the
    /// first fault is the least post and no plane changes before every mask is known.
    pub fn apply_paint<E>(
        &mut self,
        grid: &Grid,
        paint: &str,
        bound: Option<Window>,
        mut mask: impl FnMut(u64, u64) -> Result<Option<f64>, E>,
    ) -> Result<(), Fault<E>> {
        let w = match bound {
            Some(b) => match self.window.intersect(&b) {
                Some(w) => w,
                None => return Ok(()),
            },
            None => self.window,
        };
        let mut masks: Vec<(usize, f64)> = Vec::new();
        for (i, j) in w.posts() {
            let Some(k) = self.window.offset(i, j) else {
                continue;
            };
            if let Some(m) = mask(i, j).map_err(Fault::Entry)? {
                masks.push((k, checked_mask(grid, i, j, m)?));
            }
        }
        if masks.iter().any(|&(_, m)| m > 0.0) && !self.planes.contains_key(paint) {
            self.planes
                .insert(paint.to_string(), Raster::filled(self.window, 0.0));
        }
        for (name, plane) in self.planes.iter_mut() {
            let values = plane.values_mut();
            if name == paint {
                for &(k, m) in &masks {
                    values[k] = (1.0 - m) * values[k] + m;
                }
            } else {
                for &(k, m) in &masks {
                    values[k] *= 1.0 - m;
                }
            }
        }
        Ok(())
    }

    /// The weights narrowed to 32-bit floats once, each plane by `fields.md` 1.1.
    pub fn narrow(&self, grid: &Grid) -> Result<BTreeMap<String, Raster<f32>>, RasterError> {
        self.planes
            .iter()
            .map(|(k, p)| crate::raster::narrow(grid, p).map(|p| (k.clone(), p)))
            .collect()
    }
}

/// The tolerance on every check of a weights sum (`fields.md` 5.2).
pub const WEIGHTS_TOLERANCE: f64 = 1e-6;

/// The sum of narrowed weights at a post, widened to doubles and added in material id order
/// (`fields.md` 5.2). A material whose plane does not hold the post adds nothing.
pub fn weights_sum(planes: &BTreeMap<String, Raster<f32>>, i: u64, j: u64) -> f64 {
    let mut s = 0.0;
    for p in planes.values() {
        if let Some(w) = p.get(i, j) {
            s += f64::from(w);
        }
    }
    s
}

/// Posts of `window` whose narrowed weights sum below `1 - 1e-6` (`field.unpainted`): the first in
/// post order and the count. `None` when every post is painted.
pub fn unpainted(
    planes: &BTreeMap<String, Raster<f32>>,
    window: Window,
) -> Option<((u64, u64), u64)> {
    let mut first = None;
    let mut count = 0;
    for (i, j) in window.posts() {
        if weights_sum(planes, i, j) < 1.0 - WEIGHTS_TOLERANCE {
            first.get_or_insert((i, j));
            count += 1;
        }
    }
    first.map(|f| (f, count))
}

/// The first post of `window`, in post order, whose weights sum above `1 + 1e-6`: a defect, since
/// 5.2's rule keeps the sum at most 1 (the "weights sum" case).
pub fn over_painted(planes: &BTreeMap<String, Raster<f32>>, window: Window) -> Option<(u64, u64)> {
    window
        .posts()
        .find(|&(i, j)| weights_sum(planes, i, j) > 1.0 + WEIGHTS_TOLERANCE)
}

/// `keep_out`'s per-post values, and each field's first clamped post.
pub type KeepOut = (Raster<f64>, Vec<Option<(u64, u64)>>);

/// The per-post maximum over every `keep_out` field, each value clamped to 0 to 1 first, 0 where
/// there is none (`fields.md` 2.1). Each field's first clamped post, in post order, is returned
/// beside it for its one `field.role_range`. Every field must hold `window`.
pub fn keep_out(window: Window, fields: &[&Raster<f32>]) -> Result<KeepOut, RasterError> {
    let mut out = Raster::filled(window, 0.0);
    let mut first = vec![None; fields.len()];
    for (n, f) in fields.iter().enumerate() {
        if !f.window().covers(&window) {
            return Err(RasterError::Internal {
                reason: format!(
                    "keep_out field over {} not {}",
                    f.window().key(),
                    window.key()
                ),
            });
        }
        for (k, (i, j)) in window.posts().enumerate() {
            let v = f.get(i, j).map(f64::from).unwrap_or(0.0);
            if !(0.0..=1.0).contains(&v) {
                first[n].get_or_insert((i, j));
            }
            let c = via_num::clamp(v, 0.0, 1.0);
            let acc = &mut out.values_mut()[k];
            *acc = via_num::max(*acc, c);
        }
    }
    Ok((out, first))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g8() -> Grid {
        Grid::new([0, 0, 8_000, 8_000], 1_000).unwrap()
    }

    fn row(value: f64, mask: f64) -> impl FnMut(u64, u64, f64) -> Result<Option<Mix>, ()> {
        move |_, _, _| Ok(Some(Mix { value, mask }))
    }

    #[test]
    fn one_row_and_exact_mask_one() {
        let g = g8();
        let mut v = Raster::filled(Window::whole(&g), 10.0);
        apply_scalar(&g, &mut v, None, row(20.0, 0.25)).unwrap();
        assert!(v.values().iter().all(|&x| x == 12.5));
        let mut v = Raster::filled(Window::whole(&g), 0.1);
        apply_scalar(&g, &mut v, None, row(1.0 / 3.0, 1.0)).unwrap();
        assert!(v
            .values()
            .iter()
            .all(|&x| x.to_bits() == (1.0f64 / 3.0).to_bits()));
    }

    #[test]
    fn rows_apply_in_the_order_given_and_read_v() {
        // fields.md "row order": c (order -1) then a then b, all mask 1, gives b's 1.
        let g = g8();
        let mut v = Raster::filled(Window::whole(&g), 5.0);
        apply_scalar(&g, &mut v, None, |_, _, v| {
            Ok::<_, ()>(Some(Mix {
                value: v + 2.0,
                mask: 1.0,
            }))
        })
        .unwrap();
        assert_eq!(v.get(0, 0), Some(7.0));
        for value in [3.0, 2.0, 1.0] {
            apply_scalar(&g, &mut v, None, row(value, 1.0)).unwrap();
        }
        assert_eq!(v.get(4, 4), Some(1.0));
    }

    #[test]
    fn mask_out_of_range_names_the_first_post() {
        let g = g8();
        let mut v = Raster::filled(Window::whole(&g), 0.0);
        match apply_scalar(&g, &mut v, None, row(1.0, 1.5)) {
            Err(Fault::Raster(RasterError::MaskRange { at, .. })) => assert_eq!(at, (0, 0)),
            other => panic!("{other:?}"),
        }
        match apply_scalar(&g, &mut v, None, row(1.0, f64::NAN)) {
            Err(Fault::Raster(RasterError::MaskRange { .. })) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_bound_changes_nothing_beyond_it() {
        let g = g8();
        let mut v = Raster::filled(Window::whole(&g), 0.0);
        let b = Window::new(2, 2, 4, 4).unwrap();
        apply_scalar(&g, &mut v, Some(b), row(1.0, 1.0)).unwrap();
        for (i, j) in Window::whole(&g).posts() {
            assert_eq!(v.get(i, j), Some(if b.contains(i, j) { 1.0 } else { 0.0 }));
        }
    }

    #[test]
    fn weights_rule_and_sparse_planes() {
        // fields.md "weights rule": grass mask 1, rock mask 0.25: grass 0.75, rock 0.25.
        let g = g8();
        let w = Window::whole(&g);
        let mut ws = Weights::empty(w);
        ws.apply_paint(&g, "grass", None, |_, _| Ok::<_, ()>(Some(1.0)))
            .unwrap();
        ws.apply_paint(&g, "rock", None, |_, _| Ok::<_, ()>(Some(0.25)))
            .unwrap();
        // A paint whose mask is 0 everywhere makes no plane.
        ws.apply_paint(&g, "sand", None, |_, _| Ok::<_, ()>(Some(0.0)))
            .unwrap();
        assert_eq!(ws.planes().keys().collect::<Vec<_>>(), ["grass", "rock"]);
        let n = ws.narrow(&g).unwrap();
        assert_eq!(n["grass"].get(3, 3), Some(0.75));
        assert_eq!(n["rock"].get(3, 3), Some(0.25));
        assert_eq!(unpainted(&n, w), None);
        assert_eq!(over_painted(&n, w), None);
    }

    #[test]
    fn unpainted_counts_and_names_the_first() {
        // fields.md "unpainted": one rock row, mask 0.5: first post (0, 0), count 81.
        let g = g8();
        let w = Window::whole(&g);
        let mut ws = Weights::empty(w);
        ws.apply_paint(&g, "rock", None, |_, _| Ok::<_, ()>(Some(0.5)))
            .unwrap();
        assert_eq!(unpainted(&ws.narrow(&g).unwrap(), w), Some(((0, 0), 81)));
    }

    #[test]
    fn weights_sum_stays_at_most_one() {
        // fields.md "weights sum": ten rows of three materials with masks in 0 to 1.
        let g = g8();
        let w = Window::whole(&g);
        let mut ws = Weights::empty(w);
        for k in 0..10u32 {
            for (n, mat) in ["a", "b", "c"].iter().enumerate() {
                let shift = f64::from(k * 3 + n as u32);
                ws.apply_paint(&g, mat, None, |i, j| {
                    let s = via_num::sin(37.0 * i as f64 + 11.0 * j as f64 + shift);
                    Ok::<_, ()>(Some(0.5 + 0.5 * s))
                })
                .unwrap();
            }
        }
        assert_eq!(over_painted(&ws.narrow(&g).unwrap(), w), None);
    }

    #[test]
    fn keep_out_takes_the_maximum_then_clamps_once_per_field() {
        // fields.md "keep_out max": 1 at even i and 0 at odd, and the reverse: 1 everywhere.
        let g = g8();
        let w = Window::whole(&g);
        let a = Raster::from_fn(w, |i, _| Ok::<_, ()>(if i % 2 == 0 { 1.0 } else { 0.0 })).unwrap();
        let b = Raster::from_fn(w, |i, _| Ok::<_, ()>(if i % 2 == 1 { 1.0 } else { 0.0 })).unwrap();
        let (m, first) = keep_out(w, &[&a, &b]).unwrap();
        assert!(m.values().iter().all(|&x| x == 1.0));
        assert_eq!(first, vec![None, None]);
        // "keep_out range": base 1.5 is read as 1, reported at (0, 0).
        let c = Raster::filled(w, 1.5f32);
        let (m, first) = keep_out(w, &[&c]).unwrap();
        assert_eq!(m.get(5, 5), Some(1.0));
        assert_eq!(first, vec![Some((0, 0))]);
        let (none, _) = keep_out(w, &[]).unwrap();
        assert_eq!(none.get(0, 0), Some(0.0));
    }

    #[test]
    fn window_equals_cut_for_rows() {
        // A row reading position, applied whole and over windows, gives the same bits.
        let g = Grid::new([0, 0, 20_000, 13_000], 500).unwrap();
        let resolve = |w: Window| {
            let mut v = Raster::from_fn(w, |i, j| {
                Ok::<_, ()>(via_num::sin(i as f64 * 0.37) * 3.0 + j as f64 * 0.01)
            })
            .unwrap();
            let bound = Window::new(5, 7, 30, 19);
            apply_scalar(&g, &mut v, bound, |i, j, v| {
                Ok::<_, ()>(Some(Mix {
                    value: v * 1.5 + 2.0,
                    mask: via_num::clamp((i as f64 + j as f64) / 60.0, 0.0, 1.0),
                }))
            })
            .unwrap();
            crate::raster::narrow(&g, &v).unwrap()
        };
        let whole = resolve(Window::whole(&g));
        for w in [
            Window::new(0, 0, 1, 1).unwrap(),
            Window::new(3, 5, 10, 8).unwrap(),
            Window::new(17, 11, 41, 27).unwrap(),
            Window::new(39, 0, 41, 27).unwrap(),
        ] {
            let part = resolve(w);
            for (i, j) in w.posts() {
                assert_eq!(
                    part.get(i, j).map(f32::to_bits),
                    whole.get(i, j).map(f32::to_bits)
                );
            }
        }
    }
}
