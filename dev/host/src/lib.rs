//! HTTP host for the dev migration setup: configuration, the enclave client and the probes.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

mod app_state;
mod environment;
#[cfg(test)]
mod test_support;

pub mod enclave;
pub mod routes;
pub mod server;

pub use app_state::AppState;
pub use environment::Environment;
