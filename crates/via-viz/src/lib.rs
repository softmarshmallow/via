//! Map-pack renderers for the terrain stage. Plan-view PNGs: hillshaded
//! relief with rivers, log flow accumulation, basins, raw hillshade.
//! Deliberately font-free in v1; captions live in the QA report.

mod draw;
pub mod palette;

use image::{Rgb, RgbImage};
use rayon::prelude::*;

pub struct VizInput<'a> {
    pub w: u32,
    pub h: u32,
    pub dx: f64,
    pub sea_level: f64,
    pub heights_m: &'a [f64],
    pub receivers: &'a [u32],
    /// Discharge in equivalent cells — the flow metric rivers are extracted
    /// with; widths and accumulation must use the same metric or the maps
    /// contradict the climate signal they exist to show.
    pub discharge_cells: &'a [f64],
    pub strahler: &'a [u32],
    pub basin: &'a [u32],
    pub land: &'a [bool],
    /// Standing-water depth spectrum (m); cells at or above
    /// `lake_min_depth_m` render as water surfaces.
    pub water_depth: &'a [f64],
    pub lake_min_depth_m: f64,
    pub river_min_cells: u64,
}

impl VizInput<'_> {
    #[inline]
    fn is_lake(&self, i: usize) -> bool {
        self.water_depth[i] >= self.lake_min_depth_m
    }

    /// Water for rendering purposes: ocean or lake surface.
    #[inline]
    fn is_water(&self, i: usize) -> bool {
        !self.land[i] || self.is_lake(i)
    }
}

/// Horn hillshade (the standard GIS formulation), sun at azimuth 315°,
/// altitude 45°. Returns illumination in [0, 1].
pub fn hillshade(w: u32, h: u32, dx: f64, heights: &[f64]) -> Vec<f32> {
    let zenith = (90.0f64 - 45.0).to_radians();
    let az_math = ((360.0f64 - 315.0) + 90.0).rem_euclid(360.0).to_radians();
    let at = |x: i64, y: i64| -> f64 {
        let xc = x.clamp(0, w as i64 - 1);
        let yc = y.clamp(0, h as i64 - 1);
        heights[(yc * w as i64 + xc) as usize]
    };
    (0..(w as usize * h as usize))
        .into_par_iter()
        .map(|i| {
            let x = (i as u32 % w) as i64;
            let y = (i as u32 / w) as i64;
            let (a, b, c) = (at(x - 1, y - 1), at(x, y - 1), at(x + 1, y - 1));
            let (d, f) = (at(x - 1, y), at(x + 1, y));
            let (g, hh, ii) = (at(x - 1, y + 1), at(x, y + 1), at(x + 1, y + 1));
            let dzdx = ((c + 2.0 * f + ii) - (a + 2.0 * d + g)) / (8.0 * dx);
            let dzdy = ((g + 2.0 * hh + ii) - (a + 2.0 * b + c)) / (8.0 * dx);
            let slope = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            let aspect = dzdy.atan2(-dzdx);
            let shade =
                zenith.cos() * slope.cos() + zenith.sin() * slope.sin() * (az_math - aspect).cos();
            shade.clamp(0.0, 1.0) as f32
        })
        .collect()
}

fn draw_rivers(img: &mut RgbImage, inp: &VizInput, color: [u8; 3], alpha: f64, max_extra: u32) {
    let min_m2 = inp.river_min_cells as f64 * inp.dx * inp.dx;
    for i in 0..(inp.w as usize * inp.h as usize) {
        if inp.strahler[i] == 0 || inp.is_lake(i) {
            continue;
        }
        let r = inp.receivers[i] as usize;
        if r == i {
            continue;
        }
        let q_m2 = inp.discharge_cells[i] * inp.dx * inp.dx;
        let extra = ((q_m2 / min_m2).log10().max(0.0) as u32).min(max_extra);
        let (x0, y0) = ((i as u32 % inp.w) as i32, (i as u32 / inp.w) as i32);
        let (x1, y1) = ((r as u32 % inp.w) as i32, (r as u32 / inp.w) as i32);
        draw::thick_line(img, x0, y0, x1, y1, 1 + extra, color, alpha);
    }
}

