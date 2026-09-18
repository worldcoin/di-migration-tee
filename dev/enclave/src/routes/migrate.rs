use std::sync::Arc;

use di_dev_enclave_types::{self as enclave_types, MigrateRequest, MigrateResponse};

use crate::state::EnclaveState;

/// Echoes the PCP back.
///
/// The biometrics pipeline that replaces this echo will run under minijail, fed a tarball the
/// host injects. Until then what is being exercised is the round trip itself.
pub async fn handler(
    state: Arc<EnclaveState>,
    request: MigrateRequest,
) -> Result<MigrateResponse, enclave_types::Error> {
    let bytes = request.pcp.len();

    if bytes == 0 {
        tracing::warn!("migrate request carried no PCP");
        return Err(enclave_types::Error::EmptyPcp);
    }

    // The host bounds this as well, but it is the untrusted side of the boundary.
    if bytes > state.max_pcp_bytes() {
        tracing::warn!(
            bytes,
            limit = state.max_pcp_bytes(),
            "migrate request exceeded the PCP limit"
        );
        return Err(enclave_types::Error::PcpTooLarge);
    }

    tracing::info!(bytes, "echoing PCP");
    Ok(MigrateResponse {
        pcp: request.pcp.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use di_dev_enclave_types::{self as enclave_types, MigrateRequest};

    use super::handler;
    use crate::state::EnclaveState;

    fn request(bytes: usize) -> MigrateRequest {
        MigrateRequest {
            pcp: vec![7u8; bytes].into(),
        }
    }

    #[tokio::test]
    async fn a_pcp_comes_back_unchanged() {
        let state = Arc::new(EnclaveState::default());

        let response = handler(state, request(32)).await.expect("should echo");

        assert_eq!(response.pcp, vec![7u8; 32]);
    }

    #[tokio::test]
    async fn an_empty_pcp_is_rejected() {
        let state = Arc::new(EnclaveState::default());

        let error = handler(state, request(0)).await.expect_err("should reject");

        assert_eq!(error, enclave_types::Error::EmptyPcp);
    }

    /// The enclave does not take the host's word for how much it is asked to hold.
    #[tokio::test]
    async fn a_pcp_past_the_limit_is_rejected() {
        let state = Arc::new(EnclaveState::new(16));

        let error = handler(state, request(17))
            .await
            .expect_err("should reject");

        assert_eq!(error, enclave_types::Error::PcpTooLarge);
    }
}
