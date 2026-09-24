mod config;
mod db;
mod routes;
mod sqs;

use std::time::Duration;

use anyhow::Context;
use aws_config::BehaviorVersion;
use telemetry_batteries::tracing::middleware::TraceLayer;
use tokio::net::TcpListener;

use crate::{db::Db, sqs::Sqs};

#[derive(Clone)]
struct AppState {
    db: Db,
    sqs: Sqs,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = telemetry_batteries::init()
        .map_err(|error| anyhow::anyhow!("failed to initialize telemetry: {error:?}"))?;
    let config = config::Config::from_env()?;
    let aws_config = tokio::time::timeout(
        Duration::from_secs(5),
        aws_config::load_defaults(BehaviorVersion::latest()),
    )
    .await
    .context("timed out loading AWS configuration")?;
    anyhow::ensure!(
        aws_config.region().is_some(),
        "AWS region is not configured"
    );
    let db = Db::new(
        aws_sdk_dynamodb::Client::new(&aws_config),
        config.dynamodb_table_name,
    );
    let sqs = Sqs::new(aws_sdk_sqs::Client::new(&aws_config), config.sqs_queue_url);

    let listener = TcpListener::bind(config.http_addr)
        .await
        .with_context(|| format!("failed to bind HTTP server to {}", config.http_addr))?;

    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");
    axum::serve(
        listener,
        routes::router(AppState { db, sqs }).layer(TraceLayer::new_for_axum()),
    )
    .await
    .context("HTTP server failed")
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        routing::post,
    };
    use tower::ServiceExt;

    use crate::{AppState, db::Db, routes, sqs::Sqs};

    fn unavailable_db() -> Db {
        let config = aws_sdk_dynamodb::Config::builder()
            .region(aws_sdk_dynamodb::config::Region::new("us-east-1"))
            .behavior_version(aws_config::BehaviorVersion::latest())
            .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .endpoint_url("http://127.0.0.1:9")
            .retry_config(aws_sdk_dynamodb::config::retry::RetryConfig::disabled())
            .build();
        Db::new(
            aws_sdk_dynamodb::Client::from_conf(config),
            "test-table".to_owned(),
        )
    }

    fn unavailable_sqs() -> Sqs {
        let config = aws_sdk_sqs::Config::builder()
            .region(aws_sdk_sqs::config::Region::new("us-east-1"))
            .behavior_version(aws_config::BehaviorVersion::latest())
            .credentials_provider(aws_sdk_sqs::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .endpoint_url("http://127.0.0.1:9")
            .retry_config(aws_sdk_sqs::config::retry::RetryConfig::disabled())
            .build();
        Sqs::new(
            aws_sdk_sqs::Client::from_conf(config),
            "http://127.0.0.1:9/000000000000/test-queue".to_owned(),
        )
    }

    #[tokio::test]
    async fn probes_when_dependencies_are_unavailable() {
        for (path, expected) in [
            ("/healtz", StatusCode::OK),
            ("/readyz", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let response = routes::router(AppState {
                db: unavailable_db(),
                sqs: unavailable_sqs(),
            })
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
            assert_eq!(response.status(), expected, "{path}");
        }
    }

    #[tokio::test]
    async fn readiness_fails_when_only_sqs_is_unavailable() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/",
                    post(|| async {
                        (
                            [(header::CONTENT_TYPE, "application/x-amz-json-1.0")],
                            r#"{"Table":{"TableName":"test-table","TableStatus":"ACTIVE"}}"#,
                        )
                    }),
                ),
            )
            .await
            .unwrap();
        });

        let db_config = aws_sdk_dynamodb::Config::builder()
            .region(aws_sdk_dynamodb::config::Region::new("us-east-1"))
            .behavior_version(aws_config::BehaviorVersion::latest())
            .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .endpoint_url(format!("http://{address}"))
            .retry_config(aws_sdk_dynamodb::config::retry::RetryConfig::disabled())
            .build();
        let state = AppState {
            db: Db::new(
                aws_sdk_dynamodb::Client::from_conf(db_config),
                "test-table".to_owned(),
            ),
            sqs: unavailable_sqs(),
        };
        assert!(state.db.check_ready().await.is_ok());

        let response = routes::router(state)
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        server.abort();
    }
}