/// Panel 1: hypsometric tint × hillshade with the river network.
pub fn render_relief(inp: &VizInput) -> RgbImage {
    let shade = hillshade(inp.w, inp.h, inp.dx, inp.heights_m);
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let hm = inp.heights_m[i];
            let s = shade[i] as f64;
            let rgb = if inp.is_lake(i) {
                // Flat water plane tinted by depth; slightly lifted so
                // lakes read lighter than the ocean.
                let base = palette::ocean_tint(inp.water_depth[i]);
                palette::scale(base, 1.12)
            } else if inp.land[i] {
                let base = palette::land_tint(hm - inp.sea_level);
                let coast = has_water_neighbor(inp, x, y);
                let k = if coast { 0.62 } else { 0.5 + 0.5 * s };
                palette::scale(base, k)
            } else {
                let base = palette::ocean_tint(inp.sea_level - hm);
                palette::scale(base, 0.8 + 0.2 * s)
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    draw_rivers(&mut img, inp, [24, 74, 146], 0.85, 3);
    img
}

/// Panel 2: log-scaled flow accumulation (discharge).
pub fn render_accumulation(inp: &VizInput) -> RgbImage {
    let mut max_ln = 0.0f64;
    for i in 0..(inp.w as usize * inp.h as usize) {
        if inp.land[i] {
            max_ln = max_ln.max((1.0 + inp.discharge_cells[i]).ln());
        }
    }
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let rgb = if !inp.is_water(i) && max_ln > 0.0 {
                let v = (1.0 + inp.discharge_cells[i]).ln() / max_ln;
                palette::accumulation(v)
            } else {
                [12, 16, 36]
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    img
}

/// Panel 3: drainage basins (color = hashed outlet id) with rivers.
pub fn render_basins(inp: &VizInput) -> RgbImage {
    let shade = hillshade(inp.w, inp.h, inp.dx, inp.heights_m);
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let rgb = if !inp.is_water(i) {
                let hue = palette::hash01(inp.basin[i]);
                let s = shade[i] as f64;
                let [r, g, b] = palette::hsv(hue, 0.42, 0.92);
                palette::scale([r, g, b], 0.6 + 0.4 * s)
            } else {
                [24, 34, 64]
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    draw_rivers(&mut img, inp, [255, 255, 255], 0.55, 1);
    img
}

/// Panel 4: plain grayscale hillshade — the texture eyeball test.
pub fn render_hillshade_gray(inp: &VizInput) -> RgbImage {
    let shade = hillshade(inp.w, inp.h, inp.dx, inp.heights_m);
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let rgb = if !inp.is_water(i) {
                let v = (shade[i] * 255.0) as u8;
                [v, v, v]
            } else {
                // Flat water: seafloor/lakebed relief would dominate the
                // panel and this panel exists to read the *land* texture.
                [176, 182, 196]
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    img
}

fn has_water_neighbor(inp: &VizInput, x: u32, y: u32) -> bool {
    for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
        let nx = x as i64 + dx;
        let ny = y as i64 + dy;
        if nx >= 0
            && ny >= 0
            && nx < inp.w as i64
            && ny < inp.h as i64
            && inp.is_water((ny * inp.w as i64 + nx) as usize)
        {
            return true;
        }
    }
    false
}

/// Categorical class map (e.g. biomes): class colors modulated by
/// hillshade on land, flat water, rivers overlaid thinly.
pub fn render_classes(inp: &VizInput, class: &[u32], colors: &[[u8; 3]]) -> RgbImage {
    let shade = hillshade(inp.w, inp.h, inp.dx, inp.heights_m);
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let c = colors[(class[i] as usize).min(colors.len() - 1)];
            let rgb = if !inp.is_water(i) {
                palette::scale(c, 0.62 + 0.38 * shade[i] as f64)
            } else {
                c
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    draw_rivers(&mut img, inp, [30, 70, 140], 0.5, 1);
    img
}

/// Continuous scalar field over a color ramp (stops in the field's own
/// units); water cells get a flat neutral tone.
pub fn render_scalar(inp: &VizInput, field: &[f64], stops: &[(f64, [u8; 3])]) -> RgbImage {
    let mut img = RgbImage::new(inp.w, inp.h);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let rgb = if !inp.is_water(i) {
                palette::ramp(stops, field[i])
            } else {
                [190, 198, 208]
            };
            img.put_pixel(x, y, Rgb(rgb));
        }
    }
    img
}

/// Suitability overlay: relief base, suitable cells tinted, reported patches
/// highlighted (rank 1 brightest) with a dark border.
pub fn render_suitability(inp: &VizInput, suitable: &[bool], patch_rank: &[u32]) -> RgbImage {
    let mut img = render_relief(inp);
    let (w, h) = (inp.w as i64, inp.h as i64);
    for y in 0..inp.h {
        for x in 0..inp.w {
            let i = (y * inp.w + x) as usize;
            let rank = patch_rank[i];
            if !suitable[i] && rank == 0 {
                continue;
            }
            let mut border = false;
            if rank > 0 {
                let (xi, yi) = (x as i64, y as i64);
                for (nx, ny) in [(xi - 1, yi), (xi + 1, yi), (xi, yi - 1), (xi, yi + 1)] {
                    if nx >= 0
                        && ny >= 0
                        && nx < w
                        && ny < h
                        && patch_rank[(ny * w + nx) as usize] != rank
                    {
                        border = true;
                        break;
                    }
                }
            }
            let (color, alpha) = if border {
                ([92, 58, 8], 0.85)
            } else {
                match rank {
                    0 => ([250, 240, 150], 0.22),
                    1 => ([255, 210, 40], 0.60),
                    _ => ([245, 165, 60], 0.45),
                }
            };
            draw::blend(&mut img, x as i32, y as i32, color, alpha);
        }
    }
    img
}

/// Marker cells for the affordance panel; all in flat cell indices.
pub struct AffordanceOverlay<'a> {
    /// Crossability spectrum (m²/s) — tints river cells, log-scaled.
    pub crossability: &'a [f32],
    pub confluence_cells: &'a [u32],
    pub pass_cells: &'a [u32],
    pub head_cells: &'a [u32],
    /// QA: basin-boundary col cells (dark red) — diagnostic only.
    pub qa_col_cells: &'a [u32],
    /// QA: Filet change-point cells (cyan) — diagnostic only.
    pub qa_change_point_cells: &'a [u32],
}

fn mark_cells(img: &mut RgbImage, w: u32, cells: &[u32], color: [u8; 3], alpha: f64) {
    for &c in cells {
        let (x, y) = ((c % w) as i32, (c / w) as i32);
        for oy in -1..=1 {
            for ox in -1..=1 {
                draw::blend(img, x + ox, y + oy, color, alpha);
            }
        }
    }
}

/// Affordance panel: relief base, river cells tinted by crossability
/// (log ramp, green = fordable → dark red = extreme), sites marked —
/// confluences white, passes orange, heads of navigation magenta — and
/// the two QA overlays (basin-boundary cols dark red, change-points
/// cyan).
pub fn render_affordances(inp: &VizInput, ov: &AffordanceOverlay) -> RgbImage {
    let mut img = render_relief(inp);
    let max_ln = ov
        .crossability
        .iter()
        .zip(inp.strahler.iter())
        .filter(|(_, &s)| s > 0)
        .map(|(&c, _)| (1.0 + c as f64).ln())
        .fold(0.0f64, f64::max);
    if max_ln > 0.0 {
        for i in 0..(inp.w as usize * inp.h as usize) {
            if inp.strahler[i] == 0 || ov.crossability[i] <= 0.0 {
                continue;
            }
            let v = (1.0 + ov.crossability[i] as f64).ln() / max_ln;
            let color = palette::ramp(
                &[
                    (0.0, [96, 200, 96]),
                    (0.5, [230, 190, 60]),
                    (1.0, [150, 30, 30]),
                ],
                v,
            );
            let (x, y) = ((i as u32 % inp.w) as i32, (i as u32 / inp.w) as i32);
            draw::blend(&mut img, x, y, color, 0.9);
        }
    }
    mark_cells(&mut img, inp.w, ov.qa_col_cells, [120, 20, 20], 0.55);
    mark_cells(
        &mut img,
        inp.w,
        ov.qa_change_point_cells,
        [40, 220, 220],
        0.7,
    );
    mark_cells(&mut img, inp.w, ov.confluence_cells, [255, 255, 255], 0.85);
    mark_cells(&mut img, inp.w, ov.pass_cells, [255, 150, 30], 0.95);
    mark_cells(&mut img, inp.w, ov.head_cells, [230, 40, 200], 0.95);
    img
}

/// Harbour panel: land as dim relief tint; open ocean dark; coastal
/// water colored by wave fetch (log ramp: bright green sheltered →
/// deep blue exposed); sediment-supply penalty overlaid in red.
pub fn render_harbour(inp: &VizInput, fetch_m: &[f32], sediment: &[f32]) -> RgbImage {
    let n = inp.w as usize * inp.h as usize;
    let max_sed_ln = sediment
        .iter()
        .map(|&s| (1.0 + s as f64).ln())
        .fold(0.0f64, f64::max);
    let shade = hillshade(inp.w, inp.h, inp.dx, inp.heights_m);
    let mut img = RgbImage::new(inp.w, inp.h);
    for (i, &sh) in shade.iter().enumerate().take(n) {
        let (x, y) = (i as u32 % inp.w, i as u32 / inp.w);
        let rgb = if inp.land[i] {
            let base = palette::land_tint(inp.heights_m[i] - inp.sea_level);
            palette::scale(base, 0.30 + 0.25 * sh as f64)
        } else {
            [16, 22, 44]
        };
        img.put_pixel(x, y, Rgb(rgb));
    }
    // Coastal-water fetch, dilated to a 3×3 stamp so the one-cell ring
    // reads at map scale; absolute log10(km) stops so shelter structure
    // is not squashed by the cap.
    for (i, &f) in fetch_m.iter().enumerate() {
        if f <= 0.0 {
            continue;
        }
        let v = (1.0 + f as f64 / 1000.0).log10();
        let color = palette::ramp(
            &[
                (0.3, [120, 240, 160]),
                (1.0, [235, 205, 70]),
                (1.6, [70, 140, 200]),
                (2.2, [15, 35, 105]),
            ],
            v,
        );
        let (x, y) = ((i as u32 % inp.w) as i32, (i as u32 / inp.w) as i32);
        for oy in -1..=1 {
            for ox in -1..=1 {
                let (nx, ny) = (x + ox, y + oy);
                if nx >= 0 && ny >= 0 && nx < inp.w as i32 && ny < inp.h as i32 {
                    let ni = (ny as u32 * inp.w + nx as u32) as usize;
                    if !inp.land[ni] {
                        draw::blend(
                            &mut img,
                            nx,
                            ny,
                            color,
                            if ox == 0 && oy == 0 { 0.95 } else { 0.5 },
                        );
                    }
                }
            }
        }
    }
    if max_sed_ln > 0.0 {
        for (i, &sed) in sediment.iter().enumerate() {
            if sed > 0.0 {
                let a = 0.55 * (1.0 + sed as f64).ln() / max_sed_ln;
                let (x, y) = ((i as u32 % inp.w) as i32, (i as u32 / inp.w) as i32);
                draw::blend(&mut img, x, y, [220, 60, 30], a);
            }
        }
    }
    img
}

/// 2×2 composite with white margins: relief | accumulation / basins | shade.
pub fn compose_sheet(panels: &[&RgbImage; 4], margin: u32) -> RgbImage {
    let w = panels[0].width();
    let h = panels[0].height();
    let mut sheet =
        RgbImage::from_pixel(w * 2 + margin * 3, h * 2 + margin * 3, Rgb([255, 255, 255]));
    for (k, p) in panels.iter().enumerate() {
        let ox = margin + (k as u32 % 2) * (w + margin);
        let oy = margin + (k as u32 / 2) * (h + margin);
        for y in 0..h {
            for x in 0..w {
                sheet.put_pixel(ox + x, oy + y, *p.get_pixel(x, y));
            }
        }
    }
    sheet
}
