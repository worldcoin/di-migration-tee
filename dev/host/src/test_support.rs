//! Fakes shared by the route tests.

use std::sync::Arc;

use async_trait::async_trait;
use di_dev_enclave_types::{MigrateRequest, MigrateResponse};
use tokio::sync::Notify;

use crate::{
    AppState, Environment,
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

/// Fails every migration with a fixed error.
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

/// Holds every migration open until released, so a test can observe one in flight.
pub struct GatedEnclave {
    gate: Arc<Notify>,
}

impl GatedEnclave {
    pub fn new() -> (Self, Arc<Notify>) {
        let gate = Arc::new(Notify::new());
        (
            Self {
                gate: Arc::clone(&gate),
            },
            gate,
        )
    }
}

#[async_trait]
impl EnclaveClient for GatedEnclave {
    async fn health(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn migrate(&self, request: MigrateRequest) -> Result<MigrateResponse, Error> {
        self.gate.notified().await;
        Ok(MigrateResponse {
            pcp: request.pcp.to_vec(),
        })
    }
}

/// Builds state around `client`. The environment values are the ones routes read; nothing here
/// touches the process environment.
pub fn state_with(client: Arc<dyn EnclaveClient>) -> AppState {
    AppState::new(
        Environment::for_tests(1, 1000, 1024 * 1024, 1024 * 1024),
        client,
    )
}
