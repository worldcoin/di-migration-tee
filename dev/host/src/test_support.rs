//! Fakes shared by the route tests.

use std::sync::Arc;

use async_trait::async_trait;
use di_dev_enclave_types::{MigrateRequest, MigrateResponse};

use crate::{
    AppState,
    enclave::{EnclaveClient, Error},
};

/// Echoes the PCP straight back.
pub struct EchoEnclave;

#[async_trait]
impl EnclaveClient for EchoEnclave {
    async fn health(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn migrate(&self, request: MigrateRequest) -> Result<MigrateResponse, Error> {
        Ok(MigrateResponse {
            pcp: request.pcp.to_vec(),
        })
    }
}

/// Fails every call with a fixed error.
pub struct FailingEnclave(pub Error);

#[async_trait]
impl EnclaveClient for FailingEnclave {
    async fn health(&self) -> Result<(), Error> {
        Err(self.0.clone())
    }

    async fn migrate(&self, _: MigrateRequest) -> Result<MigrateResponse, Error> {
        Err(self.0.clone())
    }
}

/// Builds state around `client`.
pub fn state_with(client: Arc<dyn EnclaveClient>) -> AppState {
    AppState::new(client)
}
