//! Grid helpers the step needs around the model: exact point-in-ring tests on integer millimetres,
//! a Euclidean distance transform, square minimum filters, sampling between nodes, and order
//! statistics. Every one is a pure function with a fixed visiting order; every float operation is
//! exact or correctly rounded, or goes through `via-num` (R26).

use via_num as num;

/// The cubic Hermite step 0 to 1 between `a` and `b` (`smoothstep` of `formulas.md` 8.1).
pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = num::clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Whether the point lies inside the rings by the even-odd rule: an outline and its holes. Exact:
/// integer millimetres, crossings compared in 128-bit integers. A point on an edge may fall either
/// way, the same way every time.
pub fn in_rings(rings: &[Vec<(i64, i64)>], p: (i64, i64)) -> bool {
    let (px, py) = (p.0 as i128, p.1 as i128);
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for k in 0..n {
            let (x1, y1) = (ring[k].0 as i128, ring[k].1 as i128);
            let (x2, y2) = (ring[(k + 1) % n].0 as i128, ring[(k + 1) % n].1 as i128);
            if (y1 > py) != (y2 > py) {
                // px < x1 + (py - y1) * (x2 - x1) / (y2 - y1), multiplied through by (y2 - y1).
                let lhs = (px - x1) * (y2 - y1);
                let rhs = (py - y1) * (x2 - x1);
                let left = if y2 > y1 { lhs < rhs } else { lhs > rhs };
                if left {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Squared distance, in grid units, from every node to the nearest source node, and that source's
/// index; `None` where there is no source at all. The exact Euclidean transform of Felzenszwalb and
/// Huttenlocher (2012), columns then rows; ties keep the lower index.
pub fn distance(sources: &[bool], w: usize, h: usize) -> Vec<Option<(f64, u32)>> {
    debug_assert_eq!(sources.len(), w * h);
    // Pass 1: along each column.
    let mut col: Vec<Option<(f64, u32)>> = vec![None; w * h];
    let mut f = vec![None; h];
    for x in 0..w {
        for y in 0..h {
            f[y] = if sources[y * w + x] {
                Some((0.0, (y * w + x) as u32))
            } else {
                None
            };
        }
        let d = lower_envelope(&f);
        for y in 0..h {
            col[y * w + x] = d[y];
        }
    }
    // Pass 2: along each row, over the column results.
    let mut out = vec![None; w * h];
    let mut g = vec![None; w];
    for y in 0..h {
        g.copy_from_slice(&col[y * w..(y + 1) * w]);
        let d = lower_envelope(&g);
        out[y * w..(y + 1) * w].copy_from_slice(&d);
    }
    out
}

/// The 1-D lower envelope of parabolas `(q - p)² + f(p)` over the finite entries of `f`.
fn lower_envelope(f: &[Option<(f64, u32)>]) -> Vec<Option<(f64, u32)>> {
    let n = f.len();
    let mut v: Vec<usize> = Vec::with_capacity(n);
    let mut z: Vec<f64> = Vec::with_capacity(n + 1);
    for q in 0..n {
        let Some((fq, _)) = f[q] else { continue };
        let qf = q as f64;
        while let Some(&p) = v.last() {
            let fp = f[p].map_or(0.0, |e| e.0);
            let pf = p as f64;
            let s = ((fq + qf * qf) - (fp + pf * pf)) / (2.0 * qf - 2.0 * pf);
            if s <= *z.last().unwrap_or(&f64::NEG_INFINITY) {
                v.pop();
                z.pop();
            } else {
                z.push(s);
                break;
            }
        }
        if v.is_empty() {
            z.clear();
            z.push(f64::NEG_INFINITY);
        }
        v.push(q);
    }
    let mut out = vec![None; n];
    if v.is_empty() {
        return out;
    }
    let mut k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        let qf = q as f64;
        while k + 1 < v.len() && z[k + 1] < qf {
            k += 1;
        }
        let p = v[k];
        let (fp, idx) = f[p].unwrap_or((0.0, 0));
        let dq = qf - p as f64;
        *slot = Some((dq * dq + fp, idx));
    }
    out
}

/// The least value within `r` nodes (a square window), the window clipped to the grid.
pub fn min_filter(v: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    let mut rows = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let (a, b) = (x.saturating_sub(r), (x + r).min(w - 1));
            let mut m = v[y * w + a];
            for xx in a + 1..=b {
                m = num::min(m, v[y * w + xx]);
            }
            rows[y * w + x] = m;
        }
    }
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        let (a, b) = (y.saturating_sub(r), (y + r).min(h - 1));
        for x in 0..w {
            let mut m = rows[a * w + x];
            for yy in a + 1..=b {
                m = num::min(m, rows[yy * w + x]);
            }
            out[y * w + x] = m;
        }
    }
    out
}

