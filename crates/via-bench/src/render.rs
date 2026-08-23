//! Fabric rendering at metre scale. Visual inspection is a first-class
//! instrument (ADR 0008 D10): the eye is the discovery channel for
//! unmodelled failure modes, so every measurement ships with a rendered
//! panel. Ported from spikes/townfabric; the terrain base and parcel
//! layers stayed behind with the generator.

use image::{Rgb, RgbImage};

use crate::font;
use crate::geom::*;
use crate::graph::{Class, Graph};
use crate::Building;

pub struct Canvas {
    pub img: RgbImage,
    /// Pixels per metre.
    pub ppm: f64,
    /// World point rendered at the image centre.
    pub centre: P2,
}

impl Canvas {
    pub fn new(w: u32, h: u32, ppm: f64, centre: P2) -> Self {
        Self {
            img: RgbImage::from_pixel(w, h, Rgb([236, 232, 222])),
            ppm,
            centre,
        }
    }

    #[inline]
    pub fn px(&self, p: P2) -> (f64, f64) {
        (
            self.img.width() as f64 * 0.5 + (p[0] - self.centre[0]) * self.ppm,
            self.img.height() as f64 * 0.5 - (p[1] - self.centre[1]) * self.ppm,
        )
    }

    #[inline]
    pub fn blend(&mut self, x: i64, y: i64, c: [u8; 3], a: f64) {
        if x < 0 || y < 0 || x >= self.img.width() as i64 || y >= self.img.height() as i64 {
            return;
        }
        let p = self.img.get_pixel_mut(x as u32, y as u32);
        for (ch, &v) in p.0.iter_mut().zip(c.iter()) {
            *ch = (*ch as f64 * (1.0 - a) + v as f64 * a).round() as u8;
        }
    }

