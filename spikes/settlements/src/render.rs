//! Figures. Maps reuse via-viz's relief so the land reads exactly as it
//! does in the terrain map pack; the human layer is drawn over it.

use image::{Rgb, RgbImage};
use via_viz::VizInput;

use crate::fields::Land;
use crate::font;
use crate::measure::Measurements;
use crate::network::Network;

pub const ERA_COLORS: [[u8; 3]; 5] = [
    [196, 78, 42],
    [38, 108, 176],
    [122, 92, 168],
    [46, 140, 84],
    [198, 150, 36],
];

pub fn viz_input(land: &Land) -> VizInput<'_> {
    // River drawing needs a minimum channel size or every rill is painted;
    // 1.5 km² matches the terrain map pack's river threshold.
    let river_min_cells = (1.5e6 / (land.dx * land.dx)).round().max(1.0) as u64;
    VizInput {
        w: land.w,
        h: land.h,
        dx: land.dx,
        sea_level: land.sea_level_m,
        heights_m: &land.heights_m,
        receivers: &land.receivers,
        discharge_cells: &land.discharge_cells,
        strahler: &land.strahler,
        basin: &[],
        land: &land.land,
        water_depth: &land.water_depth_m,
        lake_min_depth_m: 0.5,
        river_min_cells,
    }
}

fn blend(img: &mut RgbImage, x: i32, y: i32, color: [u8; 3], alpha: f64) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    for (ch, &c) in p.0.iter_mut().zip(color.iter()) {
        *ch = (*ch as f64 * (1.0 - alpha) + c as f64 * alpha).round() as u8;
    }
}

fn disc(img: &mut RgbImage, cx: i32, cy: i32, r: f64, color: [u8; 3], alpha: f64) {
    let ri = r.ceil() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let d = ((dx * dx + dy * dy) as f64).sqrt();
            if d <= r {
                let edge = (r - d).min(1.0);
                blend(img, cx + dx, cy + dy, color, alpha * edge);
            }
        }
    }
}

fn ring(img: &mut RgbImage, cx: i32, cy: i32, r: f64, color: [u8; 3], alpha: f64) {
    let ri = r.ceil() as i32 + 1;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let d = ((dx * dx + dy * dy) as f64).sqrt();
            if (d - r).abs() <= 0.9 {
                blend(img, cx + dx, cy + dy, color, alpha);
            }
        }
    }
}

/// Settlement map: relief + rivers, the corridor network, and one disc per
/// center with area proportional to population.
pub fn settlement_map(
    land: &Land,
    centers: &[(usize, u32, f64)],
    net: &Network,
    era_name: &str,
    m: &Measurements,
    accent: [u8; 3],
) -> RgbImage {
    let inp = viz_input(land);
    let mut img = via_viz::render_relief(&inp);

    // Corridors first, so settlements sit on top of their roads. Drawn two
    // cells wide: one 200 m cell is a hairline at this scale and the
    // network is the point of the panel.
    for (i, &on) in net.built.iter().enumerate() {
        if on {
            let (x, y) = land.xy(i);
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                blend(&mut img, x as i32 + ox, y as i32 + oy, [46, 30, 22], 0.8);
            }
        }
    }

    let max_w = centers.iter().map(|&(_, _, w)| w).fold(0.0, f64::max);
    for &(_, cell, w) in centers {
        let (x, y) = land.xy(cell as usize);
        // Area ∝ population, floored so hamlets stay visible.
        let r = 1.6 + 9.0 * (w / max_w.max(1.0)).sqrt();
        disc(&mut img, x as i32, y as i32, r, accent, 0.92);
        ring(&mut img, x as i32, y as i32, r, [20, 16, 12], 0.75);
    }

    let pad = 6i32;
    font::draw_text_halo(
        &mut img,
        pad,
        pad,
        era_name,
        3,
        [255, 255, 255],
        [20, 20, 20],
    );
    let l2 = format!(
        "{} CENTERS / {} AGGLOM  SPACING {:.1} KM",
        m.centers, m.agglomerations, m.spacing_median_km
    );
    font::draw_text_halo(
        &mut img,
        pad,
        pad + 26,
        &l2,
        2,
        [245, 245, 245],
        [20, 20, 20],
    );
    let l3 = format!(
        "LARGEST {:.0}  PRIMACY {:.0}%  ZETA {:.2}",
        m.largest_agglomeration,
        m.agglomeration_primacy * 100.0,
        m.agglomeration_zeta
    );
    font::draw_text_halo(
        &mut img,
        pad,
        pad + 44,
        &l3,
        2,
        [245, 245, 245],
        [20, 20, 20],
    );
    let l4 = format!(
        "TRAVEL {:.2} H  ROADS {:.0} KM",
        m.mean_travel_hours, m.network_km
    );
    font::draw_text_halo(
        &mut img,
        pad,
        pad + 62,
        &l4,
        2,
        [245, 245, 245],
        [20, 20, 20],
    );
    img
}

