//! Nitro enclave workload for the dev migration setup.
//!
//! No attestation, no key material, no PCP decryption — the dev setup exists to exercise the
//! round trip and, later, the sandbox. The migration itself is an echo until the biometrics
//! pipeline runs under minijail.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

/// Pontifex operations exposed to the host.
pub mod routes;
/// Pontifex server setup and lifecycle.
pub mod server;
/// Boot-scoped enclave state.
pub mod state;
