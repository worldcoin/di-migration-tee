//! HTTP host for the dev migration setup: takes a compressed PCP inline, relays it to the
//! enclave and returns the result. One migration at a time.

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
pub mod error;
pub mod routes;
pub mod server;

pub use app_state::AppState;
