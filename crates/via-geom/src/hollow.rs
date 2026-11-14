//! One hollow: the piece of a masked region that holds a point (`derived.md` 2, `holds`).
//!
//! The mask itself is [`crate::mask`] (`geometry.md` 3.7). `holds` keeps the one piece that holds
//! the written point, by the exact closed test (inside or on its outline, its holes outside), with
//! its holes; the other pieces are not made. A piece that touches the extent's outline runs out of
//! the world at that level, which is said (`derive.open`); no piece holding the point is said too
//! (`derive.dry`). Both are warnings: the region builds, and what needed it reports its own error.

use crate::diag::{Code, GeomDiag};
use crate::pred::Side;
use crate::shape::{Piece, Region, P};

/// The first piece, in the region's canonical order, that holds `at`: inside its shell or on any
/// of its rings, and in none of its holes. Two pieces that meet at one vertex both hold that
/// vertex; the first is kept.
pub fn piece_holding(region: &Region, at: P) -> Option<&Piece> {
    region.pieces.iter().find(|pc| {
        let one = Region {
            pieces: vec![(*pc).clone()],
        };
        one.side(at) != Side::Out
    })
}

/// The first vertex of a piece on the outline of `extent` (`[x0, y0, x1, y1]`), in post order: least
/// `y`, then least `x` (`numbers.md` 4). Every ring is searched.
pub fn on_outline(piece: &Piece, extent: [i64; 4]) -> Option<P> {
    let [x0, y0, x1, y1] = extent;
    piece
        .rings()
        .into_iter()
        .flatten()
        .copied()
        .filter(|p| p.x == x0 || p.x == x1 || p.y == y0 || p.y == y1)
        .min_by_key(|p| (p.y, p.x))
}

/// `holds` over a masked region (`derived.md` 2): the one piece holding `at`, with what is said
/// about it. `value` is the condition's value at the point (`fields.md` 1.2), for `derive.dry`.
pub fn holds(region: &Region, at: P, extent: [i64; 4], value: f64) -> (Region, Vec<GeomDiag>) {
    let Some(pc) = piece_holding(region, at) else {
        // The registry declares no `data` for `derive.dry`, so the value is said in the message.
        let v = via_num::write_double(value).unwrap_or_else(|_| "not finite".to_string());
        let d = GeomDiag::new(
            Code::DeriveDry,
            format!(
                "no piece of the `where` region holds the `holds` point, so the region is empty; the condition is {v} there"
            ),
        )
        .at(at);
        return (Region::default(), vec![d]);
    };
    let mut diags = Vec::new();
    if let Some(p) = on_outline(pc, extent) {
        diags.push(
            GeomDiag::new(
                Code::DeriveOpen,
                "the hollow `holds` keeps touches the extent's outline: at this level it runs out of the world",
            )
            .at(p),
        );
    }
    (
        Region {
            pieces: vec![pc.clone()],
        },
        diags,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::{mask, MaskGrid};

    const EXTENT: [i64; 4] = [0, 0, 8_000, 8_000];

    fn grid() -> MaskGrid {
        MaskGrid {
            x0: 0,
            y0: 0,
            r: 1_000,
            nx: 9,
            ny: 9,
        }
    }

    /// `twin` of `derived.md`'s cases, and `level - twin`.
    fn twin(level: f64) -> Vec<f64> {
        let mut v = Vec::new();
        for j in 0..9i64 {
            for i in 0..9i64 {
                let h = ((i - 2).abs() + (j - 4).abs()).min((i - 6).abs() + (j - 4).abs());
                v.push(level - h as f64);
            }
        }
        v
    }

    fn twin_value(x: i64, y: i64, level: f64) -> f64 {
        // On a post, the condition's own value.
        twin(level)[(y / 1000 * 9 + x / 1000) as usize]
    }

    #[test]
    fn d01_one_hollow_of_two() {
        let (all, _) = mask(grid(), &twin(1.5), "= 1.5 - f('height')", None).unwrap();
        assert_eq!(all.pieces.len(), 2);
        let (one, d) = holds(&all, P::new(2_000, 4_000), EXTENT, 1.5);
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(one.pieces.len(), 1);
        assert_ne!(one.side(P::new(2_000, 4_000)), Side::Out);
        assert_eq!(one.side(P::new(6_000, 4_000)), Side::Out);
        // The same rings as that piece of the whole mask.
        assert!(all.pieces.contains(&one.pieces[0]));
    }

    #[test]
    fn d02_a_point_in_no_piece_is_dry() {
        let (all, _) = mask(grid(), &twin(1.5), "", None).unwrap();
        let v = twin_value(4_000, 0, 1.5);
        let (one, d) = holds(&all, P::new(4_000, 0), EXTENT, v);
        assert!(one.is_empty());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::DeriveDry);
        assert_eq!(d[0].position, Some(P::new(4_000, 0)));
        assert!(
            d[0].message.ends_with("the condition is -4.5 there"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn d03_a_hollow_over_the_rim_is_open() {
        let (all, _) = mask(grid(), &twin(3.5), "", None).unwrap();
        let (one, d) = holds(&all, P::new(2_000, 4_000), EXTENT, 3.5);
        assert_eq!(one.pieces.len(), 1);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::DeriveOpen);
        // twin < 3.5 reaches x = 0 between y = 2.5 and 5.5; the first vertex in post order.
        assert_eq!(d[0].position, Some(P::new(0, 2_500)));
        assert_eq!(on_outline(&one.pieces[0], EXTENT), Some(P::new(0, 2_500)));
    }

    #[test]
    fn d05_moving_the_point_takes_the_other_hollow() {
        let (all, _) = mask(grid(), &twin(1.5), "", None).unwrap();
        let (a, _) = holds(&all, P::new(2_000, 4_000), EXTENT, 1.5);
        let (b, _) = holds(&all, P::new(6_000, 4_000), EXTENT, 1.5);
        assert_ne!(a, b);
        assert_ne!(b.side(P::new(6_000, 4_000)), Side::Out);
    }
}
