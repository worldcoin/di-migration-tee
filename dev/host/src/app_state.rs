use std::sync::Arc;

use crate::{enclave::EnclaveClient, migrations::Store};

/// Dependencies shared by API request handlers.
#[derive(Clone)]
pub struct AppState {
    enclave_client: Arc<dyn EnclaveClient>,
    migrations: Arc<Store>,
}

impl AppState {
    /// Creates API state from the enclave client.
    #[must_use]
    pub fn new(enclave_client: Arc<dyn EnclaveClient>) -> Self {
        Self {
            enclave_client,
            migrations: Arc::new(Store::new()),
        }
    }

    /// Returns a shared enclave client.
    #[must_use]
    pub fn enclave_client(&self) -> Arc<dyn EnclaveClient> {
        Arc::clone(&self.enclave_client)
    }

    /// Returns the migration slot and recent results.
    #[must_use]
    pub fn migrations(&self) -> Arc<Store> {
        Arc::clone(&self.migrations)
    }
}
