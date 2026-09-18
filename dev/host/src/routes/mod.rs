//! HTTP route definitions.

mod health;
mod migrations;
mod readiness;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

use crate::AppState;

/// Builds the router with all API routes.
///
/// The body limit hangs off the submit route alone, not the router: the health routes and the
/// collection route send no body, so allowing multi-megabyte requests there would widen the
/// service's ingress for nothing.
pub fn handler(max_request_bytes: usize) -> Router<AppState> {
    Router::new()
        .route("/health", get(health::handler))
        .route("/ready", get(readiness::handler))
        .route(
            "/v1/migrations",
            post(migrations::submit).layer(DefaultBodyLimit::max(max_request_bytes)),
        )
        .route("/v1/migrations/{id}", get(migrations::collect))
}
