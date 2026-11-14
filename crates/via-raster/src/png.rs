//! PNG field files (`fields.md` 4, M2): 8- and 16-bit grey, grey + alpha, RGB and RGBA, Adam7
//! interlaced or not, read to stored integer levels. Palette pictures and depths below 8 are
//! refused. Ours, from the PNG specification (W3C, 2003) and DEFLATE (RFC 1951).
//!
//! Integer work only: the levels are what the file stores; mapping them to values by `range` is
//! the caller's (`fields.md` 4.2).

/// A decoded picture: `levels` row by row from the top (north), `channels` per pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct Png {
    pub width: u64,
    pub height: u64,
    /// 1 grey, 2 grey + alpha, 3 RGB, 4 RGBA.
    pub channels: usize,
    /// 8 or 16.
    pub depth: u8,
    pub levels: Vec<u16>,
    /// Chunks whose meaning was not applied (`gAMA`, `cHRM`, `sRGB`, `iCCP`, `tRNS`), by name.
    pub unapplied: Vec<String>,
}

/// Why a PNG was not read.
#[derive(Clone, Debug, PartialEq)]
pub enum PngError {
    /// Not a PNG of the subset: palette, depth below 8, or malformed.
    Format(String),
}

impl std::fmt::Display for PngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PngError::Format(s) => write!(f, "{s}"),
        }
    }
}

fn bad(s: impl Into<String>) -> PngError {
    PngError::Format(s.into())
}

/// Whether the bytes start with the PNG signature.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// Reads a PNG, at most `limit` pixels.
pub fn read(bytes: &[u8], limit: u64) -> Result<Png, PngError> {
    if !is_png(bytes) {
        return Err(bad("not a PNG"));
    }
    let mut at = 8usize;
    let mut header: Option<(u64, u64, u8, u8, u8)> = None;
    let mut data: Vec<u8> = Vec::new();
    let mut unapplied: Vec<String> = Vec::new();
    loop {
        if at + 8 > bytes.len() {
            return Err(bad("the file ends inside a chunk"));
        }
        let len = be32(&bytes[at..]) as usize;
        let kind = &bytes[at + 4..at + 8];
        let body_end = at + 8 + len;
        if body_end + 4 > bytes.len() {
            return Err(bad("the file ends inside a chunk"));
        }
        let body = &bytes[at + 8..body_end];
        match kind {
            b"IHDR" => {
                if len != 13 {
                    return Err(bad("IHDR is not 13 bytes"));
                }
                header = Some((
                    u64::from(be32(body)),
                    u64::from(be32(&body[4..])),
                    body[8],
                    body[9],
                    body[12],
                ));
                if body[10] != 0 || body[11] != 0 {
                    return Err(bad("an unknown compression or filter method"));
                }
            }
            b"IDAT" => data.extend_from_slice(body),
            b"IEND" => break,
            b"PLTE" => {}
            b"gAMA" | b"cHRM" | b"sRGB" | b"iCCP" | b"tRNS" => {
                let n = String::from_utf8_lossy(kind).into_owned();
                if !unapplied.contains(&n) {
                    unapplied.push(n);
                }
            }
            _ => {}
        }
        at = body_end + 4;
    }
    let (width, height, depth, colour, interlace) = header.ok_or_else(|| bad("no IHDR"))?;
    let channels = match colour {
        0 => 1,
        4 => 2,
        2 => 3,
        6 => 4,
        3 => return Err(bad("a palette PNG")),
        c => return Err(bad(format!("colour type {c}"))),
    };
    if depth != 8 && depth != 16 {
        return Err(bad(format!("a {depth}-bit PNG")));
    }
    if width == 0 || height == 0 || width.saturating_mul(height) > limit {
        return Err(bad("an empty or too large picture"));
    }
    let raw = zlib(&data)?;
    let bpp = channels * usize::from(depth / 8);
    let (w, h) = (width as usize, height as usize);
    let mut levels = vec![0u16; w * h * channels];
    let mut put = |x: usize, y: usize, px: &[u8]| {
        for c in 0..channels {
            let v = if depth == 8 {
                u16::from(px[c])
            } else {
                u16::from_be_bytes([px[2 * c], px[2 * c + 1]])
            };
            levels[(y * w + x) * channels + c] = v;
        }
    };
    let mut pos = 0usize;
    let passes: &[(usize, usize, usize, usize)] = if interlace == 1 {
        &[
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
    } else {
        &[(0, 0, 1, 1)]
    };
    for &(x0, y0, dx, dy) in passes {
        if x0 >= w || y0 >= h {
            continue;
        }
        let pw = (w - x0).div_ceil(dx);
        let ph = (h - y0).div_ceil(dy);
        let stride = pw * bpp;
        let mut prev = vec![0u8; stride];
        for r in 0..ph {
            if pos + 1 + stride > raw.len() {
                return Err(bad("the picture's data is short"));
            }
            let f = raw[pos];
            let mut line = raw[pos + 1..pos + 1 + stride].to_vec();
            pos += 1 + stride;
            unfilter(f, &mut line, &prev, bpp)?;
            for c in 0..pw {
                put(x0 + c * dx, y0 + r * dy, &line[c * bpp..(c + 1) * bpp]);
            }
            prev = line;
        }
    }
    Ok(Png {
        width,
        height,
        channels,
        depth,
        levels,
        unapplied,
    })
}

fn unfilter(f: u8, line: &mut [u8], prev: &[u8], bpp: usize) -> Result<(), PngError> {
    for i in 0..line.len() {
        let a = if i >= bpp {
            i32::from(line[i - bpp])
        } else {
            0
        };
        let b = i32::from(prev[i]);
        let c = if i >= bpp {
            i32::from(prev[i - bpp])
        } else {
            0
        };
        let add = match f {
            0 => 0,
            1 => a,
            2 => b,
            3 => (a + b) / 2,
            4 => {
                let p = a + b - c;
                let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                if pa <= pb && pa <= pc {
                    a
                } else if pb <= pc {
                    b
                } else {
                    c
                }
            }
            _ => return Err(bad(format!("filter type {f}"))),
        };
        line[i] = (i32::from(line[i]) + add) as u8;
    }
    Ok(())
}

/// A zlib stream (RFC 1950) inflated; the Adler-32 is not checked (the PNG's CRCs are not either).
fn zlib(d: &[u8]) -> Result<Vec<u8>, PngError> {
    if d.len() < 2 || d[0] & 0x0f != 8 || (u16::from(d[0]) << 8 | u16::from(d[1])) % 31 != 0 {
        return Err(bad("the picture's data is not zlib"));
    }
    inflate(&d[2..])
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    bit: u32,
}

impl Bits<'_> {
    fn get(&mut self, n: u32) -> Result<u32, PngError> {
        let mut v = 0u32;
        for i in 0..n {
            let byte = *self
                .d
                .get(self.pos)
                .ok_or_else(|| bad("the data ends early"))?;
            v |= u32::from((byte >> self.bit) & 1) << i;
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.pos += 1;
            }
        }
        Ok(v)
    }
}

