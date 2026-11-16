//! A generated mesh as a glTF 2.0 binary file (`build.md` 10): one node translated to the mesh's
//! south-west corner, one mesh, one primitive of indexed triangles with positions and normals, and
//! at most one material. Written by our own writer with buffers in a fixed order, so the bytes are
//! the same everywhere: glTF axes `(X, Y, Z) = (x, z, -y)`, metres.

use via_model::surfaces::MeshOut;

/// A material's name and colour, red, green, blue in `[0, 1]`.
pub struct Material<'a> {
    pub name: &'a str,
    pub colour: [f64; 3],
}

fn metres(mm: i64) -> f64 {
    via_num::metres(mm)
}

/// A JSON string with the escapes JSON needs; names here are ids and place names.
fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A number as JSON: an f32 written by its shortest round-trip text.
fn num32(v: f32) -> String {
    let t = format!("{v:?}");
    t.strip_suffix(".0").map_or(t.clone(), str::to_string)
}

/// The `.glb` bytes of one mesh.
pub fn write(m: &MeshOut, material: Option<&Material<'_>>) -> Vec<u8> {
    let (ox, oy) = m.origin;
    // Positions relative to the node, glTF axes.
    let pos: Vec<[f32; 3]> = m
        .vertices
        .iter()
        .map(|v| {
            [
                metres(v[0] - ox) as f32,
                metres(v[2]) as f32,
                -(metres(v[1] - oy) as f32),
            ]
        })
        .collect();
    // Normals: the sum of each touching triangle's cross product (area-weighted), in f64 from
    // the lattice values, then unit length.
    let mut acc = vec![[0f64; 3]; m.vertices.len()];
    for t in &m.triangles {
        let p = |k: usize| {
            let v = m.vertices[t[k] as usize];
            [metres(v[0]), metres(v[2]), -metres(v[1])]
        };
        let (a, b, c) = (p(0), p(1), p(2));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        for k in 0..3 {
            let s = &mut acc[t[k] as usize];
            s[0] += n[0];
            s[1] += n[1];
            s[2] += n[2];
        }
    }
    let nrm: Vec<[f32; 3]> = acc
        .iter()
        .map(|n| {
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if l > 0.0 {
                [(n[0] / l) as f32, (n[1] / l) as f32, (n[2] / l) as f32]
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect();
    // Seen from above, a counter-clockwise plan triangle is counter-clockwise in glTF with +Y up
    // after `(x, z, -y)`: the turn from +x to -Z is the turn from +x to +y.
    let mut bin: Vec<u8> = Vec::new();
    for v in &pos {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let pos_len = bin.len();
    for v in &nrm {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let nrm_len = bin.len() - pos_len;
    for t in &m.triangles {
        for k in t {
            bin.extend_from_slice(&k.to_le_bytes());
        }
    }
    let idx_len = bin.len() - pos_len - nrm_len;
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for v in &pos {
        for k in 0..3 {
            lo[k] = via_num::min(f64::from(lo[k]), f64::from(v[k])) as f32;
            hi[k] = via_num::max(f64::from(hi[k]), f64::from(v[k])) as f32;
        }
    }
    let list = |v: &[f32; 3]| format!("[{},{},{}]", num32(v[0]), num32(v[1]), num32(v[2]));
    let (mat_ref, materials) = match material {
        Some(mt) => (
            ",\"material\":0".to_string(),
            format!(
                ",\"materials\":[{{\"name\":{},\"pbrMetallicRoughness\":{{\"baseColorFactor\":[{},{},{},1]}}}}]",
                quote(mt.name),
                num32(mt.colour[0] as f32),
                num32(mt.colour[1] as f32),
                num32(mt.colour[2] as f32)
            ),
        ),
        None => (String::new(), String::new()),
    };
    let json = format!(
        concat!(
            "{{\"accessors\":[",
            "{{\"bufferView\":0,\"componentType\":5126,\"count\":{n},\"max\":{hi},\"min\":{lo},\"type\":\"VEC3\"}},",
            "{{\"bufferView\":1,\"componentType\":5126,\"count\":{n},\"type\":\"VEC3\"}},",
            "{{\"bufferView\":2,\"componentType\":5125,\"count\":{ni},\"type\":\"SCALAR\"}}],",
            "\"asset\":{{\"generator\":\"via\",\"version\":\"2.0\"}},",
            "\"bufferViews\":[",
            "{{\"buffer\":0,\"byteLength\":{pl},\"byteOffset\":0,\"target\":34962}},",
            "{{\"buffer\":0,\"byteLength\":{nl},\"byteOffset\":{pl},\"target\":34962}},",
            "{{\"buffer\":0,\"byteLength\":{il},\"byteOffset\":{io},\"target\":34963}}],",
            "\"buffers\":[{{\"byteLength\":{bl}}}]",
            "{materials},",
            "\"meshes\":[{{\"extras\":{{\"via\":{{\"feature\":{f},\"surface\":{s}}}}},\"name\":{name},",
            "\"primitives\":[{{\"attributes\":{{\"NORMAL\":1,\"POSITION\":0}},\"indices\":2{mat},\"mode\":4}}]}}],",
            "\"nodes\":[{{\"mesh\":0,\"name\":{name},\"translation\":[{tx},0,{tz}]}}],",
            "\"scene\":0,\"scenes\":[{{\"nodes\":[0]}}]}}"
        ),
        n = pos.len(),
        hi = list(&hi),
        lo = list(&lo),
        ni = m.triangles.len() * 3,
        pl = pos_len,
        nl = nrm_len,
        il = idx_len,
        io = pos_len + nrm_len,
        bl = bin.len(),
        materials = materials,
        f = quote(&m.owner),
        s = quote(&m.surface),
        name = quote(&m.place),
        mat = mat_ref,
        tx = via_num::write_length(ox),
        tz = via_num::write_length(-oy),
    );
    let mut jb = json.into_bytes();
    while !jb.len().is_multiple_of(4) {
        jb.push(b' ');
    }
    let total = 12 + 8 + jb.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(jb.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&jb);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> MeshOut {
        MeshOut {
            place: "hall.floor".into(),
            owner: "hall".into(),
            surface: "floor".into(),
            material: None,
            origin: (1000, 2000),
            vertices: vec![
                [1000, 2000, 500],
                [2000, 2000, 500],
                [2000, 3000, 500],
                [1000, 3000, 500],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }
    }

    #[test]
    fn header_lengths_and_up_normals() {
        let b = write(&square(), None);
        assert_eq!(&b[0..4], b"glTF");
        assert_eq!(
            u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize,
            b.len()
        );
        let jl = u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize;
        assert!(jl.is_multiple_of(4));
        let json = std::str::from_utf8(&b[20..20 + jl]).unwrap();
        assert!(json.contains("\"translation\":[1,0,-2]"), "{json}");
        assert!(!json.contains("materials"));
        // The first normal is +Y: a flat floor faces up.
        let bin = &b[20 + jl + 8..];
        let ny = f32::from_le_bytes([bin[48 + 4], bin[48 + 5], bin[48 + 6], bin[48 + 7]]);
        assert_eq!(ny, 1.0);
    }

    #[test]
    fn same_bytes_twice_and_material_named() {
        let mt = Material {
            name: "oak",
            colour: [0.5, 0.25, 0.0],
        };
        let a = write(&square(), Some(&mt));
        assert_eq!(a, write(&square(), Some(&mt)));
        let s = String::from_utf8_lossy(&a);
        assert!(s.contains("\"name\":\"oak\""));
        assert!(s.contains("\"material\":0"));
    }
}
