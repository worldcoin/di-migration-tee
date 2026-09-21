//! Bounded decompression of the inline PCP.

use std::io::Read;

/// Failures decompressing a request payload.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The payload did not start with the gzip magic bytes.
    #[error("payload is not gzip")]
    NotGzip,
    /// The payload expanded past the ceiling.
    #[error("PCP expanded past the {limit} byte limit")]
    TooLarge {
        /// The ceiling that was exceeded.
        limit: usize,
    },
    /// The stream was truncated or malformed.
    #[error("gzip stream is corrupt: {0}")]
    Corrupt(String),
}

/// Checked before decoding so a non-gzip body is a clear rejection, not an inflate error.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// Decompresses `payload`, refusing to hold more than `limit` bytes. CPU-bound; call it off
/// the async runtime.
///
/// # Errors
///
/// A payload that is not gzip, expands past `limit`, or does not decode.
pub fn decompress(payload: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    if !payload.starts_with(&GZIP_MAGIC) {
        return Err(Error::NotGzip);
    }

    // One byte past the limit, so an over-large payload is caught without buffering it. The
    // size the stream claims to expand to is never consulted.
    let mut pcp = Vec::new();
    flate2::read::GzDecoder::new(payload)
        .take(limit as u64 + 1)
        .read_to_end(&mut pcp)
        .map_err(|error| Error::Corrupt(error.to_string()))?;

    if pcp.len() > limit {
        return Err(Error::TooLarge { limit });
    }

    Ok(pcp)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{Error, decompress};

    fn gzip(payload: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).expect("should encode");
        encoder.finish().expect("should finish")
    }

    #[test]
    fn a_gzip_payload_round_trips() {
        let payload = b"a perfectly ordinary pcp".to_vec();

        assert_eq!(
            decompress(&gzip(&payload), 1024).expect("should decode"),
            payload
        );
    }

    #[test]
    fn a_payload_that_is_not_gzip_is_rejected() {
        let error = decompress(b"not compressed at all", 1024).expect_err("should reject");

        assert!(matches!(error, Error::NotGzip));
    }

    /// The property that matters: a bomb stops at the limit instead of being allocated.
    #[test]
    fn a_compression_bomb_stops_at_the_limit() {
        let compressed = gzip(&vec![0u8; 8 * 1024 * 1024]);
        assert!(compressed.len() < 64 * 1024, "the bomb should be small");

        let error = decompress(&compressed, 1024).expect_err("should reject");

        assert!(matches!(error, Error::TooLarge { limit: 1024 }));
    }

    #[test]
    fn a_payload_exactly_at_the_limit_is_accepted() {
        let payload = vec![3u8; 1024];

        assert_eq!(
            decompress(&gzip(&payload), 1024).expect("should decode"),
            payload
        );
    }

    #[test]
    fn a_truncated_stream_is_rejected() {
        let compressed = gzip(b"a perfectly ordinary pcp");

        let error = decompress(&compressed[..compressed.len() / 2], 1024).expect_err("reject");

        assert!(matches!(error, Error::Corrupt(_)));
    }
}
