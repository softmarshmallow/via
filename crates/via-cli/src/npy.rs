//! Array files (`build.md` 3.2): the NumPy `.npy` 1.0 subset, for the three element types a build
//! writes. `via-proto`'s module covers `<f4` and `|u1` for steps; a build adds `<i4` for the
//! instance columns, so the header is written here for all three, by the same rule: NumPy's own
//! dict text, padded with spaces to the smallest multiple of 64 bytes, ended by LF.

/// The element types of a build's array files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Descr {
    F4,
    I4,
    U1,
}

impl Descr {
    pub fn text(self) -> &'static str {
        match self {
            Descr::F4 => "<f4",
            Descr::I4 => "<i4",
            Descr::U1 => "|u1",
        }
    }

    pub fn size(self) -> usize {
        match self {
            Descr::F4 | Descr::I4 => 4,
            Descr::U1 => 1,
        }
    }

    fn parse(t: &str) -> Option<Descr> {
        match t {
            "<f4" => Some(Descr::F4),
            "<i4" => Some(Descr::I4),
            "|u1" => Some(Descr::U1),
            _ => None,
        }
    }
}

const MAGIC: &[u8] = b"\x93NUMPY\x01\x00";
const PREAMBLE: usize = 10;

/// The header of an array of `descr` and `shape`.
pub fn header(descr: Descr, shape: &[usize]) -> Vec<u8> {
    let dims: Vec<String> = shape.iter().map(usize::to_string).collect();
    let tuple = match dims.len() {
        1 => format!("({},)", dims[0]),
        _ => format!("({})", dims.join(", ")),
    };
    let text = format!(
        "{{'descr': '{}', 'fortran_order': False, 'shape': {}, }}",
        descr.text(),
        tuple
    );
    let unpadded = PREAMBLE + text.len() + 1;
    let pad = (64 - unpadded % 64) % 64;
    let len = u16::try_from(text.len() + pad + 1).unwrap_or(u16::MAX);
    let mut out = Vec::with_capacity(unpadded + pad);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.resize(out.len() + pad, b' ');
    out.push(b'\n');
    out
}

/// An `<f4` file. Every value must already be finite; `-0` is written as `+0` (rule 4).
pub fn f4(shape: &[usize], values: &[f32]) -> Vec<u8> {
    let mut out = header(Descr::F4, shape);
    out.reserve(values.len() * 4);
    for v in values {
        let v = if *v == 0.0 { 0.0f32 } else { *v };
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// An `<i4` file.
pub fn i4(values: &[i32]) -> Vec<u8> {
    let mut out = header(Descr::I4, &[values.len()]);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A `|u1` file.
pub fn u1(shape: &[usize], values: &[u8]) -> Vec<u8> {
    let mut out = header(Descr::U1, shape);
    out.extend_from_slice(values);
    out
}

/// A parsed header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub descr: Descr,
    pub shape: Vec<usize>,
    pub data_offset: usize,
}

impl Header {
    /// Elements the shape holds.
    pub fn count(&self) -> usize {
        self.shape.iter().product()
    }
}

/// Reads a header of the subset; the error gives the byte offset and the reason.
pub fn parse(bytes: &[u8]) -> Result<Header, (usize, String)> {
    if bytes.len() < PREAMBLE || &bytes[..8] != MAGIC {
        return Err((0, "not an .npy 1.0 file".into()));
    }
    let len = usize::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    let end = PREAMBLE + len;
    if end > bytes.len() || !end.is_multiple_of(64) {
        return Err((
            8,
            "the header length does not end on a multiple of 64".into(),
        ));
    }
    let text = std::str::from_utf8(&bytes[PREAMBLE..end])
        .map_err(|_| (PREAMBLE, "the header is not text".to_string()))?;
    if !text.ends_with('\n') {
        return Err((end - 1, "the header does not end with LF".into()));
    }
    let t = text.trim_end();
    let descr_at = t
        .find("'descr': '")
        .ok_or((PREAMBLE, "no descr".to_string()))?;
    let rest = &t[descr_at + 10..];
    let close = rest.find('\'').ok_or((PREAMBLE, "bad descr".to_string()))?;
    let descr = Descr::parse(&rest[..close]).ok_or((
        PREAMBLE + descr_at,
        format!("descr `{}` is outside the subset", &rest[..close]),
    ))?;
    if !t.contains("'fortran_order': False") {
        return Err((PREAMBLE, "fortran_order is not False".into()));
    }
    let s_at = t
        .find("'shape': (")
        .ok_or((PREAMBLE, "no shape".to_string()))?;
    let rest = &t[s_at + 10..];
    let close = rest.find(')').ok_or((PREAMBLE, "bad shape".to_string()))?;
    let mut shape = Vec::new();
    for part in rest[..close].split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        shape.push(
            p.parse::<usize>()
                .map_err(|_| (PREAMBLE + s_at, "bad shape".to_string()))?,
        );
    }
    if shape.is_empty() || shape.len() > 3 {
        return Err((
            PREAMBLE + s_at,
            "the shape has no dimension or more than three".into(),
        ));
    }
    let h = Header {
        descr,
        shape,
        data_offset: end,
    };
    let want = h.count() * descr.size();
    if bytes.len() - end != want {
        return Err((
            end,
            format!(
                "the data is {} bytes; the header says {want}",
                bytes.len() - end
            ),
        ));
    }
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_64_aligned_and_reads_back() {
        for (d, shape) in [
            (Descr::F4, vec![33, 65]),
            (Descr::I4, vec![0]),
            (Descr::U1, vec![3, 33, 33]),
        ] {
            let n: usize = shape.iter().product();
            let mut b = header(d, &shape);
            assert_eq!(b.len() % 64, 0);
            b.resize(b.len() + n * d.size(), 0);
            let h = parse(&b).unwrap();
            assert_eq!(h.shape, shape);
            assert_eq!(h.descr, d);
        }
    }

    #[test]
    fn negative_zero_is_written_positive() {
        let b = f4(&[1], &[-0.0]);
        assert_eq!(&b[b.len() - 4..], &[0, 0, 0, 0]);
    }
}
