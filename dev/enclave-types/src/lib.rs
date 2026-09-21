//! The vsock contract between the dev host and its enclave. Shares nothing with the
//! `migration` workload: a shared type would rotate its enclave's PCR0.

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    dead_code
)]

mod error;
mod health;
mod migrate;

pub use error::Error;
pub use health::HealthRequest;
pub use migrate::{MigrateRequest, MigrateResponse};
