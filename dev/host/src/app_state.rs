use std::sync::Arc;

use crate::enclave::EnclaveClient;

/// Dependencies shared by API request handlers.
#[derive(Clone)]
pub struct AppState {
    enclave_client: Arc<dyn EnclaveClient>,
}

impl AppState {
    /// Creates API state from the enclave client.
    #[must_use]
    pub fn new(enclave_client: Arc<dyn EnclaveClient>) -> Self {
        Self { enclave_client }
    }

    /// Returns a shared enclave client.
    #[must_use]
    pub fn enclave_client(&self) -> Arc<dyn EnclaveClient> {
        Arc::clone(&self.enclave_client)
    }
}
