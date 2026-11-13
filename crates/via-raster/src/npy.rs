//! Array files (`build.md` 3.2, `protocol.md` 4.2): the NumPy `.npy` 1.0 subset every field, weights
//! chunk, cover and instance column is written in, and that a field `file` is read from
//! (`fields.md` 4.1).
//!
//! The build's set is wider than a step's field file: `build.md` 3.2 adds `<i4` and one-dimensional
//! instance columns. A step's field file is checked against its own, narrower rule by
//! [`Header::check_field`]. `via-proto`'s `npy` module holds the M0 probe's copy of the step
//! subset; it is meant to call this one (`via-proto` sits above `via-raster`).

use crate::error::RasterError;
use crate::raster::Raster;

/// The element types the build writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Descr {
    /// `<f4`: field samples, weights in a step's file, and the `z`, `scale`, `tint` columns.
    F4,
    /// `|u1`: weights chunks and covers.
    U1,
    /// `<i4`: the `x`, `y`, `face` columns.
    I4,
}

impl Descr {
    /// The text in the header.
    pub fn text(self) -> &'static str {
        match self {
            Descr::F4 => "<f4",
            Descr::U1 => "|u1",
            Descr::I4 => "<i4",
        }
    }

    /// Bytes per element.
    pub fn size(self) -> usize {
        match self {
            Descr::F4 | Descr::I4 => 4,
            Descr::U1 => 1,
        }
    }
}

const MAGIC: &[u8] = b"\x93NUMPY\x01\x00";
/// Magic, version and the u16 header length.
const PREAMBLE: usize = 10;

/// The header for an array of `descr` and `shape`: NumPy's own dict text, padded with spaces to the
/// smallest total that is a multiple of 64 bytes and ended by LF (`build.md` 3.2).
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

/// Why bytes are outside the subset. The caller picks the code: `field.file_format` for an
/// author's `file`, `protocol.field_file` for a step's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpyError(pub String);

impl Header {
    /// The number of elements the shape asks for, or `None` when it overflows.
    pub fn count(&self) -> Option<usize> {
        self.shape.iter().try_fold(1usize, |a, &d| a.checked_mul(d))
    }

    /// The step subset of `protocol.md` 4.2: `<f4` or `|u1`, two or three dimensions.
    pub fn check_field(&self) -> Result<(), NpyError> {
        if self.descr == Descr::I4 {
            return Err(NpyError(
                "descr <i4 is outside a field file's subset".into(),
            ));
        }
        if !(2..=3).contains(&self.shape.len()) {
            return Err(NpyError(format!(
                "{} dimensions; a field file has 2 or 3",
                self.shape.len()
            )));
        }
        Ok(())
    }
}

/// Parses a header only: magic, version 1.0, a header length that ends on a multiple of 64, a
/// `descr` the build knows, C order and one to three dimensions. The data is not looked at, so a
/// size check can come first (`field.file_too_large` before any sample is read).
pub fn parse_header(bytes: &[u8]) -> Result<Header, NpyError> {
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
        "<i4" => Descr::I4,
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
    if !(1..=3).contains(&shape.len()) {
        return Err(NpyError(format!(
            "{} dimensions; the build writes 1 to 3",
            shape.len()
        )));
    }
    Ok(Header {
        descr,
        shape,
        data_offset,
    })
}

/// Parses a header and checks that the data is exactly the size it gives, no trailing bytes.
pub fn parse(bytes: &[u8]) -> Result<Header, NpyError> {
    let h = parse_header(bytes)?;
    let size = h
        .count()
        .and_then(|n| n.checked_mul(h.descr.size()))
        .ok_or_else(|| NpyError("shape overflows".into()))?;
    if bytes.len() - h.data_offset != size {
        return Err(NpyError(format!(
            "{} data bytes where the header gives {size}",
            bytes.len() - h.data_offset
        )));
    }
    Ok(h)
}

