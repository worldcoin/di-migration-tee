//! Boot-scoped state owned by the enclave.

/// Settings fixed for the life of the enclave; the pipeline handle lands here.
#[derive(Debug, Clone, Copy)]
pub struct EnclaveState;