/// The mask grown by one node in all eight directions.
pub fn dilate(m: &[bool], w: usize, h: usize) -> Vec<bool> {
    let mut out = m.to_vec();
    for y in 0..h {
        for x in 0..w {
            if !m[y * w + x] {
                continue;
            }
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    out[yy * w + xx] = true;
                }
            }
        }
    }
    out
}

/// Bilinear sample of a `w` by `h` grid at fractional node coordinates, clamped to the grid.
pub fn bilinear(v: &[f64], w: usize, h: usize, u: f64, t: f64) -> f64 {
    let u = num::clamp(u, 0.0, (w - 1) as f64);
    let t = num::clamp(t, 0.0, (h - 1) as f64);
    let i = (u.floor() as usize).min(w.saturating_sub(2));
    let j = (t.floor() as usize).min(h.saturating_sub(2));
    let (fx, fy) = (u - i as f64, t - j as f64);
    let at = |a: usize, b: usize| v[b.min(h - 1) * w + a.min(w - 1)];
    let lo = at(i, j) + (at(i + 1, j) - at(i, j)) * fx;
    let hi = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * fx;
    lo + (hi - lo) * fy
}

/// Bilinear over the corners `keep` marks only, their weights renormalised: `None` when no corner
/// around the point is marked. A level that holds over some cells and not others (standing water)
/// is read without blending in the cells where it does not hold.
pub fn bilinear_where(v: &[f64], keep: &[bool], w: usize, h: usize, u: f64, t: f64) -> Option<f64> {
    let u = num::clamp(u, 0.0, (w - 1) as f64);
    let t = num::clamp(t, 0.0, (h - 1) as f64);
    let i = (u.floor() as usize).min(w.saturating_sub(2));
    let j = (t.floor() as usize).min(h.saturating_sub(2));
    let (fx, fy) = (u - i as f64, t - j as f64);
    let corners = [
        (i, j, (1.0 - fx) * (1.0 - fy)),
        (i + 1, j, fx * (1.0 - fy)),
        (i, j + 1, (1.0 - fx) * fy),
        (i + 1, j + 1, fx * fy),
    ];
    let (mut sum, mut weight) = (0.0, 0.0);
    for (a, b, wt) in corners {
        let k = b.min(h - 1) * w + a.min(w - 1);
        if keep[k] {
            sum += v[k] * wt;
            weight += wt;
        }
    }
    (weight > 0.0).then(|| sum / weight)
}

/// A box blur of `radius` cells, along rows then columns, each window clamped to the grid: the
/// mean of the cells it covers, summed in index order.
pub fn box_blur(v: &[f64], w: usize, h: usize, radius: usize) -> Vec<f64> {
    let pass = |src: &[f64], along_x: bool| -> Vec<f64> {
        let mut out = vec![0.0; src.len()];
        for b in 0..h {
            for a in 0..w {
                let (c, n) = if along_x { (a, w) } else { (b, h) };
                let lo = c.saturating_sub(radius);
                let hi = (c + radius).min(n - 1);
                let mut sum = 0.0;
                for q in lo..=hi {
                    sum += if along_x {
                        src[b * w + q]
                    } else {
                        src[q * w + a]
                    };
                }
                out[b * w + a] = sum / (hi - lo + 1) as f64;
            }
        }
        out
    };
    pass(&pass(v, true), false)
}

/// Catmull-Rom bicubic sample at fractional node coordinates, the neighbourhood clamped to the
/// grid. Through every node exactly; between them, smooth where bilinear shows the cells.
pub fn bicubic(v: &[f64], w: usize, h: usize, u: f64, t: f64) -> f64 {
    let u = num::clamp(u, 0.0, (w - 1) as f64);
    let t = num::clamp(t, 0.0, (h - 1) as f64);
    let i = u.floor() as i64;
    let j = t.floor() as i64;
    let (fx, fy) = (u - i as f64, t - j as f64);
    let at = |a: i64, b: i64| {
        let a = a.clamp(0, w as i64 - 1) as usize;
        let b = b.clamp(0, h as i64 - 1) as usize;
        v[b * w + a]
    };
    let mut rows = [0.0; 4];
    for (k, row) in rows.iter_mut().enumerate() {
        let b = j - 1 + k as i64;
        *row = catmull(at(i - 1, b), at(i, b), at(i + 1, b), at(i + 2, b), fx);
    }
    catmull(rows[0], rows[1], rows[2], rows[3], fy)
}