    pub fn fill_poly(&mut self, poly: &[P2], c: [u8; 3], a: f64) {
        if poly.len() < 3 {
            return;
        }
        let pts: Vec<(f64, f64)> = poly.iter().map(|&p| self.px(p)).collect();
        let ymin = pts
            .iter()
            .map(|p| p.1)
            .fold(f64::INFINITY, f64::min)
            .floor() as i64;
        let ymax = pts
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil() as i64;
        for y in ymin..=ymax {
            let yc = y as f64 + 0.5;
            let mut xs: Vec<f64> = Vec::new();
            for i in 0..pts.len() {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % pts.len()];
                if (y0 > yc) != (y1 > yc) {
                    xs.push(x0 + (yc - y0) / (y1 - y0) * (x1 - x0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                for x in pair[0].floor() as i64..=pair[1].ceil() as i64 {
                    let xc = x as f64 + 0.5;
                    if xc >= pair[0] && xc <= pair[1] {
                        self.blend(x, y, c, a);
                    }
                }
            }
        }
    }

    pub fn line_px(&mut self, a: P2, b: P2, width_px: f64, c: [u8; 3], alpha: f64) {
        let (x0, y0) = self.px(a);
        let (x1, y1) = self.px(b);
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()).ceil() as i64).max(1);
        let r = (width_px * 0.5).max(0.5);
        let ri = r.ceil() as i64;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            let x = x0 + (x1 - x0) * t;
            let y = y0 + (y1 - y0) * t;
            for oy in -ri..=ri {
                for ox in -ri..=ri {
                    let d = ((ox * ox + oy * oy) as f64).sqrt();
                    if d <= r {
                        self.blend(x as i64 + ox, y as i64 + oy, c, alpha * (r - d).min(1.0));
                    }
                }
            }
        }
    }

    pub fn stroke_poly(&mut self, poly: &[P2], width_px: f64, c: [u8; 3], alpha: f64) {
        for i in 0..poly.len() {
            self.line_px(poly[i], poly[(i + 1) % poly.len()], width_px, c, alpha);
        }
    }

    /// Streets, drawn at the width their class carries. Declared render
    /// change vs the spike's reference panels: widths are 12/8/5 m
    /// (the spike's synthetic-graph constants) rather than the 16/8/4 m
    /// its reference driver used — cosmetic only, no measured number
    /// depends on it.
    pub fn draw_streets(&mut self, g: &Graph) {
        let width_of = |c: Class| -> f64 {
            match c {
                Class::Corridor => 12.0,
                Class::Street => 8.0,
                Class::Lane => 5.0,
            }
        };
        for ed in &g.edges {
            let w = (width_of(ed.class) * self.ppm).max(1.0);
            self.line_px(g.nodes[ed.a], g.nodes[ed.b], w + 1.4, [120, 110, 96], 0.85);
        }
        for ed in &g.edges {
            let w = (width_of(ed.class) * self.ppm).max(0.8);
            self.line_px(g.nodes[ed.a], g.nodes[ed.b], w, [246, 242, 234], 1.0);
        }
    }

    pub fn draw_blocks(&mut self, blocks: &[Vec<P2>]) {
        for b in blocks {
            self.fill_poly(b, [206, 200, 182], 0.9);
        }
    }

    /// Buildings, shaded by storeys where tagged; untagged renders as a
    /// single storey rather than falling through to black.
    pub fn draw_buildings(&mut self, buildings: &[Building]) {
        let max_s = buildings
            .iter()
            .map(|b| b.storeys)
            .filter(|s| s.is_finite())
            .fold(1.0, f64::max);
        for b in buildings {
            let storeys = if b.storeys.is_finite() {
                b.storeys
            } else {
                1.0
            };
            let t = ((storeys - 1.0) / (max_s - 1.0).max(1.0)).clamp(0.0, 1.0);
            let c = [
                (150.0 - 60.0 * t) as u8,
                (108.0 - 48.0 * t) as u8,
                (92.0 - 44.0 * t) as u8,
            ];
            self.fill_poly(&b.poly, c, 0.96);
            self.stroke_poly(&b.poly, 1.0, [58, 42, 36], 0.9);
        }
    }

    pub fn label(&mut self, x: i32, y: i32, text: &str, scale: u32) {
        font::draw_text_halo(
            &mut self.img,
            x,
            y,
            text,
            scale,
            [22, 20, 18],
            [246, 244, 238],
        );
    }

    /// Outline the window a zoom panel shows.
    pub fn mark_window(&mut self, centre: P2, half_m: f64) {
        let poly = [
            [centre[0] - half_m, centre[1] - half_m],
            [centre[0] + half_m, centre[1] - half_m],
            [centre[0] + half_m, centre[1] + half_m],
            [centre[0] - half_m, centre[1] + half_m],
        ];
        self.stroke_poly(&poly, 3.0, [196, 60, 40], 0.95);
    }

    /// Scale bar of `len_m` metres, bottom-left.
    pub fn scale_bar(&mut self, len_m: f64, label: &str) {
        let h = self.img.height() as f64;
        let x0 = 24.0;
        let y0 = h - 34.0;
        let px = len_m * self.ppm;
        for x in 0..(px as i64) {
            for y in 0..6 {
                self.blend(x0 as i64 + x, y0 as i64 + y, [30, 28, 26], 1.0);
            }
        }
        font::draw_text_halo(
            &mut self.img,
            x0 as i32,
            (y0 - 20.0) as i32,
            label,
            2,
            [22, 20, 18],
            [246, 244, 238],
        );
    }
}

/// The building-densest point: centre of the `half_m` window holding the
/// most building centroids, scanned on a 25 m lattice. Deterministic
/// (ties resolve to the first in scan order). Declared change vs the
/// spike, which evaluated building centroids as candidate centres: the
/// lattice scan is insensitive to building enumeration order. Render-only
/// — it moves the zoom window, never a number.
pub fn densest_point(buildings: &[Building], half_m: f64) -> P2 {
    if buildings.is_empty() {
        return [0.0, 0.0];
    }
    let cents: Vec<P2> = buildings
        .iter()
        .map(|b| polygon_centroid(&b.poly))
        .collect();
    let (mut minx, mut miny) = (f64::INFINITY, f64::INFINITY);
    let (mut maxx, mut maxy) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for c in &cents {
        minx = minx.min(c[0]);
        miny = miny.min(c[1]);
        maxx = maxx.max(c[0]);
        maxy = maxy.max(c[1]);
    }
    let step = 25.0;
    let mut best = (0usize, [0.0, 0.0]);
    let mut y = miny;
    while y <= maxy {
        let mut x = minx;
        while x <= maxx {
            let n = cents
                .iter()
                .filter(|c| (c[0] - x).abs() <= half_m && (c[1] - y).abs() <= half_m)
                .count();
            if n > best.0 {
                best = (n, [x, y]);
            }
            x += step;
        }
        y += step;
    }
    best.1
}
