//! A weights field written per chunk (`build.md` 6.2), with no limit on materials.

use crate::error::RasterError;
use crate::npy::u8_file;
use crate::raster::Raster;
use crate::window::{post_mm, Window};
use std::collections::BTreeMap;
use via_num::Grid;

/// One chunk of a weights field: the materials with a byte above 0 somewhere in it, in id order,
/// and one plane of bytes for each, planes in that order, each in post order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightsChunk {
    pub window: Window,
    pub materials: Vec<String>,
    pub bytes: Vec<u8>,
}

impl WeightsChunk {
    /// The chunk's array file, `|u1`, shape `(planes, rows, columns)`; `None` when it has no plane
    /// (`{ "file": null, "materials": [] }`).
    pub fn file(&self) -> Option<Vec<u8>> {
        (!self.materials.is_empty()).then(|| {
            u8_file(
                &[
                    self.materials.len(),
                    self.window.height() as usize,
                    self.window.width() as usize,
                ],
                &self.bytes,
            )
        })
    }
}

/// The bytes of one post (`build.md` 6.2): `S` the sum of the weights in id order; 0 everywhere if
/// `S` is 0; else `q = w * 255 / S` per material, bytes `floor(q)`, then one more to each of the
/// largest remainders `q - floor(q)` until the sum is 255, ties to the lower material id
/// (Hamilton's largest-remainder method).
pub fn post_bytes(weights: &[f64]) -> Vec<u8> {
    let s = via_num::sum(weights);
    let mut out = vec![0u8; weights.len()];
    if s == 0.0 {
        return out;
    }
    let mut rest: Vec<(f64, usize)> = Vec::with_capacity(weights.len());
    let mut total: u32 = 0;
    for (k, &w) in weights.iter().enumerate() {
        let q = (w * 255.0) / s;
        let f = q.floor();
        out[k] = f as u8;
        total += f as u32;
        rest.push((q - f, k));
    }
    // Largest remainder first; equal remainders by the lower id.
    rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut deficit = 255u32.saturating_sub(total);
    for &(_, k) in &rest {
        if deficit == 0 {
            break;
        }
        out[k] = out[k].saturating_add(1);
        deficit -= 1;
    }
    out
}

/// A chunk's bytes from a weights field's narrowed planes (each must hold `window`; a material
/// without a plane is 0). A weight below 0 or not finite is `build.internal`. A material's plane
/// is kept when some post of the chunk gives it a byte above 0, so a material's bytes are the same
/// in any chunking (bytes are worked out per post, before planes are chosen).
pub fn weights_chunk(
    grid: &Grid,
    planes: &BTreeMap<String, Raster<f32>>,
    window: Window,
) -> Result<WeightsChunk, RasterError> {
    let names: Vec<&String> = planes.keys().collect();
    let n = names.len();
    let posts = window.len();
    let mut all = vec![0u8; n * posts];
    let mut used = vec![false; n];
    let mut w = vec![0.0f64; n];
    for (p, (i, j)) in window.posts().enumerate() {
        for (k, name) in names.iter().enumerate() {
            let v = f64::from(planes[*name].get(i, j).unwrap_or(0.0));
            if !v.is_finite() || v < 0.0 {
                return Err(RasterError::WeightInvalid {
                    at: post_mm(grid, i, j),
                    material: (*name).clone(),
                });
            }
            w[k] = v;
        }
        for (k, b) in post_bytes(&w).into_iter().enumerate() {
            all[k * posts + p] = b;
            used[k] |= b > 0;
        }
    }
    let mut materials = Vec::new();
    let mut bytes = Vec::new();
    for k in 0..n {
        if used[k] {
            materials.push(names[k].clone());
            bytes.extend_from_slice(&all[k * posts..(k + 1) * posts]);
        }
    }
    Ok(WeightsChunk {
        window,
        materials,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_sum_to_255_or_0() {
        assert_eq!(post_bytes(&[0.0, 0.0]), vec![0, 0]);
        assert_eq!(post_bytes(&[0.75, 0.25]), vec![191, 64]);
        // Thirds: 85 each, exact.
        assert_eq!(post_bytes(&[1.0 / 3.0; 3]), vec![85, 85, 85]);
        // Halves of one: a tie in the remainders goes to the lower id.
        assert_eq!(post_bytes(&[0.5, 0.5]), vec![128, 127]);
        // An unpainted post is scaled to 255.
        assert_eq!(post_bytes(&[0.25]), vec![255]);
        for k in 1..40u32 {
            let w: Vec<f64> = (0..11)
                .map(|m| f64::from((k * 7 + m * 13) % 17) / 170.0)
                .collect();
            let b = post_bytes(&w);
            assert_eq!(b.iter().map(|&x| u32::from(x)).sum::<u32>(), 255);
        }
    }

    #[test]
    fn eleven_materials_and_planes_per_chunk() {
        // No limit of eight: eleven materials, a chunk names only its own.
        let g = Grid::new([0, 0, 10_000, 1_000], 1_000).unwrap();
        let all = Window::whole(&g);
        let mut planes = BTreeMap::new();
        for m in 0..11u64 {
            let p = Raster::from_fn(all, |i, _| Ok::<_, ()>(if i == m { 1.0f32 } else { 0.0 }))
                .unwrap();
            planes.insert(format!("m{m:02}"), p);
        }
        let c = weights_chunk(&g, &planes, all).unwrap();
        assert_eq!(c.materials.len(), 11);
        assert_eq!(c.file().unwrap().len() % 64, (11 * 11 * 2) % 64);
        let left = weights_chunk(&g, &planes, Window::new(0, 0, 3, 2).unwrap()).unwrap();
        assert_eq!(left.materials, vec!["m00", "m01", "m02"]);
        // The same material's bytes in any chunking.
        for (k, name) in left.materials.iter().enumerate() {
            let kk = c.materials.iter().position(|m| m == name).unwrap();
            for (p, (i, j)) in left.window.posts().enumerate() {
                let q = all.offset(i, j).unwrap();
                assert_eq!(left.bytes[k * 6 + p], c.bytes[kk * all.len() + q]);
            }
        }
        let none = BTreeMap::new();
        let empty = weights_chunk(&g, &none, all).unwrap();
        assert!(empty.file().is_none());
    }

    #[test]
    fn a_negative_weight_is_internal() {
        let g = Grid::new([0, 0, 1_000, 1_000], 1_000).unwrap();
        let mut planes = BTreeMap::new();
        planes.insert("a".to_string(), Raster::filled(Window::whole(&g), -0.5f32));
        assert_eq!(
            weights_chunk(&g, &planes, Window::whole(&g)).map_err(|e| e.code().as_str()),
            Err("build.internal")
        );
    }
}
