//! CLI against the dev host: upload a PCP, run a migration, fetch the result.
//!
//! Skeleton. Dev-only — it speaks the dev host's simplified API and verifies no attestation.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Non-zero rather than exiting clean: a skeleton that succeeds reads as a no-op run.
    tracing::error!("di-dev-cli is a skeleton and implements no commands yet");
    ExitCode::FAILURE
}
