//! Triangulation for export (`spec/geometry.md` 3.10): ear clipping on exact predicates, holes
//! joined to the shell by links first. No point is added. Ours; `i_triangle` and `earcutr`
//! were not judged (no network access to fetch them; section 6 leaves the choice to the
//! milestone that first exports a region as a mesh).

use crate::diag::GeomDiag;
use crate::overlay::internal;
use crate::pred::{orient, Meet};
use crate::shape::{Piece, Region, P};

/// A triangle, counter-clockwise, starting at its least vertex.
pub type Tri = [P; 3];

fn in_tri(p: P, a: P, b: P, c: P) -> bool {
    orient(a, b, p) >= 0 && orient(b, c, p) >= 0 && orient(c, a, p) >= 0
}

fn crosses_any(a: P, b: P, rings: &[&[P]]) -> bool {
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (c, d) = (r[i], r[(i + 1) % n]);
            if c == a || c == b || d == a || d == b {
                continue;
            }
            if crate::pred::meet(a, b, c, d) != Meet::None {
                return true;
            }
        }
    }
    false
}

/// Joins a piece's holes into its shell by links, giving one weakly simple polygon.
fn link_holes(piece: &Piece) -> Vec<P> {
    let mut poly = piece.shell.clone();
    let mut holes: Vec<Vec<P>> = piece.holes.clone();
    holes.sort_by_key(|h| std::cmp::Reverse(h.iter().copied().max().unwrap_or(P::new(0, 0))));
    let mut placed: Vec<Vec<P>> = Vec::new();
    for h in holes {
        let (mi, &m) = h.iter().enumerate().max().unwrap_or((0, &h[0]));
        // Candidates in order of distance.
        let mut cands: Vec<(i128, usize)> = poly
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (dx, dy) = (i128::from(p.x - m.x), i128::from(p.y - m.y));
                (dx * dx + dy * dy, i)
            })
            .collect();
        cands.sort();
        let mut rings: Vec<&[P]> = vec![&poly];
        rings.push(&h);
        for p in &placed {
            rings.push(p);
        }
        let mut chosen = None;
        for &(_, i) in &cands {
            let p = poly[i];
            if crosses_any(m, p, &rings) {
                continue;
            }
            // The bridge must leave p into the polygon's inside.
            let n = poly.len();
            let (prev, next) = (poly[(i + n - 1) % n], poly[(i + 1) % n]);
            let convex = orient(prev, p, next) >= 0;
            let inside = if convex {
                orient(prev, p, m) > 0 && orient(p, next, m) > 0
            } else {
                !(orient(prev, p, m) <= 0 && orient(p, next, m) <= 0)
            };
            if inside {
                chosen = Some(i);
                break;
            }
        }
        let Some(i) = chosen else { continue };
        let mut hole_seq: Vec<P> = h[mi..].to_vec();
        hole_seq.extend_from_slice(&h[..=mi]);
        let mut np: Vec<P> = poly[..=i].to_vec();
        np.extend(hole_seq);
        np.push(poly[i]);
        np.extend_from_slice(&poly[i + 1..]);
        poly = np;
        placed.push(h);
    }
    poly
}

fn clip(poly: Vec<P>) -> Vec<Tri> {
    let mut v = poly;
    let mut out = Vec::new();
    let mut guard = 0usize;
    while v.len() > 3 && guard < 100_000_000 {
        let n = v.len();
        let mut cut = None;
        for i in 0..n {
            guard += 1;
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let o = orient(a, b, c);
            if o < 0 {
                continue;
            }
            if o == 0 {
                // A straight vertex (or a bridge's return): drop it, no triangle.
                if a == c || crate::pred::dot(crate::pred::dir(a, b), crate::pred::dir(b, c)) > 0 {
                    cut = Some((i, false));
                    break;
                }
                continue;
            }
            let blocked = v
                .iter()
                .any(|&p| p != a && p != b && p != c && in_tri(p, a, b, c));
            if !blocked {
                cut = Some((i, true));
                break;
            }
        }
        match cut {
            Some((i, emit)) => {
                let n = v.len();
                if emit {
                    out.push([v[(i + n - 1) % n], v[i], v[(i + 1) % n]]);
                }
                v.remove(i);
            }
            None => break,
        }
    }
    if v.len() == 3 && orient(v[0], v[1], v[2]) > 0 {
        out.push([v[0], v[1], v[2]]);
    }
    out
}

fn canon(t: Tri) -> Tri {
    let i = (0..3).min_by_key(|&k| t[k]).unwrap_or(0);
    [t[i], t[(i + 1) % 3], t[(i + 2) % 3]]
}

/// `triangulate` (3.10).
pub fn triangulate(region: &Region) -> Result<Vec<Tri>, GeomDiag> {
    let mut tris: Vec<Tri> = Vec::new();
    for piece in &region.pieces {
        tris.extend(clip(link_holes(piece)).into_iter().map(canon));
    }
    tris.sort();
    let sum: i128 = tris.iter().map(|t| orient(t[0], t[1], t[2])).sum();
    if sum != region.twice_area() {
        return Err(internal(
            "triangulate",
            &format!("cover: triangles {sum} != region {}", region.twice_area()),
        ));
    }
    Ok(tris)
}