/// Travel-time field from one cell, banded into isochrones.
pub fn isochrone_map(
    land: &Land,
    hours: &[f64],
    bands: &[f64],
    label: &str,
    accent: [u8; 3],
    origin: u32,
) -> RgbImage {
    let inp = viz_input(land);
    let mut img = via_viz::render_hillshade_gray(&inp);
    for (i, &t) in hours.iter().enumerate() {
        if !t.is_finite() {
            continue;
        }
        let mut band = None;
        for (k, &b) in bands.iter().enumerate() {
            if t <= b {
                band = Some(k);
                break;
            }
        }
        let Some(k) = band else { continue };
        // Widely separated alphas: the bands must read as rings.
        let alpha: f64 = [0.80, 0.55, 0.32, 0.16][k.min(3)];
        let (x, y) = land.xy(i);
        blend(&mut img, x as i32, y as i32, accent, alpha);
    }
    let (ox, oy) = land.xy(origin as usize);
    disc(&mut img, ox as i32, oy as i32, 4.0, [255, 255, 255], 0.95);
    ring(&mut img, ox as i32, oy as i32, 4.0, [20, 20, 20], 0.9);
    font::draw_text_halo(&mut img, 6, 6, label, 3, [255, 255, 255], [20, 20, 20]);
    img
}

/// Continuous field over a ramp, with a label.
pub fn field_map(land: &Land, field: &[f64], stops: &[(f64, [u8; 3])], label: &str) -> RgbImage {
    let inp = viz_input(land);
    let mut img = via_viz::render_scalar(&inp, field, stops);
    font::draw_text_halo(&mut img, 6, 6, label, 3, [255, 255, 255], [20, 20, 20]);
    img
}

fn line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3], width: i32) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let x = x0 as f64 + (x1 - x0) as f64 * t;
        let y = y0 as f64 + (y1 - y0) as f64 * t;
        for oy in 0..width {
            for ox in 0..width {
                blend(img, x as i32 + ox, y as i32 + oy, color, 1.0);
            }
        }
    }
}

