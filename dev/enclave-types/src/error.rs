use serde::{Deserialize, Serialize};

/// Errors returned by enclave operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    /// The enclave is reachable but not ready to process requests.
    NotReady,
    /// The request carried no PCP.
    EmptyPcp,
    /// The PCP exceeded the enclave's own limit.
    ///
    /// The host bounds this too, but the host is the untrusted side of the boundary, so the
    /// enclave does not take its word for the size of what it is asked to hold in memory.
    PcpTooLarge,
    /// The enclave failed while producing a response. Detail stays in the enclave log.
    Internal,
}
