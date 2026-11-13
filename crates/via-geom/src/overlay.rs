//! Region booleans on our noding (`spec/geometry.md` 3.3; the bake-off's fallback, section 6).
//!
//! The rings of every operand are noded together. Each face of the arrangement gets a winding
//! number per group of operands, found by walking across half-edges from the unbounded face
//! (winding 0): crossing a half-edge from its right to its left adds, per group, the number of
//! that group's rings travelling along it less those travelling against it. A rule on the
//! windings selects faces; the result is the outline between selected and unselected faces.

use crate::arrange::{rings_to_region, Arr};
use crate::diag::{Code, GeomDiag};
use crate::noding::{node, Input, Noded};
use crate::shape::{Region, P};
use std::collections::BTreeMap;

/// The result of an overlay before it is made a region: the arrangement and the selected faces.
pub struct Overlay {
    /// The noding.
    pub noded: Noded,
    /// The arrangement.
    pub arr: Arr,
    /// The winding per group of each cycle (bounded cycles are faces).
    pub winding: Vec<Vec<i32>>,
}

/// Nodes the rings of each group (`groups[g]` is a list of rings, each read as given: a ring's
/// left is its inside) and computes face windings.
pub fn wind(groups: &[Vec<Vec<P>>]) -> Result<Overlay, GeomDiag> {
    let mut inputs = Vec::new();
    let mut group_of: BTreeMap<String, usize> = BTreeMap::new();
    for (g, rings) in groups.iter().enumerate() {
        for (i, r) in rings.iter().enumerate() {
            let mut pts: Vec<P> = Vec::with_capacity(r.len());
            for &p in r {
                if pts.last() != Some(&p) {
                    pts.push(p);
                }
            }
            while pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
            if pts.len() < 2 {
                continue;
            }
            let id = format!("g{g:04}r{i:08}");
            group_of.insert(id.clone(), g);
            inputs.push(Input {
                source: id,
                pts,
                closed: true,
            });
        }
    }
    let noded = node(inputs)?;
    let arr = Arr::build(&noded);
    let ng = groups.len();
    let src_group: Vec<usize> = noded.sources.iter().map(|s| group_of[s]).collect();
    // Weight of each half-edge per group: along minus against.
    let weight = |h: usize| -> Vec<i32> {
        let seg = &noded.segments[h / 2];
        let forward = h.is_multiple_of(2); // least end to other end
        let mut w = vec![0i32; ng];
        for p in &seg.prov {
            let along = forward != p.reversed;
            w[src_group[p.source]] += if along { 1 } else { -1 };
        }
        w
    };
    let nc = arr.cycles.len();
    let mut winding: Vec<Option<Vec<i32>>> = vec![None; nc];
    for k in arr.comp_order() {
        let start = arr.outer[k];
        if start == usize::MAX {
            continue;
        }
        let base = match arr.parent[k] {
            None => vec![0; ng],
            Some(pc) => winding[pc].clone().unwrap_or_else(|| vec![0; ng]),
        };
        winding[start] = Some(base);
        let mut stack = vec![start];
        while let Some(c) = stack.pop() {
            let wc = winding[c].clone().unwrap_or_else(|| vec![0; ng]);
            for &h in &arr.cycles[c] {
                let t = h ^ 1;
                let ct = arr.cycle_of[t];
                if winding[ct].is_none() {
                    let wh = weight(h);
                    let v: Vec<i32> = wc.iter().zip(&wh).map(|(a, b)| a - b).collect();
                    winding[ct] = Some(v);
                    stack.push(ct);
                }
            }
        }
    }
    Ok(Overlay {
        noded,
        arr,
        winding: winding
            .into_iter()
            .map(|w| w.unwrap_or_else(|| vec![0; ng]))
            .collect(),
    })
}

impl Overlay {
    /// The selection of each cycle by a rule; unbounded cycles are never selected.
    pub fn select(&self, rule: &dyn Fn(&[i32]) -> bool) -> Vec<bool> {
        (0..self.arr.cycles.len())
            .map(|c| self.arr.bounded(c) && rule(&self.winding[c]))
            .collect()
    }

    /// Whether the face on the left of half-edge `h` is selected.
    fn left_selected(&self, sel: &[bool], h: usize) -> bool {
        match self.arr.face_left(h) {
            Some(c) => sel[c],
            None => false,
        }
    }

    /// The boundary rings of the selected faces, each with selected on its left, as point lists
    /// and as the half-edges that make them.
    pub fn boundary(&self, sel: &[bool]) -> Vec<Vec<usize>> {
        let arr = &self.arr;
        let nh = arr.from.len();
        let is_b: Vec<bool> = (0..nh)
            .map(|h| self.left_selected(sel, h) && !self.left_selected(sel, h ^ 1))
            .collect();
        let mut used = vec![false; nh];
        let mut rings = Vec::new();
        for h0 in 0..nh {
            if !is_b[h0] || used[h0] {
                continue;
            }
            let mut ring = Vec::new();
            let mut h = h0;
            let mut guard = 0;
            loop {
                used[h] = true;
                ring.push(h);
                // Rotate clockwise about the destination from the twin until a boundary half-edge.
                let mut g = arr.next[h];
                let mut turns = 0;
                while !is_b[g] && turns < nh {
                    g = arr.next[g ^ 1];
                    turns += 1;
                }
                h = g;
                guard += 1;
                if h == h0 || used[h] || guard > nh {
                    break;
                }
            }
            rings.push(ring);
        }
        rings
    }

    /// The selected faces as a region in canonical form.
    pub fn region(&self, sel: &[bool]) -> Region {
        let rings: Vec<Vec<P>> = self
            .boundary(sel)
            .into_iter()
            .map(|r| r.into_iter().map(|h| self.arr.org(h)).collect())
            .collect();
        rings_to_region(rings)
    }
}

/// The boolean operations of 3.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    /// Union of a list, non-zero rule.
    Union,
    /// Intersection of all.
    Intersection,
    /// The first less the rest.
    Difference,
}

/// Rings of a region, each read with its inside on the left.
pub fn region_rings(r: &Region) -> Vec<Vec<P>> {
    r.rings().into_iter().map(<[P]>::to_vec).collect()
}

/// A boolean over regions (3.3).
pub fn boolean(op: BoolOp, regions: &[Region]) -> Result<Region, GeomDiag> {
    if regions.is_empty() {
        return Ok(Region::default());
    }
    let groups: Vec<Vec<Vec<P>>> = match op {
        BoolOp::Union => vec![regions.iter().flat_map(region_rings).collect()],
        BoolOp::Intersection => regions.iter().map(region_rings).collect(),
        BoolOp::Difference => vec![
            region_rings(&regions[0]),
            regions[1..].iter().flat_map(region_rings).collect(),
        ],
    };
    let n = regions.len();
    let ov = wind(&groups)?;
    let sel = match op {
        BoolOp::Union => ov.select(&|w| w[0] != 0),
        BoolOp::Intersection => ov.select(&|w| w.iter().all(|&x| x != 0) && w.len() == n),
        BoolOp::Difference => ov.select(&|w| w[0] != 0 && w.get(1).copied().unwrap_or(0) == 0),
    };
    Ok(ov.region(&sel))
}

/// `geom.internal` for a failed guarantee check.
pub fn internal(op: &str, what: &str) -> GeomDiag {
    GeomDiag::new(
        Code::Internal,
        format!("a guarantee check failed in `{op}`: {what}"),
    )
    .with("op", op)
}
