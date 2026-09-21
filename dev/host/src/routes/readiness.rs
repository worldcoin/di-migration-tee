use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// Readiness: the host takes traffic only once the enclave answers.
pub async fn handler(State(state): State<AppState>) -> StatusCode {
    match state.enclave_client().health().await {
        Ok(()) => StatusCode::OK,
        Err(error) => {
            tracing::warn!(?error, dependency = "enclave", "readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
