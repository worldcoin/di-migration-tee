use std::{net::SocketAddr, time::Duration};

use thiserror::Error;

/// S3 rejects presigned URLs that outlive seven days.
const MAX_PRESIGNED_URL_TTL_SECS: u64 = 7 * 24 * 60 * 60;
const DEFAULT_PRESIGNED_URL_TTL_SECS: u64 = 900;

#[derive(Debug)]
pub struct Config {
    pub http_addr: SocketAddr,
    pub dynamodb_table_name: String,
    pub sqs_queue_url: String,
    pub pcp_bucket: String,
    pub presigned_url_ttl: Duration,
    /// LocalStack and other S3-compatible endpoints only serve path-style addressing.
    pub s3_force_path_style: bool,
    pub enclave_id: String,
    pub stub_attestation: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read HTTP_ADDR: {0}")]
    ReadHttpAddr(#[from] std::env::VarError),
    #[error("invalid HTTP_ADDR: {0}")]
    InvalidHttpAddr(String),
    #[error("DYNAMODB_TABLE_NAME is required")]
    MissingDynamodbTableName,
    #[error("failed to read DYNAMODB_TABLE_NAME: {0}")]
    ReadDynamodbTableName(std::env::VarError),
    #[error(
        "DYNAMODB_TABLE_NAME must be 3-255 ASCII letters, digits, underscores, hyphens, or dots"
    )]
    InvalidDynamodbTableName,
    #[error("SQS_QUEUE_URL is required")]
    MissingSqsQueueUrl,
    #[error("failed to read SQS_QUEUE_URL: {0}")]
    ReadSqsQueueUrl(std::env::VarError),
    #[error("SQS_QUEUE_URL must be an HTTP(S) URL with a queue path")]
    InvalidSqsQueueUrl,
    #[error("{0} is required")]
    MissingVar(&'static str),
    #[error("failed to read {0}: {1}")]
    ReadVar(&'static str, std::env::VarError),
    #[error("PCP_BUCKET must be 3-63 lowercase letters, digits, hyphens, or dots")]
    InvalidPcpBucket,
    #[error(
        "PRESIGNED_URL_TTL_SECS must be a positive number of seconds, at most {MAX_PRESIGNED_URL_TTL_SECS}"
    )]
    InvalidPresignedUrlTtl,
    #[error("ENCLAVE_ID must be 1-128 ASCII letters, digits, underscores, or hyphens")]
    InvalidEnclaveId,
    #[error("{0} must be `true` or `false`")]
    InvalidBool(&'static str),
    #[error(
        "the migration enclave cannot attest yet; set STUB_ATTESTATION=true to serve an empty \
         attestation outside production"
    )]
    AttestationUnavailable,
}

fn optional_var(name: &'static str) -> Result<Option<String>, ConfigError> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
        Ok(_) | Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(ConfigError::ReadVar(name, error)),
    }
}

fn bool_var(name: &'static str) -> Result<bool, ConfigError> {
    match optional_var(name)?.as_deref() {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(_) => Err(ConfigError::InvalidBool(name)),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let http_addr = match std::env::var("HTTP_ADDR") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => "0.0.0.0:8080".to_owned(),
            Err(error) => return Err(error.into()),
        };
        let http_addr = http_addr
            .parse()
            .map_err(|error| ConfigError::InvalidHttpAddr(format!("{http_addr}: {error}")))?;
        let dynamodb_table_name = match std::env::var("DYNAMODB_TABLE_NAME") {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) | Err(std::env::VarError::NotPresent) => {
                return Err(ConfigError::MissingDynamodbTableName);
            }
            Err(error) => return Err(ConfigError::ReadDynamodbTableName(error)),
        };
        if !(3..=255).contains(&dynamodb_table_name.len())
            || !dynamodb_table_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(ConfigError::InvalidDynamodbTableName);
        }
        let sqs_queue_url = match std::env::var("SQS_QUEUE_URL") {
            Ok(value) if !value.is_empty() => value,
            Ok(_) | Err(std::env::VarError::NotPresent) => {
                return Err(ConfigError::MissingSqsQueueUrl);
            }
            Err(error) => return Err(ConfigError::ReadSqsQueueUrl(error)),
        };
        let queue_uri: axum::http::Uri = sqs_queue_url
            .parse()
            .map_err(|_| ConfigError::InvalidSqsQueueUrl)?;
        if !matches!(queue_uri.scheme_str(), Some("http" | "https"))
            || queue_uri.authority().is_none()
            || queue_uri.path() == "/"
        {
            return Err(ConfigError::InvalidSqsQueueUrl);
        }

        let pcp_bucket =
            optional_var("PCP_BUCKET")?.ok_or(ConfigError::MissingVar("PCP_BUCKET"))?;
        if !(3..=63).contains(&pcp_bucket.len())
            || !pcp_bucket.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
            })
        {
            return Err(ConfigError::InvalidPcpBucket);
        }

        let presigned_url_ttl_secs = match optional_var("PRESIGNED_URL_TTL_SECS")? {
            Some(value) => value
                .parse::<u64>()
                .ok()
                .filter(|secs| (1..=MAX_PRESIGNED_URL_TTL_SECS).contains(secs))
                .ok_or(ConfigError::InvalidPresignedUrlTtl)?,
            None => DEFAULT_PRESIGNED_URL_TTL_SECS,
        };

        let enclave_id =
            optional_var("ENCLAVE_ID")?.ok_or(ConfigError::MissingVar("ENCLAVE_ID"))?;
        if enclave_id.len() > 128
            || !enclave_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ConfigError::InvalidEnclaveId);
        }

        let stub_attestation = bool_var("STUB_ATTESTATION")?;
        if !stub_attestation {
            return Err(ConfigError::AttestationUnavailable);
        }

        Ok(Self {
            http_addr,
            dynamodb_table_name,
            sqs_queue_url,
            pcp_bucket,
            presigned_url_ttl: Duration::from_secs(presigned_url_ttl_secs),
            s3_force_path_style: bool_var("S3_FORCE_PATH_STYLE")?,
            enclave_id,
            stub_attestation,
        })
    }
}
