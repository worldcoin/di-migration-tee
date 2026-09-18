//! The HTTP contract between the dev client and its host.
//!
//! One definition per message, so the two ends cannot drift apart. Host-side only: the contract
//! stops at the host, so no enclave links it.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

mod error;
mod migrations;

pub use error::{ApiErrorResponse, ErrorBody};
pub use migrations::*;
