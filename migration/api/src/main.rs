mod config;
mod routes;

use anyhow::Context;
use telemetry_batteries::tracing::middleware::TraceLayer;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = telemetry_batteries::init()
        .map_err(|error| anyhow::anyhow!("failed to initialize telemetry: {error:?}"))?;
    let config = config::Config::from_env()?;

    let listener = TcpListener::bind(config.http_addr)
        .await
        .with_context(|| format!("failed to bind HTTP server to {}", config.http_addr))?;

    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");
    axum::serve(listener, routes::router().layer(TraceLayer::new_for_axum()))
        .await
        .context("HTTP server failed")
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::routes;

    #[tokio::test]
    async fn probes_are_available() {
        for path in ["/healtz", "/readyz"] {
            let response = routes::router()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
    }
}
