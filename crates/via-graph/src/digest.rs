//! The digest (`spec/graph.md` 3.1): BLAKE3 at 256 bits, written as 64 lowercase hex digits.

use std::fmt;

/// A 32-byte BLAKE3 digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    /// The digest of bytes.
    pub fn of(bytes: &[u8]) -> Digest {
        Digest(*blake3::hash(bytes).as_bytes())
    }

    /// The 32 raw bytes.
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// 64 lowercase hex digits.
    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push(HEX[usize::from(b >> 4)] as char);
            s.push(HEX[usize::from(b & 0xf)] as char);
        }
        s
    }

    /// Reads 64 lowercase hex digits; `None` for anything else.
    pub fn from_hex(s: &str) -> Option<Digest> {
        let b = s.as_bytes();
        if b.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (nibble(b[2 * i])? << 4) | nibble(b[2 * i + 1])?;
        }
        Some(Digest(out))
    }

    /// The store's fan-out folder: the first two hex digits.
    pub fn fan(&self) -> String {
        self.hex()[..2].to_string()
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.hex())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

/// The byte encoding of 3.1, also used for the store's records: integers are unsigned 64-bit
/// little-endian, a string is its byte length then its UTF-8 bytes, a digest its 32 raw bytes, a
/// list its count then its members.
#[derive(Default)]
pub struct Enc(pub Vec<u8>);

impl Enc {
    /// An empty encoding.
    pub fn new() -> Enc {
        Enc(Vec::new())
    }

    /// An unsigned 64-bit integer, little-endian.
    pub fn u64(&mut self, n: u64) -> &mut Enc {
        self.0.extend_from_slice(&n.to_le_bytes());
        self
    }

    /// A count or length.
    pub fn len(&mut self, n: usize) -> &mut Enc {
        self.u64(n as u64)
    }

    /// A byte string: its length, then its bytes.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Enc {
        self.len(b.len());
        self.0.extend_from_slice(b);
        self
    }

    /// A string: its byte length, then its UTF-8 bytes.
    pub fn str(&mut self, s: &str) -> &mut Enc {
        self.bytes(s.as_bytes())
    }

    /// A digest: its 32 raw bytes.
    pub fn digest(&mut self, d: &Digest) -> &mut Enc {
        self.0.extend_from_slice(&d.0);
        self
    }

    /// One byte (records only; keys use none).
    pub fn byte(&mut self, b: u8) -> &mut Enc {
        self.0.push(b);
        self
    }

    /// The encoded bytes.
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

/// Reads what [`Enc`] wrote. Every read is checked; a short or malformed input gives `None`,
/// which the store treats as a corrupt entry.
pub struct Dec<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Dec<'a> {
    /// A reader at the start of `b`.
    pub fn new(b: &'a [u8]) -> Dec<'a> {
        Dec { b, at: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let s = self.b.get(self.at..end)?;
        self.at = end;
        Some(s)
    }

    /// An unsigned 64-bit integer.
    pub fn u64(&mut self) -> Option<u64> {
        let s = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        Some(u64::from_le_bytes(a))
    }

    /// A count or length, refused when it could not fit what is left.
    pub fn length(&mut self) -> Option<usize> {
        let n = usize::try_from(self.u64()?).ok()?;
        (n <= self.b.len().saturating_sub(self.at)).then_some(n)
    }

    /// A count of items each at least one byte wide; refused when it could not fit.
    pub fn count(&mut self) -> Option<usize> {
        self.length()
    }

    /// A byte string.
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = self.length()?;
        self.take(n)
    }

    /// A string.
    pub fn str(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?.to_vec()).ok()
    }

    /// A digest.
    pub fn digest(&mut self) -> Option<Digest> {
        let s = self.take(32)?;
        let mut a = [0u8; 32];
        a.copy_from_slice(s);
        Some(Digest(a))
    }

    /// One byte.
    pub fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    /// True when every byte was read.
    pub fn done(&self) -> bool {
        self.at == self.b.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let d = Digest::of(b"abc");
        assert_eq!(Digest::from_hex(&d.hex()), Some(d));
        assert_eq!(
            d.hex(),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert!(Digest::from_hex(&d.hex().to_uppercase()).is_none());
        assert!(Digest::from_hex("ab").is_none());
    }

    #[test]
    fn decoder_refuses_short_input() {
        let mut e = Enc::new();
        e.str("hello").u64(7);
        let b = e.finish();
        let mut d = Dec::new(&b);
        assert_eq!(d.str().as_deref(), Some("hello"));
        assert_eq!(d.u64(), Some(7));
        assert!(d.done());
        let mut d = Dec::new(&b[..10]);
        assert!(d.str().is_none());
        let huge = u64::MAX.to_le_bytes();
        assert!(Dec::new(&huge).bytes().is_none());
    }
}
