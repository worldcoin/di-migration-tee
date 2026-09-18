use pontifex::Request;
use serde::{Deserialize, Serialize};

use crate::Error;

/// Maximum PCP the enclave will accept, after decompression.
///
/// Part of the contract rather than an enclave detail: it is the ceiling the host decompresses
/// against, so a compression bomb is stopped on the untrusted side instead of being relayed and
/// rejected. The enclave enforces it again on arrival.
pub const MAX_PCP_BYTES: usize = 64 * 1024 * 1024;

/// Requests the migration of one PCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateRequest {
    /// The decompressed PCP. The host relays it verbatim; it does not parse it.
    pub pcp: bytes::Bytes,
}

impl Request for MigrateRequest {
    const ROUTE_ID: &'static str = "/v1/migrate";
    type Response = Result<MigrateResponse, Error>;
}

/// The migrated PCP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrateResponse {
    /// The migrated PCP. Echoed verbatim until the pipeline runs in the sandbox.
    #[serde(with = "serde_bytes")]
    pub pcp: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use pontifex::Request;

    use super::MigrateRequest;

    #[test]
    fn migrate_route_id_is_versioned_and_stable() {
        assert_eq!(MigrateRequest::ROUTE_ID, "/v1/migrate");
    }
}
