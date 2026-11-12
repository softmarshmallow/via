//! Message framing (`spec/protocol.md` 3.1).
//!
//! The LSP base protocol's frame, cut down: exactly one header, `Content-Length: <n>\r\n`, then a
//! blank line, then `n` bytes of UTF-8 JSON. Any other header is refused rather than skipped, since
//! the likeliest stray bytes on this descriptor are a `print` that escaped the launcher, and that
//! must be named (`protocol.framing`), not tolerated.

use std::io::{self, BufRead, Read, Write};

/// The largest frame either side accepts. Data larger than this travels as blobs (R20).
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

const PREFIX: &[u8] = b"Content-Length: ";
/// A header line longer than this is not ours; `Content-Length: ` plus 20 digits plus CRLF fits.
const MAX_HEADER_LINE: u64 = 64;

#[derive(Debug)]
pub enum FrameError {
    /// The descriptor failed, or ended inside a frame.
    Io(io::Error),
    /// The bytes are not `Content-Length: <n>\r\n\r\n`. Holds what was read, for the log.
    Header(Vec<u8>),
    /// The header announces more than `MAX_FRAME` bytes.
    TooLarge(u64),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Io(e) => write!(f, "{e}"),
            FrameError::Header(b) => {
                write!(f, "not a frame header: {:?}", String::from_utf8_lossy(b))
            }
            FrameError::TooLarge(n) => write!(f, "frame of {n} bytes exceeds {MAX_FRAME}"),
        }
    }
}

/// Reads one frame. `Ok(None)` is a clean end of input between frames: the peer has gone.
pub fn read_frame<R: BufRead>(r: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut line = Vec::with_capacity(32);
    r.by_ref()
        .take(MAX_HEADER_LINE)
        .read_until(b'\n', &mut line)
        .map_err(FrameError::Io)?;
    if line.is_empty() {
        return Ok(None);
    }
    let n = parse_length(&line).ok_or_else(|| FrameError::Header(line.clone()))?;
    let mut blank = Vec::with_capacity(2);
    r.by_ref()
        .take(2)
        .read_until(b'\n', &mut blank)
        .map_err(FrameError::Io)?;
    if blank != b"\r\n" {
        let mut seen = line;
        seen.extend_from_slice(&blank);
        return Err(FrameError::Header(seen));
    }
    if n > MAX_FRAME as u64 {
        return Err(FrameError::TooLarge(n));
    }
    let mut body = vec![0u8; n as usize];
    r.read_exact(&mut body).map_err(FrameError::Io)?;
    Ok(Some(body))
}

fn parse_length(line: &[u8]) -> Option<u64> {
    let digits = line.strip_prefix(PREFIX)?.strip_suffix(b"\r\n")?;
    if digits.is_empty() || digits.len() > 20 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// Writes one frame and flushes, so a message never waits in a buffer while its peer waits for it.
pub fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> Result<(), FrameError> {
    if body.len() > MAX_FRAME {
        return Err(FrameError::TooLarge(body.len() as u64));
    }
    let mut head = Vec::with_capacity(32);
    head.extend_from_slice(PREFIX);
    head.extend_from_slice(body.len().to_string().as_bytes());
    head.extend_from_slice(b"\r\n\r\n");
    w.write_all(&head).map_err(FrameError::Io)?;
    w.write_all(body).map_err(FrameError::Io)?;
    w.flush().map_err(FrameError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip_two_frames() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"{\"a\":1}").unwrap();
        write_frame(&mut buf, "{\"é\":2}".as_bytes()).unwrap();
        assert!(buf.starts_with(b"Content-Length: 7\r\n\r\n{\"a\":1}"));
        let mut r = Cursor::new(buf);
        assert_eq!(read_frame(&mut r).unwrap().unwrap(), b"{\"a\":1}");
        // Content-Length counts bytes, not characters.
        assert_eq!(read_frame(&mut r).unwrap().unwrap(), "{\"é\":2}".as_bytes());
        assert!(read_frame(&mut r).unwrap().is_none());
    }

    #[test]
    fn stray_print_is_a_header_error() {
        let mut r = Cursor::new(b"hello\n".to_vec());
        assert!(matches!(read_frame(&mut r), Err(FrameError::Header(_))));
    }

    #[test]
    fn other_headers_and_lf_only_are_refused() {
        for bad in [
            &b"Content-Type: x\r\n\r\n{}"[..],
            b"Content-Length: 2\n\n{}",
            b"Content-Length: 2\r\nContent-Type: x\r\n\r\n{}",
            b"content-length: 2\r\n\r\n{}",
            b"Content-Length: -2\r\n\r\n{}",
            b"Content-Length: \r\n\r\n{}",
        ] {
            let mut r = Cursor::new(bad.to_vec());
            assert!(
                matches!(read_frame(&mut r), Err(FrameError::Header(_))),
                "{:?}",
                String::from_utf8_lossy(bad)
            );
        }
    }

    #[test]
    fn too_large_is_refused_before_reading() {
        let mut r = Cursor::new(b"Content-Length: 16777217\r\n\r\n".to_vec());
        assert!(matches!(
            read_frame(&mut r),
            Err(FrameError::TooLarge(16_777_217))
        ));
        let mut sink = Vec::new();
        let big = vec![b' '; MAX_FRAME + 1];
        assert!(matches!(
            write_frame(&mut sink, &big),
            Err(FrameError::TooLarge(_))
        ));
    }

    #[test]
    fn truncated_body_is_io() {
        let mut r = Cursor::new(b"Content-Length: 10\r\n\r\n{}".to_vec());
        assert!(matches!(read_frame(&mut r), Err(FrameError::Io(_))));
    }
}
