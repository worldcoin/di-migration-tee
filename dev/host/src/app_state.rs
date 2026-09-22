use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::enclave::EnclaveClient;

/// Dependencies shared by API request handlers.
#[derive(Clone)]
pub struct AppState {
    enclave_client: Arc<dyn EnclaveClient>,
    migration: Arc<Semaphore>,
}

impl AppState {
    /// Creates API state from the enclave client.
    #[must_use]
    pub fn new(enclave_client: Arc<dyn EnclaveClient>) -> Self {
        Self {
            enclave_client,
            migration: Arc::new(Semaphore::new(1)),
        }
    }

    /// Returns a shared enclave client.
    #[must_use]
    pub fn enclave_client(&self) -> Arc<dyn EnclaveClient> {
        Arc::clone(&self.enclave_client)
    }

    /// The single migration slot; one enclave runs one pipeline at a time.
    #[must_use]
    pub fn migration(&self) -> Arc<Semaphore> {
        Arc::clone(&self.migration)
    }
}
