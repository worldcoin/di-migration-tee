//! The vsock contract between the dev host and its enclave.
//!
//! The client↔host HTTP contract is `di-dev-api-types`. Nothing here is shared with the
//! `migration` workload: a shared type would rotate its enclave's PCR0 on a dev-only edit.

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
pub use migrate::{MAX_PCP_BYTES, MigrateRequest, MigrateResponse};
