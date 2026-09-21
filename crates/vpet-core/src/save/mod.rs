//! Blob layout and load-error codes. See docs/SAVE_FORMAT.md and docs/HOST_ABI.md's error
//! table. The header/CRC framing here is version-independent; `v1.rs` (and a future `v2.rs`,
//! ...) hold the versioned payload shape.

pub mod crc;
pub mod v1;

pub const MAGIC: [u8; 4] = *b"VPET";

/// Load error codes, matching docs/HOST_ABI.md exactly (`-1..=-6`).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    BadMagic = -1,
    BadCrc = -2,
    VersionTooNew = -3,
    TooLong = -4,
    Decode = -5,
    BadContentRef = -6,
}

impl LoadError {
    pub const fn code(self) -> i32 {
        self as i32
    }
}

/// Write `magic | save_version | content_hash | payload_len | payload | crc32` into `out` per
/// docs/SAVE_FORMAT.md's layout. Returns the total bytes written, or `None` if `out` is too
/// small or `payload` exceeds `u16::MAX`.
pub fn write_header_and_payload(
    out: &mut [u8],
    save_version: u16,
    content_hash: u32,
    payload: &[u8],
) -> Option<usize> {
    let payload_len: u16 = payload.len().try_into().ok()?;
    let total = 12usize.checked_add(payload.len())?.checked_add(4)?;
    if out.len() < total {
        return None;
    }
    out[0..4].copy_from_slice(&MAGIC);
    out[4..6].copy_from_slice(&save_version.to_le_bytes());
    out[6..10].copy_from_slice(&content_hash.to_le_bytes());
    out[10..12].copy_from_slice(&payload_len.to_le_bytes());
    out[12..12 + payload.len()].copy_from_slice(payload);
    let crc = crc::crc32(&out[0..12 + payload.len()]);
    out[12 + payload.len()..total].copy_from_slice(&crc.to_le_bytes());
    Some(total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub save_version: u16,
    pub content_hash: u32,
}

/// Parse and CRC-verify `blob`'s header, returning the header and a slice of the payload bytes
/// (still postcard-encoded; the caller decodes with the version-specific `decode`).
pub fn read_header_and_payload(blob: &[u8]) -> Result<(Header, &[u8]), LoadError> {
    if blob.len() < 4 {
        return Err(LoadError::TooLong);
    }
    if blob[0..4] != MAGIC {
        return Err(LoadError::BadMagic);
    }
    if blob.len() < 12 {
        return Err(LoadError::TooLong);
    }
    let save_version = u16::from_le_bytes([blob[4], blob[5]]);
    let content_hash = u32::from_le_bytes([blob[6], blob[7], blob[8], blob[9]]);
    let payload_len = u16::from_le_bytes([blob[10], blob[11]]) as usize;
    let total = 12 + payload_len + 4;
    if blob.len() < total {
        return Err(LoadError::TooLong);
    }
    let crc_stored = u32::from_le_bytes([
        blob[12 + payload_len],
        blob[13 + payload_len],
        blob[14 + payload_len],
        blob[15 + payload_len],
    ]);
    let crc_computed = crc::crc32(&blob[0..12 + payload_len]);
    if crc_stored != crc_computed {
        return Err(LoadError::BadCrc);
    }
    Ok((
        Header {
            save_version,
            content_hash,
        },
        &blob[12..12 + payload_len],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_header() {
        let payload = b"hello world";
        let mut out = [0u8; 64];
        let len = write_header_and_payload(&mut out, 1, 0xAABBCCDD, payload).unwrap();
        let (header, got_payload) = read_header_and_payload(&out[0..len]).unwrap();
        assert_eq!(header.save_version, 1);
        assert_eq!(header.content_hash, 0xAABBCCDD);
        assert_eq!(got_payload, payload);
    }

    #[test]
    fn bad_magic() {
        let mut out = [0u8; 64];
        write_header_and_payload(&mut out, 1, 0, b"x").unwrap();
        out[0] = b'X'; // corrupt magic
        assert_eq!(
            read_header_and_payload(&out[0..17]),
            Err(LoadError::BadMagic)
        );
    }

    #[test]
    fn bad_crc() {
        let mut out = [0u8; 64];
        let len = write_header_and_payload(&mut out, 1, 0, b"x").unwrap();
        out[len - 1] ^= 0xFF; // flip a bit in the CRC
        assert_eq!(
            read_header_and_payload(&out[0..len]),
            Err(LoadError::BadCrc)
        );
    }

    #[test]
    fn truncated_blob() {
        let mut out = [0u8; 64];
        let len = write_header_and_payload(&mut out, 1, 0, b"hello").unwrap();
        assert_eq!(
            read_header_and_payload(&out[0..len - 1]),
            Err(LoadError::TooLong)
        );
        assert_eq!(read_header_and_payload(&[]), Err(LoadError::TooLong));
    }
}
