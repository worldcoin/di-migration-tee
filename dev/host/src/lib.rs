//! HTTP host for the dev migration setup — the untrusted side of the enclave boundary.
//!
//! Takes a compressed PCP inline, decompresses it under a bound, relays it to the enclave and
//! holds the result for collection. One migration at a time; a second is refused rather than
//! queued.

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

pub mod compression;
pub mod enclave;
pub mod error;
pub mod migrations;
pub mod routes;
pub mod server;

pub use app_state::AppState;
pub use environment::Environment;