/// Log–log rank–size plot: one polyline per era, plus the ζ = 1 reference.
pub fn rank_size_chart(series: &[(String, Vec<f64>, [u8; 3])], w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(w, h, Rgb([252, 252, 250]));
    let (l, r, t, b) = (72i32, 24i32, 40i32, 52i32);
    let pw = w as i32 - l - r;
    let ph = h as i32 - t - b;

    let mut max_ls = f64::MIN;
    let mut min_ls = f64::MAX;
    let mut max_lr = 0.0f64;
    for (_, sizes, _) in series {
        for (k, &s) in sizes.iter().enumerate() {
            if s <= 0.0 {
                continue;
            }
            max_ls = max_ls.max(s.ln());
            min_ls = min_ls.min(s.ln());
            max_lr = max_lr.max(((k + 1) as f64).ln());
        }
    }
    if !max_ls.is_finite() || !min_ls.is_finite() {
        return img;
    }
    let sx = |ls: f64| l + ((ls - min_ls) / (max_ls - min_ls) * pw as f64) as i32;
    let sy = |lr: f64| t + ph - (lr / max_lr.max(1e-9) * ph as f64) as i32;

    // Axes.
    line(&mut img, l, t, l, t + ph, [40, 40, 40], 1);
    line(&mut img, l, t + ph, l + pw, t + ph, [40, 40, 40], 1);

    // Decade ticks on both log axes.
    let mut e = (min_ls / std::f64::consts::LN_10).floor();
    while e * std::f64::consts::LN_10 <= max_ls {
        let ls = e * std::f64::consts::LN_10;
        if ls >= min_ls {
            let x = sx(ls);
            line(&mut img, x, t + ph, x, t + ph + 5, [40, 40, 40], 1);
            let lbl = format!("1E{}", e as i32);
            font::draw_text(
                &mut img,
                x - font::text_width(&lbl, 2) as i32 / 2,
                t + ph + 9,
                &lbl,
                2,
                [40, 40, 40],
            );
        }
        e += 1.0;
    }
    let mut e = 0.0f64;
    while e * std::f64::consts::LN_10 <= max_lr {
        let y = sy(e * std::f64::consts::LN_10);
        line(&mut img, l - 5, y, l, y, [40, 40, 40], 1);
        let lbl = format!("1E{}", e as i32);
        font::draw_text(
            &mut img,
            l - 9 - font::text_width(&lbl, 2) as i32,
            y - font::text_height(2) as i32 / 2,
            &lbl,
            2,
            [40, 40, 40],
        );
        e += 1.0;
    }

    // ζ = 1 reference through the largest point of the first series.
    if let Some((_, sizes, _)) = series.first() {
        if let Some(&big) = sizes.first() {
            let x0 = sx(big.ln());
            let y0 = sy(0.0);
            let x1 = sx(min_ls);
            let y1 = sy((big.ln() - min_ls).max(0.0));
            let mut d = 0;
            let steps = (x0 - x1).abs().max((y1 - y0).abs()).max(1);
            for s in 0..=steps {
                let tt = s as f64 / steps as f64;
                let x = x0 as f64 + (x1 - x0) as f64 * tt;
                let y = y0 as f64 + (y1 - y0) as f64 * tt;
                d += 1;
                if d % 8 < 4 {
                    blend(&mut img, x as i32, y as i32, [150, 150, 150], 1.0);
                }
            }
        }
    }

    for (_, sizes, color) in series {
        let mut prev: Option<(i32, i32)> = None;
        for (k, &s) in sizes.iter().enumerate() {
            if s <= 0.0 {
                continue;
            }
            let x = sx(s.ln());
            let y = sy(((k + 1) as f64).ln());
            if let Some((px, py)) = prev {
                line(&mut img, px, py, x, y, *color, 2);
            }
            prev = Some((x, y));
        }
    }

    font::draw_text(
        &mut img,
        l,
        10,
        "RANK-SIZE OF AGGLOMERATIONS (DASHED: ZETA=1)",
        2,
        [30, 30, 30],
    );
    let mut lx = l;
    for (name, _, color) in series {
        disc(&mut img, lx + 4, h as i32 - 14, 4.0, *color, 1.0);
        font::draw_text(&mut img, lx + 12, h as i32 - 18, name, 2, [30, 30, 30]);
        lx += 14 + font::text_width(name, 2) as i32 + 14;
    }
    img
}

