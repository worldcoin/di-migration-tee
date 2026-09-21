use axum::{
    Json,
    body::Bytes,
    extract::{Path, State, rejection::BytesRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use di_dev_api_types::{MAX_PCP_BYTES, MIGRATION_CONTENT_TYPE, MigrationAccepted};
use di_dev_enclave_types::MigrateRequest;
use uuid::Uuid;

use crate::{
    AppState, compression,
    error::ApiError,
    migrations::{Failure, State as MigrationState},
};

/// Accepts a compressed PCP and starts a migration. Decompression happens here, not in the
/// task, so a malformed payload is a straight rejection.
pub async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<Response, ApiError> {
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some(MIGRATION_CONTENT_TYPE)
    {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "Expected application/octet-stream",
            false,
        ));
    }

    let body = body.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request_too_large",
                "The compressed PCP exceeded the body limit",
                false,
            )
        } else {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Could not read the request body",
                false,
            )
            .with_detail(error.to_string())
        }
    })?;

    if body.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "The request carried no PCP",
            false,
        ));
    }

    // CPU-bound and possibly tens of megabytes; on a runtime worker it would stall the host.
    let pcp = tokio::task::spawn_blocking(move || compression::decompress(&body, MAX_PCP_BYTES))
        .await
        .map_err(|error| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
                true,
            )
            .with_detail(format!("decompression task failed: {error}"))
        })?
        .map_err(|error| ApiError::payload(&error))?;

    // Claimed after decompression so a bad payload never occupies the slot.
    let Some(slot) = state.migrations().start() else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "migration_in_progress",
            "A migration is already running; retry once it finishes",
            true,
        ));
    };

    let id = slot.id();
    let enclave_client = state.enclave_client();
    let bytes = pcp.len();

    // The slot guard rides along, so the slot is released however this task ends.
    tokio::spawn(async move {
        match enclave_client
            .migrate(MigrateRequest { pcp: pcp.into() })
            .await
        {
            Ok(response) => {
                tracing::info!(migration_id = %id, bytes = response.pcp.len(), "migration finished");
                slot.succeed(response.pcp.into());
            }
            Err(error) => {
                let failure = Failure::from(&error);
                tracing::error!(
                    migration_id = %id,
                    ?failure,
                    dependency = "enclave",
                    "migration failed"
                );
                slot.fail(failure);
            }
        }
    });

    tracing::info!(migration_id = %id, bytes, "migration started");

    Ok((StatusCode::ACCEPTED, Json(MigrationAccepted { id })).into_response())
}

