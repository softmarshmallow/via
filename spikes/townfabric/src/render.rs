//! Fabric rendering at metre scale: terrain base, blocks, streets by
//! class, parcel boundaries, building footprints.

use image::{Rgb, RgbImage};

use crate::font;
use crate::geom::*;
use crate::graph::{Class, Graph};
use crate::parcels::{Building, Parcel};
use crate::patch::Patch;

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
    pub fn world(&self, x: f64, y: f64) -> P2 {
        [
            self.centre[0] + (x - self.img.width() as f64 * 0.5) / self.ppm,
            self.centre[1] - (y - self.img.height() as f64 * 0.5) / self.ppm,
        ]
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

    /// Terrain base: water, and a shaded relief from the DEM patch.
    pub fn draw_terrain(&mut self, patch: &Patch) {
        let (w, h) = (self.img.width(), self.img.height());
        for y in 0..h {
            for x in 0..w {
                let p = self.world(x as f64 + 0.5, y as f64 + 0.5);
                if patch.is_water(p) {
                    self.img.put_pixel(x, y, Rgb([150, 178, 200]));
                    continue;
                }
                // Illuminate from the north-west, as the terrain map pack
                // does. The sampling distance is wide on purpose: at 200 m
                // DEM cells a short baseline renders the cell grid itself.
                let d = 90.0;
                let ex = patch.elevation([p[0] + d, p[1]]) - patch.elevation([p[0] - d, p[1]]);
                let ey = patch.elevation([p[0], p[1] + d]) - patch.elevation([p[0], p[1] - d]);
                let shade = (0.72 - 1.6 * (ex - ey) / (2.0 * d)).clamp(0.45, 1.0);
                let base = [206.0, 202.0, 190.0];
                let c = [
                    (base[0] * shade) as u8,
                    (base[1] * shade) as u8,
                    (base[2] * shade) as u8,
                ];
                self.img.put_pixel(x, y, Rgb(c));
            }
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

    /// Streets, drawn at the width their class carries.
    pub fn draw_streets(&mut self, g: &Graph, width_of: impl Fn(Class) -> f64) {
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

    pub fn draw_parcels(&mut self, parcels: &[Parcel]) {
        for p in parcels {
            self.fill_poly(&p.poly, [228, 222, 204], 0.85);
            self.stroke_poly(&p.poly, 1.0, [150, 140, 122], 0.9);
        }
    }

    /// Buildings, shaded by storeys: the taller, the darker. Storey counts
    /// are often absent from real data (OSM rarely tags building:levels),
    /// and a non-finite value must not fall through to black — that made
    /// reference panels look like a different map from the synthetic ones.
    pub fn draw_buildings(&mut self, buildings: &[Building]) {
        let max_s = buildings
            .iter()
            .map(|b| b.storeys)
            .filter(|s| s.is_finite())
            .fold(1.0, f64::max);
        for b in buildings {
            let storeys = if b.storeys.is_finite() { b.storeys } else { 1.0 };
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

/// Lay panels out in a row with white gutters and a caption bar.
pub fn compose(panels: &[RgbImage], cols: u32, margin: u32, caption: &str) -> RgbImage {
    let pw = panels.iter().map(|p| p.width()).max().unwrap_or(1);
    let ph = panels.iter().map(|p| p.height()).max().unwrap_or(1);
    let rows = (panels.len() as u32).div_ceil(cols);
    let cap = if caption.is_empty() { 0 } else { 34 };
    let mut sheet = RgbImage::from_pixel(
        pw * cols + margin * (cols + 1),
        ph * rows + margin * (rows + 1) + cap,
        Rgb([255, 255, 255]),
    );
    for (k, p) in panels.iter().enumerate() {
        let ox = margin + (k as u32 % cols) * (pw + margin);
        let oy = cap + margin + (k as u32 / cols) * (ph + margin);
        for y in 0..p.height() {
            for x in 0..p.width() {
                sheet.put_pixel(ox + x, oy + y, *p.get_pixel(x, y));
            }
        }
    }
    if !caption.is_empty() {
        font::draw_text(&mut sheet, margin as i32, 10, caption, 2, [30, 30, 30]);
    }
    sheet
}
