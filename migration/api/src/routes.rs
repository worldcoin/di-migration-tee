use axum::{Router, http::StatusCode, routing::get};

pub fn router() -> Router {
    Router::new()
        .route("/healtz", get(health))
        .route("/readyz", get(ready))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn ready() -> StatusCode {
    StatusCode::OK
}
