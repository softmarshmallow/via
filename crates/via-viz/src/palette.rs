//! Color ramps. Elevation stops in metres above/below sea level.

pub fn lerp(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f64 + (b[0] as f64 - a[0] as f64) * t).round() as u8,
        (a[1] as f64 + (b[1] as f64 - a[1] as f64) * t).round() as u8,
        (a[2] as f64 + (b[2] as f64 - a[2] as f64) * t).round() as u8,
    ]
}

pub fn ramp(stops: &[(f64, [u8; 3])], v: f64) -> [u8; 3] {
    if v <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        let (v0, c0) = w[0];
        let (v1, c1) = w[1];
        if v <= v1 {
            return lerp(c0, c1, (v - v0) / (v1 - v0));
        }
    }
    stops[stops.len() - 1].1
}

const LAND: &[(f64, [u8; 3])] = &[
    (0.0, [92, 144, 82]),
    (250.0, [130, 160, 92]),
    (600.0, [178, 176, 110]),
    (1000.0, [198, 168, 116]),
    (1500.0, [164, 130, 98]),
    (2000.0, [192, 192, 192]),
    (2600.0, [246, 246, 246]),
];

const OCEAN: &[(f64, [u8; 3])] = &[
    (0.0, [152, 192, 222]),
    (20.0, [112, 162, 206]),
    (80.0, [62, 112, 172]),
    (200.0, [30, 62, 116]),
];

const ACCUM: &[(f64, [u8; 3])] = &[
    (0.0, [20, 24, 46]),
    (0.45, [34, 84, 120]),
    (0.70, [60, 150, 150]),
    (0.88, [220, 210, 90]),
    (1.0, [255, 250, 220]),
];

pub fn land_tint(elev_above_sea_m: f64) -> [u8; 3] {
    ramp(LAND, elev_above_sea_m)
}

pub fn ocean_tint(depth_m: f64) -> [u8; 3] {
    ramp(OCEAN, depth_m)
}

pub fn accumulation(v01: f64) -> [u8; 3] {
    ramp(ACCUM, v01)
}

pub fn scale(c: [u8; 3], k: f64) -> [u8; 3] {
    [
        (c[0] as f64 * k).clamp(0.0, 255.0) as u8,
        (c[1] as f64 * k).clamp(0.0, 255.0) as u8,
        (c[2] as f64 * k).clamp(0.0, 255.0) as u8,
    ]
}

/// Deterministic hash of a label to [0, 1); golden-angle spaced hues.
pub fn hash01(label: u32) -> f64 {
    let h = via_artifact::seed::splitmix64(label as u64 ^ 0xB5AD_4ECE_DA1C_E2A9);
    let u = ((h >> 11) as f64) / 9_007_199_254_740_992.0;
    (u + label as f64 * 0.618_033_988_749_895).fract()
}

pub fn hsv(h01: f64, s: f64, v: f64) -> [u8; 3] {
    let h6 = (h01.fract() * 6.0).rem_euclid(6.0);
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}
