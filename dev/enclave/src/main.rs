use std::sync::Arc;

use di_dev_enclave::{server, state::EnclaveState};
use tracing_subscriber::EnvFilter;

/// vsock port the host dials. Fixed rather than configured: the host is the only caller and
/// both sides ship together.
const PONTIFEX_PORT: u32 = 1000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state = Arc::new(EnclaveState::default());

    tracing::info!(
        port = PONTIFEX_PORT,
        max_pcp_bytes = state.max_pcp_bytes(),
        "starting enclave Pontifex server"
    );

    // Returning Err exits non-zero, which restarts the carrier and so the enclave. An enclave
    // that stayed up with a dead server would keep passing its readiness check from the host's
    // point of view only until the next request.
    server::start(state, PONTIFEX_PORT)
        .await
        .inspect_err(|error| {
            tracing::error!(%error, "enclave Pontifex server stopped");
        })
}
