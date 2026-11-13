//! Names by place for what the kernel makes (`spec/identity.md` 2): the digest of parts, and the
//! keys of junctions, edges, areas, cells, rest pieces and masked pieces. No key holds a counter:
//! an ordinal counts only among things that share every other part.

use crate::cells::{Cells, Front};
use crate::network::{Network, Side};
use crate::shape::P;
use std::collections::BTreeMap;

/// A role of a source at a junction (2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// The source starts here.
    Start,
    /// The source ends here.
    End,
    /// The source passes through.
    Pass,
}

/// A part of a digest (2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// A name: its UTF-8 bytes.
    Name(String),
    /// An integer: 8 bytes, little-endian two's complement.
    Int(i64),
    /// A side: one byte.
    Side(Side),
    /// A role: one byte.
    Role(Role),
    /// Two parts in a row.
    Pair(Box<Part>, Box<Part>),
    /// A count, then each member, sorted.
    Set(Vec<Part>),
}

impl Part {
    /// A name part.
    pub fn name(s: &str) -> Part {
        Part::Name(s.to_string())
    }

    /// A pair part.
    pub fn pair(a: Part, b: Part) -> Part {
        Part::Pair(Box::new(a), Box::new(b))
    }

    /// The byte strings this part absorbs, in order.
    pub fn encode(&self, out: &mut Vec<Vec<u8>>) {
        match self {
            Part::Name(s) => out.push(s.as_bytes().to_vec()),
            Part::Int(i) => out.push(i.to_le_bytes().to_vec()),
            Part::Side(s) => out.push(vec![match s {
                Side::Left => 0,
                Side::Right => 1,
            }]),
            Part::Role(r) => out.push(vec![match r {
                Role::Start => 0,
                Role::End => 1,
                Role::Pass => 2,
            }]),
            Part::Pair(a, b) => {
                a.encode(out);
                b.encode(out);
            }
            Part::Set(members) => {
                out.push((members.len() as i64).to_le_bytes().to_vec());
                let mut enc: Vec<Vec<Vec<u8>>> = members
                    .iter()
                    .map(|m| {
                        let mut v = Vec::new();
                        m.encode(&mut v);
                        v
                    })
                    .collect();
                enc.sort();
                for e in enc {
                    out.extend(e);
                }
            }
        }
    }
}

/// `digest(class, parts) = stream(0, ["name", class] ++ parts)` (2.2).
pub fn digest(class: &str, parts: &[Part]) -> u64 {
    let mut bytes: Vec<Vec<u8>> = vec![b"name".to_vec(), class.as_bytes().to_vec()];
    for p in parts {
        p.encode(&mut bytes);
    }
    via_num::stream(0, &bytes)
}

/// 16 lowercase hex digits.
pub fn hex(d: u64) -> String {
    format!("{d:016x}")
}

/// An integer as a key part: `n` for the minus sign.
pub fn int_part(i: i64) -> String {
    if i < 0 {
        format!("n{}", i.unsigned_abs())
    } else {
        i.to_string()
    }
}

/// Assigns ordinals among keys that share a digest, ordered by a sort key.
fn ordinals<K: PartialOrd>(base: &[String], order: &[K]) -> Vec<String> {
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, b) in base.iter().enumerate() {
        groups.entry(b.as_str()).or_default().push(i);
    }
    let mut out = vec![String::new(); base.len()];
    for (b, mut idx) in groups {
        idx.sort_by(|&x, &y| {
            order[x]
                .partial_cmp(&order[y])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(x.cmp(&y))
        });
        for (o, &i) in idx.iter().enumerate() {
            out[i] = format!("{b}_{o}");
        }
    }
    out
}

/// The keys of a network's junctions, edges and areas (2.3), in the network's orders.
#[derive(Clone, Debug)]
pub struct NetworkKeys {
    /// Junction keys.
    pub junctions: Vec<String>,
    /// Edge keys.
    pub edges: Vec<String>,
    /// Area keys.
    pub areas: Vec<String>,
}

