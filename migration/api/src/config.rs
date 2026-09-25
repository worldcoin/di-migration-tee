use std::{net::SocketAddr, time::Duration};

use clap::Parser;
use thiserror::Error;

/// S3 rejects presigned URLs that outlive seven days.
const MAX_PRESIGNED_URL_TTL_SECS: u64 = 7 * 24 * 60 * 60;
const DEFAULT_PRESIGNED_URL_TTL_SECS: &str = "900";

#[derive(Debug, Parser)]
#[command(name = "migration-api")]
pub struct Config {
    #[arg(long, env = "HTTP_ADDR", default_value = "0.0.0.0:8080")]
    pub http_addr: SocketAddr,
    #[arg(long, env = "DYNAMODB_TABLE_NAME")]
    pub dynamodb_table_name: String,
    #[arg(long, env = "SQS_QUEUE_URL")]
    pub sqs_queue_url: String,
    #[arg(long, env = "PCP_BUCKET")]
    pub pcp_bucket: String,
    #[arg(
        long,
        env = "PRESIGNED_URL_TTL_SECS",
        default_value = DEFAULT_PRESIGNED_URL_TTL_SECS,
        value_parser = parse_presigned_url_ttl
    )]
    pub presigned_url_ttl: Duration,
    /// LocalStack and other S3-compatible endpoints only serve path-style addressing.
    #[arg(long, env = "S3_FORCE_PATH_STYLE", default_value_t = false, action = clap::ArgAction::Set)]
    pub s3_force_path_style: bool,
    #[arg(long, env = "ENCLAVE_ID")]
    pub enclave_id: String,
    #[arg(long, env = "STUB_ATTESTATION", default_value_t = false, action = clap::ArgAction::Set)]
    pub stub_attestation: bool,
}

fn parse_presigned_url_ttl(raw: &str) -> Result<Duration, std::num::ParseIntError> {
    raw.parse::<u64>().map(Duration::from_secs)
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Parse(#[from] clap::Error),
    #[error("DYNAMODB_TABLE_NAME is required")]
    MissingDynamodbTableName,
    #[error(
        "DYNAMODB_TABLE_NAME must be 3-255 ASCII letters, digits, underscores, hyphens, or dots"
    )]
    InvalidDynamodbTableName,
    #[error("SQS_QUEUE_URL is required")]
    MissingSqsQueueUrl,
    #[error("SQS_QUEUE_URL must be an HTTP(S) URL with a queue path")]
    InvalidSqsQueueUrl,
    #[error("{0} is required")]
    MissingVar(&'static str),
    #[error("PCP_BUCKET must be 3-63 lowercase letters, digits, hyphens, or dots")]
    InvalidPcpBucket,
    #[error(
        "PRESIGNED_URL_TTL_SECS must be a positive number of seconds, at most {MAX_PRESIGNED_URL_TTL_SECS}"
    )]
    InvalidPresignedUrlTtl,
    #[error("ENCLAVE_ID must be 1-128 ASCII letters, digits, underscores, or hyphens")]
    InvalidEnclaveId,
    #[error(
        "the migration enclave cannot attest yet; set STUB_ATTESTATION=true to serve an empty \
         attestation outside production"
    )]
    AttestationUnavailable,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Self::try_parse()?;
        if config.dynamodb_table_name.trim().is_empty() {
            return Err(ConfigError::MissingDynamodbTableName);
        }
        if !(3..=255).contains(&config.dynamodb_table_name.len())
            || !config
                .dynamodb_table_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(ConfigError::InvalidDynamodbTableName);
        }
        if config.sqs_queue_url.is_empty() {
            return Err(ConfigError::MissingSqsQueueUrl);
        }
        let queue_uri: axum::http::Uri = config
            .sqs_queue_url
            .parse()
            .map_err(|_| ConfigError::InvalidSqsQueueUrl)?;
        if !matches!(queue_uri.scheme_str(), Some("http" | "https"))
            || queue_uri.authority().is_none()
            || queue_uri.path() == "/"
        {
            return Err(ConfigError::InvalidSqsQueueUrl);
        }

        if config.pcp_bucket.trim().is_empty() {
            return Err(ConfigError::MissingVar("PCP_BUCKET"));
        }
        if !(3..=63).contains(&config.pcp_bucket.len())
            || !config.pcp_bucket.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
            })
        {
            return Err(ConfigError::InvalidPcpBucket);
        }

        if !(1..=MAX_PRESIGNED_URL_TTL_SECS).contains(&config.presigned_url_ttl.as_secs()) {
            return Err(ConfigError::InvalidPresignedUrlTtl);
        }

        if config.enclave_id.trim().is_empty() {
            return Err(ConfigError::MissingVar("ENCLAVE_ID"));
        }
        if config.enclave_id.len() > 128
            || !config
                .enclave_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ConfigError::InvalidEnclaveId);
        }

        if !config.stub_attestation {
            return Err(ConfigError::AttestationUnavailable);
        }

        Ok(config)
    }
}
