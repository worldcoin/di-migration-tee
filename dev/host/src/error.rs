//! Universal error handling for the API.
//!
//! Every route returns [`AppError`], so status codes, response bodies and logging are decided in
//! one place.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use di_dev_api_types::{ApiErrorResponse, ErrorBody};
use di_dev_enclave_types as enclave_types;

use crate::{compression, enclave, migrations::Failure};

/// An API failure, with the status and body to return for it.
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    allow_retry: bool,
    /// Extra context for logs. Never serialized, since it may name internals.
    detail: Option<String>,
}

impl AppError {
    /// Creates an error with the given status and body.
    #[must_use]
    pub const fn new(
        status: StatusCode,
        code: &'static str,
        message: &'static str,
        allow_retry: bool,
    ) -> Self {
        Self {
            status,
            code,
            message,
            allow_retry,
            detail: None,
        }
    }

    /// Attaches context that is logged but not returned to the client.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// The status this error will return. Exposed for tests and callers that branch on it.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The machine-readable code this error will return.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Maps a payload that did not decompress.
    ///
    /// All client-side: the request said it was compressed and it was not, or it expanded past
    /// what the enclave will hold. Retrying the same bytes changes nothing.
    #[must_use]
    pub fn payload(error: &compression::Error) -> Self {
        match error {
            compression::Error::UnknownFormat => Self::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_compression",
                "The PCP must be gzip or zstd compressed",
                false,
            ),
            compression::Error::TooLarge { limit } => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "pcp_too_large",
                "The PCP expanded past the size limit",
                false,
            )
            .with_detail(format!("limit {limit} bytes")),
            compression::Error::Corrupt(format, detail) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_pcp",
                "The compressed PCP could not be read",
                false,
            )
            .with_detail(format!("{format}: {detail}")),
        }
    }

    /// Maps the recorded failure of a migration onto its collection response.
    #[must_use]
    pub fn migration_failure(failure: &Failure) -> Self {
        match failure {
            Failure::EnclaveTimeout => Self::new(
                StatusCode::GATEWAY_TIMEOUT,
                "enclave_timeout",
                "The enclave did not finish the migration in time",
                true,
            ),
            Failure::EnclaveUnreachable(detail) => Self::new(
                StatusCode::BAD_GATEWAY,
                "enclave_unreachable",
                "The enclave was unreachable",
                true,
            )
            .with_detail(detail.clone()),
            Failure::EnclaveRejected(operation) => Self::enclave_operation(*operation),
            Failure::Abandoned => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration_abandoned",
                "The migration stopped without producing a result",
                true,
            ),
        }
    }

    fn enclave_operation(operation: enclave_types::Error) -> Self {
        match operation {
            enclave_types::Error::NotReady => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "enclave_not_ready",
                "The enclave is not ready",
                true,
            ),
            // The host checks both before relaying, so the enclave disagreeing means the two
            // sides are built against different limits — a deploy fault, not a client one.
            enclave_types::Error::EmptyPcp | enclave_types::Error::PcpTooLarge => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
                false,
            )
            .with_detail(format!(
                "enclave rejected a payload the host accepted: {operation:?}"
            )),
            enclave_types::Error::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
                true,
            ),
        }
        .with_detail(format!("{operation:?}"))
    }
}

impl From<&enclave::Error> for Failure {
    fn from(error: &enclave::Error) -> Self {
        match error {
            enclave::Error::Timeout => Self::EnclaveTimeout,
            enclave::Error::Transport(detail) => Self::EnclaveUnreachable(detail.clone()),
            enclave::Error::Operation(operation) => Self::EnclaveRejected(*operation),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!(
                code = self.code,
                status = %self.status,
                detail = self.detail.as_deref().unwrap_or_default(),
                dependency = "enclave",
                "request failed"
            );
        } else {
            tracing::warn!(
                code = self.code,
                status = %self.status,
                detail = self.detail.as_deref().unwrap_or_default(),
                "request rejected"
            );
        }

        (
            self.status,
            Json(ApiErrorResponse {
                allow_retry: self.allow_retry,
                error: ErrorBody {
                    code: self.code.to_owned(),
                    message: self.message.to_owned(),
                },
            }),
        )
            .into_response()
    }
}