/// Names a network's junctions, edges and areas.
pub fn network_keys(net: &Network) -> NetworkKeys {
    let srcs = &net.noded.sources;
    let lowest = |ids: &[usize]| -> Option<usize> {
        ids.iter()
            .copied()
            .min_by(|&a, &b| srcs[a].as_bytes().cmp(srcs[b].as_bytes()))
    };
    // Junctions.
    let mut jbase = Vec::new();
    let mut jorder: Vec<f64> = Vec::new();
    for &jp in &net.junctions {
        let v = net.arr.vid(jp).unwrap_or(0);
        // Incident entries per source, and the t of this vertex on each.
        let mut count: BTreeMap<usize, usize> = BTreeMap::new();
        let mut tmin: BTreeMap<usize, f64> = BTreeMap::new();
        for &h in &net.arr.out[v] {
            let seg = &net.noded.segments[h / 2];
            for p in &seg.prov {
                *count.entry(p.source).or_insert(0) += 1;
                let t = if seg.a == jp { p.t[0] } else { p.t[1] };
                let e = tmin.entry(p.source).or_insert(t);
                if t < *e {
                    *e = t;
                }
            }
        }
        let mut members = Vec::new();
        for (&s, &c) in &count {
            let inp = &net.noded.inputs[s];
            let mut c = c;
            if !inp.closed {
                if inp.pts.first() == Some(&jp) {
                    members.push(Part::pair(Part::name(&srcs[s]), Part::Role(Role::Start)));
                    c = c.saturating_sub(1);
                }
                if inp.pts.last() == Some(&jp) {
                    members.push(Part::pair(Part::name(&srcs[s]), Part::Role(Role::End)));
                    c = c.saturating_sub(1);
                }
            }
            for _ in 0..(c / 2) {
                members.push(Part::pair(Part::name(&srcs[s]), Part::Role(Role::Pass)));
            }
        }
        jbase.push(format!(
            "j{}",
            hex(digest("junction", &[Part::Set(members)]))
        ));
        let ids: Vec<usize> = count.keys().copied().collect();
        jorder.push(
            lowest(&ids)
                .and_then(|s| tmin.get(&s).copied())
                .unwrap_or(0.0),
        );
    }
    let junctions = ordinals(&jbase, &jorder);
    // Edges.
    let mut ebase = Vec::new();
    let mut eorder: Vec<f64> = Vec::new();
    for e in &net.edges {
        let mut ends = [junctions[e.ends.0].clone(), junctions[e.ends.1].clone()];
        ends.sort();
        let set: Vec<Part> = e.sources.iter().map(|&s| Part::name(&srcs[s])).collect();
        ebase.push(format!(
            "e{}",
            hex(digest(
                "edge",
                &[Part::name(&ends[0]), Part::name(&ends[1]), Part::Set(set)]
            ))
        ));
        let lo = lowest(&e.sources);
        let mut t = f64::INFINITY;
        for &h in &e.half {
            for p in &net.noded.segments[h / 2].prov {
                if Some(p.source) == lo {
                    for x in p.t {
                        if x < t {
                            t = x;
                        }
                    }
                }
            }
        }
        eorder.push(t);
    }
    let edges = ordinals(&ebase, &eorder);
    // Areas.
    let mut abase = Vec::new();
    let mut aorder: Vec<(i64, i64)> = Vec::new();
    for a in &net.areas {
        let set: Vec<Part> = a
            .sides
            .iter()
            .map(|&(s, side)| Part::pair(Part::name(&srcs[s]), Part::Side(side)))
            .collect();
        abase.push(format!("a{}", hex(digest("area", &[Part::Set(set)]))));
        aorder.push((a.point.x, a.point.y));
    }
    let areas = ordinals(&abase, &aorder);
    NetworkKeys {
        junctions,
        edges,
        areas,
    }
}

/// The key of each cell (2.3, `cell`), given the keys of the network the cells front.
pub fn cell_keys(net: &Network, keys: &NetworkKeys, cells: &Cells) -> Vec<String> {
    let srcs = &net.noded.sources;
    cells
        .cells
        .iter()
        .map(|c| {
            let (source, side, anchor) = match c.front {
                Front::Edge(fe) => {
                    let e = &net.edges[fe.edge];
                    // The junction where the edge begins along the source's travel.
                    let h0 = e.half[0];
                    let along = net.noded.segments[h0 / 2]
                        .prov
                        .iter()
                        .find(|p| p.source == fe.source)
                        .is_some_and(|p| h0.is_multiple_of(2) != p.reversed);
                    let start = if along {
                        e.path[0]
                    } else {
                        e.path[e.path.len() - 1]
                    };
                    let j = net
                        .junctions
                        .binary_search(&start)
                        .map(|i| keys.junctions[i].clone())
                        .unwrap_or_default();
                    (srcs[fe.source].clone(), fe.side, j)
                }
                Front::Outline => (String::new(), Side::Left, String::new()),
            };
            let d = digest(
                "cell",
                &[
                    Part::name(&source),
                    Part::Side(side),
                    Part::name(&anchor),
                    Part::Int(c.stretch as i64),
                ],
            );
            format!("c{}_{}", hex(d), c.station_in_stretch)
        })
        .collect()
}

/// The keys of rest pieces of a divided region named `divided` (2.3, `rest`).
pub fn rest_keys(divided: &str, points: &[P]) -> Vec<String> {
    let base = format!("b{}", hex(digest("rest", &[Part::name(divided)])));
    let bases = vec![base; points.len()];
    let order: Vec<(i64, i64)> = points.iter().map(|p| (p.x, p.y)).collect();
    ordinals(&bases, &order)
}

/// The side of the coarse square of masked pieces, millimetres (2.3; open, decided by M1's
/// masked-region fixtures).
pub const PIECE_SQUARE_MM: i64 = 32_000;

/// The keys of the pieces of a `where` region by their points (2.3, `piece`).
pub fn piece_keys(points: &[P]) -> Vec<String> {
    let bases: Vec<String> = points
        .iter()
        .map(|p| {
            format!(
                "p{}_{}",
                int_part(p.x.div_euclid(PIECE_SQUARE_MM)),
                int_part(p.y.div_euclid(PIECE_SQUARE_MM))
            )
        })
        .collect();
    let order: Vec<(i64, i64)> = points.iter().map(|p| (p.x, p.y)).collect();
    ordinals(&bases, &order)
}
