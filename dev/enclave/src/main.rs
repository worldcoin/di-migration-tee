//! Nitro enclave workload for the dev migration setup.
//!
//! Skeleton. It will run the biometrics pipeline under a minijail sandbox, fed a tarball
//! the host injects over vsock.

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

    // Non-zero rather than idling: a skeleton that stays up reads as healthy.
    tracing::error!("di-dev-enclave is a skeleton and has no boot sequence yet");
    ExitCode::FAILURE
}
