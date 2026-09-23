use axum::{Router, extract::State, http::StatusCode, routing::get};

use crate::db::Db;

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/healtz", get(health))
        .route("/readyz", get(ready))
        .with_state(db)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn ready(State(db): State<Db>) -> StatusCode {
    match db.check_ready().await {
        Ok(()) => StatusCode::OK,
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), dependency = "dynamodb", "readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
