use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// Liveness only: restarting the host would not fix a dead enclave, so this ignores it.
pub async fn handler(State(_state): State<AppState>) -> StatusCode {
    StatusCode::OK
}