/// Collects a migration's result.
pub async fn collect(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let Some(migration) = state.migrations().state(id) else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_migration",
            "No such migration; it never started or its result has aged out",
            false,
        ));
    };

    match migration {
        MigrationState::Running => Ok(StatusCode::ACCEPTED.into_response()),
        MigrationState::Succeeded(pcp) => Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, MIGRATION_CONTENT_TYPE),
                (header::CACHE_CONTROL, "no-store"),
            ],
            pcp,
        )
            .into_response()),
        MigrationState::Failed(failure) => Err(ApiError::migration_failure(&failure)),
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use di_dev_api_types::{
        ErrorEnvelope, MAX_PCP_BYTES, MIGRATION_CONTENT_TYPE, MigrationAccepted,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{
        AppState, enclave, routes,
        test_support::{EchoEnclave, FailingEnclave, GatedEnclave, state_with},
    };

    const PCP: &[u8] = b"a perfectly ordinary pcp";

    fn gzip(payload: &[u8]) -> Vec<u8> {
        use std::io::Write;

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).expect("should encode");
        encoder.finish().expect("should finish")
    }

    async fn send(state: &AppState, request: Request<Body>) -> (StatusCode, bytes::Bytes) {
        let response = routes::handler(1024 * 1024)
            .with_state(state.clone())
            .oneshot(request)
            .await
            .expect("router should answer");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body should read")
            .to_bytes();
        (status, body)
    }

    fn submission(body: Vec<u8>) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/v1/migrations")
            .header(header::CONTENT_TYPE, MIGRATION_CONTENT_TYPE)
            .body(Body::from(body))
            .expect("request should build")
    }

    fn collection(id: Uuid) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/v1/migrations/{id}"))
            .body(Body::empty())
            .expect("request should build")
    }

    async fn submit_ok(state: &AppState, body: Vec<u8>) -> Uuid {
        let (status, body) = send(state, submission(body)).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        serde_json::from_slice::<MigrationAccepted>(&body)
            .expect("should be an acceptance")
            .id
    }

    /// Polls rather than sleeping, so the test does not depend on task scheduling.
    async fn settle(state: &AppState, id: Uuid) -> (StatusCode, bytes::Bytes) {
        for _ in 0..500 {
            let (status, body) = send(state, collection(id)).await;
            if status != StatusCode::ACCEPTED {
                return (status, body);
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        panic!("migration never settled");
    }

    #[tokio::test]
    async fn a_submitted_pcp_comes_back_decompressed() {
        let state = state_with(Arc::new(EchoEnclave));

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, body) = settle(&state, id).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), PCP);
    }

    #[tokio::test]
    async fn a_second_migration_is_refused_while_one_runs() {
        let (enclave, gate) = GatedEnclave::new();
        let state = state_with(Arc::new(enclave));

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, _) = send(&state, submission(gzip(PCP))).await;

        assert_eq!(status, StatusCode::CONFLICT);

        gate.notify_waiters();
        let (status, _) = settle(&state, id).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_running_migration_reports_accepted() {
        let (enclave, gate) = GatedEnclave::new();
        let state = state_with(Arc::new(enclave));

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, _) = send(&state, collection(id)).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        gate.notify_waiters();
    }

    #[tokio::test]
    async fn an_unknown_migration_is_not_found() {
        let state = state_with(Arc::new(EchoEnclave));

        let (status, _) = send(&state, collection(Uuid::new_v4())).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// A bad payload must not occupy the single slot — it is rejected before the claim.
    #[tokio::test]
    async fn an_uncompressed_payload_is_rejected_without_taking_the_slot() {
        let state = state_with(Arc::new(EchoEnclave));

        let (status, _) = send(&state, submission(PCP.to_vec())).await;
        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, _) = settle(&state, id).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_request_without_the_binary_content_type_is_rejected() {
        let state = state_with(Arc::new(EchoEnclave));
        let request = Request::builder()
            .method("POST")
            .uri("/v1/migrations")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(gzip(PCP)))
            .expect("request should build");

        let (status, _) = send(&state, request).await;

        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn an_empty_body_is_rejected() {
        let state = state_with(Arc::new(EchoEnclave));

        let (status, _) = send(&state, submission(Vec::new())).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn an_unreachable_enclave_surfaces_on_collection() {
        let state = state_with(Arc::new(FailingEnclave(enclave::Error::Transport(
            "connection refused".to_owned(),
        ))));

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, _) = settle(&state, id).await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn an_enclave_timeout_surfaces_on_collection() {
        let state = state_with(Arc::new(FailingEnclave(enclave::Error::Timeout)));

        let id = submit_ok(&state, gzip(PCP)).await;
        let (status, _) = settle(&state, id).await;

        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    }

    /// The body limit and the decompression ceiling are the same number, and both answer 413,
    /// so the code is what proves decompression rejected this rather than the body limit.
    #[tokio::test]
    async fn a_compression_bomb_is_rejected_by_the_decompression_ceiling() {
        let state = state_with(Arc::new(EchoEnclave));
        let bomb = gzip(&vec![0u8; MAX_PCP_BYTES + 1]);
        assert!(
            bomb.len() < MAX_PCP_BYTES,
            "the bomb must clear the body limit"
        );

        let (status, body) = send(&state, submission(bomb)).await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        let envelope: ErrorEnvelope = serde_json::from_slice(&body).expect("error envelope");
        assert_eq!(envelope.error.code, "pcp_too_large");
    }
}
