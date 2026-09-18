use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// Liveness: the process is up. Deliberately says nothing about the enclave — restarting the
/// host does not fix an enclave that is down.
pub async fn handler(State(_state): State<AppState>) -> StatusCode {
    StatusCode::OK
}
