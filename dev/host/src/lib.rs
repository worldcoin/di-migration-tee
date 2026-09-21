//! HTTP host for the dev migration setup: the enclave client, the probes, and the two
//! mechanisms a migration needs.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

mod app_state;
#[cfg(test)]
mod test_support;

pub mod compression;
pub mod enclave;
pub mod migrations;
pub mod routes;
pub mod server;

pub use app_state::AppState;
