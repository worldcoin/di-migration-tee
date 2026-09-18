//! Boot-scoped state owned by the enclave.

use di_dev_enclave_types::MAX_PCP_BYTES;

/// Limits and settings fixed for the life of the enclave.
#[derive(Debug, Clone, Copy)]
pub struct EnclaveState {
    max_pcp_bytes: usize,
}

impl EnclaveState {
    /// Creates state with an explicit PCP ceiling.
    #[must_use]
    pub const fn new(max_pcp_bytes: usize) -> Self {
        Self { max_pcp_bytes }
    }

    /// The largest PCP this enclave will hold in memory.
    #[must_use]
    pub const fn max_pcp_bytes(&self) -> usize {
        self.max_pcp_bytes
    }
}

impl Default for EnclaveState {
    fn default() -> Self {
        Self::new(MAX_PCP_BYTES)
    }
}