fn catmull(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    ((a * t + b) * t + c) * t + p1
}

/// The value at rank `floor(q * (n - 1))` of the sorted values (total order); `None` for none.
pub fn quantile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut s = values.to_vec();
    s.sort_by(f64::total_cmp);
    let k = (q * (s.len() - 1) as f64).floor() as usize;
    Some(s[k.min(s.len() - 1)])
}

/// Distance from a point to a polyline, metres.
pub fn to_polyline(p: (f64, f64), line: &[(f64, f64)]) -> f64 {
    let mut best = f64::INFINITY;
    if line.len() == 1 {
        return length(p, line[0]);
    }
    for s in line.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 {
            num::clamp(((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2, 0.0, 1.0)
        } else {
            0.0
        };
        best = num::min(best, length(p, (a.0 + t * dx, a.1 + t * dy)));
    }
    best
}

/// Euclidean length between two points, metres.
pub fn length(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    (dx * dx + dy * dy).sqrt()
}

/// Total length of a polyline.
pub fn polyline_length(line: &[(f64, f64)]) -> f64 {
    line.windows(2).map(|s| length(s[0], s[1])).sum()
}

/// Points every `step` metres along a polyline, its first and last points included, with the run
/// to each.
pub fn resample(line: &[(f64, f64)], step: f64) -> Vec<((f64, f64), f64)> {
    let total = polyline_length(line);
    if line.len() < 2 || total <= 0.0 {
        return line.iter().map(|&p| (p, 0.0)).collect();
    }
    let n = ((total / step).ceil() as usize).max(1);
    let mut out = Vec::with_capacity(n + 1);
    let mut seg = 0;
    let mut seg_start = 0.0;
    for k in 0..=n {
        let s = total * k as f64 / n as f64;
        while seg + 1 < line.len() - 1 && seg_start + length(line[seg], line[seg + 1]) < s {
            seg_start += length(line[seg], line[seg + 1]);
            seg += 1;
        }
        let (a, b) = (line[seg], line[seg + 1]);
        let l = length(a, b);
        let t = if l > 0.0 {
            num::clamp((s - seg_start) / l, 0.0, 1.0)
        } else {
            0.0
        };
        out.push(((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)), s));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_with_a_hole() {
        let outer = vec![(0, 0), (4000, 0), (4000, 4000), (0, 4000)];
        let hole = vec![(1000, 1000), (1000, 3000), (3000, 3000), (3000, 1000)];
        let r = vec![outer, hole];
        assert!(in_rings(&r, (500, 500)));
        assert!(!in_rings(&r, (2000, 2000)));
        assert!(!in_rings(&r, (5000, 500)));
    }

    /// The transform equals brute force on a small grid with scattered sources.
    fn sq(a: f64) -> f64 {
        a * a
    }

    #[test]
    fn distance_is_exact() {
        let (w, h) = (13, 9);
        let src: Vec<bool> = (0..w * h).map(|i| i % 17 == 3 || i == 50).collect();
        let d = distance(&src, w, h);
        for y in 0..h {
            for x in 0..w {
                let mut best = f64::INFINITY;
                for (k, &s) in src.iter().enumerate() {
                    if s {
                        let (sx, sy) = ((k % w) as f64, (k / w) as f64);
                        let dd = sq(sx - x as f64) + sq(sy - y as f64);
                        best = num::min(best, dd);
                    }
                }
                let (got, idx) = d[y * w + x].unwrap();
                assert_eq!(got, best, "({x}, {y})");
                let (sx, sy) = ((idx as usize % w) as f64, (idx as usize / w) as f64);
                assert_eq!(sq(sx - x as f64) + sq(sy - y as f64), best);
            }
        }
        assert!(distance(&[false; 6], 3, 2).iter().all(Option::is_none));
    }

    #[test]
    fn cubic_passes_through_nodes() {
        let v: Vec<f64> = (0..20).map(|i| (i * i % 7) as f64).collect();
        for j in 0..4 {
            for i in 0..5 {
                assert_eq!(bicubic(&v, 5, 4, i as f64, j as f64), v[j * 5 + i]);
                assert_eq!(bilinear(&v, 5, 4, i as f64, j as f64), v[j * 5 + i]);
            }
        }
    }

    #[test]
    fn resample_keeps_ends() {
        let line = [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)];
        let r = resample(&line, 2.0);
        assert_eq!(r.first().unwrap().0, (0.0, 0.0));
        assert_eq!(r.last().unwrap().0, (10.0, 5.0));
        assert_eq!(r.last().unwrap().1, 15.0);
    }
}
