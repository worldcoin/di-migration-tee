use axum::{Router, extract::State, http::StatusCode, routing::get};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healtz", get(health))
        .route("/readyz", get(ready))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn ready(State(state): State<AppState>) -> StatusCode {
    let (db_result, sqs_result) = tokio::join!(state.db.check_ready(), state.sqs.check_ready());

    if let Err(error) = &db_result {
        tracing::warn!(error = %format!("{error:#}"), dependency = "dynamodb", "readiness check failed");
    }
    if let Err(error) = &sqs_result {
        tracing::warn!(error = %format!("{error:#}"), dependency = "sqs", "readiness check failed");
    }
    if db_result.is_ok() && sqs_result.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