/// A whole `<f4` file: header, then little-endian samples in C order.
pub fn f32_file(shape: &[usize], values: &[f32]) -> Vec<u8> {
    let mut out = header(Descr::F4, shape);
    out.reserve(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A whole `|u1` file.
pub fn u8_file(shape: &[usize], values: &[u8]) -> Vec<u8> {
    let mut out = header(Descr::U1, shape);
    out.extend_from_slice(values);
    out
}

/// A whole `<i4` file.
pub fn i32_file(shape: &[usize], values: &[i32]) -> Vec<u8> {
    let mut out = header(Descr::I4, shape);
    out.reserve(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A scalar field as its array file (`build.md` 6.1): `<f4`, shape `(rows, columns)` of its
/// window, row 0 the south. Values are already narrowed and stored (no `-0`, all finite).
pub fn field_file(values: &Raster<f32>) -> Vec<u8> {
    let w = values.window();
    f32_file(&[w.height() as usize, w.width() as usize], values.values())
}

/// A cover as its array file (`build.md` 6.3): `|u1`, shape `(rows, columns)`.
pub fn cover_file(values: &Raster<u8>) -> Vec<u8> {
    let w = values.window();
    u8_file(&[w.height() as usize, w.width() as usize], values.values())
}

/// `min` and `max` of a scalar field over every post it holds (`build.md` 6.1), widened to
/// doubles, by `numbers.md` 8's fixed-order reductions.
pub fn field_range(values: &Raster<f32>) -> Result<(f64, f64), RasterError> {
    let v: Vec<f64> = values.values().iter().map(|&x| f64::from(x)).collect();
    let lo = via_num::minimum(&v).map_err(|e| RasterError::from_num(e, (0, 0)))?;
    let hi = via_num::maximum(&v).map_err(|e| RasterError::from_num(e, (0, 0)))?;
    Ok((lo, hi))
}

/// A field `file` read as samples (`fields.md` 4.1): rows from the south, columns from the west.
#[derive(Clone, Debug, PartialEq)]
pub struct Samples {
    pub rows: u64,
    pub columns: u64,
    /// In C order, row 0 the south; every value finite and no `-0`.
    pub values: Vec<f32>,
}

impl Samples {
    /// The sample at column `c`, row `r`.
    pub fn at(&self, c: u64, r: u64) -> f32 {
        self.values[(r * self.columns + c) as usize]
    }
}

/// Reads an author's array file (`fields.md` 4.1), in the chapter's order: a header outside the
/// subset, a `shape` that is not 2-D, a `descr` other than `<f4` or a dimension under 2 is
/// `field.file_format`; more samples than `limit` is `field.file_too_large`, from the header alone,
/// before any sample is read; a size that does not match is `field.file_format`; the first
/// non-finite sample in file order is `field.file_nonfinite`. `-0` reads as `+0`.
pub fn read_samples(bytes: &[u8], limit: u64) -> Result<Samples, RasterError> {
    let format = |reason: String| RasterError::FileFormat { reason };
    let h = parse_header(bytes).map_err(|e| format(e.0))?;
    if h.shape.len() != 2 {
        return Err(format(format!(
            "shape has {} dimensions; a field file is 2-D (rows, columns)",
            h.shape.len()
        )));
    }
    if h.descr != Descr::F4 {
        return Err(format(format!("descr {} is not <f4", h.descr.text())));
    }
    let (rows, columns) = (h.shape[0] as u64, h.shape[1] as u64);
    if rows < 2 || columns < 2 {
        return Err(format(format!(
            "{rows} by {columns} samples; a raster has at least 2 in each direction"
        )));
    }
    if u128::from(rows) * u128::from(columns) > u128::from(limit) {
        return Err(RasterError::FileTooLarge {
            rows,
            columns,
            limit,
        });
    }
    let h = parse(bytes).map_err(|e| format(e.0))?;
    let data = &bytes[h.data_offset..];
    let mut values = Vec::with_capacity((rows * columns) as usize);
    for (k, b) in data.chunks_exact(4).enumerate() {
        let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        if !v.is_finite() {
            let k = k as u64;
            return Err(RasterError::FileNonFinite {
                row: k / columns,
                column: k % columns,
            });
        }
        values.push(if v == 0.0 { 0.0 } else { v });
    }
    Ok(Samples {
        rows,
        columns,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::Window;

    #[test]
    fn header_is_numpy_text_padded_to_64() {
        let h = header(Descr::F4, &[513, 513]);
        assert_eq!(h.len(), 128);
        let expect = "{'descr': '<f4', 'fortran_order': False, 'shape': (513, 513), }";
        assert_eq!(&h[10..10 + expect.len()], expect.as_bytes());
        assert_eq!(*h.last().unwrap(), b'\n');
        assert_eq!(u16::from_le_bytes([h[8], h[9]]) as usize, h.len() - 10);
        assert_eq!(header(Descr::U1, &[2, 3, 4]).len() % 64, 0);
        let col = header(Descr::I4, &[0]);
        assert!(String::from_utf8_lossy(&col[10..]).contains("'shape': (0,), }"));
    }

    #[test]
    fn files_round_trip() {
        let r = Raster::from_values(
            Window::new(0, 0, 3, 2).unwrap(),
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.5],
        )
        .unwrap();
        let f = field_file(&r);
        let h = parse(&f).unwrap();
        assert_eq!(
            (h.descr, h.shape.clone(), h.data_offset),
            (Descr::F4, vec![2, 3], 128)
        );
        h.check_field().unwrap();
        let s = read_samples(&f, 1 << 26).unwrap();
        assert_eq!((s.rows, s.columns), (2, 3));
        assert_eq!(s.at(2, 1), 5.5);
        assert_eq!(field_range(&r).unwrap(), (0.0, 5.5));
        assert!(parse(&f[..f.len() - 1]).is_err());
        let ints = i32_file(&[3], &[-1, 0, 7]);
        let h = parse(&ints).unwrap();
        assert_eq!((h.descr, h.shape), (Descr::I4, vec![3]));
        assert!(Header {
            descr: Descr::I4,
            shape: vec![3],
            data_offset: 64
        }
        .check_field()
        .is_err());
    }

    #[test]
    fn field_files_refused_in_the_chapter_order() {
        let code = |b: &[u8]| {
            read_samples(b, 1 << 26)
                .map(|_| ())
                .map_err(|e| e.code().as_str())
        };
        // planes.npy, shape (2, 9, 9); double.npy, descr <f8.
        assert_eq!(
            code(&f32_file(&[2, 9, 9], &[0.0; 162])),
            Err("field.file_format")
        );
        let mut double = f32_file(&[9, 9], &[0.0; 81]);
        double[21] = b'8';
        assert_eq!(code(&double), Err("field.file_format"));
        assert_eq!(
            code(&f32_file(&[1, 4], &[0.0; 4])),
            Err("field.file_format")
        );
        // Too large: from the header alone, with no samples behind it.
        let big = header(Descr::F4, &[10_000, 10_000]);
        assert_eq!(code(&big), Err("field.file_too_large"));
        // A NaN at row 1, column 2.
        let mut v = [0.0f32; 9];
        v[5] = f32::NAN;
        match read_samples(&f32_file(&[3, 3], &v), 100) {
            Err(RasterError::FileNonFinite { row, column }) => assert_eq!((row, column), (1, 2)),
            other => panic!("{other:?}"),
        }
        let neg = read_samples(&f32_file(&[2, 2], &[-0.0; 4]), 100).unwrap();
        assert!(!neg.values[0].is_sign_negative());
    }

    #[test]
    fn wider_padding_is_read() {
        let text = "{'descr': '<f4', 'fortran_order': False, 'shape': (2, 2), }";
        let mut f = MAGIC.to_vec();
        let total = 128;
        f.extend_from_slice(&((total - 10) as u16).to_le_bytes());
        f.extend_from_slice(text.as_bytes());
        f.resize(total - 1, b' ');
        f.push(b'\n');
        for _ in 0..4 {
            f.extend_from_slice(&1.0f32.to_le_bytes());
        }
        assert_eq!(parse(&f).unwrap().data_offset, 128);
    }
}