/// A canonical Huffman code: counts per length and symbols in code order.
struct Huff {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huff {
    fn new(lengths: &[u8]) -> Huff {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[usize::from(l)] += 1;
        }
        counts[0] = 0;
        let mut offs = [0u16; 16];
        for i in 1..16 {
            offs[i] = offs[i - 1] + counts[i - 1];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[usize::from(offs[usize::from(l)])] = s as u16;
                offs[usize::from(l)] += 1;
            }
        }
        Huff { counts, symbols }
    }

    fn decode(&self, b: &mut Bits<'_>) -> Result<u16, PngError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= b.get(1)? as i32;
            let count = i32::from(self.counts[len]);
            if code - count < first {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(bad("a bad Huffman code"))
    }
}

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEXT: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// DEFLATE (RFC 1951).
pub fn inflate(d: &[u8]) -> Result<Vec<u8>, PngError> {
    let mut b = Bits { d, pos: 0, bit: 0 };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = b.get(1)?;
        match b.get(2)? {
            0 => {
                if b.bit != 0 {
                    b.bit = 0;
                    b.pos += 1;
                }
                if b.pos + 4 > d.len() {
                    return Err(bad("a stored block ends early"));
                }
                let len = usize::from(u16::from_le_bytes([d[b.pos], d[b.pos + 1]]));
                b.pos += 4;
                if b.pos + len > d.len() {
                    return Err(bad("a stored block ends early"));
                }
                out.extend_from_slice(&d[b.pos..b.pos + len]);
                b.pos += len;
            }
            1 => {
                let mut l = [0u8; 288];
                for (i, v) in l.iter_mut().enumerate() {
                    *v = match i {
                        0..=143 => 8,
                        144..=255 => 9,
                        256..=279 => 7,
                        _ => 8,
                    };
                }
                block(&mut b, &mut out, &Huff::new(&l), &Huff::new(&[5u8; 30]))?;
            }
            2 => {
                let hlit = b.get(5)? as usize + 257;
                let hdist = b.get(5)? as usize + 1;
                let hclen = b.get(4)? as usize + 4;
                const ORDER: [usize; 19] = [
                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                ];
                let mut cl = [0u8; 19];
                for &o in ORDER.iter().take(hclen) {
                    cl[o] = b.get(3)? as u8;
                }
                let ch = Huff::new(&cl);
                let mut lens: Vec<u8> = Vec::with_capacity(hlit + hdist);
                while lens.len() < hlit + hdist {
                    let s = ch.decode(&mut b)?;
                    match s {
                        0..=15 => lens.push(s as u8),
                        16 => {
                            let p = *lens
                                .last()
                                .ok_or_else(|| bad("a repeat with nothing before"))?;
                            for _ in 0..3 + b.get(2)? {
                                lens.push(p);
                            }
                        }
                        17 => {
                            let n = 3 + b.get(3)? as usize;
                            lens.resize(lens.len() + n, 0);
                        }
                        _ => {
                            let n = 11 + b.get(7)? as usize;
                            lens.resize(lens.len() + n, 0);
                        }
                    }
                }
                if lens.len() != hlit + hdist {
                    return Err(bad("code lengths overrun"));
                }
                block(
                    &mut b,
                    &mut out,
                    &Huff::new(&lens[..hlit]),
                    &Huff::new(&lens[hlit..]),
                )?;
            }
            _ => return Err(bad("a reserved block type")),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

fn block(b: &mut Bits<'_>, out: &mut Vec<u8>, lit: &Huff, dist: &Huff) -> Result<(), PngError> {
    loop {
        let s = lit.decode(b)?;
        if s < 256 {
            out.push(s as u8);
        } else if s == 256 {
            return Ok(());
        } else {
            let k = usize::from(s - 257);
            if k >= 29 {
                return Err(bad("a bad length code"));
            }
            let len = usize::from(LBASE[k]) + b.get(u32::from(LEXT[k]))? as usize;
            let dk = usize::from(dist.decode(b)?);
            if dk >= 30 {
                return Err(bad("a bad distance code"));
            }
            let dd = usize::from(DBASE[dk]) + b.get(u32::from(DEXT[dk]))? as usize;
            if dd > out.len() {
                return Err(bad("a distance before the start"));
            }
            let start = out.len() - dd;
            for i in 0..len {
                let v = out[start + i];
                out.push(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored-block zlib PNG of `w` by `h` 8-bit grey levels, filter 0, made here.
    fn grey8(w: u32, h: u32, px: &[u8]) -> Vec<u8> {
        let mut raw = Vec::new();
        for r in 0..h as usize {
            raw.push(0);
            raw.extend_from_slice(&px[r * w as usize..(r + 1) * w as usize]);
        }
        let mut z = vec![0x78, 0x01, 1];
        z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        z.extend_from_slice(&raw);
        z.extend_from_slice(&[0, 0, 0, 0]);
        let mut f = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut chunk = |k: &[u8], body: &[u8]| {
            f.extend_from_slice(&(body.len() as u32).to_be_bytes());
            f.extend_from_slice(k);
            f.extend_from_slice(body);
            f.extend_from_slice(&[0, 0, 0, 0]);
        };
        let mut ih = Vec::new();
        ih.extend_from_slice(&w.to_be_bytes());
        ih.extend_from_slice(&h.to_be_bytes());
        ih.extend_from_slice(&[8, 0, 0, 0, 0]);
        chunk(b"IHDR", &ih);
        chunk(b"gAMA", &[0, 0, 0, 1]);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        f
    }

    #[test]
    fn reads_stored_grey() {
        let p = read(&grey8(2, 2, &[255, 0, 0, 7]), 100).unwrap();
        assert_eq!((p.width, p.height, p.channels, p.depth), (2, 2, 1, 8));
        assert_eq!(p.levels, vec![255, 0, 0, 7]);
        assert_eq!(p.unapplied, vec!["gAMA".to_string()]);
    }

    #[test]
    fn inflates_fixed_huffman() {
        // "abcabcabc" compressed with fixed codes (zlib.compress at level 9, header removed).
        let d = [0x4b, 0x4c, 0x4a, 0x4e, 0x04, 0x23, 0x00];
        assert_eq!(inflate(&d).unwrap(), b"abcabcabc".to_vec());
    }
}
