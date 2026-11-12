//! Field files: the `.npy` 1.0 subset of `spec/protocol.md` 4.2 and `spec/build.md` 3.2.
//!
//! One format for a field window, a returned field and an author's `file`, so a step's output and
//! a build's field are the same bytes, and NumPy reads either with `numpy.load(mmap_mode="r")`
//! and no library of ours.

/// The element types the subset allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Descr {
    /// `<f4`: scalar fields and weights.
    F4,
    /// `|u1`: covers.
    U1,
}

impl Descr {
    fn text(self) -> &'static str {
        match self {
            Descr::F4 => "<f4",
            Descr::U1 => "|u1",
        }
    }
    pub fn size(self) -> usize {
        match self {
            Descr::F4 => 4,
            Descr::U1 => 1,
        }
    }
}

const MAGIC: &[u8] = b"\x93NUMPY\x01\x00";
/// Magic, version and the u16 header length.
const PREAMBLE: usize = 10;

/// The header for an array of `descr` and `shape`, padded with spaces to the smallest multiple of
/// 64 bytes and ended by LF, as `build.md` 3.2 fixes. The text is NumPy's own dict repr, so NumPy
/// parses it; NumPy may pad further when it writes, which is why readers accept any multiple.
pub fn header(descr: Descr, shape: &[usize]) -> Vec<u8> {
    let dims: Vec<String> = shape.iter().map(usize::to_string).collect();
    let tuple = if shape.len() == 1 {
        format!("({},)", dims[0])
    } else {
        format!("({})", dims.join(", "))
    };
    let text = format!(
        "{{'descr': '{}', 'fortran_order': False, 'shape': {}, }}",
        descr.text(),
        tuple
    );
    let unpadded = PREAMBLE + text.len() + 1;
    let pad = (64 - unpadded % 64) % 64;
    let len = (text.len() + pad + 1) as u16;
    let mut out = Vec::with_capacity(unpadded + pad);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.resize(out.len() + pad, b' ');
    out.push(b'\n');
    out
}

/// A parsed header: what the data is and where it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub descr: Descr,
    pub shape: Vec<usize>,
    /// Offset of the first data byte: a multiple of 64.
    pub data_offset: usize,
}

/// Why a file is outside the subset; the reason goes in the log and `data`, the code is
/// `protocol.field_file`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpyError(pub String);

/// Parses and checks a field file: magic, version 1.0, a header length that ends on a multiple of
/// 64, `descr` in the subset, C order, two or three dimensions, and a size that matches exactly.
pub fn parse(bytes: &[u8]) -> Result<Header, NpyError> {
    let err = |s: &str| Err(NpyError(s.to_owned()));
    if bytes.len() < PREAMBLE || &bytes[..8] != MAGIC {
        return err("not an .npy 1.0 file (magic or version)");
    }
    let hlen = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let data_offset = PREAMBLE + hlen;
    if !data_offset.is_multiple_of(64) {
        return err("header does not end on a multiple of 64 bytes");
    }
    if bytes.len() < data_offset || bytes[data_offset - 1] != b'\n' {
        return err("header is not ended by LF");
    }
    let text = std::str::from_utf8(&bytes[PREAMBLE..data_offset - 1])
        .map_err(|_| NpyError("header is not ASCII".into()))?
        .trim_end_matches(' ');
    let rest = text
        .strip_prefix("{'descr': '")
        .ok_or_else(|| NpyError("header does not start with descr".into()))?;
    let (descr_text, rest) = rest
        .split_once('\'')
        .ok_or_else(|| NpyError("unterminated descr".into()))?;
    let descr = match descr_text {
        "<f4" => Descr::F4,
        "|u1" => Descr::U1,
        other => return Err(NpyError(format!("descr {other} is outside the subset"))),
    };
    let rest = rest
        .strip_prefix(", 'fortran_order': False, 'shape': (")
        .ok_or_else(|| NpyError("not C order, or members out of NumPy's order".into()))?;
    let (dims, tail) = rest
        .split_once(')')
        .ok_or_else(|| NpyError("unterminated shape".into()))?;
    if tail != ", }" {
        return err("unexpected text after shape");
    }
    let mut shape = Vec::new();
    for d in dims.split(',').map(str::trim).filter(|d| !d.is_empty()) {
        if !d.bytes().all(|b| b.is_ascii_digit()) {
            return Err(NpyError(format!("shape element {d} is not an integer")));
        }
        shape.push(
            d.parse::<usize>()
                .map_err(|_| NpyError(format!("shape element {d} is too large")))?,
        );
    }
    if !(2..=3).contains(&shape.len()) {
        return Err(NpyError(format!(
            "{} dimensions; fields have 2 or 3",
            shape.len()
        )));
    }
    let count = shape
        .iter()
        .try_fold(1usize, |a, &d| a.checked_mul(d))
        .and_then(|n| n.checked_mul(descr.size()))
        .ok_or_else(|| NpyError("shape overflows".into()))?;
    if bytes.len() - data_offset != count {
        return Err(NpyError(format!(
            "{} data bytes where the header gives {count}",
            bytes.len() - data_offset
        )));
    }
    Ok(Header {
        descr,
        shape,
        data_offset,
    })
}

/// A whole f32 field file: header then little-endian samples.
pub fn f32_file(shape: &[usize], values: &[f32]) -> Vec<u8> {
    let mut out = header(Descr::F4, shape);
    out.reserve(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_numpy_text_padded_to_64() {
        let h = header(Descr::F4, &[513, 513]);
        assert_eq!(h.len(), 128);
        let expect = "{'descr': '<f4', 'fortran_order': False, 'shape': (513, 513), }";
        assert_eq!(&h[10..10 + expect.len()], expect.as_bytes());
        assert_eq!(*h.last().unwrap(), b'\n');
        assert_eq!(u16::from_le_bytes([h[8], h[9]]) as usize, h.len() - 10);
        assert_eq!(header(Descr::U1, &[2, 3, 4]).len() % 64, 0);
    }

    #[test]
    fn parse_round_trips_and_checks_size() {
        let f = f32_file(&[2, 3], &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        let h = parse(&f).unwrap();
        assert_eq!(h.shape, vec![2, 3]);
        assert_eq!(h.data_offset, 128);
        assert!(parse(&f[..f.len() - 1]).is_err());
    }

    #[test]
    fn parse_accepts_wider_padding() {
        // NumPy 1.x and 2.x add room for a growing axis; still a multiple of 64.
        let text = "{'descr': '<f4', 'fortran_order': False, 'shape': (1, 1), }";
        let mut f = b"\x93NUMPY\x01\x00".to_vec();
        let total = 128;
        f.extend_from_slice(&((total - 10) as u16).to_le_bytes());
        f.extend_from_slice(text.as_bytes());
        f.resize(total - 1, b' ');
        f.push(b'\n');
        f.extend_from_slice(&1.0f32.to_le_bytes());
        assert_eq!(parse(&f).unwrap().data_offset, 128);
    }

    #[test]
    fn parse_refuses_outside_the_subset() {
        let good = f32_file(&[1, 1], &[0.0]);
        let mut big_endian = good.clone();
        big_endian[21] = b'>';
        let mut fortran = good.clone();
        let at = good.windows(5).position(|w| w == b"False").unwrap();
        fortran[at..at + 5].copy_from_slice(b"True ");
        let mut v2 = good.clone();
        v2[6] = 2;
        for bad in [big_endian, fortran, v2, header(Descr::F4, &[4])] {
            assert!(parse(&bad).is_err());
        }
    }
}
