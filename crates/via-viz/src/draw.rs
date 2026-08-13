//! Minimal raster drawing: alpha blend + thick Bresenham lines.

use image::RgbImage;

pub fn blend(img: &mut RgbImage, x: i32, y: i32, color: [u8; 3], alpha: f64) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    for (ch, &c) in p.0.iter_mut().zip(color.iter()) {
        *ch = (*ch as f64 * (1.0 - alpha) + c as f64 * alpha).round() as u8;
    }
}

fn stamp(img: &mut RgbImage, x: i32, y: i32, width: u32, color: [u8; 3], alpha: f64) {
    let r = (width as i32 - 1) / 2;
    let r_hi = width as i32 - 1 - r;
    for oy in -r..=r_hi {
        for ox in -r..=r_hi {
            blend(img, x + ox, y + oy, color, alpha);
        }
    }
}

/// Bresenham line with a square brush of side `width`.
#[allow(clippy::too_many_arguments)]
pub fn thick_line(
    img: &mut RgbImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    width: u32,
    color: [u8; 3],
    alpha: f64,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        stamp(img, x, y, width, color, alpha);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
