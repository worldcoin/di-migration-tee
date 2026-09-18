//! Runtime environment configuration.
//!
//! Everything is read once at boot and panics when missing or malformed. A host that starts
//! without knowing which enclave to dial would pass its liveness probe and fail every request.

use std::env;

use di_dev_api_types::MAX_REQUEST_BYTES;
use di_dev_enclave_types::MAX_PCP_BYTES;

/// Configuration the host resolves at boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Environment {
    enclave_cid: u32,
    enclave_port: u32,
    max_request_bytes: usize,
    max_pcp_bytes: usize,
}

impl Environment {
    /// Reads the configuration from the process environment.
    ///
    /// # Panics
    ///
    /// Panics when `ENCLAVE_CID` or `ENCLAVE_PORT` is unset or is not a valid `u32`, or when
    /// `MAX_REQUEST_BYTES` or `MAX_PCP_BYTES` is set to something that is not a `usize`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            enclave_cid: Self::required_u32("ENCLAVE_CID"),
            enclave_port: Self::required_u32("ENCLAVE_PORT"),
            max_request_bytes: Self::optional_usize("MAX_REQUEST_BYTES", MAX_REQUEST_BYTES),
            max_pcp_bytes: Self::optional_usize("MAX_PCP_BYTES", MAX_PCP_BYTES),
        }
    }

    /// The Nitro enclave CID to dial.
    #[must_use]
    pub const fn enclave_cid(&self) -> u32 {
        self.enclave_cid
    }

    /// The enclave's Pontifex vsock port.
    #[must_use]
    pub const fn enclave_port(&self) -> u32 {
        self.enclave_port
    }

    /// The largest compressed request body the host accepts.
    #[must_use]
    pub const fn max_request_bytes(&self) -> usize {
        self.max_request_bytes
    }

    /// The largest PCP the host will decompress to.
    #[must_use]
    pub const fn max_pcp_bytes(&self) -> usize {
        self.max_pcp_bytes
    }

    /// Builds a configuration directly, for tests that must not read the process environment.
    #[cfg(test)]
    pub(crate) const fn for_tests(
        enclave_cid: u32,
        enclave_port: u32,
        max_request_bytes: usize,
        max_pcp_bytes: usize,
    ) -> Self {
        Self {
            enclave_cid,
            enclave_port,
            max_request_bytes,
            max_pcp_bytes,
        }
    }

    fn required_u32(name: &str) -> u32 {
        env::var(name)
            .unwrap_or_else(|_| panic!("{name} environment variable is not set"))
            .parse()
            .unwrap_or_else(|_| panic!("{name} environment variable is not a valid u32"))
    }

    fn optional_usize(name: &str, default: usize) -> usize {
        env::var(name).map_or(default, |value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("{name} environment variable is not a valid usize"))
        })
    }
}
