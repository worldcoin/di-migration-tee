use std::sync::Arc;

use di_dev_enclave::{server, state::EnclaveState};
use tracing_subscriber::EnvFilter;

/// vsock port the host dials; fixed because both sides ship together.
const PONTIFEX_PORT: u32 = 1000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state = Arc::new(EnclaveState);

    tracing::info!(port = PONTIFEX_PORT, "starting enclave Pontifex server");

    // Err exits non-zero so the carrier restarts the enclave rather than idling without a server.
    server::start(state, PONTIFEX_PORT)
        .await
        .inspect_err(|error| {
            tracing::error!(%error, "enclave Pontifex server stopped");
        })
}