/// Nearest-neighbour spacing histogram, one filled series per era.
pub fn spacing_chart(series: &[(String, Vec<f64>, [u8; 3])], w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(w, h, Rgb([252, 252, 250]));
    let (l, r, t, b) = (56i32, 20i32, 40i32, 52i32);
    let pw = w as i32 - l - r;
    let ph = h as i32 - t - b;
    let max_km = series
        .iter()
        .flat_map(|(_, v, _)| v.iter())
        .cloned()
        .fold(0.0f64, f64::max)
        .max(1.0);
    let bins = 28usize;
    let bin_w = max_km / bins as f64;

    let mut hists: Vec<Vec<f64>> = Vec::new();
    let mut peak = 0.0f64;
    for (_, v, _) in series {
        let mut hcount = vec![0.0f64; bins];
        for &x in v {
            let k = ((x / bin_w) as usize).min(bins - 1);
            hcount[k] += 1.0;
        }
        let n = v.len().max(1) as f64;
        for c in hcount.iter_mut() {
            *c /= n;
            peak = peak.max(*c);
        }
        hists.push(hcount);
    }
    line(&mut img, l, t, l, t + ph, [40, 40, 40], 1);
    line(&mut img, l, t + ph, l + pw, t + ph, [40, 40, 40], 1);

    for ((_, _, color), hist) in series.iter().zip(hists.iter()) {
        for (i, &c) in hist.iter().enumerate() {
            if c <= 0.0 {
                continue;
            }
            let x0 = l + (i as f64 / bins as f64 * pw as f64) as i32;
            let x1 = l + ((i + 1) as f64 / bins as f64 * pw as f64) as i32;
            let y0 = t + ph - (c / peak * ph as f64) as i32;
            for x in x0..x1.max(x0 + 1) {
                for y in y0..(t + ph) {
                    blend(&mut img, x, y, *color, 0.38);
                }
                blend(&mut img, x, y0, *color, 1.0);
                blend(&mut img, x, y0 + 1, *color, 1.0);
            }
        }
    }
    for k in 0..=4 {
        let km = max_km * k as f64 / 4.0;
        let x = l + (k as f64 / 4.0 * pw as f64) as i32;
        line(&mut img, x, t + ph, x, t + ph + 5, [40, 40, 40], 1);
        let lbl = format!("{km:.0}");
        font::draw_text(
            &mut img,
            x - font::text_width(&lbl, 2) as i32 / 2,
            t + ph + 9,
            &lbl,
            2,
            [40, 40, 40],
        );
    }
    font::draw_text(
        &mut img,
        l,
        10,
        "NEAREST-NEIGHBOUR SPACING (KM)",
        2,
        [30, 30, 30],
    );
    let mut lx = l;
    for (name, _, color) in series {
        disc(&mut img, lx + 4, h as i32 - 14, 4.0, *color, 1.0);
        font::draw_text(&mut img, lx + 12, h as i32 - 18, name, 2, [30, 30, 30]);
        lx += 14 + font::text_width(name, 2) as i32 + 14;
    }
    img
}

/// Lay panels out in a grid with white gutters and an optional caption bar.
pub fn compose(panels: &[RgbImage], cols: u32, margin: u32, caption: &str) -> RgbImage {
    let pw = panels[0].width();
    let ph = panels[0].height();
    let rows = (panels.len() as u32).div_ceil(cols);
    let cap_h = if caption.is_empty() { 0 } else { 34 };
    let mut sheet = RgbImage::from_pixel(
        pw * cols + margin * (cols + 1),
        ph * rows + margin * (rows + 1) + cap_h,
        Rgb([255, 255, 255]),
    );
    for (k, p) in panels.iter().enumerate() {
        let cx = k as u32 % cols;
        let cy = k as u32 / cols;
        let ox = margin + cx * (pw + margin);
        let oy = cap_h + margin + cy * (ph + margin);
        for y in 0..ph.min(p.height()) {
            for x in 0..pw.min(p.width()) {
                sheet.put_pixel(ox + x, oy + y, *p.get_pixel(x, y));
            }
        }
    }
    if !caption.is_empty() {
        font::draw_text(&mut sheet, margin as i32, 10, caption, 2, [30, 30, 30]);
    }
    sheet
}
