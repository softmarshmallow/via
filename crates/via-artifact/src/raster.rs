use std::io::{self, Read, Write};
use std::path::Path;

pub const MAGIC: [u8; 4] = *b"VRAS";
pub const FORMAT_VERSION: u32 = 1;

mod sealed {
    pub trait Sealed {}
}

/// Scalar types storable in a `.vrast` raster. Sealed: the artifact format
/// admits exactly these four, per ADR 0001.
pub trait Scalar: sealed::Sealed + Copy + Default {
    const DTYPE: u32;
    const SIZE: usize;
    fn write_le(&self, out: &mut Vec<u8>);
    fn read_le(bytes: &[u8]) -> Self;
}

macro_rules! impl_scalar {
    ($t:ty, $tag:expr) => {
        impl sealed::Sealed for $t {}
        impl Scalar for $t {
            const DTYPE: u32 = $tag;
            const SIZE: usize = std::mem::size_of::<$t>();
            fn write_le(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
            fn read_le(bytes: &[u8]) -> Self {
                <$t>::from_le_bytes(bytes.try_into().expect("scalar size"))
            }
        }
    };
}

impl_scalar!(i32, 1);
impl_scalar!(u32, 2);
impl_scalar!(u64, 3);
impl_scalar!(f32, 4);

/// A row-major grid of scalars with physical cell size. The unit of the
/// values is the owning stage's business (heights are i32 centimetres,
/// drainage areas are u64 upslope cell counts, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct Raster<T> {
    pub width: u32,
    pub height: u32,
    pub cell_size_cm: u32,
    pub data: Vec<T>,
}

impl<T: Scalar> Raster<T> {
    pub fn new(width: u32, height: u32, cell_size_cm: u32, fill: T) -> Self {
        Self {
            width,
            height,
            cell_size_cm,
            data: vec![fill; (width as usize) * (height as usize)],
        }
    }

    pub fn from_data(width: u32, height: u32, cell_size_cm: u32, data: Vec<T>) -> Self {
        assert_eq!(data.len(), (width as usize) * (height as usize));
        Self {
            width,
            height,
            cell_size_cm,
            data,
        }
    }

    /// Serialized bytes: header then row-major little-endian payload.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(24 + self.data.len() * T::SIZE);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&T::DTYPE.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.cell_size_cm.to_le_bytes());
        for v in &self.data {
            v.write_le(&mut out);
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        let err = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());
        if bytes.len() < 24 {
            return Err(err("raster too short for header"));
        }
        if bytes[0..4] != MAGIC {
            return Err(err("bad magic"));
        }
        let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        if word(4) != FORMAT_VERSION {
            return Err(err("unsupported format version"));
        }
        if word(8) != T::DTYPE {
            return Err(err("dtype mismatch"));
        }
        let (width, height, cell_size_cm) = (word(12), word(16), word(20));
        // decode is the trust boundary for on-disk artifacts: dimension
        // arithmetic must be checked, never wrapping, or a crafted header
        // panics instead of returning Err.
        let n = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| err("dimensions overflow"))?;
        let expected = n
            .checked_mul(T::SIZE)
            .ok_or_else(|| err("payload size overflows"))?;
        let payload = &bytes[24..];
        if payload.len() != expected {
            return Err(err("payload length mismatch"));
        }
        let mut data = Vec::with_capacity(n);
        for chunk in payload.chunks_exact(T::SIZE) {
            data.push(T::read_le(chunk));
        }
        Ok(Self {
            width,
            height,
            cell_size_cm,
            data,
        })
    }

    pub fn write_file(&self, path: &Path) -> io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        f.write_all(&self.encode())
    }

    pub fn read_file(path: &Path) -> io::Result<Self> {
        let with_path = |e: io::Error| io::Error::new(e.kind(), format!("{}: {e}", path.display()));
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(with_path)?
            .read_to_end(&mut bytes)
            .map_err(with_path)?;
        Self::decode(&bytes).map_err(with_path)
    }

    /// Content hash of the encoded raster; the determinism witness.
    pub fn blake3_hex(&self) -> String {
        blake3::hash(&self.encode()).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_i32() {
        let r = Raster::from_data(3, 2, 20_000, vec![1i32, -2, 3, 4, -5, 6]);
        let back = Raster::<i32>::decode(&r.encode()).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn dtype_mismatch_rejected() {
        let r = Raster::from_data(1, 1, 100, vec![1u32]);
        assert!(Raster::<i32>::decode(&r.encode()).is_err());
    }

    #[test]
    fn crafted_header_overflow_rejected_not_panicking() {
        // width = height = 0x8000_0000 → n·SIZE wraps to 0 in unchecked
        // arithmetic, which would pass the length check and then blow up.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&<u64 as Scalar>::DTYPE.to_le_bytes());
        bytes.extend_from_slice(&0x8000_0000u32.to_le_bytes());
        bytes.extend_from_slice(&0x8000_0000u32.to_le_bytes());
        bytes.extend_from_slice(&100u32.to_le_bytes());
        assert!(Raster::<u64>::decode(&bytes).is_err());
    }
}
