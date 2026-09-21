//! Every route returns [`ApiError`], so status, body and logging are decided in one place.
//! Constructors are per route rather than a blanket `From`: the mapping is context-dependent.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use di_dev_api_types::{ErrorBody, ErrorEnvelope};
use di_dev_enclave_types as enclave_types;

use crate::{compression, migrations::Failure};

/// An API failure, with the status and body to return for it.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    allow_retry: bool,
    /// Log-only context; never serialized, since it may name internals.
    detail: Option<String>,
}

impl ApiError {
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

    /// The status this error will return.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The machine-readable code this error will return.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Whether the client is told to retry. Exposed for tests.
    #[must_use]
    pub const fn allow_retry(&self) -> bool {
        self.allow_retry
    }

    /// Maps a payload that did not decompress; all client-side, so none of them retry.
    #[must_use]
    pub fn payload(error: &compression::Error) -> Self {
        match error {
            compression::Error::NotGzip => Self::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_compression",
                "The PCP must be gzip compressed",
                false,
            ),
            compression::Error::TooLarge { limit } => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "pcp_too_large",
                "The PCP expanded past the size limit",
                false,
            )
            .with_detail(format!("limit {limit} bytes")),
            compression::Error::Corrupt(detail) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_pcp",
                "The compressed PCP could not be read",
                false,
            )
            .with_detail(detail.clone()),
        }
    }

    /// Maps a migration's recorded failure, on the collection route.
    #[must_use]
    pub fn migration_failure(failure: &Failure) -> Self {
        match failure {
            Failure::EnclaveTimeout | Failure::EnclaveUnreachable(_) => {
                Self::enclave_unreachable(failure)
            }
            Failure::EnclaveRejected(operation) => Self::enclave_rejected(*operation),
            // The slot guard fired; the client just resubmits.
            Failure::Abandoned => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration_abandoned",
                "The migration stopped without producing a result",
                true,
            ),
        }
    }

    /// The migration never reached a working enclave.
    fn enclave_unreachable(failure: &Failure) -> Self {
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
            Failure::EnclaveRejected(_) | Failure::Abandoned => {
                unreachable!("caller matched a transport failure")
            }
        }
    }

    /// The enclave answered, with an error.
    fn enclave_rejected(operation: enclave_types::Error) -> Self {
        match operation {
            // The host rejects empty bodies first, so this means mismatched deploys.
            enclave_types::Error::EmptyPcp => Self::new(
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
            )
            .with_detail(format!("{operation:?}")),
        }
    }
}

impl IntoResponse for ApiError {
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

        // The envelope owns its strings, so the `&'static str`s are copied here.
        let body = ErrorEnvelope {
            allow_retry: self.allow_retry,
            error: ErrorBody {
                code: self.code.to_owned(),
                message: self.message.to_owned(),
            },
        };

        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use di_dev_enclave_types as enclave_types;

    use super::ApiError;
    use crate::{compression, migrations::Failure};

    /// Pins the payload matrix; a retry loop on unchangeable bytes is what this guards.
    #[test]
    fn every_payload_failure_is_a_client_error_and_not_retryable() {
        let cases = [
            (
                compression::Error::NotGzip,
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_compression",
            ),
            (
                compression::Error::TooLarge { limit: 64 },
                StatusCode::PAYLOAD_TOO_LARGE,
                "pcp_too_large",
            ),
            (
                compression::Error::Corrupt("truncated".to_owned()),
                StatusCode::BAD_REQUEST,
                "invalid_pcp",
            ),
        ];

        for (error, status, code) in cases {
            let mapped = ApiError::payload(&error);

            assert_eq!(mapped.status(), status, "status for {code}");
            assert_eq!(mapped.code(), code);
            assert!(mapped.status().is_client_error(), "{code} should be 4xx");
            assert!(!mapped.allow_retry(), "{code} should not be retryable");
        }
    }

    /// Pins the failure matrix; nothing else fails if one arm is changed alone.
    #[test]
    fn each_migration_failure_maps_to_its_own_status() {
        let cases = [
            (
                Failure::EnclaveTimeout,
                StatusCode::GATEWAY_TIMEOUT,
                "enclave_timeout",
                true,
            ),
            (
                Failure::EnclaveUnreachable("boom".to_owned()),
                StatusCode::BAD_GATEWAY,
                "enclave_unreachable",
                true,
            ),
            (
                Failure::EnclaveRejected(enclave_types::Error::Internal),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                true,
            ),
            (
                Failure::Abandoned,
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration_abandoned",
                true,
            ),
        ];

        for (failure, status, code, allow_retry) in cases {
            let mapped = ApiError::migration_failure(&failure);

            assert_eq!(mapped.status(), status, "status for {failure:?}");
            assert_eq!(mapped.code(), code, "code for {failure:?}");
            assert_eq!(
                mapped.allow_retry(),
                allow_retry,
                "retryability for {failure:?}"
            );
        }
    }

    /// The one enclave rejection that is not retryable: mismatched deploys, not bad input.
    #[test]
    fn a_rejection_the_host_should_have_caught_is_not_retryable() {
        let operation = enclave_types::Error::EmptyPcp;
        let mapped = ApiError::migration_failure(&Failure::EnclaveRejected(operation));

        assert_eq!(mapped.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(mapped.code(), "internal_error");
        assert!(!mapped.allow_retry(), "{operation:?} should not be retried");
    }

    /// The diagnosing arm's detail must not be overwritten by a generic one.
    #[test]
    fn a_limit_disagreement_keeps_its_diagnostic_detail() {
        let mapped =
            ApiError::migration_failure(&Failure::EnclaveRejected(enclave_types::Error::EmptyPcp));

        assert_eq!(
            mapped.detail.as_deref(),
            Some("enclave rejected a payload the host accepted: EmptyPcp")
        );
    }
}
