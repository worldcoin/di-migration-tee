//! Bounded decompression of the inline PCP; the format is sniffed, not declared.

use std::io::Read;

/// Compression formats the host accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// RFC 1952 gzip.
    Gzip,
    /// RFC 8878 zstandard.
    Zstd,
}

/// Failures decompressing a request payload.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The payload began with neither the gzip nor the zstd magic bytes.
    #[error("payload is neither gzip nor zstd")]
    UnknownFormat,
    /// The payload expanded past the ceiling.
    #[error("PCP expanded past the {limit} byte limit")]
    TooLarge {
        /// The ceiling that was exceeded.
        limit: usize,
    },
    /// The stream was truncated or malformed.
    #[error("{0} stream is corrupt: {1}")]
    Corrupt(&'static str, String),
}

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// Identifies the compression format from the payload's leading bytes.
#[must_use]
pub fn detect(payload: &[u8]) -> Option<Format> {
    if payload.starts_with(&GZIP_MAGIC) {
        Some(Format::Gzip)
    } else if payload.starts_with(&ZSTD_MAGIC) {
        Some(Format::Zstd)
    } else {
        None
    }
}

/// Decompresses `payload`, refusing to hold more than `limit` bytes. CPU-bound; call it off
/// the async runtime.
///
/// # Errors
///
/// Unrecognized format, expansion past `limit`, or a stream that does not decode.
pub fn decompress(payload: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    let format = detect(payload).ok_or(Error::UnknownFormat)?;

    let reader: Box<dyn Read> = match format {
        Format::Gzip => Box::new(flate2::read::GzDecoder::new(payload)),
        Format::Zstd => Box::new(
            ruzstd::decoding::StreamingDecoder::new(payload)
                .map_err(|error| Error::Corrupt("zstd", error.to_string()))?,
        ),
    };

    // One byte past the limit: the decoder's claimed output size is attacker-controlled.
    let mut pcp = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut pcp)
        .map_err(|error| {
            let name = match format {
                Format::Gzip => "gzip",
                Format::Zstd => "zstd",
            };
            Error::Corrupt(name, error.to_string())
        })?;

    if pcp.len() > limit {
        return Err(Error::TooLarge { limit });
    }

    Ok(pcp)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{Error, Format, decompress, detect};

    fn gzip(payload: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).expect("should encode");
        encoder.finish().expect("should finish")
    }

    #[test]
    fn gzip_is_detected_and_round_trips() {
        let payload = b"a perfectly ordinary pcp".to_vec();
        let compressed = gzip(&payload);

        assert_eq!(detect(&compressed), Some(Format::Gzip));
        assert_eq!(
            decompress(&compressed, 1024).expect("should decode"),
            payload
        );
    }

    #[test]
    fn a_payload_in_no_known_format_is_rejected() {
        let error = decompress(b"not compressed at all", 1024).expect_err("should reject");

        assert!(matches!(error, Error::UnknownFormat));
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
        let compressed = gzip(&payload);

        assert_eq!(
            decompress(&compressed, 1024).expect("should decode"),
            payload
        );
    }

    #[test]
    fn a_truncated_stream_is_rejected() {
        let compressed = gzip(b"a perfectly ordinary pcp");
        let truncated = &compressed[..compressed.len() / 2];

        let error = decompress(truncated, 1024).expect_err("should reject");

        assert!(matches!(error, Error::Corrupt("gzip", _)));
    }
}
